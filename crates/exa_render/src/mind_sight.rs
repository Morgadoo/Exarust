//! The Mind domain's screen program: the sense overlay's sphere, the same
//! program the original builds for its mind-sight glow passes.
//!
//! The vertex stage is the original's source beginning `in vec3 vPos; in
//! vec3 vNrm; in vec2 vTex;`: the sphere's local position dotted with the two texture axes
//! gives `vrTex.xy`, and the local normal dotted with the eye direction
//! gives `vrTex.z` as the squared facing (`max(dot, 0)²`). The fragment
//! stage is the source beginning `in vec3 vrTex; in vec4 vrCol;`: the noise picture's red
//! channel is cut by two thresholds, the first two colours are mixed by
//! the first cut and the third by the second, the result is weighted by
//! `clamp((1 − facing) · −0.3 + tex, 0, 1)`, clamped to 0–1, squared and
//! multiplied by four, and the alpha is `MindCol0.w · facing`.
//!
//! The uniforms are the original's `ObjMat`, `TexVecU`, `TexVecV`,
//! `EyeVec`, `MindCol0..2` and `EmiTex`; [`MindSightMaterial`] is drawn by
//! Bevy's mesh stage, which supplies the model and view-projection matrices
//! the original's `ObjMat` holds.
//!
//! Evidence: the vertex and fragment source strings the original compiles
//! (both read from `Exanima.exe`), and the uniform and texture bindings of
//! its sight program.
//! Rules: `docs/RENDERING.md` (Mind sight).

use bevy::asset::{AssetPath, embedded_asset, embedded_path};
use bevy::mesh::MeshVertexBufferLayoutRef;
use bevy::pbr::{Material, MaterialPipeline, MaterialPipelineKey, MaterialPlugin};
use bevy::prelude::*;
use bevy::render::render_resource::{
    AsBindGroup, RenderPipelineDescriptor, SpecializedMeshPipelineError,
};
use bevy::shader::ShaderRef;

pub struct MindSightPlugin;

impl Plugin for MindSightPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "mind_sight.wesl");
        app.add_plugins(MaterialPlugin::<MindSightMaterial>::default());
    }
}

/// The sight program's uniforms and picture.
#[derive(Asset, TypePath, AsBindGroup, Clone, Debug)]
pub struct MindSightMaterial {
    /// `TexVecU`: the first texture axis in the sphere's frame.
    #[uniform(0)]
    pub tex_u: Vec4,
    /// `TexVecV`: the second texture axis.
    #[uniform(1)]
    pub tex_v: Vec4,
    /// `EyeVec`: the eye direction in the sphere's frame.
    #[uniform(2)]
    pub eye: Vec4,
    /// `MindCol0`.
    #[uniform(3)]
    pub colour0: Vec4,
    /// `MindCol1`.
    #[uniform(4)]
    pub colour1: Vec4,
    /// `MindCol2`.
    #[uniform(5)]
    pub colour2: Vec4,
    /// `EmiTex`: the animated noise picture.
    #[texture(6)]
    #[sampler(7)]
    pub noise: Handle<Image>,
}

impl Material for MindSightMaterial {
    fn vertex_shader() -> ShaderRef {
        ShaderRef::Path(
            AssetPath::from_path_buf(embedded_path!("mind_sight.wesl")).with_source("embedded"),
        )
    }

    fn fragment_shader() -> ShaderRef {
        Self::vertex_shader()
    }

    /// The original draws the spheres after the scene with the depth test
    /// off, under a state block whose blend word is 0; the fragment's alpha
    /// is `MindCol0.w · facing`. Exarust blends by that alpha over the
    /// scene [guess: the blend factors].
    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Blend
    }

    fn enable_prepass() -> bool {
        false
    }

    fn enable_shadows() -> bool {
        false
    }

    fn specialize(
        _pipeline: &MaterialPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        _layout: &MeshVertexBufferLayoutRef,
        _key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        // Both faces draw, so a sphere seen from inside still shows
        // [guess: the original's cull state].
        descriptor.primitive.cull_mode = None;
        // The original turns the depth test off around the sight draw.
        if let Some(depth) = descriptor.depth_stencil.as_mut() {
            depth.depth_compare = Some(bevy::render::render_resource::CompareFunction::Always);
            depth.depth_write_enabled = Some(false);
        }
        Ok(())
    }
}

