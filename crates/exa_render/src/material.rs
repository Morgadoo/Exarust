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
        bevy::shader::load_shader_library!(app, "hair.wesl");
        bevy::shader::load_shader_library!(app, "shadesmk.wesl");
        bevy::shader::load_shader_library!(app, "sun_caster.wesl");
        embedded_asset!(app, "exa.wesl");
        embedded_asset!(app, "exa_prepass.wesl");
        embedded_asset!(app, "sun_caster_prepass.wesl");
        crate::sun_casters::load_caster_shaders(app);
        app.add_plugins(MaterialPlugin::<ExaMaterial>::default())
            .init_resource::<FadeToBlack>()
            .add_systems(
                PostUpdate,
                (mark_shadowless, sync_fade_to_black, order_blended_states),
            );
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

/// Places every blended surface where the original's scene walk draws it:
/// when a material is added or removed, the registered states are ranked
/// in list order ([`crate::passes::blended_ranks`]: the decal list, then
/// the blended list in its layer and opacity order) and each material's
/// transparent-phase bias is set from its rank. Only changed values are
/// written. Rules: `docs/RENDERING.md` (Pass order).
fn order_blended_states(
    mut materials: ResMut<Assets<ExaMaterial>>,
    mut events: MessageReader<AssetEvent<ExaMaterial>>,
) {
    let changed = events
        .read()
        .any(|e| matches!(e, AssetEvent::Added { .. } | AssetEvent::Removed { .. }));
    if !changed {
        return;
    }
    let (ids, states): (Vec<AssetId<ExaMaterial>>, Vec<crate::passes::BlendState>) = materials
        .iter()
        .filter_map(|(id, m)| m.blend_state.map(|state| (id, state)))
        .unzip();
    let ranks = crate::passes::blended_ranks(&states);
    let decals = states.iter().filter(|state| state.decal).count();
    let layers = states.iter().filter(|state| state.layer != 0).count();
    bevy::log::debug!(
        "blended walk: {} states ({decals} decal, {} blended, {layers} on a layer above 0)",
        states.len(),
        states.len() - decals
    );
    for (index, rank) in ranks {
        let bias = crate::passes::order_bias(states[index].decal, rank);
        if materials
            .get(ids[index])
            .is_some_and(|m| m.draw_order != bias)
            && let Some(mut material) = materials.get_mut(ids[index])
        {
            material.draw_order = bias;
        }
    }
}

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
    /// The surface reflects the environment cube inside the GI volume. A
    /// library record needs its reflection layer kept (`Mat.TexRef`);
    /// without it the original's specular GI there is a quarter of the
    /// ambient. Outside the volume every surface reflects the cube.
    pub const ENVIRONMENT: u32 = 256;
    /// A dirt texture is bound (the blood speckle, [`super::BloodProgram`]).
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
    /// Blood, as the original's apparel and item program shows it (dirt
    /// mode 4, [`super::BloodProgram::Cloth`]).
    pub const BLOOD: u32 = 2097152;
    /// Blood, as the original's skin program shows it
    /// ([`super::BloodProgram::Skin`]).
    pub const BLOOD_SKIN: u32 = 4194304;
    /// Blood, as the original's hair program shows it
    /// ([`super::BloodProgram::Hair`]).
    pub const BLOOD_HAIR: u32 = 8388608;
    /// The diffuse map is a single-channel BC4 image, uploaded as the
    /// original's luminance LATC format: its red channel stands for all
    /// three colour channels, and there is no sRGB variant to decode it.
    pub const GREY_DIFFUSE: u32 = 16777216;
    /// The diffuse map skips the LOD bias: a record with a secondary colour
    /// loads its diffuse that way (the original's per-slot skip bit).
    pub const SKIP_DIFFUSE_BIAS: u32 = 33554432;
    /// The specular map is a single-channel BC4 image, uploaded as the
    /// original's luminance LATC format: the shader repeats its red channel
    /// over rgb, as the hardware would for a luminance upload.
    pub const GREY_SPECULAR: u32 = 67108864;
    /// The emissive map is a single-channel BC4 image: its red channel is
    /// the whole colour, as [`Self::GREY_SPECULAR`].
    pub const GREY_EMISSIVE: u32 = 134217728;
    /// The record draws with the hair program (`shadehar_f`, program class
    /// 3): anisotropic strand highlights, a wrapping rim and the diffuse
    /// map's colour only under blood ([`crate::hair_program`]).
    pub const HAIR: u32 = 268435456;
    /// The record's material word lacks `MatSpc` (`0x4`): every program
    /// leaves its whole specular sum (lights and reflection) out.
    pub const NO_SPECULAR: u32 = 536870912;
    /// The level has an open sky: inside the GI volume the surface takes
    /// the original's outdoor GI shading ([`crate::gi::GiOutdoor`]).
    pub const OUTDOOR: u32 = 1073741824;
    /// Debug view (`EXA_GI_DEBUG=direct` or `=ambient`): the frame split in
    /// its two lights. Alone, the surface leaves its GI ambient and
    /// reflection out (the direct lights only); with [`DEBUG_GI`] it leaves
    /// the direct lights out (the GI's share only).
    pub const DEBUG_SPLIT: u32 = 2147483648;
}

