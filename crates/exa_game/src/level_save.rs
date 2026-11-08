//! The save's level section: the current level written into the save under
//! its locale name, with the live local tier and the live placed objects,
//! so a load reads the level from the save's own section and finds the
//! objects where the player left them.
//!
//! **What the original writes.** After the world databases, the save
//! exports the game's whole scene with the save switches (the story's
//! exporter adds the settings chunk) and stores it as the package section
//! named by the scene's locale. The exporter's local-database chunk
//! (`0xDBC0`) holds the **live local tier**: its `0xDBDB` records (the
//! tier's writer, the slot after the last entry included) and its `0xDBCD`
//! string library, written when the walk found a placed object and the
//! tier holds a record, then the placed objects (`0xDBCE`) as the scene
//! holds them now.
//!
//! **What Exarust writes.** Its scene model has no converter back into the
//! exporter's scene, so the export starts from the section the level was
//! read from ([`exa_world::LevelSource`]) and writes the live state over
//! it: the local tier and its library from the live database, and the
//! placed objects from the live entities: an object the load spawned keeps
//! its place in the list with its live id, condition and placement (a body
//! that moved takes the body's motion since it was registered); one the
//! load spawned that is gone (picked up, broken) leaves the list with the
//! objects inside it, and the later objects' container numbers close up; one
//! the load never spawned (a record without a model, another slot's) stays
//! as the section had it; an object dropped into the world since is added
//! at the end. A story save also writes the scene's characters: the save
//! first flushes every scene character's live possessions and health into
//! its record in its store ([`LevelSaveScene::flush_characters`]), then the
//! section's placements follow the live scene ([`characters_after`]) and its
//! character database is the live local store
//! ([`crate::world_characters::LocalCharacters`]). [Exarust's: the props,
//! the doors' and levers' positions and the settings chunk's live words are
//! the loaded section's; a character's pose, motion and AI home are the
//! stored ones.]
//!
//! The original's character section (`0xCAC00001`): each character's health
//! is stored into its record, the local character database is flushed and
//! written (`0xCECDDB00`) when it holds a record, then each character's
//! placement (`0xCAC0CA00`: the record's id, the body's x, floor and z, the
//! heading and the condition word, then the pose for a dead body, and the
//! AI home and motion in save mode).
//!
//! Evidence: the save unit's save routine (the section order, the
//! exporter's call and the locale name) and the exporter's scene walk and
//! local-database chunk (`docs/formats/SAVES.md`, Saving; `RFC.md`).

use bevy::math::Affine3A;
use bevy::prelude::*;
use exa_formats::characters::{ID_MASK, Placement, WORLD_DB};
use exa_formats::rfc::{PlacedObject, Scene};
use exa_formats::rfc_level::LevelRaw;

use crate::level_transition::{SettingsState, export_level_scene};

/// A placed object as it stands in the running scene.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LivePlacement {
    /// Its place in the loaded section's list; `None` for an object put
    /// into the world since (a drop).
    pub index: Option<u32>,
    /// Its database id, tier tag included.
    pub id: u32,
    /// The condition byte of its placement word.
    pub condition: u8,
    /// Its placement: three rotation rows and the translation (metres).
    pub rows: [[f32; 3]; 4],
}

/// What the placed-object pass did.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PlacementCounts {
    /// Objects the load spawned that are still in the scene.
    pub kept: usize,
    /// Of those, the ones whose placement changed.
    pub moved: usize,
    /// Of those, the ones that moved more than a centimetre.
    pub moved_far: usize,
    /// Objects the load spawned that are gone, with the objects inside them.
    pub removed: usize,
    /// Objects the load never spawned, kept as the section had them.
    pub unspawned: usize,
    /// Objects added since the load.
    pub added: usize,
}

/// A placement's rows from a world transform.
pub fn rows_of(affine: Affine3A) -> [[f32; 3]; 4] {
    [
        affine.matrix3.x_axis.to_array(),
        affine.matrix3.y_axis.to_array(),
        affine.matrix3.z_axis.to_array(),
        affine.translation.to_array(),
    ]
}

/// How far two placements are apart: the largest component difference.
fn rows_apart(a: &[[f32; 3]; 4], b: &[[f32; 3]; 4]) -> f32 {
    a.iter()
        .flatten()
        .zip(b.iter().flatten())
        .map(|(x, y)| (x - y).abs())
        .fold(0.0, f32::max)
}

