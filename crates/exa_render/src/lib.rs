//! Rendering: the original's look rebuilt on Bevy. The post-processing
//! chain replaces Bevy's tonemapping (grading curve, bloom); materials and
//! lighting follow. Rules: `docs/RENDERING.md`.

pub mod flame;
pub mod gi;
pub mod material;
pub mod post;
pub mod supersample;

use bevy::prelude::*;

/// Faces of the lights' cube shadow maps, texels (the original's at its top
/// shadow detail).
pub const POINT_SHADOW_SIZE: usize = 512;

/// Bevy multiplies a point light's normal bias by this before the shader
/// sees it; values carried in that field are divided by it first.
pub const NORMAL_BIAS_SCALE: f32 = 2.0 / POINT_SHADOW_SIZE as f32 * std::f32::consts::SQRT_2;

/// A node's shadow-bias level, carried to the shaders in the mesh tag:
/// terrain and tile-grid floors are 3, other grid set pieces 1, everything
/// else 0. A higher level keeps a surface's own shadow off itself (see
/// `shading.wesl`).
pub mod shadow_bias {
    pub const FLOOR: u32 = 3;
    pub const GRID_PIECE: u32 = 1;

    /// The mesh tag for `level`.
    pub fn tag(level: u32) -> bevy::mesh::MeshTag {
        bevy::mesh::MeshTag::new(level & 7)
    }
}

pub struct RenderPlugin;

impl Plugin for RenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            post::PostPlugin,
            material::MaterialsPlugin,
            gi::GiPlugin,
            flame::FlamePlugin,
            supersample::SupersamplePlugin,
        ))
            // The original's shadow maps at its top shadow detail: 512² cubes
            // for lights and fires, 4096² for the sun.
            .insert_resource(bevy::light::PointLightShadowMap {
                size: POINT_SHADOW_SIZE,
            })
            .insert_resource(bevy::light::DirectionalLightShadowMap { size: 4096 });
    }
}