/// The second flag word ([`ExaParams::flags2`]).
pub mod flags2 {
    /// The original's local reflection (`Shd.LocalRef`, shader word
    /// `0x400000`): inside the GI volume the cube reflection is mixed with
    /// the previous frame's blurred bright pass read just off the fragment's
    /// own screen position ([`crate::local_reflection`]).
    pub const LOCAL_REFLECTION: u32 = 1;
    /// `EXA_LOCAL_REF_DEBUG=1`: a [`LOCAL_REFLECTION`] surface inside the GI
    /// volume shows its low-field read × 4 and nothing else.
    pub const DEBUG_LOCAL_REFLECTION: u32 = 2;
    /// The camera's cut-away fade, blended: the surface draws with
    /// [`super::ExaParams::cut_away_opacity`] in place of its diffuse
    /// alpha (the material is blended, writing depth while its opacity is
    /// above zero). The shadow passes ignore it, as the original's do.
    pub const CUT_AWAY_BLEND: u32 = 4;
    /// The camera's cut-away fade, settled and supersampled: the surface
    /// draws opaque into half of each 2 × 2 block of the supersampled
    /// scene (the block's diagonal), as the original draws a settled mesh
    /// into half of its jittered sub-frames.
    pub const CUT_AWAY_HALF: u32 = 8;
    /// The original's dirt mode 1 (grime) in its PBR program: the dirt map
    /// at the texture coordinates, pulled to mid grey by the vertex alpha
    /// times [`super::ExaParams::dirt`]`.w`, doubles into the diffuse colour
    /// and roughens the surface ([`super::DirtMode`]).
    pub const DIRT_GRIME: u32 = 16;
    /// The original's dirt mode 2 (wear): the dirt map at the texture
    /// coordinates times the diffuse map's aspect
    /// ([`super::ExaParams::dirt`]`.xy`) shows where the vertex alpha falls
    /// and darkens the surface ([`super::DirtMode`]).
    pub const DIRT_WORN: u32 = 32;
    /// The dirt map is a single-channel BC4 image (the original's luminance
    /// upload): its red channel stands for all three.
    pub const GREY_DIRT: u32 = 64;
    /// The original's grass program (`shading_v` + `shadegrs_f`), which its
    /// terrain plants draw with: the normal never flipped on a back face
    /// (with the normal map, built on the vertex tangent as it comes),
    /// roughness from the diffuse map's colour (clamp(1 − 3·(g − 0.8·r),
    /// 0.01, 0.99)²), the fast light loop (`lightsfst`: no Fresnel share in
    /// the diffuse, Fresnel 0.04) and three quarters of the specular sum.
    pub const GRASS: u32 = 128;
    /// The original's smoke and mist program (`shading_v` + `shadesmk_f`),
    /// which every type-2 record draws with
    /// ([`crate::material_state::draws_smoke`]): black, its alpha
    /// ([`crate::smoke::smoke_alpha`]) from the facing ratio, the opacity map
    /// read twice at the scrolled offset ([`crate::smoke::smoke_uv_offset`])
    /// and the vertex alpha; nothing under the level's distance fog, where
    /// the shipped program does not build ([`crate::smoke::program_builds`]).
    pub const SMOKE: u32 = 256;
}

/// The original's dirt modes 1 and 2, which only its PBR program
/// (`shadeopt`) shades: a record with a diffuse map and a dirt map whose
/// extra flags (`0x4C & 0x7000`) are 0 (grime) or `0x1000` (wear), drawn
/// on a mesh that stores a colour stream. The stream's alpha α (1 where the
/// mesh is clean) chooses how much of the dirt map shows. Mode 3
/// (`0x2000`) binds the dirt map but the program has no branch for it, and
/// mode 4 is the blood ([`BloodProgram`]). `exa.wesl` evaluates the same
/// formulas. Rules: `docs/RENDERING.md` (Dirt).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DirtMode {
    /// Mode 1: the dirt texel d (read at the texture coordinates) is pulled
    /// to mid grey by min(α·take, 1); the diffuse colour takes 2·d and the
    /// roughness r becomes 1 − (1 − r)(1 − 2|½ − d.g|).
    Grime,
    /// Mode 2: the dirt texel d is read at the texture coordinates times
    /// the diffuse map's aspect; with w = clamp(α·3|0.4 − d.r|, 0, 1) the
    /// roughness becomes 1 − (1 − r)·w and F0 mix(0.04, F0, w), and the
    /// diffuse colour c becomes mix(0.75·α·d, c, α)·α.
    Worn,
}

impl DirtMode {
    /// The mode of the factory's shader-word dirt field (1 grime, 2 wear);
    /// none for the others.
    pub fn of_mode(mode: u32) -> Option<Self> {
        match mode {
            1 => Some(Self::Grime),
            2 => Some(Self::Worn),
            _ => None,
        }
    }

    /// The mode a record shows at shading quality `quality`: the factory's
    /// shader word carries mode 1 or 2 (the record has the diffuse bit and
    /// a dirt layer, its extra flags `0x7000` are 0 or `0x1000`) and the
    /// word selects the PBR program. The dirt map must still load.
    pub fn of_record(record: &exa_formats::material::Material, quality: u32) -> Option<Self> {
        use crate::shader_program::{SurfaceProgram, program_for_shader_word, shader, state_words};
        let (_, word) = state_words(record, quality)?;
        if program_for_shader_word(word) != SurfaceProgram::Optimized {
            return None;
        }
        Self::of_mode((word & shader::DIRT_MODE_MASK) >> shader::DIRT_MODE_SHIFT)
    }

