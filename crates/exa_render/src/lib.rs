//! Rendering: the original's look rebuilt on Bevy. The post-processing
//! chain replaces Bevy's tonemapping (grading curve, bloom); materials and
//! lighting follow. Rules: `docs/RENDERING.md`.

pub mod cube_shadows;
pub mod flame;
pub mod gi;
pub mod lights;
pub mod material;
pub mod post;
pub mod shell;
pub mod stain;
pub mod supersample;

use bevy::prelude::*;

/// Faces of the lights' cube shadow maps, texels (the original's at its top
/// shadow detail).
pub const POINT_SHADOW_SIZE: usize = 512;

/// A node's shadow-bias level ([`lights::ShadowBias`], carried to the
/// shaders in the mesh tag): terrain and tile-grid floors are 3, other grid
/// set pieces 1, everything else 0. A higher level keeps a surface's own
/// shadow off itself (see `shading.wesl`).
pub mod shadow_bias {
    pub const FLOOR: u32 = 3;
    pub const GRID_PIECE: u32 = 1;
}

pub struct RenderPlugin;

impl Plugin for RenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            post::PostPlugin,
            material::MaterialsPlugin,
            gi::GiPlugin,
            flame::FlamePlugin,
            stain::StainPlugin,
            supersample::SupersamplePlugin,
            lights::LightsPlugin,
            cube_shadows::CubeShadowsPlugin,
            shell::ShellPlugin,
        ))
            // The original's shadow maps at its top shadow detail: 512² cubes
            // for lights and fires, 4096² for the sun.
            .insert_resource(bevy::light::PointLightShadowMap {
                size: POINT_SHADOW_SIZE,
            })
            .insert_resource(bevy::light::DirectionalLightShadowMap { size: 4096 });
    }
}
