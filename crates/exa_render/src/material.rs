//! The original's surface material: its colours, opacity and gloss, and its
//! diffuse, normal, specular, emissive and opacity maps, shaded like its
//! `shadeopt` program (`exa.wesl`). Rules: `docs/RENDERING.md`.

use bevy::asset::{AssetPath, embedded_asset, embedded_path};
use bevy::mesh::MeshVertexBufferLayoutRef;
use bevy::pbr::{Material, MaterialPipeline, MaterialPipelineKey, MaterialPlugin};
use bevy::prelude::*;
use bevy::render::render_resource::{
    AsBindGroup, RenderPipelineDescriptor, ShaderType, SpecializedMeshPipelineError,
};
use bevy::render::storage::ShaderBuffer;
use bevy::shader::ShaderRef;

pub struct MaterialsPlugin;

impl Plugin for MaterialsPlugin {
    fn build(&self, app: &mut App) {
        bevy::shader::load_shader_library!(app, "shading.wesl");
        embedded_asset!(app, "exa.wesl");
        embedded_asset!(app, "exa_prepass.wesl");
        app.add_plugins(MaterialPlugin::<ExaMaterial>::default())
            .init_resource::<FadeToBlack>()
            .add_systems(PostUpdate, (mark_shadowless, sync_fade_to_black));
    }
}

/// Whether lit surfaces fade to black with their horizontal distance from
/// the world's origin, black from [`FADE_RADIUS`] out, as the original's
/// character screen fades its floor (its environment's second fog mode).
/// Every surface material follows it.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FadeToBlack(pub bool);

/// Horizontal distance from the origin (m) at which [`FadeToBlack`] reaches
/// black; the fade is a·(2 − a) with a = min(distance / radius, 1).
pub const FADE_RADIUS: f32 = 2.0;

/// Keeps every material's fade bit equal to [`FadeToBlack`].
fn sync_fade_to_black(
    fade: Res<FadeToBlack>,
    mut materials: ResMut<Assets<ExaMaterial>>,
    mut events: MessageReader<AssetEvent<ExaMaterial>>,
) {
    let added: Vec<AssetId<ExaMaterial>> = events
        .read()
        .filter_map(|e| match e {
            AssetEvent::Added { id } => Some(*id),
            _ => None,
        })
        .collect();
    let ids: Vec<AssetId<ExaMaterial>> = if fade.is_changed() {
        materials.ids().collect()
    } else {
        added
    };
    for id in ids {
        let Some(m) = materials.get(id) else { continue };
        if (m.params.flags & flags::FADE_TO_BLACK != 0) == fade.0 {
            continue;
        }
        if let Some(mut m) = materials.get_mut(id) {
            m.params.flags ^= flags::FADE_TO_BLACK;
        }
    }
}

/// Meshes whose material casts no shadows leave the shadow maps.
fn mark_shadowless(
    mut commands: Commands,
    added: Query<(Entity, &MeshMaterial3d<ExaMaterial>), Added<MeshMaterial3d<ExaMaterial>>>,
    materials: Res<Assets<ExaMaterial>>,
) {
    for (e, m) in &added {
        if materials.get(&m.0).is_some_and(|m| m.no_shadows) {
            commands.entity(e).insert(bevy::light::NotShadowCaster);
        }
    }
}