    /// The material's second-word flag that selects the mode in the shader.
    pub fn flag(self) -> u32 {
        match self {
            Self::Grime => flags2::DIRT_GRIME,
            Self::Worn => flags2::DIRT_WORN,
        }
    }

    /// The dirt map's texture-coordinate scale (`DrtUVs`) from the diffuse
    /// map's size: with f = w/h, (f, 1) when f ≥ 1, else (1, 1/f). The
    /// uniform fill computes it at each draw for modes 1 to 3; `exa.wesl`
    /// does the same from the bound map.
    pub fn uv_scale(width: u32, height: u32) -> Vec2 {
        let f = width as f32 / height as f32;
        if f >= 1.0 {
            Vec2::new(f, 1.0)
        } else {
            Vec2::new(1.0, 1.0 / f)
        }
    }

    /// The take `MatDrt.w` of a record's dirt amount (`0x30`): `2 − amount`.
    pub fn take(amount: f32) -> f32 {
        2.0 - amount
    }

    /// One pixel: the dirt texel `dirt` (rgb), the vertex alpha `alpha`, the
    /// take, and the surface's roughness, F0 and diffuse colour before the
    /// dirt; returns the three after it.
    pub fn shade(
        self,
        dirt: Vec3,
        alpha: f32,
        take: f32,
        rough: f32,
        f0: Vec3,
        colour: Vec3,
    ) -> (f32, Vec3, Vec3) {
        match self {
            Self::Grime => {
                let d = dirt.lerp(Vec3::splat(0.5), (alpha * take).min(1.0));
                let rough = 1.0 - (1.0 - rough) * (1.0 - (0.5 - d.y).abs() * 2.0);
                (rough, f0, colour * d * 2.0)
            }
            Self::Worn => {
                let w = (alpha * ((0.4 - dirt.x).abs() * 3.0)).clamp(0.0, 1.0);
                let rough = 1.0 - (1.0 - rough) * w;
                let f0 = Vec3::splat(0.04).lerp(f0, w);
                let d = dirt * alpha * 0.75;
                (rough, f0, d.lerp(colour, alpha) * alpha)
            }
        }
    }
}

/// How a surface shows the blood that wounds paint into its mesh's vertex
/// alpha: the original's dirt mode 4, in one of the three programs that
/// have it. The vertex alpha α (1 clean, lowered around wounds) is compared
/// with the tiling speckle texture [`SPATTER`] (value a, read at UV×2, at
/// UV×1 for hair): the blood amount is da = clamp((a − α)/a·k, 0, 1), so a
/// lightly painted vertex shows only the speckle's bright droplets and a
/// thoroughly painted one solid blood. Blood darkens the diffuse colour
/// toward a dark red ([`BloodProgram::tint`]) and wets the surface
/// ([`BloodProgram::roughness`]). Meshes without a vertex colour stream are
/// clean. `exa.wesl` evaluates the same formulas. Rules:
/// `docs/RENDERING.md` (Blood).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BloodProgram {
    /// Apparel, factory items and bleeding races' skins (the original's
    /// `shadeopt` program): k = 1.3.
    Cloth,
    /// The human skin (`shadesss`): k = 1, the vertex alpha floored at
    /// 0.02.
    Skin,
    /// Hair (`shadehar`): k = 1, the speckle at UV×1.
    Hair,
}

/// The dark red blood tints toward, in linear light (skin and cloth).
pub const BLOOD_TINT: Vec3 = Vec3::new(0.6, 0.03, 0.0);
/// The hair program's blood colour (its dirt colour in mode 4).
pub const HAIR_BLOOD_TINT: Vec3 = Vec3::new(0.45, 0.03, 0.02);
/// The skin program reads the vertex alpha no lower than this.
pub const SKIN_ALPHA_FLOOR: f32 = 0.02;
/// The cloth program's gain on the blood amount.
pub const CLOTH_BLOOD_GAIN: f32 = 1.3;
/// The texture whose speckle shows blood (`Resource.rpk`).
pub const SPATTER: &str = "spatter01";
/// The roughness thick blood brings cloth and hair to.
const WET_ROUGHNESS: f32 = 0.2;

impl BloodProgram {
    /// The program of a material record's type, when it has dirt modes:
    /// type 1 (PBR, the `shadeopt` program), 3 (hair) and 4 (skin). Types
    /// 0 (legacy) and 2 (unlit) have none.
    pub fn of_type(kind: u32) -> Option<Self> {
        match kind {
            1 => Some(Self::Cloth),
            3 => Some(Self::Hair),
            4 => Some(Self::Skin),
            _ => None,
        }
    }

    /// The material flag that selects the program in the shader.
    pub fn flag(self) -> u32 {
        match self {
            Self::Cloth => flags::BLOOD,
            Self::Skin => flags::BLOOD_SKIN,
            Self::Hair => flags::BLOOD_HAIR,
        }
    }

    /// How many times the speckle tiles over the texture coordinates.
    pub fn spatter_scale(self) -> f32 {
        match self {
            Self::Hair => 1.0,
            _ => 2.0,
        }
    }

    /// The blood amount da of a pixel with speckle value `spatter` (a) and
    /// interpolated vertex alpha `alpha` (α, 0..1).
    pub fn amount(self, spatter: f32, alpha: f32) -> f32 {
        let (alpha, gain) = match self {
            Self::Cloth => (alpha, CLOTH_BLOOD_GAIN),
            Self::Skin => (alpha.max(SKIN_ALPHA_FLOOR), 1.0),
            Self::Hair => (alpha, 1.0),
        };
        ((spatter - alpha) / spatter.max(1e-5) * gain).clamp(0.0, 1.0)
    }

