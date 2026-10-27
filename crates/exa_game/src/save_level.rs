//! Level loading, spawn placement and follower migration as the save unit
//! performs them.
//!
//! The save unit owns the level-to-level transition. It flushes the scene,
//! lists the characters that should travel with the player, places the
//! player on the destination's floor and puts the followers around them on
//! the offsets the original stores. The world itself belongs to other
//! units; the rules here are plain functions with the original's numbers so
//! the transition can call them when its scene exists.
//!
//! Evidence: the placement, migration, re-attach, menu-scene, terrain and
//! scene-content routines of the save unit, read from the instructions
//! (`docs/formats/SAVES.md`, Level transitions).

use crate::flow::rotation;

/// The height the original gives a body's feet above the floor, before the
/// body's own extent is taken off: 108.18 cm minus eleven point one times
/// the body's extent word.
pub const SPAWN_HEIGHT: f64 = 108.18;
/// The extent scale in [`SPAWN_HEIGHT`].
pub const SPAWN_EXTENT_SCALE: f64 = 11.1;
/// Extra lift for a body in its raised state.
pub const SPAWN_LIFT_RAISED: f32 = 13.0;
/// Extra lift for every other body.
pub const SPAWN_LIFT: f32 = 5.0;
/// The floor probe's lift before it is cast down.
pub const SPAWN_PROBE_LIFT: f32 = 100.0;
/// The floor probe's reach.
pub const SPAWN_PROBE_REACH: f32 = 300.0;

/// Where the player is put on a newly loaded level: the matrix's x and z,
/// the floor the probe found, and the height the original lifts the body
/// by. `extent` is the body's extent word; `raised` selects the 13 cm lift
/// (the original's per-body flag) instead of 5 cm.
pub fn spawn_height(extent: f32, floor: f32, raised: bool) -> f32 {
    let offset = SPAWN_HEIGHT - f64::from(extent) * SPAWN_EXTENT_SCALE;
    floor
        + offset as f32
        + if raised {
            SPAWN_LIFT_RAISED
        } else {
            SPAWN_LIFT
        }
}

/// The placement the player gets on a new level, from the saved matrix's
/// planar position, the probed floor and the body's extent.
pub fn spawn_placement(position: [f32; 3], extent: f32, floor: f32, raised: bool) -> [f32; 3] {
    [
        position[0],
        spawn_height(extent, floor, raised),
        position[2],
    ]
}

/// The six offsets followers take around the player, in the original's
/// order: two in front, one behind and two at the sides. The last two
/// entries are zero and are only reached when more followers than offsets
/// travel.
pub const FOLLOWER_OFFSETS: [[f32; 3]; 6] = [
    [-100.0, 0.0, -100.0],
    [100.0, 0.0, -100.0],
    [0.0, 0.0, 150.0],
    [100.0, 0.0, 200.0],
    [0.0, 0.0, 0.0],
    [0.0, 0.0, 0.0],
];

/// Rotates a planar offset by the player's heading and adds it to the
/// player's placement. The original builds the rotation from the player's
/// frame and adds the offset to its position.
pub fn follower_position(player: [f32; 3], heading: [[f32; 3]; 3], offset: [f32; 3]) -> [f32; 3] {
    let moved = apply(heading, offset);
    [
        player[0] + moved[0],
        player[1] + moved[1],
        player[2] + moved[2],
    ]
}

/// All follower placements around a player, one per offset, in order.
pub fn follower_positions(player: [f32; 3], heading: [[f32; 3]; 3], count: usize) -> Vec<[f32; 3]> {
    (0..count.min(FOLLOWER_OFFSETS.len()))
        .map(|index| follower_position(player, heading, FOLLOWER_OFFSETS[index]))
        .collect()
}

fn apply(m: [[f32; 3]; 3], v: [f32; 3]) -> [f32; 3] {
    [
        m[0][0] * v[0] + m[0][1] * v[1] + m[0][2] * v[2],
        m[1][0] * v[0] + m[1][1] * v[1] + m[1][2] * v[2],
        m[2][0] * v[0] + m[2][1] * v[1] + m[2][2] * v[2],
    ]
}

/// A character the scene offers the transition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SceneMotile<'a> {
    /// The character's identity in the scene.
    pub address: usize,
    /// Its name, for the duplicate warning.
    pub name: &'a str,
    /// Its controller state word; only 4 travels.
    pub controller: i32,
    /// Its flags; bit 0x800 keeps it behind.
    pub flags: u32,
}

