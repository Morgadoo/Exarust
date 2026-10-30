//! The Force effects' GPU programs: the blast, bolt and wave chunk ribbons
//! and the torrent's streams (`effectforce_v` with `effectforce_f` or
//! `effecttorrent_f`, [`ForceRibbonMaterial`]), the force-shell aura
//! (`effectshell_v`/`effectshell_f` over a copy of the caster's skinned
//! meshes or of a held body's or weapon's rigid ones, and `effectfrcstrk_f`
//! for a Force Strikes weapon aura, [`ForceShellMaterial`]), and the animated `_ForceNoise` picture
//! both bind as `EmiTex` ([`noise_image`], [`noise_mips`]).
//!
//! The render states are the materials' records: `_ForceMat01` and
//! `_ForceAura01` both carry the blend word 0x163 (source ONE, destination
//! ONE_MINUS_SRC_ALPHA, blend layer 6), which is Bevy's premultiplied
//! blend; the force record's flags 0x480018 make it two-sided, the aura's
//! 0x480008 one-sided. Blended draws write no depth. Each fragment's output
//! is clamped to [0, 1] as the original's 8-bit target clamps it.
//!
//! The original draws each effect once per supersample layer with the
//! layer's depth term ([`crate::supersample`] resolves Exarust's layers;
//! the per-layer plan is `exa_game::thaum::effect_pass`). Exarust draws
//! each effect once a frame.
//! Rules: `docs/RENDERING.md` (Force effects).

use bevy::asset::{AssetPath, RenderAssetUsages, embedded_asset, embedded_path};
use bevy::image::{ImageAddressMode, ImageFilterMode, ImageSampler, ImageSamplerDescriptor};
use bevy::mesh::MeshVertexBufferLayoutRef;
use bevy::pbr::{Material, MaterialPipeline, MaterialPipelineKey, MaterialPlugin};
use bevy::prelude::*;
use bevy::render::render_resource::{
    AsBindGroup, Extent3d, RenderPipelineDescriptor, ShaderType, SpecializedMeshPipelineError,
    TextureDimension, TextureFormat,
};
use bevy::shader::ShaderRef;

pub struct ForceEffectsPlugin;

impl Plugin for ForceEffectsPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "force_ribbon.wesl");
        embedded_asset!(app, "force_shell.wesl");
        app.add_plugins((
            MaterialPlugin::<ForceRibbonMaterial>::default(),
            MaterialPlugin::<ForceShellMaterial>::default(),
        ));
    }
}

/// The `_ForceMat01` and `_ForceAura01` blend word: source ONE (bits 8–11
/// = 1), destination ONE_MINUS_SRC_ALPHA (bits 0–3 = 3), layer 6.
pub const FORCE_BLEND_WORD: u16 = 0x163;
/// The `_ForceMat01` record's flags: emissive (0x8) and two-sided (0x10).
pub const FORCE_RECORD_FLAGS: u32 = 0x0048_0018;
/// The `_ForceAura01` record's flags: emissive, one-sided.
pub const AURA_RECORD_FLAGS: u32 = 0x0048_0008;
/// The record flags' two-sided bit.
pub const TWO_SIDED: u32 = 0x10;
/// The skinned shell copy's push along its normal: 2 cm.
pub const SHELL_PUSH_M: f32 = 0.02;

/// The blend a material's blend word names, by the original's ten-entry
/// factor table (`ZERO, ONE, SRC_ALPHA, ONE_MINUS_SRC_ALPHA, …`): the
/// premultiplied pair (ONE, ONE_MINUS_SRC_ALPHA) is Bevy's
/// [`AlphaMode::Premultiplied`], (SRC_ALPHA, ONE) and (ONE, ONE) its
/// additive modes; anything else is left to the caller.
pub fn blend_alpha_mode(word: u16) -> Option<AlphaMode> {
    match ((word >> 8) & 0xf, word & 0xf) {
        (1, 3) => Some(AlphaMode::Premultiplied),
        (2, 1) | (1, 1) => Some(AlphaMode::Add),
        (2, 3) => Some(AlphaMode::Blend),
        _ => None,
    }
}

