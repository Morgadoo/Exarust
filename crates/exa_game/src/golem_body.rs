//! The golem guardians' bodies: the sentry state that keeps a statue golem
//! at rest until an alarm wakes it.
//!
//! The original builds a golem's body with the same character builder as
//! any other race's (its race model, the shared skeleton's links, the
//! character init's golem presets), so Exarust gives the golem races the
//! physical body of the human frame ([`crate::scene_bodies::physical_frame`]).
//! A placed golem starts in Stand 2 ([`exa_ai::golem::start_motion`]).
//!
//! Each body tick, in the brains' chain (`exa_ai::BrainChain`), a brain in
//! the sentry idle state that is not fighting runs
//! the sentry tick ([`exa_ai::golem::sentry_rest`]):
//!
//! - a statue (its behaviour's statue byte set) playing the motion it asks
//!   for rests: its body's pose word becomes 2 and its statue rest is set
//!   ([`StatueRest`]), which the posture stage turns into the body's
//!   dormant drive (its tree counts its sleep down by one a tick and its
//!   limbs keep 0.95 of their retention), so it sleeps 255 ticks later;
//!   anything else stands (pose 1, not still);
//! - a statue whose state flags lack the latch
//!   ([`exa_ai::golem::SLEEP_SCALES_LATCH`], the same bit as
//!   [`exa_combat::StateFlags::TURNS_BLOWS`]) takes it and the sleep
//!   scales: every part's contact mobility becomes 0.05 of its base, then
//!   each held body (unarmed, the two fists) 0.02 of that
//!   ([`exa_ai::golem::sleep_scales`]);
//! - a body not playing Stand 2 is asked to.
//!
//! The sleep activity's entry sets the same statue rest on a sleeper's
//! body and its knock-out clears it ([`rest_sleepers`]).
//!
//! The wake ([`crate::golem_alarm`]) clears the statue byte, so the next
//! sentry tick stands the body, and undoes the scales
//! ([`exa_ai::golem::wake_scales`], [`undo_sleep_scales`]).
//!
//! Evidence: the sentry state routine, the character spawn's initial motion
//! and the golem wake of `Exanima.exe`, checked against their
//! decompilations. Not reproduced: the sentry tick's stop of a running
//! locomotion and its copies of the body's place into its move goals
//! (Exarust's sentry does not walk); the requested motion is the clip the
//! body plays (Exarust's clip player starts a request at once, so the
//! current and requested motions are always equal).

use bevy::prelude::*;
use exa_ai::golem::{self, STAND_MOTION};
use exa_ai::{Duellist, brain::Npc};
use exa_anim::{Animator, Motions};
use exa_combat::StateFlags;
use exa_physics::body::Ragdoll;
use exa_physics::part::PartState;

/// A body the sentry tick rests: a statue standing in its motion (the
/// original's pose word 2 and statue byte on the body). The posture stage
/// gives such a body the dormant drive.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct StatueRest(pub bool);

pub struct GolemBodyPlugin;

impl Plugin for GolemBodyPlugin {
    fn build(&self, app: &mut App) {
        // The sentry is an idle state of the AI tick: it runs in the
        // brains' tick, every body tick, after the walks.
        app.add_systems(
            exa_ai::BrainChain,
            (
                rest_sleepers
                    .after(exa_ai::brain::ai_tick)
                    .before(stand_sentries),
                stand_sentries
                    .after(exa_ai::brain::idle_walk)
                    .after(crate::npc_search::follow_and_search),
            ),
        )
        .add_systems(Update, trace_guardians);
    }
}

/// The parts that count as the body's held bodies: an unarmed body holds
/// its fists (the parts with the weapon bit).
pub fn held_parts(body: &Ragdoll) -> Vec<usize> {
    body.parts
        .iter()
        .enumerate()
        .filter(|(_, p)| p.state.has(PartState::STRIKING))
        .map(|(i, _)| i)
        .collect()
}

