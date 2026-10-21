//! The renderer's default material factory: the material-type registry and
//! the builder that turns a record into a draw state.
//!
//! Every material record picks a *type* by the low byte of its kind word.
//! A type is a 24-byte plug-in record: an optional per-object upload, an
//! optional material-block upload, the feature mask that filters the
//! record's loaded flags, and the base shader-state bits. Five types are
//! built in; effects register more at run time (fire, electricity,
//! thaumaturgy, teleporters, the effect renderer).
//!
//! The builder asks the shader cache for a program by key
//! ([`crate::shader_program::key_words`]), then fills the draw state: the
//! blend word, the colours with the subsurface and opacity-fall-off
//! variants, the texture slots in fixed order, and the numeric parameters.
//! The texture upload itself belongs to `exa_assets`; this module keeps the
//! decisions, pinned by tests.
//!
//! Rules: `docs/RENDERING.md` (Surfaces) and `docs/formats/FLB.md`.

use bevy::math::Vec4;
use exa_formats::material::{Material, TextureSlot};

use crate::material::{ExaParams, linear_rgb, linear_rgb4};

/// One material type's plug-in record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MaterialType {
    /// The record bits this type's plug-in keeps.
    pub mask: u32,
    /// The base shader-state bits the program starts from.
    pub base: u32,
}

impl MaterialType {
    pub const fn new(mask: u32, base: u32) -> Self {
        Self { mask, base }
    }
}

/// The built-in types, in the order the original's table resets to.
pub const BUILTIN_TYPES: [MaterialType; 5] = [
    MaterialType::new(0x01FE_001F, 0),
    MaterialType::new(0x09FE_031F, 1),
    MaterialType::new(0x0040_0018, 2),
    MaterialType::new(0x09FE_001F, 3),
    MaterialType::new(0x09FE_001F, 6),
];

/// The material-type table. The built-ins are copied in on the first reset
/// (an empty table), and plug-ins append at index 5 and above.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaterialTypes {
    entries: Vec<MaterialType>,
}

impl Default for MaterialTypes {
    fn default() -> Self {
        Self::builtin()
    }
}

impl MaterialTypes {
    /// The table with its five built-in types.
    pub fn builtin() -> Self {
        Self {
            entries: BUILTIN_TYPES.to_vec(),
        }
    }

    /// An empty table; the first [`MaterialTypes::register`] resets it to the
    /// built-ins, as the original's register does.
    pub fn empty() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Registers one more type and returns its index.
    pub fn register(&mut self, ty: MaterialType) -> usize {
        if self.entries.is_empty() {
            self.entries.extend_from_slice(&BUILTIN_TYPES);
        }
        self.entries.push(ty);
        self.entries.len() - 1
    }

    /// The type at an index.
    pub fn get(&self, index: usize) -> Option<MaterialType> {
        self.entries.get(index).copied()
    }

    /// The type a record selects: the low byte of its kind word, or type 0
    /// for an index the table does not hold.
    pub fn record_for(&self, kind: u32) -> MaterialType {
        self.get((kind & 0xFF) as usize).unwrap_or(BUILTIN_TYPES[0])
    }

