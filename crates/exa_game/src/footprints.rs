//! The characters' footprints on the navigation grid
//! (`exa_world::nav::footprint`: 63 under a character, 32 round it, 16
//! beyond along the axes, on the penalty bytes the planner and the obstacle
//! test pay for), kept each fixed tick in the behaviour stage, as the
//! original's behaviours lift and stamp them where the character stands
//! before it moves: an NPC's every tick (collapsing or dead too, while it
//! keeps its controller); the player's only while it is not walking (any
//! condition but the walk's), so a standing or fighting player is in an
//! NPC's way and a walking one is not. A vanished player
//! ([`exa_combat::StateFlags::VANISHED`]) neither stamps nor lifts: the
//! original's player tick skips the stamp call, so the footprint stays
//! where the body vanished until it shows again. An NPC walking a planned route
//! (a roam or patrol leg, a flight, a chase) at 1 cm a tick or more keeps
//! its footprint off, as the original's AI tick gates its stamp
//! ([`exa_ai::brain::stamps_footprint`]). A character that leaves takes its
//! footprint with it, and a new level's grid starts with none. Exarust's
//! standing point is the feet.
//!
//! An NPC whose stamp falls within five cells of the grid's edge
//! ([`exa_ai::combatant::foot_cell`]) overflows: the original flags the body
//! once, unlinks it from its room's list and queues its scene node for
//! deletion, logging "Character position overflow." the first time in the
//! run. Exarust despawns the body's root (its room lists forget it through
//! the brains' removal broadcast). [Exarust: the player's tick takes the
//! same branch in the original; the player is left in the scene here, as
//! Exarust has no game without its player body.]

use std::collections::HashSet;

use bevy::prelude::*;
use exa_ai::brain;
use exa_combat::{Condition, Fighter};
use exa_physics::CharacterBody;
use exa_world::nav::{NavFootprints, NavGrid};

use crate::player::PlayerBody;

pub struct FootprintPlugin;

impl Plugin for FootprintPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<NavFootprints>().add_systems(
            FixedPreUpdate,
            keep_footprints
                .after(crate::player::steer_player)
                .after(crate::limp::place_on_ground)
                .before(crate::player::release_held_wake),
        );
    }
}

/// Whether a character's footprint is on the grid this tick: an NPC's
/// always, the player's unless it walks.
pub fn stamps(player: bool, condition: Option<Condition>) -> bool {
    !player || condition.is_none_or(|c| c != Condition::WALKING)
}

/// The AI tick's footprint gate for an NPC: `following` its route, it
/// stamps only while its AI asks no walk (`desired`, m/s) or the last fixed
/// step (one body tick) walked under 1 cm along its `facing` (the original's
/// speed is the root's move projected on the heading, unsigned).
pub fn npc_stamps(following: bool, desired: Vec3, walked_m: Vec3, facing: Vec3) -> bool {
    let moving = desired.x != 0.0 || desired.z != 0.0;
    let speed_cm = (walked_m.dot(facing) * 100.0).abs();
    brain::stamps_footprint(following, moving, speed_cm)
}

/// Whether a stamp at `feet` (metres) overflows the grid: its cell lies
/// within five cells of the edge on either axis (the original's stamp,
/// [`exa_ai::combatant::foot_cell`]).
pub fn overflows(grid: &NavGrid, feet: Vec3) -> bool {
    let (width, depth) = grid.size();
    let cell = exa_ai::combatant::foot_cell(
        feet * 100.0,
        i32::try_from(width).unwrap_or(i32::MAX),
        i32::try_from(depth).unwrap_or(i32::MAX),
    );
    !cell.in_grid
}

/// Every character under its controller moves its footprint to where its
/// feet are, or takes it off; the footprints of characters gone (or off
/// their controller) come off too.
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub fn keep_footprints(
    mut commands: Commands,
    grid: Option<ResMut<NavGrid>>,
    mut prints: ResMut<NavFootprints>,
    characters: Query<
        (
            Entity,
            &ChildOf,
            Option<&Condition>,
            Has<PlayerBody>,
            Option<&exa_ai::route_walk::AiRoute>,
            Option<&Transform>,
            Option<&exa_combat::StateFlags>,
        ),
        With<Fighter>,
    >,
    roots: Query<(&CharacterBody, &Transform)>,
    mut seen: Local<HashSet<Entity>>,
    mut walking: Local<HashSet<Entity>>,
    mut overflowed: Local<HashSet<Entity>>,
) {
    let Some(mut grid) = grid else { return };
    if grid.is_added() {
        prints.forget();
    }
    seen.clear();
    for (who, parent, condition, player, route, model, flags) in &characters {
        let Ok((capsule, at)) = roots.get(parent.parent()) else {
            continue;
        };
        seen.insert(who);
        if player && flags.is_some_and(|f| f.has(exa_combat::StateFlags::VANISHED)) {
            continue;
        }
        let on = if player {
            stamps(true, condition.copied())
        } else {
            let facing = model.map_or(Vec3::Z, |t| t.rotation * Vec3::Z);
            let following = brain::follows_route(route);
            let on = npc_stamps(
                following,
                capsule.desired_velocity,
                capsule.walked(),
                facing,
            );
            if on == walking.contains(&who) {
                if on {
                    walking.remove(&who);
                    debug!("npc {who:?}: its footprint is stamped again");
                } else {
                    walking.insert(who);
                    debug!("npc {who:?} walks its route: its footprint is off the grid");
                }
            }
            on
        };
        if on && !player && overflows(&grid, capsule.feet(at)) {
            prints.lift(&mut grid, who);
            if overflowed.insert(who) {
                if exa_ai::combatant::take_overflow_log() {
                    error!("Character position overflow. ({who:?})");
                }
                info!("npc {who:?} stands off the navigation grid: removed from the scene");
                commands.entity(parent.parent()).despawn();
            }
        } else if on {
            prints.stamp(&mut grid, who, capsule.feet(at));
        } else {
            prints.lift(&mut grid, who);
        }
    }
    walking.retain(|e| seen.contains(e));
    overflowed.retain(|e| seen.contains(e));
    prints.retain(&mut grid, |e| seen.contains(&e));
}

