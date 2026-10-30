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
pub const PLUGIN_MASKS: [u32; 5] = crate::material_state::BUILTIN_MASKS;

/// The program class each material type's plug-in starts the shader word
/// with.
pub const PLUGIN_CLASSES: [u32; 5] = crate::material_state::BUILTIN_CLASSES;

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

/// The fragment entry the builder makes for a surface set: `<set>_f`.
///
/// The builder copies the set name into a sixteen-byte buffer and appends
/// `_f` only while its length is below 15; a longer name builds nothing, so
/// this returns `None` for it.
pub fn surface_entry(set: &str) -> Option<String> {
    (set.len() < 15).then(|| format!("{set}_f"))
}

/// The shader word the renderer's factory keeps for a shading level
/// (`0x04D800`): from the record's object word, bit `0x200000` is cleared
/// below level 3 and bit `0x800000` below level 4. When the renderer's
/// simplified-shader flag is set the *original* word masked with `0x600011`
/// is returned instead; when its flat-colour flag is set the level-masked
/// word has bits 1 and 2 cleared.
pub fn shader_word_for_level(level: u32, object: u32, simplified: bool, flat: bool) -> u32 {
    let mut word = object;
    if level < 3 {
        word &= !0x20_0000;
    }
    if level < 4 {
        word &= !0x80_0000;
    }
    if simplified {
        return object & 0x60_0011;
    }
    if flat {
        word &= !0b110;
    }
    word
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

/// The material and shader words after the height map's texture was read.
///
/// The material loader's parallax pass only looks at a record whose material
/// word has the height map (`0x800000`). A one-texel-tall map disables the
/// layer: the material word loses `0x800000`, the shader word its
/// displacement bit `0x100`, and the parallax texel counts become zero. A
/// taller map keeps the words and gets the texel scales
/// [`crate::shader_uniforms::parallax_texels`] computes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HeightMapState {
    pub material: u32,
    pub shader: u32,
    /// The parallax texel counts `(x, y)`; `(0, 0)` when the map is off.
    pub parallax: (u16, u16),
    /// Whether the layer still displaces.
    pub displaces: bool,
}