    /// The number of types.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// A material record's draw state as the factory builds it. Texture
/// handles are not part of it: [`State::texture_slots`] names the slots in
/// the order the builder fills them.
#[derive(Debug, Clone, PartialEq)]
pub struct State {
    /// The blend/render class word after loading ([`crate::shader_program::loaded_class`]).
    pub blend: u32,
    /// The material word (the `Mat.` flags).
    pub material: u32,
    /// The shader word (the `Shd.` flags and program class).
    pub shader: u32,
    /// `MatDif`: linear diffuse, ×1.5 before linearising for type 0,
    /// premultiplied for a `ONE / ONE_MINUS_SRC_ALPHA` blend, with the
    /// subsurface split and the fall-off's floor taken off alpha.
    pub diffuse: [f32; 4],
    /// `MatSpc`: linear specular × the record's level, zero without the
    /// specular bit, ×3 for a program whose class has bit 0 set with a
    /// specular map; alpha is the glossiness (type 0 floored to 0.01).
    pub specular: [f32; 4],
    /// `MatEmi`: linear emissive doubled for a shaded record with an
    /// emissive map; the skin path's scattered diffuse, its alpha the
    /// secondary colour's alpha plus one.
    pub emissive: [f32; 4],
    /// `MatDrt`: mode 4's constant (0.45, 0.03, 0.02, 0.5), else a zero
    /// colour with alpha `2 − the record's dirt amount`.
    pub dirt: [f32; 4],
    /// `MatDsp`: parallax texel counts (xy), displacement step (z) and bias
    /// (w), from the height layer and the record. Texel counts are zero
    /// without a height map; the builder needs the decoded image's size, so
    /// `exa_assets` fills the counts after loading it.
    pub displacement: [f32; 4],
    /// The opacity fall-off: `x` the inverted flag, `y` 1.8·exponent with
    /// the sign, `z` the amount term's magnitude, `w` zero.
    pub opacity_falloff: [f32; 4],
    /// The record's opacity, also carried in `diffuse[3]`.
    pub opacity: f32,
    /// The glossiness, also carried in `specular[3]`.
    pub glossiness: f32,
    /// Two-sided (`Mat` bit `0x10`): both faces draw.
    pub two_sided: bool,
    /// Additive blending (`SRC_ALPHA / ONE`).
    pub additive: bool,
    /// The surface's draws: the texture slots in the order the builder
    /// binds them.
    pub texture_slots: Vec<TextureSlot>,
    /// The alpha-tested caster flag (blend word `0x4000`, or the opacity
    /// flag with an opacity above 0.75).
    pub cut_shadow: bool,
    /// The material casts no shadows (opacity at or below 0.8, or extra
    /// flag `0x4`).
    pub no_shadows: bool,
    /// The type the record picked.
    pub ty: MaterialType,
}

/// The dirt colour the blood mode binds (`MatDrt` for dirt mode 4).
pub const BLOOD_DIRT: [f32; 4] = [0.45, 0.03, 0.02, 0.5];
/// The shading quality the factory is built at (the shipped setting).
pub const SHADING_QUALITY: u32 = 5;
/// Above this opacity a record's opacity flag makes it a cut-shadow caster.
const CUT_SHADOW_OPACITY: f32 = 0.75;
/// Material extra flag (`0x4C`): casts no shadows.
const NO_SHADOWS: u32 = 0x4;
/// Material flag (`0x48`) of the opacity layer.
const OPACITY_FLAG: u32 = 0x40_0000;
/// Blend-word bit of alpha-tested materials.
const ALPHA_TESTED: u32 = 0x4000;
/// The opacity fall-off amounts below which the term is not built.
const FALLOFF_MIN: f32 = 0.01;

/// Builds a record's draw state at `quality`, with the table's type record.
/// `None` when the record draws no program at all (the original logs "No
/// shader generated by factory."): no feature bits and no opacity, or a
/// masked-out feature set whose shader word has no class.
pub fn build(material: &Material, quality: u32, types: &MaterialTypes) -> Option<State> {
    let ty = types.record_for(material.kind);
    let (material_word, shader_word) = crate::shader_program::state_words(material, quality)?;
    let class = crate::shader_program::loaded_class(material);
    let masked = crate::shader_program::loaded_flags(material) & ty.mask;
    let has_emissive_layer = material.texture(TextureSlot::Emissive).is_some();
    let has_specular_layer = material.texture(TextureSlot::Specular).is_some();
    let has_colour_layer = material.texture(TextureSlot::Diffuse).is_some()
        || material.texture(TextureSlot::Opacity).is_some();
    let subsurface = shader_word & 0xF == 5;

    let linear = builder_diffuse(material);
    let mut diffuse = [linear[0], linear[1], linear[2], material.diffuse[3]];
    let mut emissive = material.emissive;
    let mut specular = linear_rgb4(material.specular);
    if subsurface {
        // The skin path: the scattered share moves to MatEmi with alpha
        // sss.w + 1, and the rest stays in MatDif.
        let sss = linear_rgb4(material.subsurface);
        let rest = [1.0 - sss[0], 1.0 - sss[1], 1.0 - sss[2]];
        let scattered = [
            diffuse[0] * sss[0],
            diffuse[1] * sss[1],
            diffuse[2] * sss[2],
        ];
        diffuse = [
            rest[0] * diffuse[0],
            rest[1] * diffuse[1],
            rest[2] * diffuse[2],
            diffuse[3],
        ];
        emissive = [
            scattered[0],
            scattered[1],
            scattered[2],
            material.subsurface[3] + 1.0,
        ];
    } else {
        let mut rgb = linear_rgb([emissive[0], emissive[1], emissive[2]]);
        if (material_word & 0x7 != 0) && has_emissive_layer && masked & 0x8_0000 != 0 {
            rgb *= 2.0;
        }
        let alpha = if material_word & 0x3 != 0 {
            material.emissive[3]
        } else {
            material.opacity.clamp(0.0, 1.0)
        };
        emissive = [rgb.x, rgb.y, rgb.z, alpha];
    }

    if material_word & 0x4_0000 != 0 && has_specular_layer {
        specular = [
            specular[0] * material.specular_level,
            specular[1] * material.specular_level,
            specular[2] * material.specular_level,
            material.glossiness,
        ];
        if shader_word & 1 != 0 {
            specular[0] *= 3.0;
            specular[1] *= 3.0;
            specular[2] *= 3.0;
        }
    } else {
        specular = [0.0, 0.0, 0.0, material.glossiness];
    }

    let stored_opacity = material.opacity;
    let mut opacity = stored_opacity;
    // The fall-off's floor comes off the opacity, after the premultiplied
    // diffuse took the stored value.
    let falloff = crate::material::ExaParams::falloff(
        material.falloff_amount,
        material.falloff_exponent,
        material.extra & 0x100 != 0,
    );
    if material.falloff_amount.abs() > FALLOFF_MIN {
        let f = (-1.8 * material.falloff_exponent).exp2();
        opacity -= (f + 1.0) * f;
    }

    // The blend word's source factor decides whether the diffuse colour is
    // premultiplied by the opacity.
    let one = 1;
    let one_minus_src_alpha = 3;
    let premultiplied = class & ALPHA_TESTED == 0
        && ((class >> 8) & 0xF, class & 0xF) == (one, one_minus_src_alpha);
    if premultiplied {
        diffuse[0] *= stored_opacity;
        diffuse[1] *= stored_opacity;
        diffuse[2] *= stored_opacity;
    }
    diffuse[3] = opacity;
    let additive = class & ALPHA_TESTED == 0 && ((class >> 8) & 0xF, class & 0xF) == (2, one);

    let dirt_mode = (shader_word & crate::shader_program::shader::DIRT_MODE_MASK)
        >> crate::shader_program::shader::DIRT_MODE_SHIFT;
    let dirt = if dirt_mode == 4 {
        BLOOD_DIRT
    } else {
        [0.0, 0.0, 0.0, 2.0 - material.dirt_amount]
    };

    let mut texture_slots = Vec::new();
    if has_colour_layer {
        texture_slots.push(if material.texture(TextureSlot::Diffuse).is_some() {
            TextureSlot::Diffuse
        } else {
            TextureSlot::Opacity
        });
    }
    for (flag, slot) in [
        (0x8_0000, TextureSlot::Emissive),
        (0x4_0000, TextureSlot::Specular),
        (0x20_0000, TextureSlot::Normal),
        (0x100_0000, TextureSlot::Gloss),
        (0x80_0000, TextureSlot::Height),
        (0x800_0000, TextureSlot::Dirt),
    ] {
        if material_word & flag != 0 && material.layer(slot).is_some() {
            texture_slots.push(slot);
        }
    }

    let opacity_flag =
        material.flags & OPACITY_FLAG != 0 || material.texture(TextureSlot::Opacity).is_some();
    Some(State {
        blend: class,
        material: material_word,
        shader: shader_word,
        diffuse,
        specular,
        emissive,
        dirt,
        displacement: [
            0.0,
            0.0,
            material.displacement_step * 0.004,
            material.displacement_bias * 0.05,
        ],
        opacity_falloff: falloff.to_array(),
        opacity,
        glossiness: material.glossiness,
        two_sided: material_word & 0x10 != 0,
        additive,
        texture_slots,
        cut_shadow: class & ALPHA_TESTED != 0
            || opacity_flag && material.opacity > CUT_SHADOW_OPACITY,
        no_shadows: material.opacity <= 0.8 || material.extra & NO_SHADOWS != 0,
        ty,
    })
}

/// The builder's parallax scale: texel counts times 3·10⁻⁵.
pub const PARALLAX_UNIT: f32 = 3e-5;

/// The record's diffuse colour after the builder's scaling order: ×1.5 in
/// the stored sRGB space before the sRGB decode for type 0, then linear.
pub fn builder_diffuse(material: &Material) -> [f32; 3] {
    let scale = if material.kind & 0xFF == 0 { 1.5 } else { 1.0 };
    let c = linear_rgb([
        material.diffuse[0] * scale,
        material.diffuse[1] * scale,
        material.diffuse[2] * scale,
    ]);
    c.to_array()
}

/// The record's glossiness with the type-0 floor.
pub fn builder_gloss(material: &Material) -> f32 {
    if material.kind & 0xFF == 0 && material.glossiness < 0.01 {
        0.01
    } else {
        material.glossiness
    }
}

impl State {
    /// The opacity fall-off as the shader's [`ExaParams`] carries it.
    pub fn falloff_params(&self) -> Vec4 {
        Vec4::from_array(self.opacity_falloff)
    }

