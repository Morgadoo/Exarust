//! The renderer's default material factory: the material-type table and
//! the factory list.
//!
//! Every material record picks a *type* by the low byte of its kind word.
//! A type is a 24-byte plug-in record: an optional per-object upload, an
//! optional material-block upload, the feature mask that filters the
//! record's loaded flags, and the base shader-state bits. Five types are
//! built in; effects register more at run time (fire, electricity,
//! thaumaturgy, teleporters, the effect renderer). The factory list holds
//! the builders a material's plug-in mask selects; the default builder
//! serves masks 1 and 4.
//!
//! The builder's decisions (the program key words, the uniforms and the
//! texture slots) are made where Exarust builds a surface:
//! [`crate::shader_program::state_words`] and `exa_assets`'s material
//! builder. This module keeps the tables and the scale order and gloss
//! floor the record loader applies first.
//!
//! Rules: `docs/RENDERING.md` (Surfaces) and `docs/formats/FLB.md`.

use exa_formats::material::{Material, TextureSlot};

use crate::material::linear_rgb;

/// One material type's plug-in record (its two upload pointers are not
/// kept: Exarust's surfaces upload through their own materials).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MaterialType {
    /// The record bits this type's plug-in keeps.
    pub mask: u32,
    /// The base shader-state bits the program starts from.
    pub base: u32,
}

/// The material type the smoke and mist draw takes over (the low byte of a
/// record's kind word).
pub const SMOKE_TYPE: u32 = 2;

/// The mask the stored table gives type 2 before the smoke unit's start-up
/// rewrites the slot (the two-sided bit `0x10` is the difference).
pub const STORED_SMOKE_SLOT_MASK: u32 = 0x0040_0018;

/// Whether a record draws with the smoke and mist program: every record of
/// type 2 ([`SMOKE_TYPE`]). In 0.9.5 only the wraith's skin and robe
/// (`wraithskin` in `wraithhabase.rfc`) are of that type.
pub fn draws_smoke(material: &Material) -> bool {
    material.kind & 0xFF == SMOKE_TYPE
}

impl MaterialType {
    pub const fn new(mask: u32, base: u32) -> Self {
        Self { mask, base }
    }
}

/// The built-in types, in the order the original's table resets to.
///
/// Type 2 is the smoke and mist type ([`SMOKE_TYPE`]): the table is stored
/// with mask `0x400018` there ([`STORED_SMOKE_SLOT_MASK`]), but the smoke
/// unit's start-up hook, which runs with the program's unit
/// initialisation before anything resets the table, rewrites that slot in
/// place with the smoke draw ([`crate::smoke`]) and mask `0x400008`, so
/// every reset copies the rewritten slot.
pub const BUILTIN_TYPES: [MaterialType; 5] = [
    MaterialType::new(0x01FE_001F, 0),
    MaterialType::new(0x09FE_031F, 1),
    MaterialType::new(0x0040_0008, 2),
    MaterialType::new(0x09FE_001F, 3),
    MaterialType::new(0x09FE_001F, 6),
];

/// The material-type table. A reset copies the five built-ins in (and
/// drops every plug-in); a registration on an empty table resets it
/// first, then appends.
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

    /// An empty table, as before the factory's start-up.
    pub fn empty() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Resets the table to the five built-ins (the factory's start-up hook
    /// does this every time it runs).
    pub fn reset(&mut self) {
        self.entries = BUILTIN_TYPES.to_vec();
    }

    /// Registers one more type and returns its index (5 for the first
    /// plug-in).
    pub fn register(&mut self, ty: MaterialType) -> usize {
        if self.entries.is_empty() {
            self.reset();
        }
        self.entries.push(ty);
        self.entries.len() - 1
    }

    /// The type at an index.
    pub fn get(&self, index: usize) -> Option<MaterialType> {
        self.entries.get(index).copied()
    }

    /// The number of types.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// One factory record: the plug-in mask bits it serves and its flags word,
/// whose low nibble is its priority.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FactoryRecord {
    pub class_mask: u32,
    pub flags: u32,
}

impl FactoryRecord {
    /// The record's priority: its flags' low nibble.
    pub fn priority(&self) -> u32 {
        self.flags & 0xF
    }
}

/// The default builder's record: plug-in mask bits 1 and 4, priority 0.
pub const DEFAULT_FACTORY: FactoryRecord = FactoryRecord {
    class_mask: 5,
    flags: 0,
};

