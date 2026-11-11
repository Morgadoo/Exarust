//! The frame-rate readout's setting. The readout itself is the 3D
//! viewport's overlay, drawn by `exa_game::view_overlay` with the rules of
//! [`crate::viewport`]. Rules: `docs/UI.md` (Frame rate).

use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

/// The ShowFPS setting: whether the frame rate is shown; on by default
/// (`--no-fps` turns it off).
#[derive(Resource, Clone, Copy, Debug)]
pub struct ShowFps(pub bool);

impl Default for ShowFps {
    fn default() -> Self {
        Self(true)
    }
}

pub(crate) fn plugin(app: &mut App) {
    app.init_resource::<ShowFps>();
}

/// White text with the coverage as alpha; the node's colour tints it.
pub(crate) fn text_image(width: usize, height: usize, alpha: &[u8]) -> Image {
    let data = alpha.iter().flat_map(|&a| [255, 255, 255, a]).collect();
    Image::new(
        Extent3d {
            width: width as u32,
            height: height as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    )
}