/// A live placement closer than this to the stored one keeps the stored
/// rows, so an object nothing touched is written back unchanged: the live
/// transform is the stored one taken through single-precision matrices
/// and a sleeping body's reconstructed frame, which leaves noise of a few
/// tenths of a millimetre on most loose objects [Exarust's: the original
/// writes every node's live matrix].
const UNMOVED: f32 = 1e-3;

/// The section's placed objects after the live scene: see the module notes.
/// `nodes` and `props` are the section's scene node and prop counts, which
/// number the containers an object's slot word names (an object's number is
/// `nodes + props + index`).
pub fn placed_after(
    placed: &[PlacedObject],
    nodes: usize,
    props: usize,
    spawned: &[u32],
    live: &[LivePlacement],
) -> (Vec<PlacedObject>, PlacementCounts) {
    let mut counts = PlacementCounts::default();
    let first_object = nodes + props;
    let by_index: std::collections::HashMap<u32, &LivePlacement> = live
        .iter()
        .filter_map(|p| p.index.map(|index| (index, p)))
        .collect();
    let spawned: std::collections::HashSet<u32> = spawned.iter().copied().collect();
    // Gone: spawned and no longer standing; then whatever was inside them.
    let mut gone: Vec<bool> = (0..placed.len())
        .map(|i| {
            let i = i as u32;
            spawned.contains(&i) && !by_index.contains_key(&i)
        })
        .collect();
    let container = |object: &PlacedObject| -> Option<usize> {
        let number = usize::try_from(object.slot).ok()?;
        number.checked_sub(first_object)
    };
    loop {
        let mut changed = false;
        for (i, object) in placed.iter().enumerate() {
            if !gone[i] && container(object).is_some_and(|c| gone.get(c) == Some(&true)) {
                gone[i] = true;
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    // Each kept object's new index, to renumber the slot words.
    let mut new_index = Vec::with_capacity(placed.len());
    let mut next = 0usize;
    for &g in &gone {
        new_index.push((!g).then_some(next));
        if !g {
            next += 1;
        }
    }
    let mut out = Vec::with_capacity(placed.len());
    for (i, object) in placed.iter().enumerate() {
        if gone[i] {
            counts.removed += 1;
            continue;
        }
        let mut object = object.clone();
        if let Some(c) = container(&object)
            && let Some(Some(at)) = new_index.get(c)
        {
            object.slot = (first_object + at) as i32;
        }
        match by_index.get(&(i as u32)) {
            Some(live) => {
                counts.kept += 1;
                object.id = live.id;
                object.flags = (object.flags & !0x00FF_0000) | (u32::from(live.condition) << 16);
                if rows_apart(&live.rows, &object.transform) > UNMOVED {
                    let travel = Vec3::from(live.rows[3]).distance(Vec3::from(object.transform[3]));
                    if travel > 0.01 {
                        counts.moved_far += 1;
                    }
                    object.transform = live.rows;
                    counts.moved += 1;
                }
            }
            None => counts.unspawned += 1,
        }
        out.push(object);
    }
    for live in live.iter().filter(|p| p.index.is_none()) {
        counts.added += 1;
        let mut object = exa_world::dropped_object(live.id, u32::from(live.condition) << 16);
        object.transform = live.rows;
        out.push(object);
    }
    (out, counts)
}

/// A character as it stands in the running scene, for the save's
/// character section.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LiveCharacter {
    /// Its record's id (the world tag for a world record).
    pub id: u32,
    /// Its feet, metres.
    pub position: [f32; 3],
    /// Its heading (Exarust's yaw).
    pub yaw: f32,
    /// Its body is dead.
    pub dead: bool,
}

/// What the character pass did.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CharacterCounts {
    /// Placements whose character still stands in the scene.
    pub kept: usize,
    /// Of those, the ones written with the live place (or death).
    pub moved: usize,
    /// Placements whose record left the level (a follower that took a
    /// door), left out.
    pub removed: usize,
    /// Placements with no live character, kept as the section had them.
    pub unspawned: usize,
    /// Characters with no placement in the section (an arrived follower),
    /// added at the end.
    pub added: usize,
}

/// A live character closer than this (metres, any axis) to its stored
/// placement keeps the stored one, so a character that only idled is
/// written back unchanged [Exarust's: the original writes every
/// character's live body position and heading; Exarust's standing bodies
/// sway by a few centimetres].
const CHARACTER_UNMOVED: f32 = 0.1;

/// Whether two ids name the same record: the same store (the world bit)
/// and the same low 28 bits.
fn same_record(a: u32, b: u32) -> bool {
    a & WORLD_DB == b & WORLD_DB && a & ID_MASK == b & ID_MASK
}

/// The section's character placements after the live scene, as the
/// exporter's walk writes the scene's characters: each stored placement
/// whose record's character stands in the scene (the nearest of that
/// record's) takes its live place, heading and death when it moved; one
/// with no live character stays as stored unless `gone` says its record
/// left the level's store (a follower flushed at a door: its node left the
/// scene); a live character with no placement is added at the end with its
/// live place and condition (living 0, dead `0xA00`) and no saved pose.
/// A character who died since the load keeps the stored pose, if any, and
/// one stored dead keeps its whole placement [Exarust's: the original
/// writes the live pose tree of a dead body].
pub fn characters_after(
    stored: &[Placement],
    live: &[LiveCharacter],
    gone: impl Fn(u32) -> bool,
) -> (Vec<Placement>, CharacterCounts) {
    let mut counts = CharacterCounts::default();
    let mut used = vec![false; live.len()];
    let mut out = Vec::with_capacity(stored.len() + live.len());
    for placement in stored {
        let here = Vec3::from_array(placement.position);
        let nearest = live
            .iter()
            .enumerate()
            .filter(|&(i, c)| !used[i] && same_record(c.id, placement.id))
            .min_by(|(_, a), (_, b)| {
                let da = Vec3::from_array(a.position).distance_squared(here);
                let db = Vec3::from_array(b.position).distance_squared(here);
                da.total_cmp(&db)
            })
            .map(|(i, _)| i);
        let Some(i) = nearest else {
            if placement.id & WORLD_DB == 0 && gone(placement.id) {
                counts.removed += 1;
            } else {
                counts.unspawned += 1;
                out.push(placement.clone());
            }
            continue;
        };
        used[i] = true;
        counts.kept += 1;
        let character = live[i];
        let mut placement = placement.clone();
        if placement.is_dead() {
            // A level's dead lie on their stored pose and do not walk.
            out.push(placement);
            continue;
        }
        let travel = (Vec3::from_array(character.position) - here)
            .abs()
            .max_element();
        let died = character.dead && !placement.is_dead();
        if travel > CHARACTER_UNMOVED || died {
            counts.moved += 1;
            placement.position = character.position;
            placement.yaw = character.yaw;
            if died {
                placement.condition = exa_formats::characters::DEAD;
            }
        }
        out.push(placement);
    }
    for (character, _) in live.iter().zip(&used).filter(|(_, used)| !**used) {
        counts.added += 1;
        out.push(Placement {
            id: character.id,
            position: character.position,
            yaw: character.yaw,
            condition: if character.dead {
                exa_formats::characters::DEAD
            } else {
                0
            },
            pose: exa_formats::characters::SavedPose::None,
            motion: None,
            ai_home: None,
        });
    }
    (out, counts)
}

/// The character part of a story save's level section: the live scene's
/// characters, the level store's records that left it, the world records
/// the placements may name and the live local store's body.
#[derive(Debug, Clone)]
pub struct SectionCharacters<'a> {
    pub live: &'a [LiveCharacter],
    /// Low ids of the records the live level store holds; a stored
    /// placement whose record it lacks left the level.
    pub held: &'a [u32],
    pub world: &'a [exa_formats::characters::CharacterRecord],
    /// The local store's body (`None`: the store is empty and the section
    /// writes none).
    pub database: Option<Vec<u8>>,
}

