//! The original's shader choice: which of the five surface programs a
//! material's shader word selects, and the flags and numbers each program is
//! expanded with.
//!
//! The material factory packs a material's type, block and blend bits, dirt
//! mode and quality options into one *shader word*. Its low nibble picks the
//! program, the rest are the `Shd.` template flags:
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
    pub const QUAD_RENDER: u32 = 0x1000;
    pub const LOCAL_REFLECTION: u32 = 0x400000;
    pub const DIRT_MODE_MASK: u32 = 0x70;
    pub const DIRT_MODE_SHIFT: u32 = 4;
    pub const BLEND_MODE_MASK: u32 = 0x7000;
    pub const BLEND_MODE_SHIFT: u32 = 12;
    pub const ALPHA_TEST: u32 = 0x8000;
    pub const NEGATIVE_NDL: u32 = 0x800;
    pub const TANGENT_NORMAL: u32 = 0x20000;
    pub const DISPLACEMENT_SHADOW: u32 = 0x100;
    pub const FAST_LIGHT: u32 = 0x200;
}

/// The object-word bits the builder adds from the geometry and the frame.
pub mod object {
    /// Animated/deformed vertices (`vDlt`).
    pub const VERTEX_DELTA: u32 = 0x100000;
    /// A per-vertex colour stream (`vCol`).
    pub const VERTEX_COLOR: u32 = 0x200000;
    /// Supersampled quadrants: `PassIdx` and the four `SdwSmp` banks.
    pub const QUAD_RENDER: u32 = 0x1000;
    /// The shadow-tap count minus one lives in bits 24–27.
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
    /// One bit per light, two bits apart, starting at bit 8 (the builder
    /// sets `0x100 << 2i` for a spot light and `0x200 << 2i` for the rest).
    pub const LIGHT_SHIFT: u32 = 8;
}

/// The feature mask the material factory applies before it keys a program:
/// the record's shading, texture-slot, opacity, refraction and two-sided
/// bits.
pub const MATERIAL_KEY_MASK: u32 = 0x09FE_0B1F;
/// The extra-flag part of the same key (dirt modes, displacement,
/// reflection).
pub const EXTRA_KEY_MASK: u32 = 0x0043_CB7F;

/// The plug-in mask of each material type: the bits of the record that
/// reach the shader word.
pub const PLUGIN_MASKS: [u32; 5] = [
    0x01FE_001F,
    0x09FE_031F,
    0x0040_0018,
    0x09FE_001F,
    0x09FE_001F,
];

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

/// The program class nibble the material factory gives a record.
///
/// Types map 0→0, 1→1, 2→2, 3→3, 4→6. A subsurface colour lifts the class
/// to 5 at shading quality 3 and above, and the translucent mode (`0x300`
/// bits of the kind word equal `0x100`) wins with 7.
pub fn program_class(material: &Material, quality: u32) -> u32 {
    let mut class = match material_type(material) {
        0 => 0,
        1 => 1,
        2 => 2,
        3 => 3,
        4 => 6,
        _ => 0,
    };
    if material.has_subsurface() && quality >= 3 {
        class = 5;
    }
    if material.kind & 0x300 == 0x100 {
        class = 7;
    }
    class
}

/// The dirt mode the loader selects: 0 none, 1 grime, 2 worn, 3 another
/// worn mode, 4 blood. The modes need a diffuse map; mode 4 also needs no
/// dirt layer (it uses the built-in speckle).
pub fn dirt_mode(material: &Material) -> u32 {
    if material.layer(TextureSlot::Diffuse).is_none() {
        return 0;
    }
    let mode = material.extra & 0x7000;
    if mode == 0x4000 {
        return 4;
    }
    if material.layer(TextureSlot::Dirt).is_none() {
        return 0;
    }
    match mode {
        0 => 1,
        0x1000 => 2,
        0x2000 => 3,
        _ => 0,
    }
}

