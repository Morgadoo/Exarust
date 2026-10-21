//! The original's shader choice: which of the five surface programs a
//! material's shader word selects, and the flags and numbers each program is
//! expanded with.
//!
//! The material factory builds two state words from a record: the
//! *material word* (the `Mat.` flags: shading and texture-slot bits) and the
//! *shader word*, whose low nibble picks the program and whose other bits
//! are the `Shd.` flags (dirt mode, blend mode, alpha test, normal and
//! displacement options, quality):
//!
//! | nibble | program |
//! |---|---|
//! | 0 | `shading_f` (legacy) |
//! | 1 | `shadeopt_f` (PBR) |
//! | 2 | `shading_f` (unlit) |
//! | 3 | `shadehar_f` (hair) |
//! | 5, 6 | `shadesss_f` (skin) |
//! | 7 | `shadetrl_f` (translucent) |
//!
//! Every surface program shares the vertex entry `shading_v` and is expanded
//! by [`crate::shader_template`]. The rules are in `docs/RENDERING.md`
//! (Shader builder) and `docs/formats/FLB.md`.

use exa_formats::material::{Material, TextureSlot};

use crate::shader_template::{
    Flags, LightKind, Numbers, ProgramText, TemplateContext, TemplateError,
};

/// Lights one object can carry (the original's `Light[18]`).
pub const MAX_LIGHTS: usize = 6;

/// The shader-word bits a template can read.
pub mod shader {
    pub const USE_NORMAL: u32 = 0x10000;
    pub const LOCAL_REFLECTION: u32 = 0x400000;
    pub const DIRT_MODE_MASK: u32 = 0x70;
    pub const DIRT_MODE_SHIFT: u32 = 4;
    /// `Shd.BlendMode`: only `0x4000` (mode 4) survives the program key.
    pub const BLEND_MODE_FOUR: u32 = 0x4000;
    pub const ALPHA_TEST: u32 = 0x8000;
    pub const NEGATIVE_NDL: u32 = 0x800;
    pub const TANGENT_NORMAL: u32 = 0x20000;
    pub const DISPLACEMENT_SHADOW: u32 = 0x100;
    pub const FAST_LIGHT: u32 = 0x200;
    /// The shading-quality lift (`ShadeQual`), bits 20–21.
    pub const QUALITY_MASK: u32 = 0x30_0000;
}

/// The object-word bits the builder adds from the geometry and the frame.
pub mod object {
    /// Animated/deformed vertices (`vDlt`).
    pub const VERTEX_DELTA: u32 = 0x100000;
    /// A per-vertex colour stream (`vCol`).
    pub const VERTEX_COLOR: u32 = 0x200000;
    /// Supersampled quadrants: `PassIdx` and the four `SdwSmp` banks.
    pub const QUAD_RENDER: u32 = 0x1000;
    /// The shadow-tap count minus one, from bit 24 up (not masked: 32 taps
    /// reach bit 28).
    pub const SHADOW_SAMPLES_SHIFT: u32 = 24;
}

/// The environment-word bits.
pub mod environment {
    pub const VOXEL_GI: u32 = 0x10;
    pub const FOG_MODE_MASK: u32 = 0x3;
    pub const WET_FLOOR: u32 = 0x20;
    pub const TONE_MAP: u32 = 0x4;
    pub const OUTDOOR: u32 = 0x40;
    pub const VOXEL_GI_SPECULAR: u32 = 0x80;
    pub const SHARPEN: u32 = 0x8;
}

/// The material-state bits the program key keeps (the builder's material
/// word): shading, slot, opacity fall-off, refraction and two-sided bits.
pub const MATERIAL_KEY_MASK: u32 = 0x09FE_0B1F;
/// The shader-state bits the program key keeps: the class nibble and dirt
/// mode, `DispSdw`, `FastLgt`, `NegNdL`, blend mode 4, `AlphaTst`,
/// `UseNrm`, `TngtNrm` and `LocalRef`. The quality lift is added after.
pub const SHADER_KEY_MASK: u32 = 0x0043_CB7F;

/// The plug-in mask of each material type: the record bits the material
/// factory takes.
pub const PLUGIN_MASKS: [u32; 5] = [
    0x01FE_001F,
    0x09FE_031F,
    0x0040_0018,
    0x09FE_001F,
    0x09FE_001F,
];

/// The program class each material type's plug-in starts the shader word
/// with.
pub const PLUGIN_CLASSES: [u32; 5] = [0, 1, 2, 3, 6];

/// The five surface programs of the material shader set.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SurfaceProgram {
    /// `shading_f`: legacy materials and unlit surfaces.
    Shading,
    /// `shadeopt_f`: PBR materials.
    Optimized,
    /// `shadehar_f`: hair.
    Hair,
    /// `shadesss_f`: subsurface (skin).
    Subsurface,
    /// `shadetrl_f`: translucent.
    Translucent,
}

impl SurfaceProgram {
    /// The library entry prefix; the fragment program is `<set>_f`.
    pub fn set(self) -> &'static str {
        match self {
            Self::Shading => "shading",
            Self::Optimized => "shadeopt",
            Self::Hair => "shadehar",
            Self::Subsurface => "shadesss",
            Self::Translucent => "shadetrl",
        }
    }

    /// Expand this program's vertex and fragment entries with `context`.
    pub fn expand(
        self,
        library: &impl crate::shader_template::Includes,
        context: &TemplateContext,
    ) -> Result<ProgramText, TemplateError> {
        crate::shader_template::expand_surface(library, self.set(), context)
    }
}