    /// The factor the diffuse colour takes at blood amount `da`, clamped to
    /// [0, 1]: skin (0.6, 0.03, 0)·(1 − da²/2)·(1 − 0.3·da) + (1 − da),
    /// cloth (0.6, 0.03, 0)·(1 − 0.3·da) + (1 − da), hair the same with
    /// (0.45, 0.03, 0.02). Thick blood (da = 1) is (0.21, 0.0105, 0) on
    /// skin and (0.42, 0.021, 0) on cloth.
    pub fn tint(self, da: f32) -> Vec3 {
        let dark = 1.0 - 0.3 * da;
        let colour = match self {
            Self::Skin => BLOOD_TINT * (1.0 - da * da * 0.5) * dark,
            Self::Cloth => BLOOD_TINT * dark,
            Self::Hair => HAIR_BLOOD_TINT * dark,
        };
        (colour + Vec3::splat(1.0 - da)).clamp(Vec3::ZERO, Vec3::ONE)
    }

    /// The roughness of a surface of roughness `rough` under blood amount
    /// `da` with speckle value `spatter` (a): skin loses da·(0.1 + 0.6·a)
    /// down to 0.1; cloth moves toward 0.2 by da²·(0.5·a + 0.5), hair by
    /// da²·(0.7·a + 0.3). Thick blood is wet, and brighter speckles wetter.
    pub fn roughness(self, rough: f32, da: f32, spatter: f32) -> f32 {
        let toward = |w: f32| rough + (WET_ROUGHNESS - rough) * w;
        match self {
            Self::Skin => (rough - da * (0.1 + 0.6 * spatter)).max(0.1),
            Self::Cloth => toward(da * da * (0.5 * spatter + 0.5)),
            Self::Hair => toward(da * da * (0.7 * spatter + 0.3)),
        }
    }
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
    /// The LOD bias the shader adds to its colour-texture samples: the
    /// texture settings' value at the material's build (−0.75 at the retail
    /// texture quality with 4x supersampling).
    pub texture_bias: f32,
    /// The second flag word ([`flags2`]).
    pub flags2: u32,
    /// The camera's cut-away fade ([`flags2::CUT_AWAY_BLEND`]): the
    /// opacity that replaces the diffuse alpha, one less the mesh's fade
    /// amount, as the original's faded pass writes it into the material's
    /// `MatDif.a` for the draw. Unused without that bit.
    pub cut_away_opacity: f32,
    /// The camera's GI block's sample origin (m; the per-object pick's slot
    /// 0, [`ExaParams::gi_origins`]) and the shared sample size (m).
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
    /// The per-object GI pick ([`crate::gi::GiSlots`]): the block grid's
    /// low edge in world centimetres (xy) and the mode (z: 1 picks by the
    /// object's origin, 0 samples slot 0).
    pub gi_grid: Vec4,
    /// Each resident block's cell in the block grid (xy), z = 1 when the
    /// slot holds a block.
    pub gi_cells: [IVec4; crate::gi::RESIDENT_BLOCKS],
    /// Each resident block's sample origin (m).
    pub gi_origins: [Vec4; crate::gi::RESIDENT_BLOCKS],
    /// The dirt modes' input ([`flags2::DIRT_GRIME`],
    /// [`flags2::DIRT_WORN`]): the take `2 − amount` (`MatDrt.w`) in w. The
    /// wear's coordinate scale (`DrtUVs`, [`DirtMode::uv_scale`]) is the
    /// bound diffuse map's aspect, which the shader reads from the map at
    /// each draw as the original's uniform fill reads the texture's size.
    pub dirt: Vec4,
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
            texture_bias: -0.75,
            flags2: 0,
            cut_away_opacity: 1.0,
            gi_origin: Vec4::ZERO,
            gi_extent: Vec4::ONE,
            displacement: Vec4::ZERO,
            subsurface: Vec4::ZERO,
            opacity_falloff: Vec4::ZERO,
            gi_grid: Vec4::ZERO,
            gi_cells: [IVec4::ZERO; crate::gi::RESIDENT_BLOCKS],
            gi_origins: [Vec4::ZERO; crate::gi::RESIDENT_BLOCKS],
            dirt: Vec4::ZERO,
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
    /// The scene's environment cube (`genref01r` indoors). It also lights the
    /// surface outside the GI volume; only [`flags::ENVIRONMENT`] materials
    /// reflect it.
    #[texture(111, dimension = "cube")]
    #[sampler(112)]
    pub environment: Option<Handle<Image>>,
    #[texture(113)]
    #[sampler(114)]
    pub dirt_map: Option<Handle<Image>>,
    #[texture(115)]
    #[sampler(116)]
    pub gloss_map: Option<Handle<Image>>,
    /// The voxel GI light volume (8-bit sRGB, sampled through an sRGB view)
    /// of the camera's block: the per-object pick's slot 0.
    #[texture(117, dimension = "3d")]
    #[sampler(118)]
    pub gi_volume: Option<Handle<Image>>,
    /// The resident neighbours' light volumes, slots 1 to 8 of the pick
    /// (read through `gi_volume`'s sampler).
    #[texture(121, dimension = "3d")]
    pub gi_volume_1: Option<Handle<Image>>,
    #[texture(122, dimension = "3d")]
    pub gi_volume_2: Option<Handle<Image>>,
    #[texture(123, dimension = "3d")]
    pub gi_volume_3: Option<Handle<Image>>,
    #[texture(124, dimension = "3d")]
    pub gi_volume_4: Option<Handle<Image>>,
    #[texture(125, dimension = "3d")]
    pub gi_volume_5: Option<Handle<Image>>,
    #[texture(126, dimension = "3d")]
    pub gi_volume_6: Option<Handle<Image>>,
    #[texture(127, dimension = "3d")]
    pub gi_volume_7: Option<Handle<Image>>,
    #[texture(128, dimension = "3d")]
    pub gi_volume_8: Option<Handle<Image>>,
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
    /// The scene's low field (the previous frame's quarter-size blurred
    /// bright pass) that a [`flags2::LOCAL_REFLECTION`] surface reads; the
    /// original's `FrmBuf` on unit 7.
    #[texture(133)]
    #[sampler(134)]
    pub scene_low: Option<Handle<Image>>,
    pub alpha_mode: AlphaMode,
    /// The surface casts no shadows (opacity ≤ 0.8, or the record says so).
    pub no_shadows: bool,
    /// The record casts through the original's alpha-tested caster,
    /// whatever the surface's blending: an alpha-tested blend word, or the
    /// opacity flag with an opacity above 0.75 (see
    /// [`ExaMaterial::cuts_shadow`]).
    pub cut_shadow: bool,
    /// When set, this pass culls that face. Cloth submits one material per
    /// pass: front, then back.
    pub cloth_cull: Option<bevy::render::render_resource::Face>,
    /// The render state a blended record registers in the original's
    /// decal or blended list; `None` for every other surface.
    pub blend_state: Option<crate::passes::BlendState>,
    /// The list walk's place for this surface on the transparent phase
    /// ([`crate::passes::order_bias`]), kept by [`order_blended_states`];
    /// 0 sorts by view depth alone.
    pub draw_order: f32,
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
            gi_volume_1: None,
            gi_volume_2: None,
            gi_volume_3: None,
            gi_volume_4: None,
            gi_volume_5: None,
            gi_volume_6: None,
            gi_volume_7: None,
            gi_volume_8: None,
            height_map: None,
            lights: Handle::default(),
            shadow_cubes: crate::cube_shadows::CUBE_POOL,
            scene_low: None,
            alpha_mode: AlphaMode::default(),
            no_shadows: false,
            cut_shadow: false,
            cloth_cull: None,
            blend_state: None,
            draw_order: 0.0,
        }
    }
}

