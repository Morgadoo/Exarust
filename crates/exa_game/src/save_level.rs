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
/// order: two at the front corners, one behind, one at a back corner, and
/// two zero entries that put a fifth and sixth follower on the player's own
/// spot. The original does not bound the index; a seventh follower would
/// read past the table, so Exarust stops at six (a deliberate divergence).
pub const FOLLOWER_OFFSETS: [[f32; 3]; 6] = [
    [-100.0, 0.0, -100.0],
    [100.0, 0.0, -100.0],
    [0.0, 0.0, 150.0],
    [100.0, 0.0, 200.0],
    [0.0, 0.0, 0.0],
    [0.0, 0.0, 0.0],
];

/// Rotates an offset by the spawn matrix's rotation rows (the offset as a
/// row vector: x times the first row plus y times the second, then z times
/// the third) and adds the player's placed position. The result is the
/// matrix position the follower is then spawned at; its own floor probe
/// and lift ([`spawn_placement`]) follow.
pub fn follower_position(player: [f32; 3], frame: [[f32; 3]; 3], offset: [f32; 3]) -> [f32; 3] {
    let moved = apply(frame, offset);
    [
        player[0] + moved[0],
        player[1] + moved[1],
        player[2] + moved[2],
    ]
}

/// All follower placements around a player, one per offset, in order.
pub fn follower_positions(player: [f32; 3], frame: [[f32; 3]; 3], count: usize) -> Vec<[f32; 3]> {
    (0..count.min(FOLLOWER_OFFSETS.len()))
        .map(|index| follower_position(player, frame, FOLLOWER_OFFSETS[index]))
        .collect()
}

/// A row vector times the matrix, summed first row, second, then third.
fn apply(m: [[f32; 3]; 3], v: [f32; 3]) -> [f32; 3] {
    let column = |j: usize| (v[0] * m[0][j] + v[1] * m[1][j]) + v[2] * m[2][j];
    [column(0), column(1), column(2)]
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

/// One character the migration pass met.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Migration {
    pub address: usize,
    /// A repeated address is logged as "Duplicate simulation motile: <name>"
    /// under the "Migration" tag and is not added to the list (nor flushed)
    /// again; callers skip the entries marked here.
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
    /// The body's controller word: always set to 1, whatever it was.
    pub controller: u32,
    /// The body's minimum distance is at least 5 cm.
    pub min_distance: f32,
}

/// The re-attach of one migrated follower: its UID, controls, carry word,
/// controller (stored as 1 unconditionally) and minimum distance. The
/// original brings a distance under 5 cm up to 5 and leaves a NaN alone
/// (the unordered compare does not branch). The pass then empties the
/// migration list.
pub fn reattach_follower(min_distance: f32) -> FollowerReset {
    FollowerReset {
        uid_cleared: true,
        controls: true,
        carry: 0.0,
        controller: 1,
        min_distance: if min_distance < 5.0 {
            5.0
        } else {
            min_distance
        },
    }
}

/// The menu background scene the original loads behind the main menu: the
/// shipped `menuscene.rfc` with the 1.8 fog preset, the scene editor at
/// mode zero, the view at (0, 500, 700), its target at (0, 0, −200), and a
/// 60 written into the camera beforehand (most likely its field of view in
/// degrees; a guess, the field is only stored).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MenuScene {
    pub file: &'static str,
    pub fog_distance: f32,
    pub view: [f32; 3],
    pub target: [f32; 3],
    pub camera_field: f32,
}

/// The menu scene's values.
pub fn menu_scene() -> MenuScene {
    MenuScene {
        file: "menuscene.rfc",
        fog_distance: 1.8,
        view: [0.0, 500.0, 700.0],
        target: [0.0, 0.0, -200.0],
        camera_field: 60.0,
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
    /// An object whose `0x10000` property's first byte is 2.
    Third,
}

/// What the object database knows about a scene node's record.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ObjectKinds {
    /// The record's kind word (its `0xff00` byte pair is tested), when the
    /// record carries the kind property.
    pub flags: Option<u32>,
    /// The first byte of the record's `0x10000` property, when present.
    pub kind_byte: Option<u8>,
}