#[cfg(test)]
mod tests {
    use exa_world::nav::source::grid_base;
    use exa_world::nav::{Columns, Layer};

    use super::*;

    /// A 40 × 40 grid of ground at 0 (voxel 221 of a 3.4 m grid top).
    fn grid() -> NavGrid {
        let mut c = Columns::new(40, 40, grid_base(3.4));
        for z in 0..40 {
            for x in 0..40 {
                c.fill_ground(x, z, 0.0);
            }
        }
        NavGrid::from_columns(&c)
    }

    /// The player stamps while it stands or fights, not while it walks;
    /// an NPC always.
    #[test]
    fn who_stamps() {
        assert!(stamps(false, Some(Condition::WALKING)));
        assert!(stamps(false, Some(Condition::DEAD)));
        assert!(stamps(true, Some(Condition::NORMAL)));
        assert!(stamps(true, Some(Condition::COMBAT_IDLE)));
        assert!(stamps(true, None));
        assert!(!stamps(true, Some(Condition::WALKING)));
    }

    /// The AI tick's gate: an NPC off its route, or asking no walk, or
    /// walking under 1 cm a tick along its facing, stamps; one walking its
    /// route at 1 cm a tick or more does not, and a NaN step skips it.
    #[test]
    fn a_route_walker_keeps_its_footprint_off() {
        let ahead = Vec3::new(0.0, 0.0, 0.012);
        let walk = Vec3::new(0.0, 0.0, 1.0);
        assert!(npc_stamps(false, walk, ahead, Vec3::Z));
        assert!(npc_stamps(true, Vec3::ZERO, ahead, Vec3::Z));
        assert!(!npc_stamps(true, walk, ahead, Vec3::Z));
        // 1.2 cm sideways is 0 along the facing; 1.2 cm back is 1.2.
        assert!(npc_stamps(true, walk, Vec3::new(0.012, 0.0, 0.0), Vec3::Z));
        assert!(!npc_stamps(true, walk, -ahead, Vec3::Z));
        assert!(npc_stamps(true, walk, Vec3::new(0.0, 0.0, 0.0099), Vec3::Z));
        assert!(!npc_stamps(
            true,
            walk,
            Vec3::new(0.0, 0.0, f32::NAN),
            Vec3::Z
        ));
    }

    /// An NPC's footprint follows it from tick to tick (63 where it
    /// stands, 16 two cells along the axis where it stood); the walking
    /// player's is off; a character that leaves takes its footprint with
    /// it, and a new grid starts with none.
    #[test]
    fn footprints_follow_their_characters() {
        let mut app = App::new();
        app.insert_resource(grid())
            .init_resource::<NavFootprints>()
            .add_systems(Update, keep_footprints);
        let world = app.world_mut();
        let npc_root = world
            .spawn((
                CharacterBody::default(),
                Transform::from_xyz(0.15, 0.0, 0.15),
            ))
            .id();
        let npc = world
            .spawn((Fighter::default(), Condition::NORMAL, ChildOf(npc_root)))
            .id();
        let player_root = world
            .spawn((
                CharacterBody::default(),
                Transform::from_xyz(1.05, 0.0, 0.15),
            ))
            .id();
        world.spawn((
            Fighter::default(),
            Condition::WALKING,
            PlayerBody::new(0.0),
            ChildOf(player_root),
        ));
        app.update();
        let pen = |app: &App, x, z| {
            app.world()
                .resource::<NavGrid>()
                .cell(x, z, Layer::Lower)
                .penalty
        };
        // Cell 20 spans 0–30 cm; the player walks: no footprint at cell 23.
        assert_eq!(pen(&app, 20, 20), 63);
        assert_eq!(pen(&app, 21, 20), 32);
        assert_eq!(pen(&app, 23, 20), 0);
        app.world_mut()
            .entity_mut(npc_root)
            .insert(Transform::from_xyz(-0.45, 0.0, 0.15));
        app.update();
        assert_eq!(pen(&app, 20, 20), 16);
        assert_eq!(pen(&app, 18, 20), 63);
        app.world_mut().entity_mut(npc).despawn();
        app.update();
        let total = |app: &App| -> u32 {
            let g = app.world().resource::<NavGrid>();
            (0..40)
                .flat_map(|z| (0..40).map(move |x| (x, z)))
                .map(|(x, z)| u32::from(g.cell(x, z, Layer::Lower).penalty))
                .sum()
        };
        assert_eq!(total(&app), 0);
        // A new grid: the old stamps are forgotten, not taken off it.
        let npc = app
            .world_mut()
            .spawn((Fighter::default(), Condition::NORMAL, ChildOf(npc_root)))
            .id();
        app.update();
        app.world_mut().remove_resource::<NavGrid>();
        app.update();
        app.insert_resource(grid());
        app.update();
        assert_eq!(pen(&app, 18, 20), 63);
        assert_eq!(total(&app), 63 + 8 * 32 + 12 * 16);
        let at = app.world().resource::<NavFootprints>().of(npc);
        assert!(at.is_some_and(|p| (p - Vec3::new(-0.45, 0.0, 0.15)).length() < 1e-6));
    }

