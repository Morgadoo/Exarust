//! `.rfi` images to Bevy images, with the original's texture parameters.
//!
//! A texture's sampler/creation word is a bit field: the filtering bits, the
//! data (non-sRGB) bit, wrap bits per axis, the detail class and the target
//! kind. The image's header flags from bit 16 up (wrap bits, stored-chain
//! bit, detail class, target kind) are or-ed into the word the request
//! passes, as the original's loader does when it reads a named image. Block-compressed
//! images are uploaded as-is with their mip chain when the GPU supports BC
//! formats and the base level is block aligned; everything else is decoded
//! to RGBA8. Which stored mip levels are uploaded, and which levels the
//! Kaiser chain generates, follows `TextureDetail` ([`TextureSettings`]).
//!
//! The original sets `GL_TEXTURE_LOD_BIAS` per bind unless the texture asks
//! to skip it. Bevy's sampler descriptor has no LOD bias; [`TextureSampler`]
//! still computes it so the rule is testable, and a renderer wanting it must
//! pass it to the shader.

use bevy::asset::RenderAssetUsages;
use bevy::image::{ImageAddressMode, ImageFilterMode, ImageSampler, ImageSamplerDescriptor};
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use exa_formats::pixel::MipConversion;
use exa_formats::rfi::{PixelFormat, Rfi, level_dims};

/// How an image's colour should be interpreted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ColorSpace {
    Srgb,
    Linear,
}

/// The bits of a texture's sampler/creation word.
pub mod flags {
    /// Magnification filter: linear, else nearest.
    pub const MAG_LINEAR: u32 = 0x20;
    /// Minification filter within a level: linear, else nearest.
    pub const MIN_LINEAR: u32 = 0x10;
    /// Sample the mip chain (without it only the base level is uploaded).
    pub const MIPMAP: u32 = 0x40;
    /// Blend between mip levels: linear, else nearest.
    pub const MIP_LINEAR: u32 = 0x80;
    /// The filtering bits `TextureQuality` masks.
    pub const FILTER_MASK: u32 = 0xF0;
    /// The image holds data, not colour: upload linear, never sRGB.
    pub const DATA: u32 = 0x100;
    /// Do not set the LOD bias on bind.
    pub const NO_LOD_BIAS: u32 = 0x200;
    pub const REPEAT_S: u32 = 0x0001_0000;
    pub const REPEAT_T: u32 = 0x0002_0000;
    pub const REPEAT_R: u32 = 0x0004_0000;
    /// The texture's target kind, bits 28–30.
    pub const TARGET_MASK: u32 = 0x7000_0000;
    pub const TARGET_2D: u32 = 0x0000_0000;
    pub const TARGET_3D: u32 = 0x1000_0000;
    /// A second 2D value the original maps to `GL_TEXTURE_2D`.
    pub const TARGET_2D_ALT: u32 = 0x2000_0000;
    pub const TARGET_CUBE: u32 = 0x3000_0000;
    pub const TARGET_CUBE_ARRAY: u32 = 0x4000_0000;
    /// The anisotropy mode, subtracted from the quality's level.
    pub const ANISOTROPY_MASK: u32 = 0x3;
    /// The filtered image's texels are vectors: every generated mip level
    /// normalises each texel's first three channels (normal maps).
    pub const NORMALISE: u32 = 0x0400_0000;
}

/// The texture settings the original reads from its globals.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TextureSettings {
    /// The `TextureQuality` mask (0xF0 at quality 5).
    pub quality_mask: u32,
    /// The anisotropy level (4 at quality 5).
    pub anisotropy_level: u32,
    /// `TextureDetail` (3 in the retail user's settings, minimum 2).
    pub texture_detail: u32,
    /// The supersampling level the LOD bias follows.
    pub supersample: i32,
    /// The LOD bias base the quality setting writes (-0.75 at quality 5,
    /// -0.5 at 4, 0 below; the static before any setting is -0.5).
    pub lod_bias_base: f32,
    /// The device's largest texture side. The original reads
    /// `GL_MAX_TEXTURE_SIZE` once its context exists, before any upload (its
    /// static 256 is never used for one); Exarust uses wgpu's default 2D
    /// limit.
    pub max_texture_size: u32,
    /// The driver's texture units, clamped to 16.
    pub max_units: u32,
}

impl TextureSettings {
    /// Applies the two limits read once the context exists: the driver's
    /// largest texture side, and its texture units clamped to 16 (the
    /// original's cap).
    pub fn apply_driver_limits(&mut self, max_texture_size: u32, max_units: u32) {
        self.max_texture_size = max_texture_size;
        self.max_units = max_units.min(16);
    }

    /// The values the `TextureQuality` setting writes: the filter mask,
    /// the anisotropy level and the LOD bias base. A negative quality keeps
    /// the statics (mask `0xF0`, level 3, base -0.5), and so does one above
    /// 6, which the setting's own clamp never lets through.
    pub fn with_quality(mut self, quality: i32) -> Self {
        let (mask, level, base) = match quality {
            0 => (0x00, 0, 0.0),
            1 => (0x40, 0, 0.0),
            2 => (0x70, 0, 0.0),
            3 => (0x70, 1, 0.0),
            4 => (0xF0, 2, -0.5),
            5 | 6 => (0xF0, 4, -0.75),
            _ => return self,
        };
        self.quality_mask = mask;
        self.anisotropy_level = level;
        self.lod_bias_base = base;
        self
    }

