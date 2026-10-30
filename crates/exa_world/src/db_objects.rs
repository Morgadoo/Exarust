//! The object manager's scene decisions, as its create, clone, release and
//! flag routines take them: which database an id names, what a record's
//! scene tree is, the node flags and kind word the tree gets, how apparel
//! materials are drawn, which trees are kept as masters, and the release
//! check.
//!
//! `Create` resolves the record in its tier ([`objdb::Tier`]). A record without a model name
//! becomes its invisible container volume (the volume section's three
//! extents, its radius their length) or, with an operative section, a
//! 50 cm placeholder; anything else has no tree. A model name starting
//! with `@` is a weapon the factory assembles. Any other model is found
//! by name in the model table: a model already loaded is cloned from its
//! master, otherwise it is loaded from `Objects.rpk`, its materials ruled
//! by the record (a weapon record hides `*` materials; an item-body
//! record hides `*` and paints `&` ones), and the tree is kept as the
//! model's master unless it was painted. Every tree the manager makes
//! carries node flag `0x4000`, and each of its children `0x5000` plus the
//! parent's hidden bit; a create asked to hide marks the whole tree
//! hidden. The collision body a first load uses is the record's mechanism
//! ([`collision_body`]: a lever's, a push surface's or a door's class, in
//! the original's section order).
//!
//! Evidence: the object manager's routines (RF_DBObjects) in the assembly.

use std::collections::HashMap;
use std::sync::Arc;

use bevy::prelude::Component;
use exa_formats::containers;
use exa_formats::objdb::{self, Item, operative};

/// The node flag every tree the manager makes carries (the model's root,
/// a factory weapon's, the container volume and the placeholder).
pub const MANAGED_NODE: u32 = 0x4000;
/// The node flags each child of a managed tree takes with its own.
pub const MANAGED_CHILD: u32 = 0x5000;
/// The hidden node flag, which children take from their parent.
pub const HIDDEN: u32 = 0x8_0000;
/// The create flag that hides the whole tree.
pub const CREATE_HIDDEN: u32 = 0x10;
/// The kind word of a door's tree.
pub const KIND_DOOR: u32 = 0x4_0000;
/// The kind word of an operative's tree and of the placeholder.
pub const KIND_OPERATIVE: u32 = 0x1_0000;
/// The kind word of a container volume ([`objdb::VOLUME`], its section).
pub const KIND_VOLUME: u32 = objdb::VOLUME;
/// The side of the placeholder cube, cm.
pub const PLACEHOLDER_SIDE: f32 = 50.0;
/// The placeholder's bounding radius as the original stores it, the
/// length of its extents rounded to single precision.
pub const PLACEHOLDER_RADIUS: f32 = 86.602_54;
/// The side of the texture an apparel paint makes, at the higher detail
/// setting and otherwise.
pub const PAINT_SIDE: (u32, u32) = (1024, 512);
/// The mode flag under which a base-tier factory weapon is first copied
/// to the local database.
pub const MODE_LOCAL_COPIES: u32 = 0x40;

/// An invisible container volume node: its extents and radius, cm.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct ContainerVolume {
    pub extents: [f32; 3],
    pub radius: f32,
}

/// A usable object's 50 cm placeholder node.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct PlaceholderNode {
    pub extents: [f32; 3],
    pub radius: f32,
}

impl Default for PlaceholderNode {
    fn default() -> Self {
        Self {
            extents: [PLACEHOLDER_SIDE; 3],
            radius: PLACEHOLDER_RADIUS,
        }
    }
}

/// What a record's scene tree is.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SceneTree<'a> {
    /// A model found by name in the model table.
    Model(&'a str),
    /// A weapon the factory assembles (the name after `@`).
    FactoryWeapon(&'a str),
    /// An invisible container volume.
    ContainerVolume(ContainerVolume),
    /// The 50 cm placeholder of an operative object.
    Placeholder,
    /// No tree.
    None,
}

/// The tree `item` gets, `payload` being its section chain: with a model
/// name, the factory weapon or the model; without one (or an empty one),
/// the container volume when the volume section is there, else the
/// placeholder for an operative record, else none.
pub fn scene_tree<'a>(item: &'a Item, payload: &[u8]) -> SceneTree<'a> {
    if !item.model.is_empty() {
        return match item.model.strip_prefix('@') {
            Some(factory) => SceneTree::FactoryWeapon(factory),
            None => SceneTree::Model(&item.model),
        };
    }
    if let Some(body) = containers::section(payload, objdb::VOLUME) {
        let extent = |at: usize| {
            body.get(at..at + 2)
                .map_or(0.0, |b| f32::from(u16::from_le_bytes([b[0], b[1]])))
        };
        return SceneTree::ContainerVolume(container_volume([extent(0), extent(2), extent(4)]));
    }
    if item.operative.is_some() {
        return SceneTree::Placeholder;
    }
    SceneTree::None
}

