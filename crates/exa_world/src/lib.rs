//! Levels: placing models, props and lights of the original levels in the
//! Bevy world. Walls and floors come from the tile grid (`grid`), outdoor
//! ground from the terrain heightfields (`terrain`); the grid's rooms decide
//! what shows (`rooms`, `room_view`).

pub mod arc;
pub mod bodies;
pub mod cloth;
pub mod cloth_draw;
pub mod cloth_parts;
pub mod db_objects;
pub mod editor;
pub mod environment;
pub mod fire;
pub mod flame;
pub mod gi_residency;
pub mod gi_volume;
pub mod grid;
pub mod grid_sets;
pub mod level_lifecycle;
pub mod light_flood;
pub mod light_source;
pub mod markers;
pub mod mechanisms;
pub mod nav;
pub mod node_parts;
pub mod particles;
pub mod presets;
pub mod published_light;
pub mod room_view;
pub mod rooms;
pub mod scene_env;
pub mod scene_nodes;
pub mod shadow_links;
pub mod start;
pub mod sun_shadow;
pub mod teleporters;
pub mod terrain;
pub mod voxeliser;

use bevy::prelude::*;
use exa_assets::{Model, ModelLoader, PackKind, transform_from_rows};
use exa_formats::objdb::{self, Appearance, Item};
use exa_formats::rdb::Database;
use exa_formats::rfc::{Light, Scene};

pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            terrain::TerrainPlugin,
            environment::EnvironmentPlugin,
            arc::ArcPlugin,
            fire::FirePlugin,
            flame::FlamePlugin,
            gi_volume::GiVolumePlugin,
            nav::NavPlugin,
            shadow_links::ShadowLinksPlugin,
            light_source::LightSourcePlugin,
            room_view::RoomViewPlugin,
            sun_shadow::SunShadowPlugin,
            cloth::ClothPlugin,
            level_lifecycle::LevelChangePlugin,
            mechanisms::MechanismPlugin,
        ));
    }
}

/// Bounding sphere of a model node, in its local space, and the half
/// extents of its bounding box about the same centre.
#[derive(Component, Debug, Clone, Copy)]
pub struct PartBounds {
    pub centre: Vec3,
    pub radius: f32,
    pub half_extents: Vec3,
}

/// The loaded level's own object database (the records its characters,
/// placed objects and containers name with local ids), replaced at each
/// level load.
#[derive(Resource, Debug, Clone, Default)]
pub struct LevelLocalItems(pub Vec<Item>);

/// The loaded level's own records as stored and its string table, replaced
/// at each level load: what the item catalogue builds the level's
/// description cards from.
#[derive(Resource, Debug, Clone, Default)]
pub struct LevelLocalRecords {
    pub records: Vec<(u32, Vec<u8>)>,
    pub strings: Option<exa_formats::objstrings::Library>,
}

/// Marks the root entity of a spawned level.
#[derive(Component)]
pub struct LevelRoot {
    pub name: String,
}

/// A placed object of the object database, on its model root: the record
/// id, the placed object's database id with its tier tag (the scene node's
/// id word the original looks up and rewrites after a promotion) and the
/// condition byte of its placement word (bits 16..23), which a power
/// crystal's charge is.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlacedItem {
    /// The record id, untagged.
    pub id: u32,
    /// The placed object's id as the level names it, tag included.
    pub tagged_id: u32,
    pub condition: u8,
}

impl PlacedItem {
    /// The item and placement.
    pub fn of(item: &Item, object: &exa_formats::rfc::PlacedObject) -> Self {
        Self {
            id: item.id,
            tagged_id: object.id,
            condition: ((object.flags >> 16) & 0xFF) as u8,
        }
    }
}

/// The placement an item record gets when it is spawned into the scene by a
/// GUI drag rather than loaded from a level: its record id as both the
/// record and the scene id, the drag's condition word in the flags, and an
/// identity frame (the drop's own point is written separately).
pub fn dropped_object(id: u32, condition: u32) -> exa_formats::rfc::PlacedObject {
    exa_formats::rfc::PlacedObject {
        id,
        flags: condition,
        object_flags: 0,
        transform: [
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 1.0],
            [0.0, 0.0, 0.0],
        ],
        slot: 0,
        second_word: 0,
        word_3d22: None,
        attachments: Vec::new(),
    }
}

/// What the player's interactions read off a placed object's record and
/// placement: the placement word (the scene node's instance word: bit 0 a
/// saved latch the container open tests, bits 16..23 the condition or used
/// mark, bit `0x8000` the first touch), its usable section and its
/// container section. The container's children are the live copy the
/// container popup loads and saves back when it closes, as the original
/// rewrites the record's container section. Only objects with a usable or a
/// container section carry it.
#[derive(Component, Debug, Clone, PartialEq)]
pub struct ObjectSections {
    pub placement: u32,
    pub usable: Option<exa_formats::objdb::Operative>,
    pub container: Option<exa_formats::containers::Container>,
}

impl ObjectSections {
    /// The sections of `item` placed by `object`, or `None` when it has
    /// neither a usable nor a container section.
    pub fn of(item: &Item, object: &exa_formats::rfc::PlacedObject) -> Option<Self> {
        (item.operative.is_some() || item.container.is_some()).then(|| Self {
            placement: object.flags,
            usable: item.operative,
            container: item.container.clone(),
        })
    }
}

/// The scene entity whose placed object carries the database id `id`, as
/// the original's lookup walks the scene's roots and their child nodes and
/// compares each node's id word. Exarust keeps a placed object's id on its
/// model root, so the walk covers those roots in spawn order; a model
/// node's own id is not a database record here.
pub fn object_entity<'a>(
    placed: impl IntoIterator<Item = (Entity, &'a PlacedItem)>,
    id: u32,
) -> Option<Entity> {
    placed
        .into_iter()
        .find(|(_, placed)| placed.tagged_id == id)
        .map(|(entity, _)| entity)
}

/// Rewrites the placed ids of the scene through the database's promotion
/// table, as the original does after a save promotes local objects to the
/// world tier: an id tagged exactly local that the table holds becomes its
/// world id, every other id stays. Returns how many ids changed. The
/// caller's query supplies the roots (the original walks their child nodes
/// too; they are not separate records here).
pub fn remap_placed_ids<'a>(
    placed: impl IntoIterator<Item = &'a mut PlacedItem>,
    db: &exa_formats::objdb::ObjectDb,
) -> usize {
    let mut changed = 0;
    for placed in placed {
        let new = db.remap_entity_id(placed.tagged_id);
        if new != placed.tagged_id {
            placed.tagged_id = new;
            changed += 1;
        }
    }
    changed
}

/// A teleporter or reconstruction device placed in a level: its operative
/// kind and record. The first `[` node of its model marks the crystal slot.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct TeleportDevice {
    pub kind: u8,
    pub operative: exa_formats::objdb::Operative,
}

/// What a level spawn produced.
#[derive(Debug, Clone)]
pub struct LevelInfo {
    pub props: usize,
    pub missing_props: usize,
    pub lights: usize,
    pub grid: Option<grid::GridStats>,
    pub terrain: Option<terrain::TerrainStats>,
    pub lighting: environment::Lighting,
    /// Characters placed in the level.
    pub characters: Vec<PlacedCharacter>,
    /// Object-database objects placed in the level, and those whose model
    /// was not found.
    pub placed_objects: usize,
    pub missing_objects: usize,
    /// The content stream's count of placed objects it built nothing for
    /// (no record, or no model), which the original reports as "N missing
    /// database objects.".
    pub missing_database_objects: usize,
    /// The placed player's position, metres: the object the stream named
    /// by the session's arrival word (an operative of its kind, or a
    /// transition marker matching both words), whose position becomes the
    /// focus the streamed characters are ordered by.
    pub player_focus: Option<Vec3>,
    /// The rigid bodies of its props, placed objects and own model:
    /// registered in the solver, and left static.
    pub bodies: bodies::BodyCounts,
    /// A floored cell to start in (stand-in for the level's start marker).
    pub spawn: Option<Vec3>,
    /// The level's transition markers, in placement order: the entrance and
    /// the id of the level each leads back from, with its position.
    pub player_markers: Vec<PlayerMarker>,
    /// The level's alarm zones, in placement order ([`alarm_volumes`]).
    pub alarm_volumes: Vec<AlarmVolume>,
    /// The level environment's reverb decay offset, seconds.
    pub reverb_decay: f32,
    pub bounds_min: Vec3,
    pub bounds_max: Vec3,
}

/// A level's transition marker: a placed object whose volume section names
/// an entrance and the level it leads back from (the object's own record
/// name is that level). The stream makes the one matching the session's two
/// words the player and orders the streamed characters by its position.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlayerMarker {
    /// The entrance number the session's arrival variable must equal.
    pub entrance: u32,
    /// The id, in `locales.rdb`, of the level the session came from.
    pub previous_level: u32,
    /// The marker's world position, metres.
    pub position: Vec3,
}

/// Every transition marker a level places, in placement order, from its
/// objects and their items' volume sections.
pub fn player_markers(scene: &Scene, global: &[Item]) -> Vec<PlayerMarker> {
    let mut out = Vec::new();
    for object in &scene.placed_objects {
        // A marker has no model of its own (it is an invisible box), so the
        // record is looked up directly rather than through the model path.
        if object.slot != 0 {
            continue;
        }
        let items: &[Item] = if object.is_local() {
            &scene.local_items
        } else {
            global
        };
        let id = object.id & exa_formats::rfc::PlacedObject::ID_MASK;
        let Some(item) = items.iter().find(|item| item.id == id) else {
            continue;
        };
        let Some(words) = item.volume.and_then(|v| v.marker()) else {
            continue;
        };
        let t = object.transform[3];
        out.push(PlayerMarker {
            entrance: words[0],
            previous_level: words[1],
            position: Vec3::new(t[0], t[1], t[2]),
        });
    }
    out
}

/// A level's alarm zone: a placed trigger volume whose record names the
/// pass item that lets its carrier cross ([`objdb::Volume::alarm_pass`]).
/// The player stepping into one without the pass wakes the statue golems
/// around it (`exa_ai::golem`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AlarmVolume {
    /// The volume's box in centimetres, the original's world unit: centred
    /// on the object's origin, its half extents the section's three, its
    /// axes the placement's rotation rows [inferred: the container volume
    /// the object manager builds keeps only the extents and a radius].
    pub bounds: exa_physics::intersect::Obb,
    /// The node's bounding radius (cm): the extents' length, the squares of
    /// x and z summed first ([`db_objects::container_volume`]).
    pub radius: f32,
    /// The pass item's id.
    pub pass: u32,
    /// The object's node is an `Alarm_` node, which the alarm return looks
    /// up: its item's name starts with `Alarm_` [inferred: the object node
    /// takes its item's name; every shipped alarm item is named `Alarm_`
    /// and its pass in hex].
    pub alarm_node: bool,
}

/// Every alarm zone a level places, in placement order, from its objects
/// and their items' volume sections: a model-less object (slot 0) whose
/// record's volume section is an alarm zone.
pub fn alarm_volumes(scene: &Scene, global: &[Item]) -> Vec<AlarmVolume> {
    let mut out = Vec::new();
    for object in &scene.placed_objects {
        if object.slot != 0 {
            continue;
        }
        let items: &[Item] = if object.is_local() {
            &scene.local_items
        } else {
            global
        };
        let id = object.id & exa_formats::rfc::PlacedObject::ID_MASK;
        let Some(item) = items.iter().find(|item| item.id == id) else {
            continue;
        };
        let Some(volume) = item.volume else {
            continue;
        };
        let Some(pass) = volume.alarm_pass() else {
            continue;
        };
        let extents = volume.extents.map(f32::from);
        let row = |i: usize| Vec3::from_array(object.transform[i]);
        let t = object.transform[3];
        out.push(AlarmVolume {
            bounds: exa_physics::intersect::Obb::new(
                Vec3::new(t[0], t[1], t[2]) * 100.0,
                Vec3::from_array(extents),
                [row(0), row(1), row(2)],
            ),
            radius: db_objects::container_volume(extents).radius,
            pass,
            alarm_node: item.name.as_bytes().starts_with(b"Alarm_"),
        });
    }
    out
}

/// A character placed in a level, resolved to what can be spawned.
#[derive(Debug, Clone)]
pub struct PlacedCharacter {
    pub name: String,
    /// Feet position, metres.
    pub position: Vec3,
    /// Facing: looks along (sin yaw, 0, cos yaw).
    pub yaw: f32,
    pub dead: bool,
    /// 0 for humans.
    pub race: u8,
    pub female: bool,
    /// Worn items, outermost last.
    pub worn: Vec<Worn>,
    /// Item in the right hand: model (`@factory` for assembled weapons) and
    /// its weapon data.
    pub held: Option<(String, Option<exa_formats::objdb::Weapon>)>,
    /// The four hand slots of the record, resolved to their items.
    pub hands: [Option<HandItem>; exa_formats::characters::HANDS],
    /// The character's whole record, from which its spawn derives its
    /// stats.
    pub record: exa_formats::characters::CharacterRecord,
    /// What it wears and the shield it holds weigh, as the spawn's armour
    /// pass sums them.
    pub encumbrance: f32,
    /// What its worn items' special kinds do, as the spawn's armour pass
    /// dispatches them (kind 2 grants it a second wind).
    pub specials: exa_formats::encumbrance::WornSpecials,
    /// The pose the level saved with it, if any.
    pub pose: exa_formats::characters::SavedPose,
    /// Spawn the dead from their built pose and let them collapse. Only the
    /// command line's synthetic corpse does; a level's dead are placed from
    /// their saved pose and never fall.
    pub fall: bool,
}