/// Whether the material is alpha-tested: the blend word's `0x4000` bit and
/// a diffuse or opacity map to cut with.
pub fn alpha_tested(material: &Material) -> bool {
    material.class & 0x4000 != 0
        && (material.layer(TextureSlot::Diffuse).is_some()
            || material.layer(TextureSlot::Opacity).is_some())
}

/// The shader word for a material at `quality`, with `tangent_normals` when
/// the normal map uses the generated tangent frame (rather than object-space
/// normals; the layer's object-space bit is not parsed yet, so this is the
/// caller's say).
pub fn shader_word(material: &Material, quality: u32, tangent_normals: bool) -> u32 {
    let mut word = program_class(material, quality);
    word |= (dirt_mode(material) & 7) << shader::DIRT_MODE_SHIFT;
    word |= material.class & shader::BLEND_MODE_MASK;
    if alpha_tested(material) {
        word |= shader::ALPHA_TEST;
    }
    // Every lit surface reads `vrNrm`; the normal include needs it on both
    // the tangent and the object-space path.
    word |= shader::USE_NORMAL;
    if tangent_normals && material.layer(TextureSlot::Normal).is_some() {
        word |= shader::TANGENT_NORMAL;
    }
    if quality >= 5 && material.extra & 0x10_0000 != 0 {
        word |= shader::DISPLACEMENT_SHADOW;
    }
    word |= quality_shader_bits(quality);
    word
}

/// The shader-word bits the shading quality adds: `ShadeQual` in bits 20–21
/// (`clamp(quality − 3, 0, 3)`) and `FastLgt` below quality 5.
pub fn quality_shader_bits(quality: u32) -> u32 {
    let lift = if quality < 4 { 0 } else { (quality - 3).min(3) };
    let mut bits = (lift << 20) & 0x30_0000;
    if quality < 5 {
        bits |= shader::FAST_LIGHT;
    }
    bits
}

/// The `ShadeQual` integer the templates read.
pub fn shade_quality(quality: u32) -> u32 {
    if quality < 4 { 0 } else { (quality - 3).min(3) }
}

/// The shadow taps one sub-frame takes, `SdwSmps`.
///
/// The shadow quality gives `2^(quality − 1)` taps over a frame; the
/// supersampling level (0 none, 1, 3 for 4×) splits them over its passes,
/// never below one per sub-frame. At the game's top settings (quality 4,
/// 4× supersampling) that is 2.
pub fn shadow_samples(quality: u32, supersample_level: u32) -> u32 {
    let level = if supersample_level > 1 {
        supersample_level - 1
    } else {
        supersample_level
    };
    let exponent = (quality.saturating_sub(1)).min(5).max(level);
    1u32 << exponent >> level
}

/// The discipline taps the displacement shadow's `DspSdws` integer reports:
/// four when the shader word has `DispSdw`, none otherwise.
pub fn displacement_samples(word: u32) -> u32 {
    if word & shader::DISPLACEMENT_SHADOW != 0 {
        4
    } else {
        0
    }
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
}