/// A container volume of `extents`: its radius is their length, the
/// squares summed x and z, then y.
pub fn container_volume(extents: [f32; 3]) -> ContainerVolume {
    let [x, y, z] = extents;
    ContainerVolume {
        extents,
        radius: (x * x + z * z + y * y).sqrt(),
    }
}

/// One paint seed of the oldest item-body layout, as the object database's
/// conversion draws it: two words from the run-time library's Mersenne
/// twister, the second's top bit cleared and placed in the high half (the
/// first word low), which the conversion then reduces modulo `0xFFFFFFFF`.
/// The original seeds that twister from the clock at start-up, so only the
/// draws' order and number are reproduced; the caller supplies the
/// generator.
pub fn paint_seed(mt: &mut crate::particles::Mt19937) -> u64 {
    let low = u64::from(mt.next_u32());
    let high = u64::from(mt.next_u32() & 0x7FFF_FFFF);
    (high << 32) | low
}

/// The node flags a child of a managed tree takes: its parent's hidden
/// bit, its own bits and [`MANAGED_CHILD`].
pub fn child_flags(parent: u32, child: u32) -> u32 {
    (parent & HIDDEN) | child | MANAGED_CHILD
}

/// The kind word a first load gives the tree: a door's or an operative's,
/// for a record that is neither a weapon nor an item body; else none.
pub fn tree_kind(item: &Item) -> u32 {
    if item.weapon.is_some() || item.burden.is_some() {
        0
    } else if item.door.is_some() {
        KIND_DOOR
    } else if item.operative.is_some() {
        KIND_OPERATIVE
    } else {
        0
    }
}

/// The kind word the database builder leaves on the node it builds for
/// `item`, which the level builder dispatches on: a tree with a model takes
/// [`tree_kind`] (a door before an operative, nothing for a weapon or an
/// item body); the container volume [`KIND_VOLUME`]; the placeholder
/// [`KIND_OPERATIVE`]; no tree, 0. Follows [`scene_tree`]'s order. A volume
/// section shorter than its 20-byte body is not parsed into the item, so it
/// reads as no volume here (guess: no shipped record has one).
pub fn node_kind(item: &Item) -> u32 {
    if !item.model.is_empty() {
        tree_kind(item)
    } else if item.volume.is_some() {
        KIND_VOLUME
    } else if item.operative.is_some() {
        KIND_OPERATIVE
    } else {
        0
    }
}

/// How a first load draws a material of the record's model.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MaterialRule {
    /// As loaded.
    Draw,
    /// Hidden: its alpha zeroed and its hidden flag set.
    Hide,
    /// Painted by the apparel painter with the record's appearance.
    Paint,
}

/// The rule for material `name` of `item`'s model: a weapon record hides
/// `*` materials; an item-body record (checked after) also paints `&`
/// ones; other records draw everything.
pub fn material_rule(item: &Item, name: &str) -> MaterialRule {
    let weapon = item.weapon.is_some();
    if !weapon && item.burden.is_none() {
        return MaterialRule::Draw;
    }
    if name.starts_with('*') {
        MaterialRule::Hide
    } else if !weapon && name.starts_with('&') {
        MaterialRule::Paint
    } else {
        MaterialRule::Draw
    }
}

/// The impact sound class a painted item body takes from its material
/// byte: a class only up to 6 (an index into the original's sound table).
pub fn impact_sound_class(byte: u8) -> Option<u8> {
    (byte <= 6).then_some(byte)
}

/// The collision body the manager builds a model's tree with. On a first
/// load the original picks one of three rigid-body classes from the
/// record's sections, before the model's own bodies are read: an
/// operative section takes the class its operative class names (a lever's
/// or a push surface's), a door's section the door's, and any other record
/// none. Because the operative test comes first, an operative record that
/// is not a lever or a push surface takes none even when it also has a
/// door section.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CollisionBody {
    /// The record's model needs no mechanism body.
    None,
    /// A hinged or sliding door's.
    Door,
    /// A lever's.
    Lever,
    /// A pressure plate's or button's push surface.
    PushSurface,
}

