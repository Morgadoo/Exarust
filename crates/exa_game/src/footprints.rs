//! The characters' footprints on the navigation grid
//! (`exa_world::nav::footprint`: 63 under a character, 32 round it, 16
//! beyond along the axes, on the penalty bytes the planner and the obstacle
//! test pay for), kept each fixed tick in the behaviour stage, as the
//! original's behaviours lift and stamp them where the character stands
//! before it moves: an NPC's every tick (collapsing or dead too, while it
//! keeps its controller); the player's only while it is not walking (any
//! condition but the walk's), so a standing or fighting player is in an
//! NPC's way and a walking one is not. An NPC walking a planned route
//! (a roam or patrol leg, a flight, a chase) at 1 cm a tick or more keeps
//! its footprint off, as the original's AI tick gates its stamp
//! ([`exa_ai::brain::stamps_footprint`]). A character that leaves takes its
//! footprint with it, and a new level's grid starts with none. Exarust's
//! standing point is the feet.

use std::collections::HashSet;

use bevy::prelude::*;
use exa_ai::Duellist;
use exa_ai::brain::{self, Npc};
use exa_combat::{Condition, Fighter};
use exa_physics::CharacterBody;
use exa_world::nav::{NavFootprints, NavGrid};

use crate::player::PlayerBody;

pub struct FootprintPlugin;

impl Plugin for FootprintPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<NavFootprints>().add_systems(
            FixedPreUpdate,
            keep_footprints.after(crate::player::steer_player),
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

/// Every character under its controller moves its footprint to where its
/// feet are, or takes it off; the footprints of characters gone (or off
/// their controller) come off too.
#[allow(clippy::type_complexity)]
pub fn keep_footprints(
    grid: Option<ResMut<NavGrid>>,
    mut prints: ResMut<NavFootprints>,
    characters: Query<
        (
            Entity,
            &ChildOf,
            Option<&Condition>,
            Has<PlayerBody>,
            Option<&Npc>,
            Option<&Duellist>,
            Option<&Transform>,
        ),
        With<Fighter>,
    >,
    roots: Query<(&CharacterBody, &Transform)>,
    mut seen: Local<HashSet<Entity>>,
    mut walking: Local<HashSet<Entity>>,
) {
    let Some(mut grid) = grid else { return };
    if grid.is_added() {
        prints.forget();
    }
    seen.clear();
    for (who, parent, condition, player, npc, duellist, model) in &characters {
        let Ok((capsule, at)) = roots.get(parent.parent()) else {
            continue;
        };
        seen.insert(who);
        let on = if player {
            stamps(true, condition.copied())
        } else {
            let facing = model.map_or(Vec3::Z, |t| t.rotation * Vec3::Z);
            let following = brain::follows_route(npc, duellist);
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
        if on {
            prints.stamp(&mut grid, who, capsule.feet(at));
        } else {
            prints.lift(&mut grid, who);
        }
    }
    walking.retain(|e| seen.contains(e));
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
}
