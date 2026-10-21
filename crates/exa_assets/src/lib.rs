//! Bridge between an Exanima installation and Bevy.
//!
//! Holds the opened [`GameData`] as a resource and converts parsed formats
//! into Bevy assets: images ([`texture`]) and models with their materials
//! ([`model`]), each cached by name.

pub mod content_stream;
pub mod human_skin;
pub mod mesh_edit;
pub mod mesh_smooth;
pub mod model;
pub mod morph;
pub mod morph_controller;
pub mod paint;
pub mod polyhedra;
pub mod skin;
pub mod stream;
pub mod tangents;
pub mod texture;
pub mod texture_cache;
pub mod vertex_cache;
pub mod weapon;

use std::sync::Arc;

use bevy::image::CompressedImageFormatSupport;
use bevy::prelude::*;
use exa_formats::rfi::Rfi;
pub use exa_rpk::{GameData, PackKind, locate_install};

pub use model::{Model, ModelCache, ModelLoader, ModelNode, NodeRigidBody, transform_from_rows};
pub use morph::BodyMorph;
pub use texture::ColorSpace;
pub use texture_cache::{TextureCache, TextureKey, TextureName, TextureRecord};

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

/// A part drawn with a race's built-in skin ([`human_skin`]), whose tone a
/// character may change.
#[derive(bevy::prelude::Component, Debug, Clone, Copy, Default)]
pub struct RaceSkinPart;

/// A character's skin: the race stem whose built-in skin it wears (a body
/// model entry without its `base.rfc`) and its tone coordinates, so a body
/// can be re-tinted after it is spawned. A body without it keeps the middle
/// tone.
#[derive(bevy::prelude::Component, Debug, Clone, PartialEq, Eq)]
pub struct SkinLook {
    pub stem: String,
    pub tone: [u16; 2],
    /// The women's texture set instead of the men's.
    pub female: bool,
}

impl SkinLook {
    /// The look of a body model entry (`humanbase.rfc` → `human`).
    pub fn of_entry(entry: &str, tone: [u16; 2], female: bool) -> Self {
        let lower = entry.to_ascii_lowercase();
        let stem = lower.strip_suffix("base.rfc").unwrap_or(&lower).to_string();
        Self { stem, tone, female }
    }

    /// The body model entry whose skin this is.
    pub fn entry(&self) -> String {
        format!("{}base.rfc", self.stem)
    }
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
