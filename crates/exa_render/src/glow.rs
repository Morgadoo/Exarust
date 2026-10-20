//! The thaumaturgy effects' stand-in material: unlit, its vertex colour
//! times a tint, added to the frame, both faces, no shadows, no depth
//! writes. The original draws its arcs, wisps and barricade with programs
//! of their own (the electricity shader, `effectforce`, `effectfbrcd`) that
//! are not ported yet; this shows the simulations' meshes in the meantime.
//! Rules: `docs/THAUMATURGY.md` (Drawing).

use bevy::asset::{AssetPath, embedded_asset, embedded_path};
use bevy::mesh::MeshVertexBufferLayoutRef;
use bevy::pbr::{Material, MaterialPipeline, MaterialPipelineKey, MaterialPlugin};
use bevy::prelude::*;
use bevy::render::render_resource::{
    AsBindGroup, RenderPipelineDescriptor, SpecializedMeshPipelineError,
};
use bevy::shader::ShaderRef;

pub struct GlowPlugin;

impl Plugin for GlowPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "glow.wesl");
        app.add_plugins(MaterialPlugin::<GlowMaterial>::default());
    }
}

/// A tint (linear colour, alpha its strength) the vertex colours multiply.
#[derive(Asset, TypePath, AsBindGroup, Clone, Debug)]
pub struct GlowMaterial {
    #[uniform(0)]
    pub tint: Vec4,
}

impl Material for GlowMaterial {
    fn fragment_shader() -> ShaderRef {
        ShaderRef::Path(
            AssetPath::from_path_buf(embedded_path!("glow.wesl")).with_source("embedded"),
        )
    }

    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Add
    }

    fn enable_prepass() -> bool {
        false
    }

    fn enable_shadows() -> bool {
        false
    }

    fn enable_oit() -> bool {
        false
    }

    // Ribbons and walls are seen from both sides.
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

/// The colour a fragment adds: the tint times the vertex colour, its
/// colour scaled by its alpha.
pub fn glow_colour(tint: Vec4, vertex: Vec4) -> Vec4 {
    let c = tint * vertex;
    Vec4::new(c.x * c.w, c.y * c.w, c.z * c.w, c.w)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_glow_scales_by_its_alpha() {
        let added = glow_colour(Vec4::new(1.0, 0.5, 1.0, 1.0), Vec4::new(0.1, 0.0, 1.0, 0.5));
        assert_eq!(added, Vec4::new(0.05, 0.0, 0.5, 0.5));
        assert_eq!(glow_colour(Vec4::ONE, Vec4::ZERO), Vec4::ZERO);
    }
}