/// The collision body `item` is built with, from its sections' record
/// flags in the original's order (see [`CollisionBody`]).
pub fn collision_body(item: &Item) -> CollisionBody {
    if let Some(operative) = item.operative {
        match operative.class {
            operative::LEVER => CollisionBody::Lever,
            operative::PUSH_SURFACE => CollisionBody::PushSurface,
            _ => CollisionBody::None,
        }
    } else if item.door.is_some() {
        CollisionBody::Door
    } else {
        CollisionBody::None
    }
}

/// Whether a factory weapon of `id` is first copied to the local database:
/// only under [`MODE_LOCAL_COPIES`] and for a base-tier id.
pub fn factory_copies_locally(mode_flags: u32, id: u32) -> bool {
    mode_flags & MODE_LOCAL_COPIES != 0 && id & objdb::TAG_MASK == 0
}

/// The masters: one loaded tree per model name, cloned by every later
/// create of that model. A painted tree is never kept, so every painted
/// object is loaded on its own. The clone is an [`Arc`] here: instances
/// share the meshes and materials.
#[derive(Debug)]
pub struct Masters<T> {
    by_model: HashMap<String, Arc<T>>,
}

impl<T> Default for Masters<T> {
    fn default() -> Self {
        Self {
            by_model: HashMap::new(),
        }
    }
}

impl<T> Masters<T> {
    /// An empty table.
    pub fn new() -> Self {
        Self::default()
    }

    /// The tree for `model`: the master's clone when there is one, else
    /// `load`'s tree, which becomes the master unless `painted`.
    pub fn create(&mut self, model: &str, painted: bool, load: impl FnOnce() -> T) -> Arc<T> {
        if let Some(master) = self.by_model.get(model) {
            return master.clone();
        }
        let tree = Arc::new(load());
        if !painted {
            self.by_model.insert(model.to_string(), tree.clone());
        }
        tree
    }

    /// The master of `model`, if one is kept.
    pub fn get(&self, model: &str) -> Option<&Arc<T>> {
        self.by_model.get(model)
    }

    /// How many masters are kept.
    pub fn len(&self) -> usize {
        self.by_model.len()
    }

    /// Whether no master is kept.
    pub fn is_empty(&self) -> bool {
        self.by_model.is_empty()
    }
}

/// A release found another master in the instance's slot; the original
/// logs "Mismatched database object instance reference" and goes on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MismatchedInstance {
    /// The instance released.
    pub instance: usize,
    /// The master the slot held.
    pub found: usize,
}

/// The release of `instance`, whose model's 1-based slot is `slots[at]`
/// and whose own master is `own_master`: nothing without a slot; else a
/// slot holding another tree is reported, and in every case the slot is
/// cleared when the instance is its own master, or handed to its master
/// otherwise. Returns whether a slot was there and the mismatch, if any.
pub fn release_instance(
    masters: &mut [usize],
    slots: &mut [usize],
    at: usize,
    instance: usize,
    own_master: usize,
) -> (bool, Option<MismatchedInstance>) {
    let Some(&slot) = slots.get(at) else {
        return (false, None);
    };
    let Some(index) = slot.checked_sub(1) else {
        return (false, None);
    };
    let Some(&held) = masters.get(index) else {
        return (false, None);
    };
    let mismatch = (held != instance).then_some(MismatchedInstance {
        instance,
        found: held,
    });
    if own_master == instance {
        masters[index] = 0;
        slots[at] = 0;
    } else {
        masters[index] = own_master;
    }
    (true, mismatch)
}

#[cfg(test)]
mod tests {
    use super::*;
    use exa_formats::objdb::{DESTINATIONS, Door, Operative, operative};

    fn item(model: &str) -> Item {
        let mut item = Item::parse(1, &[]).unwrap();
        item.model = model.into();
        item
    }

    fn section_bytes(tag: u32, body: &[u8]) -> Vec<u8> {
        let mut out = tag.to_le_bytes().to_vec();
        out.extend_from_slice(&(body.len() as u32 + 8).to_le_bytes());
        out.extend_from_slice(body);
        out
    }

