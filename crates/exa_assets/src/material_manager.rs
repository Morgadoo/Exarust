//! The original's material bookkeeping outside the model loader: the
//! built-in `_Default` material, the reference counts of the global cache
//! and the per-scene manager (a default material plus a copy of the global
//! plug-in list).
//!
//! The original keeps one material object per record, counts the users of
//! each and frees a record at zero. Exarust shares a Bevy [`Handle`] per
//! record and never frees it while the cache lives; [`MaterialRefs`] keeps
//! the same counts so a caller can follow the original's acquire/release
//! rules (a negative count is an error, and only the `_Default` material
//! never drops to zero).
//!
//! Rules: `docs/RENDERING.md` (Materials).

use std::collections::HashMap;

use bevy::prelude::*;
use exa_formats::material::{
    DEFAULT_MATERIAL_DIFFUSE, DEFAULT_MATERIAL_NAME, DEFAULT_MATERIAL_REFERENCES, Material,
};
use exa_render::material::{ExaMaterial, ExaParams};

/// The built-in `_Default` material object, as the original builds it from
/// its 32-byte payload: black emissive, opaque grey diffuse, no texture and
/// no shading flags.
pub fn default_material() -> ExaMaterial {
    ExaMaterial {
        params: ExaParams {
            diffuse: Vec4::from_array(DEFAULT_MATERIAL_DIFFUSE),
            flags: 0,
            ..default()
        },
        ..default()
    }
}

/// The source record the built-in material stands for, for callers that
/// read the record rather than the shader material.
pub fn default_material_record() -> Material {
    Material {
        diffuse: DEFAULT_MATERIAL_DIFFUSE,
        ..default()
    }
}

/// Why a release failed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RefcountError {
    /// The name was never acquired.
    Unknown,
    /// The count was already zero (the original's "Negative material
    /// reference count.").
    Negative,
}

impl std::fmt::Display for RefcountError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("negative material reference count")
    }
}

impl std::error::Error for RefcountError {}

/// The reference counts of the material cache, by lower-case name. A count
/// of zero means the record is free; the built-in `_Default` starts at the
/// original's permanent count.
#[derive(Clone, Debug)]
pub struct MaterialRefs {
    counts: HashMap<String, i32>,
}

impl Default for MaterialRefs {
    fn default() -> Self {
        Self::new()
    }
}

impl MaterialRefs {
    /// Counts holding only the built-in `_Default`.
    pub fn new() -> Self {
        let mut counts = HashMap::new();
        counts.insert(
            DEFAULT_MATERIAL_NAME.to_ascii_lowercase(),
            DEFAULT_MATERIAL_REFERENCES,
        );
        Self { counts }
    }

    /// The count of `name`, zero when it was never acquired or has been
    /// released to zero.
    pub fn count(&self, name: &str) -> i32 {
        self.counts
            .get(&name.to_ascii_lowercase())
            .copied()
            .unwrap_or(0)
    }

    /// Records a new user of `name` and returns the new count, as a
    /// get-or-load does.
    pub fn acquire(&mut self, name: &str) -> i32 {
        let count = self.counts.entry(name.to_ascii_lowercase()).or_insert(0);
        *count = count.saturating_add(1);
        *count
    }

    /// Drops a user of `name` and returns the new count. Releasing at zero
    /// is the original's negative-count error; the built-in material can
    /// never reach zero.
    pub fn release(&mut self, name: &str) -> Result<i32, RefcountError> {
        let key = name.to_ascii_lowercase();
        if key == DEFAULT_MATERIAL_NAME.to_ascii_lowercase() {
            return Ok(self.count(&key).saturating_sub(1).max(1));
        }
        let Some(count) = self.counts.get_mut(&key) else {
            return Err(RefcountError::Unknown);
        };
        if *count <= 0 {
            return Err(RefcountError::Negative);
        }
        *count -= 1;
        Ok(*count)
    }

    /// Whether the record named `name` may be freed: its count reached
    /// zero.
    pub fn freeable(&self, name: &str) -> bool {
        self.counts
            .get(&name.to_ascii_lowercase())
            .is_some_and(|&c| c == 0)
    }
}

/// A scene's material manager: the scene's default material and its own
/// copy of the global plug-in list (one entry per material type). The
/// original builds it when the scene is created, slots the default material
/// in first and keeps the active and prior name indices at -1.
#[derive(Clone, Debug)]
pub struct SceneMaterials {
    slots: Vec<Handle<ExaMaterial>>,
    /// The global plug-in names, in registration order, copied into the
    /// scene's list after the default material.
    plugins: Vec<String>,
    /// The manager's active and previous material indices (`+0x2C`,
    /// `+0x14`), -1 when none.
    pub active: i32,
    pub previous: i32,
}

