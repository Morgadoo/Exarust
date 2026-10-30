//! Rain streaks' material: the original's rain program. The drops are a
//! line list (head, tail) in the rain box's own frame, in centimetres; the
//! material reads the frame behind each streak, so it draws in Bevy's
//! transmissive pass, after the opaque scene is copied. Rules:
//! `docs/RENDERING.md` (Rain).
//!
//! - **Vertex**: the drop's colour bytes are read as plain numbers, not
//!   normalised (the original's vertex format passes them unnormalised):
//!   the grey is 160 and the alpha 0 at the head and 255 at the tail. The
//!   colour is `RainCol0.rgb × bytes.rgb`; the alpha `RainCol0.a ×
//!   bytes.a × (0.6 − 0.000667·d)⁴`, d the vertex's distance from the eye
//!   in cm. Along a streak the alpha therefore rises from 0 at the head to
//!   far above 1 at the tail, and the 8-bit target clamps it.
//! - **Fragment**: the frame behind the streak `t` (clamped to 1, the
//!   original's frame buffer being 8-bit) becomes `0.75·t + 3·max(t − 0.1,
//!   0)²`, plus the vertex colour; alpha-blended (source alpha, one minus
//!   source alpha), depth-tested, one window pixel wide: the original draws
//!   a one-pixel line in each supersample corner at window resolution, so
//!   on Exarust's supersampled target each line is drawn
//!   [`crate::supersample::SCALE`] times, one target pixel apart (the copy
//!   index in the first texture coordinate).

use bevy::asset::{AssetPath, embedded_asset, embedded_path};
use bevy::mesh::{MeshVertexBufferLayoutRef, PrimitiveTopology};
use bevy::pbr::{Material, MaterialPipeline, MaterialPipelineKey, MaterialPlugin};
use bevy::prelude::*;
use bevy::render::render_resource::{
    AsBindGroup, RenderPipelineDescriptor, SpecializedMeshPipelineError,
};
use bevy::shader::ShaderRef;

pub struct RainPlugin;

impl Plugin for RainPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "rain.wesl");
        app.add_plugins(MaterialPlugin::<RainMaterial>::default());
    }
}

/// The vertex shader's distance fade: `(0.6 − 0.000667·d)⁴`, `d` in cm.
pub fn eye_fade_cm(distance_cm: f32) -> f32 {
    let d = distance_cm * -0.000_667 + 0.6;
    let d = d * d;
    d * d
}

/// A streak's fragment over the frame colour `frame` (linear) with the
/// interpolated vertex colour `colour`: the colour written before the
/// blend, clamped to 1 as the original's 8-bit target clamps it.
pub fn streak_fragment(frame: Vec3, colour: Vec4) -> Vec4 {
    let t = frame.clamp(Vec3::ZERO, Vec3::ONE);
    let bright = (t - Vec3::splat(0.1)).max(Vec3::ZERO);
    let rgb = t * 0.75 + bright * bright * 3.0 + colour.truncate();
    rgb.extend(colour.w).clamp(Vec4::ZERO, Vec4::ONE)
}

/// What the alpha blend leaves on the frame `frame` under a fragment
/// `fragment` ([`streak_fragment`]).
pub fn blended(frame: Vec3, fragment: Vec4) -> Vec3 {
    fragment.truncate() * fragment.w + frame * (1.0 - fragment.w)
}

/// An empty line-list mesh for the drops; the caller fills its positions
/// and colours each frame.
pub fn rain_mesh() -> Mesh {
    Mesh::new(
        PrimitiveTopology::LineList,
        bevy::asset::RenderAssetUsages::default(),
    )
}

/// The rain streaks' material: `RainCol0`, the colour and alpha the rain
/// effect computes each frame.
#[derive(Asset, TypePath, AsBindGroup, Clone, Debug)]
pub struct RainMaterial {
    #[uniform(0)]
    pub colour: Vec4,
    /// A debug aid: x above 0.5 draws the streaks opaque magenta.
    #[uniform(1)]
    pub debug: Vec4,
}

fn rain_shader() -> ShaderRef {
    ShaderRef::Path(AssetPath::from_path_buf(embedded_path!("rain.wesl")).with_source("embedded"))
}

impl Material for RainMaterial {
    fn vertex_shader() -> ShaderRef {
        rain_shader()
    }

    fn fragment_shader() -> ShaderRef {
        rain_shader()
    }

    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Blend
    }

    // The fragment reads the frame behind the streak.
    fn reads_view_transmission_texture(&self) -> bool {
        true
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

    /// The fade is 0.6⁴ at the eye, (0.6 − 0.2001)⁴ three metres out, and
    /// falls to zero 9 m out.
    #[test]
    fn the_fade_follows_the_eye_distance() {
        assert!((eye_fade_cm(0.0) - 0.1296).abs() < 1e-6);
        assert!((eye_fade_cm(300.0) - 0.3999f32.powi(4)).abs() < 1e-6);
        assert!(eye_fade_cm(899.55).abs() < 1e-9);
    }

    /// A streak over a dark wall darkens it a little; over a bright sky it
    /// brightens it through the squared excess; the alpha clamps at 1.
    #[test]
    fn a_streak_reads_the_frame_behind_it() {
        let dark = streak_fragment(Vec3::splat(0.05), Vec4::new(0.0, 0.0, 0.0, 5.0));
        assert!((dark.x - 0.0375).abs() < 1e-6);
        assert_eq!(dark.w, 1.0);
        let sky = streak_fragment(Vec3::splat(0.5), Vec4::new(0.1, 0.1, 0.1, 0.5));
        // 0.375 + 3 · 0.16 + 0.1.
        assert!((sky.x - 0.955).abs() < 1e-6);
        let over = blended(Vec3::splat(0.5), sky);
        assert!((over.x - (0.955 * 0.5 + 0.25)).abs() < 1e-6);
        // An HDR frame value reads as the 8-bit 1.
        let hot = streak_fragment(Vec3::splat(4.0), Vec4::ZERO);
        assert_eq!(hot.x, 1.0);
    }
}