/// Apply a height layer's texture size to the program words, as the material
/// loader's parallax pass does when it has the height texture.
pub fn height_map(material: u32, shader: u32, height: (u32, u32), amount: f32) -> HeightMapState {
    if material & 0x80_0000 == 0 {
        return HeightMapState {
            material,
            shader,
            parallax: (0, 0),
            displaces: false,
        };
    }
    // The stored height is 16 bits; a one-texel-tall map is the loader's
    // switch-off test.
    if (height.1 & 0xFFFF) == 1 {
        return HeightMapState {
            material: material & !0x80_0000,
            shader: shader & !shader::DISPLACEMENT_SHADOW,
            parallax: (0, 0),
            displaces: false,
        };
    }
    HeightMapState {
        material,
        shader,
        parallax: crate::shader_uniforms::parallax_texels(amount, height.0, height.1),
        displaces: true,
    }
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

/// The super-sampling factor of each setting: the sample count the
/// engine's setting table holds (1, 2, 3, 4, 8, 15, 24 and 66). Other
/// settings count as off.
pub fn supersample_factor(setting: u32) -> u32 {
    crate::subframe::sample_count(setting)
}

/// The sub-frames a super-sampling quarter draws: the sample count capped at
/// four (so 1, 2, 3 or 4; the engine stores the pass count less one). A
/// setting past the table draws one.
pub fn supersample_passes(setting: u32) -> u32 {
    crate::subframe::passes(setting)
}

/// The shadow taps one sub-frame takes, `SdwSmps`.
///
/// The shadow quality gives `2^(quality − 1)` taps over a frame, at most
/// 32; super-sampling (the setting, its level being the setting less one
/// from 2 up) splits them over its passes, never below one per sub-frame.
/// At the shipped settings (shadow quality 4, super-sampling 3) that is 2.
///
/// The exponents are signed 32-bit fields and the shift counts are taken
/// modulo 32, as the x86 `shl` does; the division is a signed 32-bit one.
/// Out-of-range settings therefore follow the same wrap (a setting past 31
/// shifts by its low five bits) rather than saturating.
pub fn shadow_samples(quality: u32, supersample: u32) -> u32 {
    let mut level = supersample as i32;
    if level > 1 {
        level -= 1;
    }
    let mut exponent = (quality as i32).wrapping_sub(1);
    if exponent > 5 {
        exponent = 5;
    }
    if exponent < level {
        exponent = level;
    }
    let scaled = 1i32.wrapping_shl(exponent as u32);
    let divisor = 1i32.wrapping_shl(level as u32);
    scaled.wrapping_div(divisor) as u32
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
    /// The environment's GI mode: specular GI needs mode 3.
    pub gi_mode: u32,
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
            gi_mode: 3,
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
    ///
    /// `VXGISpec` needs the GI on and the environment's GI mode at 3, as the
    /// builder's first test does; the quality gate is the caller's.
    pub fn environment(&self) -> u32 {
        let mut env = self.fog_mode & environment::FOG_MODE_MASK;
        if self.voxel_gi {
            env |= environment::VOXEL_GI;
        }
        if self.voxel_gi && self.voxel_gi_specular && self.gi_mode == 3 {
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

/// The node flags' animated/deformed bit (`0x8000`): such an object uploads
/// a vertex-delta stream, and the engine switches its cull state.
pub const NODE_VERTEX_DELTA: u32 = 0x8000;

impl GeometryOptions {
    /// The streams a draw adds to the object word, from the renderer's
    /// colour-stream switch and the node's flags.
    pub fn of_draw(vertex_colors: bool, node_flags: u32) -> Self {
        Self {
            vertex_colors,
            vertex_delta: node_flags & NODE_VERTEX_DELTA != 0,
        }
    }
}

/// The object word of a draw: the frame part ([`base_object`]) plus the
/// streams the geometry has.
pub fn object_word(samples: u32, supersample: u32, geometry: GeometryOptions) -> u32 {
    let mut word = base_object(samples, supersample);
    if geometry.vertex_colors {
        word |= object::VERTEX_COLOR;
    }
    if geometry.vertex_delta {
        word |= object::VERTEX_DELTA;
    }
    word
}

/// The object word's per-frame half-passes bit (`0x4000_0000`): such an
/// object draws half the scene's super-sampling passes while the sun shadow
/// mode is 3.
pub const HALF_PASSES: u32 = 0x4000_0000;

/// The sub-frames a draw renders: the scene's super-sampling pass count,
/// halved for a frame word with [`HALF_PASSES`] while the sun shadow mode
/// is 3. The engine halves the last pass index (truncating toward zero), so
/// a single pass stays one and an empty pass list (last index −1) becomes
/// one pass.
pub fn draw_passes(passes: u32, frame_word: u32, sun_shadow_mode: u32) -> u32 {
    if frame_word & HALF_PASSES != 0 && sun_shadow_mode == 3 {
        let last = i64::from(passes) - 1;
        (last / 2 + 1) as u32
    } else {
        passes
    }
}

/// Whether a draw gathers lights: an object with a light list whose
/// material word has the diffuse or specular bit. An object without a list,
/// or an unlit/emissive-only material, keeps its slots cleared.
pub fn draw_needs_lights(material_word: u32, has_light_list: bool) -> bool {
    has_light_list && material_word & 6 != 0
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
    let object = object_word(samples, scene.supersample, geometry);
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

/// One step of the engine's warm-up walk.
///
/// Steps 0–28 compile one table row each; step 29 hands control to the
/// completion callback and any later step does nothing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WarmupStep {
    /// Compile the row at this index (`0..29`).
    Row(usize),
    /// The table is done: the completion callback runs.
    Complete,
    /// Past the table: nothing happens.
    OutOfRange,
}

/// The row the warm-up's step `index` compiles, or the completion step.
pub fn warmup_step(index: u32) -> WarmupStep {
    match index as usize {
        i if i < WARMUP.len() => WarmupStep::Row(i),
        i if i == WARMUP.len() => WarmupStep::Complete,
        _ => WarmupStep::OutOfRange,
    }
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
        // The exponents are signed 32-bit values and the x86 shift count is
        // taken modulo 32: a nonsensical setting past 31 wraps the level to
        // its shift's low bits.
        assert_eq!(shadow_samples(0, 0), 1);
        assert_eq!(shadow_samples(0, 3), 1);
        assert_eq!(shadow_samples(4, 32), 1);
        assert_eq!(shadow_samples(4, 33), 1);
    }

    #[test]
    fn the_draws_object_word_takes_the_stream_flags() {
        // Vertex colours come from the renderer's colour switch, vertex
        // deltas from the node's animated bit.
        let geometry = GeometryOptions::of_draw(true, 0x8000);
        assert_eq!(
            object_word(2, 3, geometry),
            (1 << 24) | object::QUAD_RENDER | object::VERTEX_COLOR | object::VERTEX_DELTA
        );
        let plain = GeometryOptions::of_draw(false, 0);
        assert_eq!(object_word(1, 0, plain), 0);
        // Only a list-carrying, lit object gathers lights; an emissive-only
        // record does not.
        assert!(draw_needs_lights(0x7, true));
        assert!(draw_needs_lights(0x2, true));
        assert!(!draw_needs_lights(0x8, true));
        assert!(!draw_needs_lights(0x7, false));
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
    fn the_warm_up_walk_has_a_completion_step() {
        assert_eq!(warmup_step(0), WarmupStep::Row(0));
        assert_eq!(warmup_step(28), WarmupStep::Row(28));
        assert_eq!(warmup_step(29), WarmupStep::Complete);
        assert_eq!(warmup_step(30), WarmupStep::OutOfRange);
        assert_eq!(warmup_step(u32::MAX), WarmupStep::OutOfRange);
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

    #[test]
    fn specular_gi_needs_the_gi_on_and_mode_three() {
        let base = SceneOptions {
            quality: 6,
            voxel_gi_specular: true,
            ..SceneOptions::default()
        };
        assert_ne!(base.environment() & environment::VOXEL_GI_SPECULAR, 0);
        let mode = SceneOptions { gi_mode: 2, ..base };
        assert_eq!(mode.environment() & environment::VOXEL_GI_SPECULAR, 0);
        let off = SceneOptions {
            voxel_gi: false,
            ..base
        };
        assert_eq!(off.environment() & environment::VOXEL_GI_SPECULAR, 0);
        // The light bits only occupy the low byte; the builder keeps them
        // apart from the scene's.
        assert_eq!(
            base.environment() & (environment::FOG_MODE_MASK | environment::VOXEL_GI),
            0x11
        );
    }

    #[test]
    fn a_height_texture_can_switch_the_layer_off() {
        // A one-texel-tall map clears the material's height bit, the
        // shader's displacement bit and both parallax counts.
        let state = height_map(0x7 | 0x80_0000, 0x100, (1024, 1), 0.6);
        assert_eq!(state.material, 0x7);
        assert_eq!(state.shader, 0);
        assert_eq!(state.parallax, (0, 0));
        assert!(!state.displaces);
        // Any other height keeps the words and scales the layer: 0.6 gives
        // 30 texels of depth on a 1024 x 2048 map, so 30 rows and 60 across.
        let state = height_map(0x7 | 0x80_0000, 0x100, (1024, 2048), 0.6);
        assert_eq!(state.material, 0x7 | 0x80_0000);
        assert_eq!(state.shader, 0x100);
        assert_eq!(state.parallax, (60, 30));
        assert!(state.displaces);
        // Without the material word's bit nothing is touched.
        let state = height_map(0x7, 0, (1024, 1), 0.6);
        assert_eq!(state.material, 0x7);
        assert_eq!(state.parallax, (0, 0));
        assert!(!state.displaces);
    }

    #[test]
    fn a_surface_entry_is_only_built_for_a_short_set_name() {
        assert_eq!(surface_entry("shading").as_deref(), Some("shading_f"));
        assert_eq!(surface_entry("shadehar").as_deref(), Some("shadehar_f"));
        // Fourteen characters still fit the builder's sixteen-byte buffer
        // once `_f` is appended; fifteen do not.
        let fourteen = "abcdefghijklmn";
        assert_eq!(surface_entry(fourteen).as_deref(), Some("abcdefghijklmn_f"));
        assert_eq!(surface_entry("abcdefghijklmno"), None);
    }

    #[test]
    fn a_supersampling_setting_picks_up_to_four_subframes() {
        assert_eq!(supersample_passes(0), 1);
        assert_eq!(supersample_passes(1), 2);
        // The 3-sample setting draws three sub-frames, not four.
        assert_eq!(supersample_passes(2), 3);
        assert_eq!(supersample_passes(3), 4);
        assert_eq!(supersample_passes(4), 4);
        assert_eq!(supersample_passes(5), 4);
        assert_eq!(supersample_passes(6), 4);
        assert_eq!(supersample_passes(7), 4);
        assert_eq!(supersample_passes(8), 1);
        // The factor is the setting table's sample count.
        assert_eq!(supersample_factor(0), 1);
        assert_eq!(supersample_factor(1), 2);
        assert_eq!(supersample_factor(2), 3);
        assert_eq!(supersample_factor(3), 4);
        assert_eq!(supersample_factor(4), 8);
        assert_eq!(supersample_factor(5), 15);
        assert_eq!(supersample_factor(6), 24);
        assert_eq!(supersample_factor(7), 66);
        assert_eq!(supersample_factor(8), 1);
    }

    #[test]
    fn a_half_pass_object_draws_half_the_passes_in_sun_mode_three() {
        assert_eq!(draw_passes(4, HALF_PASSES, 3), 2);
        assert_eq!(draw_passes(4, HALF_PASSES, 2), 4);
        assert_eq!(draw_passes(4, 0, 3), 4);
        // The engine halves the last sub-frame's index, so an odd count
        // rounds up: one pass stays one, three become two.
        assert_eq!(draw_passes(1, HALF_PASSES, 3), 1);
        assert_eq!(draw_passes(3, HALF_PASSES, 3), 2);
        assert_eq!(draw_passes(2, HALF_PASSES, 3), 1);
        assert_eq!(draw_passes(0, 0, 0), 0);
        // An empty list's last index −1 halves to 0: one pass.
        assert_eq!(draw_passes(0, HALF_PASSES, 3), 1);
    }
    #[test]
    fn the_factory_masks_the_shader_word_by_the_shading_level() {
        let object = 0x60_0017;
        // Level 3 keeps both bits, level 2 clears 0x200000, level 1 both.
        assert_eq!(shader_word_for_level(4, object, false, false), object);
        assert_eq!(shader_word_for_level(3, object, false, false), object);
        assert_eq!(
            shader_word_for_level(2, object, false, false),
            object & !0x20_0000
        );
        assert_eq!(
            shader_word_for_level(1, object, false, false),
            object & !0xA0_0000
        );
        // The simplified-shader flag returns the original word masked.
        assert_eq!(
            shader_word_for_level(1, object, true, false),
            object & 0x60_0011
        );
        // The flat-colour flag clears bits 1 and 2 of the level-masked word.
        assert_eq!(
            shader_word_for_level(1, object, false, true),
            (object & !0xA0_0000) & !0b110
        );
    }
}
