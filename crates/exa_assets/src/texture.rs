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
    /// `GL_TEXTURE_1D`.
    pub const TARGET_1D: u32 = 0x1000_0000;
    /// A second 2D value the original maps to `GL_TEXTURE_2D`.
    pub const TARGET_2D_ALT: u32 = 0x2000_0000;
    /// `GL_TEXTURE_3D`.
    pub const TARGET_3D: u32 = 0x3000_0000;
    /// `GL_TEXTURE_CUBE_MAP`.
    pub const TARGET_CUBE: u32 = 0x4000_0000;
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

    /// The LOD bias these settings write (the value the original sets per
    /// bind and a material carries to the shader).
    pub fn lod_bias(&self) -> f32 {
        lod_bias(self.lod_bias_base, self.supersample)
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

/// The texture's target, from bits 28–30 of its word: the original's switch
/// over those bits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextureTarget {
    D1,
    D2,
    D3,
    Cube,
}

impl TextureTarget {
    pub fn from_flags(flags: u32) -> TextureTarget {
        match flags & flags::TARGET_MASK {
            flags::TARGET_1D => TextureTarget::D1,
            flags::TARGET_3D => TextureTarget::D3,
            flags::TARGET_CUBE => TextureTarget::Cube,
            // Both 0, 0x20000000 and the three values above 0x40000000 map
            // to GL_TEXTURE_2D.
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
/// up) times the f32 third, then the base times that, in the order of the
/// original's instructions (its pseudo-code reads `base · f · ⅓`, which
/// rounds differently).
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

/// The original's table of uncompressed pixel formats it uploads as they
/// are, in its order: the stored format word (compared under
/// [`COMPATIBLE_MASK`]) and whether the GPU has an sRGB internal format for
/// it. ALPHA8 and the two-channel words store the same linear format in
/// both of the table's columns, so their values are never sRGB-decoded.
pub const COMPATIBLE_FORMATS: [(u32, bool); 7] = [
    (0x0100_C600, true),  // RGB8
    (0x0100_E608, true),  // RGBA8
    (0x0100_4200, true),  // LUM8
    (0x0100_2008, false), // ALPHA8
    (0x0100_6208, true),  // LUMA8
    (0x0100_8400, false), // AB8
    (0x1111_8400, false), // XY8
];

/// The mask the original's format table compares a stored word under: it
/// drops the packed-data bits (bit 16 and the top-nibble kind).
pub const COMPATIBLE_MASK: u32 = 0x800C_FFFF;

/// Whether an uncompressed stored word has an sRGB internal format in the
/// original's table. A word the table does not list is converted by the
/// image codec before the original uploads it; no shipped image has one, and
/// Exarust treats it as having no sRGB variant (its pixels stay linear).
pub fn has_srgb_format(format_word: u32) -> bool {
    let key = format_word & COMPATIBLE_MASK;
    // The two blue-first words convert to RGB8 and RGBA8 (their registered
    // fallbacks), which have sRGB internal formats.
    if matches!(key, 0x0200_C600 | 0x0200_E608) {
        return true;
    }
    COMPATIBLE_FORMATS
        .iter()
        .find(|(word, _)| word & COMPATIBLE_MASK == key)
        .is_some_and(|(_, srgb)| *srgb)
}

/// The header flags a named image adds to the requested word: everything
/// from bit 16 up (wrap, stored chain, detail class, target kind).
pub const IMAGE_FLAG_MASK: u32 = 0xffff_0000;

/// Whether the word asks for the sRGB formats: the request's data bit, the
/// stored format's own class and whether the format has an sRGB variant all
/// have a say.
pub fn is_srgb(space: ColorSpace, flags: u32, format_word: u32) -> bool {
    space == ColorSpace::Srgb
        && flags & flags::DATA == 0
        && !word_is_data(format_word)
        && (format_word & 0x8000_0000 != 0 || has_srgb_format(format_word))
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

/// Decodes one image for an upload: the `.rfi` bytes, colour space, word and
/// settings a loader request carries. `None` when the bytes are not an image
/// the loader can read (the caller uploads the placeholder), with the alpha
/// flag the header gives.
pub fn decode_uploaded(
    bytes: &[u8],
    space: ColorSpace,
    flags: u32,
    settings: &TextureSettings,
    bc_supported: bool,
) -> Option<(Image, bool)> {
    let rfi = Rfi::parse(bytes).ok()?;
    let alpha = has_alpha(&rfi);
    rfi_to_texture(&rfi, space, bc_supported, flags, settings)
        .ok()
        .map(|image| (image, alpha))
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
    // The word's target kind picks the upload: a cube map stores six faces,
    // each with its own chain; every shipped image asks for 2D.
    if TextureTarget::from_flags(flags | (rfi.header.flags & IMAGE_FLAG_MASK))
        == TextureTarget::Cube
    {
        return cube_to_texture(rfi, space, flags, settings);
    }
    let w = rfi.header.width;
    let h = rfi.header.height;
    let UploadPlan {
        word,
        srgb,
        mut first,
        mipmapped,
        compressed,
        chain,
    } = upload_plan(rfi, space, flags, settings);
    let sampler = TextureSampler::from_flags(word, settings);
    let block_format = match rfi.format {
        PixelFormat::Bc1 | PixelFormat::Bc1Alpha if srgb => Some(TextureFormat::Bc1RgbaUnormSrgb),
        PixelFormat::Bc1 | PixelFormat::Bc1Alpha => Some(TextureFormat::Bc1RgbaUnorm),
        PixelFormat::Bc3 if srgb => Some(TextureFormat::Bc3RgbaUnormSrgb),
        PixelFormat::Bc3 => Some(TextureFormat::Bc3RgbaUnorm),
        // A single grey channel: the original's luminance LATC format, which
        // has no sRGB variant, so the value is never decoded. The shader
        // repeats the red channel over rgb ([`crate::model::TextureInfo`]'s
        // grey flag).
        PixelFormat::Bc4 => Some(TextureFormat::Bc4RUnorm),
        // Two-channel normal maps: Bevy reconstructs Z for RG formats.
        PixelFormat::Bc5 if !srgb => Some(TextureFormat::Bc5RgUnorm),
        // The alpha-only and luminance-alpha BC4 variants have no wgpu
        // upload that keeps their channel meaning: they decode to RGBA8.
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
                // The chain reads the image's own flags: its wrap on each
                // axis and the normal-map renormalisation. The format word
                // picks the filtering space and the bank (uniform for data
                // words, Kaiser for colour).
                let last = if mipmapped { u32::MAX } else { first };
                let options = exa_render::mipmap::ChainOptions::for_image(
                    rfi.header.format_word,
                    rfi.header.flags,
                    first,
                    last,
                );
                let (chain, levels) = exa_render::mipmap::build_mip_chain_rgba8_with(
                    &rgba, w as usize, h as usize, &options,
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

/// What a request's word resolves to before the upload: the creation word
/// (image header flags joined), the colour space, the first level the detail
/// setting keeps, whether the chain runs to its end, and the image's own
/// compression and chain bits.
struct UploadPlan {
    word: u32,
    srgb: bool,
    first: u32,
    mipmapped: bool,
    compressed: bool,
    chain: bool,
}

fn upload_plan(rfi: &Rfi, space: ColorSpace, flags: u32, settings: &TextureSettings) -> UploadPlan {
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
    UploadPlan {
        word,
        srgb,
        first,
        mipmapped,
        compressed,
        chain,
    }
}

/// A cube map as the original uploads it: six faces, each from the chosen
/// detail level, in face order with each face's levels back to back. A
/// stored chain is sliced per face; a cube without one gets the Kaiser chain
/// generated per face, and a block-compressed cube is refused (no shipped
/// image is one; the original's per-face block upload is not decoded here).
fn cube_to_texture(
    rfi: &Rfi,
    space: ColorSpace,
    flags: u32,
    settings: &TextureSettings,
) -> exa_formats::Result<Image> {
    use bevy::render::render_resource::{TextureViewDescriptor, TextureViewDimension};

    let UploadPlan {
        word,
        srgb,
        first,
        mipmapped,
        compressed,
        chain,
    } = upload_plan(rfi, space, flags, settings);
    let (w, h) = (rfi.header.width, rfi.header.height);
    if compressed {
        return Err(exa_formats::Error::Unsupported(
            "a block-compressed cube map".into(),
        ));
    }
    let mut first = if chain {
        first.min(rfi.mip_count.saturating_sub(1))
    } else {
        first
    };
    let mut data = Vec::new();
    let mut levels = 1u32;
    for face in 0..rfi.header.faces() {
        let decoded = rfi.decode_face_mips_rgba8(face)?;
        if chain {
            let last = if mipmapped {
                rfi.mip_count.saturating_sub(1)
            } else {
                first
            };
            let mut offset = 0usize;
            for level in 0..=last {
                let (lw, lh) = level_dims(w, h, level);
                let size = (lw as usize) * (lh as usize) * 4;
                if level >= first {
                    data.extend_from_slice(decoded.get(offset..offset + size).ok_or(
                        exa_formats::Error::Truncated {
                            offset,
                            needed: size,
                            len: decoded.len(),
                        },
                    )?);
                }
                offset += size;
            }
            levels = last + 1 - first;
        } else {
            // A single stored level is filtered up from that base, as the
            // 2D path does.
            let last = if mipmapped { u32::MAX } else { first };
            let options = exa_render::mipmap::ChainOptions::for_image(
                rfi.header.format_word,
                rfi.header.flags,
                first,
                last,
            );
            let (chain, generated) = exa_render::mipmap::build_mip_chain_rgba8_with(
                &decoded, w as usize, h as usize, &options,
            );
            if generated > 0 {
                data.extend(chain);
                levels = generated;
            } else {
                // A skip past the end of a tiny image: keep the stored base
                // and no skip, as the 2D path does.
                first = 0;
                levels = 1;
                data.extend_from_slice(&decoded);
            }
        }
    }
    let mut image = Image::new_uninit(
        Extent3d {
            width: (w >> first.min(31)).max(1),
            height: (h >> first.min(31)).max(1),
            depth_or_array_layers: rfi.header.faces(),
        },
        TextureDimension::D2,
        if srgb {
            TextureFormat::Rgba8UnormSrgb
        } else {
            TextureFormat::Rgba8Unorm
        },
        RenderAssetUsages::RENDER_WORLD,
    );
    image.texture_descriptor.mip_level_count = levels;
    image.data = Some(data);
    image.texture_view_descriptor = Some(TextureViewDescriptor {
        dimension: Some(TextureViewDimension::Cube),
        ..default()
    });
    image.sampler =
        ImageSampler::Descriptor(TextureSampler::from_flags(word, settings).descriptor());
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

/// Whether the image carries an alpha channel worth alpha-testing.
pub fn has_alpha(rfi: &Rfi) -> bool {
    matches!(
        rfi.format,
        PixelFormat::Bc1Alpha
            | PixelFormat::Bc3
            | PixelFormat::Bc4Alpha
            | PixelFormat::Rgba8
            | PixelFormat::Bgra8
            | PixelFormat::Zyxv8
            | PixelFormat::La8
            | PixelFormat::A8
    )
}

/// Whether the upload is the single grey channel of a BC4 image: the
/// original's luminance LATC format, whose red channel stands for the whole
/// colour and which no sRGB format decodes. The upload takes this path when
/// the GPU supports BC and the image is block aligned, exactly as
/// [`rfi_to_texture`] does.
pub fn is_grey_upload(rfi: &Rfi, bc_supported: bool) -> bool {
    matches!(rfi.format, PixelFormat::Bc4)
        && bc_supported
        && rfi.header.width.is_multiple_of(4)
        && rfi.header.height.is_multiple_of(4)
        && !rfi.is_packed()
}

/// A render parameter stored in hundredths (an `i16`) as the float the
/// shader uses: the value widened to double, times 0.01, narrowed to f32.
pub fn hundredths_to_f32(value: i16) -> f32 {
    (f64::from(value) * 0.01) as f32
}

/// A float render parameter as its stored hundredths: `value · 100` in f32
/// converted with the processor's `CVTSS2SI` (nearest, ties to even); a NaN
/// or out-of-range value gives the indefinite integer, whose low word is 0.
/// The stored field is 16 bits, so a finite result keeps its low word.
pub fn f32_to_hundredths(value: f32) -> i16 {
    exa_formats::pixel::round_f32(value * 100.0) as i16
}

/// One texture as the unit cache sees it: the identity the cache compares,
/// its GL target, its GL name and its word.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextureBinding {
    /// What the cache compares: the texture object itself, not its GL name,
    /// so a texture whose name changed is not rebound until another texture
    /// has used the unit.
    pub object: u64,
    /// The GL target (`TEXTURE_2D`, ...), kept per unit.
    pub target: u32,
    /// The GL name; 0 means the object has no GL texture, which clears the
    /// unit like binding nothing.
    pub name: u32,
    pub flags: u32,
}

/// What a bind did. `None` fields are GL calls the bind skipped.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct BindOps {
    /// The unit made active.
    pub active_unit: Option<u32>,
    /// `(target, name)` of the texture bound.
    pub texture: Option<(u32, u32)>,
    /// The target whose LOD bias was set to zero.
    pub lod_bias_reset: Option<u32>,
}

impl BindOps {
    pub fn is_empty(&self) -> bool {
        self.active_unit.is_none() && self.texture.is_none() && self.lod_bias_reset.is_none()
    }
}

/// The per-unit texture bind cache. Each unit remembers the texture object
/// last bound and its target; the cache also remembers the active unit,
/// which starts at 0. Binding the object already on the unit does nothing.
/// Otherwise the unit is made active when it is not already, then a texture
/// with a GL name is bound (target and name) and its LOD bias parameter is
/// set to zero unless the word has bit `0x200`; a missing texture, or one
/// without a GL name, only clears the unit's entry, with no GL call. There is
/// no sampler object: the filters and wraps are texture parameters set when
/// the texture was created.
///
/// The original has a second path for a negative unit (enabling and
/// disabling the fixed-function target); callers never pass one, so it is
/// not modelled.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TextureUnitCache {
    active: u32,
    units: Vec<Option<(u64, u32)>>,
}

impl TextureUnitCache {
    pub fn new() -> Self {
        Self::default()
    }

    /// The unit last made active.
    pub fn active_unit(&self) -> u32 {
        self.active
    }

    /// The texture object and target remembered for `unit`.
    pub fn bound(&self, unit: u32) -> Option<(u64, u32)> {
        self.units.get(unit as usize).copied().flatten()
    }

    /// Binds `texture` (or nothing) to `unit`.
    pub fn bind(&mut self, unit: u32, texture: Option<TextureBinding>) -> BindOps {
        let slot = unit as usize;
        if self.units.len() <= slot {
            self.units.resize(slot + 1, None);
        }
        // A missing texture compares equal to an empty unit.
        if self.units[slot].map(|(object, _)| object) == texture.map(|t| t.object) {
            return BindOps::default();
        }
        let mut ops = BindOps::default();
        if self.active != unit {
            self.active = unit;
            ops.active_unit = Some(unit);
        }
        match texture {
            Some(binding) if binding.name != 0 => {
                ops.texture = Some((binding.target, binding.name));
                if binding.flags & flags::NO_LOD_BIAS == 0 {
                    ops.lod_bias_reset = Some(binding.target);
                }
                self.units[slot] = Some((binding.object, binding.target));
            }
            _ => self.units[slot] = None,
        }
        ops
    }
}

#[cfg(test)]
mod sampler_tests {
    use super::*;

    #[test]
    fn the_lod_bias_follows_the_supersampling_setting() {
        // base · (f · ⅓) in f32, the original's instruction order, with
        // f = ss below 2 and ss - 1 from 2 up.
        assert_eq!(lod_bias(-0.75, 0), 0.0);
        assert_eq!(lod_bias(-0.75, 1), -0.25);
        assert_eq!(lod_bias(-0.75, 2), -0.25);
        assert_eq!(lod_bias(-0.75, 3), -0.5);
        assert_eq!(lod_bias(-0.75, 4), -0.75);
        assert_eq!(lod_bias(-0.5, 4), -0.5);
        // The order is visible where the product is not exactly one:
        // -0.007 · (3 · ⅓) is exactly -0.007 (3 · ⅓ rounds to 1), where
        // (-0.007 · 3) · ⅓ is -0.0070000006.
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
    fn the_screen_settings_write_quality_detail_and_supersampling() {
        // The settings screen's live values; the device limits stay.
        let mut settings = TextureSettings {
            texture_detail: 2,
            supersample: 4,
            ..TextureSettings::default().with_quality(4)
        };
        settings.apply_driver_limits(16_384, 8);
        assert_eq!(settings.quality_mask, 0xF0);
        assert_eq!(settings.anisotropy_level, 2);
        assert_eq!(settings.lod_bias_base, -0.5);
        assert_eq!(settings.texture_detail, 2);
        assert_eq!(settings.supersample, 4);
        assert_eq!(settings.lod_bias(), -0.5);
        assert_eq!(settings.max_texture_size, 16_384);
        assert_eq!(settings.max_units, 8);
        let top = TextureSettings::default().with_quality(5);
        assert_eq!(top.lod_bias(), -0.75);
    }

    #[test]
    fn the_target_comes_from_the_top_kind_bits() {
        // The original's switch: 0x10000000 is GL_TEXTURE_1D, 0x30000000
        // GL_TEXTURE_3D, 0x40000000 the cube, and everything else (0,
        // 0x20000000 and 0x50000000 up) GL_TEXTURE_2D.
        assert_eq!(TextureTarget::from_flags(0), TextureTarget::D2);
        assert_eq!(
            TextureTarget::from_flags(flags::TARGET_1D),
            TextureTarget::D1
        );
        assert_eq!(
            TextureTarget::from_flags(flags::TARGET_2D_ALT),
            TextureTarget::D2
        );
        assert_eq!(
            TextureTarget::from_flags(flags::TARGET_3D),
            TextureTarget::D3
        );
        assert_eq!(
            TextureTarget::from_flags(flags::TARGET_CUBE),
            TextureTarget::Cube
        );
        assert_eq!(TextureTarget::from_flags(0x5000_0000), TextureTarget::D2);
        assert_eq!(TextureTarget::from_flags(0x7000_0000), TextureTarget::D2);
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
    fn the_compatible_format_table_has_no_srgb_alpha_or_two_channel() {
        // The seven uncompressed formats the original uploads as they are,
        // compared under the mask that drops the data-kind bits.
        assert_eq!(COMPATIBLE_MASK, 0x800C_FFFF);
        assert!(has_srgb_format(0x0100_C600));
        assert!(has_srgb_format(0x0100_E608));
        assert!(has_srgb_format(0x0100_4200));
        assert!(!has_srgb_format(0x0100_2008));
        assert!(has_srgb_format(0x0100_6208));
        assert!(!has_srgb_format(0x0100_8400));
        assert!(!has_srgb_format(0x1111_8400));
        // The packed data words share the RGB8 entry, but stay data.
        assert_eq!(0x0111_C600 & COMPATIBLE_MASK, 0x0000_C600);
        assert_eq!(0x1111_C600 & COMPATIBLE_MASK, 0x0000_C600);
        // An alpha-only image is uploaded linear, as its table row has the
        // same format in both columns.
        assert!(!is_srgb(ColorSpace::Srgb, 0, 0x0100_2008));
        assert!(is_srgb(ColorSpace::Srgb, 0, 0x0100_6208));
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
mod unit_cache_tests {
    use super::*;

    fn binding(object: u64, name: u32, flags: u32) -> TextureBinding {
        TextureBinding {
            object,
            target: 0x0DE1,
            name,
            flags,
        }
    }

    #[test]
    fn a_bind_skips_the_object_already_on_the_unit() {
        let mut cache = TextureUnitCache::new();
        // The active unit starts at 0, so unit 0 needs no selection.
        let first = cache.bind(0, Some(binding(1, 10, 0)));
        assert_eq!(first.active_unit, None);
        assert_eq!(first.texture, Some((0x0DE1, 10)));
        assert_eq!(first.lod_bias_reset, Some(0x0DE1));
        assert!(cache.bind(0, Some(binding(1, 10, 0))).is_empty());
        // The comparison is the object: a changed GL name is not rebound.
        assert!(cache.bind(0, Some(binding(1, 11, 0))).is_empty());
        // Another unit is selected once, then stays active.
        let second = cache.bind(2, Some(binding(7, 12, 0)));
        assert_eq!(second.active_unit, Some(2));
        assert_eq!(cache.active_unit(), 2);
        let third = cache.bind(2, Some(binding(8, 13, 0)));
        assert_eq!(third.active_unit, None);
        assert_eq!(third.texture, Some((0x0DE1, 13)));
        // Going back selects the unit again.
        assert_eq!(cache.bind(0, Some(binding(9, 14, 0))).active_unit, Some(0));
    }

    #[test]
    fn the_skip_bias_word_suppresses_only_the_bias_call() {
        let mut cache = TextureUnitCache::new();
        let ops = cache.bind(0, Some(binding(3, 6, flags::NO_LOD_BIAS)));
        assert_eq!(ops.texture, Some((0x0DE1, 6)));
        assert_eq!(ops.lod_bias_reset, None);
        assert!(!ops.is_empty());
    }

    #[test]
    fn nothing_and_a_nameless_texture_only_clear_the_unit() {
        let mut cache = TextureUnitCache::new();
        cache.bind(1, Some(binding(4, 8, 0)));
        // Clearing makes no GL bind.
        let ops = cache.bind(1, None);
        assert_eq!((ops.texture, ops.lod_bias_reset), (None, None));
        assert_eq!(cache.bound(1), None);
        assert!(cache.bind(1, None).is_empty());
        // A texture object with no GL name clears just the same.
        cache.bind(1, Some(binding(5, 9, 0)));
        let ops = cache.bind(1, Some(binding(6, 0, 0)));
        assert_eq!(ops.texture, None);
        assert_eq!(cache.bound(1), None);
        // Clearing an empty unit made active by nothing else does nothing.
        assert!(cache.bind(5, None).is_empty());
        assert_eq!(cache.active_unit(), 1);
    }

    #[test]
    fn render_parameters_store_hundredths() {
        assert_eq!(hundredths_to_f32(1234), 12.34);
        assert_eq!(hundredths_to_f32(-5), -0.05);
        assert_eq!(f32_to_hundredths(12.34), 1234);
        // Ties go to even: 0.125 · 100 = 12.5 → 12.
        assert_eq!(f32_to_hundredths(0.125), 12);
        assert_eq!(f32_to_hundredths(0.135), 14);
        // NaN gives the indefinite integer's low word.
        assert_eq!(f32_to_hundredths(f32::NAN), 0);
        // A finite value past i16 keeps the low 16 bits.
        assert_eq!(f32_to_hundredths(400.0), (40000i32 & 0xffff) as i16);
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

    /// A header and its pixel data as `.rfi` bytes.
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

    /// A BC4 image uploads as one grey channel, the original's luminance
    /// LATC format: no sRGB variant, so nothing decodes it. Without BC
    /// support (or when unaligned) it decodes to RGBA8 instead.
    #[test]
    fn a_bc4_image_uploads_as_one_grey_channel() {
        // 8 x 8, four 8-byte blocks.
        let header = RfiHeader {
            width: 8,
            height: 8,
            depth: 1,
            format_word: 0x813B_4200,
            flags: 0,
            storage: 0x1000_0000,
            data_size: 32,
        };
        let bytes = rfi_bytes(&header, &[0u8; 32]);
        let rfi = Rfi::parse(&bytes).unwrap();
        assert!(is_grey_upload(&rfi, true));
        assert!(!is_grey_upload(&rfi, false));
        let image = rfi_to_texture(
            &rfi,
            ColorSpace::Srgb,
            true,
            default_flags(),
            &TextureSettings::default(),
        )
        .unwrap();
        assert_eq!(image.texture_descriptor.format, TextureFormat::Bc4RUnorm);
        assert_eq!(image.texture_descriptor.size.width, 8);
        let decoded = rfi_to_texture(
            &rfi,
            ColorSpace::Srgb,
            false,
            default_flags(),
            &TextureSettings::default(),
        )
        .unwrap();
        assert_eq!(
            decoded.texture_descriptor.format,
            TextureFormat::Rgba8UnormSrgb
        );
        // An unaligned side cannot be a BC block texture.
        let unaligned = RfiHeader {
            width: 6,
            height: 8,
            ..header
        };
        let bytes = rfi_bytes(&unaligned, &[0u8; 32]);
        let rfi = Rfi::parse(&bytes).unwrap();
        assert!(!is_grey_upload(&rfi, true));
    }

    /// The loader's pure decode: the image and its alpha flag, or nothing
    /// for bytes that are not an image.
    #[test]
    fn a_named_image_decodes_for_an_upload() {
        let (data, header) = rgba_image(2, 2, 1, 0);
        let bytes = rfi_bytes(&header, &data);
        let settings = TextureSettings::default();
        let (image, alpha) =
            decode_uploaded(&bytes, ColorSpace::Srgb, default_flags(), &settings, true).unwrap();
        assert!(alpha, "RGBA8 carries alpha");
        assert_eq!(image.texture_descriptor.size.width, 2);
        assert_eq!(image.texture_descriptor.mip_level_count, 2);
        assert!(decode_uploaded(b"not an image", ColorSpace::Srgb, 0, &settings, true).is_none());
    }

    /// A cube map: six faces, each a 2x2 RGB8 base filled with its face
    /// number and a 1x1 level holding 40 + the face number.
    fn cube_image_bytes(flags: u32) -> Vec<u8> {
        let mut data = Vec::new();
        for face in 0..6u8 {
            for pixel in 0..4u8 {
                data.extend_from_slice(&[face, pixel * 10, 0]);
            }
            data.extend_from_slice(&[face, 40, 0]);
        }
        let header = RfiHeader {
            width: 2,
            height: 2,
            depth: 1,
            format_word: 0x0100_C600,
            flags: flags | exa_formats::rfi::FLAG_CUBE | exa_formats::rfi::FLAG_MIP_CHAIN,
            storage: 0x1000_0000,
            data_size: 15,
        };
        rfi_bytes(&header, &data)
    }

    /// A cube target uploads six faces with their chains, sliced at the
    /// detail level; its view says cube, and every face's texels are in
    /// face order.
    #[test]
    fn a_cube_map_uploads_six_faces() {
        use bevy::render::render_resource::TextureViewDimension;
        let bytes = cube_image_bytes(0);
        let rfi = Rfi::parse(&bytes).unwrap();
        assert_eq!(rfi.mip_count, 2);
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
        assert_eq!(image.texture_descriptor.size.depth_or_array_layers, 6);
        assert_eq!(image.texture_descriptor.mip_level_count, 2);
        let view = image.texture_view_descriptor.as_ref().unwrap();
        assert_eq!(view.dimension, Some(TextureViewDimension::Cube));
        let data = image.data.as_ref().unwrap();
        // Each face is its 2x2 base (16 bytes) then its 1x1 level (4).
        assert_eq!(data.len(), 6 * (16 + 4));
        assert_eq!(&data[..4], &[0, 0, 0, 255]);
        assert_eq!(&data[16..20], &[0, 40, 0, 255]);
        assert_eq!(&data[20..24], &[1, 0, 0, 255]);
        assert_eq!(&data[116..120], &[5, 40, 0, 255]);
        // Without mipmapping each face is its base alone.
        let image = rfi_to_texture(
            &rfi,
            ColorSpace::Srgb,
            false,
            default_flags() & !flags::MIPMAP,
            &TextureSettings::default(),
        )
        .unwrap();
        assert_eq!(image.texture_descriptor.mip_level_count, 1);
        assert_eq!(image.data.as_ref().unwrap().len(), 6 * 16);
    }

    /// A cube with a single stored level is filtered per face from that
    /// base to the detail level, as the 2D path does.
    #[test]
    fn a_single_level_cube_is_generated_per_face() {
        let mut data = Vec::new();
        for face in 0..6u8 {
            for _ in 0..4 {
                data.extend_from_slice(&[face, 50, 0]);
            }
        }
        let header = RfiHeader {
            width: 2,
            height: 2,
            depth: 1,
            format_word: 0x0100_C600,
            flags: exa_formats::rfi::FLAG_CUBE | (3 << 22),
            storage: 0x1000_0000,
            data_size: 12,
        };
        let bytes = rfi_bytes(&header, &data);
        let rfi = Rfi::parse(&bytes).unwrap();
        let settings = TextureSettings {
            texture_detail: 2,
            ..TextureSettings::default()
        };
        let image =
            rfi_to_texture(&rfi, ColorSpace::Srgb, false, default_flags(), &settings).unwrap();
        assert_eq!(image.texture_descriptor.size.width, 1);
        assert_eq!(image.texture_descriptor.size.height, 1);
        assert_eq!(image.texture_descriptor.size.depth_or_array_layers, 6);
        assert_eq!(image.texture_descriptor.mip_level_count, 1);
        let data = image.data.as_ref().unwrap();
        assert_eq!(data.len(), 6 * 4);
        for face in 0..6u8 {
            assert_eq!(
                &data[face as usize * 4..face as usize * 4 + 4],
                &[face, 50, 0, 255]
            );
        }
    }

    /// The detail setting slices each face's chain at the chosen level, and
    /// a level past the chain clamps to its smallest one.
    #[test]
    fn a_cube_map_starts_at_its_detail_level() {
        let settings = TextureSettings {
            texture_detail: 2,
            ..TextureSettings::default()
        };
        let bytes = cube_image_bytes(3 << 22);
        let rfi = Rfi::parse(&bytes).unwrap();
        let image =
            rfi_to_texture(&rfi, ColorSpace::Srgb, false, default_flags(), &settings).unwrap();
        // Class 3, detail 2 asks for level 2; the chain has two levels, so
        // the smallest one is the base.
        assert_eq!(image.texture_descriptor.size.width, 1);
        assert_eq!(image.texture_descriptor.mip_level_count, 1);
        let data = image.data.as_ref().unwrap();
        assert_eq!(data.len(), 6 * 4);
        for face in 0..6u8 {
            assert_eq!(
                &data[face as usize * 4..face as usize * 4 + 4],
                &[face, 40, 0, 255]
            );
        }
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
        let mut grey = 0usize;
        let mut images = 0usize;
        // The generated chains, and how many filter with the uniform bank,
        // clamp on an axis or renormalise.
        let mut generated = [0usize; 4];
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
                if !compressed && !rfi.header.has_mip_chain() {
                    let options = exa_render::mipmap::ChainOptions::for_image(
                        rfi.header.format_word,
                        rfi.header.flags,
                        first,
                        u32::MAX,
                    );
                    generated[0] += 1;
                    generated[1] +=
                        usize::from(options.kernel == exa_render::mipmap::MipKernel::Box);
                    generated[2] += usize::from(options.wrap != [true, true]);
                    generated[3] += usize::from(options.renormalise);
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
                // A BC4 image is the one single-channel upload: its format
                // says grey and nothing else does.
                let is_grey = is_grey_upload(&rfi, true);
                assert_eq!(
                    is_grey,
                    image.texture_descriptor.format == TextureFormat::Bc4RUnorm,
                    "{}",
                    entry.name
                );
                grey += usize::from(is_grey);
            }
        }
        eprintln!(
            "{images} images, {grey} single grey channels (BC4), \
             classes {classes:?}, {stored_chain} with a stored chain, \
             {unchained_compressed} compressed without one, \
             {later} start at a later level at detail {}; generated chains \
             [all, box bank, clamped on an axis, renormalised] {generated:?}; {:?}",
            settings.texture_detail,
            start.elapsed()
        );
        assert!(images > 1000);
    }
}