/// Classifies one scene node into the save unit's lists, in the order the
/// original appends it. `object` is `None` when the object database has no
/// record for the node; then only a name starting with `[` joins the second
/// list. With a record, the kind pair picks the first (`0x1100`, `0x1200`)
/// or second (`0x2200`) list, and independently a `0x10000` property whose
/// first byte is 2 adds the node to the third: one node can join two lists.
pub fn scene_content(name: &str, object: Option<ObjectKinds>) -> Vec<SceneContent> {
    let Some(object) = object else {
        return if name.as_bytes().first() == Some(&b'[') {
            vec![SceneContent::Second]
        } else {
            Vec::new()
        };
    };
    let mut lists = Vec::new();
    if let Some(flags) = object.flags {
        let pair = flags & 0xff00;
        if pair == 0x1100 || pair == 0x1200 {
            lists.push(SceneContent::First);
        } else if pair == 0x2200 {
            lists.push(SceneContent::Second);
        }
    }
    if object.kind_byte == Some(2) {
        lists.push(SceneContent::Third);
    }
    lists
}

/// The grid-environment arrays a load allocates: one zero word per grid
/// cell, a set update callback and the two flags the original writes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GridEnvironmentSetup {
    pub values: Vec<u32>,
    /// The engine flag the original stores as 1.
    pub flag: u32,
    /// The mode word the original stores as 0.
    pub mode: u32,
}

/// The arrays for `cells` grid cells: all zero, flag 1 and mode 0.
pub fn grid_environment_setup(cells: usize) -> GridEnvironmentSetup {
    GridEnvironmentSetup {
        values: vec![0; cells],
        flag: 1,
        mode: 0,
    }
}

/// One scene flush: the object database begins, the player is flushed, the
/// migration pass flushes every travelling character once, and the
/// database ends. The original empties the migration list right after, so
/// a plain flush keeps nobody for a transition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SceneFlush {
    pub object_db_begun: bool,
    pub player_flushed: bool,
    pub migrations: Vec<Migration>,
    pub object_db_ended: bool,
}

