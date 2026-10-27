//! Tests of the saved-pose placement: the bone tree's nearest-child match,
//! the skipped subtree, the flat slots, and the shipped corpses.

use bevy::math::Vec3;
use bevy::prelude::Entity;
use exa_formats::characters::{PoseRecord, SavedPose};
use exa_formats::morph::Frame;
use exa_formats::rsq::bone_index;
use exa_physics::body::{PartSetup, Ragdoll, Sleep};

use super::*;

/// No level geometry: the ship tests only need the countdown.
const NONE: &[[Vec3; 3]] = &[];

fn translated(t: [f32; 3]) -> Frame {
    [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0], t]
}

fn part(bone: usize, parent: Option<usize>, local: Frame) -> PoseNode {
    PoseNode {
        bone: Some(bone),
        parent,
        local,
    }
}

fn tree(frame: Frame, children: u32) -> PoseRecord {
    PoseRecord {
        frame,
        word: 0,
        children,
    }
}

fn close(a: [f32; 3], b: [f32; 3], tolerance: f32) -> bool {
    (0..3).all(|k| (a[k] - b[k]).abs() <= tolerance)
}

/// A record within 20 cm of the parent's child goes to that child, with the
/// record's origin in the parent's frame; a record just past 20 cm finds no
/// child and skips its whole subtree, whose nodes keep their built pose.
#[test]
fn a_tree_record_matches_the_nearest_child_within_twenty_centimetres() {
    let nodes = [
        part(1, None, translated([0.0, 0.0, 0.0])),
        part(8, Some(0), translated([0.0, 1.0, 0.0])),
        part(9, Some(1), translated([0.0, 0.2, 0.0])),
        part(2, Some(0), translated([0.5, -1.0, 0.0])),
        part(3, Some(3), translated([0.0, -0.4, 0.0])),
    ];
    let pose = SavedPose::Tree(vec![
        tree(translated([10.0, 20.0, 0.0]), 2),
        tree(translated([10.0, 21.0, 0.0]), 1),
        tree(translated([10.0, 21.19, 0.0]), 0),
        tree(translated([10.5, 19.0, 0.0]), 1),
        tree(translated([10.5, 18.55, 0.0]), 0),
    ]);
    let placed = place(&nodes, &pose);
    assert_eq!(placed.frame, [10.0, 20.0, 0.0]);
    assert!(close(placed.worlds[1][3], [10.0, 21.0, 0.0], 1e-6));
    assert!(close(placed.worlds[2][3], [10.0, 21.19, 0.0], 1e-6));
    assert!(close(placed.worlds[3][3], [10.5, 19.0, 0.0], 1e-6));
    assert!(close(placed.worlds[4][3], [10.5, 18.55, 0.0], 1e-6));
    // The root's local frame is its rotation only (the frame carries its
    // origin); the others are relative to their parent part.
    assert!(close(placed.locals[0][3], [0.0, 0.0, 0.0], 1e-6));
    assert!(close(placed.locals[1][3], [0.0, 1.0, 0.0], 1e-6));
    assert!(close(placed.locals[2][3], [0.0, 0.19, 0.0], 1e-6));
    assert!(close(placed.locals[3][3], [0.5, -1.0, 0.0], 1e-6));
    assert!(close(placed.locals[4][3], [0.0, -0.45, 0.0], 1e-6));
}

/// The boundary is strictly under 400 (20 cm): 19.99 cm matches, 20.01 cm
/// does not, and the grandchild of the unmatched record is skipped too.
#[test]
fn a_tree_record_at_twenty_centimetres_does_not_match() {
    let nodes = [
        part(1, None, translated([0.0, 0.0, 0.0])),
        part(8, Some(0), translated([0.0, 0.2, 0.0])),
        part(9, Some(1), translated([0.0, 0.1, 0.0])),
    ];
    let near = SavedPose::Tree(vec![
        tree(translated([0.0, 20.0, 0.0]), 1),
        tree(translated([0.0, 20.3999, 0.0]), 1),
        tree(translated([0.0, 20.4999, 0.0]), 0),
    ]);
    let placed = place(&nodes, &near);
    assert!(close(placed.worlds[1][3], [0.0, 20.3999, 0.0], 1e-6));
    assert!(close(placed.worlds[2][3], [0.0, 20.4999, 0.0], 1e-6));

    let far = SavedPose::Tree(vec![
        tree(translated([0.0, 20.0, 0.0]), 1),
        tree(translated([0.0, 20.4001, 0.0]), 1),
        tree(translated([0.0, 20.5, 0.0]), 0),
    ]);
    let placed = place(&nodes, &far);
    assert_eq!(placed.worlds[1], translated([0.0, 0.2, 0.0]));
    assert_eq!(placed.worlds[2], translated([0.0, 0.3, 0.0]));
}

