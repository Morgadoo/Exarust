//! Bridge between an Exanima installation and Bevy.
//!
//! Holds the opened [`GameData`] as a resource and converts parsed formats
//! into Bevy assets: images ([`texture`]) and models with their materials
//! ([`model`]), each cached by name.

pub mod model;
pub mod paint;
pub mod texture;

use std::sync::Arc;

use bevy::image::CompressedImageFormatSupport;
use bevy::prelude::*;
use exa_formats::rfi::Rfi;
pub use exa_rpk::{GameData, PackKind, locate_install};

pub use model::{Model, ModelCache, ModelLoader, ModelNode, transform_from_rows};
pub use texture::ColorSpace;

pub struct AssetsPlugin;

impl Plugin for AssetsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ModelCache>();
    }
}

/// The installation the app was started against.
#[derive(Resource, Clone)]
pub struct Game(pub Arc<GameData>);

impl Game {
    pub fn load_texture(
        &self,
        name: &str,
        space: ColorSpace,
        support: Option<&CompressedImageFormatSupport>,
    ) -> Result<Image, String> {
        let bytes = self
            .0
            .texture(name)
            .ok_or_else(|| format!("texture {name:?} not found"))?;
        let rfi = Rfi::parse(bytes).map_err(|e| e.to_string())?;
        texture::rfi_to_image(&rfi, space, model::bc_supported(support)).map_err(|e| e.to_string())
    }
}