/// The level section a save writes: `source` (the section the level was
/// read from) with the live placed objects and the live local tier
/// (`records`, the tier's writer, and `strings`, its library's) written
/// over it, exported with the save switches ([`export_level_scene`]).
#[allow(clippy::too_many_arguments)]
pub fn saved_section(
    source: &[u8],
    spawned: &[u32],
    live: &[LivePlacement],
    records: Vec<u8>,
    strings: Vec<u8>,
    story: bool,
    settings: SettingsState,
    characters: Option<SectionCharacters<'_>>,
) -> exa_formats::Result<(Vec<u8>, PlacementCounts)> {
    let mut scene = Scene::parse(source)?;
    let mut raw = LevelRaw::scan(source)?;
    if let Some(characters) = characters {
        let chars = scene.characters.get_or_insert_with(Default::default);
        let (placements, _) = characters_after(&chars.placements, characters.live, |id| {
            !characters.held.contains(&(id & ID_MASK))
        });
        chars.placements = placements;
        // The world records the placements name, for their node names.
        chars.records.extend(characters.world.iter().cloned());
        raw.character_database = characters.database;
    }
    let (placed, counts) = placed_after(
        &scene.placed_objects,
        scene.nodes.len(),
        scene.props.len(),
        spawned,
        live,
    );
    scene.placed_objects = placed;
    raw.object_database = Some(records);
    raw.object_strings = Some(strings);
    let bytes = export_level_scene(&scene, &raw, story, settings)?;
    Ok((bytes, counts))
}