/// The program a shader word selects, in the original's branch order.
pub fn program_for_shader_word(word: u32) -> SurfaceProgram {
    let class = word & 0xF;
    if class == 7 {
        SurfaceProgram::Translucent
    } else if class & 4 == 0 {
        if class & 7 == 3 {
            SurfaceProgram::Hair
        } else if class & 1 == 0 {
            SurfaceProgram::Shading
        } else {
            SurfaceProgram::Optimized
        }
    } else {
        SurfaceProgram::Subsurface
    }
}

/// The material type of a record: the low byte of its kind word (`0x50`).
pub fn material_type(material: &Material) -> u32 {
    material.kind & 0xFF
}

/// The plug-in index of a record's type. Types past the five built-in
/// plug-ins are taken as type 0 [guess: no shipped record has one].
fn plugin(material: &Material) -> usize {
    let index = material_type(material) as usize;
    if index < PLUGIN_MASKS.len() { index } else { 0 }
}

fn has_layer(material: &Material, slot: TextureSlot) -> bool {
    material.layer(slot).is_some()
}

/// The record's feature bits after loading: its flags (`0x48`), plus the
/// fall-off bit for a positive fall-off, the bits the diffuse, opacity,
/// height, gloss, `0x4000000` and dirt layers add, and the emissive bit an
/// opacity-only unlit record gets.
pub fn loaded_flags(material: &Material) -> u32 {
    let mut flags = material.flags;
    if material.falloff_amount > 0.0 {
        flags |= 0x800;
    }
    for layer in &material.layers {
        flags |= match layer.slot {
            TextureSlot::Diffuse => 0x20000,
            TextureSlot::Opacity => 0x40_0000,
            TextureSlot::Height => 0x80_0000,
            TextureSlot::Gloss => 0x100_0000,
            TextureSlot::Dirt => 0x800_0000,
            TextureSlot::Other(0x400_0000) => 0x400_0000,
            _ => 0,
        };
    }
    if flags & 0x40_000F == 0x40_0000 {
        flags |= 0x8;
    }
    flags
}

/// The record's render class after loading: alpha-tested words lose their
/// blend factors (the low nibble and bits 8–13 become `0x100`).
pub fn loaded_class(material: &Material) -> u32 {
    let mut class = material.class;
    if class & 0x4000 != 0 {
        class = class & 0xFFFF_C0F0 | 0x100;
    }
    class
}

/// The record's feature bits after loading, masked by its type's plug-in:
/// its flags (`0x48`), plus the fall-off bit for a positive fall-off, the
/// bits the diffuse, opacity, height, gloss, `0x4000000` and dirt layers
/// add, and the emissive bit an opacity-only unlit record gets. The factory
/// replaces a word with no low nibble by `MatEmi` (`0x8`) when the record
/// still has an opacity above zero, and returns no state (`0`) when it does
/// not.
pub fn feature_key(material: &Material) -> u32 {
    let mut key = loaded_flags(material) & PLUGIN_MASKS[plugin(material)];
    if key & 0xF == 0 {
        if material.opacity <= 0.0 || material.opacity.is_nan() {
            return 0;
        }
        key = 0x8;
    }
    key
}

/// The factory's test for a normal map: any of the material word's lit,
/// specular, opacity-fall-off or reflection bits (`0x100807`). Only such a
/// surface binds the normal layer and gets `UseNrm`.
pub fn wants_normal_map(material_word: u32) -> bool {
    material_word & 0x10_0807 != 0
}

