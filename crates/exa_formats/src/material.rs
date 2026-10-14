//! Material records: the payload stored after the name in a chunk tree's
//! material list (`0xBA01`) and as entries of `.rml` material libraries.
//!
//! The payload is a 184-byte block of shading parameters followed by 56-byte
//! texture layer records. Of the block, the diffuse colour (bytes 16..32) and
//! specular colour (32..48) are decoded; the rest is not yet. Each layer names a texture and
//! the slot it fills. Slot roles were assigned from the suffixes of the
//! texture names that use them across the retail data (`d` diffuse, `n`
//! normal, …).

use crate::{Error, Result};

const PARAMETER_BLOCK: usize = 184;
const LAYER_LEN: usize = 56;
const LAYER_NAME_OFFSET: usize = 40;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TextureSlot {
    Diffuse,
    Normal,
    Gloss,
    Specular,
    Height,
    /// Environment-map override (a cube map).
    Reflection,
    /// Opacity: the alpha channel is used (cobwebs, foliage cards).
    Opacity,
    Emissive,
    /// Dirt and grime, mixed in by vertex alpha.
    Dirt,
    Other(u32),
}

impl TextureSlot {
    pub fn from_flag(flag: u32) -> Self {
        match flag {
            0x0002_0000 => Self::Diffuse,
            0x0020_0000 => Self::Normal,
            0x0100_0000 => Self::Gloss,
            0x0004_0000 => Self::Specular,
            0x0080_0000 => Self::Height,
            0x0010_0000 => Self::Reflection,
            0x0040_0000 => Self::Opacity,
            0x0008_0000 => Self::Emissive,
            0x0800_0000 => Self::Dirt,
            other => Self::Other(other),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextureLayer {
    pub slot: TextureSlot,
    /// Texture entry name, without any `.rfi`/`.dds` extension.
    pub texture: String,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Material {
    /// Colours as stored: sRGB, linearised by the renderer.
    pub emissive: [f32; 4],
    pub diffuse: [f32; 4],
    pub specular: [f32; 4],
    /// 1 solid, 0 glass, 0.6 decals.
    pub opacity: f32,
    /// 0..1.
    pub glossiness: f32,
    /// Multiplies the specular colour.
    pub specular_level: f32,
    /// Render class: low nibble 3 is transparent; bits 12..14 the blend mode.
    pub class: u32,
    /// Shading flags ([`flags`]) and the texture slots in use.
    pub flags: u32,
    /// 0: legacy (diffuse ×1.5); bit 0: the specular map is a packed
    /// intensity/gloss/metal map.
    pub kind: u32,
    pub layers: Vec<TextureLayer>,
}

/// Material shading flags.
pub mod flags {
    /// Lit by ambient and lights.
    pub const LIT: u32 = 0x3;
    /// Has a specular colour.
    pub const SPECULAR: u32 = 0x4;
    pub const TWO_SIDED: u32 = 0x10;
    pub const REFRACT: u32 = 0x200;
    pub const OPACITY_FALLOFF: u32 = 0x800;
}

impl Material {
    pub fn parse(payload: &[u8]) -> Result<Self> {
        if payload.len() < PARAMETER_BLOCK {
            return Err(Error::Truncated {
                offset: 0,
                needed: PARAMETER_BLOCK,
                len: payload.len(),
            });
        }
        let rgba = |o: usize| -> [f32; 4] {
            std::array::from_fn(|i| {
                f32::from_le_bytes(
                    payload[o + i * 4..o + i * 4 + 4]
                        .try_into()
                        .expect("4 bytes"),
                )
            })
        };
        let f32_at = |o: usize| f32::from_le_bytes(payload[o..o + 4].try_into().expect("4 bytes"));
        let u32_at = |o: usize| u32::from_le_bytes(payload[o..o + 4].try_into().expect("4 bytes"));
        let mut layers = Vec::new();
        let mut p = PARAMETER_BLOCK;
        while p < payload.len() {
            let rec = payload.get(p..p + LAYER_LEN).ok_or(Error::Truncated {
                offset: p,
                needed: LAYER_LEN,
                len: payload.len(),
            })?;
            let flag = u32::from_le_bytes(rec[0..4].try_into().expect("4 bytes"));
            let len = u32::from_le_bytes(rec[4..8].try_into().expect("4 bytes")) as usize;
            if len != LAYER_LEN {
                return Err(Error::Unsupported(format!(
                    "texture layer of {len} bytes at offset {p}"
                )));
            }
            let raw = &rec[LAYER_NAME_OFFSET..];
            let end = raw.iter().position(|&c| c == 0).unwrap_or(raw.len());
            let name: String = raw[..end].iter().map(|&c| c as char).collect();
            layers.push(TextureLayer {
                slot: TextureSlot::from_flag(flag),
                texture: strip_extension(&name).to_string(),
            });
            p += LAYER_LEN;
        }
        Ok(Self {
            emissive: rgba(0x00),
            diffuse: rgba(0x10),
            specular: rgba(0x20),
            opacity: f32_at(0x38),
            glossiness: f32_at(0x3C),
            specular_level: f32_at(0x40),
            class: u32_at(0x44),
            flags: u32_at(0x48),
            kind: u32_at(0x50),
            layers,
        })
    }

    pub fn texture(&self, slot: TextureSlot) -> Option<&str> {
        self.layers
            .iter()
            .find(|l| l.slot == slot)
            .map(|l| l.texture.as_str())
    }
}

fn strip_extension(name: &str) -> &str {
    let lower = name.to_ascii_lowercase();
    for ext in [".rfi", ".dds"] {
        if lower.ends_with(ext) {
            return &name[..name.len() - ext.len()];
        }
    }
    name
}

#[cfg(test)]
mod tests {
    use super::*;

    fn layer(flag: u32, name: &str) -> Vec<u8> {
        let mut r = vec![0u8; LAYER_LEN];
        r[0..4].copy_from_slice(&flag.to_le_bytes());
        r[4..8].copy_from_slice(&(LAYER_LEN as u32).to_le_bytes());
        r[LAYER_NAME_OFFSET..LAYER_NAME_OFFSET + name.len()].copy_from_slice(name.as_bytes());
        r
    }

    #[test]
    fn reads_layers_and_strips_extensions() {
        let mut p = vec![0u8; PARAMETER_BLOCK];
        p[16..20].copy_from_slice(&0.5f32.to_le_bytes());
        p.extend(layer(0x0002_0000, "GrassPalA01D"));
        p.extend(layer(0x0020_0000, "wall01n.rfi"));
        let m = Material::parse(&p).unwrap();
        assert_eq!(m.texture(TextureSlot::Diffuse), Some("GrassPalA01D"));
        assert_eq!(m.texture(TextureSlot::Normal), Some("wall01n"));
        assert_eq!(m.diffuse[0], 0.5);
    }

    #[test]
    fn rejects_short_and_misaligned_payloads() {
        assert!(Material::parse(&[0; 10]).is_err());
        let mut p = vec![0u8; PARAMETER_BLOCK];
        p.extend(layer(0x0002_0000, "x"));
        p[PARAMETER_BLOCK + 4] = 40;
        assert!(Material::parse(&p).is_err());
    }
}
