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

use exa_formats::material::Material;

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

    #[test]
    fn the_five_builtins_match_the_factory_table() {
        let types = MaterialTypes::builtin();
        assert_eq!(types.len(), 5);
        assert_eq!(types.get(0), Some(MaterialType::new(0x01FE_001F, 0)));
        assert_eq!(types.get(1), Some(MaterialType::new(0x09FE_031F, 1)));
        assert_eq!(types.get(2), Some(MaterialType::new(0x0040_0018, 2)));
        assert_eq!(types.get(3), Some(MaterialType::new(0x09FE_001F, 3)));
        assert_eq!(types.get(4), Some(MaterialType::new(0x09FE_001F, 6)));
        // The shader-word tables agree with the registry.
        for (i, ty) in BUILTIN_TYPES.iter().enumerate() {
            assert_eq!(crate::shader_program::PLUGIN_MASKS[i], ty.mask);
            assert_eq!(crate::shader_program::PLUGIN_CLASSES[i], ty.base);
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
}