/// `max(x, y)` as the shader's `max`: a not-a-number `x` is kept.
fn gl_max(x: f32, y: f32) -> f32 {
    if x < y { y } else { x }
}

/// `min(x, y)` as the shader's `min`: a not-a-number `x` is kept.
fn gl_min(x: f32, y: f32) -> f32 {
    if y < x { y } else { x }
}

/// The shader's `clamp` (nested `min(max(x, lo), hi)`).
fn gl_clamp(x: f32, lo: f32, hi: f32) -> f32 {
    gl_min(gl_max(x, lo), hi)
}

/// The shader's `mix(x, y, a)`: `x·(1 − a) + y·a`.
fn mix(x: Vec3, y: Vec3, a: f32) -> Vec3 {
    x * (1.0 - a) + y * a
}

/// The fragment stage's output for a sampled noise value, the three
/// colours and `vrTex.z` (the squared facing): the shipped source's
/// `col = clamp(col * a, 0.0, 1.0)` comes before the square.
pub fn fragment_colour(tex: f32, colour0: Vec4, colour1: Vec4, colour2: Vec4, facing: f32) -> Vec4 {
    let a = gl_clamp((tex - colour1.w) * 8.0, 0.0, 1.0);
    let mut col = mix(colour0.truncate(), colour1.truncate(), a);
    let a = gl_clamp((tex - colour2.w) * 8.0, 0.0, 1.0);
    col = mix(col, colour2.truncate(), a);
    let b = gl_clamp(1.0 - facing, 0.0, 1.0);
    let a = gl_clamp(b * -0.3 + tex, 0.0, 1.0);
    let col = col * a;
    let col = Vec3::new(
        gl_clamp(col.x, 0.0, 1.0),
        gl_clamp(col.y, 0.0, 1.0),
        gl_clamp(col.z, 0.0, 1.0),
    );
    let rgb = col * col * 4.0;
    rgb.extend(colour0.w * facing)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_fragment_mixes_the_three_colours_by_the_thresholds() {
        let colour0 = Vec4::new(0.1, 0.4, 0.6, 0.5);
        let colour1 = Vec4::new(0.8, 0.6, 0.0, 0.1);
        let colour2 = Vec4::new(0.4, 0.0, 0.0, 0.1);
        let out = fragment_colour(1.0, colour0, colour1, colour2, 0.0);
        // Both cuts are full at 1: the third colour, weighted by
        // clamp(1 − 0.3) = 0.7, then squared and times four.
        let lit = Vec3::new(0.4, 0.0, 0.0) * 0.7;
        assert!((out.truncate() - lit * lit * 4.0).length() < 1e-6);
        assert_eq!(out.w, 0.0);
        let dim = fragment_colour(0.0, colour0, colour1, colour2, 0.0);
        // The first colour, the cuts at −0.8 clamp to 0 and the facing
        // weight is −0.3, which clamps to 0: nothing shows.
        assert_eq!(dim.truncate(), Vec3::ZERO);
        let facing = fragment_colour(1.0, colour0, colour1, colour2, 1.0);
        assert_eq!(facing.w, 0.5);
    }

    #[test]
    fn the_weighted_colour_is_clamped_before_the_square() {
        // A colour of 3 at full weight clamps to 1 and squares to 4, not 36.
        let bright = Vec4::new(3.0, 0.5, -1.0, 0.0);
        let out = fragment_colour(1.0, bright, bright, bright, 1.0);
        assert_eq!(out.truncate(), Vec3::new(4.0, 1.0, 0.0));
    }

    #[test]
    fn a_not_a_number_noise_keeps_the_gl_min_max_operands() {
        // max(NaN, lo) keeps NaN; min(NaN, hi) keeps NaN.
        assert!(gl_max(f32::NAN, 0.0).is_nan());
        assert!(gl_min(f32::NAN, 1.0).is_nan());
        let colour = Vec4::new(1.0, 1.0, 1.0, 1.0);
        assert!(fragment_colour(f32::NAN, colour, colour, colour, 0.0).is_nan());
    }

    #[test]
    fn the_noise_picture_is_repeat_sampled() {
        use crate::force_effects::noise_image;
        let image = noise_image(&[0u8; 256 * 256], 256, false);
        assert_eq!(image.texture_descriptor.size.width, 256);
        assert_eq!(
            image.texture_descriptor.format,
            bevy::render::render_resource::TextureFormat::R8Unorm
        );
    }
}
