//! Material records: the payload stored after the name in a chunk tree's
//! material list (`0xBA01`) and as entries of `.rml` material libraries.
//!
//! The payload is a 184-byte block of shading parameters followed by 56-byte
//! texture layer records. Of the block, the emissive, diffuse and specular
//! colours, opacity, glossiness, specular level, render class, shading flags,
//! material kind, extra flags, the displacement-shadow step and bias, the
//! opacity fall-off, the physical divisor and the collision sound prefix are
//! decoded; the rest (among it the subsurface colour and the refraction
//! parameters) is not yet. Each layer names a texture and
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

    /// The slot flag a layer record stores for this slot.
    pub fn flag(self) -> u32 {
        match self {
            Self::Diffuse => 0x0002_0000,
            Self::Normal => 0x0020_0000,
            Self::Gloss => 0x0100_0000,
            Self::Specular => 0x0004_0000,
            Self::Height => 0x0080_0000,
            Self::Reflection => 0x0010_0000,
            Self::Opacity => 0x0040_0000,
            Self::Emissive => 0x0008_0000,
            Self::Dirt => 0x0800_0000,
            Self::Other(flag) => flag,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TextureLayer {
    pub slot: TextureSlot,
    /// Texture entry name, without any `.rfi`/`.dds` extension.
    pub texture: String,
    /// The layer's amount (body +4); for a height layer, the parallax depth.
    pub amount: f32,
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
    /// Extra flags (`0x4C`): `0x100000` enables displacement shadows,
    /// `0x100` turns the opacity fall-off toward the silhouette around.
    pub extra: u32,
    /// Subsurface colour (`0x6C`): non-zero materials draw skin's scattered
    /// light, and their diffuse texture skips the LOD bias.
    pub subsurface: [f32; 4],
    /// Displacement-shadow step and bias (`0x5C`, `0x60`), with a height map.
    pub displacement_step: f32,
    pub displacement_bias: f32,
    /// Opacity fall-off amount and exponent (`0x64`, `0x68`): surfaces
    /// fade with the view angle when the amount exceeds 0.01.
    pub falloff_amount: f32,
    pub falloff_exponent: f32,
    /// Physical divisor (`0xA4`): a rigid body on a node whose first mesh
    /// part has this material moves by its stored mobility over it. The
    /// original loads 0 as 1 ([`Material::mobility_divisor`]).
    pub physical_divisor: f32,
    /// Collision sound prefix (`0xAC`, up to 4 characters, empty in most
    /// records): the original's impact sound set is this prefix, or the
    /// material name's first three characters when empty, followed by
    /// `col`.
    pub collision_sound: String,
    /// Texture load flags (`0x54`): the sampler word every texture of the
    /// material starts from (`0x70000` = repeat on all axes in the shipped
    /// data), combined with the texture quality mask by the texture loader.
    pub texture_flags: u32,
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
                amount: f32::from_le_bytes(rec[12..16].try_into().expect("4 bytes")),
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
            extra: u32_at(0x4C),
            subsurface: rgba(0x6C),
            texture_flags: u32_at(0x54),
            displacement_step: f32_at(0x5C),
            displacement_bias: f32_at(0x60),
            falloff_amount: f32_at(0x64),
            falloff_exponent: f32_at(0x68),
            physical_divisor: f32_at(0xA4),
            collision_sound: {
                let raw = &payload[0xAC..0xB0];
                let end = raw.iter().position(|&c| c == 0).unwrap_or(raw.len());
                raw[..end].iter().map(|&c| c as char).collect()
            },
            layers,
        })
    }

    /// Whether the record carries a subsurface (secondary) colour: the
    /// squared length of its red, green and blue, summed in that order in
    /// single precision, is above zero (alpha does not count; NaN or values
    /// whose squares underflow do not either). The original's diffuse
    /// texture then skips the LOD bias.
    pub fn has_secondary_colour(&self) -> bool {
        let [r, g, b, _] = self.subsurface;
        r * r + g * g + b * b > 0.0
    }

    /// What a rigid body's stored mobility is divided by when the body is
    /// registered: the physical divisor, 1 when it is stored as 0 (as the
    /// original's material loader does).
    pub fn mobility_divisor(&self) -> f32 {
        if self.physical_divisor == 0.0 {
            1.0
        } else {
            self.physical_divisor
        }
    }

    /// The impact sound set of the material named `name`: the collision
    /// sound prefix, or the name's first three characters, then `col`.
    pub fn collision_sound_set(&self, name: &str) -> String {
        let prefix: String = if self.collision_sound.is_empty() {
            name.chars().take(3).collect()
        } else {
            self.collision_sound.clone()
        };
        format!("{prefix}col")
    }

    pub fn layer(&self, slot: TextureSlot) -> Option<&TextureLayer> {
        self.layers.iter().find(|l| l.slot == slot)
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
    fn slot_flags_round_trip() {
        for flag in [
            0x0002_0000,
            0x0020_0000,
            0x0100_0000,
            0x0004_0000,
            0x0080_0000,
            0x0010_0000,
            0x0040_0000,
            0x0008_0000,
            0x0800_0000,
            0x0400_0000,
        ] {
            assert_eq!(TextureSlot::from_flag(flag).flag(), flag);
        }
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
    fn reads_the_texture_load_flags() {
        let mut p = vec![0u8; PARAMETER_BLOCK];
        assert_eq!(Material::parse(&p).unwrap().texture_flags, 0);
        p[0x54..0x58].copy_from_slice(&0x0007_0000u32.to_le_bytes());
        assert_eq!(Material::parse(&p).unwrap().texture_flags, 0x0007_0000);
    }

    #[test]
    fn a_secondary_colour_is_a_non_zero_rgb() {
        let mut p = vec![0u8; PARAMETER_BLOCK];
        assert!(!Material::parse(&p).unwrap().has_secondary_colour());
        // Alpha alone does not count.
        p[0x78..0x7C].copy_from_slice(&1.0f32.to_le_bytes());
        assert!(!Material::parse(&p).unwrap().has_secondary_colour());
        // A blue whose square underflows does not either.
        p[0x74..0x78].copy_from_slice(&1e-30f32.to_le_bytes());
        assert!(!Material::parse(&p).unwrap().has_secondary_colour());
        p[0x74..0x78].copy_from_slice(&0.46f32.to_le_bytes());
        assert!(Material::parse(&p).unwrap().has_secondary_colour());
        p[0x6C..0x70].copy_from_slice(&f32::NAN.to_le_bytes());
        assert!(!Material::parse(&p).unwrap().has_secondary_colour());
    }

    #[test]
    fn reads_height_depth_and_displacement_settings() {
        let mut p = vec![0u8; PARAMETER_BLOCK];
        p[0x4C..0x50].copy_from_slice(&0x0010_0100u32.to_le_bytes());
        p[0x5C..0x60].copy_from_slice(&0.5f32.to_le_bytes());
        p[0x60..0x64].copy_from_slice(&0.25f32.to_le_bytes());
        p[0x64..0x68].copy_from_slice(&1.0f32.to_le_bytes());
        p[0x68..0x6C].copy_from_slice(&2.0f32.to_le_bytes());
        let mut h = layer(0x0080_0000, "UA1FA01H.rfi");
        h[12..16].copy_from_slice(&0.6f32.to_le_bytes());
        p.extend(h);
        let m = Material::parse(&p).unwrap();
        let height = m.layer(TextureSlot::Height).unwrap();
        assert_eq!(height.texture, "UA1FA01H");
        assert!((height.amount - 0.6).abs() < 1e-6);
        assert_eq!(m.extra, 0x0010_0100);
        assert_eq!((m.displacement_step, m.displacement_bias), (0.5, 0.25));
        assert_eq!((m.falloff_amount, m.falloff_exponent), (1.0, 2.0));
    }

    /// The physical divisor at `0xA4`, 1 when stored as 0 (`mtliron04` 3,
    /// most records 0), and the collision sound prefix at `0xAC`, else the
    /// name's first three characters, followed by `col`.
    #[test]
    fn reads_the_physical_divisor_and_the_collision_sound() {
        let mut p = vec![0u8; PARAMETER_BLOCK];
        let m = Material::parse(&p).unwrap();
        assert_eq!((m.physical_divisor, m.mobility_divisor()), (0.0, 1.0));
        assert_eq!(m.collision_sound, "");
        assert_eq!(m.collision_sound_set("mtliron04"), "mtlcol");
        p[0xA4..0xA8].copy_from_slice(&3.0f32.to_le_bytes());
        p[0xAC..0xAF].copy_from_slice(b"wdl");
        let m = Material::parse(&p).unwrap();
        assert_eq!((m.physical_divisor, m.mobility_divisor()), (3.0, 3.0));
        assert_eq!(m.collision_sound_set("barrelwood"), "wdlcol");
        // Four characters fill the field.
        p[0xAC..0xB0].copy_from_slice(b"MtlM");
        assert_eq!(Material::parse(&p).unwrap().collision_sound, "MtlM");
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
