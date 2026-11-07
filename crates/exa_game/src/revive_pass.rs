//! The story revive's world pass over the AI characters: the behaviour's
//! reset each brain gets (the original's behaviour method slot 26, whose
//! only caller is that pass; the player's own slot is empty).
//!
//! For each AI character that is neither collapsing or dead nor dormant
//! (`exa_ai::presence::reenter`): its current health returns to its lasting
//! pool when that is over 0.01, its contacts and target are forgotten, and
//! a roaming one (activity 1) that is not a statue is moved along freshly
//! planned wander and home-side plans (twice, each plan kept only when its
//! path spans over 150 cm), placed there, and asked the scene's "is the
//! space free" query (`exa_physics::sector_lists::space_taken`): a body
//! that meets another is moved to the navigation grid's nearest walkable
//! point and placed again. It then leaves combat, stops following, and its
//! idle state restarts (the idle return).
//!
//! Exarust's differences: the plans start from the character root's ground
//! point and are routed at its height, as the roam's own plans are; the
//! nearest walkable point is asked from the root's point (the original
//! asks from the standing point at the pelvis's height); the footprint's
//! lift and re-stamp, the neighbour list refresh a 0x2000000 roles flag
//! asks, the bodies' wake and the removed bodies' contact drop the pass
//! also runs are not reproduced.

use bevy::math::Affine3A;
use bevy::prelude::*;
use exa_ai::Duellist;
use exa_ai::brain::Npc;
use exa_ai::combatant::Combatant;
use exa_combat::{Condition, Fighter, StateFlags};
use exa_physics::body::Ragdoll;
use exa_physics::sim_app::FixedCommands;
use exa_world::room_view::LevelRooms;

/// The AI characters the pass resets.
type Brains<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static GlobalTransform,
        Option<&'static ChildOf>,
        &'static mut Npc,
        &'static mut Fighter,
        &'static mut Combatant,
        &'static Condition,
        Option<&'static StateFlags>,
        Option<&'static mut Duellist>,
    ),
>;

/// The character roots the pass moves.
type Roots<'w, 's> = Query<'w, 's, &'static mut Transform, Without<Npc>>;

/// The squared flat span (cm²) from a path's first point to its last.
fn span2(points: &[Vec3]) -> f32 {
    exa_ai::route::route_span2(points)
}

