//! Fire particles' material: the original's built-in `_FirePrt02` (small
//! fires) and `_FirePrt01` (large): unlit, the particle texture times each
//! particle's vertex colour, blended premultiplied (ONE,
//! ONE_MINUS_SRC_ALPHA), so faint particles add up almost additively.
//! Rules: `docs/RENDERING.md` (Fires).

use bevy::asset::{AssetPath, embedded_asset, embedded_path};
use bevy::mesh::MeshVertexBufferLayoutRef;
use bevy::pbr::{Material, MaterialPipeline, MaterialPipelineKey, MaterialPlugin};
use bevy::prelude::*;
use bevy::render::render_resource::{
    AsBindGroup, RenderPipelineDescriptor, SpecializedMeshPipelineError,
};
use bevy::shader::ShaderRef;

pub struct FlamePlugin;

impl Plugin for FlamePlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "flame.wesl");
        app.add_plugins(MaterialPlugin::<FlameMaterial>::default());
    }
}

#[derive(Asset, TypePath, AsBindGroup, Clone, Debug)]
pub struct FlameMaterial {
    /// The particle texture (`FirePrt02.rfi`), in the emissive slot.
    #[texture(0)]
    #[sampler(1)]
    pub texture: Handle<Image>,
}

impl Material for FlameMaterial {
    fn fragment_shader() -> ShaderRef {
        ShaderRef::Path(
            AssetPath::from_path_buf(embedded_path!("flame.wesl")).with_source("embedded"),
        )
    }

    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Premultiplied
    }

    // Particles face the camera whichever way their diamond is wound.
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