/// The material and shader state words the original's material factory
/// builds for a record at shading quality `quality`, before the program key
/// masks them. `None` when the record draws nothing (no feature bits and no
/// opacity).
///
/// Every bit needs its feature in the record (masked by the type's plug-in)
/// and, for the texture bits, the layer that fills the slot: lit bits
/// (`MatDif`, `MatAmbDif`), the fall-off above 0.01, the subsurface switch
/// (a secondary colour at quality 3 and above: class 5, material bits
/// `0x300`), the specular bit with `NegNdL` from extra flag `0x400`, the
/// diffuse map (with `AlphaTst` when the opacity bit and the class's
/// `0x4000` are set), the emissive, specular, reflection (`LocalRef` from
/// extra `0x800` above quality 4), normal (`TngtNrm` unless the layer's
/// parameter has `0x10`; `UseNrm` on any shaded surface), gloss and height
/// maps (`DispSdw` from extra `0x100000` above quality 4, which needs the
/// normal map), the dirt modes, and the translucent (class 7 with `NegNdL`)
/// and liquid material modes.
pub fn state_words(material: &Material, quality: u32) -> Option<(u32, u32)> {
    let plugin = plugin(material);
    let key = feature_key(material);
    if key == 0 {
        return None;
    }
    let extra = material.extra;
    let class = loaded_class(material);
    let normal = material.layer(TextureSlot::Normal);
    let mut mat = key & 0x8000_0010;
    let mut shd = PLUGIN_CLASSES[plugin];
    let lit = key & 3 != 0;
    if lit {
        mat |= key & 3;
        if extra & 0x200 != 0 {
            shd |= shader::FAST_LIGHT;
        }
    }
    if class & 0xF == 4 {
        shd |= shader::BLEND_MODE_FOUR;
    }
    if material.falloff_amount.abs() > 0.01 {
        mat |= 0x800;
    }
    if material.has_secondary_colour() && quality >= 3 {
        mat |= 0x300;
        shd = shd & !0xF | 5;
    } else if key & 0x8 != 0 {
        mat |= 0x8;
    }
    if key & 0x4 != 0 {
        if extra & 0x400 != 0 {
            shd |= shader::NEGATIVE_NDL;
        }
        mat |= 0x4;
    }
    let colour_map =
        has_layer(material, TextureSlot::Diffuse) || has_layer(material, TextureSlot::Opacity);
    if key & 0x20000 != 0 && colour_map {
        if lit {
            mat |= 0x20000;
        }
        if key & 0x40_0000 != 0 {
            mat |= 0x40_0000;
            if class & 0x4000 != 0 {
                shd |= shader::ALPHA_TEST;
            }
        }
    } else if key & 0x40_0000 != 0 && colour_map {
        mat |= 0x40_0000;
    }
    if key & 0x8_0000 != 0 && has_layer(material, TextureSlot::Emissive) {
        if key & 0x8 != 0 {
            mat |= 0x8_0000;
        }
        if key & 0x40_0000 != 0 {
            mat |= 0x40_0000;
        }
    }
    if key & 0x4_0000 != 0 && has_layer(material, TextureSlot::Specular) && key & 0x10_0004 != 0 {
        mat |= 0x4_0000;
    }
    if key & 0x10_0000 != 0 && has_layer(material, TextureSlot::Reflection) {
        mat |= 0x10_0000;
        if extra & 0x800 != 0 && quality > 4 {
            shd |= shader::LOCAL_REFLECTION;
        }
    }
    // The normal map and `UseNrm` go with any surface that is shaded: lit,
    // specular, fall-off or reflection bits.
    if wants_normal_map(mat) {
        if key & 0x20_0000 != 0
            && let Some(layer) = normal
        {
            mat |= 0x20_0000;
            if layer.parameter & 0x10 != 0x10 {
                shd |= shader::TANGENT_NORMAL;
            }
        }
        shd |= shader::USE_NORMAL;
    }
    if key & 0x100_0000 != 0 && has_layer(material, TextureSlot::Gloss) {
        mat |= 0x100_0000;
    }
    if key & 0x80_0000 != 0
        && let Some(height) = material.layer(TextureSlot::Height)
        && normal.is_some()
    {
        let shadows = extra & 0x10_0000 != 0 && quality > 4;
        if height.amount > 0.0 {
            mat |= 0x80_0000;
        }
        if shadows {
            shd |= shader::DISPLACEMENT_SHADOW;
        }
    }
    if key & 0x400_0000 != 0 && has_layer(material, TextureSlot::Other(0x400_0000)) {
        mat |= 0x400_0000;
    }
    if key & 0x20000 != 0 {
        let mode = extra & 0x7000;
        if mode == 0x4000 {
            // Blood: the built-in speckle, with or without a dirt layer.
            mat |= 0x800_0000;
            shd |= 4 << shader::DIRT_MODE_SHIFT;
        } else if key & 0x800_0000 != 0 && has_layer(material, TextureSlot::Dirt) {
            mat |= 0x800_0000;
            shd |= match mode {
                0 => 1,
                0x1000 => 2,
                0x2000 => 3,
                _ => 0,
            } << shader::DIRT_MODE_SHIFT;
        }
    }
    match key & 0x300 {
        0x100 => {
            mat |= 0x100;
            shd = shd & !0xF | 0x807;
        }
        0x200 => mat |= 0x200,
        _ => {}
    }
    Some((mat, shd))
}

/// The program class nibble the material factory gives a record.
pub fn program_class(material: &Material, quality: u32) -> u32 {
    state_words(material, quality).map_or(PLUGIN_CLASSES[plugin(material)], |(_, shd)| shd & 0xF)
}

/// The dirt mode the factory selects: 0 none, 1 grime, 2 worn, 3 another
/// worn mode, 4 blood.
pub fn dirt_mode(material: &Material) -> u32 {
    state_words(material, 5).map_or(0, |(_, shd)| {
        (shd & shader::DIRT_MODE_MASK) >> shader::DIRT_MODE_SHIFT
    })
}

/// The shader-word bits the shading quality adds: `ShadeQual` in bits 20–21
/// (`quality − 3` from quality 4, wrapping past 6 as the original's mask
/// does) and `FastLgt` below quality 5.
pub fn quality_shader_bits(quality: u32) -> u32 {
    let quality = quality as i32;
    let mut bits = if quality < 4 {
        0
    } else {
        ((quality - 3) as u32).wrapping_mul(0x10_0000) & shader::QUALITY_MASK
    };
    if quality < 5 {
        bits |= shader::FAST_LIGHT;
    }
    bits
}

/// The `ShadeQual` integer a shader word gives the templates.
pub fn shade_quality(word: u32) -> u32 {
    (word & shader::QUALITY_MASK) >> 20
}

/// The program-key words of a record: the state words masked as the
/// builder keys a program, the quality bits added.
pub fn key_words(material: &Material, quality: u32) -> Option<(u32, u32)> {
    state_words(material, quality).map(|(mat, shd)| {
        (
            mat & MATERIAL_KEY_MASK,
            shd & SHADER_KEY_MASK | quality_shader_bits(quality),
        )
    })
}

/// The super-sampling factor of each setting (the table the setting
/// indexes): off, then 2, 3, 4, 8 and 15. Other values count as off.
pub fn supersample_factor(setting: u32) -> u32 {
    match setting {
        1 => 2,
        2 => 3,
        3 => 4,
        4 => 8,
        5 => 15,
        _ => 1,
    }
}

/// The shadow taps one sub-frame takes, `SdwSmps`.
///
/// The shadow quality gives `2^(quality − 1)` taps over a frame, at most
/// 32; super-sampling (the setting, its level being the setting less one
/// from 2 up) splits them over its passes, never below one per sub-frame.
/// At the shipped settings (shadow quality 4, super-sampling 3) that is 2.
pub fn shadow_samples(quality: u32, supersample: u32) -> u32 {
    let level = if supersample > 1 {
        supersample - 1
    } else {
        supersample
    };
    let exponent = (quality as i32 - 1).min(5).max(level as i32).clamp(0, 31) as u32;
    let level = level.min(31);
    (1u32 << exponent) / (1u32 << level)
}