/// Flags telling the shader which maps and paths a material uses.
pub mod flags {
    pub const DIFFUSE_MAP: u32 = 1;
    pub const NORMAL_MAP: u32 = 2;
    pub const SPECULAR_MAP: u32 = 4;
    pub const EMISSIVE_MAP: u32 = 8;
    pub const OPACITY_MAP: u32 = 16;
    pub const LIT: u32 = 32;
    /// The specular map packs intensity, gloss and metalness.
    pub const PACKED_SPECULAR: u32 = 64;
    pub const TWO_SIDED: u32 = 128;
    /// The environment cube lights and reflects the surface.
    pub const ENVIRONMENT: u32 = 256;
    /// A dirt map roughens the surface where it deviates from mid-grey.
    pub const DIRT_MAP: u32 = 512;
    /// A gloss map scales the glossiness.
    pub const GLOSS_MAP: u32 = 1024;
    /// Ambient comes from the voxel GI volume.
    pub const GI: u32 = 2048;
    /// A height map displaces the texture coordinates (parallax).
    pub const HEIGHT_MAP: u32 = 4096;
    /// The height map also shadows the lights.
    pub const DISPLACEMENT_SHADOWS: u32 = 8192;
    /// Debug view: the surface shows its raw GI sample (`EXA_GI_DEBUG`).
    pub const DEBUG_GI: u32 = 16384;
    /// The frame is supersampled: each sub-frame takes its share of the
    /// shadow and GI taps ([`crate::supersample::SplitTaps`]).
    pub const SPLIT_TAPS: u32 = 32768;
    /// Skin: light scatters under the surface ([`super::ExaParams::subsurface`]),
    /// as the original's subsurface program draws a body's skin.
    pub const SUBSURFACE: u32 = 65536;
    /// The surface fades to black away from the origin
    /// ([`super::FadeToBlack`]).
    pub const FADE_TO_BLACK: u32 = 131072;
    /// The diffuse map times the diffuse colour is an sRGB colour, decoded
    /// after the product: a map read as stored (not decoded) is tinted in
    /// sRGB space, as the original bakes a character's skin texture from the
    /// race's undecoded one, without encoding, into an sRGB texture.
    pub const TINT_AS_SRGB: u32 = 262144;
    /// The surface adds its colour weighted by its alpha (the original's
    /// SRC_ALPHA, ONE), drawn through Bevy's premultiplied blending.
    pub const ADDITIVE: u32 = 524288;
    /// The surface's alpha gains the opacity fall-off with the view angle
    /// ([`super::ExaParams::opacity_falloff`]); unlit surfaces only with an
    /// emissive map.
    pub const OPACITY_FALLOFF: u32 = 1048576;
}

#[derive(Clone, Copy, Debug, ShaderType)]
pub struct ExaParams {
    /// Linear rgb (×1.5 for legacy materials); a = opacity, less the
    /// opacity fall-off's floor.
    pub diffuse: Vec4,
    /// Linear rgb × specular level; w = glossiness.
    pub specular: Vec4,
    /// Linear rgb (×2 for lit materials with an emissive map); a = the
    /// opacity, for unlit materials.
    pub emissive: Vec4,
    pub flags: u32,
    pub pad0: u32,
    pub pad1: u32,
    pub pad2: u32,
    /// The GI volume's lowest corner (m), and its size (m).
    pub gi_origin: Vec4,
    pub gi_extent: Vec4,
    /// Parallax scale per UV axis (xy), displacement-shadow step (z) and
    /// bias (w).
    pub displacement: Vec4,
    /// With [`flags::SUBSURFACE`]: the share of the diffuse colour that
    /// scatters under the surface (linear rgb; the rest is plain diffuse),
    /// and the blur (w): the mip level of the normal map the scattered light
    /// reads, which also widens how far the light wraps past the terminator.
    pub subsurface: Vec4,
    /// The opacity fall-off: alpha += 2^((x − N·V)·y)·z, with x = 1 and
    /// y < 0 when it fades toward the silhouette, x = 0 and y > 0 when
    /// toward the centre; a lit surface's emissive map is weighted by
    /// N·V²·z + 1 − z (zero for none).
    pub opacity_falloff: Vec4,
}

impl ExaParams {
    /// The opacity fall-off of a record's amount (`0x64`) and exponent
    /// (`0x68`), as the original packs it: the amount term
    /// (2^(−1.8·exponent) + 1)·amount and the exponent term 1.8·exponent are
    /// both negated when `inverted` (extra flag `0x100`); z is the amount
    /// term's magnitude, x is 1 when the amount term is negative, and y is
    /// the exponent term. With the usual positive amount, an inverted
    /// fall-off fades toward the silhouette.
    pub fn falloff(amount: f32, exponent: f32, inverted: bool) -> Vec4 {
        let f = (-1.8 * exponent).exp2();
        let sign = if inverted { -1.0 } else { 1.0 };
        let term = (f + 1.0) * amount * sign;
        let x = if term < 0.0 { 1.0 } else { 0.0 };
        Vec4::new(x, 1.8 * exponent * sign, term.abs(), 0.0)
    }
}

