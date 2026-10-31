//! The Force effects' GPU programs: the blast, bolt and wave chunk ribbons
//! and the torrent's streams (`effectforce_v` with `effectforce_f` or
//! `effecttorrent_f`, [`ForceRibbonMaterial`]), the displacement tendril's
//! smoke (`effectsmoke_f` over the same vertex stage, also
//! [`ForceRibbonMaterial`] with `smoke`), the force-shell aura
//! (`effectshell_v`/`effectshell_f` over a copy of the caster's skinned
//! meshes or of a held body's or weapon's rigid ones, and `effectfrcstrk_f`
//! for a Force Strikes weapon aura, and `effectscan_f` for the teleporter
//! scan's aura, [`ForceShellMaterial`]), the displacement shell
//! (`dispshell_v`/`dispshell_f` over a copy of a vanishing body's skinned
//! meshes, and that copy's depth-only pass, [`DispShellMaterial`]), the
//! teleport bubble (`effecttele_v`/`effecttele_f` over the bubble's sphere,
//! [`TeleBubbleMaterial`]), and the animated `_ForceNoise` picture the force programs bind as `EmiTex`
//! ([`noise_image`], [`noise_mips`]).
//!
//! The render states are the materials' records: `_ForceMat01` and
//! `_ForceAura01` both carry the blend word 0x163 (source ONE, destination
//! ONE_MINUS_SRC_ALPHA, blend layer 6), which is Bevy's premultiplied
//! blend; the force record's flags 0x480018 make it two-sided, the aura's
//! 0x480008 one-sided. `_EffectShell01` (the displacement shell) and
//! `_ScanAura01` (the scan) carry the same blend word and flags; the
//! displacement shell's `EmiTex` is its record's emissive layer,
//! `FlameletsA3`. `_TeleBubble01` (the teleport bubble) carries the same
//! blend word and flags; its draw binds the `_ForceNoise` picture on unit 0
//! in place of its record's `ForceEfctA2E` layer. Blended draws write no
//! depth. Each fragment's output
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
    AsBindGroup, ColorWrites, Extent3d, RenderPipelineDescriptor, ShaderType,
    SpecializedMeshPipelineError, TextureDimension, TextureFormat,
};
use bevy::shader::ShaderRef;

pub struct ForceEffectsPlugin;

