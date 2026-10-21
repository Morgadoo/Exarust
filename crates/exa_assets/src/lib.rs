//! Bridge between an Exanima installation and Bevy.
//!
//! Holds the opened [`GameData`] as a resource and converts parsed formats
//! into Bevy assets: images ([`texture`]) and models with their materials
//! ([`model`]), each cached by name.

pub mod content_stream;
pub mod hair;
pub mod human_skin;
pub mod merge;
pub mod mesh_edit;
pub mod mesh_smooth;
pub mod mesh_weld;
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
pub mod texture_stream;
pub mod vertex_cache;
pub mod weapon;

use std::sync::Arc;

use bevy::image::CompressedImageFormatSupport;
use bevy::prelude::*;
use exa_formats::rfi::Rfi;
pub use exa_rpk::{Archive, GameData, PackKind, locate_install};

pub use model::{
    Model, ModelCache, ModelLoader, ModelNode, NodeRigidBody, TextureInfo, transform_from_rows,
};
pub use morph::BodyMorph;
pub use texture::ColorSpace;
pub use texture_cache::{TextureCache, TextureKey, TextureName, TextureRecord};
pub use texture_stream::{TextureLoader, TextureStreamPlugin};

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
        // The running game decodes textures on the loader's queue with a
        // placeholder until the upload arrives, as the original does; tools
        // and tests that build their own cache keep decoding where they ask.
        let mut cache = ModelCache::default();
        cache.textures.set_streaming(true);
        app.insert_resource(cache);
        app.add_plugins(TextureStreamPlugin);
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

/// Raw RGBA pixels as an `.rfi` decodes them: rows bottom-up, as the
/// original's OpenGL textures store them.
#[derive(Clone, PartialEq, Eq)]
pub struct RgbaImage {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

impl std::fmt::Debug for RgbaImage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RgbaImage")
            .field("width", &self.width)
            .field("height", &self.height)
            .field("pixels", &self.pixels.len())
            .finish()
    }
}

/// The pictures the power tree draws: the six domain icon sheets, the
/// frames, the modifier marks, the flamelets and the domain wheel. A
/// missing sheet stays empty.
#[derive(Debug, Clone, Default)]
pub struct TreeSheets {
    /// Index 0 is unused; 1 to 6 are the domain sheets.
    pub domains: [Option<RgbaImage>; 7],
    pub frames: Option<RgbaImage>,
    pub mods: Option<RgbaImage>,
    pub flamelets: Option<RgbaImage>,
    pub wheel: Option<RgbaImage>,
}

impl Game {
    /// An `.rfi` as raw RGBA, with its rows bottom-up.
    pub fn load_rgba(&self, name: &str) -> Result<RgbaImage, String> {
        let bytes = self
            .0
            .texture(name)
            .ok_or_else(|| format!("texture {name:?} not found"))?;
        let rfi = Rfi::parse(bytes).map_err(|e| e.to_string())?;
        let pixels = rfi.decode_rgba8().map_err(|e| e.to_string())?;
        Ok(RgbaImage {
            width: rfi.header.width,
            height: rfi.header.height,
            pixels,
        })
    }

    /// The power tree's pictures, by the original's names.
    pub fn load_tree_sheets(&self) -> TreeSheets {
        const DOMAINS: [&str; 7] = [
            "",
            "thmmndico1",
            "thmfrcico1",
            "thmbdyico1",
            "thmnrgico1",
            "thmlgtico1",
            "thmdspico1",
        ];
        let mut sheets = TreeSheets::default();
        for (index, name) in DOMAINS.iter().enumerate().skip(1) {
            sheets.domains[index] = self.load_rgba(name).ok();
        }
        sheets.frames = self.load_rgba("pwrsframes").ok();
        sheets.mods = self.load_rgba("pwrsmods").ok();
        sheets.flamelets = self.load_rgba("flameletsa1").ok();
        sheets.wheel = self.load_rgba("domainwheel").ok();
        sheets
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_tree_sheets_load() {
        let Some(dir) = crate::locate_install(None) else {
            return;
        };
        let game = crate::Game(std::sync::Arc::new(
            crate::GameData::open(dir).expect("the install opens"),
        ));
        let sheets = game.load_tree_sheets();
        let mind = sheets.domains[1].as_ref().expect("the mind sheet");
        assert_eq!((mind.width, mind.height), (2048, 2048));
        assert_eq!(mind.pixels.len(), 2048 * 2048 * 4);
        let force = sheets.domains[2].as_ref().expect("the force sheet");
        assert_eq!((force.width, force.height), (2048, 2048));
        let frames = sheets.frames.as_ref().expect("the frame sheet");
        assert_eq!((frames.width, frames.height), (128, 256));
        let mods = sheets.mods.as_ref().expect("the modifier sheet");
        assert_eq!((mods.width, mods.height), (1024, 512));
        let flamelets = sheets.flamelets.as_ref().expect("the flamelets");
        assert_eq!((flamelets.width, flamelets.height), (1024, 1024));
        let wheel = sheets.wheel.as_ref().expect("the domain wheel");
        assert_eq!((wheel.width, wheel.height), (384, 384));
    }
}