    fn operative_item(model: &str) -> Item {
        let mut it = item(model);
        it.operative = Some(Operative {
            class: operative::KEY,
            kind: 0,
            flags: 0,
            value: 0,
            time: 0,
            target: 0,
            second: 0,
            atlas: [0; 16],
            destinations: [0; DESTINATIONS],
        });
        it
    }

    #[test]
    fn a_tree_is_a_model_a_factory_weapon_a_volume_or_a_placeholder() {
        assert_eq!(scene_tree(&item("chair"), &[]), SceneTree::Model("chair"));
        assert_eq!(
            scene_tree(&item("@ub05ssworda"), &[]),
            SceneTree::FactoryWeapon("ub05ssworda")
        );
        assert_eq!(scene_tree(&item(""), &[]), SceneTree::None);
        assert_eq!(scene_tree(&operative_item(""), &[]), SceneTree::Placeholder);
        // A named model wins over the operative section.
        assert_eq!(
            scene_tree(&operative_item("lever"), &[]),
            SceneTree::Model("lever")
        );
        // The volume section wins over the operative section.
        let volume = section_bytes(objdb::VOLUME, &[30, 0, 40, 0, 120, 0]);
        let SceneTree::ContainerVolume(v) = scene_tree(&operative_item(""), &volume) else {
            panic!("a volume");
        };
        assert_eq!(v.extents, [30.0, 40.0, 120.0]);
        assert_eq!(v.radius, 130.0);
        let p = PlaceholderNode::default();
        assert_eq!(p.extents, [50.0; 3]);
        assert_eq!(p.radius, container_volume([50.0; 3]).radius);
    }

    #[test]
    fn a_paint_seed_is_two_twister_words_with_the_top_bit_cleared() {
        // The standard seed 5489 gives the twister's reference outputs
        // 0xD091BB5C then 0x22AE9EF6, so the seed is
        // 0x22AE9EF6_D091BB5C and reduces to 0xF3405A52.
        let mut mt = crate::particles::Mt19937::new(5489);
        let seed = paint_seed(&mut mt);
        assert_eq!(seed, 0x22AE_9EF6_D091_BB5C);
        assert_eq!(seed % 0xFFFF_FFFF, 0xF340_5A52);
        // The database's oldest body layout takes one per body through the
        // store's seed hook.
        let mut file = exa_formats::objdb::MAGIC.to_le_bytes().to_vec();
        file.extend(16u32.to_le_bytes());
        let mut seeds = 0;
        let store = exa_formats::objdb::Store::parse_with_seeds(&file, &mut || {
            seeds += 1;
            paint_seed(&mut crate::particles::Mt19937::new(7))
        });
        assert!(store.is_ok());
        // An empty database has no bodies, so the hook is never drawn from.
        assert_eq!(seeds, 0);
    }

    #[test]
    fn child_flags_take_the_hidden_bit() {
        assert_eq!(child_flags(MANAGED_NODE, 0x2), 0x5002);
        assert_eq!(
            child_flags(MANAGED_NODE | HIDDEN, KIND_DOOR),
            HIDDEN | KIND_DOOR | MANAGED_CHILD
        );
    }

    #[test]
    fn materials_follow_the_record_kind() {
        let plain = item("chair");
        assert_eq!(material_rule(&plain, "*glass"), MaterialRule::Draw);
        assert_eq!(material_rule(&plain, "&cloth"), MaterialRule::Draw);
        // An item-body section (0x1000) gives the record its burden.
        let body = Item::parse(1, &section_bytes(0x1000, &[0; 0x300])).unwrap();
        assert!(body.burden.is_some());
        assert_eq!(material_rule(&body, "*mask"), MaterialRule::Hide);
        assert_eq!(material_rule(&body, "&cloth"), MaterialRule::Paint);
        assert_eq!(material_rule(&body, "leather"), MaterialRule::Draw);
        assert_eq!(impact_sound_class(6), Some(6));
        assert_eq!(impact_sound_class(7), None);
    }