/// The factory list, by descending priority. A registration goes after
/// every record of the same or a higher priority. A material calls every
/// record whose mask meets its plug-in mask, in list order.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FactoryList {
    records: Vec<FactoryRecord>,
}

impl FactoryList {
    /// Registers a record.
    pub fn register(&mut self, record: FactoryRecord) {
        let at = self
            .records
            .iter()
            .position(|r| r.priority() < record.priority())
            .unwrap_or(self.records.len());
        self.records.insert(at, record);
    }

    /// The records a material with plug-in mask `mask` calls, in order.
    pub fn for_mask(&self, mask: u32) -> impl Iterator<Item = &FactoryRecord> + '_ {
        self.records
            .iter()
            .filter(move |r| r.class_mask & mask != 0)
    }

    pub fn records(&self) -> &[FactoryRecord] {
        &self.records
    }

    pub fn len(&self) -> usize {
        self.records.len()
    }

    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }
}

/// The factory's normal-map predicate on a material state word: any of
/// `0x100807` (lit `0x3`, specular `0x4`, opacity fall-off `0x800`, the
/// reflection layer `0x100000`) asks for the normal map.
pub fn wants_normal_map(material_word: u32) -> bool {
    material_word & 0x10_0807 != 0
}

/// The record's diffuse colour as the record loader leaves it: type 0
/// scaled ×1.5 in the stored sRGB space, then linearised.
pub fn builder_diffuse(material: &Material) -> [f32; 3] {
    let scale = if material.kind & 0xFF == 0 { 1.5 } else { 1.0 };
    let c = linear_rgb([
        material.diffuse[0] * scale,
        material.diffuse[1] * scale,
        material.diffuse[2] * scale,
    ]);
    c.to_array()
}

/// The record's glossiness as the record loader leaves it: type 0 below
/// 0.01 is raised to 0.01.
pub fn builder_gloss(material: &Material) -> f32 {
    if material.kind & 0xFF == 0 && material.glossiness < 0.01 {
        0.01
    } else {
        material.glossiness
    }
}

/// The material's shared colour slot: one texture holds both the diffuse
/// and the opacity layer's image. The record loader loads each of the two
/// layers into the same slot as it meets them, so the later layer in the
/// record wins (and a failed load leaves the slot empty).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ColourSlot<'a> {
    /// The texture the slot keeps.
    pub texture: &'a str,
    /// The layer it came from ([`TextureSlot::Diffuse`] or
    /// [`TextureSlot::Opacity`]).
    pub from: TextureSlot,
    /// The record has both layers, so the earlier one's image is dropped.
    pub both: bool,
}

impl ColourSlot<'_> {
    /// The slot holds the opacity layer's image, which the loader loads in
    /// plain colour mode (a diffuse layer may skip the LOD bias).
    pub fn from_opacity(&self) -> bool {
        self.from == TextureSlot::Opacity
    }
}

/// The record's shared colour slot ([`ColourSlot`]), or `None` when it has
/// neither a diffuse nor an opacity layer.
pub fn colour_slot(material: &Material) -> Option<ColourSlot<'_>> {
    let mut slot: Option<ColourSlot<'_>> = None;
    let (mut diffuse, mut opacity) = (false, false);
    for layer in &material.layers {
        match layer.slot {
            TextureSlot::Diffuse => diffuse = true,
            TextureSlot::Opacity => opacity = true,
            _ => continue,
        }
        slot = Some(ColourSlot {
            texture: &layer.texture,
            from: layer.slot,
            both: false,
        });
    }
    slot.map(|s| ColourSlot {
        both: diffuse && opacity,
        ..s
    })
}

/// What the default builder binds from the shared colour slot for a record
/// with feature word `key` (see [`crate::shader_program::feature_key`]).
/// With the diffuse bit (`0x20000`) a lit record (`key & 3`) binds the slot
/// as its colour, and the opacity bit (`0x400000`) then reads the same
/// image; an unlit record binds nothing there, even with the opacity bit.
/// Without the diffuse bit, the opacity bit binds the slot as the opacity
/// map alone.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ColourBinding {
    /// The slot is the surface's diffuse map.
    pub colour: bool,
    /// The slot is the surface's opacity map.
    pub opacity: bool,
}

/// The builder's binding of the shared colour slot ([`ColourBinding`]);
/// `has_slot` is whether the slot holds an image.
pub fn colour_binding(key: u32, has_slot: bool) -> ColourBinding {
    if !has_slot {
        return ColourBinding::default();
    }
    if key & 0x2_0000 != 0 {
        let colour = key & 3 != 0;
        ColourBinding {
            colour,
            opacity: colour && key & 0x40_0000 != 0,
        }
    } else {
        ColourBinding {
            colour: false,
            opacity: key & 0x40_0000 != 0,
        }
    }
}