    /// The values the original forces when its window gets a generic
    /// (software) pixel format: mask 3, detail 1, anisotropy level 0.
    pub fn fallback() -> Self {
        Self {
            quality_mask: 3,
            anisotropy_level: 0,
            texture_detail: 1,
            ..Self::default()
        }
    }
}

impl Default for TextureSettings {
    /// The retail user's settings: quality 5, detail 3, 4x supersampling.
    fn default() -> Self {
        Self {
            quality_mask: flags::FILTER_MASK,
            anisotropy_level: 4,
            texture_detail: 3,
            supersample: 4,
            lod_bias_base: -0.75,
            max_texture_size: DEVICE_MAX_TEXTURE_SIZE,
            max_units: 16,
        }
    }
}

/// wgpu's default limit on a 2D texture's side, which every Exarust device
/// meets.
pub const DEVICE_MAX_TEXTURE_SIZE: u32 = 8192;

/// The texture's target, from bits 28–30 of its word.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextureTarget {
    D2,
    D3,
    Cube,
    CubeArray,
}

impl TextureTarget {
    pub fn from_flags(flags: u32) -> TextureTarget {
        match flags & flags::TARGET_MASK {
            flags::TARGET_3D => TextureTarget::D3,
            flags::TARGET_CUBE => TextureTarget::Cube,
            flags::TARGET_CUBE_ARRAY => TextureTarget::CubeArray,
            // Both 0 and 0x20000000 map to GL_TEXTURE_2D.
            _ => TextureTarget::D2,
        }
    }
}

/// The sampler state the word asks for.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TextureSampler {
    pub address_mode_u: ImageAddressMode,
    pub address_mode_v: ImageAddressMode,
    pub address_mode_w: ImageAddressMode,
    pub mag_filter: ImageFilterMode,
    pub min_filter: ImageFilterMode,
    pub mipmap_filter: ImageFilterMode,
    pub anisotropy_clamp: u16,
    /// The LOD bias in the original, `None` when the texture skips it.
    pub lod_bias: Option<f32>,
}

/// The LOD bias of a supersampling setting: the setting (or one less from 2
/// up) times the f32 third, then times the base, in the original's f32
/// order.
pub fn lod_bias(base: f32, supersample: i32) -> f32 {
    let factor = if supersample < 2 {
        supersample as f32
    } else {
        (supersample as i64 - 1) as f32
    };
    base * (factor * 0.333_333_34)
}

impl TextureSampler {
    /// Reads the sampler state from a texture word. Wrap comes from the
    /// word's bits; the image's own header bits are or-ed in before this is
    /// called.
    pub fn from_flags(flags: u32, settings: &TextureSettings) -> TextureSampler {
        let address = |bit: u32| {
            if flags & bit != 0 {
                ImageAddressMode::Repeat
            } else {
                ImageAddressMode::ClampToEdge
            }
        };
        let min_filter = if flags & flags::MIN_LINEAR != 0 {
            ImageFilterMode::Linear
        } else {
            ImageFilterMode::Nearest
        };
        let mipmap_filter = if flags & flags::MIPMAP == 0 {
            ImageFilterMode::Nearest
        } else if flags & flags::MIP_LINEAR != 0 {
            ImageFilterMode::Linear
        } else {
            ImageFilterMode::Nearest
        };
        // 2^(level − mode) while the mode is below the level, else 1; mode 3
        // never filters anisotropically. The hardware clamps it to what it
        // supports (16), and wgpu only accepts it with linear filters
        // everywhere.
        let mode = flags & flags::ANISOTROPY_MASK;
        let level = settings.anisotropy_level;
        let mut anisotropy_clamp = if mode == 3 || mode >= level {
            1
        } else {
            1u16 << (level - mode).min(4)
        };
        let all_linear = flags & flags::MAG_LINEAR != 0
            && min_filter == ImageFilterMode::Linear
            && mipmap_filter == ImageFilterMode::Linear;
        if !all_linear {
            anisotropy_clamp = 1;
        }
        let lod_bias = if flags & flags::NO_LOD_BIAS != 0 {
            None
        } else {
            Some(lod_bias(settings.lod_bias_base, settings.supersample))
        };
        TextureSampler {
            address_mode_u: address(flags::REPEAT_S),
            address_mode_v: address(flags::REPEAT_T),
            address_mode_w: address(flags::REPEAT_R),
            mag_filter: if flags & flags::MAG_LINEAR != 0 {
                ImageFilterMode::Linear
            } else {
                ImageFilterMode::Nearest
            },
            min_filter,
            mipmap_filter,
            anisotropy_clamp,
            lod_bias,
        }
    }