/// One character on the migration list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Migration {
    pub address: usize,
    /// A repeated address is logged as "Duplicate simulation motile: <name>"
    /// under the "Migration" tag and is not added twice.
    pub duplicate: bool,
}

/// Builds the migration list: every character whose controller is 4 and
/// whose flags do not carry 0x800, in scene order, each address once. A
/// repeated address is reported as a duplicate and skipped.
pub fn migrations(motiles: &[SceneMotile<'_>]) -> Vec<Migration> {
    let mut out: Vec<Migration> = Vec::new();
    for motile in motiles {
        if motile.controller != 4 || motile.flags & 0x800 != 0 {
            continue;
        }
        if out.iter().any(|seen| seen.address == motile.address) {
            out.push(Migration {
                address: motile.address,
                duplicate: true,
            });
            continue;
        }
        out.push(Migration {
            address: motile.address,
            duplicate: false,
        });
    }
    out
}

/// The state a migrated follower is re-attached with.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FollowerReset {
    /// The UID manager is freed and forgotten.
    pub uid_cleared: bool,
    /// The player controls are re-created for the follower.
    pub controls: bool,
    /// The character's carry weight is reset to zero.
    pub carry: f32,
    /// The body's controller word becomes 1.
    pub controller: u32,
    /// The body's minimum distance is at least 5 cm.
    pub min_distance: f32,
}

/// The re-attach of one migrated follower: its UID, controls, carry word,
/// controller and minimum distance. The original brings a distance under
/// 5 cm up to 5 and leaves a NaN alone (the unordered compare does not
/// branch).
pub fn reattach_follower(controller: u32, min_distance: f32) -> FollowerReset {
    FollowerReset {
        uid_cleared: true,
        controls: true,
        carry: 0.0,
        controller: controller.max(1),
        min_distance: if min_distance < 5.0 {
            5.0
        } else {
            min_distance
        },
    }
}

/// The menu background scene the original loads behind the main menu: the
/// shipped `menuscene.rfc` with the 1.8 fog preset, the scene editor at
/// mode zero, the view at (0, 500, 700) and its target 200 cm down the
/// z axis, 60 cm ahead.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MenuScene {
    pub file: &'static str,
    pub fog_distance: f32,
    pub view: [f32; 3],
    pub target: [f32; 3],
    pub camera_distance: f32,
}

/// The menu scene's values.
pub fn menu_scene() -> MenuScene {
    MenuScene {
        file: "menuscene.rfc",
        fog_distance: 1.8,
        view: [0.0, 500.0, 700.0],
        target: [0.0, 0.0, -200.0],
        camera_distance: 60.0,
    }
}

/// The terrain file that goes with a saved level's locale: its extension
/// replaced by `.rft`.
pub fn terrain_file_name(locale: &str) -> String {
    exa_formats::text::strip_extension_append(locale, ".rft")
}

/// The list a scene node joins when the save unit classifies it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SceneContent {
    /// An object record whose kind byte pair is `0x1100` or `0x1200`.
    First,
    /// A node whose name starts with `[`, or an object record whose kind
    /// byte pair is `0x2200` (a guess at the shared list's meaning).
    Second,
    /// An object whose kind word is `0x10000` and whose kind byte is 2.
    Third,
}

/// Classifies one scene node from its name and, when the object database
/// knows it, its record's kind flags and its `0x10000` property's kind
/// byte. `None` is a node the save unit leaves out of all three lists.
///
/// The original tests the object archive first; only when the record is
/// missing does it look at the `[` name prefix.
pub fn scene_content(
    name: &str,
    object_flags: Option<u32>,
    object_kind_byte: Option<u8>,
) -> Option<SceneContent> {
    if let Some(flags) = object_flags {
        let pair = flags & 0xff00;
        if pair == 0x1100 || pair == 0x1200 {
            return Some(SceneContent::First);
        }
        if pair == 0x2200 {
            return Some(SceneContent::Second);
        }
        if object_kind_byte == Some(2) {
            return Some(SceneContent::Third);
        }
        return None;
    }
    name.as_bytes()
        .first()
        .is_some_and(|first| *first == b'[')
        .then_some(SceneContent::Second)
}