impl Default for ExaParams {
    fn default() -> Self {
        Self {
            diffuse: Vec4::ONE,
            specular: Vec4::new(0.04, 0.04, 0.04, 0.3),
            emissive: Vec4::new(0.0, 0.0, 0.0, 1.0),
            flags: flags::LIT,
            pad0: 0,
            pad1: 0,
            pad2: 0,
            gi_origin: Vec4::ZERO,
            gi_extent: Vec4::ONE,
            displacement: Vec4::ZERO,
            subsurface: Vec4::ZERO,
            opacity_falloff: Vec4::ZERO,
        }
    }
}

/// A surface shaded like the original's.
#[derive(Asset, TypePath, AsBindGroup, Clone, Debug)]
#[bind_group_data(ExaMaterialKey)]
pub struct ExaMaterial {
    #[uniform(100)]
    pub params: ExaParams,
    #[texture(101)]
    #[sampler(102)]
    pub diffuse_map: Option<Handle<Image>>,
    #[texture(103)]
    #[sampler(104)]
    pub normal_map: Option<Handle<Image>>,
    #[texture(105)]
    #[sampler(106)]
    pub specular_map: Option<Handle<Image>>,
    #[texture(107)]
    #[sampler(108)]
    pub emissive_map: Option<Handle<Image>>,
    #[texture(109)]
    #[sampler(110)]
    pub opacity_map: Option<Handle<Image>>,
    /// The scene's environment cube (`genref01r` indoors).
    #[texture(111, dimension = "cube")]
    #[sampler(112)]
    pub environment: Option<Handle<Image>>,
    #[texture(113)]
    #[sampler(114)]
    pub dirt_map: Option<Handle<Image>>,
    #[texture(115)]
    #[sampler(116)]
    pub gloss_map: Option<Handle<Image>>,
    /// The voxel GI light volume (8-bit sRGB, sampled through an sRGB view).
    #[texture(117, dimension = "3d")]
    #[sampler(118)]
    pub gi_volume: Option<Handle<Image>>,
    /// Height map (red, linear) for parallax and displacement shadows.
    #[texture(119)]
    #[sampler(120)]
    pub height_map: Option<Handle<Image>>,
    /// Light records and mesh light lists ([`crate::lights`]); the default
    /// handle is the shared buffer.
    #[storage(130, read_only)]
    pub lights: Handle<ShaderBuffer>,
    /// The cached cube shadow pool ([`crate::cube_shadows`]).
    #[texture(131, dimension = "cube_array", sample_type = "depth")]
    #[sampler(132, sampler_type = "comparison")]
    pub shadow_cubes: Handle<Image>,
    pub alpha_mode: AlphaMode,
    /// The surface casts no shadows (opacity ≤ 0.8, or the record says so).
    pub no_shadows: bool,
    /// The record casts through the original's alpha-tested caster,
    /// whatever the surface's blending: an alpha-tested blend word, or the
    /// opacity flag with an opacity above 0.75 (see
    /// [`ExaMaterial::cuts_shadow`]).
    pub cut_shadow: bool,
}

impl Default for ExaMaterial {
    fn default() -> Self {
        Self {
            params: ExaParams::default(),
            diffuse_map: None,
            normal_map: None,
            specular_map: None,
            emissive_map: None,
            opacity_map: None,
            environment: None,
            dirt_map: None,
            gloss_map: None,
            gi_volume: None,
            height_map: None,
            lights: Handle::default(),
            shadow_cubes: crate::cube_shadows::CUBE_POOL,
            alpha_mode: AlphaMode::default(),
            no_shadows: false,
            cut_shadow: false,
        }
    }
}

impl ExaMaterial {
    /// Whether the surface casts as the original's alpha-tested caster
    /// does: both sides, dropping the texels whose cut texture (the opacity
    /// map, else the diffuse map) has an alpha below ½. Alpha-tested
    /// surfaces and those marked [`ExaMaterial::cut_shadow`] (cobwebs).
    pub fn cuts_shadow(&self) -> bool {
        self.cut_shadow || matches!(self.alpha_mode, AlphaMode::Mask(_))
    }

