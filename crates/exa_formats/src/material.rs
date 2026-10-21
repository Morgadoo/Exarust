//! Material records: the payload stored after the name in a chunk tree's
//! material list (`0xBA01`) and as entries of `.rml` material libraries.
//!
//! The payload is a 184-byte block of shading parameters followed by 56-byte
//! texture layer records. Of the block, the emissive, diffuse and specular
//! colours, opacity, glossiness, specular level, render class, shading flags,
//! material kind, extra flags, the displacement-shadow step and bias, the
//! opacity fall-off, the subsurface colour, the physical divisor and the
//! collision sound prefix are decoded; the rest (among it the refraction
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
    /// The layer's parameter word (body +0); on a normal-map layer `0x10`
    /// marks an object-space map (no tangent frame).
    pub parameter: u32,
    /// The layer's amount (body +4); for a height layer, the parallax depth.
    pub amount: f32,
}

/// The texture-coordinate matrix the record loader builds for a record
/// whose extra flags (`0x4C`) carry `0x40`, from its angle (`0x8C`), scale
/// (`0x84`) and translation (`0x7C`): a 3 × 3 rotation-scale block and a
/// translation row, row-major. With `(s, c)` the engine's sine and cosine
/// of the negated angle:
///
/// `m[0] = c·sx`, `m[1] = −s·sy`, `m[3] = s·sx`, `m[4] = c·sy`, `m[8] = 1`,
/// `m[9] = ((½ − m[0]·½) − m[3]·½) + tx`, `m[10] = ((½ − m[1]·½) −
/// m[4]·½) + ty`, the rest zero, each step in single precision in that
/// order (the scale and rotation are about the coordinates' centre).
pub fn uv_transform(angle: f32, scale: [f32; 2], offset: [f32; 2]) -> [f32; 12] {
    let (s, c) = crate::factory::sin_cos(f32::from_bits(angle.to_bits() ^ 0x8000_0000));
    let mut m = [0.0f32; 12];
    m[0] = c * scale[0];
    m[1] = -s * scale[1];
    m[3] = s * scale[0];
    m[4] = c * scale[1];
    m[8] = 1.0;
    m[9] = ((0.5 - m[0] * 0.5) - m[3] * 0.5) + offset[0];
    m[10] = ((0.5 - m[1] * 0.5) - m[4] * 0.5) + offset[1];
    m
}

/// The engine's single-precision epsilon: the loader replaces a refraction
/// word smaller than it in magnitude by 1.
const REFRACTION_EPSILON: f32 = f32::from_bits(0x33FF_FFD7);

