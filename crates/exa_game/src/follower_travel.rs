//! Followers carried across a door's level change, as the save unit's
//! transition carries them.
//!
//! The original's transition (the game-main unit's locale switch calls it
//! for every door and portal) flushes the player, then walks the scene's
//! simulation characters: every one whose AI activity word is 4 (the
//! follow) and whose flags lack `0x800` joins the migration list once
//! ([`crate::save_level::migrations`]) and is flushed at once, its carried
//! items promoted to the world tier as the player's are. Each travelling
//! character's node is then taken out of the scene (its body controller in
//! mode 8, detached from the old scene), so the destination's import does
//! not destroy it. After the destination is in and the player is placed,
//! each follower is placed on the stored offsets around the player
//! ([`crate::save_level::follower_positions`], turned by the spawn matrix:
//! the portal node's, else the identity), with its own floor probe and
//! lift, and the re-attach pass runs: the record's UID manager is freed,
//! the character joins the new scene, its record takes the new level's id,
//! its controller's phase word becomes 1, its controller is reset and its
//! minimum distance is raised to 5 cm ([`crate::save_level::reattach_follower`]).
//! The list is emptied.
//!
//! Exarust's level teardown despawns every level character, so the
//! follower cannot keep its body across the load. [`collect_followers`]
//! takes each travelling character down into a [`Traveller`] (its record
//! with its live possessions and health written back, its held roles), and
//! the host respawns it on the destination with
//! [`crate::save_level::follower_positions`] and [`arrival_yaw`];
//! [`restore_arrivals`] loads the carried health onto the new body. The
//! flush registers each follower in the live world character database
//! ([`crate::world_characters::WorldCharacters::register`]: a level's own
//! character moves there under a world id with its origin, and takes the
//! level it leaves as its level), and the host's re-attach stores the
//! destination's id on the record
//! ([`crate::world_characters::WorldCharacters::arrive`]). A save
//! load, a new session or an arena round (a world reset) carries nobody,
//! as those entry points do not run the transition. [Exarust's: the body's
//! pose, wounds, contacts and the follow's walk start afresh; the original
//! keeps the same object.]

use bevy::prelude::*;
use exa_formats::characters::CharacterRecord;

use crate::save_level::{SceneMotile, migrations};

/// One follower taken down for the destination: the character as the
/// destination's spawn places it (its position and facing are set on
/// arrival) and the roles it held, which keep it following.
#[derive(Debug, Clone)]
pub struct Traveller {
    pub placed: exa_world::PlacedCharacter,
    pub roles: exa_ai::roles::Roles,
}

/// The migration list between the old level's teardown and the
/// destination's spawn. The host takes it once the destination's
/// collision is in.
#[derive(Resource, Debug, Clone, Default)]
pub struct TravellingFollowers {
    pub travellers: Vec<Traveller>,
}

/// Marks a follower respawned on the destination; [`restore_arrivals`]
/// loads its carried health onto the body once.
#[derive(Component, Debug, Clone, Copy)]
pub struct ArrivedFollower;

/// The scene characters' query for the migration pass.
type FollowerQuery<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static exa_ai::brain::Npc,
        &'static exa_ai::roles::Roles,
        &'static GlobalTransform,
        Option<&'static exa_character::record::CharacterData>,
        Option<&'static exa_items::possession::Possessions>,
        Option<&'static exa_combat::Fighter>,
        Option<&'static exa_combat::StateFlags>,
        Option<&'static exa_combat::Condition>,
        Option<&'static crate::thaum::cast::PowerCaster>,
    ),
    Without<crate::player::PlayerBody>,
>;