    #[test]
    fn a_door_or_operative_kind_only_without_weapon_or_body() {
        let mut door = item("door");
        door.door = Some(Door {
            flags: 0,
            key: 0,
            lock_sound: 0,
            slam_sound: 0,
            hinge_sound: 0,
            unlatch_sound: 0,
        });
        assert_eq!(tree_kind(&door), KIND_DOOR);
        assert_eq!(tree_kind(&operative_item("lever")), KIND_OPERATIVE);
        assert_eq!(tree_kind(&item("chair")), 0);
        // The node kind follows the tree: a model takes the tree kind (door
        // first), a model-less volume record the volume kind even with an
        // operative section, the placeholder the operative kind.
        let mut both = operative_item("gate");
        both.door = door.door;
        assert_eq!(node_kind(&both), KIND_DOOR);
        let mut volume = Item::parse(1, &section_bytes(objdb::VOLUME, &[0; 20])).unwrap();
        assert_eq!(node_kind(&volume), KIND_VOLUME);
        volume.operative = operative_item("").operative;
        assert_eq!(node_kind(&volume), KIND_VOLUME);
        volume.model = "crate".into();
        assert_eq!(node_kind(&volume), KIND_OPERATIVE);
        assert_eq!(node_kind(&operative_item("")), KIND_OPERATIVE);
        assert_eq!(node_kind(&item("")), 0);
        // The content stream dispatches on the same three words.
        assert_eq!(
            (KIND_OPERATIVE, KIND_DOOR, KIND_VOLUME),
            (
                exa_assets::content_stream::OBJECT_OPERATIVE,
                exa_assets::content_stream::OBJECT_DOOR,
                exa_assets::content_stream::OBJECT_VOLUME
            )
        );
        assert!(factory_copies_locally(0x40, 7));
        assert!(!factory_copies_locally(0x40, 0x4000_0007));
        assert!(!factory_copies_locally(0, 7));
    }

    #[test]
    fn the_collision_body_follows_the_records_sections_in_the_originals_order() {
        let mut lever = item("lever");
        lever.operative = Some(Operative {
            class: operative::LEVER,
            ..operative_item("").operative.unwrap()
        });
        assert_eq!(collision_body(&lever), CollisionBody::Lever);
        let mut plate = item("plate");
        plate.operative = Some(Operative {
            class: operative::PUSH_SURFACE,
            ..operative_item("").operative.unwrap()
        });
        assert_eq!(collision_body(&plate), CollisionBody::PushSurface);
        let mut door = item("door");
        door.door = Some(Door {
            flags: 0,
            key: 0,
            lock_sound: 0,
            slam_sound: 0,
            hinge_sound: 0,
            unlatch_sound: 0,
        });
        assert_eq!(collision_body(&door), CollisionBody::Door);
        // The operative section is tested first: a lever that is also a
        // door takes the lever's class, and an operative of another class
        // takes none even with a door section.
        let mut both = lever;
        both.door = door.door;
        assert_eq!(collision_body(&both), CollisionBody::Lever);
        let mut other = operative_item("gate");
        other.door = door.door;
        assert_eq!(collision_body(&other), CollisionBody::None);
        assert_eq!(collision_body(&item("chair")), CollisionBody::None);
    }

    #[test]
    fn a_master_is_kept_per_model_unless_painted() {
        let mut masters = Masters::new();
        let mut loads = 0;
        let a = masters.create("chair", false, || {
            loads += 1;
            1
        });
        let b = masters.create("chair", false, || {
            loads += 1;
            2
        });
        assert_eq!(loads, 1);
        assert!(Arc::ptr_eq(&a, &b));
        let c = masters.create("tunic", true, || 3);
        let d = masters.create("tunic", true, || 4);
        assert_eq!((*c, *d), (3, 4), "painted trees load each time");
        assert_eq!(masters.len(), 1);
        assert!(masters.get("tunic").is_none());
    }

    #[test]
    fn a_release_reports_a_mismatch_and_still_relinks() {
        let mut masters = [10, 0];
        let mut slots = [1, 0];
        // Another tree in the slot is reported, and the slot still goes to
        // the instance's master.
        assert_eq!(
            release_instance(&mut masters, &mut slots, 0, 11, 12),
            (
                true,
                Some(MismatchedInstance {
                    instance: 11,
                    found: 10
                })
            )
        );
        assert_eq!((masters[0], slots[0]), (12, 1));
        // The master itself clears the slot.
        assert_eq!(
            release_instance(&mut masters, &mut slots, 0, 12, 12),
            (true, None)
        );
        assert_eq!((masters[0], slots[0]), (0, 0));
        assert_eq!(
            release_instance(&mut masters, &mut slots, 1, 9, 9),
            (false, None)
        );
    }
}
