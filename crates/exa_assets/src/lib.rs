//! Bridge between an Exanima installation and Bevy.
//!
//! Holds the opened [`GameData`] as a resource and converts parsed formats
//! into Bevy assets. Conversions currently decode on the CPU; uploading block
//! compressed data directly and an `exanima://` asset source are Phase 1 work
//! in `docs/PLAN.md`.

use std::sync::Arc;

use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use exa_formats::rfi::Rfi;
pub use exa_rpk::{GameData, PackKind, locate_install};

pub struct AssetsPlugin;

impl Plugin for AssetsPlugin {
    fn build(&self, _app: &mut App) {}
}

/// The installation the app was started against.
#[derive(Resource, Clone)]
pub struct Game(pub Arc<GameData>);

/// Packs searched, in order, when a texture is referenced by bare name.
const TEXTURE_PACKS: [PackKind; 5] = [
    PackKind::Textures,
    PackKind::Apparel,
    PackKind::ComponentTex,
    PackKind::Emblems,
    PackKind::Resource,
];

impl Game {
    pub fn texture_bytes(&self, name: &str) -> Option<&[u8]> {
        let with_ext = format!("{name}.rfi");
        TEXTURE_PACKS.iter().find_map(|&kind| {
            let pack = self.0.pack(kind)?;
            pack.get(name).or_else(|| pack.get(&with_ext))
        })
    }

    pub fn load_texture(&self, name: &str, srgb: bool) -> Result<Image, String> {
        let bytes = self
            .texture_bytes(name)
            .ok_or_else(|| format!("texture {name:?} not found"))?;
        let rfi = Rfi::parse(bytes).map_err(|e| e.to_string())?;
        rfi_to_image(&rfi, srgb).map_err(|e| e.to_string())
    }
}

pub fn rfi_to_image(rfi: &Rfi, srgb: bool) -> exa_formats::Result<Image> {
    let rgba = rfi.decode_rgba8()?;
    Ok(Image::new(
        Extent3d {
            width: rfi.header.width,
            height: rfi.header.height,
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
    ))
}