/// Whether the record's plug-in mask (`0x94`) calls the default builder
/// ([`DEFAULT_FACTORY`], the only factory record the shipped game
/// registers): material activation runs every factory record whose mask
/// meets the record's, so a record whose mask misses bits 1 and 4 gets no
/// render state and draws nothing. Every shipped record has mask 1.
pub fn builds_default(material: &Material) -> bool {
    STARTUP
        .factories
        .for_mask(material.plugin_mask)
        .any(|r| *r == DEFAULT_FACTORY)
}

/// The type a record selects (the low byte of its kind word) in the table
/// the start-up hook leaves. A type past the table is taken as type 0 [guess:
/// the original reads past its table; no shipped record has one].
pub fn record_type(material: &Material) -> MaterialType {
    STARTUP
        .types
        .get((material.kind & 0xFF) as usize)
        .unwrap_or(BUILTIN_TYPES[0])
}

/// The factory's two tables as its start-up hook leaves them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaterialFactory {
    /// The type table, reset to the five built-ins.
    pub types: MaterialTypes,
    /// The factory list, with the default builder's record.
    pub factories: FactoryList,
}

impl MaterialFactory {
    /// The tables after the factory's start-up hook: it resets the type
    /// table and registers the default builder's record. The effect units
    /// that register more types (electricity, fire, the thaumaturgy forces
    /// and dispersals, the selection marker) build their records in code
    /// and draw them with their own programs in Exarust, so their types are
    /// not registered here.
    pub fn startup() -> Self {
        let mut types = MaterialTypes::empty();
        types.reset();
        let mut factories = FactoryList::default();
        factories.register(DEFAULT_FACTORY);
        Self { types, factories }
    }
}

/// The tables every record build reads ([`MaterialFactory::startup`]).
static STARTUP: std::sync::LazyLock<MaterialFactory> =
    std::sync::LazyLock::new(MaterialFactory::startup);

/// The record's class after the loader's late opaque rewrite: a loaded
/// class whose destination nibble is 3 (the transparent class) becomes
/// source ONE / destination ZERO (`class & 0xFFFFC0F0 | 0x100`) in
/// refraction mode (loaded flags `0x300` = `0x200`), or when its opacity is
/// above 0.99 and its fall-off amount below 0.01 (both widened to double)
/// and its loaded flags lack the opacity layer (`0x400000`). It starts from
/// the alpha-test rewrite ([`crate::shader_program::loaded_class`]); NaN
/// fails both comparisons. The shipped `0.99` is above the bound and
/// `0.01` below it once widened.
pub fn late_class(material: &Material) -> u32 {
    let mut class = crate::shader_program::loaded_class(material);
    let flags = crate::shader_program::loaded_flags(material);
    if class & 0xF == 3
        && (flags & 0x300 == 0x200
            || ((material.opacity as f64) > 0.99
                && (material.falloff_amount as f64) < 0.01
                && flags & 0x40_0000 == 0))
    {
        class = class & 0xFFFF_C0F0 | 0x100;
    }
    class
}

/// The render-state word the record loader derives (the material's
/// stream needs, which a geometry part's mode word copies): the record's
/// word (`0x90`) keeps only `0x20` and gains `1`; `2` when the record is lit
/// or has a positive fall-off; `0x100` for extra flags `0x4002`; `2` and
/// `0x100` for a type-2 (smoke) record; `4` with a normal layer; `0x100`
/// with a dirt layer; and `0x10` when the loaded flags have any of
/// `0xFEF0000` (the texture-layer bits but the reflection one).
pub fn loaded_render_state(material: &Material) -> u32 {
    let early = material.flags
        | if material.falloff_amount > 0.0 {
            0x800
        } else {
            0
        };
    let mut word = material.render_state & 0x20 | 1;
    if early & 3 != 0 || early & 0x800 != 0 {
        word |= 2;
    }
    if material.extra & 0x4002 != 0 {
        word |= 0x100;
    }
    if material.kind & 0xFF == SMOKE_TYPE {
        word |= 2 | 0x100;
    }
    for layer in &material.layers {
        match layer.slot {
            TextureSlot::Normal => word |= 4,
            TextureSlot::Dirt => word |= 0x100,
            _ => {}
        }
    }
    if crate::shader_program::loaded_flags(material) & 0x0FEF_0000 != 0 {
        word |= 0x10;
    }
    word
}