impl Plugin for ForceEffectsPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "force_ribbon.wesl");
        embedded_asset!(app, "force_shell.wesl");
        embedded_asset!(app, "disp_shell.wesl");
        embedded_asset!(app, "tele_bubble.wesl");
        app.add_plugins((
            MaterialPlugin::<ForceRibbonMaterial>::default(),
            MaterialPlugin::<ForceShellMaterial>::default(),
            MaterialPlugin::<DispShellMaterial>::default(),
            MaterialPlugin::<TeleBubbleMaterial>::default(),
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
/// The `_EffectShell01` record's blend word: ONE, ONE_MINUS_SRC_ALPHA,
/// layer 6, as the force records'.
pub const DISP_SHELL_BLEND_WORD: u16 = 0x163;
/// The `_EffectShell01` record's flags: emissive, one-sided.
pub const DISP_SHELL_RECORD_FLAGS: u32 = 0x0048_0008;
/// The `_EffectShell01` record's emissive layer: the picture the
/// displacement shell binds as `EmiTex`.
pub const DISP_SHELL_TEXTURE: &str = "flameletsa3";
/// The `_ScanAura01` record's blend word and flags: the aura's.
pub const SCAN_BLEND_WORD: u16 = 0x163;
pub const SCAN_RECORD_FLAGS: u32 = 0x0048_0008;
/// The `_TeleBubble01` record's blend word and flags: the aura's. The
/// record is one-sided; the bubble's mesh draws every face both ways.
pub const TELE_BUBBLE_BLEND_WORD: u16 = 0x163;
pub const TELE_BUBBLE_RECORD_FLAGS: u32 = 0x0048_0008;
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
        "disp" => embedded_path!("disp_shell.wesl"),
        "tele" => embedded_path!("tele_bubble.wesl"),
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
    /// The tendril's smoke program (`effectsmoke_f`) instead of the
    /// ribbon's: black with the smoke's alpha, premultiplied.
    pub smoke: bool,
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
            smoke: false,
        }
    }

    /// The tendril's smoke ribbon with `TexOfs` = `tex_ofs`.
    pub fn smoke(noise: Handle<Image>, tex_ofs: Vec3) -> Self {
        Self {
            params: ForceRibbonParams {
                tex_ofs: tex_ofs.extend(0.0),
            },
            noise,
            torrent: false,
            smoke: true,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct ForceRibbonKey {
    torrent: bool,
    smoke: bool,
}

impl From<&ForceRibbonMaterial> for ForceRibbonKey {
    fn from(m: &ForceRibbonMaterial) -> Self {
        Self {
            torrent: m.torrent,
            smoke: m.smoke,
        }
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
        if key.bind_group_data.smoke
            && let Some(fragment) = descriptor.fragment.as_mut()
        {
            fragment.shader_defs.push("SMOKE".into());
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
    /// `Colour` (the scan program only): the scan's colour times its
    /// colour scale.
    pub colour: Vec4,
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
            colour: Vec4::ZERO,
        }
    }
}

/// The force-shell aura's material, one per aura, shared by its copies.
/// With `strikes` it runs the strikes program (`effectfrcstrk_f`, the aura
/// Force Strikes puts on a weapon) over the same vertex stage, with `scan`
/// the teleporter scan's (`effectscan_f`, the `_ScanAura01` aura).
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
    /// The scan program instead of the shell's.
    pub scan: bool,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct ForceShellKey {
    strikes: bool,
    scan: bool,
}

impl From<&ForceShellMaterial> for ForceShellKey {
    fn from(m: &ForceShellMaterial) -> Self {
        Self {
            strikes: m.strikes,
            scan: m.scan,
        }
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
        if key.bind_group_data.scan
            && let Some(fragment) = descriptor.fragment.as_mut()
        {
            fragment.shader_defs.push("SCAN".into());
        }
        Ok(())
    }
}

/// The displacement shell program's uniform, laid out as `DispShellParams`
/// in `disp_shell.wesl`.
#[derive(Clone, Copy, Debug, PartialEq, ShaderType)]
pub struct DispShellParams {
    /// `LclMat` times the metre-to-centimetre scale.
    pub local_from_world: Mat4,
    /// `ViewPos` (cm, local); w unused.
    pub view_pos: Vec4,
    /// `FdPln`.
    pub fade_plane: Vec4,
    /// `TexPos` in xy, `TexOfs` in z.
    pub tex: Vec4,
    /// `Fade`: (1 − shown²)².
    pub fade: f32,
    /// `Interior`: 1 − the vanish's shown amount.
    pub interior: f32,
    /// `TexScale`.
    pub tex_scale: f32,
    /// `Opacity`.
    pub opacity: f32,
    /// The copy's push along its normal (m): 2 cm for the shell, none for
    /// the depth pass.
    pub push: f32,
}

impl Default for DispShellParams {
    fn default() -> Self {
        Self {
            local_from_world: Mat4::from_scale(Vec3::splat(100.0)),
            view_pos: Vec4::ZERO,
            fade_plane: Vec4::ZERO,
            tex: Vec4::ZERO,
            fade: 1.0,
            interior: 0.0,
            tex_scale: 0.015,
            opacity: 1.0,
            push: SHELL_PUSH_M,
        }
    }
}

/// The displacement shell's material (`dispshell_v`/`dispshell_f`, the
/// `_EffectShell01` record), one per shell, shared by its copies. With
/// `depth_only` it is the vanish's depth pass over the same copies: the
/// body's parts drawn with colour writes off and depth writes on, sorted
/// just before the shell.
#[derive(Asset, TypePath, AsBindGroup, Clone, Debug)]
#[bind_group_data(DispShellKey)]
pub struct DispShellMaterial {
    #[uniform(0)]
    pub params: DispShellParams,
    /// `EmiTex` (unit 0): the record's emissive layer, `FlameletsA3`.
    #[texture(1)]
    #[sampler(2)]
    pub texture: Handle<Image>,
    /// The depth pass instead of the shell.
    pub depth_only: bool,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct DispShellKey {
    depth_only: bool,
}

impl From<&DispShellMaterial> for DispShellKey {
    fn from(m: &DispShellMaterial) -> Self {
        Self {
            depth_only: m.depth_only,
        }
    }
}

/// How far ahead of the shell (in the blended draws' back-to-front sort)
/// the depth pass goes, so it is drawn first.
pub const DEPTH_PASS_SORT_BIAS: f32 = -0.001;

impl Material for DispShellMaterial {
    fn vertex_shader() -> ShaderRef {
        shader("disp")
    }

    fn fragment_shader() -> ShaderRef {
        shader("disp")
    }

    fn alpha_mode(&self) -> AlphaMode {
        if self.depth_only {
            // Drawn among the blended surfaces, after the opaque scene, as
            // the vanish's pass runs once the scene is down.
            AlphaMode::Blend
        } else {
            blend_alpha_mode(DISP_SHELL_BLEND_WORD).unwrap_or(AlphaMode::Premultiplied)
        }
    }

    fn depth_bias(&self) -> f32 {
        if self.depth_only {
            DEPTH_PASS_SORT_BIAS
        } else {
            0.0
        }
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
        if DISP_SHELL_RECORD_FLAGS & TWO_SIDED != 0 {
            descriptor.primitive.cull_mode = None;
        }
        if key.bind_group_data.depth_only {
            if let Some(depth) = descriptor.depth_stencil.as_mut() {
                depth.depth_write_enabled = Some(true);
            }
            if let Some(fragment) = descriptor.fragment.as_mut() {
                fragment.shader_defs.push("DEPTH_ONLY".into());
                for target in fragment.targets.iter_mut().flatten() {
                    target.write_mask = ColorWrites::empty();
                }
            }
        }
        Ok(())
    }
}

/// The teleport bubble program's uniforms (`effecttele_v`/`effecttele_f`).
#[derive(Clone, Copy, Debug, PartialEq, ShaderType)]
pub struct TeleBubbleParams {
    /// `ViewPos`: the camera in the bubble's frame (cm); w unused.
    pub view_pos: Vec4,
    /// `TexPos` in xy (the bubble's two creation draws); zw unused.
    pub tex_pos: Vec4,
    /// `Scale`: the swell.
    pub scale: f32,
    /// `Fade`.
    pub fade: f32,
    /// `Pulse`: the charge.
    pub pulse: f32,
    /// `Anim`: the rolling texture offset.
    pub anim: f32,
    /// `Colr`: the colour scale.
    pub colr: f32,
}

impl Default for TeleBubbleParams {
    fn default() -> Self {
        Self {
            view_pos: Vec4::ZERO,
            tex_pos: Vec4::ZERO,
            scale: 1.0,
            fade: 1.0,
            pulse: 0.0,
            anim: 0.0,
            colr: 1.0,
        }
    }
}

/// The teleport bubble's material (`effecttele_v`/`effecttele_f`, the
/// `_TeleBubble01` record): one a bubble, over a mesh in centimetres whose
/// entity carries the centimetre-to-metre scale.
#[derive(Asset, TypePath, AsBindGroup, Clone, Debug)]
pub struct TeleBubbleMaterial {
    #[uniform(0)]
    pub params: TeleBubbleParams,
    /// `EmiTex` (unit 0): the `_ForceNoise` picture.
    #[texture(1)]
    #[sampler(2)]
    pub noise: Handle<Image>,
}

impl Material for TeleBubbleMaterial {
    fn vertex_shader() -> ShaderRef {
        shader("tele")
    }

    fn fragment_shader() -> ShaderRef {
        shader("tele")
    }

    fn alpha_mode(&self) -> AlphaMode {
        blend_alpha_mode(TELE_BUBBLE_BLEND_WORD).unwrap_or(AlphaMode::Premultiplied)
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
        if TELE_BUBBLE_RECORD_FLAGS & TWO_SIDED != 0 {
            descriptor.primitive.cull_mode = None;
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
        program_fragment(include_str!("force_shell.wesl"), "ForceShellParams", defs)
    }

    /// A shell-like program's fragment stage (`source`, its uniform struct
    /// `uniform`), as [`shell_fragment`] cuts it.
    fn program_fragment(source: &str, uniform: &str, defs: &[&str]) -> String {
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
        let start = out.find(&format!("struct {uniform}")).expect("the uniform");
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

    fn validate(source: &str, defs: &[&str]) {
        let module = naga::front::wgsl::parse_str(source)
            .unwrap_or_else(|e| panic!("{defs:?}: {}\n{source}", e.emit_to_string(source)));
        naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::all(),
        )
        .validate(&module)
        .unwrap_or_else(|e| panic!("{defs:?}: {e:?}"));
    }

    #[test]
    fn the_scan_fragment_validates_with_its_own_terms() {
        let source = shell_fragment(&["SCAN"]);
        validate(&source, &["SCAN"]);
        // Fixed 0.03 coordinates, the fade term up to 100, no ripple
        // points, the `Colour` uniform's ramp and alpha limit, no
        // `ColourInt` on the rim.
        assert!(source.contains("a = 0.03;"));
        assert!(source.contains("f_max = 100.0;"));
        assert!(source.contains("points = 0;"));
        assert!(source.contains("fcol = shell.colour.rgb * 0.3;"));
        assert!(source.contains("alpha_scale = min(shell.colour.a, 1.0);"));
        assert!(!source.contains("va = va * colour_int;"));
        let plain = shell_fragment(&[]);
        assert!(plain.contains("va = va * colour_int;"));
        assert!(!plain.contains("f_max = 100.0;"));
    }

    #[test]
    fn the_displacement_shell_fragment_validates() {
        for defs in [&[][..], &["DEPTH_ONLY"][..]] {
            let source = program_fragment(include_str!("disp_shell.wesl"), "DispShellParams", defs);
            validate(&source, defs);
            assert!(source.contains("th = mix(th, tst, shell.interior);"));
            assert_eq!(source.contains("out = vec4(0.0);"), !defs.is_empty());
        }
    }

    #[test]
    fn the_tele_bubble_uniform_matches_the_shaders_layout() {
        let source = include_str!("tele_bubble.wesl");
        let size = wgsl_struct_size(source, "TeleBubbleParams");
        // Two vec4 and five words, padded to 16.
        assert_eq!(size, 64);
        assert_eq!(size, <TeleBubbleParams as ShaderType>::min_size().get());
        let offsets = wgsl_offsets(source, "TeleBubbleParams");
        let got: Vec<(&str, u32)> = offsets.iter().map(|(n, o)| (n.as_str(), *o)).collect();
        assert_eq!(
            got,
            [
                ("view_pos", 0),
                ("tex_pos", 16),
                ("scale", 32),
                ("fade", 36),
                ("pulse", 40),
                ("anim", 44),
                ("colr", 48),
            ]
        );
        assert_eq!(
            blend_alpha_mode(TELE_BUBBLE_BLEND_WORD),
            Some(AlphaMode::Premultiplied)
        );
        assert_eq!(TELE_BUBBLE_RECORD_FLAGS & TWO_SIDED, 0);
        // The program declares `TexOfs` but never reads it, and its alpha is 0.
        assert!(source.contains("vec4(col * col, 0.0)"));
        assert!(!source.contains("tex_ofs"));
    }

    #[test]
    fn the_displacement_shell_uniform_matches_the_shaders_layout() {
        let source = include_str!("disp_shell.wesl");
        let size = wgsl_struct_size(source, "DispShellParams");
        // A matrix, three vec4, four words and the push, padded to 16.
        assert_eq!(size, 144);
        assert_eq!(size, <DispShellParams as ShaderType>::min_size().get());
        let offsets = wgsl_offsets(source, "DispShellParams");
        let expected = [
            ("local_from_world", 0),
            ("view_pos", 64),
            ("fade_plane", 80),
            ("tex", 96),
            ("fade", 112),
            ("interior", 116),
            ("tex_scale", 120),
            ("opacity", 124),
            ("push", 128),
        ];
        let got: Vec<(&str, u32)> = offsets.iter().map(|(n, o)| (n.as_str(), *o)).collect();
        assert_eq!(got, expected);
        let params = DispShellParams {
            interior: 0.75,
            push: 0.0,
            tex: Vec4::new(0.25, 0.5, 0.125, 0.0),
            ..DispShellParams::default()
        };
        let mut buffer = bevy::render::render_resource::encase::UniformBuffer::new(Vec::new());
        buffer.write(&params).unwrap();
        let bytes = buffer.into_inner();
        let word = |at: usize| f32::from_le_bytes(bytes[at..at + 4].try_into().unwrap());
        assert_eq!(word(104), 0.125);
        assert_eq!(word(112), 1.0);
        assert_eq!(word(116), 0.75);
        assert_eq!(word(120), 0.015);
        assert_eq!(word(124), 1.0);
        assert_eq!(word(128), 0.0);
    }

    #[test]
    fn the_displacement_shell_and_scan_records_blend_as_the_aura() {
        assert_eq!(
            blend_alpha_mode(DISP_SHELL_BLEND_WORD),
            Some(AlphaMode::Premultiplied)
        );
        assert_eq!(DISP_SHELL_RECORD_FLAGS & TWO_SIDED, 0);
        assert_eq!(
            blend_alpha_mode(SCAN_BLEND_WORD),
            Some(AlphaMode::Premultiplied)
        );
        assert_eq!(SCAN_RECORD_FLAGS, AURA_RECORD_FLAGS);
        let shell = DispShellMaterial {
            params: DispShellParams::default(),
            texture: Handle::default(),
            depth_only: false,
        };
        let depth = DispShellMaterial {
            depth_only: true,
            ..shell.clone()
        };
        assert_eq!(shell.alpha_mode(), AlphaMode::Premultiplied);
        assert_eq!(depth.alpha_mode(), AlphaMode::Blend);
        assert_eq!(shell.depth_bias(), 0.0);
        assert!(depth.depth_bias() < 0.0);
        assert_ne!(DispShellKey::from(&shell), DispShellKey::from(&depth));
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
        // A matrix, two vec4, eight ripple points, a vec4, four words, the
        // strikes gradient and the scan colour.
        assert_eq!(size, 64 + 16 + 16 + 8 * 16 + 16 + 16 + 16 + 16);
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
            ("colour", 272),
        ];
        let got: Vec<(&str, u32)> = offsets.iter().map(|(n, o)| (n.as_str(), *o)).collect();
        assert_eq!(got, expected);
        // The Rust struct writes the same bytes at the same places.
        let params = ForceShellParams {
            colour_int: 2.0,
            push: 3.0,
            gradient: Vec4::new(0.0, 70.0, 0.0, 0.01),
            colour: Vec4::new(1.35, 2.85, 0.99, 3.0),
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
        assert_eq!(word(284), 3.0);
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
        let smoke = ForceRibbonMaterial::smoke(noise.clone(), Vec3::new(0.0, 0.3, 2.0));
        let torrent = ForceRibbonMaterial::new(noise, Vec3::Z, true);
        assert_eq!(blast.alpha_mode(), AlphaMode::Premultiplied);
        assert_eq!(blast.params.tex_ofs, Vec4::new(0.0, 4.0, 1.5, 0.0));
        assert_ne!(ForceRibbonKey::from(&blast), ForceRibbonKey::from(&torrent));
        // The tendril's smoke shares the vertex stage and the blend but not
        // the fragment body.
        assert!(smoke.smoke && !smoke.torrent);
        assert_eq!(smoke.alpha_mode(), AlphaMode::Premultiplied);
        assert_eq!(smoke.params.tex_ofs, Vec4::new(0.0, 0.3, 2.0, 0.0));
        assert_ne!(ForceRibbonKey::from(&blast), ForceRibbonKey::from(&smoke));
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