    /// Sets the flags of the maps present, keeping the shading flags.
    pub fn with_map_flags(mut self) -> Self {
        let f = &mut self.params.flags;
        for (map, bit) in [
            (self.diffuse_map.is_some(), flags::DIFFUSE_MAP),
            (self.normal_map.is_some(), flags::NORMAL_MAP),
            (self.specular_map.is_some(), flags::SPECULAR_MAP),
            (self.emissive_map.is_some(), flags::EMISSIVE_MAP),
            (self.opacity_map.is_some(), flags::OPACITY_MAP),
            (self.environment.is_some(), flags::ENVIRONMENT),
            (self.dirt_map.is_some(), flags::DIRT_MAP),
            (self.gloss_map.is_some(), flags::GLOSS_MAP),
        ] {
            if map {
                *f |= bit;
            } else {
                *f &= !bit;
            }
        }
        self
    }
}

impl From<Color> for ExaMaterial {
    /// A plain lit surface of this colour.
    fn from(c: Color) -> Self {
        let l = c.to_linear();
        Self {
            params: ExaParams {
                diffuse: Vec4::new(l.red, l.green, l.blue, l.alpha),
                ..default()
            },
            ..default()
        }
    }
}

/// What the pipeline needs to know about a material.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct ExaMaterialKey {
    two_sided: bool,
}

impl From<&ExaMaterial> for ExaMaterialKey {
    fn from(m: &ExaMaterial) -> Self {
        Self {
            two_sided: m.params.flags & flags::TWO_SIDED != 0,
        }
    }
}

impl Material for ExaMaterial {
    fn fragment_shader() -> ShaderRef {
        ShaderRef::Path(
            AssetPath::from_path_buf(embedded_path!("exa.wesl")).with_source("embedded"),
        )
    }

    fn prepass_fragment_shader() -> ShaderRef {
        ShaderRef::Path(
            AssetPath::from_path_buf(embedded_path!("exa_prepass.wesl")).with_source("embedded"),
        )
    }

    fn alpha_mode(&self) -> AlphaMode {
        self.alpha_mode
    }

    fn specialize(
        _pipeline: &MaterialPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        _layout: &MeshVertexBufferLayoutRef,
        key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        if key.bind_group_data.two_sided {
            descriptor.primitive.cull_mode = None;
        }
        Ok(())
    }
}

/// The sRGB transfer function's inverse, as the original applies to material
/// and light colours.
pub fn linear(c: f32) -> f32 {
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

/// An sRGB colour as linear rgb.
pub fn linear_rgb(c: [f32; 3]) -> Vec3 {
    Vec3::new(linear(c[0]), linear(c[1]), linear(c[2]))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The alpha the shader gives an unlit surface with fall-off.
    fn falloff_alpha(opacity: f32, f: Vec4, ndv: f32) -> f32 {
        opacity + ((f.x - ndv) * f.y).exp2() * f.z
    }

    #[test]
    fn salve_glow_fades_toward_its_silhouette() {
        // The salve's glow: opacity 0, amount 1, exponent 2, inverted.
        let f = ExaParams::falloff(1.0, 2.0, true);
        assert!((f.z - 1.0825).abs() < 1e-4);
        assert!((falloff_alpha(0.0, f, 1.0) - 1.0825).abs() < 1e-4);
        assert!((falloff_alpha(0.0, f, 0.0) - 0.0893).abs() < 1e-4);
        // Not inverted, the rim is the bright part.
        let rim = ExaParams::falloff(1.0, 2.0, false);
        assert!((falloff_alpha(0.0, rim, 0.0) - 1.0825).abs() < 1e-4);
        assert!(falloff_alpha(0.0, rim, 1.0) < 0.09);
    }

    #[test]
    fn negative_falloff_amount_flips_the_offset_only() {
        // A negative amount (`wraithskin`: −1, exponent 1, not inverted)
        // makes the packed amount term negative: x = 1 and the magnitude,
        // with the exponent term as the flag leaves it.
        let f = ExaParams::falloff(-1.0, 1.0, false);
        assert_eq!((f.x, f.y), (1.0, 1.8));
        assert!((f.z - (2f32.powf(-1.8) + 1.0)).abs() < 1e-6);
        // Inverted as well, the two negations cancel in x.
        let g = ExaParams::falloff(-1.0, 1.0, true);
        assert_eq!((g.x, g.y), (0.0, -1.8));
        assert_eq!(g.z, f.z);
    }
}