    fn descriptor(self) -> ImageSamplerDescriptor {
        ImageSamplerDescriptor {
            address_mode_u: self.address_mode_u,
            address_mode_v: self.address_mode_v,
            address_mode_w: self.address_mode_w,
            mag_filter: self.mag_filter,
            min_filter: self.min_filter,
            mipmap_filter: self.mipmap_filter,
            anisotropy_clamp: self.anisotropy_clamp,
            ..ImageSamplerDescriptor::linear()
        }
    }
}

/// The word a material texture starts from: the record's texture load
/// flags or-ed with the quality mask, as the material loader does. Each slot
/// then adds its data and bias bits, and the image its header flags.
pub fn material_word(record_flags: u32, settings: &TextureSettings) -> u32 {
    record_flags | settings.quality_mask
}

/// Whether a stored format word names data rather than colour. An
/// uncompressed word with bit 16 set (the `0x0111` and `0x1111` words:
/// packed specular, normal and height maps) is uploaded linear by the
/// original's upload routine itself; a compressed one is data when its top
/// byte is `0x82` (BC1 specular, BC3) or `0x92` (BC5 normals), whose format
/// table entries have no sRGB variant. Those are never uploaded as sRGB,
/// whatever the material asks for.
pub fn word_is_data(format_word: u32) -> bool {
    if format_word & 0x8000_0000 == 0 {
        format_word & 0x0001_0000 != 0
    } else {
        matches!(format_word >> 24, 0x82 | 0x92)
    }
}

/// The header flags a named image adds to the requested word: everything
/// from bit 16 up (wrap, stored chain, detail class, target kind).
pub const IMAGE_FLAG_MASK: u32 = 0xffff_0000;

/// Whether the word asks for the sRGB formats: the request's data bit and
/// the stored format's own class both have a say.
pub fn is_srgb(space: ColorSpace, flags: u32, format_word: u32) -> bool {
    space == ColorSpace::Srgb && flags & flags::DATA == 0 && !word_is_data(format_word)
}

/// The header flags of the original's placeholder image: repeat on every
/// axis, a 2D target.
pub const PLACEHOLDER_FLAGS: u32 = 0x2007_0000;

/// The 1×1 grey placeholder the original uploads when an image cannot be
/// read: an RGB8 image of (64, 64, 64), opaque, whose header flags (repeat
/// on every axis) join the requested word like any named image's.
pub fn placeholder_image(space: ColorSpace, flags: u32, settings: &TextureSettings) -> Image {
    let word = flags | PLACEHOLDER_FLAGS;
    let srgb = is_srgb(space, word, exa_formats::pixel::id::RGB8);
    let sampler = TextureSampler::from_flags(word, settings);
    let mut image = Image::new_fill(
        Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[0x40, 0x40, 0x40, 0xff],
        if srgb {
            TextureFormat::Rgba8UnormSrgb
        } else {
            TextureFormat::Rgba8Unorm
        },
        RenderAssetUsages::RENDER_WORLD,
    );
    image.sampler = ImageSampler::Descriptor(sampler.descriptor());
    image
}

/// The image with the original's parameters, from the default texture word
/// (a material texture at the retail settings).
pub fn rfi_to_image(
    rfi: &Rfi,
    space: ColorSpace,
    bc_supported: bool,
) -> exa_formats::Result<Image> {
    rfi_to_texture(
        rfi,
        space,
        bc_supported,
        default_flags(),
        &TextureSettings::default(),
    )
}

/// The word a material texture gets when the record does not say.
pub fn default_flags() -> u32 {
    flags::REPEAT_S
        | flags::REPEAT_T
        | flags::REPEAT_R
        | flags::MAG_LINEAR
        | flags::MIN_LINEAR
        | flags::MIPMAP
        | flags::MIP_LINEAR
}

