//! Bridge between an Exanima installation and Bevy.
//!
//! Holds the opened [`GameData`] as a resource and converts parsed formats
//! into Bevy assets: images ([`texture`]) and models with their materials
//! ([`model`]), each cached by name.

pub mod human_skin;
pub mod model;
pub mod morph;
pub mod morph_controller;
pub mod paint;
pub mod skin;
pub mod texture;
pub mod weapon;

use std::sync::Arc;

use bevy::image::CompressedImageFormatSupport;
use bevy::prelude::*;
use exa_formats::rfi::Rfi;
pub use exa_rpk::{GameData, PackKind, locate_install};

pub use model::{Model, ModelCache, ModelLoader, ModelNode, NodeRigidBody, transform_from_rows};
pub use morph::BodyMorph;
pub use texture::ColorSpace;

/// A mesh whose vertices carry skin weights over a character's bones, in
/// the space of body part `bone`; bound to the skeleton once it is known.
///
/// The entity gets its `Mesh3d` only when it is bound: drawn before then,
/// its skinned vertex layout would meet Bevy's unskinned bind group.
#[derive(bevy::prelude::Component, Debug, Clone)]
pub struct SkinnedPart {
    pub bone: u16,
    pub mesh: Handle<Mesh>,
}

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
