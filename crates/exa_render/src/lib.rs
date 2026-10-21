//! Rendering: the original's look rebuilt on Bevy. The post-processing
//! chain replaces Bevy's tonemapping (grading curve, bloom); materials and
//! lighting follow. Rules: `docs/RENDERING.md`.

pub mod bound_sphere;
pub mod cube_shadows;
pub mod electricity;
pub mod flame;
pub mod gi;
pub mod glow;
pub mod image_noise;
pub mod lights;
pub mod material;
pub mod material_state;
pub mod mipmap;
pub mod passes;
pub mod post;
pub mod quads;
pub mod quality;
pub mod shader_program;
pub mod shader_template;
pub mod shader_uniforms;
pub mod shell;
pub mod smoke;
pub mod spray;
pub mod stain;
pub mod sun_fit;
pub mod supersample;
pub mod view;

use bevy::prelude::*;

/// Faces of the lights' cube shadow maps, texels (the original's at its top
/// shadow detail).
pub const POINT_SHADOW_SIZE: usize = 512;
/// The sun's fitted shadow map, texels on a side. The original sizes it from
/// the user's shadow detail plus the level's sun flags: 256/512/1024/2048/
/// 4096/8192 for detail 1–6 at flag 0, which is 4096 at the top detail.
pub const SUN_SHADOW_SIZE: usize = 4096;

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
            quality::QualityPlugin,
            material::MaterialsPlugin,
            gi::GiPlugin,
            flame::FlamePlugin,
            electricity::ElectricityPlugin,
            spray::SprayPlugin,
            stain::StainPlugin,
            supersample::SupersamplePlugin,
            lights::LightsPlugin,
            cube_shadows::CubeShadowsPlugin,
            shell::ShellPlugin,
            glow::GlowPlugin,
            smoke::SmokePlugin,
        ))
            // The original's shadow maps at its top shadow detail: 512² cubes
            // for lights and fires, 4096² for the sun.
            .insert_resource(bevy::light::PointLightShadowMap {
                size: POINT_SHADOW_SIZE,
            })
            .insert_resource(bevy::light::DirectionalLightShadowMap {
                size: SUN_SHADOW_SIZE,
            });
    }
}
