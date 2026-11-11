//! Fire particles' material: the original's built-in `_FirePrt02` (small
//! fires) and `_FirePrt01` (large): unlit, the particle texture times each
//! particle's vertex colour, blended premultiplied (ONE,
//! ONE_MINUS_SRC_ALPHA), so faint particles add up almost additively.
//! The level portals' built-in `_PrtlPrt01` (`PrtDot02`) draws the same
//! way with the blend word 0x101, ONE and ONE: its particles add in full
//! ([`FlameMaterial::additive`]).
//! Rules: `docs/RENDERING.md` (Fires, Level portals).

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
#[bind_group_data(FlameKey)]
pub struct FlameMaterial {
    /// The particle texture (`FirePrt02.rfi`), in the emissive slot.
    #[texture(0)]
    #[sampler(1)]
    pub texture: Handle<Image>,
    /// The blend word 0x101 (ONE, ONE) instead of the fires' 0x2123 (ONE,
    /// ONE_MINUS_SRC_ALPHA): the colour adds and the alpha is unused.
    pub additive: bool,
}

impl Material for FlameMaterial {
    fn fragment_shader() -> ShaderRef {
        ShaderRef::Path(
            AssetPath::from_path_buf(embedded_path!("flame.wesl")).with_source("embedded"),
        )
    }

    // ONE, ONE_MINUS_SRC_ALPHA; the additive form writes no alpha, so the
    // same blend adds its colour in full (ONE, ONE).
    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Premultiplied
    }

    // Particles face the camera whichever way their diamond is wound.
    fn specialize(
        _pipeline: &MaterialPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        _layout: &MeshVertexBufferLayoutRef,
        key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        descriptor.primitive.cull_mode = None;
        if key.bind_group_data.additive
            && let Some(fragment) = descriptor.fragment.as_mut()
        {
            fragment.shader_defs.push("ADDITIVE".into());
        }
        Ok(())
    }
}

/// The pipeline key: the additive form is its own pipeline.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct FlameKey {
    additive: bool,
}

impl From<&FlameMaterial> for FlameKey {
    fn from(m: &FlameMaterial) -> Self {
        Self {
            additive: m.additive,
        }
    }
}