    /// The parallax value with the texel counts known (from the height
    /// image's size and the layer's amount), in the builder's order.
    pub fn with_parallax_texels(mut self, x: i16, y: i16) -> Self {
        self.displacement[0] = f32::from(x) * PARALLAX_UNIT;
        self.displacement[1] = f32::from(y) * PARALLAX_UNIT;
        self
    }

    /// Writes the state's numbers into a surface material's parameters,
    /// keeping the handles that `exa_assets` loaded.
    pub fn apply_to(&self, params: &mut ExaParams) {
        params.diffuse = Vec4::from_array(self.diffuse);
        params.specular = Vec4::from_array(self.specular);
        params.emissive = Vec4::from_array(self.emissive);
        params.displacement = Vec4::from_array(self.displacement);
        params.opacity_falloff = self.falloff_params();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use exa_formats::material::TextureLayer;

    fn layer(flag: u32, name: &str) -> TextureLayer {
        TextureLayer {
            slot: TextureSlot::from_flag(flag),
            texture: name.into(),
            parameter: 0,
            amount: 0.0,
        }
    }

    fn record(kind: u32) -> Material {
        let mut m = Material {
            kind,
            opacity: 1.0,
            glossiness: 0.5,
            specular_level: 1.0,
            flags: 0x2_0000,
            diffuse: [0.5, 0.5, 0.5, 1.0],
            ..Default::default()
        };
        m.layers.push(layer(0x2_0000, "d"));
        m
    }

    #[test]
    fn the_five_builtins_match_the_factory_table() {
        let types = MaterialTypes::builtin();
        assert_eq!(types.len(), 5);
        assert_eq!(types.get(0), Some(MaterialType::new(0x01FE_001F, 0)));
        assert_eq!(types.get(1), Some(MaterialType::new(0x09FE_031F, 1)));
        assert_eq!(types.get(2), Some(MaterialType::new(0x0040_0018, 2)));
        assert_eq!(types.get(3), Some(MaterialType::new(0x09FE_001F, 3)));
        assert_eq!(types.get(4), Some(MaterialType::new(0x09FE_001F, 6)));
        assert_eq!(types.record_for(0x1234).mask, 0x01FE_001F);
        assert_eq!(types.record_for(0x103).base, 3);
        assert_eq!(types.record_for(0x104).base, 6);
        // The shader-word tables agree with the registry.
        for (i, ty) in BUILTIN_TYPES.iter().enumerate() {
            assert_eq!(crate::shader_program::PLUGIN_MASKS[i], ty.mask);
            assert_eq!(crate::shader_program::PLUGIN_CLASSES[i], ty.base);
        }
    }

    #[test]
    fn registering_a_plugin_initialises_the_table_and_appends() {
        let mut types = MaterialTypes::empty();
        assert!(types.is_empty());
        assert_eq!(types.register(MaterialType::new(0x1, 9)), 5);
        assert_eq!(types.len(), 6);
        assert_eq!(types.get(0), Some(BUILTIN_TYPES[0]));
        assert_eq!(types.get(5), Some(MaterialType::new(0x1, 9)));
        assert_eq!(types.register(MaterialType::new(0x2, 10)), 6);
    }

    #[test]
    fn a_legacy_record_scales_before_decoding_and_floors_its_gloss() {
        let mut m = record(0);
        m.glossiness = 0.001;
        assert_eq!(builder_diffuse(&m), {
            let c = linear_rgb([0.75, 0.75, 0.75]);
            c.to_array()
        });
        assert_eq!(builder_gloss(&m), 0.01);
        m.kind = 1;
        assert_eq!(builder_gloss(&m), 0.001);
    }

    #[test]
    fn a_type_0_record_with_no_features_but_opacity_draws() {
        let mut m = record(0);
        m.flags = 0;
        m.layers.clear();
        let state = build(&m, SHADING_QUALITY, &MaterialTypes::builtin()).unwrap();
        assert_eq!(state.material & 0xF, 0x8);
        assert_eq!((state.diffuse[0] > 0.0, state.emissive[3]), (true, 1.0));
        m.opacity = 0.0;
        assert!(build(&m, SHADING_QUALITY, &MaterialTypes::builtin()).is_none());
        m.opacity = f32::NAN;
        assert!(build(&m, SHADING_QUALITY, &MaterialTypes::builtin()).is_none());
    }

    #[test]
    fn the_subsurface_path_splits_the_diffuse_and_sets_the_blur() {
        let mut m = record(1);
        m.subsurface = [1.0, 0.5, 0.25, 0.0];
        let state = build(&m, SHADING_QUALITY, &MaterialTypes::builtin()).unwrap();
        assert_eq!(state.shader & 0xF, 5);
        assert_eq!(state.emissive[3], 1.0);
        assert!(state.diffuse[0].abs() < 1e-6);
        assert!(state.emissive[0] > 0.0);
        // At shading quality 2 the secondary colour does not lift the
        // program: the record stays plain.
        let state = build(&m, 2, &MaterialTypes::builtin()).unwrap();
        assert_ne!(state.shader & 0xF, 5);
    }

    #[test]
    fn a_specular_map_triples_the_colour_for_a_pbr_program() {
        let mut m = record(1);
        m.flags = 0x3 | 0x4 | 0x2_0000 | 0x4_0000;
        m.specular = [0.1, 0.1, 0.1, 1.0];
        m.layers.push(layer(0x4_0000, "s"));
        let state = build(&m, SHADING_QUALITY, &MaterialTypes::builtin()).unwrap();
        assert!(state.material & 0x4_0000 != 0);
        let one = linear_rgb([0.1, 0.1, 0.1]).x;
        assert!((state.specular[0] - one * 3.0).abs() < 1e-6);
        // Without the specular bit the colour is zero.
        m.flags = 0x2_0000;
        let state = build(&m, SHADING_QUALITY, &MaterialTypes::builtin()).unwrap();
        assert_eq!(&state.specular[..3], &[0.0, 0.0, 0.0]);
    }

    #[test]
    fn the_blend_word_premultiplies_the_diffuse_and_marks_additive() {
        let mut m = record(1);
        m.opacity = 0.5;
        m.class = 0x103;
        let state = build(&m, SHADING_QUALITY, &MaterialTypes::builtin()).unwrap();
        assert!((state.diffuse[3] - 0.5).abs() < 1e-6);
        let lin = linear_rgb([0.5, 0.5, 0.5]).x;
        assert!((state.diffuse[0] - lin * 0.5).abs() < 1e-6);
        assert!(!state.additive);
        m.class = 0x201;
        let state = build(&m, SHADING_QUALITY, &MaterialTypes::builtin()).unwrap();
        assert!(state.additive);
        assert!((state.diffuse[0] - lin).abs() < 1e-6);
    }

    #[test]
    fn the_falloff_takes_its_floor_off_the_opacity_and_is_packed() {
        let mut m = record(1);
        m.falloff_amount = 1.0;
        m.falloff_exponent = 2.0;
        let state = build(&m, SHADING_QUALITY, &MaterialTypes::builtin()).unwrap();
        let params = ExaParams::falloff(1.0, 2.0, false);
        assert_eq!(state.opacity_falloff, params.to_array());
        let f = (-1.8f32 * 2.0).exp2();
        assert!((state.opacity - (1.0 - (f + 1.0) * f)).abs() < 1e-6);
        assert_eq!(state.material & 0x800, 0x800);
    }

    #[test]
    fn the_blood_dirt_mode_binds_its_constant_colour() {
        let mut m = record(1);
        m.flags |= 0x3;
        m.extra = 0x4000;
        let state = build(&m, SHADING_QUALITY, &MaterialTypes::builtin()).unwrap();
        assert_eq!(state.dirt, BLOOD_DIRT);
        assert_eq!(state.shader & 0x70, 0x40);
        m.extra = 0;
        m.dirt_amount = 1.25;
        let state = build(&m, SHADING_QUALITY, &MaterialTypes::builtin()).unwrap();
        assert_eq!(state.dirt[3], 0.75);
    }

    #[test]
    fn the_texture_slots_come_in_the_builders_order() {
        let mut m = record(1);
        m.flags = 0x3 | 0x2_0000 | 0x8_0000 | 0x8 | 0x4 | 0x4_0000 | 0x20_0000 | 0x100_0000;
        m.layers.push(layer(0x8_0000, "e"));
        m.layers.push(layer(0x4_0000, "s"));
        m.layers.push(layer(0x20_0000, "n"));
        m.layers.push(layer(0x100_0000, "g"));
        let state = build(&m, SHADING_QUALITY, &MaterialTypes::builtin()).unwrap();
        assert_eq!(
            state.texture_slots,
            [
                TextureSlot::Diffuse,
                TextureSlot::Emissive,
                TextureSlot::Specular,
                TextureSlot::Normal,
                TextureSlot::Gloss
            ]
        );
    }

    #[test]
    fn shadows_and_the_cut_flag_follow_the_record() {
        let mut m = record(1);
        m.opacity = 0.8;
        let state = build(&m, SHADING_QUALITY, &MaterialTypes::builtin()).unwrap();
        assert!(state.no_shadows);
        m.opacity = 0.81;
        m.flags |= 0x40_0000;
        let state = build(&m, SHADING_QUALITY, &MaterialTypes::builtin()).unwrap();
        assert!(!state.no_shadows);
        assert!(state.cut_shadow);
        m.opacity = 0.75;
        let state = build(&m, SHADING_QUALITY, &MaterialTypes::builtin()).unwrap();
        assert!(!state.cut_shadow);
        m.extra = 0x4;
        m.opacity = 1.0;
        assert!(
            build(&m, SHADING_QUALITY, &MaterialTypes::builtin())
                .unwrap()
                .no_shadows
        );
    }

    #[test]
    fn parallax_texels_are_scaled_by_the_builders_unit() {
        let state = build(&record(1), SHADING_QUALITY, &MaterialTypes::builtin()).unwrap();
        let state = state.with_parallax_texels(200, 100);
        assert!((state.displacement[0] - 200.0 * 3e-5).abs() < 1e-9);
        assert!((state.displacement[1] - 100.0 * 3e-5).abs() < 1e-9);
    }
}
