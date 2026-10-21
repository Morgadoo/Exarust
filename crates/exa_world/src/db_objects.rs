//! The object manager's scene decisions: which tier an object id names,
//! which collision body its record asks for, what its scene tree is, the
//! entity kind it is marked with, the invisible container volume and the
//! 50 cm usable placeholder, the apparel paint and hide rules, the master
//! cache every instance clones, and the release check.
//!
//! The original's `Create` resolves the record, loads the model from
//! `Objects.rpk` once into a per-record master and instances clones of it;
//! a factory weapon is assembled instead, a record without a model becomes
//! an invisible container volume or, when usable, a 50 cm placeholder. The
//! record's flags choose the collision body: an operative's class 3 (a
//! lever) or 5 (a push surface) and a door each take a tetrahedral variant,
//! everything else none. Wearables mark their node tree (`0x4000`, children
//! `|0x5000`), doors `0x40000`, operatives `0x10000`; apparel's `&`
//! materials are painted and its `*` materials hidden.
//!
//! Evidence: the object manager's create, release, flag and collision-class
//! routines in the component map (RF_DBObjects).

use std::collections::HashMap;
use std::sync::Arc;

use bevy::prelude::Component;
use exa_assets::PackKind;
use exa_formats::containers;
use exa_formats::objdb::{Item, operative};

/// An invisible container volume's extents, cm: the original's scene node
/// without a model, sized by its volume section.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ContainerVolume {
    pub extents: [u16; 3],
}

/// A usable object's 50 cm placeholder node: the extents and the bounding
/// sphere radius the original gives it.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct PlaceholderNode {
    pub extents: [f32; 3],
    pub radius: f32,
}

impl Default for PlaceholderNode {
    fn default() -> Self {
        let (extents, radius) = placeholder();
        Self { extents, radius }
    }
}

/// The section of an invisible container volume.
pub const CONTAINER_VOLUME: u32 = 0x8000_ac07;
/// The section of a usable object.
pub const USABLE: u32 = 0x1_0000;
/// The side of a usable object's placeholder cube, in centimetres.
pub const PLACEHOLDER_SIDE: f32 = 50.0;
/// The radius the original gives the 50 cm cube's bounding sphere:
/// `√(50² + 50² + 50²)`.
pub const PLACEHOLDER_RADIUS: f32 = 86.602_54;

/// The node flag of a wearable object.
pub const WEARABLE: u32 = 0x4000;
/// The node flag of a door.
pub const DOOR: u32 = 0x4_0000;
/// The node flag of an operative object.
pub const OPERATIVE: u32 = 0x1_0000;
/// The node flag every child of a wearable receives with its own bits.
pub const CHILD_WEARABLE: u32 = 0x5000;
/// The flag a child copies from its parent's `0x80000`.
pub const HIDDEN: u32 = 0x8_0000;

/// Which object database an id names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tier {
    /// The base templates.
    Base,
    /// The promoted world objects.
    World,
    /// The level's local objects.
    Local,
}

impl Tier {
    /// The tier the id's top bits choose: bit 31 local, else bit 30 world,
    /// else base. The lookup id drops those bits.
    pub fn of(id: u32) -> (Self, u32) {
        if id & 0x8000_0000 != 0 {
            (Tier::Local, id & 0x3FFF_FFFF)
        } else if id & 0x4000_0000 != 0 {
            (Tier::World, id & 0x3FFF_FFFF)
        } else {
            (Tier::Base, id)
        }
    }
}

/// The flags a record's sections give its object.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ObjectFlags(pub u32);

impl ObjectFlags {
    /// Wearable: has an item body.
    pub fn wearable(self) -> bool {
        self.0 & WEARABLE != 0
    }
    /// Door.
    pub fn door(self) -> bool {
        self.0 & DOOR != 0
    }
    /// Operative.
    pub fn operative(self) -> bool {
        self.0 & OPERATIVE != 0
    }
}

/// The flag word a record's sections imply, checked in the original's
/// order: wearable first, then door, then operative.
pub fn object_flags(item: &Item) -> ObjectFlags {
    if !item.worn_model.is_empty() || item.burden.is_some() || item.appearance.is_some() {
        ObjectFlags(WEARABLE)
    } else if item.door.is_some() {
        ObjectFlags(DOOR)
    } else if item.operative.is_some() {
        ObjectFlags(OPERATIVE)
    } else {
        ObjectFlags(0)
    }
}

/// The collision body a record's flags ask for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CollisionClass {
    /// No body of its own.
    None,
    /// A lever's tetrahedron.
    Lever,
    /// A push surface's tetrahedron.
    PushSurface,
    /// A door's tetrahedron.
    Door,
}

