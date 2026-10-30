//! Bridge between an Exanima installation and Bevy.
//!
//! Holds the opened [`GameData`] as a resource and converts parsed formats
//! into Bevy assets: images ([`texture`]) and models with their materials
//! ([`model`]), each cached by name.

pub mod content_stream;
pub mod decomp;
pub mod hair;
pub mod human_skin;
pub mod image_format;
pub mod material_manager;
pub mod merge;
pub mod mesh_edit;
pub mod mesh_smooth;
pub mod mesh_weld;
pub mod model;
pub mod morph;
pub mod morph_controller;
pub mod noise;
pub mod paint;
pub mod paint_jobs;
pub mod polyhedra;
pub mod skin;
pub mod skin_composite;
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
    ClothMesh, ImageInsert, Model, ModelCache, ModelLoader, ModelNode, NodeRigidBody, TextureInfo,
    mesh_triangles, transform_from_rows,
};
pub use morph::BodyMorph;
pub use skin_composite::{SkinBeard, SkinCompositeJob, SkinDecomp};
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

/// The body model whose node tree a character was spawned from
/// (`humanbase.rfc`, `skelbase.rfc`, …), as the `Resource.rpk` entry name.
/// The dressed-mesh pass loads the same model again (morphed by the
/// character's physique) to merge the body's parts with its garments; a
/// character without it keeps the one-mesh-per-node draw.
#[derive(bevy::prelude::Component, Debug, Clone, PartialEq, Eq)]
pub struct BodyModel(pub String);

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
    /// The domain wheel's texture as drawn: `domainwheel` resampled to the
    /// GUI unit ([`TreeSheets::resample_wheel`]).
    pub wheel: Option<RgbaImage>,
    /// The flamelets sheet's mip chain, level 0 the picture above. The
    /// sheet is created with the mip bit (`0x40`) and the wheel's three
    /// programs sample it with a trilinear filter, so they pick a level by
    /// the drawn size; the other programs read level 0.
    pub flamelets_mips: Vec<RgbaImage>,
    /// The wheel's picture as it ships, with its format word and flags.
    pub wheel_source: Option<(RgbaImage, u32, u32)>,
    /// The GUI unit `wheel` was resampled for (0 before the first).
    pub wheel_unit: i32,
    /// The GUI font atlas the tree's text samples. The running screen fills
    /// this with the interface level's font, or with the base font's atlas
    /// when the install does not ship that level's font (the packs ship only
    /// `fontbase24r`).
    pub font: Option<RgbaImage>,
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
        sheets.flamelets_mips = self.load_rgba_chain("flameletsa1").unwrap_or_default();
        sheets.wheel_source = self.load_rgba_with_format("domainwheel").ok();
        sheets.resample_wheel(4);
        sheets
    }

    /// An `.rfi`'s mip chain as RGBA8 pictures, level 0 first. A file that
    /// stores its chain is decoded level by level; a single-level
    /// uncompressed image gets the engine's generated chain
    /// ([`exa_render::mipmap`]) with the image's own wrap flags and format
    /// word. The wheel's flamelets sheet is made with the mip bit and its
    /// programs sample across the chain.
    pub fn load_rgba_chain(&self, name: &str) -> Result<Vec<RgbaImage>, String> {
        let bytes = self
            .0
            .texture(name)
            .ok_or_else(|| format!("texture {name:?} not found"))?;
        let rfi = Rfi::parse(bytes).map_err(|e| e.to_string())?;
        let (width, height) = (rfi.header.width, rfi.header.height);
        if width == 0 || height == 0 {
            return Err(format!("texture {name:?} has no pixels"));
        }
        let mut levels = Vec::new();
        if rfi.mip_count > 1 && !rfi.is_packed() {
            let pixels = rfi.pixels().map_err(|e| e.to_string())?;
            for level in 0..rfi.mip_count {
                let (w, h) = exa_formats::rfi::level_dims(width, height, level);
                let data = rfi
                    .decode_level_rgba8(&pixels, level)
                    .map_err(|e| e.to_string())?;
                if data.len() != w as usize * h as usize * 4 {
                    break;
                }
                levels.push(RgbaImage {
                    width: w,
                    height: h,
                    pixels: data,
                });
            }
            if !levels.is_empty() {
                return Ok(levels);
            }
        }
        let base = rfi.decode_rgba8().map_err(|e| e.to_string())?;
        let options = exa_render::mipmap::ChainOptions::for_image(
            rfi.header.format_word,
            rfi.header.flags,
            0,
            u32::MAX,
        );
        let (chain, count) = exa_render::mipmap::build_mip_chain_rgba8_with(
            &base,
            width as usize,
            height as usize,
            &options,
        );
        let mut offset = 0usize;
        for level in 0..count {
            let (w, h) = exa_formats::rfi::level_dims(width, height, level);
            let size = w as usize * h as usize * 4;
            let Some(pixels) = chain.get(offset..offset + size) else {
                break;
            };
            levels.push(RgbaImage {
                width: w,
                height: h,
                pixels: pixels.to_vec(),
            });
            offset += size;
        }
        if levels.is_empty() {
            return Err(format!("texture {name:?} has no chain"));
        }
        Ok(levels)
    }

    /// An `.rfi` as raw RGBA (rows bottom-up) with its format word and its
    /// header flags.
    pub fn load_rgba_with_format(&self, name: &str) -> Result<(RgbaImage, u32, u32), String> {
        let bytes = self
            .0
            .texture(name)
            .ok_or_else(|| format!("texture {name:?} not found"))?;
        let rfi = Rfi::parse(bytes).map_err(|e| e.to_string())?;
        let pixels = rfi.decode_rgba8().map_err(|e| e.to_string())?;
        Ok((
            RgbaImage {
                width: rfi.header.width,
                height: rfi.header.height,
                pixels,
            },
            rfi.header.format_word,
            rfi.header.flags,
        ))
    }
}

