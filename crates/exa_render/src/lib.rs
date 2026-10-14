//! Rendering: the original's look rebuilt on Bevy. The post-processing
//! chain replaces Bevy's tonemapping (grading curve, bloom); materials and
//! lighting follow. Rules: `docs/RENDERING.md`.

pub mod material;
pub mod post;

use bevy::prelude::*;

pub struct RenderPlugin;

impl Plugin for RenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((post::PostPlugin, material::MaterialsPlugin));
    }
}