/// The collision body class, in the original's branch order: an operative
/// object checks its class (3 a lever, 5 a push surface, anything else
/// none, even when the record also has a door section); without an
/// operative section a door section gives the door body.
pub fn collision_class(item: &Item) -> CollisionClass {
    if let Some(operative) = item.operative {
        return match operative.class {
            operative::LEVER => CollisionClass::Lever,
            operative::PUSH_SURFACE => CollisionClass::PushSurface,
            _ => CollisionClass::None,
        };
    }
    if item.door.is_some() {
        CollisionClass::Door
    } else {
        CollisionClass::None
    }
}

/// What a record's scene tree is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SceneTree<'a> {
    /// The model from a pack, as named.
    Model { pack: PackKind, entry: &'a str },
    /// A weapon the factory assembles (`@` and the name after it).
    FactoryWeapon(&'a str),
    /// An invisible container volume, with its three extents.
    ContainerVolume([u16; 3]),
    /// A 50 cm placeholder cube for a usable object.
    Placeholder,
    /// The record or its model is missing.
    Missing,
}

/// The tree `item` gets when its model resolves to `model` (the pack and
/// entry the object manager found), as the original's create decides it.
pub fn scene_tree<'a>(item: &'a Item, model: Option<(PackKind, &'a str)>) -> SceneTree<'a> {
    if let Some(factory) = item.model.strip_prefix('@') {
        return SceneTree::FactoryWeapon(factory);
    }
    if let Some((pack, entry)) = model {
        return SceneTree::Model { pack, entry };
    }
    SceneTree::Missing
}

/// The tree for a record with no model: its container volume when it has
/// the volume section, else a placeholder for a usable object, else
/// missing. `payload` is the record's section chain.
pub fn modelless_tree(item: &Item, payload: &[u8]) -> SceneTree<'static> {
    if let Some(body) = containers::section(payload, CONTAINER_VOLUME) {
        let extent = |at: usize| {
            body.get(at..at + 2)
                .map(|b| u16::from_le_bytes([b[0], b[1]]))
                .unwrap_or(0)
        };
        return SceneTree::ContainerVolume([extent(0), extent(2), extent(4)]);
    }
    if item.operative.is_some() || containers::section(payload, USABLE).is_some() {
        return SceneTree::Placeholder;
    }
    SceneTree::Missing
}

/// The placeholder cube's extents and radius: 50 cm a side, and the sphere
/// the original gives it.
pub fn placeholder() -> ([f32; 3], f32) {
    ([PLACEHOLDER_SIDE; 3], PLACEHOLDER_RADIUS)
}

/// The node flags a wearable's children take: the parent's hidden bit, the
/// child's own bits, and `0x5000` (wearable and draw-through).
pub fn child_flags(parent: ObjectFlags, child: u32) -> u32 {
    (parent.0 & HIDDEN) | child | CHILD_WEARABLE
}

/// The node flags of a wearable object itself: its own kind and `0x4000`.
pub fn wearable_flags() -> u32 {
    WEARABLE
}

/// The node flags of a hidden instance: `0x80000`.
pub fn hidden_flags(flags: u32) -> u32 {
    flags | HIDDEN
}

/// The impact sound class of an item body's byte at `0x1a6`: a class only
/// below 7. The original indexes its `FlsCol` name table with it; the table
/// itself is retail content, so the index is what this returns.
pub fn impact_sound_class(byte: u8) -> Option<u8> {
    (byte < 7).then_some(byte)
}

/// Whether an apparel material name is one the original hides rather than
/// draws: the `*` prefix (the `_null` groups are dropped by the loader
/// already).
pub fn hidden_material(name: &str) -> bool {
    name.starts_with('*')
}

/// Whether a material name is one the apparel painter takes: the `&`
/// prefix.
pub fn painted_material(name: &str) -> bool {
    name.starts_with('&')
}

/// A master cache: one loaded scene tree per object id, cloned by every
/// instance. `T` is the scene tree type; the clone is an [`Arc`] so all
/// instances share the meshes and materials, as the original's clones do.
#[derive(Debug)]
pub struct Masters<T> {
    by_id: HashMap<u32, Arc<T>>,
    /// The per-model index the original keeps beside the ids.
    by_model: HashMap<String, Arc<T>>,
}

impl<T> Default for Masters<T> {
    fn default() -> Self {
        Self {
            by_id: HashMap::new(),
            by_model: HashMap::new(),
        }
    }
}

impl<T> Masters<T> {
    /// An empty cache.
    pub fn new() -> Self {
        Self::default()
    }

    /// The master of `id`, loading it once; a later call shares it.
    pub fn master(&mut self, id: u32, load: impl FnOnce() -> T) -> Arc<T> {
        if let Some(master) = self.by_id.get(&id) {
            return master.clone();
        }
        let master = Arc::new(load());
        self.by_id.insert(id, master.clone());
        master
    }

    /// The master of a model key, loading it once; the original's table
    /// over the pack's model slots.
    pub fn master_of(&mut self, key: &str, load: impl FnOnce() -> T) -> Arc<T> {
        if let Some(master) = self.by_model.get(key) {
            return master.clone();
        }
        let master = Arc::new(load());
        self.by_model.insert(key.to_string(), master.clone());
        master
    }

    /// The master already loaded for `id`.
    pub fn get(&self, id: u32) -> Option<&Arc<T>> {
        self.by_id.get(&id)
    }

    /// Forgets the master of `id`, as a release does when the last instance
    /// goes.
    pub fn release(&mut self, id: u32) -> Option<Arc<T>> {
        self.by_id.remove(&id)
    }

    /// How many masters are cached.
    pub fn len(&self) -> usize {
        self.by_id.len()
    }

    /// Whether no master is cached.
    pub fn is_empty(&self) -> bool {
        self.by_id.is_empty()
    }
}

/// The release check failed: an instance named a master the table does not
/// hold for its slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MismatchedInstance {
    pub instance: usize,
    pub expected: usize,
    pub found: usize,
}