/// Flat slots go by the saved slot table: slot 1 is the pelvis, slot 8 the
/// head, slots 9 to 12 the left arm, slot 13 the right thigh, and slots 19
/// and 20 are never read.
#[test]
fn flat_slots_map_by_the_saved_part_table() {
    let parents = [
        (1, None),
        (8, Some(0)),
        (9, Some(1)),
        (11, Some(2)),
        (12, Some(3)),
        (13, Some(4)),
        (17, Some(5)),
        (14, Some(2)),
        (15, Some(7)),
        (16, Some(8)),
        (18, Some(9)),
        (10, Some(2)),
        (2, Some(0)),
        (3, Some(11)),
        (4, Some(12)),
        (5, Some(0)),
        (6, Some(14)),
        (7, Some(15)),
    ];
    let nodes: Vec<PoseNode> = parents
        .iter()
        .map(|&(bone, parent)| part(bone, parent, translated([bone as f32, 0.0, 0.0])))
        .collect();
    let mut frames: Vec<Frame> = FLAT_SLOT_BONES
        .iter()
        .enumerate()
        .map(|(slot, &bone)| translated([(slot + 1) as f32 * 10.0, bone as f32 * 100.0, 0.0]))
        .collect();
    // The two inert slots the shipped files carry.
    frames.push(translated([900.0, 900.0, 900.0]));
    frames.push(translated([901.0, 901.0, 901.0]));
    let placed = place(&nodes, &SavedPose::Flat(frames));
    assert_eq!(placed.frame, [10.0, 100.0, 0.0]);
    for (slot, &bone) in FLAT_SLOT_BONES.iter().enumerate() {
        let i = nodes.iter().position(|n| n.bone == Some(bone)).unwrap();
        let want = [((slot + 1) as f32) * 10.0, bone as f32 * 100.0, 0.0];
        assert!(
            close(placed.worlds[i][3], want, 1e-5),
            "slot {} bone {bone}: {:?}",
            slot + 1,
            placed.worlds[i][3]
        );
    }
    // The head is slot 8, not the model's eighth part.
    let head = nodes.iter().position(|n| n.bone == Some(10)).unwrap();
    assert!(close(placed.worlds[head][3], [80.0, 1000.0, 0.0], 1e-5));
    // The pelvis's local frame is its rotation only.
    assert!(close(placed.locals[0][3], [0.0, 0.0, 0.0], 1e-6));
    let belly = nodes.iter().position(|n| n.bone == Some(8)).unwrap();
    assert!(close(placed.locals[belly][3], [10.0, 700.0, 0.0], 1e-5));
}

/// The installed `exanima05_sw` corpses carry the legacy flat pose; mapped
/// by the saved slot table, every one has the body's rigid distances: the
/// pelvis 8.8 cm under the belly, the shoulders 7.6 cm apart and the thigh
/// 43.3 cm (skipped without the game).
#[test]
fn shipped_flat_corpses_map_by_the_slot_table() {
    let Some(dir) = exa_assets::locate_install(None) else {
        return;
    };
    let game = exa_assets::GameData::open(&dir).unwrap();
    let resource = game.pack(exa_assets::PackKind::Resource).unwrap();
    let level = exa_formats::rfc::Scene::parse(resource.get("exanima05_sw.rfc").unwrap()).unwrap();
    let characters = level.characters.as_ref().unwrap();
    let model = exa_formats::rfc::Scene::parse(resource.get("humanbase.rfc").unwrap()).unwrap();
    let nodes: Vec<PoseNode> = model
        .nodes
        .iter()
        .map(|n| PoseNode {
            bone: bone_index(&n.name).filter(|&b| b >= 1),
            parent: n.parent.map(|p| p - 1),
            local: n.transform,
        })
        .collect();
    let bone = |placed: &PlacedPose, bone: usize| {
        let i = nodes.iter().position(|n| n.bone == Some(bone)).unwrap();
        Vec3::from_array(placed.worlds[i][3])
    };
    let mut corpses = 0;
    for placement in characters.placements.iter().filter(|p| p.is_dead()) {
        assert!(matches!(placement.pose, SavedPose::Flat(_)));
        let placed = place(&nodes, &placement.pose);
        let d = |a: usize, b: usize| (bone(&placed, a) - bone(&placed, b)).length() * 100.0;
        let pelvis_belly = d(1, 8);
        let shoulders = d(11, 14);
        let thigh = d(2, 3);
        assert!(
            (7.5..=10.0).contains(&pelvis_belly),
            "{:?}: pelvis-belly {pelvis_belly}",
            placement.id
        );
        assert!(
            (7.0..=8.2).contains(&shoulders),
            "{:?}: shoulders {shoulders}",
            placement.id
        );
        assert!(
            (42.5..=44.0).contains(&thigh),
            "{:?}: thigh {thigh}",
            placement.id
        );
        corpses += 1;
    }
    assert_eq!(corpses, 13);
}