/// The image with a texture word: the stored or generated levels the detail
/// setting keeps, the sRGB or linear format the word and space pick, and the
/// sampler state it asks for.
pub fn rfi_to_texture(
    rfi: &Rfi,
    space: ColorSpace,
    bc_supported: bool,
    flags: u32,
    settings: &TextureSettings,
) -> exa_formats::Result<Image> {
    let w = rfi.header.width;
    let h = rfi.header.height;
    // The image's own header keeps the wrap, chain, detail and target bits;
    // the request supplies the filtering and data bits.
    let mut word = flags | (rfi.header.flags & IMAGE_FLAG_MASK);
    let compressed = rfi.header.format_word & 0x8000_0000 != 0;
    let chain = rfi.header.has_mip_chain();
    // A compressed image without a stored chain cannot be mipmapped: the
    // original drops both mip bits from its word and uploads the base alone.
    if compressed && !chain && settings.quality_mask & word & flags::MIPMAP != 0 {
        word &= !(flags::MIPMAP | flags::MIP_LINEAR);
    }
    let srgb = is_srgb(space, word, rfi.header.format_word);
    let sampler = TextureSampler::from_flags(word, settings);
    let mut first = rfi
        .header
        .first_mip_level(settings.texture_detail, settings.max_texture_size);
    if compressed && !chain {
        first = 0;
    }
    if chain {
        // A chosen level past the stored chain uploads the smallest stored
        // one; the original would upload nothing (retail data never asks
        // for this).
        first = first.min(rfi.mip_count.saturating_sub(1));
    }
    // The chain runs to its end only when the quality mask lets the word's
    // mip bit through.
    let mipmapped = settings.quality_mask & word & flags::MIPMAP != 0;
    let block_format = match rfi.format {
        PixelFormat::Bc1 if srgb => Some(TextureFormat::Bc1RgbaUnormSrgb),
        PixelFormat::Bc1 => Some(TextureFormat::Bc1RgbaUnorm),
        PixelFormat::Bc3 if srgb => Some(TextureFormat::Bc3RgbaUnormSrgb),
        PixelFormat::Bc3 => Some(TextureFormat::Bc3RgbaUnorm),
        // Two-channel normal maps: Bevy reconstructs Z for RG formats.
        PixelFormat::Bc5 if !srgb => Some(TextureFormat::Bc5RgUnorm),
        _ => None,
    };
    let image = match block_format {
        Some(format)
            if bc_supported && w.is_multiple_of(4) && h.is_multiple_of(4) && !rfi.is_packed() =>
        {
            // A stored chain is uploaded from the chosen level; a compressed
            // image without one keeps its base level alone.
            let last = if mipmapped && chain {
                rfi.mip_count.saturating_sub(1)
            } else {
                first
            };
            block_image(rfi, format, first, last)?
        }
        _ => {
            // Every kept level is decoded; a chain that is not stored is
            // generated, starting at the level the detail setting chose.
            let mut mip_count = 1u32;
            let mut rgba = if chain {
                let last = if mipmapped {
                    rfi.mip_count.saturating_sub(1)
                } else {
                    first
                };
                let pixels = rfi.pixels()?;
                let mut out = Vec::new();
                for level in first..=last {
                    out.extend(rfi.decode_level_rgba8(&pixels, level)?);
                }
                mip_count = last + 1 - first;
                out
            } else {
                rfi.decode_rgba8()?
            };
            if !chain && !compressed {
                // With mipmapping the chain runs to its end; without it only
                // the chosen level is generated, or the base when the
                // chosen level is 0.
                let last = if mipmapped { u32::MAX } else { first };
                let (chain, levels) = exa_render::mipmap::build_mip_chain_rgba8_between_flagged(
                    &rgba,
                    w as usize,
                    h as usize,
                    first,
                    last,
                    true,
                    mip_conversion(rfi.header.format_word),
                    rfi.header.flags & flags::NORMALISE != 0,
                );
                if levels > 0 {
                    rgba = chain;
                    mip_count = levels;
                } else {
                    // A skip past the end of a tiny image: keep the stored
                    // base (the retail data never asks for this).
                    first = 0;
                }
            }
            let mut image = Image::new_uninit(
                Extent3d {
                    width: (w >> first.min(31)).max(1),
                    height: (h >> first.min(31)).max(1),
                    depth_or_array_layers: 1,
                },
                TextureDimension::D2,
                if srgb {
                    TextureFormat::Rgba8UnormSrgb
                } else {
                    TextureFormat::Rgba8Unorm
                },
                RenderAssetUsages::RENDER_WORLD,
            );
            image.texture_descriptor.mip_level_count = mip_count;
            image.data = Some(rgba);
            image
        }
    };
    let mut image = image;
    image.sampler = ImageSampler::Descriptor(sampler.descriptor());
    Ok(image)
}