/// The master the release found for an instance: the slot table gives the
/// master's index, and that master must be the instance's own. A null
/// instance clears the slot. The original compares the two pointers and
/// reports "Mismatched database object instance reference" on a mismatch.
pub fn release_instance(
    masters: &mut [usize],
    slots: &mut [usize],
    instance: usize,
    slot: usize,
    own_master: usize,
) -> Result<bool, MismatchedInstance> {
    let Some(&at) = slots.get(slot) else {
        return Ok(false);
    };
    if at == 0 {
        return Ok(false);
    }
    let master = at - 1;
    if masters.get(master).copied() != Some(instance) {
        return Err(MismatchedInstance {
            instance,
            expected: own_master,
            found: master,
        });
    }
    if own_master == instance {
        masters[master] = 0;
        slots[slot] = 0;
    } else {
        masters[master] = own_master;
    }
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use exa_formats::objdb::{DESTINATIONS, Door, Operative};

    fn item(model: &str) -> Item {
        Item {
            id: 1,
            name: "thing".into(),
            model: model.into(),
            model_flags: 0,
            worn_model: String::new(),
            appearance: None,
            slot_layers: [0; exa_formats::objdb::SLOTS],
            occupancy: [0; exa_formats::objdb::OCCUPANCY_BONES],
            weapon: None,
            armour: None,
            shield: None,
            door: None,
            operative: None,
            burden: None,
            equip_lock: 0,
        }
    }

    fn door_record() -> Door {
        Door {
            flags: 0,
            key: 0,
            lock_sound: 0,
            slam_sound: 0,
            hinge_sound: 0,
            unlatch_sound: 0,
        }
    }

    fn section_bytes(tag: u32, body: &[u8]) -> Vec<u8> {
        let mut out = tag.to_le_bytes().to_vec();
        out.extend_from_slice(&(body.len() as u32 + 8).to_le_bytes());
        out.extend_from_slice(body);
        out
    }

    fn operative_item(class: u8) -> Item {
        let mut it = item("model");
        it.operative = Some(Operative {
            class,
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
    fn ids_choose_their_tier() {
        assert_eq!(Tier::of(7), (Tier::Base, 7));
        assert_eq!(Tier::of(0x4000_0007), (Tier::World, 7));
        assert_eq!(Tier::of(0x8000_0007), (Tier::Local, 7));
        assert_eq!(Tier::of(0xC000_0007), (Tier::Local, 0x0000_0007));
    }

    #[test]
    fn the_record_flags_and_collision_follow_the_branch_order() {
        let plain = item("model");
        assert_eq!(object_flags(&plain), ObjectFlags(0));
        assert_eq!(collision_class(&plain), CollisionClass::None);
        let mut wearable = plain.clone();
        wearable.worn_model = "Trouser_A".into();
        assert_eq!(object_flags(&wearable), ObjectFlags(WEARABLE));
        let mut door = plain.clone();
        door.door = Some(door_record());
        assert_eq!(object_flags(&door), ObjectFlags(DOOR));
        assert_eq!(collision_class(&door), CollisionClass::Door);
        assert_eq!(
            collision_class(&operative_item(operative::LEVER)),
            CollisionClass::Lever
        );
        assert_eq!(
            collision_class(&operative_item(operative::PUSH_SURFACE)),
            CollisionClass::PushSurface
        );
        assert_eq!(
            collision_class(&operative_item(operative::KEY)),
            CollisionClass::None
        );
        // An operative item with a door section still checks its class.
        let mut both = operative_item(operative::KEY);
        both.door = Some(door_record());
        assert_eq!(collision_class(&both), CollisionClass::None);
    }

    #[test]
    fn a_model_or_factory_or_placeholder() {
        let plain = item("chair");
        assert_eq!(
            scene_tree(&plain, Some((PackKind::Objects, "chair"))),
            SceneTree::Model {
                pack: PackKind::Objects,
                entry: "chair"
            }
        );
        let factory = item("@ub05ssworda");
        assert_eq!(
            scene_tree(&factory, None),
            SceneTree::FactoryWeapon("ub05ssworda")
        );
        assert_eq!(scene_tree(&plain, None), SceneTree::Missing);
        assert_eq!(modelless_tree(&plain, &[]), SceneTree::Missing);
        assert_eq!(
            modelless_tree(&operative_item(operative::KEY), &[]),
            SceneTree::Placeholder
        );
        let volume = section_bytes(CONTAINER_VOLUME, &[10, 0, 20, 0, 30, 0]);
        assert_eq!(
            modelless_tree(&plain, &volume),
            SceneTree::ContainerVolume([10, 20, 30])
        );
        let usable = section_bytes(USABLE, &[]);
        assert_eq!(modelless_tree(&plain, &usable), SceneTree::Placeholder);
        assert_eq!(placeholder().0, [50.0; 3]);
        assert!((placeholder().1 - (7500.0f32).sqrt()).abs() < 1e-3);
    }

    #[test]
    fn flags_travel_down_and_hide() {
        assert_eq!(wearable_flags(), 0x4000);
        assert_eq!(child_flags(ObjectFlags(0), OPERATIVE), 0x5000 | OPERATIVE);
        assert_eq!(
            child_flags(ObjectFlags(HIDDEN), DOOR),
            HIDDEN | DOOR | CHILD_WEARABLE
        );
        assert_eq!(hidden_flags(0), HIDDEN);
    }

    #[test]
    fn materials_and_impact_classes() {
        assert!(hidden_material("*mask"));
        assert!(!hidden_material("&cloth"));
        assert!(painted_material("&cloth"));
        assert!(!painted_material("*mask"));
        assert_eq!(impact_sound_class(0), Some(0));
        assert_eq!(impact_sound_class(6), Some(6));
        assert_eq!(impact_sound_class(7), None);
        assert_eq!(impact_sound_class(255), None);
    }

    #[test]
    fn a_master_is_loaded_once_and_cloned() {
        let mut masters = Masters::new();
        let mut loads = 0;
        let a = masters.master(7, || {
            loads += 1;
            "tree".to_string()
        });
        let b = masters.master(7, || {
            loads += 1;
            "other".to_string()
        });
        assert_eq!(loads, 1);
        assert!(Arc::ptr_eq(&a, &b));
        assert_eq!(&*a, "tree");
        assert_eq!(masters.len(), 1);
        let c = masters.master_of("chair", || "chair-tree".to_string());
        assert_eq!(&*c, "chair-tree");
        assert!(masters.get(8).is_none());
        masters.release(7);
        assert!(masters.is_empty());
    }

    #[test]
    fn a_release_checks_the_master_and_relinks() {
        let mut masters = [10, 0, 12];
        let mut slots = [1, 0, 0];
        // A mismatched instance is reported, the tables untouched.
        assert_eq!(
            release_instance(&mut masters, &mut slots, 11, 0, 11),
            Err(MismatchedInstance {
                instance: 11,
                expected: 11,
                found: 0
            })
        );
        assert_eq!(slots[0], 1);
        // The last instance clears its slot; another relinks it to its own
        // master.
        assert_eq!(
            release_instance(&mut masters, &mut slots, 10, 0, 10),
            Ok(true)
        );
        assert_eq!(masters[0], 0);
        assert_eq!(slots[0], 0);
        assert_eq!(
            release_instance(&mut masters, &mut slots, 9, 1, 9),
            Ok(false)
        );
        let mut masters = [10];
        let mut slots = [1];
        assert_eq!(
            release_instance(&mut masters, &mut slots, 10, 0, 10),
            Ok(true)
        );
    }
}