/// The world pass's brain reset ([`crate::flow::Step::ReviveBrains`]),
/// taken out of the flow's steps before they are applied. See the module.
#[allow(clippy::too_many_arguments)]
pub fn reset_brains(
    mut queued: ResMut<crate::flow::FlowSteps>,
    mut brains: Brains,
    mut roots: Roots,
    mut bodies: Query<(Entity, &mut Ragdoll)>,
    level: Option<Res<LevelRooms>>,
    nav: Option<Res<exa_world::nav::NavGrid>>,
    mut fixed: ResMut<FixedCommands>,
) {
    let Some(at) = queued
        .steps
        .iter()
        .position(|step| *step == crate::flow::Step::ReviveBrains)
    else {
        return;
    };
    queued.steps.remove(at);
    let (mut reset, mut moved, mut refused) = (0, 0, 0);
    for (entity, gt, parent, mut npc, mut fighter, mut combatant, condition, flags, duellist) in
        &mut brains
    {
        let npc = &mut *npc;
        let roaming = npc
            .applied
            .as_ref()
            .is_some_and(|a| a.activity == exa_ai::activity::activities::ROAM);
        let room = npc.level_room;
        let avoid = npc.previous_room;
        let exits = || match level.as_deref() {
            Some(level) if room >= 0 => exa_ai::rooms::wander_exits(&level.rooms, room as u16),
            _ => Vec::new(),
        };
        let mut roll = npc.roll;
        let toward = std::cell::Cell::new(npc.walk.toward);
        let route = |from: Vec3, goal: Vec3| -> Vec<Vec3> {
            nav.as_deref().map_or_else(Vec::new, |grid| {
                use exa_ai::route::Router;
                exa_ai::route::NavRouter { grid, size: 0 }
                    .route(from, Vec3::new(goal.x, from.y, goal.z))
            })
        };
        let plan = |points: Vec<Vec3>| -> Option<(Vec3, f32)> {
            (points.len() > 1).then(|| (*points.last().unwrap(), span2(&points)))
        };
        let mut wander = |from: Vec3| {
            let picked = exa_ai::idle::pick_wander(room, &exits(), avoid, &mut roll)?;
            toward.set(picked.node);
            plan(route(from, picked.goal))
        };
        let mut home = |from: Vec3| {
            let picked = exa_ai::idle::pick_home(room, &exits(), toward.get())?;
            plan(route(from, picked.goal))
        };
        let from = gt.translation() * 100.0;
        let done = exa_ai::presence::reenter(
            condition.dying(),
            flags.is_some_and(|f| f.has(StateFlags::DORMANT)),
            roaming,
            npc.statue,
            &mut fighter,
            &mut combatant,
            from,
            &mut wander,
            &mut home,
        );
        npc.roll = roll;
        npc.walk.toward = toward.get();
        if done.cleared {
            reset += 1;
        }
        if !done.left_combat {
            continue;
        }
        // Placed where the plans ended (or where it stands), then asked
        // whether the space is free.
        let target = done.moved_to.map_or(gt.translation(), |cm| {
            Vec3::new(cm.x / 100.0, gt.translation().y, cm.z / 100.0)
        });
        let mut shift = target - gt.translation();
        let taken = bodies.get(entity).ok().is_some_and(|(_, body)| {
            let worlds: Vec<Affine3A> = body
                .parts
                .iter()
                .map(|p| Affine3A::from_translation(shift) * p.transform())
                .collect();
            let others = bodies.iter();
            exa_physics::sector_lists::space_taken(entity, body, &worlds, others)
        });
        if taken && let Some(grid) = nav.as_deref() {
            let free = exa_world::nav::walkable::nearest_walkable(grid, target);
            shift = Vec3::new(free.x, gt.translation().y, free.z) - gt.translation();
            refused += 1;
        }
        if shift != Vec3::ZERO {
            moved += 1;
            if let Some(mut root) = parent.and_then(|p| roots.get_mut(p.parent()).ok()) {
                root.translation += shift;
            }
            if let Ok((_, mut body)) = bodies.get_mut(entity) {
                fixed.write(entity, &mut body, move |body| {
                    let worlds: Vec<Affine3A> = body
                        .parts
                        .iter()
                        .map(|p| Affine3A::from_translation(shift) * p.transform())
                        .collect();
                    body.place_at(&worlds);
                });
            }
        }
        // It leaves combat, stops following and its idle state restarts.
        if let Some(mut duellist) = duellist {
            duellist.leave();
        }
        combatant.in_combat = false;
        npc.walk.route.clear();
        npc.walk.velocity = Vec3::ZERO;
        let body_look = exa_ai::motion::yaw_of(gt.rotation() * Vec3::Z);
        exa_ai::lifecycle::return_to_idle(&mut npc.gaze, &mut npc.turn, body_look);
        exa_ai::lifecycle::restart_idle(npc);
    }
    info!(
        "revive pass: {reset} brain(s) reset, {moved} moved, {refused} refused by the space-free query"
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The pass takes its step, heals and clears every awake brain, leaves
    /// a collapsing one alone, and a roaming brain with no room to plan in
    /// leaves combat where it stands (its space free: no other body).
    #[test]
    fn the_revive_pass_resets_every_awake_brain() {
        let mut app = App::new();
        exa_physics::sim_app::add_fixed_commands(&mut app);
        app.init_resource::<crate::flow::FlowSteps>()
            .add_systems(Update, reset_brains);
        app.world_mut()
            .resource_mut::<crate::flow::FlowSteps>()
            .steps = vec![crate::flow::Step::ReviveBrains];
        let spawn = |app: &mut App, condition: Condition, roam: bool| {
            let mut npc = Npc::new([0.5; 4]);
            if roam {
                npc.applied = Some(exa_ai::activity::Applied {
                    activity: exa_ai::activity::activities::ROAM,
                    idle_activity: 0,
                    state: exa_ai::activity::IdleState::Roam,
                    leaves_party: false,
                    follow_band_cm: None,
                    phase_word: 0,
                    create_idle: None,
                    drop_idle: false,
                    skipped: Vec::new(),
                });
            }
            let mut fighter = Fighter::default();
            fighter.health.yellow = 0.2;
            fighter.health.red = 0.6;
            let mut combatant = Combatant::default();
            combatant.contact_index(Entity::PLACEHOLDER);
            combatant.in_combat = true;
            app.world_mut()
                .spawn((
                    GlobalTransform::from_translation(Vec3::new(1.0, 0.0, 2.0)),
                    npc,
                    fighter,
                    combatant,
                    condition,
                ))
                .id()
        };
        let awake = spawn(&mut app, Condition::default(), true);
        let falling = spawn(&mut app, Condition::COLLAPSING, true);
        app.update();
        assert!(
            app.world()
                .resource::<crate::flow::FlowSteps>()
                .steps
                .is_empty()
        );
        let world = app.world();
        assert_eq!(world.get::<Fighter>(awake).unwrap().health.yellow, 0.6);
        let combatant = world.get::<Combatant>(awake).unwrap();
        assert!(combatant.contacts.is_empty() && !combatant.in_combat);
        assert_eq!(world.get::<Fighter>(falling).unwrap().health.yellow, 0.2);
        assert_eq!(world.get::<Combatant>(falling).unwrap().contacts.len(), 1);
    }

    #[test]
    fn a_span_is_measured_flat_from_the_first_point() {
        assert_eq!(
            span2(&[
                Vec3::ZERO,
                Vec3::new(50.0, 9.0, 0.0),
                Vec3::new(30.0, 99.0, 40.0)
            ]),
            2500.0
        );
    }
}