/// The displacement-shadow taps per sub-frame, `DspSdws`: none without
/// `Shd.DispSdw`, otherwise four over the super-sampling factor, at least
/// one (4, 2, 1).
pub fn displacement_samples(word: u32, supersample: u32) -> u32 {
    if word & shader::DISPLACEMENT_SHADOW == 0 {
        0
    } else {
        warmup_displacement_samples(supersample)
    }
}

/// `DspSdws` as the warm-up sets it, whatever the shader word.
pub fn warmup_displacement_samples(supersample: u32) -> u32 {
    let taps = 4 / supersample_factor(supersample);
    if taps < 2 { 1 } else { taps }
}

/// The object word's frame part: the shadow-tap count less one from bit 24
/// and, with super-sampling on, the quadrant mode.
fn base_object(samples: u32, supersample: u32) -> u32 {
    let mut word = samples
        .wrapping_sub(1)
        .wrapping_mul(1 << object::SHADOW_SAMPLES_SHIFT);
    if supersample > 0 {
        word |= object::QUAD_RENDER;
    }
    word
}

/// The environment word's light part: each slot's word shifted two bits
/// per slot.
pub fn light_bits(lights: &[LightKind]) -> u32 {
    lights
        .iter()
        .enumerate()
        .fold(0, |bits, (i, kind)| bits | kind.slot() << (2 * i as u32))
}

/// A fully expanded program variant: the four flag words plus the integer
/// variables and light kinds.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProgramKey {
    pub flags: Flags,
    pub numbers: Numbers,
    pub lights: Vec<LightKind>,
}

impl ProgramKey {
    /// The template state for this key.
    pub fn context(&self) -> TemplateContext {
        let mut context = TemplateContext::new(self.flags, self.numbers);
        context.lights = self.lights.clone();
        context
    }

    /// Give light slot `index` its kind, and the environment word its bits.
    pub fn set_light(&mut self, index: usize, kind: LightKind) {
        if let Some(slot) = self.lights.get_mut(index) {
            *slot = kind;
            self.flags.environment = self.flags.environment & 0xFF | light_bits(&self.lights);
        }
    }
}

/// The scene options that change every material's program key.
#[derive(Clone, Copy, Debug)]
pub struct SceneOptions {
    /// The `ShadingQuality` setting, 1 through 6.
    pub quality: u32,
    /// The `ShadowQuality` setting, 1 through 6: the shadow taps a frame
    /// takes before super-sampling splits them.
    pub shadow_quality: u32,
    /// The super-sampling setting: 0 off, 1, 3 (the original's 4×), 4, 5.
    pub supersample: u32,
    /// The level's fog mode: 0 off, 1 distance fog, 2 fade to black.
    pub fog_mode: u32,
    /// Ambient bounce light (the game always has it on, GI mode 3).
    pub voxel_gi: bool,
    /// Specular GI from the volume, at shading quality 6; dropped for
    /// programs with `FastLgt`.
    pub voxel_gi_specular: bool,
    /// The sky's environment cube.
    pub outdoor: bool,
    /// The weather's wet floor flag.
    pub wet_floor: bool,
    /// The forward tone curve (off in the shipped game).
    pub tone_map: bool,
    /// The production-mode sharpen in the forward shaders.
    pub sharpen: bool,
}

impl Default for SceneOptions {
    fn default() -> Self {
        Self {
            quality: 5,
            shadow_quality: 4,
            supersample: 3,
            fog_mode: 1,
            voxel_gi: true,
            voxel_gi_specular: false,
            outdoor: false,
            wet_floor: false,
            tone_map: false,
            sharpen: false,
        }
    }
}

impl SceneOptions {
    /// The scene's environment word (before the light slots).
    fn environment(&self) -> u32 {
        let mut env = self.fog_mode & environment::FOG_MODE_MASK;
        if self.voxel_gi {
            env |= environment::VOXEL_GI;
        }
        if self.voxel_gi_specular {
            env |= environment::VOXEL_GI_SPECULAR;
        }
        if self.outdoor {
            env |= environment::OUTDOOR;
        }
        if self.wet_floor {
            env |= environment::WET_FLOOR;
        }
        if self.tone_map {
            env |= environment::TONE_MAP;
        }
        if self.sharpen {
            env |= environment::SHARPEN;
        }
        env
    }
}

/// The geometry options of a draw: which streams the mesh has.
#[derive(Clone, Copy, Debug, Default)]
pub struct GeometryOptions {
    pub vertex_colors: bool,
    pub vertex_delta: bool,
}

/// The program keys a material compiles to in one scene: one per light
/// count, 0 through 6, for lit materials, one for unlit ones (no light is
/// gathered for them). Every light is a point light; [`ProgramKey::set_light`]
/// places a directional one. Empty when the record draws nothing.
pub fn program_keys(
    material: &Material,
    geometry: GeometryOptions,
    scene: &SceneOptions,
) -> Vec<ProgramKey> {
    let Some((mat, shd)) = key_words(material, scene.quality) else {
        return Vec::new();
    };
    let samples = shadow_samples(scene.shadow_quality, scene.supersample);
    let mut object = base_object(samples, scene.supersample);
    if geometry.vertex_colors {
        object |= object::VERTEX_COLOR;
    }
    if geometry.vertex_delta {
        object |= object::VERTEX_DELTA;
    }
    let mut environment = scene.environment();
    if shd & shader::FAST_LIGHT != 0 {
        environment &= !environment::VOXEL_GI_SPECULAR;
    }
    environment &= 0xFF;
    let flags = Flags {
        material: mat,
        shader: shd,
        object,
        environment,
    };
    let numbers = Numbers {
        lights: 0,
        shadow_samples: samples,
        displacement_samples: displacement_samples(shd, scene.supersample),
        supersample: scene.supersample,
    };
    let lit = mat & 6 != 0;
    let counts: &[u32] = if lit { &[0, 1, 2, 3, 4, 5, 6] } else { &[0] };
    counts
        .iter()
        .map(|&lights| {
            let kinds = vec![LightKind::Point; lights as usize];
            ProgramKey {
                flags: Flags {
                    environment: environment | light_bits(&kinds),
                    ..flags
                },
                numbers: Numbers { lights, ..numbers },
                lights: kinds,
            }
        })
        .collect()
}

