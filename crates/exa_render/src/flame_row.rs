//! The flame row's material: the original's `_FlameMat01`, the ribbon drawn
//! along a lit blade.
//!
//! The program pair is the row's own. Its vertex stage takes the built
//! ribbon's `vPos`, `vTex` and RGBA8 `vCol` (the colour byte over 255, as
//! the original's `vCol · 0.003921568`), transforms by `ObjMat` and hands
//! `vrTex` and `vrCol` on. The fragment stage (`flame_row.wesl`) is the
//! original's procedural flame: four `EmiTex` samples over scrolled texture
//! coordinates, the row's vertical falloff and the palette lookup in
//! `GrdTex`.
//!
//! `Anim` is the per-object uniform: the rolling texture offset, twice its
//! negative, and half of one less the fade. `EmiTex` is the object's own
//! material texture on unit 0; `GrdTex` is the 1 × 5 palette below on
//! unit 1, a 0..1 ramp from transparent to warm white.
//!
//! The original's record blend state is not in the shipped code this module
//! reproduces; the fragment writes no alpha, so the draw is taken as
//! additive (a guess, as the palette's dark end would otherwise darken the
//! scene). The original draws the same ribbon once per supersample branch
//! with the branch's scissor; Exarust draws it once.
//!
//! Rules: `docs/RENDERING.md` (Fires), `exa_game::effects::fire_row`.

use bevy::asset::{AssetPath, RenderAssetUsages, embedded_asset, embedded_path};
use bevy::image::Image;
use bevy::mesh::MeshVertexBufferLayoutRef;
use bevy::pbr::{Material, MaterialPipeline, MaterialPipelineKey, MaterialPlugin};
use bevy::prelude::*;
use bevy::render::render_resource::{
    AsBindGroup, Extent3d, RenderPipelineDescriptor, ShaderType, SpecializedMeshPipelineError,
    TextureDimension, TextureFormat,
};
use bevy::shader::ShaderRef;

pub struct FlameRowPlugin;

impl Plugin for FlameRowPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "flame_row.wesl");
        app.add_plugins(MaterialPlugin::<FlameRowMaterial>::default());
    }
}

/// The `GrdTex` palette, top row last in the image: the original's 1 × 5
/// RGBA8 table. The fragment reads its red, green and blue; the alpha is
/// unused.
pub const GRADIENT_PALETTE: [[u8; 4]; 5] = [
    [0, 0, 0, 0],
    [64, 28, 9, 0],
    [132, 78, 16, 0],
    [190, 158, 99, 32],
    [255, 230, 200, 64],
];
/// The palette's width and height, texels.
pub const GRADIENT_WIDTH: u32 = 1;
pub const GRADIENT_HEIGHT: u32 = 5;

/// The palette as a texture. Its rows run bottom to top in the image's
/// storage order, the same order the original uploads.
pub fn gradient_image() -> Image {
    let data: Vec<u8> = GRADIENT_PALETTE.iter().flatten().copied().collect();
    Image::new(
        Extent3d {
            width: GRADIENT_WIDTH,
            height: GRADIENT_HEIGHT,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    )
}

/// The material's `Anim` uniform: the row's texture scroll and fade.
#[derive(Clone, Copy, Debug, PartialEq, ShaderType)]
pub struct FlameRowAnim {
    /// `Anim`: the rolling offset, twice its negative, and the fade's other
    /// half.
    pub anim: Vec4,
}

impl Default for FlameRowAnim {
    fn default() -> Self {
        Self { anim: Vec4::ZERO }
    }
}

/// `_FlameMat01` over a built flame ribbon.
#[derive(Asset, TypePath, AsBindGroup, Clone, Debug)]
pub struct FlameRowMaterial {
    #[uniform(0)]
    pub anim: FlameRowAnim,
    /// `EmiTex` (unit 0): the object material's own texture.
    #[texture(1)]
    #[sampler(2)]
    pub emissive: Handle<Image>,
    /// `GrdTex` (unit 1): [`gradient_image`].
    #[texture(3)]
    #[sampler(4)]
    pub gradient: Handle<Image>,
}

impl FlameRowMaterial {
    /// A row material reading `emissive` and the palette `gradient`.
    pub fn new(emissive: Handle<Image>, gradient: Handle<Image>) -> Self {
        Self {
            anim: FlameRowAnim::default(),
            emissive,
            gradient,
        }
    }

    /// The uniform for a frame at `now_ms` and a fade in 0..1
    /// (`exa_game::effects::fire_row::texture_animation`).
    pub fn set_animation(&mut self, now_ms: u32, fade: f32) {
        let offset = (f64::from(now_ms % 50_000) * 0.000_02) as f32;
        self.anim.anim = Vec4::new(offset, offset * -2.0, (1.0 - fade) * 0.5, 0.0);
    }
}

impl Material for FlameRowMaterial {
    fn fragment_shader() -> ShaderRef {
        ShaderRef::Path(
            AssetPath::from_path_buf(embedded_path!("flame_row.wesl")).with_source("embedded"),
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

    fn specialize(
        _pipeline: &MaterialPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        _layout: &MeshVertexBufferLayoutRef,
        _key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        // The ribbon is built as pairs of quads facing both ways.
        descriptor.primitive.cull_mode = None;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_palette_is_the_original_five_texel_ramp() {
        assert_eq!(GRADIENT_PALETTE.len(), 5);
        assert_eq!(GRADIENT_PALETTE[0], [0, 0, 0, 0]);
        assert_eq!(GRADIENT_PALETTE[4], [255, 230, 200, 64]);
        let image = gradient_image();
        assert_eq!(image.width(), 1);
        assert_eq!(image.height(), 5);
        assert_eq!(image.data.as_deref().unwrap().len(), 20);
        assert_eq!(&image.data.as_deref().unwrap()[..4], &[0, 0, 0, 0]);
        assert_eq!(&image.data.as_deref().unwrap()[16..], &[255, 230, 200, 64]);
    }

    #[test]
    fn the_animation_uniform_is_the_roll_and_half_the_fade() {
        let mut material = FlameRowMaterial::new(Handle::default(), Handle::default());
        material.set_animation(25_000, 0.0);
        assert_eq!(material.anim.anim, Vec4::new(0.5, -1.0, 0.5, 0.0));
        material.set_animation(50_000, 1.0);
        assert_eq!(material.anim.anim, Vec4::new(0.0, -0.0, 0.0, 0.0));
    }
}
