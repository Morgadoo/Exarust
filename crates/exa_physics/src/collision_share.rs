//! The collision world on both sides of a free-running simulation.
//!
//! The simulation owns [`Physics`]: the level's colliders, the props'
//! stand-ins the bodies move, the world the characters and the bodies query.
//! The frame's picks (the hand, the cursor's ground, the spectator's
//! clicks, the GUI's drops) query [`FramePhysics`], a replica kept with the
//! same handles: every insert, removal and clear happens on both, in the
//! same order (the frame's are applied on the replica at once and posted to
//! the simulation; the simulation's are made in the step and sent to the
//! frame after it), and after a step that moved or changed anything the
//! frame takes the moved colliders' poses and a copy of the step's
//! broad-phase tree ([`share_collision`]). The frame's picks thus query the
//! world as the last finished step left it. The original's main thread
//! reads its simulation's scene directly, with no lock (its sector lock
//! only keeps the object lists stable); a pick against the last finished
//! step is the race-free form of that read.

use std::sync::atomic::{AtomicU64, Ordering};

use bevy::prelude::*;
use rapier3d::prelude::{Collider, ColliderHandle, Pose};

use crate::Physics;
use crate::sim_app::{FixedCommands, FrameCommands};

/// The frame's replica of the collision world, for its picks.
#[derive(Resource, Default)]
pub struct FramePhysics(pub Physics);

impl std::ops::Deref for FramePhysics {
    type Target = Physics;
    fn deref(&self) -> &Physics {
        &self.0
    }
}

impl std::ops::DerefMut for FramePhysics {
    fn deref_mut(&mut self) -> &mut Physics {
        &mut self.0
    }
}

/// What changed in the simulation's collision world since it last shared
/// it: colliders the step moved, colliders it removed, and whether the
/// frame's posted changes or the step touched the tree otherwise.
#[derive(Resource, Debug, Default)]
pub struct CollisionChanges {
    moved: Vec<ColliderHandle>,
    removed: Vec<ColliderHandle>,
    touched: bool,
}

impl CollisionChanges {
    /// The step moved `handle`.
    pub fn moved(&mut self, handle: ColliderHandle) {
        self.moved.push(handle);
    }

    /// The step removed `handle` (the frame removes it from its replica).
    pub fn removed(&mut self, handle: ColliderHandle) {
        self.removed.push(handle);
    }

    /// A posted change from the frame landed: the tree is sent again.
    pub fn touched(&mut self) {
        self.touched = true;
    }
}

/// Removes `handle` from the step's world and records it for the frame.
pub fn remove_collider(
    physics: &mut Physics,
    changes: &mut CollisionChanges,
    handle: ColliderHandle,
) {
    physics.0.remove_collider(handle);
    changes.removed(handle);
}

/// After a step that changed the collision world: the frame's replica takes
/// the removals, the moved colliders' poses and the step's broad-phase tree.
/// The replica's cost, logged every [`crate::sim_app::LOG_EVERY`] steps:
/// steps, steps shared, the step's side (µs) and the frame's (µs).
static COST: [AtomicU64; 4] = [const { AtomicU64::new(0) }; 4];

fn log_cost() {
    let steps = COST[0].load(Ordering::Relaxed);
    if steps == 0 || !steps.is_multiple_of(crate::sim_app::LOG_EVERY) {
        return;
    }
    let [_, shared, sim, frame] = [0, 1, 2, 3].map(|i| COST[i].swap(0, Ordering::Relaxed));
    COST[0].store(0, Ordering::Relaxed);
    info!(
        "collision replica: {shared} of {steps} steps shared, {:.1} us a step on the step's side, {:.1} us on the frame's",
        sim as f64 / steps as f64,
        frame as f64 / steps as f64
    );
}