/// An item in one of a record's four hand slots.
#[derive(Debug, Clone)]
pub struct HandItem {
    /// The model it shows (`@factory` for an assembled weapon).
    pub model: String,
    /// Its weapon section, when it is a weapon.
    pub weapon: Option<exa_formats::objdb::Weapon>,
    /// It blocks as a shield.
    pub shield: bool,
    /// Its shield record (block factors), when it has one.
    pub shield_record: Option<exa_formats::weapon_record::ShieldRecord>,
    /// It has the torch section.
    pub torch: bool,
}

/// An item a placed character wears.
#[derive(Debug, Clone)]
pub struct Worn {
    /// `Characters.rpk` entry.
    pub model: String,
    pub appearance: Option<Appearance>,
    /// Body slots it hides, as a bit mask.
    pub hides: u32,
    /// Its coverage word on the head slot (layer L is byte L): the hair
    /// reads the worn set's union of these.
    pub head_word: u64,
    pub armour: Option<exa_formats::objdb::Armour>,
}

/// A record's appearance as a spawn reads it: its worn items and the items
/// in its four hands.
#[derive(Debug, Clone, Default)]
pub struct RecordLook {
    /// Worn items, outermost last.
    pub worn: Vec<Worn>,
    /// The four hand slots, resolved.
    pub hands: [Option<HandItem>; exa_formats::characters::HANDS],
    /// The main hand's item: model and weapon data.
    pub held: Option<(String, Option<exa_formats::objdb::Weapon>)>,
}

/// Resolves a record's worn items and hands from the items `find` names.
///
/// A worn item is shown as the model its body names, which may differ from
/// the one it shows on the ground or lack one. An item without a model is
/// not shown at all. `find` receives the record's reference (its local bit
/// kept), so a caller with a level's local database can route on it.
pub fn record_look<'a>(
    record: &exa_formats::characters::CharacterRecord,
    mut find: impl FnMut(u32) -> Option<&'a Item>,
) -> RecordLook {
    let hand = |hand: exa_formats::characters::ItemRef| -> Option<HandItem> {
        let item = find(hand.id)?;
        (!item.model.is_empty()).then(|| HandItem {
            model: item.model.clone(),
            weapon: item.weapon,
            shield: item.shield.is_some(),
            shield_record: item.shield,
            torch: item.torch,
        })
    };
    let hands = record.hands.map(hand);
    let held = hands[0]
        .as_ref()
        .map(|item| (item.model.clone(), item.weapon));
    let worn = record
        .worn
        .iter()
        .filter_map(|worn| {
            let item = find(worn.id)?;
            (!item.worn_model.is_empty()).then(|| Worn {
                model: item.worn_model.clone(),
                appearance: item.appearance,
                hides: item.hidden_slots(),
                head_word: item.slot_word(10),
                armour: item.armour,
            })
        })
        .collect();
    RecordLook { worn, hands, held }
}

/// Resolves a level's placed characters: records from its own database,
/// worn items from its local object database or the game's.
pub fn placed_characters(
    scene: &Scene,
    global: &[Item],
    locale: Option<u32>,
) -> Vec<PlacedCharacter> {
    use exa_formats::characters::{ID_MASK, LOCAL_ITEM};
    let Some(chars) = &scene.characters else {
        return Vec::new();
    };
    chars
        .placements
        .iter()
        .filter_map(|p| {
            let record = chars.record(p.id)?;
            // A world-database character is created only in its own locale:
            // the level's local records always are.
            let world = p.id & 0x4000_0000 != 0 || record.id & 0x4000_0000 != 0;
            if world && locale.is_some_and(|here| record.locale != here) {
                return None;
            }
            let find = |id: u32| -> Option<&Item> {
                let items: &[Item] = if id & LOCAL_ITEM != 0 {
                    &scene.local_items
                } else {
                    global
                };
                items.iter().find(|i| i.id == id & ID_MASK)
            };
            let look = record_look(record, find);
            Some(PlacedCharacter {
                name: record.name.clone(),
                position: Vec3::from_array(p.position),
                yaw: p.yaw,
                dead: p.is_dead(),
                race: record.race,
                female: record.female,
                worn: look.worn,
                held: look.held,
                hands: look.hands,
                record: record.clone(),
                encumbrance: exa_formats::encumbrance::of_record(record, find),
                specials: exa_formats::encumbrance::WornSpecials::of_record(record, find),
                pose: p.pose.clone(),
                fall: false,
            })
        })
        .collect()
}

/// The item a placed object shows, when it is the level's own and has a
/// model.
fn placed_item<'a>(
    scene: &'a Scene,
    global: &'a [Item],
    object: &exa_formats::rfc::PlacedObject,
) -> Option<&'a Item> {
    if object.slot != 0 {
        return None;
    }
    let items: &[Item] = if object.is_local() {
        &scene.local_items
    } else {
        global
    };
    let id = object.id & exa_formats::rfc::PlacedObject::ID_MASK;
    // Triggers and markers have no model.
    items
        .iter()
        .find(|i| i.id == id)
        .filter(|i| !i.model.is_empty())
}

/// The pack and entry of a placed object's model (none for weapons a
/// factory assembles).
fn placed_object_model(
    loader: &ModelLoader<'_>,
    scene: &Scene,
    global: &[Item],
    object: &exa_formats::rfc::PlacedObject,
) -> Option<(PackKind, String)> {
    let item = placed_item(scene, global, object)?;
    if item.model.starts_with('@') {
        return None;
    }
    [PackKind::Objects, PackKind::Objlib]
        .into_iter()
        .find(|&kind| {
            loader
                .game
                .pack(kind)
                .is_some_and(|p| p.find(&item.model).is_some())
        })
        .map(|kind| (kind, item.model.clone()))
}

/// What spawning a placed object came to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PlacedSpawn {
    /// Placed as `entity`, bringing this many lights, its bodies registered
    /// or left static.
    Placed {
        entity: Entity,
        lights: usize,
        bodies: bodies::BodyCounts,
    },
    /// Its model is missing.
    Missing,
}

/// Spawns an object-database object a level places (an item, door or
/// container) as a static model. `None` for objects without a model
/// (triggers, markers) or of a slot other than the level's.
fn spawn_placed_object(
    commands: &mut Commands,
    loader: &mut ModelLoader<'_>,
    scene: &Scene,
    global: &[Item],
    parent: Entity,
    object: &exa_formats::rfc::PlacedObject,
    mechanism_bodies: bodies::MechanismBodies,
) -> Option<PlacedSpawn> {
    let item = placed_item(scene, global, object)?;
    // `@` names a weapon assembled by a factory.
    if let Some(factory) = item.model.strip_prefix('@') {
        let Some(model) = loader.factory_weapon(factory, item.weapon.as_ref()) else {
            return Some(PlacedSpawn::Missing);
        };
        let e = spawn_model(commands, &model, None);
        commands.entity(e).insert((
            transform_from_rows(&object.transform),
            Name::new(item.name.clone()),
            room_view::PendingRoom::default(),
            PlacedItem::of(item, object),
            markers::PlacedTemplate(item.parent),
            ChildOf(parent),
        ));
        if let Some(sections) = ObjectSections::of(item, object) {
            commands.entity(e).insert(sections);
        }
        return Some(PlacedSpawn::Placed {
            entity: e,
            lights: 0,
            bodies: bodies::BodyCounts::default(),
        });
    }
    let model = placed_object_model(loader, scene, global, object).and_then(|(kind, entry)| {
        let placement = transform_from_rows(&object.transform);
        placement_base(loader, kind, &entry, &object.attachments, placement)
    });
    let Some(model) = model else {
        // A record without a model name and with an operative section gets
        // the original's 50 cm placeholder node, so its placement exists.
        // A named model that is not found has no tree, as in the original.
        if db_objects::scene_tree(item, &[]) == db_objects::SceneTree::Placeholder {
            let entity = commands
                .spawn((
                    transform_from_rows(&object.transform),
                    Name::new(item.name.clone()),
                    room_view::PendingRoom::default(),
                    PlacedItem::of(item, object),
                    db_objects::PlaceholderNode::default(),
                    ChildOf(parent),
                ))
                .id();
            if let Some(sections) = ObjectSections::of(item, object) {
                commands.entity(entity).insert(sections);
            }
            return Some(PlacedSpawn::Placed {
                entity,
                lights: 0,
                bodies: bodies::BodyCounts::default(),
            });
        }
        return Some(PlacedSpawn::Missing);
    };
    let share = item.light_share(object.flags);
    let scaled;
    let model = if share == 1.0 {
        model.as_ref()
    } else {
        let mut m = with_light_share(&model, share);
        dim_child_glows(&mut m, share, loader.materials);
        scaled = m;
        &scaled
    };
    let with_overrides;
    let placement = transform_from_rows(&object.transform);
    let model = match with_placement_transforms(model, &object.attachments, placement) {
        Some(m) => {
            with_overrides = m;
            &with_overrides
        }
        None => model,
    };
    let (e, nodes) = spawn_model_nodes(commands, model, Some(placement));
    mark_cloth_starts(commands, model, &nodes, &object.attachments);
    commands.entity(e).insert((
        Name::new(item.name.clone()),
        room_view::PendingRoom::default(),
        PlacedItem::of(item, object),
        markers::PlacedTemplate(item.parent),
        ChildOf(parent),
    ));
    if let Some(sections) = ObjectSections::of(item, object) {
        commands.entity(e).insert(sections);
    }
    // Its door and operative records, for the signals and mechanisms.
    if let Some(records) = mechanisms::MechanismRecords::of(object.id, item) {
        commands.entity(e).insert(records);
    }
    if let Some(operative) = item.operative.filter(|o| {
        o.class == exa_formats::objdb::operative::DEVICE
            && (o.kind == exa_formats::objdb::operative::TELEPORTER
                || o.kind == exa_formats::objdb::operative::RES_DEVICE)
    }) {
        commands.entity(e).insert(TeleportDevice {
            kind: operative.kind,
            operative,
        });
    }
    // A hinged door's leaves, a lever's arm and a push surface register with
    // their rules (a sliding door's leaf too when asked for); any other
    // mechanism body stays static. The
    // placement's static bits (`PlacedObject::is_static`) mark only the
    // object's root node in the original, which, unlike the prop reader,
    // does not spread the flag to the nodes under it; what that root flag
    // does to the model's bodies is not traced, so it is not applied.
    let placement = bodies::placement_of(item, object, mechanism_bodies);
    let geometry = if object.is_static() {
        bodies::LevelGeometry::Root
    } else {
        bodies::LevelGeometry::None
    };
    bodies::mark_moving_meshes(commands, model, &nodes, geometry);
    Some(PlacedSpawn::Placed {
        entity: e,
        lights: model.nodes.iter().filter(|n| n.light.is_some()).count(),
        bodies: bodies::register_bodies(commands, model, &nodes, placement),
    })
}

/// The content stream's level-object builder over a level's placed
/// objects: each entry is spawned as [`spawn_placed_object`] does and
/// reported with what the stream reads off the original's built node (its
/// operative kind, position and transition-marker words).
struct PlacedObjectBuilder<'a, 'w, 's, 'l> {
    commands: &'a mut Commands<'w, 's>,
    loader: &'a mut ModelLoader<'l>,
    scene: &'a Scene,
    items: &'a [Item],
    root: Entity,
    mechanism_bodies: bodies::MechanismBodies,
    /// What the last entry's spawn came to.
    spawned: Option<PlacedSpawn>,
}

impl exa_assets::content_stream::ContentBuilders for PlacedObjectBuilder<'_, '_, '_, '_> {
    fn level_object(&mut self, index: usize) -> Option<exa_assets::content_stream::LevelNode> {
        let object = self.scene.placed_objects.get(index)?;
        self.spawned = spawn_placed_object(
            self.commands,
            self.loader,
            self.scene,
            self.items,
            self.root,
            object,
            self.mechanism_bodies,
        );
        let entity = match self.spawned {
            // The database builder found no model: the original's missing
            // count.
            Some(PlacedSpawn::Missing) => return None,
            Some(PlacedSpawn::Placed { entity, .. }) => entity,
            // Exarust spawns nothing for a record without a model (triggers,
            // markers) or of another slot; the original builds a node for
            // any record it finds. Guess: a record of another slot counts
            // as built, as the builder is not seen to filter by slot.
            None => Entity::PLACEHOLDER,
        };
        let items: &[Item] = if object.is_local() {
            &self.scene.local_items
        } else {
            self.items
        };
        let id = object.id & exa_formats::rfc::PlacedObject::ID_MASK;
        // A record the database does not hold builds nothing.
        let item = items.iter().find(|item| item.id == id)?;
        let t = object.transform[3];
        // The class is the kind word the database builder left on the node
        // (a door before an operative on a model tree; the volume kind on a
        // model-less volume record), which the level builder dispatches on.
        let class = db_objects::node_kind(item);
        Some(exa_assets::content_stream::LevelNode {
            node: entity.to_bits(),
            class,
            operative_kind: item.operative.map(|o| u32::from(o.kind)),
            // The original's units: centimetres.
            position: [t[0] * 100.0, t[1] * 100.0, t[2] * 100.0],
            marker: item.volume.and_then(|v| v.marker()),
        })
    }

    fn object(&mut self, _: &exa_assets::content_stream::ObjectRequest) -> Option<u64> {
        None
    }

    fn character(&mut self, _: &exa_assets::content_stream::CharacterRequest) -> Option<u64> {
        None
    }

    fn streamed_character(
        &mut self,
        _: &exa_assets::content_stream::StreamedCharacter,
    ) -> Option<u64> {
        None
    }
}

/// `model` with the voxel lights hanging directly under its root scaled by
/// an item's condition share ([`Item::light_share`]), as the original dims
/// a salve by its charge.
fn with_light_share(model: &Model, share: f32) -> Model {
    let mut model = model.clone();
    let root = model.roots().next();
    for node in &mut model.nodes {
        if node.parent.is_some()
            && node.parent == root
            && let Some(light) = &mut node.voxel_light
        {
            light.intensity *= share;
        }
    }
    model
}