/// The migration pass of a door's level change, run by the lifecycle just
/// before its teardown ([`exa_world::level_lifecycle::BeforeDoorTeardown`],
/// after the player's world sync): every following scene character (each
/// once, in the scene's order) is flushed (its local items promoted to the
/// world tier, [`crate::save_live::promote_followers`]) and taken down into
/// [`TravellingFollowers`]. A change behind a world reset, or a teardown
/// with no destination, does not run it.
pub fn collect_followers(
    mut commands: Commands,
    mut live: ResMut<exa_assets::live_objects::LiveObjects>,
    followers: FollowerQuery,
    mut world: Option<ResMut<crate::world_characters::WorldCharacters>>,
    level: Option<Res<exa_ai::brain::RoleDatabase>>,
    mut local: Option<ResMut<crate::world_characters::LocalCharacters>>,
) {
    // The scene's characters in a fixed order, as the scene's list is.
    let mut scene: Vec<_> = followers.iter().collect();
    scene.sort_by_key(|(entity, ..)| *entity);
    let names: Vec<String> = scene
        .iter()
        .map(|(_, _, _, _, data, ..)| data.map_or_else(String::new, |d| d.record.name.clone()))
        .collect();
    let motiles: Vec<SceneMotile<'_>> = scene
        .iter()
        .zip(&names)
        .map(|((entity, npc, ..), name)| SceneMotile {
            address: entity.to_bits() as usize,
            name,
            controller: npc.applied.as_ref().map_or(1, |a| a.activity),
            // Exarust keeps no body flag that holds a follower back.
            flags: 0,
        })
        .collect();
    let list = migrations(&motiles);
    if list.travelling.is_empty() {
        return;
    }
    for repeat in &list.duplicates {
        warn!("Migration: Duplicate simulation motile: {repeat:#x}");
    }
    // Each travelling character's record with its live possessions and
    // health written back, in the list's order.
    let mut records: Vec<CharacterRecord> = Vec::new();
    let mut travelling_motiles: Vec<SceneMotile<'_>> = Vec::new();
    let mut kept = Vec::new();
    for migration in &list.travelling {
        let Some(at) = motiles.iter().position(|m| m.address == migration.address) else {
            continue;
        };
        let (_, _, roles, transform, data, possessions, fighter, flags, condition, caster) =
            scene[at];
        let reset = exa_character::record::CharacterData::of_record(CharacterRecord::reset());
        let record = crate::save_live::scene_character_record(
            data.unwrap_or(&reset),
            possessions,
            fighter.map(|f| (f, flags.copied(), condition.copied(), caster)),
            Some(roles),
        );
        records.push(record);
        travelling_motiles.push(motiles[at]);
        kept.push((roles.clone(), transform.translation()));
    }
    // The flush: each follower's local items take world ids, so they keep
    // their records when the destination replaces the local tier.
    let promoted =
        crate::save_live::promote_followers(live.db_mut(), &travelling_motiles, &mut records);
    // The flush's database part: each follower joins the world character
    // database (a level's own character under its first free id, with its
    // origin) and takes the level it leaves as its level.
    let leaving = level.as_ref().map_or(0, |level| level.locale);
    if let Some(world) = world.as_mut() {
        for record in &mut records {
            let old = record.id;
            let id = world.register(record, leaving);
            // A level's own character leaves the level's store first.
            let removed = old & exa_formats::character_db::WORLD_TAG == 0
                && local.as_mut().is_some_and(|local| local.remove(old));
            info!(
                "Migration: {} registered in the world character database: {old:#x} -> {id:#x} (level {leaving}; deleted from the level's: {removed})",
                record.name
            );
        }
    }
    let mut travellers = Vec::new();
    for (record, (roles, from)) in records.into_iter().zip(kept) {
        let ids: Vec<u32> = record
            .hands
            .iter()
            .map(|h| h.id)
            .chain(record.worn.iter().map(|w| w.id))
            .chain(record.inventory.iter().map(|c| c.item))
            .filter(|&id| id != 0)
            .collect();
        let items: Vec<exa_formats::objdb::Item> =
            ids.iter().filter_map(|&id| live.item(id)).collect();
        let find = |id: u32| items.iter().find(|item| item.id == id);
        let look = exa_world::record_look(&record, find);
        info!(
            "Migration: {} (record {:#x}) leaves {from} with the player ({} items carried)",
            record.name,
            record.id,
            ids.len()
        );
        travellers.push(Traveller {
            placed: exa_world::PlacedCharacter {
                name: record.name.clone(),
                position: Vec3::ZERO,
                yaw: 0.0,
                dead: false,
                race: record.race,
                female: record.female,
                worn: look.worn,
                held: look.held,
                hands: look.hands,
                encumbrance: exa_formats::encumbrance::of_record(&record, find),
                specials: exa_formats::encumbrance::WornSpecials::of_record(&record, find),
                record,
                pose: exa_formats::characters::SavedPose::None,
                motion: None,
                ai_home: None,
                fall: false,
            },
            roles,
        });
    }
    info!(
        "Migration: {} followers travel ({promoted} flushed, {} repeats left out)",
        travellers.len(),
        list.duplicates.len()
    );
    commands.insert_resource(TravellingFollowers { travellers });
}

