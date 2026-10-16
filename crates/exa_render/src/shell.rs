//! The rooms' black shells: the occluders filling the hollows behind wall
//! facades and the lids over rooms. The original writes them into depth
//! with colour writes off before anything else, so whatever lies under them
//! keeps the black clear colour. Exarust draws them as opaque, unlit pure
//! black instead: the same image over a black clear (every tile-grid level
//! clears to black) without depending on draw order. A room fading in or out
//! gets its lid drawn last as a black overlay, alpha-blended, without depth
//! test or depth write. Rules: `docs/RENDERING.md` (Rooms and black fill).

use bevy::asset::{AssetPath, embedded_asset, embedded_path};
use bevy::mesh::MeshVertexBufferLayoutRef;
use bevy::pbr::{Material, MaterialPipeline, MaterialPipelineKey, MaterialPlugin};
use bevy::prelude::*;
use bevy::render::render_resource::{
    AsBindGroup, CompareFunction, RenderPipelineDescriptor, SpecializedMeshPipelineError,
};
use bevy::shader::ShaderRef;

pub struct ShellPlugin;

impl Plugin for ShellPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "shell.wesl");
        app.add_plugins(MaterialPlugin::<ShellMaterial>::default());
    }
}

/// Added to an overlay's view depth so it sorts after every other blended
/// surface (particles included): it is drawn last.
const OVERLAY_SORT_BIAS: f32 = 1.0e9;

/// A flat colour, unlit: opaque black for shells, black at a fading alpha
/// for overlays.
#[derive(Asset, TypePath, AsBindGroup, Clone, Debug)]
#[bind_group_data(ShellKey)]
pub struct ShellMaterial {
    /// Linear colour; alpha is the overlay's opacity.
    #[uniform(0)]
    pub colour: Vec4,
    /// Drawn last over everything, blended, without depth test.
    pub overlay: bool,
}

impl ShellMaterial {
    /// An opaque black shell.
    pub fn black() -> Self {
        Self {
            colour: Vec4::new(0.0, 0.0, 0.0, 1.0),
            overlay: false,
        }
    }

    /// A black overlay at `alpha`.
    pub fn overlay(alpha: f32) -> Self {
        Self {
            colour: Vec4::new(0.0, 0.0, 0.0, alpha),
            overlay: true,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct ShellKey {
    overlay: bool,
}

impl From<&ShellMaterial> for ShellKey {
    fn from(m: &ShellMaterial) -> Self {
        Self { overlay: m.overlay }
    }
}

impl Material for ShellMaterial {
    fn fragment_shader() -> ShaderRef {
        ShaderRef::Path(
            AssetPath::from_path_buf(embedded_path!("shell.wesl")).with_source("embedded"),
        )
    }

    fn alpha_mode(&self) -> AlphaMode {
        if self.overlay {
            AlphaMode::Blend
        } else {
            AlphaMode::Opaque
        }
    }

    fn depth_bias(&self) -> f32 {
        if self.overlay { OVERLAY_SORT_BIAS } else { 0.0 }
    }

    // Shells never cast shadows.
    fn enable_shadows() -> bool {
        false
    }

    fn specialize(
        _pipeline: &MaterialPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        _layout: &MeshVertexBufferLayoutRef,
        key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        if key.bind_group_data.overlay
            && let Some(depth) = descriptor.depth_stencil.as_mut()
        {
            depth.depth_compare = Some(CompareFunction::Always);
            depth.depth_write_enabled = Some(false);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shells_are_opaque_black_and_overlays_blend_last() {
        let shell = ShellMaterial::black();
        assert_eq!(shell.alpha_mode(), AlphaMode::Opaque);
        assert_eq!(shell.colour, Vec4::new(0.0, 0.0, 0.0, 1.0));
        assert_eq!(shell.depth_bias(), 0.0);
        let overlay = ShellMaterial::overlay(0.25);
        assert_eq!(overlay.alpha_mode(), AlphaMode::Blend);
        assert_eq!(overlay.colour.w, 0.25);
        assert!(overlay.depth_bias() >= OVERLAY_SORT_BIAS);
        assert_ne!(ShellKey::from(&shell), ShellKey::from(&overlay));
    }
}