/// Scales the emissive colour of the meshes hanging directly under
/// `model`'s root by an item's condition share, as the original scales a
/// salve's glow mesh with its voxel light: the parts get copies of their
/// materials, so the shared ones stay as they are.
fn dim_child_glows(
    model: &mut Model,
    share: f32,
    materials: &mut Assets<exa_render::material::ExaMaterial>,
) {
    let root = model.roots().next();
    for node in &mut model.nodes {
        if node.parent.is_none() || node.parent != root {
            continue;
        }
        for (_, handle) in &mut node.parts {
            let Some(material) = materials.get(handle.id()) else {
                continue;
            };
            let e = material.params.emissive;
            if e.truncate() == Vec3::ZERO {
                continue;
            }
            let mut dimmed = material.clone();
            dimmed.params.emissive = (e.truncate() * share).extend(e.w);
            *handle = materials.add(dimmed);
        }
    }
}

/// Each of a placement's node records with the model node it applies to,
/// in the order the original applies them: a record looks its node up from
/// the node it applies to (the model's root at the top level), as that node
/// itself for an empty name, else as its first direct child of exactly that
/// name; a transform record's nested records apply to its node's children.
/// A record whose node is not there is skipped with its nested records.
fn resolved_attachments<'a>(
    model: &Model,
    attachments: &'a [exa_formats::level::PropAttachment],
) -> Vec<(usize, &'a exa_formats::level::PropAttachment)> {
    resolved_records(model, attachments)
        .into_iter()
        .map(|(node, a, _)| (node, a))
        .collect()
}

/// [`resolved_attachments`], each with whether it came from a placement
/// tree (`0x3DE0EC00`) rather than a node record (`0x3DE0ECAC`,
/// `0x3DE0ECDB`).
fn resolved_records<'a>(
    model: &Model,
    attachments: &'a [exa_formats::level::PropAttachment],
) -> Vec<(usize, &'a exa_formats::level::PropAttachment, bool)> {
    use exa_formats::level::PropAttachment;
    type Resolved<'a> = Vec<(usize, &'a PropAttachment, bool)>;
    fn walk<'a>(model: &Model, at: usize, list: &'a [PropAttachment], out: &mut Resolved<'a>) {
        for a in list {
            if let PropAttachment::Tree { entries } = a {
                walk_tree(model, at, entries, out);
                continue;
            }
            let target = if a.name().is_empty() {
                Some(at)
            } else {
                model
                    .nodes
                    .iter()
                    .position(|n| n.parent == Some(at) && n.name == a.name())
            };
            let Some(target) = target else { continue };
            out.push((target, a, false));
            if let PropAttachment::Transform { children, .. } = a {
                walk(model, target, children, out);
            }
        }
    }
    /// A `0x3DE0EC00` tree: every entry, its own children included, names
    /// the first direct child of exactly that name (an empty name included;
    /// the original compares the two 16-byte names), never the node itself.
    fn walk_tree<'a>(
        model: &Model,
        at: usize,
        entries: &'a [PropAttachment],
        out: &mut Resolved<'a>,
    ) {
        for entry in entries {
            let target = model
                .nodes
                .iter()
                .position(|n| n.parent == Some(at) && n.name == entry.name());
            let Some(target) = target else { continue };
            out.push((target, entry, true));
            if let PropAttachment::Transform { children, .. } = entry {
                walk_tree(model, target, children, out);
            }
        }
    }
    let mut out = Vec::new();
    if let Some(root) = model.roots().next() {
        walk(model, root, attachments, &mut out);
    }
    out
}

/// The model a placement draws before its transform records: `entry`'s,
/// rebuilt from its file when the placement replaces a mesh's positions
/// (`0x3DE0ECDB`), as the original replaces the position-stream elements of
/// its copy of the object before it builds the render meshes (only when the
/// count is the stream's). The record's positions are where the level saved
/// the mesh (a banner's settled cloth), in the level's frame: every shipped
/// record's box sits about its placement, so they are taken into the
/// node's own frame at `placement`. The normals follow the new positions.
/// A rebuilt model is cached per distinct set of positions.
fn placement_base(
    loader: &mut ModelLoader<'_>,
    pack: PackKind,
    entry: &str,
    attachments: &[exa_formats::level::PropAttachment],
    placement: Transform,
) -> Option<std::sync::Arc<Model>> {
    use exa_formats::level::PropAttachment;
    use std::hash::{Hash, Hasher};
    let model = loader.model(pack, entry)?;
    let placed = with_placement_transforms(&model, attachments, placement);
    let frames: &Model = placed.as_ref().unwrap_or(&model);
    let edits: Vec<(usize, Vec<[f32; 3]>)> = resolved_attachments(&model, attachments)
        .into_iter()
        .filter_map(|(node, a)| match a {
            PropAttachment::MeshPositions { positions, .. } => {
                Some((node, level_to_node(frames, node, placement, positions)))
            }
            PropAttachment::Transform { .. } | PropAttachment::Tree { .. } => None,
        })
        .collect();
    if edits.is_empty() {
        return Some(model);
    }
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    for (node, positions) in &edits {
        (node, positions.len()).hash(&mut hasher);
        for v in positions.iter().flatten() {
            v.to_bits().hash(&mut hasher);
        }
    }
    let variant = format!("positions-{:016x}", hasher.finish());
    loader.edited_model(pack, entry, &variant, |scene| {
        for (node, positions) in &edits {
            let Some(exa_formats::rfc::NodeBody::Mesh(mesh)) =
                scene.nodes.get_mut(*node).map(|n| &mut n.body)
            else {
                continue;
            };
            if mesh.position_count != positions.len() {
                continue;
            }
            for (p, &id) in mesh.positions.iter_mut().zip(&mesh.position_ids) {
                if let Some(&q) = positions.get(id as usize) {
                    *p = q;
                }
            }
            mesh.recompute_normals();
        }
    })
}

/// Positions in the level's frame (metres) taken into node `node`'s own
/// frame, with `model` spawned at `placement`.
fn level_to_node(
    model: &Model,
    node: usize,
    placement: Transform,
    positions: &[[f32; 3]],
) -> Vec<[f32; 3]> {
    let to_node = node_world(model, node, placement).inverse();
    positions
        .iter()
        .map(|p| to_node.transform_point3(Vec3::from_array(*p)).to_array())
        .collect()
}

/// A node's frame in the level when `model` is spawned at `placement`
/// ([`spawn_model_nodes`]'s rule: the placement stands for the first root,
/// another root sits relative to it).
fn node_world(model: &Model, node: usize, placement: Transform) -> Mat4 {
    let first_root = model.roots().next();
    let first_inverse = first_root
        .map(|r| model.nodes[r].transform.to_matrix().inverse())
        .unwrap_or(Mat4::IDENTITY);
    let mut world = Mat4::IDENTITY;
    let mut at = Some(node);
    // Bounded by the node count, so a cycle in bad input ends.
    for _ in 0..=model.nodes.len() {
        let Some(i) = at else { break };
        let Some(n) = model.nodes.get(i) else { break };
        let local = match n.parent {
            None if Some(i) == first_root => Mat4::IDENTITY,
            None => first_inverse * n.transform.to_matrix(),
            Some(_) => n.transform.to_matrix(),
        };
        world = local * world;
        at = n.parent;
    }
    placement.to_matrix() * world
}

/// Each placement record that names a cloth's mesh (`0x3DE0ECDB`) also sets
/// that cloth's sleep countdown from its word, as the original's placement
/// load does (0 in every shipped record: a placed cloth starts asleep and
/// wakes only for a moving body); the word goes on the node's entity for
/// the cloth builder.
fn mark_cloth_starts(
    commands: &mut Commands,
    model: &Model,
    nodes: &[Entity],
    attachments: &[exa_formats::level::PropAttachment],
) {
    use exa_formats::level::PropAttachment;
    for (node, a) in resolved_attachments(model, attachments) {
        if let PropAttachment::MeshPositions { word, .. } = a
            && model.nodes[node].cloth
            && let Some(&entity) = nodes.get(node)
        {
            commands
                .entity(entity)
                .insert(cloth::ClothStartSleep(*word as i32));
        }
    }
}

/// A copy of `model` with the placement's transform records (`0x3DE0ECAC`)
/// applied at `placement`, or `None` when none applies, so the model is
/// shared as it is.
fn with_placement_transforms(
    model: &Model,
    attachments: &[exa_formats::level::PropAttachment],
    placement: Transform,
) -> Option<Model> {
    use exa_formats::level::PropAttachment;
    let resolved = resolved_attachments(model, attachments);
    if !resolved
        .iter()
        .any(|(_, a)| matches!(a, PropAttachment::Transform { .. }))
    {
        return None;
    }
    let mut out = model.clone();
    apply_transform_attachments(&mut out, attachments, placement);
    Some(out)
}

/// The transform records of a placement, applied to the model's nodes in
/// the order they are resolved ([`resolved_attachments`]).
///
/// A node record (`0x3DE0ECAC`) holds the node's frame in the level, as the
/// level saved it: the original's placement loader copies it over the
/// node's world matrix and derives the local rotation from the parent's
/// world matrix (the parent's already placed: the object's root at the
/// placement, or the node of the record it is nested in). Every exanima01
/// record matches the placement composed with the model's own local
/// transform to within 1 cm (sacks, buckets, carts, wall rings, chains,
/// mattresses). Here the node's local transform becomes that frame seen
/// from its parent's frame at `placement`, so its frame in the level is the
/// record's. A record on a root node (none shipped) keeps the placement,
/// which stands for the root. A placement tree's entries (`0x3DE0EC00`,
/// none shipped) replace their nodes' local transforms as they are.
fn apply_transform_attachments(
    model: &mut Model,
    attachments: &[exa_formats::level::PropAttachment],
    placement: Transform,
) {
    use exa_formats::level::PropAttachment;
    for (node, a, from_tree) in resolved_records(model, attachments) {
        let PropAttachment::Transform { transform, .. } = a else {
            continue;
        };
        let record = transform_from_rows(transform);
        if from_tree {
            model.nodes[node].transform = record;
            continue;
        }
        let Some(parent) = model.nodes[node].parent else {
            continue;
        };
        let local = node_world(model, parent, placement).inverse() * record.to_matrix();
        model.nodes[node].transform = Transform::from_matrix(local);
    }
}

/// Spawns `model` under a new entity. With a placement, the placement takes
/// the place of the model's root transform, the way a level places an object;
/// without one the stored node transforms are used.
pub fn spawn_model(commands: &mut Commands, model: &Model, placement: Option<Transform>) -> Entity {
    spawn_model_nodes(commands, model, placement).0
}

/// [`spawn_model`], also returning each node's entity (in the model's node
/// order).
pub fn spawn_model_nodes(
    commands: &mut Commands,
    model: &Model,
    placement: Option<Transform>,
) -> (Entity, Vec<Entity>) {
    let root = commands
        .spawn((placement.unwrap_or_default(), Visibility::default()))
        .id();
    let first_root = model.roots().next();
    let first_inverse = first_root
        .map(|r| model.nodes[r].transform.to_matrix().inverse())
        .unwrap_or(Mat4::IDENTITY);
    let mut entities = Vec::with_capacity(model.nodes.len());
    let mut parts_of: Vec<Vec<Entity>> = Vec::with_capacity(model.nodes.len());
    for (i, node) in model.nodes.iter().enumerate() {
        let local = match (placement, node.parent) {
            (Some(_), None) if Some(i) == first_root => Transform::IDENTITY,
            (Some(_), None) => Transform::from_matrix(first_inverse * node.transform.to_matrix()),
            _ => node.transform,
        };
        let e = commands
            .spawn((
                Name::new(node.name.clone()),
                // A pick of a static mesh under this node returns the node,
                // whose name the click hooks compare; generated level
                // geometry carries no marker and stays the level.
                exa_physics::StaticNode,
                local,
                Visibility::default(),
                PartBounds {
                    centre: node.bounds.0,
                    radius: node.bounds.1,
                    half_extents: node.half_extents,
                },
            ))
            .id();
        // A model's node has no object record, so a `[` name puts it in
        // the item cursor's placement markers, as the original's sector
        // attach does.
        if let Some(marker) = markers::model_node_marker(&node.name) {
            commands.entity(e).insert(marker);
        }
        let mut parts = Vec::with_capacity(node.parts.len());
        if !node.placeholder {
            for (part_index, (mesh, material)) in node.parts.iter().enumerate() {
                let part = commands
                    .spawn((MeshMaterial3d(material.clone()), ChildOf(e)))
                    .id();
                match node.skin_bone {
                    Some(bone) => commands.entity(part).insert(exa_assets::SkinnedPart {
                        bone,
                        mesh: mesh.clone(),
                    }),
                    None => commands.entity(part).insert(Mesh3d(mesh.clone())),
                };
                if node.is_skin {
                    commands.entity(part).insert(exa_assets::RaceSkinPart);
                }
                if node.cloth {
                    commands.entity(part).insert(cloth::ClothNode);
                    if let Some(source) = &node.cloth_mesh {
                        commands.entity(part).insert(cloth::ClothSource {
                            mesh: source.clone(),
                            part: part_index,
                        });
                    }
                }
                // The level the mesh ships with. Builders that set their own
                // (grid sets, floors) replace it.
                if node.shadow_bias != 0 {
                    commands
                        .entity(part)
                        .insert(exa_render::lights::ShadowBias(node.shadow_bias));
                }
                parts.push(part);
            }
            // `_null` geometry never draws, but the cursor's pick meets it:
            // a pick-only static collider under this node, in no draw.
            for mesh in &node.collision {
                commands.spawn((
                    Mesh3d(mesh.clone()),
                    exa_physics::PickOnly,
                    Visibility::Hidden,
                    ChildOf(e),
                ));
            }
        }
        if !parts.is_empty() {
            commands
                .entity(e)
                .insert(node_parts::NodeParts::new(parts.clone()));
        }
        parts_of.push(parts);
        if node.placeholder {
            entities.push(e);
            continue;
        }
        if let Some(light) = &node.light {
            commands
                .entity(e)
                .insert((light_bundle(light), light_source::LightSource::of(light)));
        }
        if let Some(light) = &node.voxel_light {
            commands
                .entity(e)
                .insert(exa_render::gi::VoxelLight::from_record(
                    light.color,
                    light.intensity,
                    light.radius,
                ));
        }
        if let Some(size) = node.fire {
            fire::spawn_fire(commands, e, size);
        }
        if let Some(size) = node.portal {
            commands.entity(e).insert(PortalMarker { size });
        }
        if !node.gi_points.is_empty() {
            commands
                .entity(e)
                .insert(gi_volume::GiPoints(node.gi_points.clone()));
            // The node's centred GI variant (flags2 0x10): its points gain
            // half a voxel in height and leave their column's lowest slice
            // alone, as the original's second splat path does.
            if node.gi_centred {
                commands.entity(e).insert(gi_volume::GiCentredMesh);
            }
            // A thin mesh (its first surface's material) fills the
            // occupancy at a quarter weight.
            if node.gi_thin {
                commands.entity(e).insert(gi_volume::GiThinMesh);
            }
        }
        if !node.tracks.is_empty() {
            commands
                .entity(e)
                .insert(exa_anim::tracks::NodeTracks(node.tracks.clone()));
        }
        entities.push(e);
    }
    for (i, node) in model.nodes.iter().enumerate() {
        let parent = node.parent.map_or(root, |p| entities[p]);
        commands.entity(entities[i]).insert(ChildOf(parent));
    }
    // A fire marker suppresses its host mesh's own emissive: remember the
    // host's first part so the fire can drive it.
    for (i, node) in model.nodes.iter().enumerate() {
        if node.fire.is_none() {
            continue;
        }
        let Some(parent) = node.parent else { continue };
        if let Some(&part) = parts_of[parent].first() {
            commands
                .entity(entities[i])
                .insert(fire::FireHost { part, base: None });
        }
    }
    let mut excluded: Vec<Vec<Entity>> = vec![Vec::new(); model.nodes.len()];
    for (node, anchor) in self_shadowless(model) {
        excluded[node].push(entities[anchor]);
    }
    for (node, anchors) in excluded.into_iter().enumerate() {
        if anchors.is_empty() {
            continue;
        }
        for &part in &parts_of[node] {
            commands
                .entity(part)
                .insert(shadow_links::ShadowExclude(anchors.clone()));
        }
    }
    (root, entities)
}