/// The scene options that change every material's program key.
#[derive(Clone, Copy, Debug)]
pub struct SceneOptions {
    /// The `ShadingQuality` setting, 1 through 6.
    pub quality: u32,
    /// The `ShadowQuality` setting, 1 through 6: the shadow taps a frame
    /// takes before supersampling splits them.
    pub shadow_quality: u32,
    /// The supersampling level: 0 off, 1, or 3 for the original's 4×.
    pub supersample_level: u32,
    /// The level's fog mode: 0 off, 1 distance fog, 2 fade to black.
    pub fog_mode: u32,
    /// Ambient bounce light (the game always has it on, GI mode 3).
    pub voxel_gi: bool,
    /// Specular GI from the volume, at shading quality 6.
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
            supersample_level: 3,
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
/// count for lit materials (the original warms 0 through 6 every time a
/// material's light list changes), one for unlit ones.
pub fn program_keys(
    material: &Material,
    geometry: GeometryOptions,
    scene: &SceneOptions,
) -> Vec<ProgramKey> {
    let tangent_normals = material.layer(TextureSlot::Normal).is_some();
    let word = shader_word(material, scene.quality, tangent_normals);
    let samples = shadow_samples(scene.shadow_quality, scene.supersample_level);
    let mut object = ((samples.saturating_sub(1)) << object::SHADOW_SAMPLES_SHIFT) & 0x0F00_0000;
    if scene.supersample_level > 0 {
        object |= object::QUAD_RENDER;
    }
    if geometry.vertex_colors {
        object |= object::VERTEX_COLOR;
    }
    if geometry.vertex_delta {
        object |= object::VERTEX_DELTA;
    }
    let flags = Flags {
        material: material.flags & MATERIAL_KEY_MASK,
        shader: word,
        object,
        environment: scene.environment(),
    };
    let numbers = Numbers {
        lights: 0,
        shadow_samples: samples,
        shade_quality: shade_quality(scene.quality),
        displacement_samples: displacement_samples(word),
    };
    let lit = material.flags & 3 != 0;
    let counts: &[u32] = if lit { &[0, 1, 2, 3, 4, 5, 6] } else { &[0] };
    counts
        .iter()
        .map(|&lights| {
            let mut numbers = numbers;
            numbers.lights = lights;
            ProgramKey {
                flags,
                numbers,
                lights: vec![LightKind::Point; lights as usize],
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

/// The keys the warm-up compiles for the given settings.
pub fn warmup_keys(
    quality: u32,
    shadow_quality: u32,
    supersample_level: u32,
    voxel_gi_specular: bool,
) -> Vec<WarmupKey> {
    let samples = shadow_samples(shadow_quality, supersample_level);
    let shade = shade_quality(quality);
    let quality_bits = quality_shader_bits(quality);
    let mut base_object =
        ((samples.saturating_sub(1)) << object::SHADOW_SAMPLES_SHIFT) & 0x0F00_0000;
    if supersample_level > 0 {
        base_object |= object::QUAD_RENDER;
    }
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
        let mut object = base_object | row.object;
        if row.state & 0x4 != 0 {
            object |= object::VERTEX_COLOR;
        }
        if row.state & 0x40 != 0 {
            object |= object::VERTEX_DELTA;
        }
        let mut word = quality_bits | row.shader;
        if quality < 5 {
            word &= !shader::LOCAL_REFLECTION;
        }
        let lit = row.material & 3 != 0;
        let counts: &[u32] = if lit { &[0, 1, 2, 3, 4, 5, 6] } else { &[0] };
        for &lights in counts {
            let mut environment = base_environment;
            for i in 0..lights {
                environment |= 0x100 << (2 * i);
            }
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
                        shade_quality: shade,
                        displacement_samples: displacement_samples(word),
                    },
                    lights: vec![LightKind::Point; lights as usize],
                },
            });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn material(kind: u32, flags: u32, class: u32, extra: u32) -> Material {
        Material {
            kind,
            flags,
            class,
            extra,
            ..Material::default()
        }
    }

    #[test]
    fn program_classes_follow_the_material_type() {
        assert_eq!(program_class(&material(0, 3, 0, 0), 5), 0);
        assert_eq!(program_class(&material(1, 3, 0, 0), 5), 1);
        assert_eq!(program_class(&material(2, 0, 0, 0), 5), 2);
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

    fn surface(class: u32) -> SurfaceProgram {
        program_for_shader_word(class)
    }

    #[test]
    fn subsurface_and_translucent_lift_the_class() {
        let mut m = material(4, 3, 0, 0);
        m.subsurface = [1.0, 0.6, 0.2, 1.0];
        assert_eq!(program_class(&m, 2), 6);
        assert_eq!(program_class(&m, 3), 5);
        assert_eq!(
            program_for_shader_word(program_class(&m, 5)),
            SurfaceProgram::Subsurface
        );
        m.kind = 0x100;
        assert_eq!(program_class(&m, 5), 7);
        assert_eq!(
            program_for_shader_word(program_class(&m, 5)),
            SurfaceProgram::Translucent
        );
    }

    #[test]
    fn dirt_modes_need_a_diffuse_map() {
        let mut m = material(1, 3, 0, 0x1000_0000 | 0x1000);
        assert_eq!(dirt_mode(&m), 0);
        m.layers.push(exa_formats::material::TextureLayer {
            slot: TextureSlot::Diffuse,
            texture: "d".into(),
            amount: 0.0,
        });
        assert_eq!(dirt_mode(&m), 0);
        m.layers.push(exa_formats::material::TextureLayer {
            slot: TextureSlot::Dirt,
            texture: "g".into(),
            amount: 0.0,
        });
        assert_eq!(dirt_mode(&m), 2);
        m.extra = 0x4000;
        assert_eq!(dirt_mode(&m), 4);
        // Blood does not need a dirt layer, only a diffuse map.
        m.layers.retain(|l| l.slot != TextureSlot::Dirt);
        assert_eq!(dirt_mode(&m), 4);
    }

    #[test]
    fn shader_words_carry_the_quality_options() {
        let m = material(1, 0x2 | 0x4, 0x2300, 0x100000);
        let w = shader_word(&m, 4, true);
        assert_eq!(w & 0xF, 1);
        assert_eq!((w & shader::BLEND_MODE_MASK) >> 12, 2);
        assert_eq!(w & shader::FAST_LIGHT, shader::FAST_LIGHT);
        assert_eq!(w & shader::USE_NORMAL, shader::USE_NORMAL);
        assert_eq!(w & 0x30_0000, 0x10_0000);
        assert_eq!(w & shader::DISPLACEMENT_SHADOW, 0);
        let w = shader_word(&m, 5, true);
        assert_eq!(w & shader::FAST_LIGHT, 0);
        assert_eq!(w & shader::DISPLACEMENT_SHADOW, shader::DISPLACEMENT_SHADOW);
        assert_eq!(displacement_samples(w), 4);
    }

    #[test]
    fn shadow_samples_split_over_the_supersampling_passes() {
        assert_eq!(shadow_samples(4, 0), 8);
        assert_eq!(shadow_samples(4, 3), 2);
        assert_eq!(shadow_samples(4, 1), 4);
        assert_eq!(shadow_samples(1, 3), 1);
        assert_eq!(shadow_samples(6, 0), 32);
        assert_eq!(shadow_samples(2, 0), 2);
        assert_eq!(shade_quality(3), 0);
        assert_eq!(shade_quality(4), 1);
        assert_eq!(shade_quality(5), 2);
        assert_eq!(shade_quality(6), 3);
    }

    #[test]
    fn program_keys_cover_zero_through_six_lights() {
        let mut m = material(1, 0x2 | 0x4 | 0x20000, 0x100, 0);
        m.layers.push(exa_formats::material::TextureLayer {
            slot: TextureSlot::Diffuse,
            texture: "d".into(),
            amount: 0.0,
        });
        let keys = program_keys(&m, GeometryOptions::default(), &SceneOptions::default());
        assert_eq!(keys.len(), 7);
        assert_eq!(keys[0].numbers.lights, 0);
        assert_eq!(keys[6].numbers.lights, 6);
        assert_eq!(
            keys[3].flags.environment & environment::VOXEL_GI,
            environment::VOXEL_GI
        );
        assert_eq!(keys[0].flags.shader & 0xF, 1);
        assert_eq!(
            keys[0].flags.object & object::QUAD_RENDER,
            object::QUAD_RENDER
        );
        assert_eq!(keys[0].numbers.shadow_samples, 2);
        assert_eq!(keys[0].numbers.shade_quality, 2);
        // An unlit material has one key and the legacy shading program.
        let unlit = material(2, 0, 0, 0);
        let keys = program_keys(&unlit, GeometryOptions::default(), &SceneOptions::default());
        assert_eq!(keys.len(), 1);
        assert_eq!(
            program_for_shader_word(keys[0].flags.shader),
            SurfaceProgram::Shading
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