pub fn share_collision(
    physics: Res<Physics>,
    mut changes: ResMut<CollisionChanges>,
    mut frame: ResMut<FrameCommands>,
) {
    let started = std::time::Instant::now();
    COST[0].fetch_add(1, Ordering::Relaxed);
    log_cost();
    // Colliders of awake rigid bodies moved with the step itself.
    let mut moved = std::mem::take(&mut changes.moved);
    for (_, body) in physics.0.active_bodies() {
        moved.extend(body.colliders().iter().copied());
    }
    let removed = std::mem::take(&mut changes.removed);
    // A debug aid (`EXA_SHARE_ALWAYS=1`): the tree goes every step, for the
    // replica's worst-case cost.
    static ALWAYS: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    let always = *ALWAYS.get_or_init(|| std::env::var("EXA_SHARE_ALWAYS").is_ok_and(|v| v == "1"));
    let touched = std::mem::take(&mut changes.touched) || always;
    if moved.is_empty() && removed.is_empty() && !touched {
        return;
    }
    moved.sort_unstable_by_key(|h| h.into_raw_parts());
    moved.dedup();
    let poses: Vec<(ColliderHandle, Pose)> = moved
        .into_iter()
        .filter_map(|h| physics.0.colliders.get(h).map(|c| (h, *c.position())))
        .collect();
    let tree = physics.0.broad_phase.clone();
    let bodies = physics.0.bodies.clone();
    COST[1].fetch_add(1, Ordering::Relaxed);
    COST[2].fetch_add(started.elapsed().as_micros() as u64, Ordering::Relaxed);
    frame.send(move |world| {
        let started = std::time::Instant::now();
        let Some(mut replica) = world.get_resource_mut::<FramePhysics>() else {
            return;
        };
        let replica = &mut replica.0.0;
        for h in removed {
            replica.remove_collider(h);
        }
        for (h, pose) in poses {
            if let Some(c) = replica.colliders.get_mut(h) {
                c.set_position(pose);
            }
        }
        replica.bodies = bodies;
        replica.broad_phase = tree;
        COST[3].fetch_add(started.elapsed().as_micros() as u64, Ordering::Relaxed);
    });
}

/// Inserts `colliders` on the frame's replica now and on the step's world
/// at the next step, in the same order (so both give the same handles).
/// Returns the replica's handles.
pub fn insert_colliders(
    replica: &mut FramePhysics,
    fixed: Option<&mut FixedCommands>,
    colliders: Vec<Collider>,
) -> Vec<ColliderHandle> {
    let handles: Vec<ColliderHandle> = colliders
        .iter()
        .map(|c| replica.0.0.insert_collider(c.clone(), None))
        .collect();
    let expected = handles.clone();
    post(fixed, move |world| {
        let Some(mut physics) = world.get_resource_mut::<Physics>() else {
            return;
        };
        for (c, want) in colliders.into_iter().zip(expected) {
            let got = physics.0.insert_collider(c, None);
            debug_assert_eq!(got, want, "the collision replica's handles went apart");
        }
        if let Some(mut changes) = world.get_resource_mut::<CollisionChanges>() {
            changes.touched();
        }
    });
    handles
}

/// Inserts `colliders` (already on the frame's replica, in this order) on
/// the step's world at the next step.
pub fn post_inserts(fixed: Option<&mut FixedCommands>, colliders: Vec<Collider>) {
    post(fixed, move |world| {
        let Some(mut physics) = world.get_resource_mut::<Physics>() else {
            return;
        };
        for c in colliders {
            physics.0.insert_collider(c, None);
        }
        if let Some(mut changes) = world.get_resource_mut::<CollisionChanges>() {
            changes.touched();
        }
    });
}

/// Removes `handles` from the frame's replica now and from the step's world
/// at the next step.
pub fn remove_colliders(
    replica: &mut FramePhysics,
    fixed: Option<&mut FixedCommands>,
    handles: Vec<ColliderHandle>,
) {
    for &h in &handles {
        replica.0.0.remove_collider(h);
    }
    post(fixed, move |world| {
        if let Some(mut physics) = world.get_resource_mut::<Physics>() {
            for h in handles {
                physics.0.remove_collider(h);
            }
        }
        if let Some(mut changes) = world.get_resource_mut::<CollisionChanges>() {
            changes.touched();
        }
    });
}

/// Empties the frame's replica now and the step's world at the next step.
pub fn clear(replica: &mut FramePhysics, fixed: Option<&mut FixedCommands>) {
    replica.0.clear();
    post(fixed, |world| {
        if let Some(mut physics) = world.get_resource_mut::<Physics>() {
            physics.clear();
        }
        if let Some(mut changes) = world.get_resource_mut::<CollisionChanges>() {
            *changes = CollisionChanges {
                touched: true,
                ..default()
            };
        }
    });
}