/// Nodes whose meshes a light of the same model leaves out of its shadow
/// map, as the original does: the light's siblings, its parent and its
/// grandparent (a fire's light hangs under the fire node, so a torch does
/// not shadow its own flame). A light at the model's root leaves out
/// nothing. Each node comes with the light's parent node, which identifies
/// the light ([`shadow_links::ShadowExclude`]).
fn self_shadowless(model: &Model) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    for (i, node) in model.nodes.iter().enumerate() {
        // The light's parent: the fire node for fires, the light node's
        // parent for lights.
        let parent = if node.fire.is_some() {
            Some(i)
        } else if node.light.is_some() {
            node.parent
        } else {
            continue;
        };
        let Some(parent) = parent else { continue };
        let siblings = (0..model.nodes.len()).filter(|&j| model.nodes[j].parent == Some(parent));
        out.extend(
            siblings
                .filter(|&j| j != i || node.fire.is_some())
                .map(|j| (j, parent)),
        );
        out.push((parent, parent));
        out.extend(model.nodes[parent].parent.map(|g| (g, parent)));
    }
    out.sort_unstable();
    out.dedup();
    out
}

/// A level portal's marker node (the `0x3DD0E004` emitter record) with its
/// stored size. The game replaces it with the portal's sparks and light
/// (`exa_game::effects::portal`).
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct PortalMarker {
    pub size: f32,
}

/// Approximation of the original omni light in Bevy units.
/// Lumens giving the shader a colour of 2/π per unit of intensity (Bevy
/// divides lumens by 4π).
pub const LUMENS_PER_INTENSITY: f32 = 8.0;

/// A light's shadow filter radius (radians around the light direction) for
/// its softness, at the original's top shading quality.
pub fn shadow_radius(softness: f32) -> f32 {
    (softness + 1.0) * 0.005
}

/// A light's vertical shaping (y, z) from its spot values q, p: the shader
/// scales the light by max((2^(z·(Ly + (1 − Ly)·y)) − 1)/(2^z − 1), 0), with
/// Ly the vertical part of the direction to the light (1 straight below it).
/// Lights without a spot get (1, 1), a factor of 1.
pub fn vertical_shaping(light: &Light) -> (f32, f32) {
    let Some([q, p]) = light.spot else {
        return (1.0, 1.0);
    };
    let half_pi = std::f32::consts::FRAC_PI_2;
    let y = if q >= 0.0 {
        0.5 * (q * half_pi).sin()
    } else {
        3.0 - 3.0 / (q * half_pi).cos()
    };
    (y, p + 1.0)
}

/// How far an unranged light reaches, metres (it does not fall off).
const UNRANGED: f32 = 500.0;

/// A level light as the original shades it, with its share of the GI
/// injection: colour linearised and scaled by the intensity (the multiplier
/// scales only the GI), falling to nothing at 1.25 radii when ranged, shaped
/// vertically when it has a spot, casting shadows softened by its softness.
pub fn light_bundle(
    light: &Light,
) -> (
    PointLight,
    exa_render::gi::GiMultiplier,
    exa_render::gi::GiRadius,
    shadow_links::ShadowReach,
    exa_render::lights::LightShaping,
    exa_render::gi::GiShadowBias,
    exa_render::gi::GiRanged,
) {
    /// The falloff reaches zero this far out, in radii.
    const REACH: f32 = 1.25;
    let c = exa_render::material::linear_rgb(light.color);
    let (y, z) = vertical_shaping(light);
    let point = PointLight {
        color: Color::linear_rgb(c.x, c.y, c.z),
        intensity: light.intensity.max(0.0) * LUMENS_PER_INTENSITY,
        range: if light.ranged {
            (light.radius * REACH).max(0.5)
        } else {
            UNRANGED
        },
        // The shadow filter radius (read by the shader).
        radius: shadow_radius(light.softness),
        // Static casters shadow it through the cached cubes; Bevy's map is
        // turned on only while a skinned mesh is linked to it.
        shadow_maps_enabled: false,
        shadow_map_near_z: 0.03,
        ..default()
    };
    (
        point,
        exa_render::gi::GiMultiplier(light.multiplier),
        exa_render::gi::GiRadius(light.radius),
        shadow_links::ShadowReach {
            ranged: light.ranged,
            shaped: light.spot.map(|_| y),
            ..shadow_links::ShadowReach::new(c, light.intensity, light.radius)
        },
        exa_render::lights::LightShaping(Vec2::new(y, z)),
        // The GI's cube lookup scales the receiver's distance by
        // bias + 0.94 over the radius; the bias is the shadow detail plus
        // the light's own offset, and an unranged light's far plane is
        // 3.4e38 (see `GiRanged`).
        exa_render::gi::GiShadowBias(exa_render::gi::gi_shadow_bias(
            light_source::shadow_quality_offset(light.flags),
        )),
        exa_render::gi::GiRanged(light.ranged),
    )
}

/// Spawns a level from `Resource.rpk` (`name` with or without `.rfc`): its own
/// scene nodes (meshes and lights) and every prop from the object library.
/// All of it at once; [`LevelLoad`] spreads the same work over frames.
pub fn spawn_level(
    commands: &mut Commands,
    loader: &mut ModelLoader<'_>,
    name: &str,
) -> Option<(Entity, LevelInfo)> {
    let mut load = LevelLoad::begin(commands, loader, name)?;
    loop {
        if let LoadProgress::Finished(root, info) =
            load.step(commands, loader, std::time::Duration::MAX)
        {
            return Some((root, *info));
        }
    }
}

/// What [`LevelLoad::step`] reached.
pub enum LoadProgress {
    /// More steps to go: what is being built and how far the load is (0–1).
    Loading { stage: &'static str, fraction: f32 },
    /// The level is spawned.
    Finished(Entity, Box<LevelInfo>),
}

/// The parts of a level load, in order.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Stage {
    Grid,
    Terrain,
    OwnModel,
    Props,
    PlacedObjects,
    Lighting,
    Done,
}

/// A level spawned a step at a time, so a caller can keep the window
/// answering (and a loading screen drawn) while it builds. Each step does
/// as much of the current part as fits in the step's time budget (at least
/// one piece of it): the grid's sets and rows of cells, the terrain's tiles
/// and their textures, the level's own model, props, placed objects, and
/// last the lighting.
pub struct LevelLoad {
    entry: String,
    /// The level's id in `locales.rdb`, when it has one.
    locale: Option<u32>,
    scene: Scene,
    root: Entity,
    stage: Stage,
    grid: Option<grid::GridBuild>,
    grid_stats: Option<grid::GridStats>,
    terrain: Option<terrain::TerrainBuild>,
    terrain_stats: Option<terrain::TerrainStats>,
    /// The terrain's tiles on the X/Z plane, metres: low and high corners.
    terrain_extent: Option<(Vec2, Vec2)>,
    lights: usize,
    min: Vec3,
    max: Vec3,
    missing: usize,
    prop_entities: Vec<Entity>,
    /// The game's object database, read for the placed objects.
    global_items: Vec<Item>,
    /// Placed objects looked at, spawned and missing their model.
    objects_done: usize,
    placed_objects: usize,
    missing_objects: usize,
    bodies: bodies::BodyCounts,
    /// Which mechanisms become bodies with their rules (hinged doors,
    /// levers and push surfaces by default).
    pub mechanism_bodies: bodies::MechanismBodies,
    /// The session's arrival word and the id of the level it came from,
    /// which name the placed player ([`exa_assets::content_stream`]); by
    /// default nothing set them.
    pub arrival: u32,
    pub previous_level: u32,
    /// The content stream the placed objects are built through, while the
    /// level's list is unfinished (behind a lock only so the load can live
    /// in a resource: the stream's job ring holds callbacks that are not
    /// `Sync`; the load reaches it through `get_mut`, never locking).
    stream: Option<std::sync::Mutex<exa_assets::content_stream::ContentStream>>,
    /// The stream's count of placed objects whose record or model was not
    /// found, and the placed player's position (metres) once built.
    missing_database_objects: usize,
    player_focus: Option<Vec3>,
}

impl LevelLoad {
    /// Parses the level and spawns its root. `None` when the level is not
    /// in `Resource.rpk` or does not parse.
    pub fn begin(
        commands: &mut Commands,
        loader: &mut ModelLoader<'_>,
        name: &str,
    ) -> Option<Self> {
        let entry = if name.ends_with(".rfc") {
            name.to_string()
        } else {
            format!("{name}.rfc")
        };
        let bytes = loader.game.pack(PackKind::Resource)?.get(&entry)?;
        let scene = match Scene::parse(bytes) {
            Ok(s) => s,
            Err(e) => {
                loader.warn_once(format!("level {entry}: {e}"));
                return None;
            }
        };
        for err in &scene.level_errors {
            loader.warn_once(format!("level {entry}: {err}"));
        }
        // A previous level's terrain stream goes with its entities, and its
        // composite sets go back to the renderer's host.
        commands.remove_resource::<terrain::live::TerrainStreaming>();
        commands.insert_resource(exa_render::terrain_composite::TerrainCompositeHost::default());
        commands.queue(|world: &mut World| {
            if let Some(mut queue) =
                world.get_resource_mut::<exa_render::terrain_composite::TerrainBakeQueue>()
            {
                queue.clear();
            }
            if let Some(mut queue) =
                world.get_resource_mut::<exa_render::terrain_composite::TerrainCopyQueue>()
            {
                queue.clear();
            }
        });
        commands.insert_resource(LevelLocalItems(scene.local_items.clone()));
        commands.insert_resource(LevelLocalRecords {
            records: scene.local_records.clone(),
            strings: scene.local_strings.clone(),
        });
        let root = commands
            .spawn((
                LevelRoot {
                    name: entry.clone(),
                },
                level_lifecycle::LevelEntity,
                Transform::default(),
                Visibility::default(),
            ))
            .id();
        let props = scene.props.len();
        let locale = loader
            .game
            .pack(PackKind::Resource)
            .and_then(|p| p.get("locales.rdb"))
            .and_then(|b| exa_formats::roles::locale_id(b, &entry));
        Some(Self {
            entry,
            locale,
            scene,
            root,
            stage: Stage::Grid,
            grid: None,
            grid_stats: None,
            terrain: None,
            terrain_stats: None,
            terrain_extent: None,
            lights: 0,
            min: Vec3::splat(f32::MAX),
            max: Vec3::splat(f32::MIN),
            missing: 0,
            prop_entities: Vec::with_capacity(props),
            global_items: Vec::new(),
            objects_done: 0,
            placed_objects: 0,
            missing_objects: 0,
            bodies: bodies::BodyCounts::default(),
            mechanism_bodies: bodies::MechanismBodies::default(),
            arrival: exa_assets::content_stream::NO_ARRIVAL_WORD,
            previous_level: exa_assets::content_stream::NO_PREVIOUS_LEVEL,
            stream: None,
            missing_database_objects: 0,
            player_focus: None,
        })
    }

    /// The level's root entity (spawned by [`LevelLoad::begin`]).
    pub fn root(&self) -> Entity {
        self.root
    }

