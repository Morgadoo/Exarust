//! Rendering: the original's look rebuilt on Bevy. The post-processing
//! chain replaces Bevy's tonemapping (grading curve, bloom); materials and
//! lighting follow. Rules: `docs/RENDERING.md`.

pub mod flame;
pub mod gi;
pub mod material;
pub mod post;

use bevy::prelude::*;

pub struct RenderPlugin;

impl Plugin for RenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            post::PostPlugin,
            material::MaterialsPlugin,
            gi::GiPlugin,
            flame::FlamePlugin,
        ))
            // The original's shadow maps at its top shadow detail: 512² cubes
            // for lights and fires, 4096² for the sun.
            .insert_resource(bevy::light::PointLightShadowMap { size: 512 })
            .insert_resource(bevy::light::DirectionalLightShadowMap { size: 4096 });
    }
}
