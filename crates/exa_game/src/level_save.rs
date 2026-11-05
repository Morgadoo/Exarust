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
//! at the end. [Exarust's: the props, the doors' and levers' positions, the
//! scene characters and the settings chunk's live words are the loaded
//! section's.]
//!
//! Evidence: the save unit's save routine (the section order, the
//! exporter's call and the locale name) and the exporter's scene walk and
//! local-database chunk (`docs/formats/SAVES.md`, Saving; `RFC.md`).

use bevy::math::Affine3A;
use bevy::prelude::*;
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

/// The level section a save writes: `source` (the section the level was
/// read from) with the live placed objects and the live local tier
/// (`records`, the tier's writer, and `strings`, its library's) written
/// over it, exported with the save switches ([`export_level_scene`]).
pub fn saved_section(
    source: &[u8],
    spawned: &[u32],
    live: &[LivePlacement],
    records: Vec<u8>,
    strings: Vec<u8>,
    story: bool,
    settings: SettingsState,
) -> exa_formats::Result<(Vec<u8>, PlacementCounts)> {
    let mut scene = Scene::parse(source)?;
    let mut raw = LevelRaw::scan(source)?;
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
}

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

    /// The section the save writes for the running level, named by its
    /// locale; `None` without a loaded level or when it does not export.
    pub fn section(
        &self,
        db: &exa_formats::objdb::ObjectDb,
        story: bool,
    ) -> Option<(String, Vec<u8>)> {
        let source = self.source.as_deref()?;
        let live = self.live_placements();
        let records = db.write(exa_formats::objdb::Tier::Local);
        let strings = db.library(exa_formats::objdb::Tier::Local).write();
        match saved_section(
            &source.bytes,
            &source.spawned,
            &live,
            records,
            strings,
            story,
            SettingsState::reset(),
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