/// A heading matrix for a yaw in radians, as the follower placement needs
/// it. The engine's rotation builder takes degrees.
pub fn heading(yaw: f32) -> [[f32; 3]; 3] {
    rotation(0.0, yaw.to_degrees(), 0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_spawn_height_subtracts_eleven_point_one_per_extent() {
        // 108.18 − 1·11.1 + 5.
        assert!((spawn_height(1.0, 0.0, false) - 102.08).abs() < 1e-4);
        // 108.18 − 1·11.1 + 13.
        assert!((spawn_height(1.0, 0.0, true) - 110.08).abs() < 1e-4);
        // The extent is widened to double before the product.
        assert!((spawn_height(0.5, 40.0, false) - 147.63).abs() < 1e-4);
        let placed = spawn_placement([1.0, 999.0, 2.0], 1.0, 0.0, false);
        assert_eq!(placed[0], 1.0);
        assert_eq!(placed[2], 2.0);
        assert!((placed[1] - 102.08).abs() < 1e-4);
    }

    #[test]
    fn followers_take_the_stored_offsets_rotated_by_the_heading() {
        let yaw = std::f32::consts::FRAC_PI_2;
        let m = heading(yaw);
        // A quarter turn sends the local −100 x offset to world −100 z.
        let front = follower_position([10.0, 0.0, 20.0], m, FOLLOWER_OFFSETS[0]);
        assert!((front[0] - 110.0).abs() < 1e-3);
        assert!((front[2] + 80.0).abs() < 1e-3);
        let all = follower_positions(
            [0.0; 3],
            [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
            3,
        );
        assert_eq!(
            all,
            vec![
                [-100.0, 0.0, -100.0],
                [100.0, 0.0, -100.0],
                [0.0, 0.0, 150.0]
            ]
        );
        assert_eq!(follower_positions([0.0; 3], m, 99).len(), 6);
    }

    #[test]
    fn migration_keeps_travelling_characters_once_and_flags_repeats() {
        let motiles = [
            SceneMotile {
                address: 0x10,
                name: "Ann",
                controller: 4,
                flags: 0,
            },
            SceneMotile {
                address: 0x20,
                name: "Bob",
                controller: 3,
                flags: 0,
            },
            SceneMotile {
                address: 0x30,
                name: "Cat",
                controller: 4,
                flags: 0x800,
            },
            SceneMotile {
                address: 0x10,
                name: "Ann",
                controller: 4,
                flags: 0,
            },
            SceneMotile {
                address: 0x40,
                name: "Dan",
                controller: 4,
                flags: 0,
            },
        ];
        let list = migrations(&motiles);
        assert_eq!(
            list,
            vec![
                Migration {
                    address: 0x10,
                    duplicate: false
                },
                Migration {
                    address: 0x10,
                    duplicate: true
                },
                Migration {
                    address: 0x40,
                    duplicate: false
                },
            ]
        );
    }

    #[test]
    fn a_reattached_follower_gets_its_distance_floor() {
        let reset = reattach_follower(0, 1.0);
        assert_eq!(reset.carry, 0.0);
        assert_eq!(reset.min_distance, 5.0);
        assert_eq!(reset.controller, 1);
        assert!(reset.controls && reset.uid_cleared);
        // NaN does not compare below 5 and is kept.
        assert!(reattach_follower(2, f32::NAN).min_distance.is_nan());
        assert_eq!(reattach_follower(7, 8.0).min_distance, 8.0);
    }

    #[test]
    fn the_menu_scene_has_the_original_values() {
        let scene = menu_scene();
        assert_eq!(scene.file, "menuscene.rfc");
        assert_eq!(scene.fog_distance, 1.8);
        assert_eq!(scene.view, [0.0, 500.0, 700.0]);
        assert_eq!(scene.target, [0.0, 0.0, -200.0]);
        assert_eq!(scene.camera_distance, 60.0);
    }

    #[test]
    fn the_terrain_file_replaces_the_locale_extension() {
        assert_eq!(terrain_file_name("exanima04"), "exanima04.rft");
        assert_eq!(terrain_file_name("arenao1.rfc"), "arenao1.rft");
    }

    #[test]
    fn scene_content_follows_the_object_kind_then_the_name() {
        assert_eq!(
            scene_content("door", Some(0x1100), None),
            Some(SceneContent::First)
        );
        assert_eq!(
            scene_content("lever", Some(0x12ff), None),
            Some(SceneContent::First)
        );
        assert_eq!(
            scene_content("x", Some(0x2200), None),
            Some(SceneContent::Second)
        );
        assert_eq!(
            scene_content("x", Some(0x0100), Some(2)),
            Some(SceneContent::Third)
        );
        assert_eq!(scene_content("[trigger]", Some(0x0100), None), None);
        assert_eq!(
            scene_content("[trigger]", None, None),
            Some(SceneContent::Second)
        );
        assert_eq!(scene_content("plain", None, None), None);
    }
}