    /// Does the next part of the load, until `budget` has passed (at least
    /// one piece of work per step).
    pub fn step(
        &mut self,
        commands: &mut Commands,
        loader: &mut ModelLoader<'_>,
        budget: std::time::Duration,
    ) -> LoadProgress {
        let started = std::time::Instant::now();
        match self.stage {
            Stage::Grid => {
                let done = match &self.scene.grid {
                    Some(g) => {
                        let build = self.grid.get_or_insert_with(|| {
                            // The rooms first: the grid gives each its
                            // shells, and the rest of the level is sorted
                            // into them.
                            let rooms = std::sync::Arc::new(rooms::Rooms::flood(g));
                            commands.insert_resource(room_view::LevelRooms::new(rooms.clone()));
                            grid::GridBuild::new(self.root, Some(rooms))
                        });
                        build.step(commands, loader, g, budget)
                    }
                    None => {
                        commands.remove_resource::<room_view::LevelRooms>();
                        true
                    }
                };
                if done {
                    let rooms = self.grid.as_ref().and_then(|b| b.rooms());
                    self.grid_stats = self.grid.take().map(|b| b.stats());
                    // Lights stop at the grid's walls.
                    match self
                        .scene
                        .grid
                        .as_ref()
                        .and_then(shadow_links::LevelWalls::new)
                    {
                        Some(walls) => commands.insert_resource(walls),
                        None => commands.remove_resource::<shadow_links::LevelWalls>(),
                    }
                    self.terrain = terrain::TerrainBuild::begin(
                        loader,
                        &self.entry,
                        self.root,
                        self.scene.grid.as_ref().zip(rooms),
                    );
                    self.terrain_extent = self
                        .terrain
                        .as_ref()
                        .and_then(terrain::TerrainBuild::horizontal_extent);
                    if self.terrain.is_none() {
                        commands.remove_resource::<terrain::ground::TerrainGround>();
                    }
                    self.stage = Stage::Terrain;
                }
            }
            Stage::Terrain => {
                let done = self
                    .terrain
                    .as_mut()
                    .is_none_or(|t| t.step(commands, loader, budget));
                if done {
                    self.terrain_stats = self.terrain.take().map(|t| t.stats().clone());
                    self.stage = Stage::OwnModel;
                }
            }
            Stage::OwnModel => {
                // Its materials first, over as many steps as they take.
                if let Some(deadline) = started.checked_add(budget)
                    && !loader.warm_materials(PackKind::Resource, &self.entry, deadline)
                {
                    return self.progress();
                }
                if let Some(own) = loader.model(PackKind::Resource, &self.entry) {
                    self.lights += own.nodes.iter().filter(|n| n.light.is_some()).count();
                    let (e, nodes) = spawn_model_nodes(commands, &own, None);
                    bodies::mark_moving_meshes(commands, &own, &nodes, bodies::LevelGeometry::None);
                    self.bodies.add(bodies::register_bodies(
                        commands,
                        &own,
                        &nodes,
                        bodies::Placement::Loose,
                    ));
                    // Each of its top nodes goes to its own room.
                    commands.entity(e).insert((
                        ChildOf(self.root),
                        room_view::PendingRoom {
                            split: true,
                            ..default()
                        },
                    ));
                }
                self.stage = Stage::Props;
            }
            Stage::Props => {
                while let Some(prop) = self.scene.props.get(self.prop_entities.len()) {
                    // The original searches the project's object libraries
                    // for the prop's entry; `Objlib` is the usual one.
                    let kind = loader.object_pack(&prop.object).unwrap_or(PackKind::Objlib);
                    // A prop whose materials do not warm up in time waits
                    // for the next step.
                    if let Some(deadline) = started.checked_add(budget)
                        && !loader.warm_materials(kind, &prop.object, deadline)
                    {
                        break;
                    }
                    let placement = transform_from_rows(&prop.transform);
                    self.min = self.min.min(placement.translation);
                    self.max = self.max.max(placement.translation);
                    let base =
                        placement_base(loader, kind, &prop.object, &prop.attachments, placement);
                    let e = match base {
                        Some(model) => {
                            self.lights += model.nodes.iter().filter(|n| n.light.is_some()).count();
                            let owned;
                            let model: &Model = match with_placement_transforms(
                                &model,
                                &prop.attachments,
                                placement,
                            ) {
                                Some(m) => {
                                    owned = m;
                                    &owned
                                }
                                None => &model,
                            };
                            let (e, nodes) = spawn_model_nodes(commands, model, Some(placement));
                            mark_cloth_starts(commands, model, &nodes, &prop.attachments);
                            // A prop placed static is level geometry.
                            let (placement, geometry) = if prop.is_static() {
                                (bodies::Placement::Static, bodies::LevelGeometry::All)
                            } else {
                                (bodies::Placement::Loose, bodies::LevelGeometry::None)
                            };
                            bodies::mark_moving_meshes(commands, model, &nodes, geometry);
                            self.bodies
                                .add(bodies::register_bodies(commands, model, &nodes, placement));
                            e
                        }
                        None => {
                            self.missing += 1;
                            commands.spawn((placement, Visibility::default())).id()
                        }
                    };
                    commands.entity(e).insert(Name::new(prop.object.clone()));
                    self.prop_entities.push(e);
                    if started.elapsed() >= budget {
                        break;
                    }
                }
                if self.prop_entities.len() == self.scene.props.len() {
                    self.link_props(commands);
                    self.global_items = loader
                        .game
                        .pack(PackKind::Resource)
                        .and_then(|p| p.get("objdb.rdb"))
                        .and_then(|b| Database::parse(b).ok())
                        .map(|db| objdb::items(&db))
                        .unwrap_or_default();
                    self.stage = Stage::PlacedObjects;
                }
            }
            Stage::PlacedObjects => {
                // The placed objects go through the content stream's level
                // pass: one entry a pass, in file order, the player named by
                // the session's two words, the missing ones counted.
                let total = self.scene.placed_objects.len();
                let stream = self
                    .stream
                    .get_or_insert_with(|| {
                        let mut stream = exa_assets::content_stream::ContentStream::new();
                        stream.begin_level(
                            self.root.to_bits(),
                            total,
                            self.arrival,
                            self.previous_level,
                        );
                        std::sync::Mutex::new(stream)
                    })
                    .get_mut()
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
                while let Some(object) = self.scene.placed_objects.get(self.objects_done) {
                    if let Some(deadline) = started.checked_add(budget)
                        && let Some((kind, model)) =
                            placed_object_model(loader, &self.scene, &self.global_items, object)
                        && !loader.warm_materials(kind, &model, deadline)
                    {
                        break;
                    }
                    let mut builder = PlacedObjectBuilder {
                        commands: &mut *commands,
                        loader: &mut *loader,
                        scene: &self.scene,
                        items: &self.global_items,
                        root: self.root,
                        mechanism_bodies: self.mechanism_bodies,
                        spawned: None,
                    };
                    stream.step(&mut builder);
                    match builder.spawned {
                        Some(PlacedSpawn::Placed { lights, bodies, .. }) => {
                            self.placed_objects += 1;
                            self.lights += lights;
                            self.bodies.add(bodies);
                        }
                        Some(PlacedSpawn::Missing) => self.missing_objects += 1,
                        None => {}
                    }
                    self.objects_done += 1;
                    if started.elapsed() >= budget {
                        break;
                    }
                }
                if self.objects_done == total {
                    let mut builder = PlacedObjectBuilder {
                        commands: &mut *commands,
                        loader: &mut *loader,
                        scene: &self.scene,
                        items: &self.global_items,
                        root: self.root,
                        mechanism_bodies: self.mechanism_bodies,
                        spawned: None,
                    };
                    // Every entry is done, so the wait builds nothing more:
                    // it reports the missing objects and clears the list.
                    if let Some(level) = stream.wait_for_level(&mut builder, || {}) {
                        if let Some(message) =
                            exa_assets::content_stream::missing_message(level.missing)
                        {
                            warn!("{}: {message}", exa_assets::content_stream::LOG_SOURCE);
                        }
                        self.missing_database_objects = level.missing;
                    }
                    self.player_focus = stream
                        .player()
                        .map(|_| Vec3::from_array(stream.focus()) / 100.0);
                    self.stream = None;
                    self.stage = Stage::Lighting;
                }
            }
            Stage::Lighting => return self.finish(commands),
            Stage::Done => {}
        }
        self.progress()
    }

    fn progress(&self) -> LoadProgress {
        LoadProgress::Loading {
            stage: self.stage(),
            fraction: self.fraction(),
        }
    }

    /// Parents each prop to the prop it names, else to the level root; a
    /// prop under the root goes to the room of its bounds' centre, a prop
    /// under a prop to its parent's.
    fn link_props(&self, commands: &mut Commands) {
        for (i, prop) in self.scene.props.iter().enumerate() {
            let parent = match prop.parent {
                Some(p) if p >= 1 && p <= self.prop_entities.len() && p - 1 != i => {
                    self.prop_entities[p - 1]
                }
                _ => self.root,
            };
            let mut e = commands.entity(self.prop_entities[i]);
            e.insert(ChildOf(parent));
            if parent == self.root {
                e.insert(room_view::PendingRoom::default());
            }
        }
    }

    /// The level's box and vertical limits for the sun map: the props'
    /// origins `min..max` widened by the terrain's tiles, and the
    /// environment record's floor and ceiling (whole centimetres in the
    /// record). Fitted only when the level has a terrain, as in the
    /// original; other levels cover the whole scene.
    fn sun_shadow_region(&self, min: Vec3, max: Vec3) -> sun_shadow::SunShadowRegion {
        let (mut min, mut max) = (min, max);
        if let Some((low, high)) = self.terrain_extent {
            min = min.min(Vec3::new(low.x, min.y, low.y));
            max = max.max(Vec3::new(high.x, max.y, high.y));
        }
        let scene = exa_render::sun_fit::SceneBox::from_bounds(min * 100.0, max * 100.0);
        let record = self
            .scene
            .environment
            .as_ref()
            .map(|e| ((e.floor * 100.0).round(), (e.ceiling * 100.0).round()));
        sun_shadow::SunShadowRegion::new(
            scene,
            exa_render::sun_fit::VerticalRange::of_level(&scene, record),
            self.terrain_stats.is_some(),
        )
    }

    /// Lighting and the GI request; the load's last step.
    fn finish(&mut self, commands: &mut Commands) -> LoadProgress {
        // A level load applies the scene preset its name selects (the save
        // unit's indoor and outdoor presets), drawing the outdoor day and
        // weather from the clock seed the original takes at scene creation.
        let engine_seed = environment::clock_seed();
        let scene_lighting = environment::level_lighting_scene(
            &self.entry,
            self.scene.environment.as_ref(),
            self.lights,
            None,
            Some(environment::PresetInput {
                seed: 0,
                engine_seed,
                now: 0,
            }),
        );
        environment::apply_lighting(commands, &scene_lighting.lighting, self.root);
        // The live scene environment keeps ticking the preset's weather and
        // republishes its frame; both of its streams start from the same
        // clock seed the load drew with.
        commands.insert_resource(environment::SceneEnvRuntime::new(
            scene_lighting.env,
            engine_seed,
        ));
        let (min, max) = if self.scene.props.is_empty() {
            (Vec3::splat(-10.0), Vec3::splat(10.0))
        } else {
            (self.min, self.max)
        };
        let env = self.scene.environment.clone().unwrap_or_default();
        let reverb_decay = env.reverb_decay;
        let grid = self
            .scene
            .grid
            .as_ref()
            .map(|g| gi_volume::GiGrid {
                width: g.width,
                height: g.height,
                edges: std::sync::Arc::new(g.edges.clone()),
                synthetic: false,
            })
            .unwrap_or_else(gi_volume::GiGrid::synthetic);
        commands.insert_resource(gi_volume::PendingGi {
            centre: (min + max) / 2.0,
            top: env.ceiling,
            bottom: env.floor,
            base_ambient: [env.fill[0], env.fill[1], env.fill[2]],
            grid,
            top_slice: env.flags & exa_formats::environment::GI_TOP_SLICE != 0,
        });
        let (width, depth) =
            nav::source::grid_size(self.scene.grid.as_ref().map(|g| (g.width, g.height)));
        commands.insert_resource(self.sun_shadow_region(min, max));
        commands.insert_resource(nav::PendingNav {
            root: self.root,
            width,
            depth,
            base: nav::source::grid_base(env.grid_top),
        });
        self.stage = Stage::Done;
        LoadProgress::Finished(
            self.root,
            Box::new(LevelInfo {
                props: self.scene.props.len(),
                missing_props: self.missing,
                lights: self.lights,
                grid: self.grid_stats,
                terrain: self.terrain_stats.clone(),
                lighting: scene_lighting.lighting.clone(),
                characters: placed_characters(&self.scene, &self.global_items, self.locale),
                placed_objects: self.placed_objects,
                missing_objects: self.missing_objects,
                missing_database_objects: self.missing_database_objects,
                player_focus: self.player_focus,
                bodies: self.bodies,
                spawn: self.scene.grid.as_ref().and_then(grid::spawn_point),
                player_markers: player_markers(&self.scene, &self.global_items),
                alarm_volumes: alarm_volumes(&self.scene, &self.global_items),
                reverb_decay,
                bounds_min: min,
                bounds_max: max,
            }),
        )
    }

