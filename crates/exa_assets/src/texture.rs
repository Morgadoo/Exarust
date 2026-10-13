//! `.rfi` images to Bevy images.
//!
//! Block-compressed images are uploaded as-is with their mip chain when the
//! GPU supports BC formats and the base level is block aligned; everything
//! else is decoded to RGBA8 on the CPU (base level only).

use bevy::asset::RenderAssetUsages;
use bevy::image::{ImageAddressMode, ImageFilterMode, ImageSampler, ImageSamplerDescriptor};
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use exa_formats::rfi::{PixelFormat, Rfi, level_dims};

/// How an image's colour should be interpreted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ColorSpace {
    Srgb,
    Linear,
}

pub fn rfi_to_image(
    rfi: &Rfi,
    space: ColorSpace,
    bc_supported: bool,
) -> exa_formats::Result<Image> {
    let w = rfi.header.width;
    let h = rfi.header.height;
    let srgb = space == ColorSpace::Srgb;
    let block_format = match rfi.format {
        PixelFormat::Bc1 if srgb => Some(TextureFormat::Bc1RgbaUnormSrgb),
        PixelFormat::Bc1 => Some(TextureFormat::Bc1RgbaUnorm),
        PixelFormat::Bc3 if srgb => Some(TextureFormat::Bc3RgbaUnormSrgb),
        PixelFormat::Bc3 => Some(TextureFormat::Bc3RgbaUnorm),
        // Two-channel normal maps: Bevy reconstructs Z for RG formats.
        PixelFormat::Bc5 if !srgb => Some(TextureFormat::Bc5RgUnorm),
        _ => None,
    };
    let mut image = match block_format {
        Some(format)
            if bc_supported && w.is_multiple_of(4) && h.is_multiple_of(4) && !rfi.is_packed() =>
        {
            let pixels = rfi.pixels()?;
            let mut len = 0;
            for level in 0..rfi.mip_count {
                let (lw, lh) = level_dims(w, h, level);
                len += rfi.format.level_size(lw, lh).unwrap_or(0);
            }
            let mut image = Image::new_uninit(
                Extent3d {
                    width: w,
                    height: h,
                    depth_or_array_layers: 1,
                },
                TextureDimension::D2,
                format,
                RenderAssetUsages::RENDER_WORLD,
            );
            image.texture_descriptor.mip_level_count = rfi.mip_count;
            image.data = Some(pixels[..len.min(pixels.len())].to_vec());
            image
        }
        _ => {
            let rgba = rfi.decode_rgba8()?;
            Image::new(
                Extent3d {
                    width: w,
                    height: h,
                    depth_or_array_layers: 1,
                },
                TextureDimension::D2,
                rgba,
                if srgb {
                    TextureFormat::Rgba8UnormSrgb
                } else {
                    TextureFormat::Rgba8Unorm
                },
                RenderAssetUsages::RENDER_WORLD,
            )
        }
    };
    image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
        address_mode_u: ImageAddressMode::Repeat,
        address_mode_v: ImageAddressMode::Repeat,
        address_mode_w: ImageAddressMode::Repeat,
        mag_filter: ImageFilterMode::Linear,
        min_filter: ImageFilterMode::Linear,
        mipmap_filter: ImageFilterMode::Linear,
        anisotropy_clamp: 8,
        ..ImageSamplerDescriptor::linear()
    });
    Ok(image)
}

/// Whether the image carries an alpha channel worth alpha-testing.
pub fn has_alpha(rfi: &Rfi) -> bool {
    matches!(
        rfi.format,
        PixelFormat::Bc3 | PixelFormat::Rgba8 | PixelFormat::La8 | PixelFormat::A8
    )
}