/// The blend class each built-in type starts from, derived from
/// [`BUILTIN_TYPES`] so the table and the program classes cannot drift.
pub const BUILTIN_MASKS: [u32; 5] = [
    BUILTIN_TYPES[0].mask,
    BUILTIN_TYPES[1].mask,
    BUILTIN_TYPES[2].mask,
    BUILTIN_TYPES[3].mask,
    BUILTIN_TYPES[4].mask,
];
pub const BUILTIN_CLASSES: [u32; 5] = [
    BUILTIN_TYPES[0].base,
    BUILTIN_TYPES[1].base,
    BUILTIN_TYPES[2].base,
    BUILTIN_TYPES[3].base,
    BUILTIN_TYPES[4].base,
];

/// The loader's transparency flag: a record in refraction mode
/// (loaded flags `0x300` = `0x200`) is translucent; otherwise one whose
/// opacity is above 0.8 in double precision and whose loaded class is not
/// the opaque nibble 1 is. The record's no-shadow flag (extra bit 2) clears
/// it. A NaN opacity takes the unordered branch, which clears the flag.
/// The flag is computed before the late opaque rewrite, so it uses the
/// class the alpha-test rewrite left ([`crate::shader_program::loaded_class`]).
pub fn draws_translucent(material: &Material) -> bool {
    if material.extra & 0x4 != 0 {
        return false;
    }
    let flags = crate::shader_program::loaded_flags(material);
    let class = crate::shader_program::loaded_class(material);
    flags & 0x300 == 0x200 || ((material.opacity as f64) > 0.8 && class & 0xF != 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(kind: u32) -> Material {
        Material {
            kind,
            glossiness: 0.5,
            diffuse: [0.5, 0.5, 0.5, 1.0],
            ..Default::default()
        }
    }

    fn layered(kind: u32, flags: u32, slots: &[(TextureSlot, &str)]) -> Material {
        Material {
            kind,
            flags,
            opacity: 1.0,
            layers: slots
                .iter()
                .map(|&(slot, texture)| exa_formats::material::TextureLayer {
                    slot,
                    texture: texture.into(),
                    parameter: 0,
                    amount: 1.0,
                })
                .collect(),
            ..Default::default()
        }
    }

    #[test]
    fn the_colour_slot_keeps_the_later_of_diffuse_and_opacity() {
        use TextureSlot::*;
        assert_eq!(colour_slot(&layered(1, 3, &[(Normal, "n")])), None);
        let d = layered(1, 3, &[(Diffuse, "d"), (Normal, "n")]);
        let slot = colour_slot(&d).unwrap();
        assert_eq!((slot.texture, slot.from, slot.both), ("d", Diffuse, false));
        assert!(!slot.from_opacity());
        // The shipped order: the opacity layer after the diffuse one (an
        // arenao4 awning names two different images; the opacity one wins).
        let both = layered(1, 3, &[(Diffuse, "d"), (Normal, "n"), (Opacity, "o")]);
        let slot = colour_slot(&both).unwrap();
        assert_eq!((slot.texture, slot.from, slot.both), ("o", Opacity, true));
        assert!(slot.from_opacity());
        let back = layered(1, 3, &[(Opacity, "o"), (Diffuse, "d")]);
        assert_eq!(colour_slot(&back).unwrap().texture, "d");
        assert!(colour_slot(&back).unwrap().both);
        let twice = layered(1, 3, &[(Opacity, "a"), (Opacity, "b")]);
        let slot = colour_slot(&twice).unwrap();
        assert_eq!((slot.texture, slot.both), ("b", false));
    }

    #[test]
    fn the_builder_binds_the_colour_slot_by_the_diffuse_and_opacity_bits() {
        let bind = |key| colour_binding(key, true);
        let none = ColourBinding::default();
        let colour = ColourBinding {
            colour: true,
            opacity: false,
        };
        let both = ColourBinding {
            colour: true,
            opacity: true,
        };
        let opacity = ColourBinding {
            colour: false,
            opacity: true,
        };
        assert_eq!(bind(0x2_0003), colour);
        assert_eq!(bind(0x62_0001), both);
        // Unlit with the diffuse bit: nothing, even with the opacity bit.
        assert_eq!(bind(0x2_0008), none);
        assert_eq!(bind(0x62_0008), none);
        // No diffuse bit: the opacity bit binds it alone, lit or not.
        assert_eq!(bind(0x40_0008), opacity);
        assert_eq!(bind(0x40_0003), opacity);
        assert_eq!(bind(0x3), none);
        assert_eq!(colour_binding(0x62_0003, false), none);
    }

    #[test]
    fn the_start_up_tables_hold_the_builtins_and_the_default_builder() {
        let f = MaterialFactory::startup();
        assert_eq!(f.types, MaterialTypes::builtin());
        assert_eq!(f.factories.records(), [DEFAULT_FACTORY]);
        let mut m = record(0x102);
        assert_eq!(record_type(&m), BUILTIN_TYPES[2]);
        m.kind = 9;
        assert_eq!(record_type(&m), BUILTIN_TYPES[0]);
        // Every shipped mask is 1; bit 4 is the builder's too; a mask
        // without either builds nothing, so the record has no feature word.
        for (mask, builds) in [(1, true), (4, true), (5, true), (0x20, false), (0, false)] {
            m.plugin_mask = mask;
            assert_eq!(builds_default(&m), builds, "{mask:#x}");
        }
        m.kind = 1;
        m.flags = 3;
        m.plugin_mask = 0x20;
        assert_eq!(crate::shader_program::feature_key(&m), 0);
        assert_eq!(crate::shader_program::state_words(&m, 5), None);
        m.plugin_mask = 1;
        assert_eq!(crate::shader_program::feature_key(&m), 3);
    }

    #[test]
    fn the_loader_derives_the_render_state_word() {
        use TextureSlot::*;
        let mut m = layered(1, 0, &[]);
        m.render_state = 0x7F;
        assert_eq!(loaded_render_state(&m), 0x21);
        m.flags = 0x1;
        assert_eq!(loaded_render_state(&m), 0x23);
        m.flags = 0;
        m.falloff_amount = 0.5;
        assert_eq!(loaded_render_state(&m), 0x23);
        m.falloff_amount = -0.5;
        m.extra = 0x4000;
        assert_eq!(loaded_render_state(&m), 0x121);
        // A smoke record and its layers.
        let smoke = layered(2, 0, &[(Opacity, "o")]);
        assert_eq!(loaded_render_state(&smoke), 0x113);
        let lit = layered(1, 3, &[(Normal, "n"), (Specular, "s")]);
        assert_eq!(loaded_render_state(&lit), 0x7);
        // The specular layer adds no loaded flag; the record's own bit does.
        let spec = layered(1, 0x4_0003, &[]);
        assert_eq!(loaded_render_state(&spec), 0x13);
        let dirt = layered(1, 3, &[(Dirt, "g")]);
        assert_eq!(loaded_render_state(&dirt), 0x113);
        // The reflection bit alone is not a texture-layer bit here.
        let reflect = layered(1, 0x10_0003, &[(Reflection, "r")]);
        assert_eq!(loaded_render_state(&reflect), 0x3);
    }

    #[test]
    fn the_late_rewrite_makes_a_full_transparent_record_opaque() {
        let mut m = record(1);
        m.class = 0x0203;
        m.opacity = 1.0;
        assert_eq!(late_class(&m), 0x0100);
        m.opacity = 0.99;
        // The stored single widens above 0.99.
        assert_eq!(late_class(&m), 0x0100);
        m.opacity = 0.98;
        assert_eq!(late_class(&m), 0x0203);
        m.flags = 0x200;
        assert_eq!(late_class(&m), 0x0100);
        m.flags = 0x40_0000;
        m.opacity = 1.0;
        assert_eq!(late_class(&m), 0x0203);
        m.flags = 0;
        m.falloff_amount = f32::NAN;
        assert_eq!(late_class(&m), 0x0203);
    }

    #[test]
    fn the_five_builtins_match_the_factory_table() {
        let types = MaterialTypes::builtin();
        assert_eq!(types.len(), 5);
        assert_eq!(types.get(0), Some(MaterialType::new(0x01FE_001F, 0)));
        assert_eq!(types.get(1), Some(MaterialType::new(0x09FE_031F, 1)));
        // Slot 2 as the smoke unit's start-up hook rewrites it.
        assert_eq!(types.get(2), Some(MaterialType::new(0x0040_0008, 2)));
        assert_eq!(STORED_SMOKE_SLOT_MASK & !types.get(2).unwrap().mask, 0x10);
        assert_eq!(types.get(3), Some(MaterialType::new(0x09FE_001F, 3)));
        assert_eq!(types.get(4), Some(MaterialType::new(0x09FE_001F, 6)));
        // The shader-word tables agree with the registry.
        for (i, ty) in BUILTIN_TYPES.iter().enumerate() {
            assert_eq!(crate::shader_program::PLUGIN_MASKS[i], ty.mask);
            assert_eq!(crate::shader_program::PLUGIN_CLASSES[i], ty.base);
        }
    }

    #[test]
    fn only_type_two_draws_smoke() {
        assert!(draws_smoke(&record(2)));
        assert!(draws_smoke(&record(0x8000_0102)));
        for kind in [0, 1, 3, 4, 0x200] {
            assert!(!draws_smoke(&record(kind)), "kind {kind:#x}");
        }
    }

    #[test]
    fn registering_a_plugin_initialises_the_table_and_a_reset_drops_it() {
        let mut types = MaterialTypes::empty();
        assert!(types.is_empty());
        assert_eq!(types.register(MaterialType::new(0x1, 9)), 5);
        assert_eq!(types.len(), 6);
        assert_eq!(types.get(0), Some(BUILTIN_TYPES[0]));
        assert_eq!(types.get(5), Some(MaterialType::new(0x1, 9)));
        assert_eq!(types.register(MaterialType::new(0x2, 10)), 6);
        types.reset();
        assert_eq!(types, MaterialTypes::builtin());
    }

    #[test]
    fn the_factory_list_inserts_after_equal_priorities() {
        let mut list = FactoryList::default();
        list.register(DEFAULT_FACTORY);
        let a = FactoryRecord {
            class_mask: 0x8,
            flags: 0x53,
        };
        let b = FactoryRecord {
            class_mask: 0x10,
            flags: 0x1,
        };
        let c = FactoryRecord {
            class_mask: 0x8,
            flags: 0x3,
        };
        list.register(a);
        list.register(b);
        list.register(c);
        assert_eq!(list.records(), [a, c, b, DEFAULT_FACTORY]);
        // Every record whose mask meets the material's is called.
        assert_eq!(list.for_mask(0x8).count(), 2);
        assert_eq!(
            list.for_mask(0x5).copied().collect::<Vec<_>>(),
            [DEFAULT_FACTORY]
        );
        assert_eq!(list.for_mask(0x2).count(), 0);
        // A second start-up appends another default record.
        list.register(DEFAULT_FACTORY);
        assert_eq!(list.for_mask(1).count(), 2);
    }

    #[test]
    fn the_normal_map_predicate_tests_the_four_bits() {
        assert!(!wants_normal_map(0));
        for bit in [0x1, 0x2, 0x4, 0x800, 0x10_0000] {
            assert!(wants_normal_map(bit), "{bit:#x}");
        }
        assert!(!wants_normal_map(0x8 | 0x10 | 0x2_0000));
    }

    #[test]
    fn a_legacy_record_scales_before_decoding_and_floors_its_gloss() {
        let mut m = record(0);
        m.glossiness = 0.001;
        assert_eq!(
            builder_diffuse(&m),
            linear_rgb([0.75, 0.75, 0.75]).to_array()
        );
        assert_eq!(builder_gloss(&m), 0.01);
        m.kind = 1;
        assert_eq!(builder_gloss(&m), 0.001);
        assert_eq!(builder_diffuse(&m), linear_rgb([0.5, 0.5, 0.5]).to_array());
    }

    #[test]
    fn the_loader_flags_a_record_translucent_by_its_class_and_opacity() {
        let mut m = record(1);
        m.class = 0x203;
        m.opacity = 0.9;
        assert!(draws_translucent(&m));
        // Below 0.8 is opaque; the compare widens the stored single to
        // double, so the single 0.8 (0.80000001) is already translucent.
        m.opacity = 0.79;
        assert!(!draws_translucent(&m));
        m.opacity = 0.8;
        assert!(draws_translucent(&m));
        // The opaque class nibble 1 is opaque above the bound. (never reached)
        m.opacity = 0.9;
        m.class = 0x201;
        assert!(!draws_translucent(&m));
        // Refraction mode is always translucent.
        m.class = 0x201;
        m.flags = 0x200;
        assert!(draws_translucent(&m));
        // The no-shadow extra bit clears it last.
        m.extra = 0x4;
        assert!(!draws_translucent(&m));
        m.extra = 0;
        // A NaN opacity takes the unordered branch, which is opaque, as
        // the original's JP goes to the clear.
        m.flags = 0;
        m.class = 0x203;
        m.opacity = f32::NAN;
        assert!(!draws_translucent(&m));
    }
}