    /// What the next step builds.
    pub fn stage(&self) -> &'static str {
        match self.stage {
            Stage::Grid => "walls and floors",
            Stage::Terrain => "terrain",
            Stage::OwnModel => "level model",
            Stage::Props => "props",
            Stage::PlacedObjects => "objects",
            Stage::Lighting => "lights",
            Stage::Done => "done",
        }
    }

    /// How far the load is, 0–1: the props weigh most, then the placed
    /// objects; the grid, terrain and level model a tenth each.
    fn fraction(&self) -> f32 {
        let share = |done: usize, total: usize| {
            if total == 0 {
                1.0
            } else {
                done as f32 / total as f32
            }
        };
        let (props, objects) = (
            share(self.prop_entities.len(), self.scene.props.len()),
            share(self.objects_done, self.scene.placed_objects.len()),
        );
        let past = |stage: Stage| self.stage as u8 > stage as u8;
        let part = |stage: Stage| if past(stage) { 0.1 } else { 0.0 };
        (part(Stage::Grid)
            + part(Stage::Terrain)
            + part(Stage::OwnModel)
            + props * 0.5
            + objects * 0.2)
            .min(1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::world::CommandQueue;
    use exa_assets::{GameData, ModelCache};
    use exa_formats::characters::{
        CharacterRecord, ItemRef, LOCAL_ITEM, LevelCharacters, Placement,
    };

    /// The skinned shadow maps that Bevy still renders for characters keep
    /// the original's 3 cm near plane, not Bevy's 10 cm default. At 10 cm the
    /// torch's close-range character shadow smeared into a floor blob.
    #[test]
    fn point_shadows_start_at_the_original_near_plane() {
        let light = exa_formats::rfc::Light {
            flags: 0,
            color: [1.0, 0.68, 0.43],
            multiplier: 1.0,
            intensity: 8.0,
            radius: 8.4,
            softness: 2.0,
            spot: None,
            ranged: true,
            animation: None,
        };
        let (point, ..) = light_bundle(&light);
        assert_eq!(point.shadow_map_near_z, 0.03);
    }

    /// The vertical shaping factor the original's shader takes for a
    /// surface under a light at drop `dy` and horizontal offset `dx`.
    fn shaping_at(light: &exa_formats::rfc::Light, dy: f32, dx: f32) -> f32 {
        let (y, z) = vertical_shaping(light);
        let ly = dy / (dy * dy + dx * dx).sqrt();
        ((2f32.powf(z * (ly + (1.0 - ly) * y)) - 1.0) / (2f32.powf(z) - 1.0)).max(0.0)
    }

    /// exanima01's start room is lit by two level lights over the spawn:
    /// the pool light, grey (204/255), intensity 60, radius 630 cm, GI
    /// multiplier 0.1, shaped downwards with q = 0 and p = 15, 4 m up; and a
    /// faint fill, white, intensity 0.1, radius 2450 cm, GI multiplier 0,
    /// 2.5 m up. The pool's shaping (0, 16) halves its light about 1.5 m out
    /// on the floor 3.9 m below and leaves under a tenth at 3 m: the cold
    /// white pool about 3 m wide the original shows around a player who
    /// holds no torch. Both fall off with distance (1.25 radii).
    #[test]
    fn the_start_room_pool_light() {
        let pool = exa_formats::rfc::Light {
            flags: 0,
            color: [0.8, 0.8, 0.8],
            multiplier: 0.1,
            intensity: 60.0,
            radius: 6.3,
            softness: 0.5,
            spot: Some([0.0, 15.0]),
            ranged: true,
            animation: None,
        };
        assert_eq!(vertical_shaping(&pool), (0.0, 16.0));
        assert_eq!(shaping_at(&pool, 3.88, 0.0), 1.0);
        let half = shaping_at(&pool, 3.88, 1.5);
        assert!((0.45..0.52).contains(&half), "{half}");
        assert!(shaping_at(&pool, 3.88, 3.0) < 0.1);
        let (point, gi, ..) = light_bundle(&pool);
        assert!((point.range - 7.875).abs() < 1e-5);
        assert_eq!(gi.0, 0.1);
        let fill = exa_formats::rfc::Light {
            color: [1.0, 1.0, 1.0],
            multiplier: 0.0,
            intensity: 0.1,
            radius: 24.5,
            spot: None,
            ..pool
        };
        assert_eq!(vertical_shaping(&fill), (1.0, 1.0));
        let (point, gi, ..) = light_bundle(&fill);
        assert!((point.range - 30.625).abs() < 1e-4);
        assert_eq!(gi.0, 0.0);
    }

    /// The installed exanima01 (with `EXANIMA_DIR`) holds the start room's
    /// two lights with the record values above.
    #[test]
    fn shipped_start_room_lights() {
        let Some(dir) = exa_assets::locate_install(None) else {
            return;
        };
        let game = GameData::open(&dir).unwrap();
        let resource = game.pack(exa_assets::PackKind::Resource).unwrap();
        let scene = exa_formats::rfc::Scene::parse(resource.get("exanima01.rfc").unwrap()).unwrap();
        let lights: Vec<&exa_formats::rfc::Light> = scene
            .nodes
            .iter()
            .filter_map(|n| match &n.body {
                exa_formats::rfc::NodeBody::Light(l) => Some(l),
                _ => None,
            })
            .collect();
        let pool = lights
            .iter()
            .find(|l| l.intensity == 60.0 && l.spot.is_some())
            .expect("the pool light");
        assert_eq!(pool.spot, Some([0.0, 15.0]));
        assert!((pool.radius - 6.3).abs() < 1e-4, "{}", pool.radius);
        assert!((pool.multiplier - 0.1).abs() < 1e-6);
        assert!(
            pool.color.iter().all(|c| (c - 0.8).abs() < 0.01),
            "{:?}",
            pool.color
        );
        assert!(pool.ranged);
        let fill = lights
            .iter()
            .find(|l| l.intensity == 0.1 && l.multiplier == 0.0 && (l.radius - 24.5).abs() < 1e-3)
            .expect("the fill light");
        assert_eq!(fill.color, [1.0, 1.0, 1.0]);
        assert!(fill.ranged && fill.spot.is_none());
        // The range rule's radius test: shipped lights with the flag and no
        // radius, over the levels and the object library.
        let mut flagged_without_radius = 0;
        let mut lights = 0;
        for kind in [
            exa_assets::PackKind::Resource,
            exa_assets::PackKind::Objects,
        ] {
            let Some(pack) = game.pack(kind) else {
                continue;
            };
            for entry in pack.entries() {
                if !entry.name.to_ascii_lowercase().ends_with(".rfc") {
                    continue;
                }
                let Some(Ok(scene)) = pack.get(&entry.name).map(exa_formats::rfc::Scene::parse)
                else {
                    continue;
                };
                for n in &scene.nodes {
                    if let exa_formats::rfc::NodeBody::Light(l) = &n.body {
                        lights += 1;
                        if n.flags2 & exa_formats::rfc::RANGED != 0 && !l.ranged {
                            flagged_without_radius += 1;
                        }
                    }
                }
            }
        }
        eprintln!(
            "shipped lights: {lights}, range flag without a radius: {flagged_without_radius}"
        );
    }

    /// A spawned record's placement is its own id in both id words, the
    /// drag's condition word in the flags (its third byte the placed
    /// condition), and an identity frame.
    #[test]
    fn a_dropped_record_takes_its_id_and_condition() {
        let object = dropped_object(0x4000_0007, 0x0012_3400);
        assert_eq!(object.id, 0x4000_0007);
        assert_eq!(object.flags, 0x0012_3400);
        assert_eq!(object.slot, 0);
        assert_eq!(object.transform[3], [0.0, 0.0, 0.0]);
        let record = item(7, "", "", 0);
        let placed = PlacedItem::of(&record, &object);
        assert_eq!(placed.id, 7);
        assert_eq!(placed.tagged_id, 0x4000_0007);
        assert_eq!(placed.condition, 0x12);
    }

    /// Every level's transition marker is an object whose volume section
    /// names an entrance and the level it leads back from; `exanima01`'s
    /// single marker leads back from `exanima02` (locale id 3) at entrance
    /// 0 (skipped without the game).
    #[test]
    fn shipped_player_markers_come_from_volume_sections() {
        let Some(dir) = exa_assets::locate_install(None) else {
            return;
        };
        let game = GameData::open(&dir).unwrap();
        let resource = game.pack(exa_assets::PackKind::Resource).unwrap();
        let scene = Scene::parse(resource.get("exanima01.rfc").unwrap()).unwrap();
        let db = exa_formats::rdb::Database::parse(resource.get("objdb.rdb").unwrap()).unwrap();
        let markers = player_markers(&scene, &exa_formats::objdb::items(&db));
        assert_eq!(markers.len(), 1);
        assert_eq!((markers[0].entrance, markers[0].previous_level), (0, 3));
        // The marker box sits in the first room at about
        // (-8.95, 0.49, -94.15) m, on the floor.
        assert!((markers[0].position.x + 8.948565).abs() < 1e-3);
        assert!((markers[0].position.y - 0.49198326).abs() < 1e-3);
        assert!((markers[0].position.z + 94.14694).abs() < 1e-3);
    }

    /// `exanima06` places the game's 14 alarm zones, each a 340 × 200 ×
    /// 300 cm box (350 wide for three) standing on the floor, passed by one
    /// of the key tokens 474–479; the one at (10.5, 1, 69) m is turned a
    /// quarter about y and passed by token 474 (skipped without the game).
    #[test]
    fn shipped_alarm_volumes_are_exanima06s() {
        let Some(dir) = exa_assets::locate_install(None) else {
            return;
        };
        let game = GameData::open(&dir).unwrap();
        let resource = game.pack(exa_assets::PackKind::Resource).unwrap();
        let db = exa_formats::rdb::Database::parse(resource.get("objdb.rdb").unwrap()).unwrap();
        let global = exa_formats::objdb::items(&db);
        let scene = Scene::parse(resource.get("exanima06.rfc").unwrap()).unwrap();
        let zones = alarm_volumes(&scene, &global);
        assert_eq!(zones.len(), 14);
        assert!(zones.iter().all(|z| (474..=479).contains(&z.pass)));
        assert!(
            zones
                .iter()
                .all(|z| (z.bounds.centre.y - 100.0).abs() < 1e-3)
        );
        assert!(zones.iter().all(|z| z.bounds.half_extents.y == 100.0));
        let gate = zones
            .iter()
            .find(|z| (z.bounds.centre - Vec3::new(1050.0, 100.0, 6900.0)).length() < 0.5)
            .expect("the zone at (10.5, 1, 69) m");
        assert_eq!(gate.pass, 474);
        assert!(
            zones.iter().all(|z| z.alarm_node),
            "every alarm item is named Alarm_"
        );
        assert_eq!(gate.bounds.half_extents, Vec3::new(170.0, 100.0, 150.0));
        assert!((gate.bounds.axes[0] - Vec3::Z).length() < 1e-5);
        assert!(
            (gate.radius - (170.0f32 * 170.0 + 150.0 * 150.0 + 100.0 * 100.0).sqrt()).abs() < 1e-3
        );
        // No other story level has one.
        let first = Scene::parse(resource.get("exanima01.rfc").unwrap()).unwrap();
        assert!(alarm_volumes(&first, &global).is_empty());
    }

    /// What spawning `name` produced: the level's info, the names spawned
    /// and the number of entities and assets. Built at once, or in steps of
    /// `budget`.
    fn spawn(game: &GameData, name: &str, budget: Option<std::time::Duration>) -> Vec<String> {
        let mut world = World::new();
        let mut queue = CommandQueue::default();
        let mut cache = ModelCache::default();
        let mut meshes = Assets::<Mesh>::default();
        let mut materials = Assets::<exa_render::material::ExaMaterial>::default();
        let mut images = Assets::<Image>::default();
        let mut loader = ModelLoader {
            game,
            cache: &mut cache,
            meshes: &mut meshes,
            materials: &mut materials,
            images: &mut images,
            bc_supported: true,
        };
        let info = {
            let mut commands = Commands::new(&mut queue, &world);
            match budget {
                None => spawn_level(&mut commands, &mut loader, name).map(|r| r.1),
                Some(budget) => {
                    let mut load =
                        LevelLoad::begin(&mut commands, &mut loader, name).expect("level parses");
                    let mut steps = 0;
                    loop {
                        steps += 1;
                        if let LoadProgress::Finished(_, info) =
                            load.step(&mut commands, &mut loader, budget)
                        {
                            assert!(steps > 1, "{name}: built in one step");
                            break Some(*info);
                        }
                    }
                }
            }
        };
        queue.apply(&mut world);
        let mut names: Vec<String> = world
            .query::<&Name>()
            .iter(&world)
            .map(|n| n.to_string())
            .collect();
        names.sort();
        // The outdoor preset draws its day from the process clock, so two
        // loads of the same level differ there; the build comparison keeps
        // the level's record and counts but not the clock-seeded lighting.
        let info = info.map(|mut info| {
            info.lighting = environment::Lighting {
                ambient: Vec3::ZERO,
                sun: None,
                fog: None,
                sky: false,
                background: Vec3::ZERO,
                sky_light: Vec3::ZERO,
            };
            info
        });
        let mut out = vec![
            format!("{info:?}"),
            format!("{} entities", world.entities().len()),
            format!(
                "{} meshes, {} materials, {} images",
                meshes.len(),
                materials.len(),
                images.len()
            ),
        ];
        out.extend(names);
        out
    }

    /// A level's rooms, its grid and its shells' triangles (world metres).
    type LevelShells = (
        std::sync::Arc<rooms::Rooms>,
        exa_formats::level::LevelGrid,
        Vec<(room_view::RoomShell, Vec<[Vec3; 3]>)>,
    );

    fn level_shells(game: &GameData, name: &str) -> LevelShells {
        let mut world = World::new();
        let mut queue = CommandQueue::default();
        let mut cache = ModelCache::default();
        let mut meshes = Assets::<Mesh>::default();
        let mut materials = Assets::<exa_render::material::ExaMaterial>::default();
        let mut images = Assets::<Image>::default();
        let mut loader = ModelLoader {
            game,
            cache: &mut cache,
            meshes: &mut meshes,
            materials: &mut materials,
            images: &mut images,
            bc_supported: true,
        };
        {
            let mut commands = Commands::new(&mut queue, &world);
            spawn_level(&mut commands, &mut loader, name).expect("level spawns");
        }
        queue.apply(&mut world);
        let rooms = world.resource::<room_view::LevelRooms>().rooms.clone();
        let bytes = game
            .pack(PackKind::Resource)
            .and_then(|p| p.get(&format!("{name}.rfc")))
            .unwrap();
        let grid = Scene::parse(bytes).unwrap().grid.unwrap();
        let mut q = world.query::<(&room_view::RoomShell, &Mesh3d)>();
        let shells = q
            .iter(&world)
            .map(|(shell, mesh)| {
                let mesh = meshes.get(&mesh.0).unwrap();
                let p = mesh
                    .attribute(Mesh::ATTRIBUTE_POSITION)
                    .and_then(|a| a.as_float3())
                    .unwrap();
                let i: Vec<usize> = mesh.indices().unwrap().iter().collect();
                let tris = i
                    .chunks(3)
                    .map(|t| [0, 1, 2].map(|k| Vec3::from(p[t[k]])))
                    .collect();
                (*shell, tris)
            })
            .collect();
        (rooms, grid, shells)
    }

    /// From above, every built cell of a room (special tiles aside) is
    /// covered by the room's shells with up-facing triangles: an adjacent
    /// room shows no floor through its lid (skipped without the game).
    #[test]
    fn room_lids_cover_their_cells() {
        let Some(dir) = exa_assets::locate_install(None) else {
            return;
        };
        let game = GameData::open(&dir).unwrap();
        for name in ["arenahub", "exanima01"] {
            let (rooms, grid, shells) = level_shells(&game, name);
            let (w, h) = (grid.width as i64, grid.height as i64);
            let mut report = Vec::new();
            for (room, _) in rooms.rooms().iter().enumerate() {
                let lid: Vec<&[Vec3; 3]> = shells
                    .iter()
                    .filter(|(s, _)| {
                        s.room == room as u16 && s.kind != room_view::ShellKind::Overlay
                    })
                    .flat_map(|(_, t)| t.iter())
                    .collect();
                let (mut holes, mut down) = (0, 0);
                for y in 1..h - 1 {
                    for x in 1..w - 1 {
                        let c = (y * w + x) as usize;
                        let open = grid.edges[c] & rooms::CELL_OPEN != 0;
                        let built = grid.plane_c[c] & 0xF != 0xF
                            || grid.edges[c] & rooms::CELL_TERRAIN != 0;
                        // A special tile's walls can close part of its cell
                        // with their own black tops.
                        let special = (grid.plane_c[c] >> 16) & 0x1F != 0;
                        if rooms.room_of_cell(x, y) != Some(room as u16)
                            || !open
                            || !built
                            || special
                        {
                            continue;
                        }
                        let centre = Vec2::new(
                            ((x - w / 2) as f32 + 0.5) * 6.0,
                            ((y - h / 2) as f32 + 0.5) * 6.0,
                        );
                        for j in 0..15 {
                            for i in 0..15 {
                                // Off the pieces' 1.5 m seams, where
                                // strips meet edge to edge.
                                let p = centre
                                    + Vec2::new(i as f32 - 7.0, j as f32 - 7.0) * 0.4
                                    + Vec2::splat(0.07);
                                let mut best: Option<(f32, bool)> = None;
                                for t in &lid {
                                    let [a, b, c] = t.map(|v| Vec2::new(v.x, v.z));
                                    let side = |u: Vec2, v: Vec2| (v - u).perp_dot(p - u);
                                    let (s0, s1, s2) = (side(a, b), side(b, c), side(c, a));
                                    let inside = (s0 >= 0.0 && s1 >= 0.0 && s2 >= 0.0)
                                        || (s0 <= 0.0 && s1 <= 0.0 && s2 <= 0.0);
                                    if !inside {
                                        continue;
                                    }
                                    let n = (t[1] - t[0]).cross(t[2] - t[0]);
                                    let top = t[0].y.max(t[1].y).max(t[2].y);
                                    if best.is_none_or(|(y, _)| top > y) {
                                        best = Some((top, n.y > 0.0));
                                    }
                                }
                                match best {
                                    None => holes += 1,
                                    Some((_, false)) => down += 1,
                                    Some(_) => {}
                                }
                            }
                        }
                    }
                }
                if holes + down > 0 {
                    report.push(format!(
                        "room {room}: {holes} uncovered, {down} down-facing"
                    ));
                }
            }
            assert!(report.is_empty(), "{name}: {report:#?}");
        }
    }

    /// A level built a step at a time is the level built at once (run with
    /// `cargo test -p exa_world -- --ignored` and the game installed).
    #[test]
    #[ignore]
    fn a_level_built_in_steps_is_the_level_built_at_once() {
        let Some(dir) = exa_assets::locate_install(None) else {
            return;
        };
        let game = GameData::open(&dir).unwrap();
        // The level's terrain builds on the async pool in game, as it does
        // here.
        bevy::tasks::AsyncComputeTaskPool::get_or_init(bevy::tasks::TaskPool::default);
        // A tile grid, a terrain, and both with thousands of props.
        for name in ["arena", "arenao1", "exanima01"] {
            let whole = spawn(&game, name, None);
            let stepped = spawn(&game, name, Some(std::time::Duration::ZERO));
            assert_eq!(whole.len(), stepped.len(), "{name}");
            for (a, b) in whole.iter().zip(&stepped) {
                assert_eq!(a, b, "{name}");
            }
        }
    }

    /// With the texture loader's streaming on, a level loaded in one call
    /// still finishes: the terrain's queued textures are finished on the
    /// spot rather than waited for over frames; the placed
    /// objects come through the content stream with no player named by the
    /// default session words (run with `--ignored` and the game installed).
    #[test]
    #[ignore]
    fn a_streaming_level_loaded_in_one_call_finishes_its_terrain_textures() {
        let Some(dir) = exa_assets::locate_install(None) else {
            return;
        };
        let game = GameData::open(&dir).unwrap();
        bevy::tasks::AsyncComputeTaskPool::get_or_init(bevy::tasks::TaskPool::default);
        let world = World::new();
        let mut queue = CommandQueue::default();
        let mut cache = ModelCache::default();
        cache.textures.set_streaming(true);
        let mut meshes = Assets::<Mesh>::default();
        let mut materials = Assets::<exa_render::material::ExaMaterial>::default();
        let mut images = Assets::<Image>::default();
        let mut loader = ModelLoader {
            game: &game,
            cache: &mut cache,
            meshes: &mut meshes,
            materials: &mut materials,
            images: &mut images,
            bc_supported: true,
        };
        let mut commands = Commands::new(&mut queue, &world);
        let (_, info) = spawn_level(&mut commands, &mut loader, "arenao1").expect("level spawns");
        assert!(info.terrain.is_some(), "a terrain level");
        assert!(info.missing_database_objects >= info.missing_objects);
        assert_eq!(info.player_focus, None);
        // Reaching here is the check: a terrain texture left waiting for a
        // loader that never runs would hold the one-call load forever. The
        // props' material textures stay queued behind their placeholders.
        assert!(
            cache
                .textures
                .entries()
                .iter()
                .any(|record| !record.pending)
        );
    }

    pub(crate) fn node(
        name: &str,
        parent: Option<usize>,
        intensity: Option<f32>,
    ) -> exa_assets::ModelNode {
        exa_assets::ModelNode {
            name: name.into(),
            parent,
            transform: Transform::IDENTITY,
            parts: Vec::new(),
            part_materials: Vec::new(),
            part_flags: Vec::new(),
            collision: Vec::new(),
            skin_bone: None,
            light: None,
            voxel_light: intensity.map(|intensity| exa_formats::rfc::VoxelLight {
                color: [0.2157, 0.6784, 1.0],
                intensity,
                radius: 3.0,
            }),
            fire: None,
            portal: None,
            bounds: (Vec3::ZERO, 1.0),
            half_extents: Vec3::ZERO,
            gi_points: Default::default(),
            gi_centred: false,
            gi_thin: false,
            rigid: None,
            tracks: Vec::new(),
            shadow_bias: 0,
            is_skin: false,
            placeholder: false,
            cloth: false,
            cloth_mesh: None,
            mesh: false,
            render_flags: 0,
            triangles: None,
        }
    }

    /// A node's centred GI flag (flags2 0x10) reaches its entity as
    /// [`crate::gi_volume::GiCentredMesh`], and only a node with points.
    #[test]
    fn a_centred_gi_node_carries_its_splat_variant() {
        let mut model = Model {
            nodes: vec![node("Plain", None, None), node("Centred", None, None)],
        };
        model.nodes[0].gi_points = std::sync::Arc::new(vec![Vec3::ZERO]);
        model.nodes[1].gi_points = std::sync::Arc::new(vec![Vec3::ZERO]);
        model.nodes[1].gi_centred = true;
        model.nodes[0].gi_thin = true;
        let mut queue = CommandQueue::default();
        let mut world = World::new();
        {
            let mut commands = Commands::new(&mut queue, &world);
            spawn_model_nodes(&mut commands, &model, None);
        }
        queue.apply(&mut world);
        let mut query = world.query::<(
            Has<crate::gi_volume::GiPoints>,
            Has<crate::gi_volume::GiCentredMesh>,
            Has<crate::gi_volume::GiThinMesh>,
        )>();
        let (with_points, centred, thin) =
            query
                .iter(&world)
                .fold((0, 0, 0), |(points, centred, thin), (has, is, slim)| {
                    (
                        points + has as usize,
                        centred + is as usize,
                        thin + slim as usize,
                    )
                });
        assert_eq!(with_points, 2);
        assert_eq!(centred, 1);
        assert_eq!(thin, 1);
    }

    /// A placement's node transforms change the named direct child of the
    /// prop root, or its own named child under a record; other nodes stay.
    /// Each record is the node's frame in the level, so with the prop
    /// placed at the origin the ring sits at 1 and its pin at 2 (1 from the
    /// ring).
    #[test]
    fn placement_attachments_move_the_named_nodes() {
        use exa_formats::level::PropAttachment;
        let mut model = Model {
            nodes: vec![
                node("Root", None, None),
                node("MetalRing", Some(0), None),
                node("Other", Some(0), None),
                node("Pin", Some(1), None),
                node("Pin", Some(0), None),
            ],
        };
        let rows = |x: f32| {
            [
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [0.0, 0.0, 1.0],
                [x, 0.0, 0.0],
            ]
        };
        let transform =
            |name: &str, x: f32, children: Vec<PropAttachment>| PropAttachment::Transform {
                name: name.into(),
                word: 0,
                transform: rows(x),
                children,
            };
        apply_transform_attachments(
            &mut model,
            &[
                // The ring, then its own child `Pin` (not the root's).
                transform("MetalRing", 1.0, vec![transform("Pin", 2.0, Vec::new())]),
                // Names match exactly, case included.
                transform("other", 3.0, Vec::new()),
                // A record whose node is missing skips its nested records.
                transform("nope", 4.0, vec![transform("Pin", 5.0, Vec::new())]),
            ],
            Transform::IDENTITY,
        );
        let x = |i: usize| model.nodes[i].transform.translation.x;
        assert_eq!((x(0), x(1), x(2), x(3), x(4)), (0.0, 1.0, 0.0, 1.0, 0.0));
    }

    /// A node record is the node's frame in the level: a wall ring placed
    /// turned half round at (1603, 141, 7123) cm, whose record puts the
    /// ring 10 cm in front of the wall, ends with the model's own local
    /// transform (10 cm along its z), not the record's level coordinates
    /// added on top of the placement.
    #[test]
    fn a_node_record_is_taken_from_the_level_frame() {
        use exa_formats::level::PropAttachment;
        let mut model = Model {
            nodes: vec![
                node("MetalRingH", None, None),
                node("MetalRing", Some(0), None),
            ],
        };
        let placement = Transform::from_xyz(16.03, 1.41, 71.23)
            .with_rotation(Quat::from_rotation_y(std::f32::consts::PI));
        // The ring's frame in the level: the placement, then 10 cm along
        // the model's z.
        let world = placement.to_matrix() * Mat4::from_translation(Vec3::new(0.0, 0.0, 0.1));
        let cols = world.to_cols_array_2d();
        // The level stores the rows of its row-vector matrix: the columns
        // of Bevy's.
        let rows = [
            [cols[0][0], cols[0][1], cols[0][2]],
            [cols[1][0], cols[1][1], cols[1][2]],
            [cols[2][0], cols[2][1], cols[2][2]],
            [cols[3][0], cols[3][1], cols[3][2]],
        ];
        assert!(
            transform_from_rows(&rows)
                .to_matrix()
                .abs_diff_eq(world, 1e-5)
        );
        apply_transform_attachments(
            &mut model,
            &[PropAttachment::Transform {
                name: "MetalRing".into(),
                word: 0,
                transform: rows,
                children: Vec::new(),
            }],
            placement,
        );
        let local = model.nodes[1].transform.translation;
        assert!(local.abs_diff_eq(Vec3::new(0.0, 0.0, 0.1), 1e-4), "{local}");
        assert!(
            node_world(&model, 1, placement).abs_diff_eq(world, 1e-4),
            "the ring is where the level saved it"
        );
    }

    /// A banner's saved positions are in the level's frame: placed turned
    /// half round at (19.0, 1.4, 71.2) m, a point the level saved 10 cm in
    /// front of the placement comes back 10 cm along the node's own z.
    #[test]
    fn saved_mesh_positions_come_from_the_level_frame() {
        let model = Model {
            nodes: vec![
                node("UW01_Banne", None, None),
                node("UW01_Bann", Some(0), None),
            ],
        };
        let placement = Transform::from_xyz(19.0, 1.4, 71.2)
            .with_rotation(Quat::from_rotation_y(std::f32::consts::PI));
        let saved = placement.transform_point(Vec3::new(0.0, 0.0, 0.1));
        let local = level_to_node(&model, 1, placement, &[saved.to_array()]);
        assert!(
            Vec3::from_array(local[0]).abs_diff_eq(Vec3::new(0.0, 0.0, 0.1), 1e-4),
            "{:?}",
            local[0]
        );
    }

    /// A placement tree (`0x3DE0EC00`) names direct children all the way
    /// down, an empty name included: it never stands for the node itself.
    #[test]
    fn a_placement_tree_names_children_at_every_level() {
        use exa_formats::level::PropAttachment;
        let mut model = Model {
            nodes: vec![
                node("Root", None, None),
                node("Arm", Some(0), None),
                node("", Some(1), None),
                node("Hand", Some(1), None),
            ],
        };
        let entry = |name: &str, x: f32, children: Vec<PropAttachment>| PropAttachment::Transform {
            name: name.into(),
            word: 0,
            transform: [
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [0.0, 0.0, 1.0],
                [x, 0.0, 0.0],
            ],
            children,
        };
        apply_transform_attachments(
            &mut model,
            &[PropAttachment::Tree {
                entries: vec![
                    // Under the arm, the empty name is the unnamed child.
                    entry("Arm", 1.0, vec![entry("", 2.0, Vec::new())]),
                    // The root has no unnamed child: nothing moves.
                    entry("", 3.0, Vec::new()),
                ],
            }],
            Transform::IDENTITY,
        );
        let x = |i: usize| model.nodes[i].transform.translation.x;
        assert_eq!((x(0), x(1), x(2), x(3)), (0.0, 1.0, 2.0, 0.0));
    }

    #[test]
    fn condition_dims_only_the_voxel_lights_under_the_root() {
        // A salve: the jar at the root, its voxel light under it, and (for
        // the test) one deeper and one beside the root.
        let model = Model {
            nodes: vec![
                node("HealSalve", None, None),
                node("@Omni001", Some(0), Some(16.0)),
                node("Deeper", Some(1), Some(16.0)),
                node("Beside", None, Some(16.0)),
            ],
        };
        let scaled = with_light_share(&model, 0.25);
        let intensity = |i: usize| scaled.nodes[i].voxel_light.map(|l| l.intensity);
        assert_eq!(intensity(1), Some(4.0));
        assert_eq!(intensity(2), Some(16.0));
        assert_eq!(intensity(3), Some(16.0));
        assert_eq!(intensity(0), None);
    }

    #[test]
    fn condition_dims_the_glow_meshes_under_the_root() {
        use exa_render::material::{ExaMaterial, ExaParams};
        let mut materials = Assets::<ExaMaterial>::default();
        let mut meshes = Assets::<Mesh>::default();
        let mesh = meshes.add(Cuboid::default());
        let material = |materials: &mut Assets<ExaMaterial>, e: Vec4| {
            materials.add(ExaMaterial {
                params: ExaParams {
                    emissive: e,
                    ..default()
                },
                ..default()
            })
        };
        let glow = material(&mut materials, Vec4::new(0.04, 0.18, 0.35, 0.0));
        let glass = material(&mut materials, Vec4::new(0.0, 0.0, 0.0, 1.0));
        // The jar (glass and glow) at the root, the glow mesh under it, and
        // a glow deeper down.
        let mut model = Model {
            nodes: vec![
                node("HealSalve", None, None),
                node("JarGlow", Some(0), None),
                node("Deeper", Some(1), None),
            ],
        };
        model.nodes[0].parts = vec![(mesh.clone(), glow.clone()), (mesh.clone(), glass.clone())];
        model.nodes[1].parts = vec![(mesh.clone(), glow.clone()), (mesh.clone(), glass.clone())];
        model.nodes[2].parts = vec![(mesh.clone(), glow.clone())];
        dim_child_glows(&mut model, 0.25, &mut materials);
        let emissive = |n: usize, p: usize| {
            materials
                .get(model.nodes[n].parts[p].1.id())
                .unwrap()
                .params
                .emissive
        };
        // Only the child's glowing part gets a dimmed copy; its alpha stays.
        assert!(
            (emissive(1, 0) - Vec4::new(0.01, 0.045, 0.0875, 0.0))
                .abs()
                .max_element()
                < 1e-6
        );
        assert_ne!(model.nodes[1].parts[0].1, glow);
        assert_eq!(model.nodes[1].parts[1].1, glass);
        assert_eq!(model.nodes[0].parts[0].1, glow);
        assert_eq!(model.nodes[2].parts[0].1, glow);
        // The shared material is untouched.
        assert_eq!(emissive(0, 0), Vec4::new(0.04, 0.18, 0.35, 0.0));
    }

    fn item(id: u32, model: &str, worn: &str, colour: u8) -> Item {
        let mut layers = [objdb::Layer::default(); 8];
        layers[0] = objdb::Layer {
            mask: 1,
            random_set: 3,
            material: 1,
            colour,
        };
        Item {
            id,
            name: worn.into(),
            parent: 0,
            model: model.into(),
            model_flags: 0,
            worn_model: worn.into(),
            appearance: Some(Appearance {
                quality: 160,
                damage: 64,
                dirt: 0,
                layers,
            }),
            slot_layers: [0; objdb::SLOTS],
            slot_words: [0; objdb::SLOTS],
            occupancy: [0; objdb::OCCUPANCY_BONES],
            weapon: None,
            armour: None,
            shield: None,
            door: None,
            operative: None,
            burden: None,
            equip_lock: 0,
            torch: false,
            volume: None,
            rank_word: 0,
            body_rank: 0,
            loadout_rank: 0,
            armour_weight: 0.0,
            armour_type: 0,
            container: None,
            random_table: None,
            has_body: false,
            has_weapon: false,
            has_shield: false,
        }
    }

    #[test]
    fn only_objects_with_a_usable_or_container_section_carry_their_sections() {
        let object = exa_formats::rfc::PlacedObject {
            id: objdb::LOCAL | 4,
            flags: 0x0003_0001,
            object_flags: 0,
            transform: [[0.0; 3]; 4],
            slot: 0,
            second_word: 0,
            word_3d22: None,
            attachments: Vec::new(),
        };
        let mut chest = item(4, "chest", "", 0);
        assert_eq!(ObjectSections::of(&chest, &object), None);
        chest.container = Some(exa_formats::containers::Container {
            width: 50,
            height: 30,
            capacity: 400,
            max_item: 60,
            children: Vec::new(),
        });
        let sections = ObjectSections::of(&chest, &object).unwrap();
        assert_eq!(sections.placement, 0x0003_0001);
        assert!(sections.usable.is_none());
        assert_eq!(sections.container.unwrap().width, 50);
    }

    fn record(id: u32, worn: Vec<u32>) -> CharacterRecord {
        CharacterRecord {
            id,
            name: format!("record {id}"),
            worn: worn.into_iter().map(|id| ItemRef::new(id, 0)).collect(),
            ..Default::default()
        }
    }

    fn placement(id: u32) -> Placement {
        Placement {
            id,
            position: [0.0; 3],
            yaw: 0.0,
            condition: 0,
            pose: exa_formats::characters::SavedPose::None,
            motion: None,
            ai_home: None,
        }
    }

    /// A world-database character is created only in its own locale; a
    /// level's local records always are. Without the level's locale nothing
    /// is filtered.
    #[test]
    fn world_characters_wait_for_their_locale() {
        let mut local = record(3, Vec::new());
        local.locale = 9;
        let mut elsewhere = record(0x4000_0004, Vec::new());
        elsewhere.locale = 5;
        let mut here = record(0x4000_0005, Vec::new());
        here.locale = 2;
        let scene = Scene {
            version: 0,
            nodes: Vec::new(),
            materials: Vec::new(),
            grid: None,
            props: Vec::new(),
            environment: None,
            characters: Some(LevelCharacters {
                records: vec![local, elsewhere, here],
                placements: vec![placement(3), placement(0x4000_0004), placement(0x4000_0005)],
            }),
            local_items: Vec::new(),
            local_records: Vec::new(),
            local_strings: None,
            local_string_error: None,
            placed_objects: Vec::new(),
            unknown_chunks: Vec::new(),
            level_errors: Vec::new(),
        };
        let ids = |locale| {
            placed_characters(&scene, &[], locale)
                .iter()
                .map(|c| c.record.id)
                .collect::<Vec<_>>()
        };
        assert_eq!(ids(Some(2)), [3, 0x4000_0005]);
        assert_eq!(ids(Some(5)), [3, 0x4000_0004]);
        assert_eq!(ids(None).len(), 3);
    }

    #[test]
    fn placed_characters_wear_their_stored_items() {
        // The underwear has no ground model; quilted trousers show
        // `Trouser_At` on the ground but are worn as `Trouser_A`.
        let global = [item(8, "", "Braies_A", 1), item(9, "Shirt_B", "Shirt_B", 2)];
        let scene = Scene {
            version: 0,
            nodes: Vec::new(),
            materials: Vec::new(),
            grid: None,
            props: Vec::new(),
            environment: None,
            characters: Some(LevelCharacters {
                records: vec![record(1, vec![8, LOCAL_ITEM | 3]), record(2, Vec::new())],
                placements: vec![placement(1), placement(2)],
            }),
            // The trousers weigh 10 × 0.0025.
            local_items: vec![Item {
                burden: Some(exa_formats::objdb::Burden {
                    encumbrance: 10,
                    ..Default::default()
                }),
                ..item(3, "Trouser_At", "Trouser_A", 5)
            }],
            local_records: Vec::new(),
            local_strings: None,
            local_string_error: None,
            placed_objects: Vec::new(),
            unknown_chunks: Vec::new(),
            level_errors: Vec::new(),
        };
        let placed = placed_characters(&scene, &global, None);
        assert_eq!(placed.len(), 2);
        assert_eq!((placed[0].encumbrance, placed[1].encumbrance), (0.025, 0.0));
        let worn: Vec<&str> = placed[0].worn.iter().map(|w| w.model.as_str()).collect();
        assert_eq!(worn, ["Braies_A", "Trouser_A"]);
        let braies = placed[0].worn[0].appearance.unwrap();
        assert_eq!(braies.layers[0].colour, 1);
        assert!(placed[1].worn.is_empty());
    }

    /// A placed object's node with a keyframe track (the secret switch's
    /// brick, under the model's root) plays it when the object is started:
    /// the brick sinks along its parent's −z while the placement stays.
    #[test]
    fn placed_models_play_their_node_tracks() {
        use bevy::time::TimeUpdateStrategy;
        use exa_anim::tracks::{NodeTracks, StartTracks, TracksPlugin};
        use exa_formats::key_track::{KeyTrack, PlayMode, PositionKey};
        let key = |time, z: f32| PositionKey {
            time,
            position: [0.0, 0.0, z * 0.01],
        };
        let mut brick = node("Box001", Some(0), None);
        brick.tracks = vec![KeyTrack {
            mode: PlayMode::Once,
            duration: 3.0,
            keys: vec![key(0.0, 0.0), key(1.0, -5.0), key(2.0, -5.0), key(3.0, 0.0)],
        }];
        let model = Model {
            nodes: vec![node("Plane001", None, None), brick],
        };
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, TracksPlugin))
            .insert_resource(TimeUpdateStrategy::ManualDuration(
                std::time::Duration::from_millis(250),
            ));
        app.update();
        let placement = Transform::from_xyz(4.0, 1.0, -2.0);
        let mut queue = CommandQueue::default();
        let root = spawn_model(
            &mut Commands::new(&mut queue, app.world()),
            &model,
            Some(placement),
        );
        queue.apply(app.world_mut());
        let mut named = app.world_mut().query::<(Entity, &Name, Has<NodeTracks>)>();
        let nodes: Vec<(String, bool)> = named
            .iter(app.world())
            .map(|(_, n, tracks)| (n.to_string(), tracks))
            .collect();
        assert!(nodes.contains(&("Box001".into(), true)), "{nodes:?}");
        assert!(nodes.contains(&("Plane001".into(), false)), "{nodes:?}");
        let box001 = named
            .iter(app.world())
            .find(|(_, n, _)| n.as_str() == "Box001")
            .unwrap()
            .0;
        app.world_mut().write_message(StartTracks(root));
        for _ in 0..3 {
            app.update();
        }
        // 0.5 s into the first (reversed) play: the curve at 2.5 s.
        let z = app.world().get::<Transform>(box001).unwrap().translation.z;
        assert!((z - -0.028125).abs() < 1e-6, "{z}");
        assert_eq!(app.world().get::<Transform>(root), Some(&placement));
    }

    #[test]
    fn scene_ids_are_looked_up_and_remapped() {
        use exa_formats::objdb::{LOCAL, ObjectDb, Store, WORLD};
        const NAME: u32 = 0x1000_0000;
        let mut body = vec![0u8; 0x18];
        body[..3].copy_from_slice(b"Box");
        let mut payload = NAME.to_le_bytes().to_vec();
        payload.extend((0x18u32 + 8).to_le_bytes());
        payload.extend(body);
        let mut db = ObjectDb::default();
        db.local = Store::from_records([(1u32, NAME, &payload[..])]);
        db.set_remaps(true);
        assert_eq!(db.promote(LOCAL | 1), WORLD | 1);

        let mut world = World::new();
        let local = world.spawn_empty().id();
        let other = world.spawn_empty().id();
        let placed = [
            (
                local,
                PlacedItem {
                    id: 1,
                    tagged_id: LOCAL | 1,
                    condition: 0,
                },
            ),
            (
                other,
                PlacedItem {
                    id: 2,
                    tagged_id: 2,
                    condition: 0,
                },
            ),
        ];
        let lookup = |id| object_entity(placed.iter().map(|(e, p)| (*e, p)), id);
        assert_eq!(lookup(LOCAL | 1), Some(local));
        assert_eq!(lookup(LOCAL | 2), None);
        // The node id is compared whole: an untagged id does not find the
        // local node.
        assert_eq!(lookup(1), None);

        let mut items = [
            PlacedItem {
                id: 1,
                tagged_id: LOCAL | 1,
                condition: 0,
            },
            PlacedItem {
                id: 2,
                tagged_id: 2,
                condition: 0,
            },
            PlacedItem {
                id: 3,
                tagged_id: WORLD | 1,
                condition: 0,
            },
            PlacedItem {
                id: 4,
                tagged_id: LOCAL | WORLD | 1,
                condition: 0,
            },
        ];
        assert_eq!(remap_placed_ids(items.iter_mut(), &db), 1);
        assert_eq!(items[0].tagged_id, WORLD | 1);
        assert_eq!(items[1].tagged_id, 2);
        assert_eq!(items[2].tagged_id, WORLD | 1);
        assert_eq!(items[3].tagged_id, LOCAL | WORLD | 1);
    }
}