/// The installed `exanima01` corpses are placed on their saved frames, and
/// each body runs one tick before it sleeps (skipped without the game).
#[test]
fn shipped_corpses_are_placed_and_sleep() {
    let Some(dir) = exa_assets::locate_install(None) else {
        return;
    };
    let game = exa_assets::GameData::open(&dir).unwrap();
    let resource = game.pack(exa_assets::PackKind::Resource).unwrap();
    let level = exa_formats::rfc::Scene::parse(resource.get("exanima01.rfc").unwrap()).unwrap();
    let characters = level.characters.as_ref().unwrap();
    let races = exa_formats::races::parse(resource.get("races.rdb").unwrap()).unwrap();
    let mut corpses = 0;
    for placement in characters.placements.iter().filter(|p| p.is_dead()) {
        let record = characters.record(placement.id).unwrap();
        let race = exa_formats::races::of_character(&races, record.race).unwrap();
        let model = exa_formats::rfc::Scene::parse(resource.get(&race.model()).unwrap()).unwrap();
        let nodes: Vec<PoseNode> = model
            .nodes
            .iter()
            .map(|n| PoseNode {
                bone: bone_index(&n.name).filter(|&b| b >= 1),
                parent: n.parent.map(|p| p - 1),
                local: n.transform,
            })
            .collect();
        let placed = place(&nodes, &placement.pose);
        // Every record's bone sits on its saved world frame (the shipped
        // words are the bone numbers, as the notes say).
        let saved: Vec<(usize, Frame)> = match &placement.pose {
            SavedPose::Tree(records) => records
                .iter()
                .filter(|r| (1..=18).contains(&r.word))
                .map(|r| (r.word as usize, r.frame))
                .collect(),
            SavedPose::Flat(frames) => FLAT_SLOT_BONES
                .iter()
                .copied()
                .zip(frames.iter().copied())
                .collect(),
            SavedPose::None => Vec::new(),
        };
        assert!(saved.len() >= 18, "{saved:?}");
        for (bone, frame) in &saved {
            let Some(i) = nodes.iter().position(|n| n.bone == Some(*bone)) else {
                continue;
            };
            assert!(
                close(placed.worlds[i][3], frame[3], 0.02),
                "{:?} bone {bone}: {:?} against {:?}",
                placement.id,
                placed.worlds[i][3],
                frame[3]
            );
            for (row, want_row) in frame.iter().enumerate().take(3) {
                for (col, want) in want_row.iter().enumerate() {
                    assert!(
                        (placed.worlds[i][row][col] - want).abs() < 1e-4,
                        "{:?} bone {bone} row {row}",
                        placement.id
                    );
                }
            }
        }
        // A ragdoll built where the pose left the parts runs one tick and
        // sleeps in that tick's post-step; the next tick is deep sleep.
        let parts: Vec<usize> = (0..nodes.len())
            .filter(|&i| nodes[i].bone.is_some())
            .collect();
        let part_of: Vec<Option<usize>> = {
            let mut out = vec![None; nodes.len()];
            for (k, &i) in parts.iter().enumerate() {
                out[i] = Some(k);
            }
            out
        };
        let parent_part: Vec<Option<usize>> = parts
            .iter()
            .map(|&i| {
                let mut p = nodes[i].parent;
                while let Some(j) = p {
                    if let Some(k) = part_of.get(j).copied().flatten() {
                        return Some(k);
                    }
                    p = nodes.get(j).and_then(|n| n.parent);
                }
                None
            })
            .collect();
        let setup: Vec<PartSetup> = parts
            .iter()
            .enumerate()
            .map(|(k, &i)| PartSetup {
                bone: nodes[i].bone.unwrap(),
                entity: Entity::PLACEHOLDER,
                parent: parent_part[k],
                world: affine(&placed.worlds[i]),
                centre: model.nodes[i].bounds_centre.into(),
                half: model.nodes[i].bounds_half_extents.into(),
                origin: placed.locals[i][3].into(),
                links: Vec::new(),
                foot: None,
                pins: Vec::new(),
                spheres: None,
            })
            .collect();
        let mut body = Ragdoll::new(&setup);
        body.register_placed();
        assert_eq!(body.countdown(), 0);
        body.tick(NONE);
        assert!(body.asleep(), "{:?}", placement.id);
        body.tick(NONE);
        assert_eq!(body.sleep, Sleep::Deep, "{:?}", placement.id);
        corpses += 1;
    }
    assert_eq!(corpses, 7);
}