/// The save unit's scene flush over the offered characters.
pub fn flush_scene(motiles: &[SceneMotile<'_>]) -> SceneFlush {
    SceneFlush {
        object_db_begun: true,
        player_flushed: true,
        migrations: migrations(motiles),
        object_db_ended: true,
    }
}

/// Where the player and the followers land on a newly loaded level.
#[derive(Debug, Clone, PartialEq)]
pub struct LandingPlan {
    pub player: [f32; 3],
    pub followers: Vec<[f32; 3]>,
}

/// The landing plan: the player at the saved planar position and the
/// lifted floor height, the travelling followers around them in the stored
/// offset order, rotated by the spawn matrix's rotation rows `frame` (the
/// saved player matrix, or the identity default). Each follower position
/// is the matrix position before that follower's own floor probe and lift.
pub fn landing_plan(
    position: [f32; 3],
    frame: [[f32; 3]; 3],
    extent: f32,
    floor: f32,
    raised: bool,
    followers: usize,
) -> LandingPlan {
    let player = spawn_placement(position, extent, floor, raised);
    LandingPlan {
        player,
        followers: follower_positions(player, frame, followers),
    }
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
        // A quarter-turn frame with rows (0, 0, −1), (0, 1, 0), (1, 0, 0):
        // the row vector (−100, 0, −100) gives −100·row0 + −100·row2 =
        // (−100, 0, 100), plus the player's (10, 0, 20).
        let m = [[0.0, 0.0, -1.0], [0.0, 1.0, 0.0], [1.0, 0.0, 0.0]];
        let front = follower_position([10.0, 0.0, 20.0], m, FOLLOWER_OFFSETS[0]);
        assert_eq!(front, [-90.0, 0.0, 120.0]);
        // The third offset (0, 0, 150) is 150·row0 = (150, 0, 0).
        let back = follower_position([0.0; 3], m, FOLLOWER_OFFSETS[2]);
        assert_eq!(back, [150.0, 0.0, 0.0]);
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
        let reset = reattach_follower(1.0);
        assert_eq!(reset.carry, 0.0);
        assert_eq!(reset.min_distance, 5.0);
        assert_eq!(reset.controller, 1);
        assert!(reset.controls && reset.uid_cleared);
        // NaN does not compare below 5 and is kept.
        assert!(reattach_follower(f32::NAN).min_distance.is_nan());
        let kept = reattach_follower(8.0);
        assert_eq!(kept.min_distance, 8.0);
        // The controller word is stored as 1 whatever it was before.
        assert_eq!(kept.controller, 1);
    }

    #[test]
    fn the_grid_arrays_start_zeroed_with_the_two_flags() {
        let setup = grid_environment_setup(3);
        assert_eq!(setup.values, vec![0; 3]);
        assert_eq!((setup.flag, setup.mode), (1, 0));
    }

    #[test]
    fn the_scene_flush_lists_the_travelling_characters() {
        let motiles = [SceneMotile {
            address: 1,
            name: "Ann",
            controller: 4,
            flags: 0,
        }];
        let flush = flush_scene(&motiles);
        assert!(flush.object_db_begun && flush.player_flushed && flush.object_db_ended);
        assert_eq!(flush.migrations.len(), 1);
        assert!(!flush.migrations[0].duplicate);
    }

    #[test]
    fn the_landing_plan_lifts_the_player_and_places_the_followers() {
        let identity = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
        let plan = landing_plan([1.0, 2.0, 3.0], identity, 1.0, 0.0, false, 2);
        assert!((plan.player[0] - 1.0).abs() < 1e-6);
        assert!((plan.player[1] - 102.08).abs() < 1e-4);
        assert!((plan.player[2] - 3.0).abs() < 1e-6);
        assert_eq!(plan.followers.len(), 2);
        assert!((plan.followers[0][0] + 99.0).abs() < 1e-4);
        assert!((plan.followers[0][2] + 97.0).abs() < 1e-4);
    }

    #[test]
    fn the_menu_scene_has_the_original_values() {
        let scene = menu_scene();
        assert_eq!(scene.file, "menuscene.rfc");
        assert_eq!(scene.fog_distance, 1.8);
        assert_eq!(scene.view, [0.0, 500.0, 700.0]);
        assert_eq!(scene.target, [0.0, 0.0, -200.0]);
        assert_eq!(scene.camera_field, 60.0);
    }

    #[test]
    fn the_terrain_file_replaces_the_locale_extension() {
        assert_eq!(terrain_file_name("exanima04"), "exanima04.rft");
        assert_eq!(terrain_file_name("arenao1.rfc"), "arenao1.rft");
    }

    #[test]
    fn scene_content_follows_the_object_kind_then_the_name() {
        let kinds = |flags, kind_byte| {
            Some(ObjectKinds {
                flags: Some(flags),
                kind_byte,
            })
        };
        assert_eq!(
            scene_content("door", kinds(0x1100, None)),
            vec![SceneContent::First]
        );
        assert_eq!(
            scene_content("lever", kinds(0x12ff, None)),
            vec![SceneContent::First]
        );
        assert_eq!(
            scene_content("x", kinds(0x2200, None)),
            vec![SceneContent::Second]
        );
        assert_eq!(
            scene_content("x", kinds(0x0100, Some(2))),
            vec![SceneContent::Third]
        );
        // The third list is tested independently of the kind pair.
        assert_eq!(
            scene_content("x", kinds(0x1100, Some(2))),
            vec![SceneContent::First, SceneContent::Third]
        );
        // A record without the kind property can still join the third.
        assert_eq!(
            scene_content(
                "x",
                Some(ObjectKinds {
                    flags: None,
                    kind_byte: Some(2)
                })
            ),
            vec![SceneContent::Third]
        );
        // With a record the name prefix is not looked at.
        assert!(scene_content("[trigger]", kinds(0x0100, None)).is_empty());
        assert_eq!(scene_content("[trigger]", None), vec![SceneContent::Second]);
        assert!(scene_content("plain", None).is_empty());
    }
}