/// The domain wheel's texture at the GUI unit `unit`, as the tree's sheet
/// load makes it: the picture resampled to `unit · 32` pixels a side by the
/// engine's resample-to-size path (the mip chain's level for a
/// power-of-two ratio, else the phase-shifted window), in the space its
/// format word picks and wrapping by its flags. `None` for a unit whose
/// side is not positive or does not fit, or a picture the resample refuses.
pub fn wheel_texture(
    source: &RgbaImage,
    format_word: u32,
    flags: u32,
    unit: i32,
) -> Option<RgbaImage> {
    let side = unit
        .checked_mul(32)
        .filter(|side| (1..=0x4000).contains(side))? as usize;
    let pixels = exa_render::mipmap::resample_rgba8_to_size(
        &source.pixels,
        source.width as usize,
        source.height as usize,
        side,
        side,
        format_word,
        flags,
    );
    (pixels.len() == side * side * 4).then_some(RgbaImage {
        width: side as u32,
        height: side as u32,
        pixels,
    })
}

impl TreeSheets {
    /// Resamples the wheel's texture for the GUI unit `unit` when it was
    /// made for another (the original's load runs again when the interface
    /// is rescaled). Without a source picture the wheel stays empty.
    pub fn resample_wheel(&mut self, unit: i32) {
        if self.wheel_unit == unit && self.wheel.is_some() {
            return;
        }
        self.wheel_unit = unit;
        self.wheel = self
            .wheel_source
            .as_ref()
            .and_then(|(source, format, flags)| wheel_texture(source, *format, *flags, unit));
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_wheel_texture_is_resampled_to_thirty_two_units() {
        // A flat 96-pixel picture shrinks by three to 32 pixels at the
        // one-pixel unit and stays flat; 64 pixels halve to the chain's first
        // level; a unit with no side resamples nothing.
        let flat = crate::RgbaImage {
            width: 96,
            height: 96,
            pixels: [200u8, 100, 50, 255].repeat(96 * 96),
        };
        let small = crate::wheel_texture(&flat, 0x0100_2008, 0x2020_0000, 1).expect("a texture");
        assert_eq!((small.width, small.height), (32, 32));
        assert!(small.pixels.chunks(4).all(|p| p == [200, 100, 50, 255]));
        let half = crate::RgbaImage {
            width: 64,
            height: 64,
            pixels: [10u8, 20, 30, 40].repeat(64 * 64),
        };
        let level = crate::wheel_texture(&half, 0x0100_2008, 0, 1).expect("a level");
        assert_eq!(level.pixels.len(), 32 * 32 * 4);
        assert!(crate::wheel_texture(&flat, 0x0100_2008, 0, 0).is_none());
        let mut sheets = crate::TreeSheets {
            wheel_source: Some((flat, 0x0100_2008, 0)),
            ..Default::default()
        };
        sheets.resample_wheel(2);
        assert_eq!(sheets.wheel.as_ref().map(|w| w.width), Some(64));
        assert_eq!(sheets.wheel_unit, 2);
    }

    #[test]
    fn the_tree_sheets_load() {
        let Some(dir) = crate::locate_install(None) else {
            return;
        };
        let game = crate::Game(std::sync::Arc::new(
            crate::GameData::open(dir).expect("the install opens"),
        ));
        let sheets = game.load_tree_sheets();
        // The sheets that are installed are square and a whole number of
        // the icon cells the original's texture scale counts (512 pixels):
        // the shipped mind and force sheets are 2048 (four cells) and the
        // energy and displacement ones 512 (one). Some domains' sheets may
        // not be installed; their icons are not drawn.
        assert!(sheets.domains[1].is_some() && sheets.domains[2].is_some());
        for sheet in sheets.domains.iter().flatten() {
            assert_eq!(sheet.width, sheet.height);
            assert!(matches!(sheet.width, 512 | 2048), "{}", sheet.width);
            assert!((1..=4).contains(&(sheet.width >> 9)));
        }
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
        // The sheet ships a stored 11-level chain; the wheel's programs
        // sample across it.
        assert_eq!(sheets.flamelets_mips.len(), 11);
        assert_eq!(
            (
                sheets.flamelets_mips[1].width,
                sheets.flamelets_mips[1].height
            ),
            (512, 512)
        );
        assert_eq!(
            (
                sheets.flamelets_mips[10].width,
                sheets.flamelets_mips[10].height
            ),
            (1, 1)
        );
        assert_eq!(
            sheets.flamelets_mips[0].pixels, flamelets.pixels,
            "level 0 is the picture itself"
        );
        // The wheel ships at 384 pixels in the raw packed word 0x1002008
        // without wrap flags and is drawn resampled to 32 units: 128 pixels
        // at the four-pixel unit, 160 at five.
        let (source, format, flags) = sheets.wheel_source.as_ref().expect("the domain wheel");
        assert_eq!((source.width, source.height), (384, 384));
        assert_eq!((*format, *flags), (0x0100_2008, 0x2020_0000));
        let wheel = sheets.wheel.as_ref().expect("the resampled wheel");
        assert_eq!(
            (wheel.width, wheel.height, sheets.wheel_unit),
            (128, 128, 4)
        );
        let mut sheets = sheets;
        sheets.resample_wheel(5);
        assert_eq!(sheets.wheel.as_ref().map(|w| w.width), Some(160));
        // The font atlas is the running screen's choice: the sheet loader
        // leaves it empty, and `fontbase24r` (the one the packs ship) is
        // loaded by the screen itself.
        assert!(sheets.font.is_none());
        let base = game.load_rgba("fontbase24r").expect("the base font");
        assert_eq!((base.width, base.height), (1024, 1024));
    }
}