    /// An NPC standing within five cells of the grid's edge overflows: it
    /// stamps nothing and its body is removed from the scene; the player
    /// there stays; one standing inside keeps its footprint.
    #[test]
    fn an_npc_off_the_grid_is_removed_from_the_scene() {
        let mut app = App::new();
        app.insert_resource(grid())
            .init_resource::<NavFootprints>()
            .add_systems(Update, keep_footprints);
        // Cell = 20 + (x/0.3 - 0.5) rounded (ties to even): x = 4.8 m is
        // cell 35.5 -> 36, past 40 - 5: it overflows; x = 4.2 m is 33.5 ->
        // 34 and x = 3.9 m is 32.5 -> 32, inside.
        let spawn = |app: &mut App, x: f32, player: bool| {
            let root = app
                .world_mut()
                .spawn((CharacterBody::default(), Transform::from_xyz(x, 0.0, 0.15)))
                .id();
            let mut body =
                app.world_mut()
                    .spawn((Fighter::default(), Condition::NORMAL, ChildOf(root)));
            if player {
                body.insert(PlayerBody::new(0.0));
            }
            (root, body.id())
        };
        let (edge_root, edge) = spawn(&mut app, 4.8, false);
        let (inside_root, inside) = spawn(&mut app, 3.9, false);
        let (player_root, _) = spawn(&mut app, 4.8, true);
        app.update();
        assert!(app.world().get_entity(edge_root).is_err());
        assert!(app.world().get_entity(edge).is_err());
        assert!(app.world().get_entity(inside_root).is_ok());
        assert!(app.world().get_entity(player_root).is_ok());
        assert!(app.world().resource::<NavFootprints>().of(inside).is_some());
        assert!(overflows(
            app.world().resource::<NavGrid>(),
            Vec3::new(4.8, 0.0, 0.15)
        ));
        assert!(!overflows(
            app.world().resource::<NavGrid>(),
            Vec3::new(4.2, 0.0, 0.15)
        ));
        assert!(overflows(
            app.world().resource::<NavGrid>(),
            Vec3::new(0.0, 0.0, -4.7)
        ));
    }

    /// A standing player stamps; once vanished its footprint stays where
    /// it was while the body moves, and it follows the body again once the
    /// vanished bit clears.
    #[test]
    fn a_vanished_players_footprint_stays_put() {
        let mut app = App::new();
        app.insert_resource(grid())
            .init_resource::<NavFootprints>()
            .add_systems(Update, keep_footprints);
        let root = app
            .world_mut()
            .spawn((
                CharacterBody::default(),
                Transform::from_xyz(0.15, 0.0, 0.15),
            ))
            .id();
        let player = app
            .world_mut()
            .spawn((
                Fighter::default(),
                Condition::NORMAL,
                PlayerBody::new(0.0),
                ChildOf(root),
            ))
            .id();
        app.update();
        let pen = |app: &App, x, z| {
            app.world()
                .resource::<NavGrid>()
                .cell(x, z, Layer::Lower)
                .penalty
        };
        assert_eq!(pen(&app, 20, 20), 63);
        app.world_mut()
            .entity_mut(player)
            .insert(exa_combat::StateFlags(exa_combat::StateFlags::VANISHED));
        app.world_mut()
            .entity_mut(root)
            .insert(Transform::from_xyz(-0.45, 0.0, 0.15));
        app.update();
        // Still centred where it vanished: two cells along is its edge.
        assert_eq!(pen(&app, 20, 20), 63, "the footprint stays where it was");
        assert_eq!(pen(&app, 18, 20), 16);
        app.world_mut()
            .entity_mut(player)
            .insert(exa_combat::StateFlags::NONE);
        app.update();
        assert_eq!(pen(&app, 18, 20), 63);
        assert_eq!(pen(&app, 20, 20), 16);
    }
}