#[derive(Debug, Clone, PartialEq)]
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
    /// Dirt strength (`0x30`): the dirt modes' colour take, `2 − this` in the
    /// material uniform's `MatDrt.w`.
    pub dirt_amount: f32,
    /// The word at `0x34`, which the factory copies into the draw state of
    /// a record whose plug-in-masked flags have bit 0; its reader there is
    /// not traced. It ships as 1 in most records.
    pub dirt_lane: f32,
    /// Shading flags ([`flags`]) and the texture slots in use.
    pub flags: u32,
    /// 0: legacy (diffuse ×1.5); bit 0: the specular map is a packed
    /// intensity/gloss/metal map; bits 8–9 (`0x300`) hold the material mode,
    /// `0x100` being the translucent program.
    pub kind: u32,
    /// Extra flags (`0x4C`): `0x100000` enables displacement shadows,
    /// `0x100` turns the opacity fall-off toward the silhouette around.
    pub extra: u32,
    /// Subsurface colour (`0x6C`): non-zero materials draw skin's scattered
    /// light (the skin program at shading quality 3 and above), and their
    /// diffuse texture skips the LOD bias.
    pub subsurface: [f32; 4],
    /// Displacement-shadow step and bias (`0x5C`, `0x60`), with a height map.
    pub displacement_step: f32,
    pub displacement_bias: f32,
    /// Opacity fall-off amount and exponent (`0x64`, `0x68`): surfaces
    /// fade with the view angle when the amount exceeds 0.01.
    pub falloff_amount: f32,
    pub falloff_exponent: f32,
    /// The refraction word (`0x58`), replaced by 1 when its magnitude is
    /// below 2⁻²³ (a NaN is kept): the factory packs it into the opacity
    /// fall-off uniform's fourth lane for a record in refraction mode
    /// (`flags & 0x300 == 0x200`).
    pub refraction: f32,
    /// The texture-coordinate matrix ([`uv_transform`]) when extra flag
    /// `0x40` asks for it.
    pub uv_transform: Option<[f32; 12]>,
    /// The texture-coordinate rotation angle (`0x8C`), radians.
    pub uv_angle: f32,
    /// The texture-coordinate translation (`0x7C`) and scale (`0x84`).
    pub uv_offset: [f32; 2],
    pub uv_scale: [f32; 2],
    /// Render-state word (`0x90`): the loader keeps its bit `0x20` and adds
    /// its own bits.
    pub render_state: u32,
    /// Plug-in mask (`0x94`): the factory records (and plug-ins) a record
    /// calls are those whose mask meets it.
    pub plugin_mask: u32,
    /// Words `0x98`, `0x9C`, `0xA0` and `0xA8`: kept whole, no reader found
    /// for them in the shipped data.
    pub reserved: [u32; 4],
    /// Physical divisor (`0xA4`): a rigid body on a node whose first mesh
    /// part has this material moves by its stored mobility over it. The
    /// original loads 0 as 1 ([`Material::mobility_divisor`]).
    pub physical_divisor: f32,
    /// The last two block words (`0xB0`, `0xB4`), kept whole.
    pub tail: [u32; 2],
    /// Collision sound prefix (`0xAC`, up to 4 characters, empty in most
    /// records): the original's impact sound set is this prefix, or the
    /// material name's first three characters when empty, followed by
    /// `col`.
    pub collision_sound: String,
    /// Texture load flags (`0x54`): the sampler word every texture of the
    /// material starts from (`0x70000` = repeat on all axes in the shipped
    /// data), combined with the texture quality mask by the texture loader.
    pub texture_flags: u32,
    /// Every word of the 184-byte block, in order: the whole record, so a
    /// pass-through rewrite keeps it.
    pub raw_block: [u32; PARAMETER_BLOCK / 4],
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

impl Default for Material {
    fn default() -> Self {
        Self {
            emissive: [0.0; 4],
            diffuse: [0.0; 4],
            specular: [0.0; 4],
            opacity: 0.0,
            glossiness: 0.0,
            specular_level: 0.0,
            class: 0,
            dirt_amount: 0.0,
            dirt_lane: 0.0,
            flags: 0,
            kind: 0,
            extra: 0,
            subsurface: [0.0; 4],
            displacement_step: 0.0,
            displacement_bias: 0.0,
            falloff_amount: 0.0,
            falloff_exponent: 0.0,
            refraction: 0.0,
            uv_transform: None,
            uv_angle: 0.0,
            uv_offset: [0.0; 2],
            uv_scale: [0.0; 2],
            render_state: 0,
            plugin_mask: 0,
            reserved: [0; 4],
            physical_divisor: DEFAULT_PHYSICAL_DIVISOR,
            tail: [0; 2],
            collision_sound: String::new(),
            texture_flags: DEFAULT_TEXTURE_FLAGS,
            raw_block: [0; PARAMETER_BLOCK / 4],
            layers: Vec::new(),
        }
    }
}