/// A placed object's entity as the save reads it: its id, its place in
/// the loaded list, its world transform and whether a body moves it.
type PlacedNode = (
    Entity,
    &'static mut exa_world::PlacedItem,
    Option<&'static exa_world::PlacedIndex>,
    &'static GlobalTransform,
    Has<exa_physics::prop::BodyNode>,
);

/// What the save reads of the running level: where it came from and its
/// placed objects with their bodies.
#[derive(bevy::ecs::system::SystemParam)]
pub struct LevelSaveScene<'w, 's> {
    source: Option<Res<'w, exa_world::LevelSource>>,
    placed: Query<'w, 's, PlacedNode>,
    bodies: Query<
        'w,
        's,
        (
            Entity,
            &'static exa_physics::prop::PropBody,
            &'static exa_physics::body::Ragdoll,
        ),
    >,
    parents: Query<'w, 's, &'static ChildOf>,
    characters: Query<'w, 's, SceneCharacter, Without<crate::player::PlayerBody>>,
    local: Option<ResMut<'w, crate::world_characters::LocalCharacters>>,
}

/// A scene character as the save's flush reads it.
type SceneCharacter = (
    &'static GlobalTransform,
    &'static exa_character::record::CharacterData,
    Option<&'static exa_items::possession::Possessions>,
    Option<&'static exa_combat::Fighter>,
    Option<&'static exa_combat::StateFlags>,
    Option<&'static exa_combat::Condition>,
    Option<&'static crate::thaum::cast::PowerCaster>,
    Option<&'static exa_physics::body::locomotion::Locomotor>,
);

impl LevelSaveScene<'_, '_> {
    /// The arena save's entity rewrite: every scene object whose id is a
    /// local one the id remap table holds takes its world id
    /// ([`exa_formats::objdb::ObjectDb::remap_entity_id`]). Returns how many
    /// changed.
    pub fn rewrite_entity_ids(&mut self, db: &exa_formats::objdb::ObjectDb) -> usize {
        let mut changed = 0;
        for (_, mut item, ..) in &mut self.placed {
            let id = db.remap_entity_id(item.tagged_id);
            if id != item.tagged_id {
                let item = item.bypass_change_detection();
                item.tagged_id = id;
                item.id = id & exa_formats::objdb::ID_MASK;
                changed += 1;
            }
        }
        changed
    }

    /// The live placements: each placed object's id and condition, and its
    /// placement (an object a body moves directly is where its node is; one
    /// over a moved body takes the body's motion since it was registered).
    pub fn live_placements(&self) -> Vec<LivePlacement> {
        // The motion of each body since its registration, by the placed
        // object at or above its root node.
        let mut motion = std::collections::HashMap::new();
        for (entity, body, ragdoll) in &self.bodies {
            let (Some(placed), Some(part)) = (body.placed().first(), ragdoll.parts.first()) else {
                continue;
            };
            let delta = part.transform() * placed.inverse();
            let mut at = entity;
            loop {
                if self.placed.contains(at) {
                    motion.entry(at).or_insert(delta);
                    break;
                }
                match self.parents.get(at) {
                    Ok(parent) => at = parent.parent(),
                    Err(_) => break,
                }
            }
        }
        let mut out: Vec<LivePlacement> = self
            .placed
            .iter()
            .map(|(entity, item, index, global, moved_directly)| {
                let affine = global.affine();
                let affine = match motion.get(&entity) {
                    Some(delta) if !moved_directly => *delta * affine,
                    _ => affine,
                };
                LivePlacement {
                    index: index.map(|i| i.0),
                    id: item.tagged_id,
                    condition: item.condition,
                    rows: rows_of(affine),
                }
            })
            .collect();
        // The list's order, then the drops in spawn order.
        out.sort_by_key(|p| p.index.unwrap_or(u32::MAX));
        out
    }