impl ExaMaterial {
    /// Binds the resident GI blocks in their slots, with the half-voxel
    /// lift the original's per-object lookup gives most surfaces.
    pub fn set_gi_slots(&mut self, slots: &crate::gi::GiSlots) {
        let [s0, s1, s2, s3, s4, s5, s6, s7, s8] = slots.lights.clone();
        self.gi_volume = s0;
        self.gi_volume_1 = s1;
        self.gi_volume_2 = s2;
        self.gi_volume_3 = s3;
        self.gi_volume_4 = s4;
        self.gi_volume_5 = s5;
        self.gi_volume_6 = s6;
        self.gi_volume_7 = s7;
        self.gi_volume_8 = s8;
        self.params.gi_origin = slots.origins[0];
        self.params.gi_extent = slots.extent;
        self.params.gi_grid = slots.grid;
        self.params.gi_cells = slots.cells;
        self.params.gi_origins = slots.origins;
    }

    /// Whether the surface casts as the original's alpha-tested caster
    /// does: both sides, dropping the texels whose cut texture (the opacity
    /// map, else the diffuse map) has an alpha below ½. Alpha-tested
    /// surfaces and those marked [`ExaMaterial::cut_shadow`] (cobwebs).
    pub fn cuts_shadow(&self) -> bool {
        self.cut_shadow || matches!(self.alpha_mode, AlphaMode::Mask(_))
    }

    /// One material per cloth draw. Each pass culls the face the pass names,
    /// so the pipeline submits the front and the back separately.
    pub fn cloth_draws(&self, culls: &[bevy::render::render_resource::Face]) -> Vec<Self> {
        culls
            .iter()
            .copied()
            .map(|face| {
                let mut pass = self.clone();
                pass.cloth_cull = Some(face);
                pass
            })
            .collect()
    }

    /// The surface shows wound blood with `program`, the speckle texture
    /// `spatter` bound as its dirt map. Without the speckle it shows none.
    pub fn with_blood(mut self, program: BloodProgram, spatter: Option<Handle<Image>>) -> Self {
        self.params.flags &= !(flags::BLOOD | flags::BLOOD_SKIN | flags::BLOOD_HAIR);
        if spatter.is_some() {
            self.params.flags |= program.flag();
        }
        self.dirt_map = spatter;
        self.with_map_flags()
    }

    /// The surface shows dirt mode `mode` through `map` (a single-channel
    /// image when `grey`) with the take of the record's dirt `amount`
    /// ([`DirtMode`]). Only meshes with a colour
    /// stream show it.
    pub fn with_dirt(
        mut self,
        mode: DirtMode,
        map: Handle<Image>,
        grey: bool,
        amount: f32,
    ) -> Self {
        self.params.flags2 &= !(flags2::DIRT_GRIME | flags2::DIRT_WORN | flags2::GREY_DIRT);
        self.params.flags2 |= mode.flag();
        if grey {
            self.params.flags2 |= flags2::GREY_DIRT;
        }
        self.params.dirt = Vec4::new(0.0, 0.0, 0.0, DirtMode::take(amount));
        self.dirt_map = Some(map);
        self.with_map_flags()
    }