/// The texture load flags a material object starts from: the original's
/// `0xF0` with its `0x70000` sampler bits (repeat wrapping on every axis).
pub const DEFAULT_TEXTURE_FLAGS: u32 = 0xF0 | 0x70000;
/// The physical divisor a material object starts from.
pub const DEFAULT_PHYSICAL_DIVISOR: f32 = 1.0;
/// The material record used for a name the libraries do not know: the
/// original's built-in `_Default`, opaque grey with no texture.
pub const DEFAULT_MATERIAL_NAME: &str = "_Default";
/// The built-in `_Default` material's diffuse colour (sRGB grey) and
/// opacity, as its 32-byte payload stores them (emissive black).
pub const DEFAULT_MATERIAL_DIFFUSE: [f32; 4] = [0.5, 0.5, 0.5, 1.0];
/// The reference count the built-in `_Default` material starts with: it is
/// never freed.
pub const DEFAULT_MATERIAL_REFERENCES: i32 = 0x3FFF_FFFF;

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
        let vec2_at = |o: usize| [f32_at(o), f32_at(o + 4)];
        let mut raw_block = [0u32; PARAMETER_BLOCK / 4];
        for (i, word) in raw_block.iter_mut().enumerate() {
            *word = u32_at(i * 4);
        }
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
                parameter: u32::from_le_bytes(rec[8..12].try_into().expect("4 bytes")),
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
            dirt_amount: f32_at(0x30),
            dirt_lane: f32_at(0x34),
            flags: u32_at(0x48),
            kind: u32_at(0x50),
            extra: u32_at(0x4C),
            subsurface: rgba(0x6C),
            texture_flags: u32_at(0x54),
            displacement_step: f32_at(0x5C),
            displacement_bias: f32_at(0x60),
            falloff_amount: f32_at(0x64),
            falloff_exponent: f32_at(0x68),
            refraction: {
                let v = f32_at(0x58);
                if v.abs() < REFRACTION_EPSILON { 1.0 } else { v }
            },
            uv_angle: f32_at(0x8C),
            uv_offset: vec2_at(0x7C),
            uv_scale: vec2_at(0x84),
            uv_transform: (u32_at(0x4C) & 0x40 != 0)
                .then(|| uv_transform(f32_at(0x8C), vec2_at(0x84), vec2_at(0x7C))),
            render_state: u32_at(0x90),
            plugin_mask: u32_at(0x94),
            reserved: [u32_at(0x98), u32_at(0x9C), u32_at(0xA0), u32_at(0xA8)],
            tail: [u32_at(0xB0), u32_at(0xB4)],
            raw_block,
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

    /// Whether the record carries a subsurface colour, the original's test
    /// for the skin program ([`Material::has_secondary_colour`]).
    pub fn has_subsurface(&self) -> bool {
        self.has_secondary_colour()
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
    fn reads_the_subsurface_colour() {
        let mut p = vec![0u8; PARAMETER_BLOCK];
        for (i, value) in [1.0f32, 0.8, 0.46, 1.0].iter().enumerate() {
            p[0x6C + i * 4..0x70 + i * 4].copy_from_slice(&value.to_le_bytes());
        }
        let m = Material::parse(&p).unwrap();
        assert_eq!(m.subsurface, [1.0, 0.8, 0.46, 1.0]);
        assert!(m.has_subsurface());
        let mut black = vec![0u8; PARAMETER_BLOCK];
        black[0x78..0x7C].copy_from_slice(&1.0f32.to_le_bytes());
        assert!(!Material::parse(&black).unwrap().has_subsurface());
        assert!(
            !Material::parse(&[0u8; PARAMETER_BLOCK])
                .unwrap()
                .has_subsurface()
        );
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
        let mut n = layer(0x0020_0000, "n");
        n[8..12].copy_from_slice(&0x10u32.to_le_bytes());
        p.extend(n);
        let m = Material::parse(&p).unwrap();
        assert_eq!(m.layer(TextureSlot::Normal).unwrap().parameter, 0x10);
        let height = m.layer(TextureSlot::Height).unwrap();
        assert_eq!(height.parameter, 0);
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

    /// Every word of the 184-byte block is decoded, including the ones with
    /// no established meaning, so a rewrite keeps the record whole.
    #[test]
    fn the_whole_block_is_decoded() {
        let mut p = vec![0u8; PARAMETER_BLOCK];
        for (i, word) in p.chunks_mut(4).enumerate() {
            word.copy_from_slice(&(0x1000_0000u32 + i as u32).to_le_bytes());
        }
        let m = Material::parse(&p).unwrap();
        assert_eq!(m.raw_block.len(), 46);
        assert_eq!(m.raw_block[0], 0x1000_0000);
        assert_eq!(m.raw_block[45], 0x1000_002D);
        assert_eq!(m.raw_block[0xB0 / 4], m.tail[0]);
        assert_eq!(m.raw_block[0xB4 / 4], m.tail[1]);
        assert_eq!(m.raw_block[0x98 / 4], m.reserved[0]);
        assert_eq!(m.raw_block[0xA8 / 4], m.reserved[3]);
        assert_eq!(m.render_state, 0x1000_0024);
        assert_eq!(m.plugin_mask, 0x1000_0025);
        // The refraction word becomes one when its magnitude is under the
        // engine's epsilon; all other floats are kept as stored.
        let mut q = vec![0u8; PARAMETER_BLOCK];
        assert_eq!(Material::parse(&q).unwrap().refraction, 1.0);
        q[0x58..0x5C].copy_from_slice(&0.5f32.to_le_bytes());
        assert_eq!(Material::parse(&q).unwrap().refraction, 0.5);
        q[0x58..0x5C].copy_from_slice(&1e-30f32.to_le_bytes());
        assert_eq!(Material::parse(&q).unwrap().refraction, 1.0);
        q[0x58..0x5C].copy_from_slice(&(-1.0e-7f32).to_le_bytes());
        assert_eq!(Material::parse(&q).unwrap().refraction, 1.0);
        q[0x58..0x5C].copy_from_slice(&f32::NAN.to_le_bytes());
        assert!(Material::parse(&q).unwrap().refraction.is_nan());
    }

    #[test]
    fn the_uv_builder_scales_about_the_centre_then_rotates_by_the_negated_angle() {
        // Unit scale, no rotation: the identity, no translation.
        let m = uv_transform(0.0, [1.0, 1.0], [0.0, 0.0]);
        assert_eq!((m[0], m[1], m[3], m[4], m[8]), (1.0, 0.0, 0.0, 1.0, 1.0));
        assert_eq!((m[9], m[10]), (0.0, 0.0));
        // Scale 2 about the centre: the translation row is −0.5.
        let m = uv_transform(0.0, [2.0, 2.0], [0.0, 0.0]);
        assert_eq!((m[0], m[4], m[9], m[10]), (2.0, 2.0, -0.5, -0.5));
        // A translation adds last.
        let m = uv_transform(0.0, [1.0, 1.0], [0.25, -0.5]);
        assert_eq!((m[9], m[10]), (0.25, -0.5));
        // A quarter turn: (s, c) are the sine and cosine of −π/2, so
        // m[1] = −s = 1 and m[3] = s = −1; then m[9] = ½ − c·½ + ½ = 1 and
        // m[10] = ½ − ½ − c·½ = 0 (to the polynomial's precision).
        let (s, c) = crate::factory::sin_cos(-std::f32::consts::FRAC_PI_2);
        let m = uv_transform(std::f32::consts::FRAC_PI_2, [1.0, 1.0], [0.0, 0.0]);
        assert_eq!((m[0], m[1], m[3], m[4]), (c, -s, s, c));
        assert!((m[1] - 1.0).abs() < 1e-6 && (m[3] + 1.0).abs() < 1e-6);
        assert_eq!(m[9], ((0.5 - m[0] * 0.5) - m[3] * 0.5) + 0.0);
        assert!((m[9] - 1.0).abs() < 1e-6 && m[10].abs() < 1e-6);
    }

    #[test]
    fn the_uv_transform_is_built_only_with_extra_flag_0x40() {
        let mut p = vec![0u8; PARAMETER_BLOCK];
        p[0x84..0x88].copy_from_slice(&2.0f32.to_le_bytes());
        p[0x88..0x8C].copy_from_slice(&2.0f32.to_le_bytes());
        let m = Material::parse(&p).unwrap();
        assert_eq!(m.uv_transform, None);
        assert_eq!(m.uv_scale, [2.0, 2.0]);
        p[0x4C..0x50].copy_from_slice(&0x40u32.to_le_bytes());
        let m = Material::parse(&p).unwrap().uv_transform.unwrap();
        assert_eq!((m[0], m[9]), (2.0, -0.5));
    }

    /// The constructor's defaults: texture load flags 0x700F0 (0xF0 and the
    /// 0x70000 sampler bits) and a physical divisor of 1. The built-in
    /// `_Default` material is opaque grey with a reference count it never
    /// loses.
    #[test]
    fn constructed_material_defaults() {
        let m = Material::default();
        assert_eq!(m.uv_transform, None);
        assert_eq!(m.texture_flags, 0x700F0);
        assert_eq!(m.physical_divisor, 1.0);
        assert_eq!(m.mobility_divisor(), 1.0);
        assert_eq!(DEFAULT_MATERIAL_DIFFUSE, [0.5, 0.5, 0.5, 1.0]);
        assert_eq!(DEFAULT_MATERIAL_NAME, "_Default");
        assert_eq!(DEFAULT_MATERIAL_REFERENCES, 0x3FFF_FFFF);
    }
}