    /// The save's flush of the scene's characters (the player aside, whose
    /// record the save writes itself): each one's live possessions and
    /// health go into its record ([`crate::save_live::scene_character_record`]),
    /// which becomes the live copy in its store: the world store for a
    /// world record (a follower that took a door), the level's for the
    /// rest. Returns where each stands, for the level section.
    pub fn flush_characters(
        &mut self,
        mut world: Option<&mut crate::world_characters::WorldCharacters>,
    ) -> Vec<LiveCharacter> {
        let mut out = Vec::new();
        let (mut to_world, mut to_local) = (0, 0);
        for (transform, data, possessions, fighter, flags, condition, caster, locomotor) in
            &self.characters
        {
            let record = crate::save_live::scene_character_record(
                data,
                possessions,
                fighter.map(|f| (f, flags.copied(), condition.copied(), caster)),
            );
            if record.id & WORLD_DB != 0 {
                if let Some(world) = world.as_deref_mut() {
                    world.store(&record);
                    to_world += 1;
                }
            } else if let Some(local) = self.local.as_deref_mut()
                && local.store(&record)
            {
                to_local += 1;
            }
            // The body's yaw is its locomotion's; the node keeps its spawn
            // turn.
            let yaw = locomotor.map_or_else(
                || transform.rotation().to_euler(EulerRot::YXZ).0,
                |locomotor| locomotor.yaw,
            );
            out.push(LiveCharacter {
                id: record.id,
                position: transform.translation().to_array(),
                yaw,
                dead: condition.is_some_and(|c| c.is_dead()),
            });
        }
        // The scene's order is the entities'; keep it stable.
        info!(
            "save: {} scene characters flushed ({to_world} into the world character database, {to_local} into the level's)",
            out.len()
        );
        out
    }