/// One row of the original's 29-program warm-up table.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WarmupRow {
    /// The material flag word.
    pub material: u32,
    /// The shader word.
    pub shader: u32,
    /// The object word.
    pub object: u32,
    /// The render-state word; bit 0x4 adds vertex colours and bit 0x40 adds
    /// vertex deltas to the draw's object word.
    pub state: u32,
}

/// The common variants the game compiles while its shaders warm up, in the
/// original's order. The first eight are ordinary lit materials; the rest
/// add maps, transparency, dirt and the effect programs' combinations.
pub const WARMUP: [WarmupRow; 29] = [
    WarmupRow {
        material: 0x0122_0007,
        shader: 0x0003_0001,
        object: 0,
        state: 0x1B,
    },
    WarmupRow {
        material: 0x01A2_0007,
        shader: 0x0003_0001,
        object: 0,
        state: 0x1B,
    },
    WarmupRow {
        material: 0x01A2_0007,
        shader: 0x0003_0101,
        object: 0,
        state: 0x1B,
    },
    WarmupRow {
        material: 0x0922_0007,
        shader: 0x0003_0011,
        object: 0,
        state: 0x1F,
    },
    WarmupRow {
        material: 0x09A2_0007,
        shader: 0x0003_0011,
        object: 0,
        state: 0x1F,
    },
    WarmupRow {
        material: 0x09A2_0007,
        shader: 0x0003_0111,
        object: 0,
        state: 0x1F,
    },
    WarmupRow {
        material: 0x0162_0007,
        shader: 0x0003_0001,
        object: 0,
        state: 0x1B,
    },
    WarmupRow {
        material: 0x012A_000F,
        shader: 0x0003_0001,
        object: 0,
        state: 0x1B,
    },
    WarmupRow {
        material: 0x0026_0007,
        shader: 0x0003_0001,
        object: 0,
        state: 0x1B,
    },
    WarmupRow {
        material: 0x0036_0007,
        shader: 0x0003_0001,
        object: 0,
        state: 0x1B,
    },
    WarmupRow {
        material: 0x0836_0007,
        shader: 0x0043_0041,
        object: 0,
        state: 0x1F,
    },
    WarmupRow {
        material: 0x0836_0007,
        shader: 0x0003_0011,
        object: 0,
        state: 0x1F,
    },
    WarmupRow {
        material: 0x0836_0007,
        shader: 0x0003_0021,
        object: 0,
        state: 0x1F,
    },
    WarmupRow {
        material: 0x00B6_0007,
        shader: 0x0003_0001,
        object: 0,
        state: 0x1B,
    },
    WarmupRow {
        material: 0x00B6_0007,
        shader: 0x0003_0101,
        object: 0,
        state: 0x1B,
    },
    WarmupRow {
        material: 0x08A6_0007,
        shader: 0x0003_0011,
        object: 0,
        state: 0x1F,
    },
    WarmupRow {
        material: 0x00A6_0007,
        shader: 0x0003_0101,
        object: 0,
        state: 0x1B,
    },
    WarmupRow {
        material: 0x00A6_0007,
        shader: 0x0003_0001,
        object: 0,
        state: 0x1B,
    },
    WarmupRow {
        material: 0x0026_0007,
        shader: 0x0003_0101,
        object: 0,
        state: 0x1B,
    },
    WarmupRow {
        material: 0x08B6_0007,
        shader: 0x0003_0111,
        object: 0,
        state: 0x1F,
    },
    WarmupRow {
        material: 0x0010_0807,
        shader: 0x0041_0001,
        object: 0,
        state: 0x03,
    },
    WarmupRow {
        material: 0x0000_0003,
        shader: 0x0001_0000,
        object: 0,
        state: 0x03,
    },
    WarmupRow {
        material: 0x0836_0007,
        shader: 0x0043_0041,
        object: 0,
        state: 0x5F,
    },
    WarmupRow {
        material: 0x0836_0307,
        shader: 0x0003_0045,
        object: 0,
        state: 0x5F,
    },
    WarmupRow {
        material: 0x0036_0007,
        shader: 0x0003_0001,
        object: 0,
        state: 0x5B,
    },
    WarmupRow {
        material: 0x0972_0007,
        shader: 0x0003_0843,
        object: 0,
        state: 0x1F,
    },
    WarmupRow {
        material: 0x0972_0007,
        shader: 0x0003_0843,
        object: 0,
        state: 0x5F,
    },
    WarmupRow {
        material: 0x0132_0207,
        shader: 0x0003_0001,
        object: 0,
        state: 0x1B,
    },
    WarmupRow {
        material: 0x0122_0307,
        shader: 0x0003_0005,
        object: 0,
        state: 0x1B,
    },
];

/// Whether the warm-up compiles this row at `quality`: rows with
/// displacement shadows need quality 5, rows with a height map quality 4.
pub fn warmup_applies(row: &WarmupRow, quality: u32) -> bool {
    quality > 4
        || (row.shader & shader::DISPLACEMENT_SHADOW == 0
            && (quality > 3 || row.material & 0x80_0000 == 0))
}