fn shader(name: &str) -> ShaderRef {
    let path = match name {
        "ribbon" => embedded_path!("force_ribbon.wesl"),
        _ => embedded_path!("force_shell.wesl"),
    };
    ShaderRef::Path(AssetPath::from_path_buf(path).with_source("embedded"))
}

/// The ribbon program's uniform.
#[derive(Clone, Copy, Debug, Default, PartialEq, ShaderType)]
pub struct ForceRibbonParams {
    /// `TexOfs`; w unused.
    pub tex_ofs: Vec4,
}

/// A force ribbon: the blast family's chunks (`effectforce_f`) or the
/// torrent's streams (`effecttorrent_f`).
#[derive(Asset, TypePath, AsBindGroup, Clone, Debug)]
#[bind_group_data(ForceRibbonKey)]
pub struct ForceRibbonMaterial {
    #[uniform(0)]
    pub params: ForceRibbonParams,
    /// `EmiTex`: the `_ForceNoise` picture.
    #[texture(1)]
    #[sampler(2)]
    pub noise: Handle<Image>,
    /// The torrent's program instead of the blast's.
    pub torrent: bool,
}

impl ForceRibbonMaterial {
    /// A ribbon with `TexOfs` = `tex_ofs`.
    pub fn new(noise: Handle<Image>, tex_ofs: Vec3, torrent: bool) -> Self {
        Self {
            params: ForceRibbonParams {
                tex_ofs: tex_ofs.extend(0.0),
            },
            noise,
            torrent,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct ForceRibbonKey {
    torrent: bool,
}

impl From<&ForceRibbonMaterial> for ForceRibbonKey {
    fn from(m: &ForceRibbonMaterial) -> Self {
        Self { torrent: m.torrent }
    }
}

impl Material for ForceRibbonMaterial {
    fn fragment_shader() -> ShaderRef {
        shader("ribbon")
    }

    fn alpha_mode(&self) -> AlphaMode {
        blend_alpha_mode(FORCE_BLEND_WORD).unwrap_or(AlphaMode::Premultiplied)
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
        key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        if FORCE_RECORD_FLAGS & TWO_SIDED != 0 {
            descriptor.primitive.cull_mode = None;
        }
        if key.bind_group_data.torrent
            && let Some(fragment) = descriptor.fragment.as_mut()
        {
            fragment.shader_defs.push("TORRENT".into());
        }
        Ok(())
    }
}

/// The shell program's uniform, laid out as `ForceShellParams` in
/// `force_shell.wesl`.
#[derive(Clone, Copy, Debug, PartialEq, ShaderType)]
pub struct ForceShellParams {
    /// `LclMat` times the metre-to-centimetre scale.
    pub local_from_world: Mat4,
    /// `ViewPos` (cm, local); w unused.
    pub view_pos: Vec4,
    /// `FdPln`.
    pub fade_plane: Vec4,
    /// `ColPt`.
    pub colour_points: [Vec4; 8],
    /// `TexPos` in xy.
    pub tex_pos: Vec4,
    /// `ColourInt`.
    pub colour_int: f32,
    /// `TexScale`.
    pub tex_scale: f32,
    /// `Fade`.
    pub fade: f32,
    /// The copy's push along its normal (m).
    pub push: f32,
    /// `Gradient` (the strikes program only): the point the aura glows
    /// brightest at (cm, local) and the dimming per centimetre from it.
    pub gradient: Vec4,
}

impl Default for ForceShellParams {
    fn default() -> Self {
        Self {
            local_from_world: Mat4::from_scale(Vec3::splat(100.0)),
            view_pos: Vec4::ZERO,
            fade_plane: Vec4::ZERO,
            colour_points: [Vec4::ZERO; 8],
            tex_pos: Vec4::ZERO,
            colour_int: 1.0,
            tex_scale: 0.015,
            fade: 1.0,
            push: SHELL_PUSH_M,
            gradient: Vec4::ZERO,
        }
    }
}

/// The force-shell aura's material, one per aura, shared by its copies.
/// With `strikes` it runs the strikes program (`effectfrcstrk_f`, the aura
/// Force Strikes puts on a weapon) over the same vertex stage.
#[derive(Asset, TypePath, AsBindGroup, Clone, Debug)]
#[bind_group_data(ForceShellKey)]
pub struct ForceShellMaterial {
    #[uniform(0)]
    pub params: ForceShellParams,
    /// `EmiTex` (unit 3 in the original): the `_ForceNoise` picture.
    #[texture(1)]
    #[sampler(2)]
    pub noise: Handle<Image>,
    /// The strikes program instead of the shell's.
    pub strikes: bool,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct ForceShellKey {
    strikes: bool,
}

impl From<&ForceShellMaterial> for ForceShellKey {
    fn from(m: &ForceShellMaterial) -> Self {
        Self { strikes: m.strikes }
    }
}

impl Material for ForceShellMaterial {
    fn vertex_shader() -> ShaderRef {
        shader("shell")
    }

    fn fragment_shader() -> ShaderRef {
        shader("shell")
    }

    fn alpha_mode(&self) -> AlphaMode {
        blend_alpha_mode(FORCE_BLEND_WORD).unwrap_or(AlphaMode::Premultiplied)
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
        key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        if AURA_RECORD_FLAGS & TWO_SIDED != 0 {
            descriptor.primitive.cull_mode = None;
        }
        if key.bind_group_data.strikes
            && let Some(fragment) = descriptor.fragment.as_mut()
        {
            fragment.shader_defs.push("STRIKES".into());
        }
        Ok(())
    }
}

/// The noise picture's mip chain, as the upload's mipmap generation builds
/// it: level 0 is `pixels` (`side`² bytes), each next level the rounded
/// mean of each 2×2 block, down to one pixel. Answers the bytes of every
/// level in order and the level count.
pub fn noise_mips(pixels: &[u8], side: u32) -> (Vec<u8>, u32) {
    let mut out = pixels.to_vec();
    let mut level = pixels.to_vec();
    let mut size = side as usize;
    let mut levels = 1;
    while size > 1 {
        let half = size / 2;
        let next: Vec<u8> = (0..half * half)
            .map(|i| {
                let (x, y) = ((i % half) * 2, (i / half) * 2);
                let sum = u32::from(level[y * size + x])
                    + u32::from(level[y * size + x + 1])
                    + u32::from(level[(y + 1) * size + x])
                    + u32::from(level[(y + 1) * size + x + 1]);
                ((sum + 2) / 4) as u8
            })
            .collect();
        out.extend_from_slice(&next);
        level = next;
        size = half;
        levels += 1;
    }
    (out, levels)
}

/// The `_ForceNoise` texture: `side`² one-byte red texels (internal format
/// one unsigned byte a channel), repeating, filtered linearly, with its
/// mip chain when `mipmaps` (the parameter word's bit 0x40).
pub fn noise_image(pixels: &[u8], side: u32, mipmaps: bool) -> Image {
    let mut image = Image::new_uninit(
        Extent3d {
            width: side,
            height: side,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        TextureFormat::R8Unorm,
        RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD,
    );
    set_noise_pixels(&mut image, pixels, side, mipmaps);
    image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
        address_mode_u: ImageAddressMode::Repeat,
        address_mode_v: ImageAddressMode::Repeat,
        address_mode_w: ImageAddressMode::Repeat,
        mag_filter: ImageFilterMode::Linear,
        min_filter: ImageFilterMode::Linear,
        mipmap_filter: if mipmaps {
            ImageFilterMode::Linear
        } else {
            ImageFilterMode::Nearest
        },
        ..default()
    });
    image
}

/// Replaces the picture's texels (a fresh upload), rebuilding its mips.
pub fn set_noise_pixels(image: &mut Image, pixels: &[u8], side: u32, mipmaps: bool) {
    if mipmaps {
        let (data, levels) = noise_mips(pixels, side);
        image.texture_descriptor.mip_level_count = levels;
        image.data = Some(data);
    } else {
        image.texture_descriptor.mip_level_count = 1;
        image.data = Some(pixels.to_vec());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The size naga lays a WGSL struct out at, from its declaration in
    /// `source`.
    fn wgsl_struct_size(source: &str, name: &str) -> u64 {
        let start = source
            .find(&format!("struct {name} {{"))
            .unwrap_or_else(|| panic!("{name} is declared"));
        let end = start + source[start..].find("\n}").expect("the struct closes") + 2;
        let module_source = format!(
            "{}\n@group(0) @binding(0) var<uniform> probe: {name};\n",
            &source[start..end]
        );
        let module = naga::front::wgsl::parse_str(&module_source).expect("the struct parses");
        let mut layouter = naga::proc::Layouter::default();
        layouter
            .update(module.to_ctx())
            .expect("the struct lays out");
        let (handle, _) = module
            .types
            .iter()
            .find(|(_, ty)| ty.name.as_deref() == Some(name))
            .expect("the struct is a type");
        u64::from(layouter[handle].size)
    }

    /// The byte offsets naga gives a WGSL struct's members, by name.
    fn wgsl_offsets(source: &str, name: &str) -> Vec<(String, u32)> {
        let start = source.find(&format!("struct {name} {{")).unwrap();
        let end = start + source[start..].find("\n}").unwrap() + 2;
        let module = naga::front::wgsl::parse_str(&source[start..end]).unwrap();
        let (_, ty) = module
            .types
            .iter()
            .find(|(_, ty)| ty.name.as_deref() == Some(name))
            .unwrap();
        match &ty.inner {
            naga::TypeInner::Struct { members, .. } => members
                .iter()
                .map(|m| (m.name.clone().unwrap_or_default(), m.offset))
                .collect(),
            _ => panic!("{name} is a struct"),
        }
    }

    #[test]
    fn the_ribbon_uniform_matches_the_shaders_layout() {
        let source = include_str!("force_ribbon.wesl");
        let size = wgsl_struct_size(source, "ForceRibbonParams");
        assert_eq!(size, 16);
        assert_eq!(size, <ForceRibbonParams as ShaderType>::min_size().get());
    }

    /// The shell program's fragment stage with the `@if` blocks taken as
    /// `defs` defines them, the vertex stage and the imports cut out and the
    /// Bevy vertex output stubbed, so naga can parse and validate it.
    fn shell_fragment(defs: &[&str]) -> String {
        let source = include_str!("force_shell.wesl");
        let defined = |cond: &str| {
            cond.split("&&").all(|term| {
                let term = term.trim();
                match term.strip_prefix('!') {
                    Some(name) => !defs.contains(&name),
                    None => defs.contains(&term),
                }
            })
        };
        let mut out = String::new();
        let mut stack: Vec<bool> = Vec::new();
        for line in source.lines() {
            if let Some(cond) = line
                .strip_prefix("@if(")
                .and_then(|rest| rest.strip_suffix(") {"))
            {
                stack.push(defined(cond));
                continue;
            }
            if line == "}" && !stack.is_empty() {
                stack.pop();
                continue;
            }
            if stack.iter().all(|&on| on) {
                out.push_str(line);
                out.push('\n');
            }
        }
        let start = out.find("struct ForceShellParams").expect("the uniform");
        let vertex = out.find("@vertex").expect("the vertex stage");
        let vertex_end = vertex + out[vertex..].find("\n}\n").expect("its end") + 3;
        let body = format!("{}{}", &out[start..vertex], &out[vertex_end..]);
        format!(
            "struct VertexOutput {{\n    @builtin(position) position: vec4<f32>,\n    \
             @location(0) world_position: vec4<f32>,\n    \
             @location(1) world_normal: vec3<f32>,\n}}\n{}",
            body.replace("constants::MATERIAL_BIND_GROUP", "2")
        )
    }

    #[test]
    fn the_shell_and_strikes_fragments_validate() {
        for defs in [&[][..], &["STRIKES"][..]] {
            let source = shell_fragment(defs);
            let module = naga::front::wgsl::parse_str(&source)
                .unwrap_or_else(|e| panic!("{defs:?}: {}\n{source}", e.emit_to_string(&source)));
            naga::valid::Validator::new(
                naga::valid::ValidationFlags::all(),
                naga::valid::Capabilities::all(),
            )
            .validate(&module)
            .unwrap_or_else(|e| panic!("{defs:?}: {e:?}"));
            assert_eq!(source.contains("a = 0.01;"), !defs.is_empty());
            assert_eq!(source.contains("shell.gradient.w"), !defs.is_empty());
        }
    }

    #[test]
    fn the_shell_uniform_matches_the_shaders_layout() {
        let source = include_str!("force_shell.wesl");
        let size = wgsl_struct_size(source, "ForceShellParams");
        // A matrix, two vec4, eight ripple points, a vec4, four words and
        // the strikes gradient.
        assert_eq!(size, 64 + 16 + 16 + 8 * 16 + 16 + 16 + 16);
        assert_eq!(size, <ForceShellParams as ShaderType>::min_size().get());
        let offsets = wgsl_offsets(source, "ForceShellParams");
        let expected = [
            ("local_from_world", 0),
            ("view_pos", 64),
            ("fade_plane", 80),
            ("colour_points", 96),
            ("tex_pos", 224),
            ("colour_int", 240),
            ("tex_scale", 244),
            ("fade", 248),
            ("push", 252),
            ("gradient", 256),
        ];
        let got: Vec<(&str, u32)> = offsets.iter().map(|(n, o)| (n.as_str(), *o)).collect();
        assert_eq!(got, expected);
        // The Rust struct writes the same bytes at the same places.
        let params = ForceShellParams {
            colour_int: 2.0,
            push: 3.0,
            gradient: Vec4::new(0.0, 70.0, 0.0, 0.01),
            ..ForceShellParams::default()
        };
        let mut buffer = bevy::render::render_resource::encase::UniformBuffer::new(Vec::new());
        buffer.write(&params).unwrap();
        let bytes = buffer.into_inner();
        let word = |at: usize| f32::from_le_bytes(bytes[at..at + 4].try_into().unwrap());
        assert_eq!(word(0), 100.0);
        assert_eq!(word(240), 2.0);
        assert_eq!(word(244), 0.015);
        assert_eq!(word(252), 3.0);
        assert_eq!(word(260), 70.0);
        assert_eq!(word(268), 0.01);
    }

    #[test]
    fn the_force_records_blend_premultiplied() {
        assert_eq!(
            blend_alpha_mode(FORCE_BLEND_WORD),
            Some(AlphaMode::Premultiplied)
        );
        // The arcs' 0x161 is additive and the default 0x203 alpha-blended.
        assert_eq!(blend_alpha_mode(0x161), Some(AlphaMode::Add));
        assert_eq!(blend_alpha_mode(0x203), Some(AlphaMode::Blend));
        assert_eq!(blend_alpha_mode(0x100), None);
        assert_ne!(FORCE_RECORD_FLAGS & TWO_SIDED, 0);
        assert_eq!(AURA_RECORD_FLAGS & TWO_SIDED, 0);
        let noise = Handle::<Image>::default();
        let blast = ForceRibbonMaterial::new(noise.clone(), Vec3::new(0.0, 4.0, 1.5), false);
        let torrent = ForceRibbonMaterial::new(noise, Vec3::Z, true);
        assert_eq!(blast.alpha_mode(), AlphaMode::Premultiplied);
        assert_eq!(blast.params.tex_ofs, Vec4::new(0.0, 4.0, 1.5, 0.0));
        assert_ne!(ForceRibbonKey::from(&blast), ForceRibbonKey::from(&torrent));
    }

    #[test]
    fn the_noise_mips_average_each_block_down_to_one_texel() {
        let pixels: Vec<u8> = (0..16).map(|i| (i * 16) as u8).collect();
        let (data, levels) = noise_mips(&pixels, 4);
        assert_eq!(levels, 3);
        assert_eq!(data.len(), 16 + 4 + 1);
        // (0 + 16 + 64 + 80 + 2) / 4 = 40.
        assert_eq!(data[16], 40);
        assert_eq!(data[20], ((40u32 + 72 + 168 + 200 + 2) / 4) as u8);
        let (_, full) = noise_mips(&vec![0; 256 * 256], 256);
        assert_eq!(full, 9);
    }

    #[test]
    fn the_noise_image_is_one_red_byte_a_texel() {
        let image = noise_image(&vec![7; 256 * 256], 256, true);
        assert_eq!(image.texture_descriptor.format, TextureFormat::R8Unorm);
        assert_eq!(image.texture_descriptor.mip_level_count, 9);
        assert_eq!(image.data.as_ref().unwrap().len(), 87_381);
        let flat = noise_image(&vec![7; 256 * 256], 256, false);
        assert_eq!(flat.texture_descriptor.mip_level_count, 1);
        assert_eq!(flat.data.as_ref().unwrap().len(), 0x1_0000);
    }
}