/// The rotation rows of the spawn matrix a placement's heading gives: the
/// engine's yaw rows `(cos, 0, −sin)`, `(0, 1, 0)`, `(sin, 0, cos)` (the
/// inverse of [`crate::level_change::PlayerPlacement::of_flow`]'s heading).
/// A change with no portal placement takes the identity.
pub fn spawn_frame(heading: Option<f32>) -> [[f32; 3]; 3] {
    match heading {
        Some(yaw) => {
            let (s, c) = yaw.sin_cos();
            [[c, 0.0, -s], [0.0, 1.0, 0.0], [s, 0.0, c]]
        }
        None => [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
    }
}

/// The heading a follower spawns with: the spawn matrix's, as the original
/// spawns each follower with the same matrix (only its position moved).
pub fn arrival_yaw(heading: Option<f32>) -> f32 {
    heading.unwrap_or(0.0)
}

/// Where each follower lands around the player standing at `player`
/// (metres) with the spawn matrix `frame`: the stored offsets turned by
/// the frame and added to the player's position, then each one's own floor
/// probe ([`crate::save_level::landing_feet`] with `cast`). At most six
/// followers are placed.
pub fn arrival_positions(
    player: Vec3,
    frame: [[f32; 3]; 3],
    count: usize,
    mut cast: impl FnMut(Vec3, Vec3, f32) -> Option<Vec3>,
) -> Vec<Vec3> {
    const METRES: f32 = 0.01;
    crate::save_level::follower_positions((player / METRES).to_array(), frame, count)
        .into_iter()
        .map(|at| {
            crate::save_level::landing_feet(Vec3::from_array(at) * METRES, |from, dir, reach| {
                cast(from, dir, reach)
            })
        })
        .collect()
}

/// The re-attach pass's record part for a record outside the world
/// character database (one carried without a live store): an id that
/// names one of the destination's own characters takes the id after them,
/// so the two never share an id. A world-tagged id, the store's (every
/// registered follower's), is kept, as the original keeps the follower's
/// world record. `taken` holds the destination's record ids (masked).
pub fn arrival_id(id: u32, taken: &[u32]) -> u32 {
    let mask = exa_formats::characters::ID_MASK;
    if id & exa_formats::character_db::WORLD_TAG != 0 || !taken.contains(&(id & mask)) {
        return id;
    }
    taken.iter().copied().max().unwrap_or(0) + 1
}

/// Loads each arrived follower's carried health onto its new body, once,
/// as the spawn's last write loads a record's ([`crate::save_live::restore_body_health`]).
#[allow(clippy::type_complexity)]
pub fn restore_arrivals(
    mut commands: Commands,
    mut arrived: Query<
        (
            Entity,
            &exa_character::record::CharacterData,
            &mut exa_combat::Fighter,
            Option<&mut exa_combat::StateFlags>,
        ),
        With<ArrivedFollower>,
    >,
) {
    for (entity, data, mut fighter, flags) in &mut arrived {
        let mut state = flags.as_deref().copied().unwrap_or_default();
        let mut focus = crate::save_live::record_focus(&data.record);
        crate::save_live::restore_body_health(&data.record, &mut fighter, &mut state, &mut focus);
        match flags {
            Some(mut flags) => *flags = state,
            None => {
                commands.entity(entity).insert(state);
            }
        }
        commands.entity(entity).remove::<ArrivedFollower>();
        info!(
            "Migration: {} arrives: yellow {:.3} red {:.3} of {:.3}",
            data.record.name, fighter.health.yellow, fighter.health.red, fighter.health.max
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_portal_heading_turns_the_offsets_and_no_portal_keeps_them() {
        let id = spawn_frame(None);
        assert_eq!(id[0], [1.0, 0.0, 0.0]);
        // A quarter turn: the first offset (−100, 0, −100) row-times the
        // rows (0,0,−1), (0,1,0), (1,0,0) is (−100, 0, 100).
        let quarter = spawn_frame(Some(std::f32::consts::FRAC_PI_2));
        let at = crate::save_level::follower_position(
            [0.0; 3],
            quarter,
            crate::save_level::FOLLOWER_OFFSETS[0],
        );
        assert!((at[0] + 100.0).abs() < 1e-3 && (at[2] - 100.0).abs() < 1e-3);
        // The heading is the one PlayerPlacement reads back.
        let rows = spawn_frame(Some(0.7));
        assert!((rows[2][0].atan2(rows[0][0]) - 0.7).abs() < 1e-6);
    }

    #[test]
    fn arrivals_stand_on_the_probed_floor_around_the_player() {
        let floor = 2.0;
        let at = arrival_positions(
            Vec3::new(10.0, 2.05, 20.0),
            spawn_frame(None),
            3,
            |from, _, _| Some(Vec3::new(from.x, floor, from.z)),
        );
        assert_eq!(at.len(), 3);
        assert!((at[0] - Vec3::new(9.0, 2.05, 19.0)).length() < 1e-4);
        assert!((at[1] - Vec3::new(11.0, 2.05, 19.0)).length() < 1e-4);
        assert!((at[2] - Vec3::new(10.0, 2.05, 21.5)).length() < 1e-4);
        // A miss keeps the offset position.
        let missed = arrival_positions(Vec3::ZERO, spawn_frame(None), 1, |_, _, _| None);
        assert!((missed[0] - Vec3::new(-1.0, 0.0, -1.0)).length() < 1e-5);
        assert_eq!(
            arrival_positions(Vec3::ZERO, spawn_frame(None), 9, |_, _, _| None).len(),
            6
        );
    }

    #[test]
    fn a_carried_id_that_a_destination_character_holds_moves_past_them() {
        assert_eq!(arrival_id(0x2b, &[1, 2, 3]), 0x2b);
        assert_eq!(arrival_id(0x2b, &[0x2b, 0x30]), 0x31);
        // A world record keeps its id: the tags tell the stores apart.
        assert_eq!(arrival_id(0x4000_0002, &[2, 3]), 0x4000_0002);
    }

    fn brain(activity: i32) -> exa_ai::brain::Npc {
        let mut npc = exa_ai::brain::Npc::new([0.0; 4]);
        npc.applied = Some(exa_ai::activity::Applied {
            activity,
            idle_activity: 0,
            state: exa_ai::activity::IdleState::of(activity),
            leaves_party: false,
            follow_band_cm: None,
            phase_word: 0,
            create_idle: None,
            drop_idle: false,
            skipped: Vec::new(),
        });
        npc
    }

    fn scene() -> App {
        let mut app = App::new();
        app.init_resource::<exa_assets::live_objects::LiveObjects>();
        for (name, activity) in [("Derrin", 4), ("Guard", 1)] {
            let record = CharacterRecord {
                name: name.into(),
                ..CharacterRecord::reset()
            };
            app.world_mut().spawn((
                brain(activity),
                exa_ai::roles::Roles::default(),
                GlobalTransform::default(),
                exa_character::record::CharacterData::of_record(record),
            ));
        }
        app
    }

    /// The migration pass takes the following character along and leaves
    /// the others.
    #[test]
    fn the_migration_pass_takes_only_the_follower() {
        use bevy::ecs::system::RunSystemOnce;
        let mut app = scene();
        app.world_mut().run_system_once(collect_followers).unwrap();
        let list = app.world().resource::<TravellingFollowers>();
        assert_eq!(list.travellers.len(), 1);
        assert_eq!(list.travellers[0].placed.name, "Derrin");
    }
}