impl SceneMaterials {
    /// A scene manager from its default material and the global plug-in
    /// list.
    pub fn new(default: Handle<ExaMaterial>, plugins: impl IntoIterator<Item = String>) -> Self {
        let plugins: Vec<String> = plugins.into_iter().collect();
        let mut slots = Vec::with_capacity(plugins.len() + 1);
        slots.push(default);
        slots.extend(std::iter::repeat_n(Handle::default(), plugins.len()));
        Self {
            slots,
            plugins,
            active: -1,
            previous: -1,
        }
    }

    /// The scene's default material.
    pub fn default_material(&self) -> &Handle<ExaMaterial> {
        &self.slots[0]
    }

    /// The plug-in names in the scene's list.
    pub fn plugins(&self) -> &[String] {
        &self.plugins
    }

    /// The number of slots (the default plus every plug-in), the
    /// constructor's ``global count + 1``.
    pub fn len(&self) -> usize {
        self.slots.len()
    }

    pub fn is_empty(&self) -> bool {
        self.slots.is_empty()
    }

    /// Puts a material in the slot of plug-in `index` (0-based in the
    /// plug-in list).
    pub fn set_plugin_material(&mut self, index: usize, material: Handle<ExaMaterial>) {
        if index < self.plugins.len() {
            self.slots[index + 1] = material;
        }
    }

    /// The material in a plug-in slot.
    pub fn plugin_material(&self, index: usize) -> Option<&Handle<ExaMaterial>> {
        self.slots.get(index + 1)
    }

    /// Clears the plug-in list and its slots, keeping the default material,
    /// as the destructor frees them.
    pub fn clear(&mut self) {
        self.plugins.clear();
        self.slots.truncate(1);
        self.active = -1;
        self.previous = -1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_builtin_default_is_opaque_grey() {
        let m = default_material();
        assert_eq!(m.params.diffuse, Vec4::new(0.5, 0.5, 0.5, 1.0));
        assert_eq!(m.params.emissive, Vec4::new(0.0, 0.0, 0.0, 1.0));
        assert_eq!(m.params.flags, 0);
        assert!(m.diffuse_map.is_none());
        let record = default_material_record();
        assert_eq!(record.diffuse, DEFAULT_MATERIAL_DIFFUSE);
        assert_eq!(record.uv_transform, None);
    }

    #[test]
    fn reference_counts_rise_and_fall_and_the_default_never_frees() {
        let mut refs = MaterialRefs::new();
        assert_eq!(refs.count("_Default"), 0x3FFF_FFFF);
        assert!(!refs.freeable("_Default"));
        assert_eq!(refs.acquire("Cobweb01"), 1);
        assert_eq!(refs.acquire("cobweb01"), 2);
        assert_eq!(refs.count("COBWEB01"), 2);
        assert!(!refs.freeable("cobweb01"));
        assert_eq!(refs.release("Cobweb01"), Ok(1));
        assert_eq!(refs.release("cobweb01"), Ok(0));
        assert!(refs.freeable("cobweb01"));
        assert_eq!(refs.release("cobweb01"), Err(RefcountError::Negative));
        assert_eq!(refs.release("missing"), Err(RefcountError::Unknown));
        // The default's count cannot fall to zero.
        let before = refs.count("_Default");
        assert_eq!(refs.release("_Default"), Ok(before - 1));
        assert!(refs.count("_Default") > 0);
    }

    #[test]
    fn a_scene_copies_the_plugin_list_behind_its_default_material() {
        let default = Handle::<ExaMaterial>::default();
        let global = vec!["surface".to_string(), "flame".to_string()];
        let mut scene = SceneMaterials::new(default.clone(), global.clone());
        assert_eq!(scene.len(), 3);
        assert_eq!(scene.default_material(), &default);
        assert_eq!(scene.plugins(), global.as_slice());
        assert_eq!((scene.active, scene.previous), (-1, -1));
        let plugin = Handle::<ExaMaterial>::default();
        scene.set_plugin_material(1, plugin.clone());
        assert_eq!(scene.plugin_material(1), Some(&plugin));
        assert_eq!(scene.plugin_material(9), None);
        // The copy is the scene's own: clearing it leaves the global list.
        scene.clear();
        assert_eq!(scene.len(), 1);
        assert!(scene.plugins().is_empty());
        assert_eq!(global.len(), 2);
        assert_eq!((scene.active, scene.previous), (-1, -1));
    }
}