/// A compressed image's uploaded levels, from the stored chain.
fn block_image(
    rfi: &Rfi,
    format: TextureFormat,
    first: u32,
    last: u32,
) -> exa_formats::Result<Image> {
    let w = rfi.header.width;
    let h = rfi.header.height;
    let pixels = rfi.pixels()?;
    let mut offset = 0usize;
    let mut start = 0usize;
    let mut len = 0usize;
    for level in 0..=last {
        let (lw, lh) = level_dims(w, h, level);
        let size = rfi.format.level_size(lw, lh).unwrap_or(0);
        if level == first {
            start = offset;
        }
        if level >= first {
            len += size;
        }
        offset += size;
    }
    let end = (start + len).min(pixels.len());
    let data = pixels[start.min(end)..end].to_vec();
    let mut image = Image::new_uninit(
        Extent3d {
            width: w >> first.min(31),
            height: h >> first.min(31),
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        format,
        RenderAssetUsages::RENDER_WORLD,
    );
    image.texture_descriptor.mip_level_count = (last - first + 1).max(1);
    image.data = Some(data);
    Ok(image)
}

/// The filtering space of a generated chain: the original's pick for the
/// format word (squared colour for plain 8-bit images, linear for data
/// words, centred for signed ones). Block-compressed images decoded on the
/// CPU, which the original never mips itself, filter linearly.
fn mip_conversion(word: u32) -> MipConversion {
    exa_formats::pixel::mip_conversion(word).unwrap_or(MipConversion::Linear)
}

/// Whether the image carries an alpha channel worth alpha-testing.
pub fn has_alpha(rfi: &Rfi) -> bool {
    matches!(
        rfi.format,
        PixelFormat::Bc3 | PixelFormat::Rgba8 | PixelFormat::La8 | PixelFormat::A8
    )
}

#[cfg(test)]
mod sampler_tests {
    use super::*;

    #[test]
    fn the_lod_bias_follows_the_supersampling_setting() {
        // base * f / 3 in f32, f = ss below 2 and ss - 1 from 2 up.
        assert_eq!(lod_bias(-0.75, 0), 0.0);
        assert_eq!(lod_bias(-0.75, 1), -0.25);
        assert_eq!(lod_bias(-0.75, 2), -0.25);
        assert_eq!(lod_bias(-0.75, 3), -0.5);
        assert_eq!(lod_bias(-0.75, 4), -0.75);
        assert!((lod_bias(-0.5, 4) + 0.5).abs() < 1e-7);
        // The third is applied to the factor first: base -0.007 at 4x
        // gives exactly -0.007, where (base * 3) * third would give
        // -0.0070000007.
        assert_eq!(lod_bias(-0.007, 4), -0.007);
        assert_ne!(-0.007f32 * 3.0 * 0.333_333_34, -0.007);
    }

    #[test]
    fn the_sampler_word_picks_filters_wrap_and_anisotropy() {
        let settings = TextureSettings::default();
        // A material texture: record flags 0x70000 and the quality mask.
        let word = material_word(0x70000, &settings);
        assert_eq!(word, 0x700F0);
        let sampler = TextureSampler::from_flags(word, &settings);
        assert_eq!(sampler.mag_filter, ImageFilterMode::Linear);
        assert_eq!(sampler.min_filter, ImageFilterMode::Linear);
        assert_eq!(sampler.mipmap_filter, ImageFilterMode::Linear);
        assert_eq!(sampler.address_mode_u, ImageAddressMode::Repeat);
        assert_eq!(sampler.address_mode_v, ImageAddressMode::Repeat);
        assert_eq!(sampler.address_mode_w, ImageAddressMode::Repeat);
        assert_eq!(sampler.anisotropy_clamp, 16);
        assert_eq!(sampler.lod_bias, Some(-0.75));
        // A data map that asks to skip the bias and clamps both ways.
        let sampler = TextureSampler::from_flags(0x300 | 0x20, &settings);
        assert_eq!(sampler.address_mode_u, ImageAddressMode::ClampToEdge);
        assert_eq!(sampler.mipmap_filter, ImageFilterMode::Nearest);
        assert_eq!(sampler.lod_bias, None);
        // Without linear filters everywhere wgpu takes no anisotropy.
        assert_eq!(sampler.anisotropy_clamp, 1);
        // Anisotropy mode 1 halves the level, mode 3 turns it off, and a
        // mode at or above the level leaves 1.
        let aniso = |word, level| {
            let settings = TextureSettings {
                anisotropy_level: level,
                ..TextureSettings::default()
            };
            TextureSampler::from_flags(word, &settings).anisotropy_clamp
        };
        assert_eq!(aniso(0x700F1, 4), 8);
        assert_eq!(aniso(0x700F2, 4), 4);
        assert_eq!(aniso(0x700F3, 4), 1);
        assert_eq!(aniso(0x700F2, 2), 1);
        assert_eq!(aniso(0x700F1, 2), 2);
        assert_eq!(aniso(0x700F0, 0), 1);
    }

    #[test]
    fn the_quality_setting_writes_mask_level_and_bias() {
        let base = TextureSettings::default();
        let q = |quality| {
            let s = base.with_quality(quality);
            (s.quality_mask, s.anisotropy_level, s.lod_bias_base)
        };
        assert_eq!(q(0), (0x00, 0, 0.0));
        assert_eq!(q(1), (0x40, 0, 0.0));
        assert_eq!(q(2), (0x70, 0, 0.0));
        assert_eq!(q(3), (0x70, 1, 0.0));
        assert_eq!(q(4), (0xF0, 2, -0.5));
        assert_eq!(q(5), (0xF0, 4, -0.75));
        assert_eq!(q(6), (0xF0, 4, -0.75));
        // Outside the table the values stay as they are.
        assert_eq!(q(7), (0xF0, 4, -0.75));
        assert_eq!(q(-1), (0xF0, 4, -0.75));
        assert_eq!(base, TextureSettings::default().with_quality(5));
    }

    #[test]
    fn the_target_comes_from_the_top_kind_bits() {
        assert_eq!(TextureTarget::from_flags(0), TextureTarget::D2);
        assert_eq!(
            TextureTarget::from_flags(flags::TARGET_3D),
            TextureTarget::D3
        );
        assert_eq!(
            TextureTarget::from_flags(flags::TARGET_2D_ALT),
            TextureTarget::D2
        );
        assert_eq!(
            TextureTarget::from_flags(flags::TARGET_CUBE),
            TextureTarget::Cube
        );
        assert_eq!(
            TextureTarget::from_flags(flags::TARGET_CUBE_ARRAY),
            TextureTarget::CubeArray
        );
    }

    #[test]
    fn the_driver_limits_cap_the_texture_units() {
        let mut settings = TextureSettings::default();
        settings.apply_driver_limits(16_384, 32);
        assert_eq!(settings.max_texture_size, 16_384);
        assert_eq!(settings.max_units, 16);
        settings.apply_driver_limits(8192, 8);
        assert_eq!(settings.max_texture_size, 8192);
        assert_eq!(settings.max_units, 8);
        // The fallback's values, as the original sets them without a settings
        // file.
        let fallback = TextureSettings::fallback();
        assert_eq!(fallback.quality_mask, 3);
        assert_eq!(fallback.texture_detail, 1);
        assert_eq!(fallback.anisotropy_level, 0);
    }

    #[test]
    fn the_data_bit_and_the_format_word_beat_the_colour_space() {
        let colour = 0x0100_C600;
        assert!(is_srgb(ColorSpace::Srgb, 0, colour));
        assert!(!is_srgb(ColorSpace::Srgb, flags::DATA, colour));
        assert!(!is_srgb(ColorSpace::Linear, 0, colour));
        assert!(!is_srgb(ColorSpace::Linear, flags::DATA, colour));
        // A specular BC1, a normal BC5, and the uncompressed words with bit
        // 16 (packed specular, normal and height maps) are data formats.
        for word in [
            0x823B_C600,
            0x927B_8400,
            0x827B_A408,
            0x0111_C600,
            0x1111_C600,
            0x0111_4200,
            0x1111_8400,
        ] {
            assert!(word_is_data(word), "{word:#010x}");
            assert!(!is_srgb(ColorSpace::Srgb, 0, word), "{word:#010x}");
        }
        // Colour words; a compressed word's bit 16 says nothing.
        for word in [
            0x813B_C600,
            0x817B_E608,
            0x813B_4200,
            0x0100_E608,
            0x0100_4200,
            0x0100_C600,
            0x0100_6208,
        ] {
            assert!(!word_is_data(word), "{word:#010x}");
            assert!(is_srgb(ColorSpace::Srgb, 0, word), "{word:#010x}");
        }
    }

    #[test]
    fn a_failed_image_gets_the_grey_placeholder() {
        let settings = TextureSettings::default();
        let image = placeholder_image(ColorSpace::Srgb, default_flags(), &settings);
        assert_eq!(image.texture_descriptor.size.width, 1);
        assert_eq!(image.texture_descriptor.size.height, 1);
        assert_eq!(image.data.as_deref(), Some(&[0x40, 0x40, 0x40, 0xff][..]));
        assert_eq!(
            image.texture_descriptor.format,
            TextureFormat::Rgba8UnormSrgb
        );
        let linear = placeholder_image(ColorSpace::Linear, default_flags(), &settings);
        assert_eq!(linear.texture_descriptor.format, TextureFormat::Rgba8Unorm);
        // Its own header repeats it on every axis, whatever the request.
        let clamped = placeholder_image(ColorSpace::Srgb, 0xF0, &settings);
        let ImageSampler::Descriptor(sampler) = clamped.sampler else {
            panic!("the placeholder has a sampler");
        };
        assert_eq!(sampler.address_mode_u, ImageAddressMode::Repeat);
        assert_eq!(sampler.address_mode_v, ImageAddressMode::Repeat);
    }
}

#[cfg(test)]
mod level_tests {
    use super::*;
    use exa_formats::rfi::RfiHeader;

    /// An image with `levels` RGBA8 levels of `w` x `h` (base), filled with
    /// `value` per level.
    fn rgba_image(w: u32, h: u32, levels: u32, flags: u32) -> (Vec<u8>, RfiHeader) {
        let mut data = Vec::new();
        let mut size = 0;
        for level in 0..levels {
            let (lw, lh) = level_dims(w, h, level);
            data.extend(std::iter::repeat_n(level as u8, (lw * lh * 4) as usize));
            size += lw * lh * 4;
        }
        let header = RfiHeader {
            width: w,
            height: h,
            depth: 1,
            format_word: 0x0100_E608,
            flags: if levels > 1 { flags | 0x8_0000 } else { flags },
            storage: 0x1000_0000,
            data_size: size,
        };
        (data, header)
    }

    #[test]
    fn the_detail_setting_keeps_the_chosen_stored_level() {
        // 4x4 plus 2x2 plus 1x1, class 3, detail 3: the 2x2 is the base.
        let (data, mut header) = rgba_image(4, 4, 3, 3 << 22);
        header.flags |= 3 << 22;
        let mut bytes = Vec::new();
        for v in [
            exa_formats::rfi::RFI_MAGIC,
            header.width,
            header.height,
            header.depth,
            header.format_word,
            header.flags,
            header.storage,
            header.data_size,
        ] {
            bytes.extend_from_slice(&v.to_le_bytes());
        }
        bytes.extend_from_slice(&data);
        let rfi = Rfi::parse(&bytes).unwrap();
        assert_eq!(rfi.mip_count, 3);
        let image = rfi_to_texture(
            &rfi,
            ColorSpace::Srgb,
            false,
            default_flags(),
            &TextureSettings::default(),
        )
        .unwrap();
        assert_eq!(image.texture_descriptor.size.width, 2);
        assert_eq!(image.texture_descriptor.size.height, 2);
        // Mipmapping keeps the rest of the chain: level 1 (1x1) here.
        assert_eq!(image.texture_descriptor.mip_level_count, 2);
        assert_eq!(image.data.as_ref().unwrap().len(), 2 * 2 * 4 + 4);
        assert_eq!(image.data.as_ref().unwrap()[0], 1);
        // Without mipmapping only the chosen level is uploaded.
        let image = rfi_to_texture(
            &rfi,
            ColorSpace::Srgb,
            false,
            default_flags() & !flags::MIPMAP,
            &TextureSettings::default(),
        )
        .unwrap();
        assert_eq!(image.texture_descriptor.mip_level_count, 1);
        assert_eq!(image.data.as_ref().unwrap().len(), 2 * 2 * 4);
    }

    /// A single-level image gets the Kaiser chain only when the word asks
    /// for mipmapping; otherwise it is uploaded as its base alone.
    #[test]
    fn mipmapping_decides_whether_a_chain_is_generated() {
        let (data, header) = rgba_image(4, 4, 1, 0);
        let mut bytes = Vec::new();
        for v in [
            exa_formats::rfi::RFI_MAGIC,
            header.width,
            header.height,
            header.depth,
            header.format_word,
            header.flags,
            header.storage,
            header.data_size,
        ] {
            bytes.extend_from_slice(&v.to_le_bytes());
        }
        bytes.extend_from_slice(&data);
        let rfi = Rfi::parse(&bytes).unwrap();
        let settings = TextureSettings::default();
        let plain = rfi_to_texture(
            &rfi,
            ColorSpace::Srgb,
            false,
            default_flags() & !flags::MIPMAP,
            &settings,
        )
        .unwrap();
        assert_eq!(plain.texture_descriptor.mip_level_count, 1);
        assert_eq!(plain.data.as_ref().unwrap().len(), 4 * 4 * 4);
        let chained =
            rfi_to_texture(&rfi, ColorSpace::Srgb, false, default_flags(), &settings).unwrap();
        assert_eq!(chained.texture_descriptor.mip_level_count, 3);
        assert_eq!(
            chained.data.as_ref().unwrap().len(),
            4 * 4 * 4 + 2 * 2 * 4 + 4
        );
    }

    fn rfi_bytes(header: &RfiHeader, data: &[u8]) -> Vec<u8> {
        let mut bytes = Vec::new();
        for v in [
            exa_formats::rfi::RFI_MAGIC,
            header.width,
            header.height,
            header.depth,
            header.format_word,
            header.flags,
            header.storage,
            header.data_size,
        ] {
            bytes.extend_from_slice(&v.to_le_bytes());
        }
        bytes.extend_from_slice(data);
        bytes
    }

    /// A compressed image without a stored chain keeps its base level and
    /// loses its mip filter, whatever the detail class asks, on the GPU
    /// and on the CPU path alike.
    #[test]
    fn a_compressed_image_without_a_chain_uploads_its_base_alone() {
        // 8 x 8 BC1, one level of four blocks, detail class 3.
        let header = RfiHeader {
            width: 8,
            height: 8,
            depth: 1,
            format_word: 0x813B_C600,
            flags: 3 << 22,
            storage: 0x1000_0000,
            data_size: 32,
        };
        let bytes = rfi_bytes(&header, &[0u8; 32]);
        let rfi = Rfi::parse(&bytes).unwrap();
        assert_eq!(rfi.mip_count, 1);
        for bc in [true, false] {
            let image = rfi_to_texture(
                &rfi,
                ColorSpace::Srgb,
                bc,
                default_flags(),
                &TextureSettings::default(),
            )
            .unwrap();
            assert_eq!(image.texture_descriptor.size.width, 8, "bc {bc}");
            assert_eq!(image.texture_descriptor.mip_level_count, 1, "bc {bc}");
            let ImageSampler::Descriptor(sampler) = image.sampler else {
                panic!("a sampler");
            };
            assert_eq!(sampler.mipmap_filter, ImageFilterMode::Nearest);
        }
    }

    /// The image's own wrap bits join the requested word: a material that
    /// asks for no wrap still repeats along the axes the file repeats.
    #[test]
    fn the_file_wrap_bits_join_the_word() {
        let (data, header) = rgba_image(4, 4, 1, 0x2001_0000);
        let bytes = rfi_bytes(&header, &data);
        let rfi = Rfi::parse(&bytes).unwrap();
        let image = rfi_to_texture(
            &rfi,
            ColorSpace::Srgb,
            false,
            0xF0,
            &TextureSettings::default(),
        )
        .unwrap();
        let ImageSampler::Descriptor(sampler) = image.sampler else {
            panic!("a sampler");
        };
        assert_eq!(sampler.address_mode_u, ImageAddressMode::Repeat);
        assert_eq!(sampler.address_mode_v, ImageAddressMode::ClampToEdge);
    }

    /// The quality mask gates the chain: a word that asks for mipmaps gets
    /// none when the mask drops the mip bit.
    #[test]
    fn the_quality_mask_gates_the_chain() {
        let (data, header) = rgba_image(4, 4, 1, 0);
        let bytes = rfi_bytes(&header, &data);
        let rfi = Rfi::parse(&bytes).unwrap();
        let settings = TextureSettings::default().with_quality(0);
        let image =
            rfi_to_texture(&rfi, ColorSpace::Srgb, false, default_flags(), &settings).unwrap();
        assert_eq!(image.texture_descriptor.mip_level_count, 1);
    }

    /// A chosen level past a stored chain clamps to its smallest level.
    #[test]
    fn a_chosen_level_past_a_stored_chain_clamps() {
        let (data, mut header) = rgba_image(2, 2, 2, 3 << 22);
        header.flags |= 3 << 22;
        let mut bytes = Vec::new();
        for v in [
            exa_formats::rfi::RFI_MAGIC,
            header.width,
            header.height,
            header.depth,
            header.format_word,
            header.flags,
            header.storage,
            header.data_size,
        ] {
            bytes.extend_from_slice(&v.to_le_bytes());
        }
        bytes.extend_from_slice(&data);
        let rfi = Rfi::parse(&bytes).unwrap();
        assert_eq!(rfi.mip_count, 2);
        let settings = TextureSettings {
            texture_detail: 2,
            ..TextureSettings::default()
        };
        let image =
            rfi_to_texture(&rfi, ColorSpace::Srgb, false, default_flags(), &settings).unwrap();
        assert_eq!(image.texture_descriptor.size.width, 1);
        assert_eq!(image.texture_descriptor.mip_level_count, 1);
    }

    /// A detail level past the natural chain still yields the floored 1×1
    /// level the original filters: the sides never reach zero.
    #[test]
    fn a_skip_past_the_chain_still_yields_a_one_texel_level() {
        let (data, mut header) = rgba_image(2, 2, 1, 3 << 22);
        header.flags |= 3 << 22;
        let mut bytes = Vec::new();
        for v in [
            exa_formats::rfi::RFI_MAGIC,
            header.width,
            header.height,
            header.depth,
            header.format_word,
            header.flags,
            header.storage,
            header.data_size,
        ] {
            bytes.extend_from_slice(&v.to_le_bytes());
        }
        bytes.extend_from_slice(&data);
        let rfi = Rfi::parse(&bytes).unwrap();
        let settings = TextureSettings {
            texture_detail: 2,
            ..TextureSettings::default()
        };
        let image =
            rfi_to_texture(&rfi, ColorSpace::Srgb, false, default_flags(), &settings).unwrap();
        assert_eq!(image.texture_descriptor.size.width, 1);
        assert_eq!(image.texture_descriptor.size.height, 1);
        assert_eq!(image.texture_descriptor.mip_level_count, 1);
        assert_eq!(image.data.as_ref().unwrap().len(), 4);
    }
}

#[cfg(test)]
mod corpus_tests {
    use super::*;
    use exa_formats::FileKind;
    use exa_rpk::GameData;

    /// Every shipped image converts with the material word at the retail
    /// settings (skipped without the game), and its stored-chain flag
    /// agrees with the levels its data size holds. Prints the detail
    /// classes, how many images the detail setting starts at a later level,
    /// how many compressed images have no stored chain (and so no mips),
    /// and the time the GPU (BC) path takes for the whole corpus.
    #[test]
    fn every_shipped_image_converts() {
        let Some(dir) = crate::locate_install(None) else {
            return;
        };
        let game = GameData::open(dir).unwrap();
        let settings = TextureSettings::default();
        let word = material_word(0x0007_0000, &settings);
        let mut classes = [0usize; 4];
        let mut later = 0usize;
        let mut stored_chain = 0usize;
        let mut unchained_compressed = 0usize;
        let mut images = 0usize;
        let start = std::time::Instant::now();
        for (_, pack) in game.packs() {
            for entry in pack.entries() {
                let bytes = pack.bytes(entry);
                if FileKind::detect(bytes) != FileKind::Image {
                    continue;
                }
                let Ok(rfi) = Rfi::parse(bytes) else {
                    continue;
                };
                images += 1;
                assert_eq!(
                    rfi.header.has_mip_chain(),
                    rfi.mip_count > 1,
                    "{}",
                    entry.name
                );
                classes[rfi.header.detail_class() as usize] += 1;
                stored_chain += usize::from(rfi.header.has_mip_chain());
                let first = rfi
                    .header
                    .first_mip_level(settings.texture_detail, settings.max_texture_size);
                let compressed = rfi.header.format_word & 0x8000_0000 != 0;
                unchained_compressed += usize::from(compressed && !rfi.header.has_mip_chain());
                if first > 0 && !(compressed && !rfi.header.has_mip_chain()) {
                    later += 1;
                }
                let space = if word_is_data(rfi.header.format_word) {
                    ColorSpace::Linear
                } else {
                    ColorSpace::Srgb
                };
                let image = rfi_to_texture(&rfi, space, true, word, &settings)
                    .unwrap_or_else(|e| panic!("{}: {e}", entry.name));
                let size = image.texture_descriptor.size;
                assert!(size.width >= 1 && size.height >= 1, "{}", entry.name);
                assert!(image.data.as_ref().is_some_and(|d| !d.is_empty()));
            }
        }
        eprintln!(
            "{images} images, classes {classes:?}, {stored_chain} with a stored chain, \
             {unchained_compressed} compressed without one, \
             {later} start at a later level at detail {}; {:?}",
            settings.texture_detail,
            start.elapsed()
        );
        assert!(images > 1000);
    }
}