/// Every part's contact mobility base.
fn contact_bases(body: &Ragdoll) -> Vec<f32> {
    (0..body.parts.len())
        .map(|i| body.contact_base(i).unwrap_or(0.0))
        .collect()
}

/// Puts the sleep scales on a statue's body.
pub fn apply_sleep_scales(body: &mut Ragdoll) {
    let scaled = golem::sleep_scales(&contact_bases(body), &held_parts(body));
    for (part, mobility) in body.parts.iter_mut().zip(scaled) {
        part.pairing.mobility = mobility;
    }
}

/// Takes the sleep scales off a woken guardian's body.
pub fn undo_sleep_scales(body: &mut Ragdoll) {
    let woken = golem::wake_scales(&contact_bases(body), &held_parts(body));
    for (part, mobility) in body.parts.iter_mut().zip(woken) {
        part.pairing.mobility = mobility;
    }
}

/// The sleep activity's statue byte and gait word on the body (both kept
/// as its statue rest): its entry sets them (gait 2), a knock-out clears
/// them (gait 0), as the brain reports ([`exa_ai::brain::StatueByte`]).
pub fn rest_sleepers(
    mut commands: Commands,
    mut changed: MessageReader<exa_ai::brain::StatueByte>,
    mut rests: Query<&mut StatueRest>,
) {
    for change in changed.read() {
        match rests.get_mut(change.entity) {
            Ok(mut rest) => {
                if rest.0 != change.set {
                    rest.0 = change.set;
                }
            }
            Err(_) => {
                if let Ok(mut entity) = commands.get_entity(change.entity) {
                    entity.insert(StatueRest(change.set));
                }
            }
        }
        debug!(
            "{:?}: the statue rest {} with the sleep",
            change.entity,
            if change.set { "is set" } else { "clears" }
        );
    }
}

/// The sentry tick of every brain standing sentry out of combat.
#[allow(clippy::type_complexity)]
pub fn stand_sentries(
    mut commands: Commands,
    motions: Option<Res<Motions>>,
    mut sentries: Query<(
        Entity,
        &Npc,
        &mut Animator,
        Option<&Duellist>,
        Option<&mut Ragdoll>,
        Option<&mut StateFlags>,
        Option<&mut StatueRest>,
    )>,
) {
    let Some(motions) = motions else { return };
    for (entity, npc, mut anim, duellist, body, flags, rest) in &mut sentries {
        if npc.routine.kind != exa_ai::idle::Kind::Sentry
            || duellist.is_some_and(|d| d.engaged || d.fighting())
        {
            continue;
        }
        let current = motions
            .0
            .action_index(&anim.action)
            .map_or(-1, |i| i as i32);
        let latch = flags
            .as_deref()
            .is_some_and(|f| f.has(golem::SLEEP_SCALES_LATCH));
        let tick = golem::sentry_rest(npc.statue, current, current, latch);
        // The scales need the body and the flag word that latches them.
        if tick.scales
            && let (Some(mut body), Some(mut flags)) = (body, flags)
        {
            flags.0 |= golem::SLEEP_SCALES_LATCH;
            apply_sleep_scales(&mut body);
            debug!("{entity:?}: the sentry puts the sleep scales on the statue");
        }
        if tick.request == Some(STAND_MOTION)
            && let Some(name) = motions.action_name(STAND_MOTION as u32)
        {
            let name = name.to_string();
            anim.play_loop(&name);
        }
        match rest {
            Some(mut rest) => {
                if rest.0 != tick.still {
                    rest.0 = tick.still;
                }
            }
            None => {
                commands.entity(entity).insert(StatueRest(tick.still));
            }
        }
    }
}

/// Frames between two lines of [`trace_guardians`].
const TRACE_FRAMES: u32 = 60;