/// A key in the warm-up table: the row, its light count and the resulting
/// program.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WarmupKey {
    pub row: usize,
    pub key: ProgramKey,
}

/// The keys the warm-up compiles for the given settings: the row's words
/// as stored (the quality bits added, `LocalRef` dropped below quality 5),
/// `DspSdws` set whatever the row, and point lights in every slot.
pub fn warmup_keys(
    quality: u32,
    shadow_quality: u32,
    supersample: u32,
    voxel_gi_specular: bool,
) -> Vec<WarmupKey> {
    let samples = shadow_samples(shadow_quality, supersample);
    let quality_bits = quality_shader_bits(quality);
    let frame_object = base_object(samples, supersample);
    let base_environment = environment::VOXEL_GI
        | if voxel_gi_specular {
            environment::VOXEL_GI_SPECULAR
        } else {
            0
        };
    let mut out = Vec::new();
    for (index, row) in WARMUP.iter().enumerate() {
        if !warmup_applies(row, quality) {
            continue;
        }
        let mut object = frame_object;
        if row.state & 0x4 != 0 {
            object |= object::VERTEX_COLOR;
        }
        if row.state & 0x40 != 0 {
            object |= object::VERTEX_DELTA;
        }
        object |= row.object;
        let mut word = quality_bits | row.shader;
        if quality < 5 {
            word &= !shader::LOCAL_REFLECTION;
        }
        let lit = row.material & 3 != 0;
        let counts: &[u32] = if lit { &[0, 1, 2, 3, 4, 5, 6] } else { &[0] };
        for &lights in counts {
            let kinds = vec![LightKind::Point; lights as usize];
            let environment = if lit {
                base_environment | light_bits(&kinds)
            } else {
                0
            };
            out.push(WarmupKey {
                row: index,
                key: ProgramKey {
                    flags: Flags {
                        material: row.material,
                        shader: word,
                        object,
                        environment,
                    },
                    numbers: Numbers {
                        lights,
                        shadow_samples: samples,
                        displacement_samples: warmup_displacement_samples(supersample),
                        supersample,
                    },
                    lights: kinds,
                },
            });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use exa_formats::material::TextureLayer;

    fn material(kind: u32, flags: u32, class: u32, extra: u32) -> Material {
        Material {
            kind,
            flags,
            class,
            extra,
            opacity: 1.0,
            ..Material::default()
        }
    }

    fn layer(slot: TextureSlot) -> TextureLayer {
        TextureLayer {
            slot,
            texture: "t".into(),
            parameter: 0,
            amount: 0.0,
        }
    }

    fn with(mut m: Material, slots: &[TextureSlot]) -> Material {
        m.layers.extend(slots.iter().map(|&s| layer(s)));
        m
    }

    fn surface(class: u32) -> SurfaceProgram {
        program_for_shader_word(class)
    }

    #[test]
    fn program_classes_follow_the_material_type() {
        assert_eq!(program_class(&material(0, 3, 0, 0), 5), 0);
        assert_eq!(program_class(&material(1, 3, 0, 0), 5), 1);
        assert_eq!(program_class(&material(2, 8, 0, 0), 5), 2);
        assert_eq!(program_class(&material(3, 3, 0, 0), 5), 3);
        assert_eq!(program_class(&material(4, 3, 0, 0), 5), 6);
        assert_eq!(surface(0), SurfaceProgram::Shading);
        assert_eq!(surface(1), SurfaceProgram::Optimized);
        assert_eq!(surface(2), SurfaceProgram::Shading);
        assert_eq!(surface(3), SurfaceProgram::Hair);
        assert_eq!(surface(6), SurfaceProgram::Subsurface);
        assert_eq!(surface(5), SurfaceProgram::Subsurface);
        assert_eq!(surface(7), SurfaceProgram::Translucent);
    }

    #[test]
    fn subsurface_and_translucent_modes_set_the_class() {
        let mut m = material(0, 3, 0, 0);
        m.subsurface = [1.0, 0.6, 0.2, 1.0];
        assert_eq!(program_class(&m, 2), 0);
        assert_eq!(program_class(&m, 3), 5);
        let (mat, _) = state_words(&m, 3).unwrap();
        assert_eq!(mat & 0x300, 0x300);
        // The translucent mode is the record's flag bits 0x300 = 0x100, in a
        // type whose plug-in passes them (type 1), and it adds NegNdL.
        let m = material(1, 0x103, 0, 0);
        let (_, shd) = state_words(&m, 5).unwrap();
        assert_eq!(shd & 0xF, 7);
        assert_eq!(shd & shader::NEGATIVE_NDL, shader::NEGATIVE_NDL);
        assert_eq!(program_class(&material(0, 0x103, 0, 0), 5), 0);
        assert_eq!(program_class(&material(1, 0x203, 0, 0), 5), 1);
    }

    #[test]
    fn a_record_without_features_draws_its_opacity_as_emission() {
        let m = material(0, 0, 0, 0);
        assert_eq!(state_words(&m, 5), Some((0x8, 0)));
        let mut clear = m;
        clear.opacity = 0.0;
        assert_eq!(state_words(&clear, 5), None);
        clear.opacity = f32::NAN;
        assert_eq!(state_words(&clear, 5), None);
    }

    #[test]
    fn texture_bits_need_their_layers() {
        // Slot bits without layers stay off; the diffuse layer adds its own
        // bit when loaded.
        let m = material(1, 0x7 | 0x4_0000 | 0x20_0000, 0, 0);
        let (mat, shd) = state_words(&m, 5).unwrap();
        assert_eq!(mat & 0xFFF_0000, 0);
        assert_eq!(shd & shader::USE_NORMAL, shader::USE_NORMAL);
        let m = with(
            m,
            &[
                TextureSlot::Diffuse,
                TextureSlot::Specular,
                TextureSlot::Normal,
            ],
        );
        let (mat, shd) = state_words(&m, 5).unwrap();
        assert_eq!(mat & 0xFFF_0000, 0x26_0000);
        assert_eq!(shd & shader::TANGENT_NORMAL, shader::TANGENT_NORMAL);
        // An object-space normal map (parameter 0x10) takes no tangents.
        let mut object_space = m.clone();
        object_space.layers[2].parameter = 0x10;
        let (_, shd) = state_words(&object_space, 5).unwrap();
        assert_eq!(shd & shader::TANGENT_NORMAL, 0);
    }

    #[test]
    fn alpha_test_needs_the_diffuse_map_and_the_opacity_bit() {
        let m = with(material(1, 0x7, 0x4000, 0), &[TextureSlot::Diffuse]);
        let (_, shd) = state_words(&m, 5).unwrap();
        assert_eq!(shd & shader::ALPHA_TEST, 0);
        let m = with(
            material(1, 0x7 | 0x40_0000, 0x4103, 0),
            &[TextureSlot::Diffuse],
        );
        let (mat, shd) = state_words(&m, 5).unwrap();
        assert_eq!(shd & shader::ALPHA_TEST, shader::ALPHA_TEST);
        assert_eq!(mat & 0x40_0000, 0x40_0000);
        // An opacity layer alone gives TexOpc, never the alpha test.
        let m = with(material(1, 0x7, 0x4000, 0), &[TextureSlot::Opacity]);
        let (mat, shd) = state_words(&m, 5).unwrap();
        assert_eq!(mat & 0x40_0000, 0x40_0000);
        assert_eq!(shd & shader::ALPHA_TEST, 0);
    }

    #[test]
    fn blend_mode_four_is_the_class_nibble() {
        let (_, shd) = state_words(&material(1, 0x7, 0x2304, 0), 5).unwrap();
        assert_eq!(shd & shader::BLEND_MODE_FOUR, shader::BLEND_MODE_FOUR);
        let (_, shd) = state_words(&material(1, 0x7, 0x4103, 0), 5).unwrap();
        assert_eq!(shd & 0x7000, 0);
    }

    #[test]
    fn dirt_modes_need_the_diffuse_bit() {
        let m = material(1, 3, 0, 0x1000);
        assert_eq!(dirt_mode(&m), 0);
        let m = with(m, &[TextureSlot::Diffuse]);
        assert_eq!(dirt_mode(&m), 0);
        let m = with(m, &[TextureSlot::Dirt]);
        assert_eq!(dirt_mode(&m), 2);
        let mut blood = m.clone();
        blood.extra = 0x4000;
        assert_eq!(dirt_mode(&blood), 4);
        // Blood does not need a dirt layer, only the diffuse bit.
        blood.layers.retain(|l| l.slot != TextureSlot::Dirt);
        assert_eq!(dirt_mode(&blood), 4);
        // A mode the factory does not name binds the dirt map but no mode.
        let mut other = m;
        other.extra = 0x3000;
        assert_eq!(dirt_mode(&other), 0);
    }

    #[test]
    fn displacement_shadows_need_a_normal_map_and_quality_five() {
        let mut m = with(
            material(1, 0x7, 0, 0x10_0000),
            &[TextureSlot::Normal, TextureSlot::Height],
        );
        m.layers[1].amount = 0.5;
        let (mat, shd) = state_words(&m, 5).unwrap();
        assert_eq!(mat & 0x80_0000, 0x80_0000);
        assert_eq!(
            shd & shader::DISPLACEMENT_SHADOW,
            shader::DISPLACEMENT_SHADOW
        );
        let (_, shd) = state_words(&m, 4).unwrap();
        assert_eq!(shd & shader::DISPLACEMENT_SHADOW, 0);
        // A flat height layer still casts displacement shadows, but has no
        // parallax.
        m.layers[1].amount = 0.0;
        let (mat, shd) = state_words(&m, 5).unwrap();
        assert_eq!(mat & 0x80_0000, 0);
        assert_eq!(
            shd & shader::DISPLACEMENT_SHADOW,
            shader::DISPLACEMENT_SHADOW
        );
        m.layers.remove(0);
        let (_, shd) = state_words(&m, 5).unwrap();
        assert_eq!(shd & shader::DISPLACEMENT_SHADOW, 0);
    }

    #[test]
    fn extra_flags_reach_the_shader_word() {
        let m = with(
            material(1, 0x7 | 0x10_0000, 0, 0x200 | 0x400 | 0x800),
            &[TextureSlot::Reflection],
        );
        let (mat, shd) = state_words(&m, 5).unwrap();
        assert_eq!(mat & 0x10_0000, 0x10_0000);
        assert_eq!(shd & shader::FAST_LIGHT, shader::FAST_LIGHT);
        assert_eq!(shd & shader::NEGATIVE_NDL, shader::NEGATIVE_NDL);
        assert_eq!(shd & shader::LOCAL_REFLECTION, shader::LOCAL_REFLECTION);
        let (_, shd) = state_words(&m, 4).unwrap();
        assert_eq!(shd & shader::LOCAL_REFLECTION, 0);
    }

    #[test]
    fn key_words_carry_the_quality_options() {
        let m = material(1, 0x7, 0x2304, 0);
        let (_, w) = key_words(&m, 4).unwrap();
        assert_eq!(w & 0xF, 1);
        assert_eq!(w & shader::FAST_LIGHT, shader::FAST_LIGHT);
        assert_eq!(w & 0x30_0000, 0x10_0000);
        assert_eq!(shade_quality(w), 1);
        let (_, w) = key_words(&m, 5).unwrap();
        assert_eq!(w & shader::FAST_LIGHT, 0);
        assert_eq!(shade_quality(w), 2);
        assert_eq!(shade_quality(quality_shader_bits(6)), 3);
        // Past 6 the lift wraps in its two bits.
        assert_eq!(shade_quality(quality_shader_bits(7)), 0);
    }

    #[test]
    fn sample_counts_split_over_the_supersampling_passes() {
        assert_eq!(shadow_samples(4, 0), 8);
        assert_eq!(shadow_samples(4, 3), 2);
        assert_eq!(shadow_samples(4, 1), 4);
        assert_eq!(shadow_samples(1, 3), 1);
        assert_eq!(shadow_samples(6, 0), 32);
        assert_eq!(shadow_samples(2, 0), 2);
        assert_eq!(displacement_samples(0, 0), 0);
        assert_eq!(displacement_samples(shader::DISPLACEMENT_SHADOW, 0), 4);
        assert_eq!(displacement_samples(shader::DISPLACEMENT_SHADOW, 1), 2);
        assert_eq!(displacement_samples(shader::DISPLACEMENT_SHADOW, 3), 1);
        assert_eq!(displacement_samples(shader::DISPLACEMENT_SHADOW, 5), 1);
        assert_eq!(warmup_displacement_samples(0), 4);
        // 32 taps put the count less one past bit 27.
        assert_eq!(base_object(32, 0), 31 << 24);
    }

    #[test]
    fn program_keys_cover_zero_through_six_lights() {
        let m = with(
            material(1, 0x2 | 0x4 | 0x20000, 0x100, 0),
            &[TextureSlot::Diffuse],
        );
        let keys = program_keys(&m, GeometryOptions::default(), &SceneOptions::default());
        assert_eq!(keys.len(), 7);
        assert_eq!(keys[0].numbers.lights, 0);
        assert_eq!(keys[6].numbers.lights, 6);
        assert_eq!(
            keys[3].flags.environment & environment::VOXEL_GI,
            environment::VOXEL_GI
        );
        // Three point lights: 0x100 in each slot, two bits apart.
        assert_eq!(keys[3].flags.environment & !0xFF, 0x1500);
        assert_eq!(keys[0].flags.shader & 0xF, 1);
        assert_eq!(
            keys[0].flags.object & object::QUAD_RENDER,
            object::QUAD_RENDER
        );
        assert_eq!(keys[0].numbers.shadow_samples, 2);
        assert_eq!(shade_quality(keys[0].flags.shader), 2);
        // A directional first light moves its slot bits.
        let mut key = keys[1].clone();
        key.set_light(0, LightKind::Directional);
        assert_eq!(key.flags.environment & !0xFF, 0x200);
        // An unlit material has one key and the legacy shading program.
        let unlit = material(2, 0x8, 0, 0);
        let keys = program_keys(&unlit, GeometryOptions::default(), &SceneOptions::default());
        assert_eq!(keys.len(), 1);
        assert_eq!(
            program_for_shader_word(keys[0].flags.shader),
            SurfaceProgram::Shading
        );
    }

    #[test]
    fn fast_lighting_drops_the_specular_gi() {
        let scene = SceneOptions {
            voxel_gi_specular: true,
            quality: 4,
            ..SceneOptions::default()
        };
        let keys = program_keys(&material(1, 0x7, 0, 0), GeometryOptions::default(), &scene);
        assert_eq!(
            keys[0].flags.environment & environment::VOXEL_GI_SPECULAR,
            0
        );
        let scene = SceneOptions {
            quality: 6,
            ..scene
        };
        let keys = program_keys(&material(1, 0x7, 0, 0), GeometryOptions::default(), &scene);
        assert_eq!(
            keys[0].flags.environment & environment::VOXEL_GI_SPECULAR,
            environment::VOXEL_GI_SPECULAR
        );
    }

    #[test]
    fn warmup_table_has_the_engines_rows() {
        assert_eq!(WARMUP.len(), 29);
        assert_eq!(WARMUP[0].material, 0x0122_0007);
        assert_eq!(WARMUP[0].shader, 0x0003_0001);
        assert_eq!(WARMUP[0].state, 0x1B);
        assert_eq!(WARMUP[28].material, 0x0122_0307);
        assert_eq!(WARMUP[28].shader, 0x0003_0005);
        let at = |q| WARMUP.iter().filter(|r| warmup_applies(r, q)).count();
        assert_eq!(at(3), 18);
        assert_eq!(at(4), 23);
        assert_eq!(at(5), 29);
    }

    #[test]
    fn warmup_keys_include_every_light_count() {
        let keys = warmup_keys(5, 4, 3, false);
        let lit = WARMUP.iter().filter(|r| r.material & 3 != 0).count();
        assert_eq!(keys.len(), WARMUP.len() * 7);
        assert_eq!(lit, 29);
        let first = &keys[0];
        assert_eq!(first.key.numbers.lights, 0);
        assert_eq!(first.key.numbers.shadow_samples, 2);
        assert_eq!(first.key.numbers.displacement_samples, 1);
        let last = &keys[WARMUP.len() * 7 - 1];
        assert_eq!(last.key.numbers.lights, 6);
        assert_eq!(last.key.flags.environment & 0xFF_FF00, 0x5_5500);
        // The table mixes legacy, PBR and effect variants by its shader word.
        assert_eq!(
            program_for_shader_word(WARMUP[20].shader),
            SurfaceProgram::Optimized
        );
        assert_eq!(
            program_for_shader_word(WARMUP[21].shader),
            SurfaceProgram::Shading
        );
    }
}
