//! The bridge between a level change and the save unit's level export and
//! landing plan.
//!
//! The original's level transition flushes the scene, saves the current
//! locale's level section, loads the destination (from the save's own
//! section or the archive), places the player from the saved spawn matrix
//! and puts the travelling followers around them. Exarust has the rules as
//! tested pieces ([`crate::save_level`], [`exa_formats::rfc_write`]); this
//! module ties them into one plan a level change can execute:
//!
//! * [`export_for_save`] writes the current scene with the save's exporter,
//!   adding the story's settings chunk only when the save's gate allows it
//!   (the arena never does);
//! * [`plan`] pairs that section with the destination's landing: the player
//!   at the saved planar position and probed floor, the followers in the
//!   stored offset order, each with its own probe and lift, plus the
//!   migration list and the per-follower re-attach state.
//!
//! **What still blocks the save export.** Exarust's level loading keeps its
//! own simplified [`exa_world::Scene`] and not the export model, and it does
//! not track a level's changed state (moved props, taken items, corpses), so
//! it has no [`ExportScene`] to hand to the exporter; converting the loaded
//! RFC back into the export model is not written. The level change itself is
//! now in process (`exa_world::level_lifecycle`), so `PendingLanding` reaches
//! a live load; exporting the current level into a save waits for that
//! conversion. The plan remains the tested function the save path will call:
//! the scene, the floor probes and the follower bodies are its inputs.

use bevy::prelude::Resource;
use exa_formats::rfc_write::{ExportOptions, ExportScene, export_save_level};
use exa_formats::save_chunk::{SettingsChunk, save_tag_bytes, settings_chunk_written};

use crate::save_level::{
    FollowerBody, FollowerReset, LandingPlan, Migration, SceneMotile, landing_plan, migrations,
    place_followers, reattach_follower,
};

/// The settings state the story's exporter gates its chunk on: the read
/// tag's `old` flag, the tag itself and the two words the settings store
/// carries (`first` is the store's word, `saved` the saved word).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SettingsState {
    pub old: bool,
    pub tag: [u8; 12],
    pub first: u32,
    pub saved: u32,
}

impl SettingsState {
    /// The state a fresh load leaves (`save_chunk::load_reset`): the flag
    /// clear, the default tag and both words zero.
    pub fn reset() -> Self {
        Self {
            old: false,
            tag: save_tag_bytes(false, [0; 12]),
            first: 0,
            saved: 0,
        }
    }
}

/// A level change's inputs.
#[derive(Debug, Clone)]
pub struct TransitionInput<'a> {
    /// The current level's scene as the exporter sees it.
    pub scene: &'a ExportScene,
    /// The export switches; the save's own adds the characters' AI home and
    /// motion state.
    pub options: ExportOptions,
    /// Which mode exports: the story appends the settings chunk, the arena
    /// does not.
    pub story: bool,
    /// The story's settings state at the save.
    pub settings: SettingsState,
    /// The saved spawn matrix's planar position (cm).
    pub position: [f32; 3],
    /// The spawn matrix's rotation rows, which turn the follower offsets.
    pub frame: [[f32; 3]; 3],
    /// The saved heading (radians), kept for the destination's placement.
    pub heading: f32,
    /// The player body's extent word, the destination's probed floor and
    /// whether the body is raised (the 13 cm lift).
    pub extent: f32,
    pub floor: f32,
    pub raised: bool,
    /// The travelling followers, in the migration list's order.
    pub followers: Vec<FollowerBody>,
    /// Each follower's minimum distance before the re-attach.
    pub follower_min_distances: Vec<f32>,
    /// The scene's characters, from which the migration list is built.
    pub motiles: &'a [SceneMotile<'a>],
}

impl<'a> TransitionInput<'a> {
    /// A transition with the save's export switches and the default
    /// settings state.
    pub fn new(scene: &'a ExportScene) -> Self {
        Self {
            scene,
            options: ExportOptions {
                save_mode: true,
                ..ExportOptions::default()
            },
            story: true,
            settings: SettingsState::reset(),
            position: [0.0; 3],
            frame: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
            heading: 0.0,
            extent: 0.0,
            floor: 0.0,
            raised: false,
            followers: Vec::new(),
            follower_min_distances: Vec::new(),
            motiles: &[],
        }
    }
}