/// A debug trace of the guardians (`RUST_LOG=exa_game::golem_body=debug`):
/// every [`TRACE_FRAMES`] frames, each brain whose statue byte is clear and
/// whose race word carries the AI bit logs where it stands (cm), its clip,
/// whether its duellist is engaged and fighting, and whether its body is
/// at rest; one line sums up the statues: how many, how many rest, how many
/// have a simulated body and how many of those sleep, and the farthest any
/// has moved from where it was first traced.
#[allow(clippy::type_complexity)]
fn trace_guardians(
    mut frame: Local<u32>,
    mut first: Local<bevy::platform::collections::HashMap<Entity, Vec3>>,
    guardians: Query<(
        Entity,
        &Npc,
        &exa_character::record::CharacterData,
        &GlobalTransform,
        Option<&Animator>,
        Option<&Duellist>,
        Option<&StatueRest>,
        Option<&Ragdoll>,
    )>,
) {
    *frame = frame.wrapping_add(1);
    if !(*frame).is_multiple_of(TRACE_FRAMES) {
        return;
    }
    let (mut statues, mut resting, mut farthest) = (0, 0, 0.0f32);
    let (mut bodies, mut asleep) = (0, 0);
    for (entity, npc, data, gt, anim, duellist, rest, body) in &guardians {
        if data.record.profession() & golem::AI_BIT == 0 {
            continue;
        }
        let at = gt.translation() * 100.0;
        let from = *first.entry(entity).or_insert(at);
        if npc.statue {
            statues += 1;
            resting += usize::from(rest.is_some_and(|r| r.0));
            farthest = farthest.max(at.distance(from));
            bodies += usize::from(body.is_some());
            asleep += usize::from(body.is_some_and(Ragdoll::asleep));
            continue;
        }
        debug!(
            "guardian {entity:?} frame {}: at ({:.0}, {:.0}, {:.0}) cm, clip {:?}, engaged {}, fighting {}, at rest {}",
            *frame,
            at.x,
            at.y,
            at.z,
            anim.map(|a| a.action.as_str()),
            duellist.is_some_and(|d| d.engaged),
            duellist.is_some_and(|d| d.fighting()),
            rest.is_some_and(|r| r.0)
        );
    }
    if statues > 0 {
        debug!(
            "statues frame {}: {statues}, {resting} at rest, {bodies} with a body, {asleep} asleep, \
             farthest moved {farthest:.1} cm",
            *frame
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use exa_ai::personality::{AI_FLAG, STATUE_FLAG};
    use exa_formats::characters::CharacterRecord;
    use exa_formats::rsq::{Action, MotionSet};
    use std::sync::Arc;

    fn motions() -> Motions {
        let action = |name: &str| Action {
            name: name.into(),
            params: [0, 0],
        };
        let mut actions: Vec<Action> = (0..0x1a).map(|i| action(&format!("A{i}"))).collect();
        actions[0x13] = action("Idle");
        actions[0x19] = action("Stand 2");
        let set = MotionSet {
            bone_count: 18,
            actions,
            movesets: Vec::new(),
        };
        Motions::new(Arc::new(set), Arc::default(), 0.0)
    }

    fn app() -> App {
        let mut app = App::new();
        app.insert_resource(motions())
            .add_systems(Update, stand_sentries);
        app
    }

    /// A sentry golem's brain: a statue when `statue`.
    fn sentry(statue: bool) -> Npc {
        let record = CharacterRecord {
            race: 16,
            profession_rest: AI_FLAG | if statue { STATUE_FLAG } else { 0 },
            ..CharacterRecord::default()
        };
        let mut roll = 0;
        let mut npc = Npc::of_record(&record, &mut roll);
        npc.routine = exa_ai::idle::Routine::sentry();
        npc
    }

    /// A statue standing in Stand 2 rests and latches its flags; a woken
    /// one stands; a sentry playing Idle is asked for Stand 2.
    #[test]
    fn a_statue_rests_and_a_woken_guardian_stands() {
        let mut app = app();
        let statue = app
            .world_mut()
            .spawn((
                sentry(true),
                Animator::new("Golem", "Stand 2"),
                StateFlags::default(),
            ))
            .id();
        let woken = app
            .world_mut()
            .spawn((sentry(false), Animator::new("Golem", "Idle")))
            .id();
        app.update();
        let w = app.world();
        assert_eq!(w.get::<StatueRest>(statue), Some(&StatueRest(true)));
        // No body yet: the latch waits for it.
        assert_eq!(w.get::<StateFlags>(statue).unwrap().0, 0);
        assert_eq!(w.get::<StatueRest>(woken), Some(&StatueRest(false)));
        assert_eq!(w.get::<Animator>(woken).unwrap().action, "Stand 2");
        // Once woken (the statue byte cleared), the statue stands.
        app.world_mut().get_mut::<Npc>(statue).unwrap().statue = false;
        app.update();
        assert_eq!(
            app.world().get::<StatueRest>(statue),
            Some(&StatueRest(false))
        );
    }

    /// A brain that is not standing sentry is left alone.
    #[test]
    fn a_roaming_brain_is_not_a_sentry() {
        let mut app = app();
        let mut npc = sentry(true);
        npc.routine = exa_ai::idle::Routine::roam(0);
        let roamer = app
            .world_mut()
            .spawn((npc, Animator::new("Golem", "Idle")))
            .id();
        app.update();
        assert!(app.world().get::<StatueRest>(roamer).is_none());
        assert_eq!(app.world().get::<Animator>(roamer).unwrap().action, "Idle");
    }

    /// A body of a pelvis (bone 1) and a right hand (bone 17) marked a fist.
    fn body() -> Ragdoll {
        use exa_physics::body::PartSetup;
        let part = |bone: usize, parent: Option<usize>| PartSetup {
            bone,
            entity: Entity::PLACEHOLDER,
            parent,
            world: bevy::math::Affine3A::IDENTITY,
            centre: Vec3::ZERO,
            half: Vec3::splat(0.1),
            origin: Vec3::ZERO,
            links: vec![],
            foot: None,
            pins: Vec::new(),
            spheres: None,
        };
        let mut body = Ragdoll::new(&[part(1, None), part(17, Some(0))]).with_drive();
        assert!(body.set_fist(17, true));
        body
    }

    /// The statue's sleep scales: the pelvis keeps 0.05 of its base (0.45
    /// × ⅔), the fist 0.05 × 0.02 of its own (2.5 × ⅔); the wake gives the
    /// pelvis its base back and the fist fifty times its base.
    #[test]
    fn the_sleep_scales_on_a_statue_body() {
        let mut b = body();
        assert_eq!(held_parts(&b), vec![1]);
        let (pelvis, hand) = (b.contact_base(0).unwrap(), b.contact_base(1).unwrap());
        assert_eq!(pelvis, 0.45 * (2.0 / 3.0));
        assert_eq!(hand, 2.5 * (2.0 / 3.0));
        apply_sleep_scales(&mut b);
        assert_eq!(b.parts[0].pairing.mobility, (pelvis as f64 * 0.05) as f32);
        let slept = (hand as f64 * 0.05) as f32;
        assert_eq!(b.parts[1].pairing.mobility, (slept as f64 * 0.02) as f32);
        undo_sleep_scales(&mut b);
        assert_eq!(b.parts[0].pairing.mobility, pelvis);
        assert_eq!(b.parts[1].pairing.mobility, hand * 50.0);
    }

    /// With its body and flags, the statue's first sentry tick latches the
    /// flags (the blow-turning bit) and scales the body, once.
    #[test]
    fn the_first_sentry_tick_latches_the_statue() {
        let mut app = app();
        let statue = app
            .world_mut()
            .spawn((
                sentry(true),
                Animator::new("Golem", "Stand 2"),
                StateFlags::default(),
                body(),
            ))
            .id();
        app.update();
        let flags = *app.world().get::<StateFlags>(statue).unwrap();
        assert!(flags.has(golem::SLEEP_SCALES_LATCH));
        assert!(flags.has(StateFlags::TURNS_BLOWS));
        let once = app.world().get::<Ragdoll>(statue).unwrap().parts[0]
            .pairing
            .mobility;
        app.update();
        let twice = app.world().get::<Ragdoll>(statue).unwrap().parts[0]
            .pairing
            .mobility;
        assert_eq!(once, twice);
    }
}