    /// The section the save writes for the running level, named by its
    /// locale; `None` without a loaded level or when it does not export.
    /// A story save passes the scene's characters (`characters`, from
    /// [`Self::flush_characters`]) and the world records their placements
    /// may name (`world`): the section's placements and its character
    /// database are then the live ones ([`characters_after`],
    /// [`crate::world_characters::LocalCharacters::section_body`]).
    pub fn section(
        &mut self,
        db: &exa_formats::objdb::ObjectDb,
        story: bool,
        characters: Option<(
            &[LiveCharacter],
            &[exa_formats::characters::CharacterRecord],
        )>,
    ) -> Option<(String, Vec<u8>)> {
        let live = self.live_placements();
        let characters = characters.filter(|_| story).and_then(|(live, world)| {
            let local = self.local.as_deref_mut()?;
            let held: Vec<u32> = local.db.ids().collect();
            Some((live, world, held, local.section_body()))
        });
        let source = self.source.as_deref()?;
        let records = db.write(exa_formats::objdb::Tier::Local);
        let strings = db.library(exa_formats::objdb::Tier::Local).write();
        if let Some((live, _, held, _)) = &characters {
            let stored = exa_formats::rfc::Scene::parse(&source.bytes)
                .ok()
                .and_then(|scene| scene.characters)
                .map(|chars| chars.placements)
                .unwrap_or_default();
            let (after, counts) =
                characters_after(&stored, live, |id| !held.contains(&(id & ID_MASK)));
            // The first moved characters' stored and written places, for a
            // check of the live positions against the placements.
            for (before, now) in stored
                .iter()
                .zip(&after)
                .filter(|(a, b)| a.id == b.id && a.position != b.position)
                .take(4)
            {
                info!(
                    "save: character {:#x} written at {:?} (stored {:?}, yaw {:.3} -> {:.3})",
                    now.id, now.position, before.position, before.yaw, now.yaw
                );
            }
            info!(
                "save: level section {} characters: {} kept ({} moved), {} gone, {} never spawned, {} added",
                source.locale(),
                counts.kept,
                counts.moved,
                counts.removed,
                counts.unspawned,
                counts.added
            );
        }
        match saved_section(
            &source.bytes,
            &source.spawned,
            &live,
            records,
            strings,
            story,
            SettingsState::reset(),
            characters
                .as_ref()
                .map(|(live, world, held, database)| SectionCharacters {
                    live,
                    held,
                    world,
                    database: database.clone(),
                }),
        ) {
            Ok((bytes, counts)) => {
                info!(
                    "save: level section {} exported ({} bytes, local tier {} records): {} placed objects kept ({} moved, {} by more than 1 cm), {} gone, {} never spawned, {} added",
                    source.locale(),
                    bytes.len(),
                    db.store(exa_formats::objdb::Tier::Local).len(),
                    counts.kept,
                    counts.moved,
                    counts.moved_far,
                    counts.removed,
                    counts.unspawned,
                    counts.added
                );
                Some((source.locale().to_string(), bytes))
            }
            Err(error) => {
                warn!(
                    "save: level section {} does not export: {error}",
                    source.locale()
                );
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn object(id: u32, slot: i32, x: f32) -> PlacedObject {
        let mut object = exa_world::dropped_object(id, 0x0005_0000);
        object.slot = slot;
        object.transform[3] = [x, 0.0, 0.0];
        object
    }

    fn live(index: Option<u32>, id: u32, x: f32) -> LivePlacement {
        let mut rows = object(id, 0, x).transform;
        rows[3] = [x, 0.0, 0.0];
        LivePlacement {
            index,
            id,
            condition: 5,
            rows,
        }
    }

    /// A spawned object that is gone leaves with what it held, the later
    /// objects' container numbers close up, an unspawned one stays, a moved
    /// one takes its live placement and a drop is added at the end.
    #[test]
    fn the_live_scene_rewrites_the_placed_objects() {
        // Two nodes and one prop: objects are numbered from 3.
        let placed = [
            object(0x8000_0001, 0, 1.0), // 0: gone
            object(0x8000_0002, 3, 2.0), // 1: inside 0, gone with it
            object(0x8000_0003, 0, 3.0), // 2: moved
            object(0x8000_0004, 5, 4.0), // 3: inside 2, unspawned
            object(0x8000_0005, 0, 5.0), // 4: kept in place
        ];
        let spawned = [0, 1, 2, 4];
        let scene = [
            live(Some(2), 0x8000_0003, 3.5),
            live(Some(4), 0x8000_0005, 5.0),
            live(None, 0x8000_000B, 9.0),
        ];
        let (out, counts) = placed_after(&placed, 2, 1, &spawned, &scene);
        assert_eq!(
            counts,
            PlacementCounts {
                kept: 2,
                moved: 1,
                moved_far: 1,
                removed: 2,
                unspawned: 1,
                added: 1
            }
        );
        let ids: Vec<u32> = out.iter().map(|o| o.id).collect();
        assert_eq!(ids, [0x8000_0003, 0x8000_0004, 0x8000_0005, 0x8000_000B]);
        assert_eq!(out[0].transform[3], [3.5, 0.0, 0.0]);
        // The unspawned child now names its container's new number.
        assert_eq!(out[1].slot, 3);
        assert_eq!(out[2].transform[3], [5.0, 0.0, 0.0]);
        assert_eq!((out[3].slot, out[3].flags), (0, 0x0005_0000));
    }

    /// An object nothing moved keeps its stored rows exactly.
    #[test]
    fn an_unmoved_object_keeps_its_stored_rows() {
        let mut stored = object(7, 0, 1.0);
        stored.transform[0] = [0.6, 0.0, 0.8];
        let mut nearly = live(Some(0), 7, 1.0);
        nearly.rows = stored.transform;
        nearly.rows[3][1] = 3e-4;
        let (out, counts) = placed_after(&[stored.clone()], 0, 0, &[0], &[nearly]);
        assert_eq!(counts.moved, 0);
        assert_eq!(out[0].transform, stored.transform);
    }

    /// A shipped level's section written for a save reads back with the
    /// live local tier in place of its own, without the object the player
    /// took, with a moved object where it stands and a dropped one at the
    /// end (skipped without the game).
    #[test]
    fn a_shipped_level_saves_its_live_state_and_reads_it_back() {
        let Some(dir) = exa_assets::locate_install(None) else {
            return;
        };
        let game = exa_assets::GameData::open(&dir).unwrap();
        let pack = game.pack(exa_assets::PackKind::Resource).unwrap();
        let source = pack.get("exanima01.rfc").unwrap();
        let scene = Scene::parse(source).unwrap();
        let count = scene.placed_objects.len() as u32;
        assert!(count > 2);
        // The first object (a world one, inside nothing) is taken; the
        // second moves 50 cm up; the rest stand.
        let taken = (0..count)
            .find(|&i| scene.placed_objects[i as usize].slot == 0)
            .unwrap();
        let spawned: Vec<u32> = (0..count).collect();
        let mut live: Vec<LivePlacement> = scene
            .placed_objects
            .iter()
            .enumerate()
            .filter(|&(i, object)| i as u32 != taken && object.slot == 0)
            .map(|(i, object)| LivePlacement {
                index: Some(i as u32),
                id: object.id,
                condition: ((object.flags >> 16) & 0xFF) as u8,
                rows: object.transform,
            })
            .collect();
        let moved = live[0].index.unwrap();
        live[0].rows[3][1] += 0.5;
        live.push(LivePlacement {
            index: None,
            id: 0x8000_0003,
            condition: 0x40,
            rows: [
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [0.0, 0.0, 1.0],
                [1.0, 2.0, 3.0],
            ],
        });
        // The live local tier: the level's own with one record dropped.
        let mut local = scene.local_database.clone().unwrap();
        let last = local.entries().last().unwrap().id;
        let at = local.find_index(last).unwrap();
        local.store(at, &[]);
        let records = local.write();
        let strings = scene.local_strings.clone().unwrap_or_default().write();
        let (saved, counts) = saved_section(
            source,
            &spawned,
            &live,
            records.clone(),
            strings,
            true,
            SettingsState::reset(),
            None,
        )
        .unwrap();
        assert_eq!(counts.added, 1);
        assert!(counts.removed >= 1);
        let back = Scene::parse(&saved).unwrap();
        assert!(back.level_errors.is_empty(), "{:?}", back.level_errors);
        assert_eq!(back.local_database.unwrap().write(), records);
        let kept = back.placed_objects.len();
        assert_eq!(kept, scene.placed_objects.len() - counts.removed + 1);
        let dropped = back.placed_objects.last().unwrap();
        assert_eq!(dropped.id, 0x8000_0003);
        assert!((dropped.transform[3][2] - 3.0).abs() < 1e-4);
        assert_eq!((dropped.flags >> 16) & 0xFF, 0x40);
        let moved_before = &scene.placed_objects[moved as usize];
        assert!(
            back.placed_objects
                .iter()
                .any(|object| object.id == moved_before.id
                    && (object.transform[3][1] - moved_before.transform[3][1] - 0.5).abs() < 1e-3)
        );
    }

    fn stored(id: u32, x: f32) -> Placement {
        Placement {
            id,
            position: [x, 0.0, 0.0],
            yaw: 0.5,
            condition: 0,
            pose: exa_formats::characters::SavedPose::None,
            motion: None,
            ai_home: None,
        }
    }

    fn standing(id: u32, x: f32, dead: bool) -> LiveCharacter {
        LiveCharacter {
            id,
            position: [x, 0.0, 0.0],
            yaw: 1.0,
            dead,
        }
    }

    /// An idling character keeps its placement, a moved one takes its live
    /// place, one who left the level's store leaves, one never spawned
    /// stays, a death is written and an arrival with a world id is added.
    #[test]
    fn the_live_scene_rewrites_the_character_placements() {
        let placements = [
            stored(4, 1.0), // idles
            stored(5, 2.0), // walks 3 m
            stored(6, 3.0), // left with the player
            stored(7, 4.0), // never built
            stored(8, 5.0), // died where it stood
            stored(2, 6.0), // a level record 2: not the world follower 2
        ];
        let live = [
            standing(4, 1.05, false),
            standing(5, 5.0, false),
            standing(8, 5.0, true),
            standing(2, 6.0, false),
            standing(WORLD_DB | 2, 9.0, false),
        ];
        let held = [4, 5, 7, 8, 2];
        let (out, counts) = characters_after(&placements, &live, |id| !held.contains(&id));
        // A stored corpse keeps its placement whatever its node says.
        let mut corpse = stored(9, 1.0);
        corpse.condition = exa_formats::characters::DEAD;
        let (kept, _) = characters_after(
            std::slice::from_ref(&corpse),
            &[standing(9, 4.0, true)],
            |_| false,
        );
        assert_eq!(kept, [corpse]);
        assert_eq!(
            counts,
            CharacterCounts {
                kept: 4,
                moved: 2,
                removed: 1,
                unspawned: 1,
                added: 1
            }
        );
        let ids: Vec<u32> = out.iter().map(|p| p.id).collect();
        assert_eq!(ids, [4, 5, 7, 8, 2, WORLD_DB | 2]);
        assert_eq!((out[0].position[0], out[0].yaw), (1.0, 0.5));
        assert_eq!((out[1].position[0], out[1].yaw), (5.0, 1.0));
        assert!(out[3].is_dead());
        assert_eq!(out[4].position[0], 6.0);
        assert_eq!(out[5].position, [9.0, 0.0, 0.0]);
        assert_eq!(out[5].condition, 0);
    }

    /// Derrin carried into exanima02 under world id 2 is written into the
    /// saved section's placements beside the level's own characters, and
    /// the section placed again finds him in the world store at level 3
    /// (skipped without the game).
    #[test]
    fn a_carried_follower_is_written_into_the_saved_section() {
        let Some(dir) = exa_assets::locate_install(None) else {
            return;
        };
        let game = exa_assets::GameData::open(&dir).unwrap();
        let pack = game.pack(exa_assets::PackKind::Resource).unwrap();
        let source = pack.get("exanima02.rfc").unwrap();
        let scene = Scene::parse(source).unwrap();
        let chars = scene.characters.clone().unwrap();
        let mut local = crate::world_characters::LocalCharacters::of_section(source);
        assert!(!local.db.is_empty());
        let mut derrin = exa_formats::characters::CharacterRecord::reset();
        derrin.id = WORLD_DB | 2;
        derrin.name = "Derrin".into();
        derrin.locale = 3;
        let live = [LiveCharacter {
            id: WORLD_DB | 2,
            position: [1.0, 0.5, 2.0],
            yaw: 0.25,
            dead: false,
        }];
        let held: Vec<u32> = local.db.ids().collect();
        let records = scene
            .local_database
            .clone()
            .map(|db| db.write())
            .unwrap_or_default();
        let strings = scene.local_strings.clone().unwrap_or_default().write();
        let world = [derrin.clone()];
        let (saved, _) = saved_section(
            source,
            &[],
            &[],
            records,
            strings,
            true,
            SettingsState::reset(),
            Some(SectionCharacters {
                live: &live,
                held: &held,
                world: &world,
                database: local.section_body(),
            }),
        )
        .unwrap();
        let back = Scene::parse(&saved).unwrap();
        let back_chars = back.characters.as_ref().unwrap();
        // Nobody of the level's stood in the scene, so all are kept.
        assert_eq!(back_chars.placements.len(), chars.placements.len() + 1);
        assert_eq!(back_chars.records.len(), chars.records.len());
        let last = back_chars.placements.last().unwrap();
        assert_eq!(last.id, WORLD_DB | 2);
        assert!((last.position[2] - 2.0).abs() < 1e-3);
        let placed = exa_world::placed_characters(&back, &[], Some(3), &world);
        assert!(placed.iter().any(|c| c.name == "Derrin"));
        // Elsewhere (level 2) the locale gate keeps him out.
        let elsewhere = exa_world::placed_characters(&back, &[], Some(2), &world);
        assert!(!elsewhere.iter().any(|c| c.name == "Derrin"));
    }

    #[test]
    fn a_world_transform_gives_its_rows() {
        let affine = Affine3A::from_rotation_translation(
            Quat::from_rotation_y(std::f32::consts::FRAC_PI_2),
            Vec3::new(1.0, 2.0, 3.0),
        );
        let rows = rows_of(affine);
        let back = exa_assets::model::transform_from_rows(&rows);
        assert!(back.translation.abs_diff_eq(Vec3::new(1.0, 2.0, 3.0), 1e-6));
        let turned = back.rotation * Vec3::X;
        assert!(
            turned.abs_diff_eq(affine.matrix3 * Vec3::X, 1e-5),
            "{turned}"
        );
    }
}
