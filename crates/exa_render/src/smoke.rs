//! Smoke and mist: the original's `shadesmk_f` draw.
//!
//! The draw binds the program built from `shading_v` + `shadesmk_f` for
//! the material's state (compiled once per state key, the state's bits
//! under [`STATE_KEY_MASK`]), sets its `UVOfst` uniform and draws the mesh:
//! `UVOfst = (0.5, t + 0.5, t)` with `t = parent node scale + (scene ms
//! mod 80 000) / 80 000`, the sum in double precision. The program reads
//! the opacity texture once shifted by `t` down and again shifted by
//! `(0.5, t + 0.5)` more, multiplies the two alphas, and adds the surface's
//! facing ratio and the vertex alpha (`shadesmk.wesl`). With the
//! supersampled quad passes on, the original repeats the draw once per
//! pass with that pass's viewport; Exarust's supersampled camera draws the
//! scene once.
//!
//! Nothing spawns a smoke mesh yet: the material plug-in that routes a
//! record here is registered at run time and not ported.
//!
//! Rules: `docs/RENDERING.md` (Smoke and mist).

use bevy::asset::{AssetPath, embedded_asset, embedded_path};
use bevy::mesh::MeshVertexBufferLayoutRef;
use bevy::pbr::{Material, MaterialPipeline, MaterialPipelineKey, MaterialPlugin};
use bevy::prelude::*;
use bevy::render::render_resource::{
    AsBindGroup, RenderPipelineDescriptor, ShaderType, SpecializedMeshPipelineError,
};
use bevy::shader::ShaderRef;

pub struct SmokePlugin;

impl Plugin for SmokePlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "shadesmk.wesl");
        app.add_plugins(MaterialPlugin::<SmokeMaterial>::default())
            .add_systems(Update, scroll_smoke);
    }
}

/// The bits of a material state that select a smoke program.
pub const STATE_KEY_MASK: u64 = 0x0003_C00F_0048_0008;
/// The texture scroll's cycle, milliseconds.
pub const SCROLL_PERIOD_MS: u64 = 80_000;

/// The smoke program's state key from a material state's 64-bit word.
pub fn smoke_state_key(state: u64) -> u64 {
    state & STATE_KEY_MASK
}

/// `UVOfst` at scene time `time_ms` under a parent node of scale `scale`.
pub fn smoke_uv_offset(scale: f32, time_ms: u32) -> Vec3 {
    let phase = f64::from(time_ms % SCROLL_PERIOD_MS as u32) / SCROLL_PERIOD_MS as f64;
    let t = (f64::from(scale) + phase) as f32;
    Vec3::new(0.5, 0.5 + t, t)
}

/// A smoke or mist mesh: its material and its parent node's scale.
#[derive(Component, Debug, Clone, PartialEq)]
pub struct SmokeMist {
    pub material: Handle<SmokeMaterial>,
    pub scale: f32,
}

/// The program's uniforms.
#[derive(Clone, Copy, Debug, Default, PartialEq, ShaderType)]
pub struct SmokeParams {
    /// `UVOfst`.
    pub uv_offset: Vec3,
    /// 1 with an opacity texture (`Mat.TexOpc`).
    pub has_opacity: u32,
}

/// A smoke or mist surface: alpha-blended, both faces drawn (the program
/// takes the facing ratio's magnitude).
#[derive(Asset, TypePath, AsBindGroup, Clone, Debug)]
pub struct SmokeMaterial {
    #[uniform(0)]
    pub params: SmokeParams,
    #[texture(1)]
    #[sampler(2)]
    pub opacity: Handle<Image>,
}

fn smoke_shader() -> ShaderRef {
    ShaderRef::Path(
        AssetPath::from_path_buf(embedded_path!("shadesmk.wesl")).with_source("embedded"),
    )
}

impl Material for SmokeMaterial {
    fn fragment_shader() -> ShaderRef {
        smoke_shader()
    }

    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Blend
    }

    fn specialize(
        _pipeline: &MaterialPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        _layout: &MeshVertexBufferLayoutRef,
        _key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        descriptor.primitive.cull_mode = None;
        Ok(())
    }
}

/// Sets each smoke mesh's `UVOfst` for the frame.
fn scroll_smoke(
    time: Res<Time>,
    mist: Query<&SmokeMist>,
    mut materials: ResMut<Assets<SmokeMaterial>>,
) {
    let now = time.elapsed().as_millis() as u32;
    for smoke in &mist {
        if let Some(mut material) = materials.get_mut(&smoke.material) {
            material.params.uv_offset = smoke_uv_offset(smoke.scale, now);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_offset_scrolls_once_each_80_seconds() {
        assert_eq!(smoke_uv_offset(1.0, 0), Vec3::new(0.5, 1.5, 1.0));
        assert_eq!(smoke_uv_offset(1.0, 40_000), Vec3::new(0.5, 2.0, 1.5));
        assert_eq!(smoke_uv_offset(1.0, 80_000), smoke_uv_offset(1.0, 0));
        assert_eq!(smoke_uv_offset(0.0, 20_000), Vec3::new(0.5, 0.75, 0.25));
        assert_eq!(smoke_uv_offset(1.0, 120_000), smoke_uv_offset(1.0, 40_000));
        // The sum is in double precision before the single-precision
        // offset is formed.
        let t = (f64::from(0.1f32) + 1.0 / 80_000.0) as f32;
        assert_eq!(smoke_uv_offset(0.1, 1), Vec3::new(0.5, 0.5 + t, t));
    }

    #[test]
    fn the_state_key_keeps_the_selected_bits_only() {
        assert_eq!(smoke_state_key(0), 0);
        assert_eq!(smoke_state_key(u64::MAX), STATE_KEY_MASK);
        assert_eq!(smoke_state_key(0x8000), 0);
        assert_eq!(smoke_state_key(0x8), 0x8);
        assert_eq!(
            smoke_state_key(0x0002_0000_0040_0000),
            0x0002_0000_0040_0000
        );
    }
}