/// What a level change does: the level section to write and the destination's
/// landing.
#[derive(Debug, Clone, PartialEq)]
pub struct TransitionPlan {
    /// The exported level section (the save's locale section).
    pub level: Vec<u8>,
    /// The player and the follower offset positions.
    pub landing: LandingPlan,
    /// The followers' final placements after their own probe and lift.
    pub followers: Vec<[f32; 3]>,
    /// Each follower's re-attach state (UID and controls re-created, carry
    /// cleared, controller 1, the minimum distance floor).
    pub resets: Vec<FollowerReset>,
    /// The migration list the flush reported, duplicates included.
    pub migrations: Vec<Migration>,
}

/// Writes the current level's section for a save: the scene's chunk tree,
/// then the story's settings chunk when the gate allows it (an old tag or a
/// saved word above zero) and never for the arena.
pub fn export_for_save(input: &TransitionInput<'_>) -> exa_formats::Result<Vec<u8>> {
    let settings = (input.story
        && settings_chunk_written(input.settings.old, input.settings.saved))
    .then_some(SettingsChunk {
        tag: save_tag_bytes(input.settings.old, input.settings.tag),
        first: input.settings.first,
        saved: input.settings.saved,
    });
    export_save_level(input.scene, &input.options, settings)
}

/// The whole level change: the exported section, the landing plan and the
/// followers' placements and re-attach state.
pub fn plan(input: &TransitionInput<'_>) -> exa_formats::Result<TransitionPlan> {
    let level = export_for_save(input)?;
    let landing = landing_plan(
        input.position,
        input.frame,
        input.extent,
        input.floor,
        input.raised,
        input.followers.len(),
    );
    let followers = place_followers(landing.player, input.frame, &input.followers);
    let resets = input
        .followers
        .iter()
        .zip(&input.follower_min_distances)
        .map(|(_, distance)| reattach_follower(*distance))
        .collect();
    // The flush keeps the travelling characters once and reports repeats;
    // a plain flush empties the list, which is the caller's list to keep.
    let migrations = migrations(input.motiles);
    Ok(TransitionPlan {
        level,
        landing,
        followers,
        resets,
        migrations,
    })
}

/// A landing plan waiting for the level loader: the player's feet position
/// and heading and the followers' positions, all in metres. The host sets it
/// before a level loads (a transition or a load of a save's placement) and
/// the loader puts the player there instead of the level's own start; the
/// followers wait for the live follower spawn, which Exarust does not have
/// yet.
#[derive(Resource, Debug, Clone, Default, PartialEq)]
pub struct PendingLanding {
    pub player: Option<([f32; 3], f32)>,
    pub followers: Vec<[f32; 3]>,
}

