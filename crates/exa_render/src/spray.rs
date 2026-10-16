//! Blood droplets' material: the original's built-in `_BloodPrt02`. Each
//! droplet is a diamond of the droplet texture (`BldPrt02`, `Textures.rpk`:
//! white with four small dark-red splats in a 2 × 2 grid) that darkens
//! what lies behind it: the frame is multiplied by the texel mixed toward
//! white by the droplet's alpha, so white leaves the scene as it is and a
//! fading droplet fades out. Unlit, no shadows, no depth writes. Rules:
//! `docs/RENDERING.md` (Blood sprays).

use bevy::asset::{AssetPath, embedded_asset, embedded_path};
use bevy::mesh::MeshVertexBufferLayoutRef;
use bevy::pbr::{Material, MaterialPipeline, MaterialPipelineKey, MaterialPlugin};
use bevy::prelude::*;
use bevy::render::render_resource::{
    AsBindGroup, RenderPipelineDescriptor, SpecializedMeshPipelineError,
};
use bevy::shader::ShaderRef;

pub struct SprayPlugin;

impl Plugin for SprayPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "spray.wesl");
        app.add_plugins(MaterialPlugin::<SprayMaterial>::default());
    }
}

/// The droplet texture (`Textures.rpk`), sRGB, clamped on both axes (no
/// wrap bit in the material's load flags or the file's).
pub const SPRAY_TEXTURE: &str = "bldprt02";

/// The colour the frame is multiplied by under a droplet whose texel is
/// `texel` (linear) and whose alpha is `alpha`: the texel mixed toward
/// white by the alpha.
pub fn spray_colour(texel: Vec3, alpha: f32) -> Vec3 {
    Vec3::ONE.lerp(texel, alpha)
}

/// The droplets of the scene's blood sprays.
#[derive(Asset, TypePath, AsBindGroup, Clone, Debug)]
pub struct SprayMaterial {
    /// The droplet texture ([`SPRAY_TEXTURE`]), in the emissive slot.
    #[texture(0)]
    #[sampler(1)]
    pub texture: Handle<Image>,
}

impl Material for SprayMaterial {
    fn fragment_shader() -> ShaderRef {
        ShaderRef::Path(
            AssetPath::from_path_buf(embedded_path!("spray.wesl")).with_source("embedded"),
        )
    }

    // Frame × colour; Bevy's multiply blend with an opaque fragment.
    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Multiply
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

    // Droplets face the camera whichever way their diamond is wound.
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

#[cfg(test)]
mod tests {
    use super::*;

    /// White texels leave the frame unchanged; a splat darkens it by its
    /// colour at full alpha, by half as much at half alpha, not at all at 0.
    #[test]
    fn droplets_darken_by_their_alpha() {
        let splat = Vec3::new(0.45, 0.02, 0.01);
        assert_eq!(spray_colour(Vec3::ONE, 0.7), Vec3::ONE);
        assert_eq!(spray_colour(splat, 1.0), splat);
        assert_eq!(spray_colour(splat, 0.0), Vec3::ONE);
        let half = spray_colour(splat, 0.5);
        assert!((half - Vec3::new(0.725, 0.51, 0.505)).length() < 1e-6);
    }
}