/// Runs `change` on the step's world at the next step; with no queue (a
/// test's bare app) at once is impossible, so the change is dropped there
/// and the replica alone moves.
fn post(fixed: Option<&mut FixedCommands>, change: impl FnOnce(&mut World) + Send + 'static) {
    if let Some(fixed) = fixed {
        fixed.post(change);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::system::RunSystemOnce;
    use rapier3d::prelude::ColliderBuilder;

    /// A step: the step's world steps, then shares, then the frame runs
    /// what was sent; posted changes from the frame land first.
    fn step(world: &mut World) {
        crate::sim_app::run_fixed_commands(world);
        world.resource_mut::<Physics>().0.step();
        world.run_system_once(share_collision).unwrap();
        crate::sim_app::run_frame_commands(world);
    }

    fn picks(physics: &Physics) -> Vec<Option<(ColliderHandle, f32)>> {
        let mut out = Vec::new();
        for x in -10..=10 {
            for z in -10..=10 {
                let origin = Vec3::new(x as f32 * 0.5, 5.0, z as f32 * 0.5);
                out.push(physics.fixed_pick(origin, Vec3::NEG_Y, 20.0));
            }
        }
        out
    }

    /// The frame's replica answers every pick as the step's world does,
    /// with the same handles, through the frame's inserts and removals and
    /// the step's moves and removals.
    #[test]
    fn the_replica_picks_as_the_steps_world() {
        let mut world = World::new();
        world.init_resource::<Physics>();
        world.init_resource::<FramePhysics>();
        world.init_resource::<CollisionChanges>();
        world.init_resource::<FixedCommands>();
        world.init_resource::<FrameCommands>();
        let boxes: Vec<Collider> = (0..12)
            .map(|i| {
                ColliderBuilder::cuboid(0.4, 0.2, 0.4)
                    .translation(Vec3::new(
                        (i % 4) as f32 * 1.2 - 2.0,
                        (i / 4) as f32 * 0.5,
                        0.0,
                    ))
                    .build()
            })
            .collect();
        let handles = world.resource_scope(|world, mut replica: Mut<FramePhysics>| {
            let mut fixed = world.resource_mut::<FixedCommands>();
            insert_colliders(&mut replica, Some(&mut fixed), boxes)
        });
        step(&mut world);
        let same = |world: &World| {
            picks(world.resource::<Physics>()) == picks(&world.resource::<FramePhysics>().0)
        };
        assert!(same(&world));
        let hit = picks(&world.resource::<FramePhysics>().0)
            .into_iter()
            .flatten()
            .count();
        assert!(hit > 0);
        // The step moves one and removes another.
        world.resource_scope(|world, mut physics: Mut<Physics>| {
            let mut changes = world.resource_mut::<CollisionChanges>();
            let c = physics.0.colliders.get_mut(handles[1]).unwrap();
            c.set_position(Pose::from_translation(Vec3::new(3.0, 1.0, 2.0)));
            changes.moved(handles[1]);
            remove_collider(&mut physics, &mut changes, handles[5]);
        });
        step(&mut world);
        assert!(same(&world));
        // The frame removes one and inserts one, which takes the same handle
        // on both sides.
        let new = world.resource_scope(|world, mut replica: Mut<FramePhysics>| {
            let mut fixed = world.resource_mut::<FixedCommands>();
            remove_colliders(&mut replica, Some(&mut fixed), vec![handles[2]]);
            insert_colliders(
                &mut replica,
                Some(&mut fixed),
                vec![
                    ColliderBuilder::ball(0.5)
                        .translation(Vec3::new(-1.0, 0.0, 1.5))
                        .build(),
                ],
            )
        });
        step(&mut world);
        assert!(same(&world));
        assert!(
            world
                .resource::<Physics>()
                .0
                .colliders
                .get(new[0])
                .is_some()
        );
        assert_eq!(
            world.resource::<Physics>().collider_count(),
            world.resource::<FramePhysics>().collider_count()
        );
    }
}