impl PendingLanding {
    /// The landing a completed plan asks for, with the saved heading. The
    /// plan's positions are the save's centimetres.
    pub fn of(plan: &TransitionPlan, heading: f32) -> Self {
        const METRES: f32 = 0.01;
        Self {
            player: Some((plan.landing.player.map(|v| v * METRES), heading)),
            followers: plan
                .followers
                .iter()
                .map(|at| at.map(|v| v * METRES))
                .collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use exa_formats::rfc_write::{ExportScene, SceneNode, class};
    use exa_formats::save_chunk::SAVE_SETTINGS_ID;
    use exa_formats::save_package::parse_chunks;

    fn scene() -> ExportScene {
        let mut name = [0u8; 16];
        name[..4].copy_from_slice(b"door");
        ExportScene {
            nodes: vec![SceneNode {
                name,
                flags: class::PLAIN,
                local: exa_formats::rfc_write::IDENTITY,
                world: exa_formats::rfc_write::IDENTITY,
                ..Default::default()
            }],
            whole_scene: true,
            ..Default::default()
        }
    }

    /// Top-level chunks of a level section (the four sanity/version bytes
    /// come first).
    fn section_chunks(level: &[u8]) -> Vec<u32> {
        parse_chunks(&level[4..])
            .unwrap()
            .iter()
            .map(|chunk| chunk.id)
            .collect()
    }

    #[test]
    fn the_story_level_section_gates_the_settings_chunk() {
        let scene = scene();
        let mut input = TransitionInput::new(&scene);
        // The load's reset state: no old tag, a zero saved word, so the
        // story's hook writes nothing.
        let level = export_for_save(&input).unwrap();
        assert!(!section_chunks(&level).contains(&SAVE_SETTINGS_ID));
        // The scene itself is there and reads back.
        let parsed = exa_formats::rfc::Scene::parse(&level).unwrap();
        assert_eq!(parsed.nodes.len(), 1);
        assert_eq!(parsed.nodes[0].name, "door");

        // A saved word above zero (or an old tag) writes the chunk after
        // the scene.
        input.settings.saved = 7;
        let level = export_for_save(&input).unwrap();
        let ids = section_chunks(&level);
        assert_eq!(ids.last(), Some(&SAVE_SETTINGS_ID));
        let chunks = parse_chunks(&level[4..]).unwrap();
        let settings = chunks.last().unwrap();
        assert_eq!(&settings.payload[..4], &[0, 0, 0, 0]);
        assert_eq!(&settings.payload[4..8], &1_100_000.0f32.to_le_bytes());
        assert_eq!(settings.payload[16..20], 7u32.to_le_bytes());

        // An old tag is written back, as the exporter's rule says.
        input.settings.old = true;
        input.settings.saved = 0;
        input.settings.tag = [0xff; 12];
        let level = export_for_save(&input).unwrap();
        let chunks = parse_chunks(&level[4..]).unwrap();
        let settings = chunks.last().unwrap();
        assert_eq!(&settings.payload[..12], &[0xff; 12]);

        // The arena never writes it, whatever the state.
        input.story = false;
        let level = export_for_save(&input).unwrap();
        assert!(!section_chunks(&level).contains(&SAVE_SETTINGS_ID));
    }

    #[test]
    fn the_plan_places_the_player_and_the_followers_with_their_own_lifts() {
        let scene = scene();
        // A quarter-turn frame: rows (0, 0, −1), (0, 1, 0), (1, 0, 0).
        let quarter = [[0.0, 0.0, -1.0], [0.0, 1.0, 0.0], [1.0, 0.0, 0.0]];
        let motiles = [
            SceneMotile {
                address: 1,
                name: "Ann",
                controller: 4,
                flags: 0,
            },
            SceneMotile {
                address: 2,
                name: "Bob",
                controller: 3,
                flags: 0,
            },
            SceneMotile {
                address: 1,
                name: "Ann",
                controller: 4,
                flags: 0,
            },
        ];
        let input = TransitionInput {
            scene: &scene,
            position: [100.0, 0.0, 200.0],
            frame: quarter,
            heading: 0.5,
            extent: 1.0,
            floor: 40.0,
            raised: false,
            followers: vec![
                FollowerBody {
                    extent: 0.5,
                    floor: 10.0,
                    raised: true,
                },
                FollowerBody {
                    extent: 1.0,
                    floor: 0.0,
                    raised: false,
                },
            ],
            follower_min_distances: vec![1.0, f32::NAN],
            motiles: &motiles,
            ..TransitionInput::new(&scene)
        };
        let plan = plan(&input).unwrap();
        // 40 + (108.18 − 11.1) + 5.
        assert!((plan.landing.player[1] - 142.08).abs() < 1e-4);
        assert_eq!(plan.landing.player[0], 100.0);
        assert_eq!(plan.landing.player[2], 200.0);
        // The first offset (−100, 0, −100) turns to (−100, 0, 100); with
        // the player at (100, ·, 200) that is (0, ·, 300), and its own
        // 0.5 extent and 13 cm raised lift make 10 + 102.63 + 13.
        assert_eq!(plan.followers[0][0], 0.0);
        assert_eq!(plan.followers[0][2], 300.0);
        assert!((plan.followers[0][1] - 125.63).abs() < 1e-4);
        // The second offset (100, 0, −100) turns to (−100, 0, −100):
        // (0, ·, 100), with 5 cm over the floor.
        assert_eq!(plan.followers[1][0], 0.0);
        assert_eq!(plan.followers[1][2], 100.0);
        assert!((plan.followers[1][1] - 102.08).abs() < 1e-4);
        // The re-attach state: the distance floor and a kept NaN.
        assert_eq!(plan.resets.len(), 2);
        assert_eq!(plan.resets[0].min_distance, 5.0);
        assert!(plan.resets[1].min_distance.is_nan());
        assert!(plan.resets.iter().all(|reset| reset.controller == 1));
        // The loader's landing takes the same places in metres.
        let pending = PendingLanding::of(&plan, input.heading);
        let (at, heading) = pending.player.unwrap();
        assert!((at[0] - 1.0).abs() < 1e-6);
        assert!((at[1] - 1.420_8).abs() < 1e-4);
        assert!((at[2] - 2.0).abs() < 1e-6);
        assert_eq!(heading, 0.5);
        assert_eq!(pending.followers.len(), 2);
        assert!((pending.followers[0][2] - 3.0).abs() < 1e-6);
        // The migration list keeps the travelling characters once and
        // flags the repeat; a controller 3 does not travel.
        assert_eq!(plan.migrations.len(), 2);
        assert!(!plan.migrations[0].duplicate);
        assert!(plan.migrations[1].duplicate);
    }

    #[test]
    fn a_scene_that_does_not_export_is_an_error_not_a_panic() {
        // A parent past the node list is inconsistent input.
        let mut name = [0u8; 16];
        name[..1].copy_from_slice(b"x");
        let scene = ExportScene {
            nodes: vec![SceneNode {
                name,
                flags: class::PLAIN,
                parent: Some(4),
                ..Default::default()
            }],
            whole_scene: true,
            ..Default::default()
        };
        let input = TransitionInput::new(&scene);
        assert!(plan(&input).is_err());
    }

    /// The installed game's arena hub (with `EXANIMA_DIR`): a real placed
    /// character's saved position and heading drive the landing plan. The
    /// plan keeps the planar position, lifts the probed floor, and turns the
    /// followers' offsets by the heading.
    #[test]
    fn shipped_placements_drive_the_landing_plan() {
        let Some(dir) = exa_assets::locate_install(None) else {
            return;
        };
        let game = exa_assets::GameData::open(&dir).unwrap();
        let bytes = game
            .pack(exa_assets::PackKind::Resource)
            .unwrap()
            .get("arenahub.rfc")
            .unwrap();
        let level = exa_formats::rfc::Scene::parse(bytes).unwrap();
        let placements = level
            .characters
            .as_ref()
            .map(|characters| characters.placements.clone())
            .unwrap_or_default();
        let Some(placement) = placements.first() else {
            return;
        };
        let scene = scene();
        let yaw = placement.yaw;
        let (sin, cos) = (yaw.sin(), yaw.cos());
        let frame = [[cos, 0.0, -sin], [0.0, 1.0, 0.0], [sin, 0.0, cos]];
        let input = TransitionInput {
            scene: &scene,
            position: [
                placement.position[0] * 100.0,
                placement.position[1] * 100.0,
                placement.position[2] * 100.0,
            ],
            frame,
            heading: yaw,
            extent: 0.0,
            floor: 25.0,
            raised: false,
            followers: vec![FollowerBody {
                extent: 0.0,
                floor: 25.0,
                raised: false,
            }],
            follower_min_distances: vec![2.0],
            ..TransitionInput::new(&scene)
        };
        let plan = plan(&input).unwrap();
        assert!((plan.landing.player[0] - placement.position[0] * 100.0).abs() < 1e-3);
        assert!((plan.landing.player[2] - placement.position[2] * 100.0).abs() < 1e-3);
        // 25 + 108.18 + 5.
        assert!((plan.landing.player[1] - 138.18).abs() < 1e-4);
        // The first offset turned by the heading, then the same lift.
        assert_eq!(plan.followers.len(), 1);
        assert!((plan.followers[0][1] - 138.18).abs() < 1e-4);
        assert_eq!(plan.resets[0].min_distance, 5.0);
    }
}