    /// The dirt mode the surface shows, if any.
    pub fn dirt(&self) -> Option<DirtMode> {
        [DirtMode::Grime, DirtMode::Worn]
            .into_iter()
            .find(|m| self.params.flags2 & m.flag() != 0)
    }

    /// The blood program the surface shows wounds with, if any.
    pub fn blood(&self) -> Option<BloodProgram> {
        [BloodProgram::Cloth, BloodProgram::Skin, BloodProgram::Hair]
            .into_iter()
            .find(|p| self.params.flags & p.flag() != 0)
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

/// Sets every loaded material's texture-bias base: the original sets the
/// LOD bias from the texture quality's global on every bind (unless the
/// texture's word carries the skip bit), so a quality change takes effect
/// on the materials already built, not only on those built afterwards.
/// Returns how many materials changed.
pub fn set_texture_bias(materials: &mut Assets<ExaMaterial>, bias: f32) -> usize {
    let mut changed = 0;
    for (_, material) in materials.iter_mut() {
        if material.params.texture_bias != bias {
            material.params.texture_bias = bias;
            changed += 1;
        }
    }
    changed
}

/// What the pipeline needs to know about a material.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct ExaMaterialKey {
    two_sided: bool,
    cloth_cull: Option<bevy::render::render_resource::Face>,
    /// The surface casts the alpha-tested way ([`ExaMaterial::cuts_shadow`]).
    cuts_shadow: bool,
    /// The surface is a cut-away fade that writes depth (blended, opacity
    /// above zero): the original lays the faded mesh's depth first, so
    /// only its nearest layer shows.
    cut_away_depth: bool,
}

impl From<&ExaMaterial> for ExaMaterialKey {
    fn from(m: &ExaMaterial) -> Self {
        Self {
            two_sided: m.params.flags & flags::TWO_SIDED != 0,
            cloth_cull: m.cloth_cull,
            cuts_shadow: m.cuts_shadow(),
            cut_away_depth: m.params.flags2 & flags2::CUT_AWAY_BLEND != 0
                && m.params.cut_away_opacity > 0.0,
        }
    }
}

impl ExaMaterialKey {
    /// The face this pass culls, when the material is one cloth draw.
    pub fn cloth_cull(self) -> Option<bevy::render::render_resource::Face> {
        self.cloth_cull
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

    /// Bevy's transparent phase sorts by view depth plus this value; the
    /// original walks its blended surfaces by list and rank instead.
    fn depth_bias(&self) -> f32 {
        self.draw_order
    }

    fn specialize(
        _pipeline: &MaterialPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        _layout: &MeshVertexBufferLayoutRef,
        key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        let data = key.bind_group_data;
        // A draw into the sun map takes the original's caster state.
        if crate::sun_casters::specialize_sun_caster(
            descriptor,
            key.mesh_key,
            crate::sun_casters::CasterShader::Material,
            crate::sun_casters::sun_caster_cull(data.cuts_shadow, data.two_sided, data.cloth_cull),
            data.cuts_shadow,
        ) {
            return Ok(());
        }
        if data.cut_away_depth
            && let Some(depth) = descriptor.depth_stencil.as_mut()
        {
            depth.depth_write_enabled = Some(true);
        }
        if let Some(face) = key.bind_group_data.cloth_cull {
            descriptor.primitive.cull_mode = Some(face);
        } else if key.bind_group_data.two_sided {
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

    /// A texture-quality change rewrites the bias of every loaded material,
    /// as the original's bind does on each draw; one already at the value
    /// is left alone.
    #[test]
    fn a_quality_change_rewrites_loaded_materials_bias() {
        let mut materials = Assets::<ExaMaterial>::default();
        let plain = materials.add(ExaMaterial::default());
        let other = materials.add(ExaMaterial {
            params: ExaParams {
                texture_bias: -0.25,
                ..default()
            },
            ..default()
        });
        assert_eq!(materials.get(&plain).unwrap().params.texture_bias, -0.75);
        assert_eq!(set_texture_bias(&mut materials, -0.25), 1);
        assert_eq!(materials.get(&plain).unwrap().params.texture_bias, -0.25);
        assert_eq!(materials.get(&other).unwrap().params.texture_bias, -0.25);
        assert_eq!(set_texture_bias(&mut materials, -0.25), 0);
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

    /// The original's blood colours at full blood (its skin, apparel and
    /// hair programs): skin (0.21, 0.0105, 0), cloth (0.42, 0.021, 0), hair
    /// (0.315, 0.021, 0.014); a clean pixel keeps its colour.
    #[test]
    fn thick_blood_is_dark_red() {
        let close = |a: Vec3, b: Vec3| (a - b).abs().max_element() < 1e-6;
        assert!(close(
            BloodProgram::Skin.tint(1.0),
            Vec3::new(0.21, 0.0105, 0.0)
        ));
        assert!(close(
            BloodProgram::Cloth.tint(1.0),
            Vec3::new(0.42, 0.021, 0.0)
        ));
        assert!(close(
            BloodProgram::Hair.tint(1.0),
            Vec3::new(0.315, 0.021, 0.014)
        ));
        for p in [BloodProgram::Skin, BloodProgram::Cloth, BloodProgram::Hair] {
            assert_eq!(p.tint(0.0), Vec3::ONE);
        }
        // Half blood on skin: (0.6, 0.03, 0)·0.875·0.85 + 0.5.
        assert!(close(
            BloodProgram::Skin.tint(0.5),
            Vec3::new(0.94625, 0.5223125, 0.5)
        ));
    }

    /// The blood amount: (a − α)/a·k, k 1.3 for cloth, the skin's alpha no
    /// lower than 0.02, and a black speckle texel showing nothing.
    #[test]
    fn blood_shows_where_the_alpha_falls_below_the_speckle() {
        let close = |a: f32, b: f32| (a - b).abs() < 1e-6;
        // A vertex painted to 128 (α 0.502) shows blood only on speckle
        // texels above it: bright droplets first.
        let alpha = 128.0 / 255.0;
        assert_eq!(BloodProgram::Skin.amount(0.4, alpha), 0.0);
        assert!(close(BloodProgram::Skin.amount(1.0, alpha), 1.0 - alpha));
        assert!(close(
            BloodProgram::Cloth.amount(1.0, alpha),
            (1.0 - alpha) * 1.3
        ));
        assert!(close(BloodProgram::Hair.amount(1.0, alpha), 1.0 - alpha));
        // At α 0 the whole speckle fills in: solid blood on cloth and hair;
        // on skin the floor keeps the faintest texels clean.
        assert_eq!(BloodProgram::Cloth.amount(0.05, 0.0), 1.0);
        assert_eq!(BloodProgram::Skin.amount(0.02, 0.0), 0.0);
        assert!(close(BloodProgram::Skin.amount(0.5, 0.0), 0.96));
        // Clean (α 1) shows none anywhere, and a black texel never bleeds.
        for p in [BloodProgram::Skin, BloodProgram::Cloth, BloodProgram::Hair] {
            assert_eq!(p.amount(1.0, 1.0), 0.0);
            assert_eq!(p.amount(0.0, 0.0), 0.0);
        }
        assert_eq!(BloodProgram::Hair.spatter_scale(), 1.0);
        assert_eq!(BloodProgram::Skin.spatter_scale(), 2.0);
        assert_eq!(BloodProgram::Cloth.spatter_scale(), 2.0);
    }

    /// Thick blood is wet: skin loses da·(0.1 + 0.6·a) down to 0.1, cloth
    /// and hair move toward 0.2.
    #[test]
    fn blood_wets_the_surface() {
        let close = |a: f32, b: f32| (a - b).abs() < 1e-6;
        assert!(close(BloodProgram::Skin.roughness(0.6, 1.0, 0.5), 0.2));
        assert!(close(BloodProgram::Skin.roughness(0.6, 1.0, 1.0), 0.1));
        assert!(close(BloodProgram::Skin.roughness(0.6, 0.0, 1.0), 0.6));
        assert!(close(BloodProgram::Cloth.roughness(0.8, 1.0, 1.0), 0.2));
        // da 1, a 0: half way for cloth, 0.3 of the way for hair.
        assert!(close(BloodProgram::Cloth.roughness(0.8, 1.0, 0.0), 0.5));
        assert!(close(BloodProgram::Hair.roughness(0.8, 1.0, 0.0), 0.62));
        assert!(close(BloodProgram::Hair.roughness(0.8, 0.0, 1.0), 0.8));
    }

    #[test]
    fn blood_programs_follow_the_record_type() {
        assert_eq!(BloodProgram::of_type(1), Some(BloodProgram::Cloth));
        assert_eq!(BloodProgram::of_type(3), Some(BloodProgram::Hair));
        assert_eq!(BloodProgram::of_type(4), Some(BloodProgram::Skin));
        assert_eq!(BloodProgram::of_type(0), None);
        assert_eq!(BloodProgram::of_type(2), None);
        let spatter = Some(Handle::<Image>::default());
        let m = ExaMaterial::default().with_blood(BloodProgram::Skin, spatter.clone());
        assert_eq!(m.blood(), Some(BloodProgram::Skin));
        assert_ne!(m.params.flags & flags::DIRT_MAP, 0);
        let m = m.with_blood(BloodProgram::Cloth, spatter);
        assert_eq!(m.blood(), Some(BloodProgram::Cloth));
        assert_eq!(m.params.flags & flags::BLOOD_SKIN, 0);
        // Without the speckle there is no blood.
        let m = ExaMaterial::default().with_blood(BloodProgram::Cloth, None);
        assert_eq!(m.blood(), None);
        assert_eq!(m.params.flags & flags::DIRT_MAP, 0);
    }

    /// A clean vertex (α = 1) leaves grime's surface as it is whenever the
    /// take reaches 1 (amount ≤ 1): the texel is pulled to mid grey, which
    /// doubles to white and keeps the roughness. A dirty vertex (α = 0)
    /// shows the texel doubled, and its green's distance from ½ smooths.
    #[test]
    fn grime_shows_where_the_vertex_alpha_falls() {
        let dirt = Vec3::new(0.2, 0.3, 0.4);
        let f0 = Vec3::splat(0.04);
        let colour = Vec3::new(0.8, 0.6, 0.4);
        let take = DirtMode::take(0.5);
        assert_eq!(take, 1.5);
        let (r, f, c) = DirtMode::Grime.shade(dirt, 1.0, take, 0.7, f0, colour);
        assert_eq!((r, f, c), (0.7, f0, colour));
        let (r, f, c) = DirtMode::Grime.shade(dirt, 0.0, take, 0.7, f0, colour);
        assert!((c - colour * dirt * 2.0).abs().max_element() < 1e-6);
        assert!((r - (1.0 - 0.3 * (1.0 - 0.4))).abs() < 1e-6);
        assert_eq!(f, f0);
        // A larger amount takes less: at amount 1.8 (take 0.2) a clean
        // vertex still shows 80% of the texel.
        let (_, _, c) = DirtMode::Grime.shade(dirt, 1.0, DirtMode::take(1.8), 0.7, f0, colour);
        let d = dirt.lerp(Vec3::splat(0.5), 0.2);
        assert!((c - colour * d * 2.0).abs().max_element() < 1e-6);
    }

    /// Wear: a clean vertex keeps the colour; as α falls the colour fades
    /// to black through the dimmed dirt texel, and the surface roughens
    /// where the texel's red sits near 0.4.
    #[test]
    fn wear_darkens_where_the_vertex_alpha_falls() {
        let f0 = Vec3::splat(0.5);
        let colour = Vec3::new(0.8, 0.6, 0.4);
        let dirt = Vec3::new(0.9, 0.5, 0.1);
        let (r, f, c) = DirtMode::Worn.shade(dirt, 1.0, 1.0, 0.3, f0, colour);
        // w = clamp(1·3·0.5) = 1: the roughness and F0 stay.
        assert_eq!((r, f), (0.3, f0));
        assert!((c - colour).abs().max_element() < 1e-6);
        let (r, f, c) = DirtMode::Worn.shade(dirt, 0.0, 1.0, 0.3, f0, colour);
        assert_eq!((r, f, c), (1.0, Vec3::splat(0.04), Vec3::ZERO));
        let near = Vec3::new(0.4, 0.5, 0.1);
        let (r, f, _) = DirtMode::Worn.shade(near, 0.5, 1.0, 0.3, f0, colour);
        assert_eq!((r, f), (1.0, Vec3::splat(0.04)));
    }

    /// `DrtUVs`: the diffuse map's aspect, the long side scaled.
    #[test]
    fn wear_scales_by_the_diffuse_aspect() {
        assert_eq!(DirtMode::uv_scale(512, 256), Vec2::new(2.0, 1.0));
        assert_eq!(DirtMode::uv_scale(256, 1024), Vec2::new(1.0, 4.0));
        assert_eq!(DirtMode::uv_scale(64, 64), Vec2::ONE);
    }

    #[test]
    fn dirt_modes_set_their_flags() {
        assert_eq!(DirtMode::of_mode(1), Some(DirtMode::Grime));
        assert_eq!(DirtMode::of_mode(2), Some(DirtMode::Worn));
        assert_eq!(DirtMode::of_mode(3), None);
        assert_eq!(DirtMode::of_mode(4), None);
        let m = ExaMaterial::default().with_dirt(DirtMode::Worn, Handle::default(), true, 0.25);
        assert_eq!(m.dirt(), Some(DirtMode::Worn));
        assert_ne!(m.params.flags & flags::DIRT_MAP, 0);
        assert_ne!(m.params.flags2 & flags2::GREY_DIRT, 0);
        assert_eq!(m.params.dirt.w, 1.75);
        let m = m.with_dirt(DirtMode::Grime, Handle::default(), false, 0.5);
        assert_eq!(m.dirt(), Some(DirtMode::Grime));
        assert_eq!(m.params.flags2 & (flags2::DIRT_WORN | flags2::GREY_DIRT), 0);
        assert_eq!(ExaMaterial::default().dirt(), None);
    }

    /// The record's mode comes from the factory's shader word under the
    /// PBR program only: a hair or legacy record with a dirt layer shows
    /// none, nor does mode 3, nor blood.
    #[test]
    fn dirt_modes_follow_the_record() {
        use exa_formats::material::{Material, TextureLayer, TextureSlot};
        let mode = |extra: u32, kind: u32| {
            let layer = |slot| TextureLayer {
                slot,
                texture: "t".into(),
                parameter: 0,
                amount: 0.0,
            };
            let m = Material {
                kind,
                flags: 3,
                extra,
                opacity: 1.0,
                layers: vec![layer(TextureSlot::Diffuse), layer(TextureSlot::Dirt)],
                ..Material::default()
            };
            DirtMode::of_record(&m, 5)
        };
        assert_eq!(mode(0, 1), Some(DirtMode::Grime));
        assert_eq!(mode(0x1000, 1), Some(DirtMode::Worn));
        assert_eq!(mode(0x2000, 1), None);
        assert_eq!(mode(0x4000, 1), None);
        assert_eq!(mode(0, 3), None);
        assert_eq!(mode(0, 0), None);
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

    #[test]
    fn a_cloth_draw_culls_the_face_it_submits() {
        use bevy::render::render_resource::Face;
        let draws = ExaMaterial::default().cloth_draws(&[Face::Front, Face::Back]);
        assert_eq!(draws.len(), 2);
        assert_eq!(
            ExaMaterialKey::from(&draws[0]).cloth_cull,
            Some(Face::Front)
        );
        assert_eq!(ExaMaterialKey::from(&draws[1]).cloth_cull, Some(Face::Back));
        assert!(
            ExaMaterialKey::from(&ExaMaterial::default())
                .cloth_cull
                .is_none()
        );
    }
}
