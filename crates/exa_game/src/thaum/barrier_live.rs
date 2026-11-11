//! The force barrier in the running game: the dome entity its cast makes,
//! ticked on its caster and pushing the scene's other bodies out.
//!
//! - **Start** ([`start_barrier`]): the creator hands the power's cost to
//!   the node as its rate and the radius 100; the start plays cast motion
//!   0x23 with the power id 0x10205, and when that motion starts the
//!   caster's cast hook becomes the barrier's, which ends the barrier (state
//!   2) and answers −1. The dome then rises over the caster
//!   ([`Barrier::cast`]).
//! - **The hook** ([`hook_called`]): a later cast's start calls it (it
//!   replaces the hook and the power id), and so do the player's buttons and
//!   the body tick's release (pressing the barrier again), which end the
//!   cast: the hook and power id go.
//! - **Each tick** (after the body tick): while under state 2 the barrier
//!   pays its rate from the caster's focus, half of it once standing; when
//!   what is left falls short the caster's hook is called (the barrier
//!   ends) and the cast ends. Then [`Barrier::tick`] follows the caster's
//!   place and facing; at state 4 the entity leaves the scene.
//! - **Keeping out** ([`keep_out`]): every other body's limb spheres are
//!   taken into the cap's frame and pushed by [`Barrier::part_push`] (the
//!   part's mass its own), each push striking a ripple; a character any of
//!   whose parts were pushed takes [`Barrier::contact`]: a 40 cm shove and
//!   a knock. [inf: the cap's frame here is the dome's centre with z along
//!   the caster's flattened facing and y up; the original reads the scene
//!   node's matrix. The shove is a knockback step along the shove, as the
//!   blast's knock is, and the contact runs once a body, not once a part.]
//!
//! The cast's `thmfrc_ss`, the hum and the hits' `thmfrc_bi` are queued on
//! [`LiveBarrier::sounds`] and played by [`super::dome_sound`]; the cap is
//! drawn by [`crate::effects::dome_draw`].
//!
//! Each pushed part's contact also counts a knock on the character's
//! instability counter, drains its energy (0.001 a contact, more beyond
//! 100 cm), drops its AI's route, and a held weapon's struck part breaks
//! its holder's attack ([`apply_contact`]).
//!
//! Evidence: the barrier's creator, start, hook, tick and keep-out, the
//! cast-motion start, the spend and the cast end, read from `Exanima.exe`.

use bevy::math::Affine3A;
use bevy::prelude::*;
use exa_physics::body::locomotion::Locomotor;
use exa_physics::body::{BodyTicks, Ragdoll};

use super::barricade::WAKE_TICKS;
use super::blast_live::{LimbSphere, bounds_miss, gather_limbs};
use super::cast::PowerCaster;
use super::casting::{self, CastBody, CastHook, spend_tick};
use super::dome_sound::{DomeSet, DomeSounds};
use super::live::{EffectDraws, EffectRoll};
use crate::effects::force_dome::{
    BARRIER_BALANCE_STEP, Barrier, CONTACT_PUSH, DomeHit, Phase, Sound, balance_after,
};

/// Centimetres a metre.
const CM: f32 = 100.0;
/// The barrier's power id and its cast motion.
pub const BARRIER: u32 = 0x10205;
pub const BARRIER_MOTION: u32 = 0x23;
/// The barrier's hook: −1 to every event.
pub const BARRIER_HOOK: CastHook = CastHook::NEVER_KEEPS;
/// The barrier's radius, cm.
pub const BARRIER_RADIUS: f32 = 100.0;

/// A barrier in the scene.
#[derive(Component, Clone, Debug)]
pub struct LiveBarrier {
    pub barrier: Barrier,
    pub caster: Entity,
    /// The caster's cast hook is this barrier's.
    pub hooked: bool,
    /// The cast generation whose hook the barrier took.
    pub generation: u64,
    /// The sounds it queued and its hum.
    pub sounds: DomeSounds,
}

/// The barrier's start on its caster: cast motion 0x23 with the power id,
/// and the hook when the motion starts.
pub fn start_barrier(body: &mut CastBody) -> bool {
    let started = casting::start_cast(body, BARRIER_MOTION, BARRIER);
    if started {
        body.cast_hook = Some(BARRIER_HOOK);
    }
    started
}

/// The barrier's hook: it ends the barrier.
fn call_hook(barrier: &mut Barrier) {
    barrier.dome.phase = Phase::Ending;
}

/// Whether the caster called the barrier's hook since the last tick: it is
/// no longer the barrier's (a call answers −1 and ends the cast, or a later
/// cast's start replaced it), and the barrier ends.
pub fn hook_called(live: &mut LiveBarrier, body: &mut CastBody) -> bool {
    if !live.hooked {
        return false;
    }
    let called = body.cast_generation != live.generation
        || body.casting != BARRIER
        || body.cast_hook != Some(BARRIER_HOOK);
    if called {
        info!(
            "force barrier: its hook was called (events asked {:#x}, casting {:#x}), it ends",
            body.hook_asked, body.casting
        );
        live.hooked = false;
        call_hook(&mut live.barrier);
    }
    called
}

/// The tick's focus: under state 2 the rate, half of it once standing. A
/// shortfall calls the caster's hook, which is the barrier's while it holds
/// it, and ends the cast.
pub fn pay(live: &mut LiveBarrier, body: &mut CastBody) {
    let dome = &live.barrier.dome;
    if dome.phase >= Phase::Ending {
        return;
    }
    let drain = if dome.phase == Phase::Growing {
        live.barrier.rate
    } else {
        live.barrier.rate * 0.5
    };
    if spend_tick(body, drain) {
        info!(
            "force barrier: focus {:.4} spent {:.4} falls short of {drain:.4}, it ends",
            body.focus, body.spent
        );
        if live.hooked {
            call_hook(&mut live.barrier);
            live.hooked = false;
        }
        casting::end_cast(body);
    }
}

/// The cap's frame: the dome's centre, z along the flattened facing, y up.
pub(crate) fn cap_frame(at: Vec3, facing: Vec3) -> Affine3A {
    let z = Vec3::new(facing.x, 0.0, facing.z)
        .try_normalize()
        .unwrap_or(Vec3::Z);
    let x = Vec3::Y.cross(z);
    Affine3A::from_mat3_translation(Mat3::from_cols(x, Vec3::Y, z), at)
}

/// One body against the barrier: each limb sphere is pushed out of the
/// shell and strikes a ripple. `limbs` is the caller's scratch buffer;
/// `parts` is filled with the pushed parts, in order. The last pushed
/// part's shell point in the world (cm), when any was pushed.
pub(crate) fn keep_out(
    barrier: &mut Barrier,
    facing: Vec3,
    body: &mut Ragdoll,
    limbs: &mut Vec<LimbSphere>,
    parts: &mut Vec<usize>,
) -> Option<Vec3> {
    parts.clear();
    let frame = cap_frame(barrier.dome.at, facing);
    let to_local = frame.inverse();
    barrier.dome.ripples.begin_body();
    // A sphere the shell pushes is within the dome's radius plus its own of
    // the dome's centre.
    let reach = barrier.dome.radius.abs();
    let (low, high) = gather_limbs(body, limbs);
    let at = barrier.dome.at;
    if bounds_miss(
        low,
        high,
        (at - Vec3::splat(reach), at + Vec3::splat(reach)),
    ) {
        return None;
    }
    let mut pushed = None;
    for limb in limbs.iter() {
        if limb.inverse_mass <= 0.0 {
            continue;
        }
        let mass = 1.0 / limb.inverse_mass;
        let local = to_local.transform_point3(limb.centre);
        let Some(push) = barrier.part_push(local, limb.radius, mass) else {
            continue;
        };
        body.push_sphere(
            limb.part,
            limb.sphere,
            frame.transform_vector3(push.push) / CM,
        );
        barrier.dome.ripples.strike(push.ripple_at, push.strength);
        pushed = Some(frame.transform_point3(push.ripple_at));
        if parts.last() != Some(&limb.part) {
            parts.push(limb.part);
        }
    }
    pushed
}

/// What one pushed part's contact does to its character: the 40 cm shove,
/// and for a knock the instability counter's rise, the energy drain taken
/// at the contact's distance with `step` ([`balance_after`]) and the AI's
/// route dropped (the behaviour's route byte cleared, so it plans again);
/// a held weapon's struck part breaks its holder's attack at `now`. The
/// original's order: shove, sound, counter, energy, route.
pub(crate) fn apply_contact(
    contact: &crate::effects::force_dome::Contact,
    body: &mut Ragdoll,
    route: Option<&mut exa_ai::drive::Driven>,
    holder_swing: Option<&mut exa_combat::swing::SwingState>,
    step: f64,
    now: u32,
) {
    if let Some(shove) = contact.shove {
        body.begin_knockback(shove * (CONTACT_PUSH / CM));
    }
    if contact.knocks {
        if let Some(drive) = body.drive.as_mut() {
            let counter = drive.balance().counter() + 1;
            drive.balance_mut().set_counter(counter);
            if let Some(distance) = contact.balance_distance {
                drive.energy = balance_after(drive.energy, distance, step);
            }
            info!(
                "dome contact: knock, instability counter {counter}, energy {:.4}, route {}",
                drive.energy,
                if route.is_some() { "dropped" } else { "none" }
            );
        }
        if let Some(route) = route {
            route.0.driving = false;
        }
    }
    if contact.struck
        && let Some(swing) = holder_swing
    {
        swing.force_interrupt(u64::from(now));
        info!("dome contact: a held weapon's part is struck, its holder's attack breaks");
    }
}

/// Whether `part` of a held body is its weapon's strike chain (the part's
/// flag 0x10000).
pub(crate) fn strikes(body: &Ragdoll, part: usize) -> bool {
    body.parts
        .get(part)
        .is_some_and(|p| p.state.has(exa_physics::part::PartState::STRIKE_CHAIN))
}

/// The caster's facing: its locomotor's yaw, else its transform's.
fn facing_of(locomotor: Option<&Locomotor>, at: Option<&GlobalTransform>) -> Vec3 {
    match (locomotor, at) {
        (Some(l), _) => Vec3::new(l.yaw.sin(), 0.0, l.yaw.cos()),
        (None, Some(at)) => at.rotation() * Vec3::Z,
        (None, None) => Vec3::Z,
    }
}

/// Ticks every barrier in the scene.
#[allow(clippy::type_complexity)]
#[allow(clippy::too_many_arguments)]
pub(crate) fn tick_barriers(
    mut commands: Commands,
    mut barriers: Query<(Entity, &mut LiveBarrier)>,
    mut casters: Query<(
        &mut PowerCaster,
        Option<&Locomotor>,
        Option<&GlobalTransform>,
    )>,
    mut bodies: Query<(Entity, &mut Ragdoll, Option<&GlobalTransform>)>,
    (mut routes, mut swings): (
        Query<&mut exa_ai::drive::Driven>,
        Query<&mut exa_combat::swing::SwingState>,
    ),
    ticks: Option<Res<BodyTicks>>,
    mut roll: ResMut<EffectRoll>,
    (mut limbs, mut parts): (Local<Vec<LimbSphere>>, Local<Vec<usize>>),
) {
    let now = ticks.as_deref().map_or(0, |t| t.millis() as u32);
    for (entity, mut live) in &mut barriers {
        let live = &mut *live;
        let mut scratch = CastBody::default();
        let mut place = live.barrier.dome.at;
        let mut facing = Vec3::from_array(live.barrier.pose.second);
        let mut caster = casters.get_mut(live.caster).ok();
        let cast_body = match caster.as_mut() {
            Some((caster, locomotor, at)) => {
                facing = facing_of(*locomotor, *at);
                if let Some(at) = at {
                    place = at.translation() * CM;
                }
                &mut caster.body
            }
            None => {
                call_hook(&mut live.barrier);
                live.hooked = false;
                &mut scratch
            }
        };
        hook_called(live, cast_body);
        pay(live, cast_body);
        if let Some((caster, _, _)) = caster.as_mut() {
            caster.sync_body();
        }
        live.barrier.snapshot(0);
        live.barrier.tick(place, place.y, facing, now);
        if live.barrier.dome.phase == Phase::Gone {
            let gone = live.barrier.dome.destroy(true);
            info!(
                "force barrier: destroyed (hum voice freed {}, GI light removed {})",
                gone.hum, gone.light
            );
            commands.entity(entity).despawn();
            continue;
        }
        let centre = live.barrier.dome.at;
        for (other, mut body, at) in &mut bodies {
            if other == live.caster {
                continue;
            }
            let Some(shell_point) =
                keep_out(&mut live.barrier, facing, &mut body, &mut limbs, &mut parts)
            else {
                continue;
            };
            body.touch(WAKE_TICKS);
            // A held body's parts are its holder's: no character holds a
            // loose one, and the contact asks only about characters' parts.
            let limp = body.drive.is_none();
            let holder = body.held_by;
            if limp && holder.is_none() {
                continue;
            }
            // The contact measures from the character's chest point (its
            // balance point), else its transform's place.
            let chest = body
                .drive
                .as_ref()
                .and_then(|d| d.balance().chest())
                .map(|c| c * CM);
            let Some(at) = chest.or_else(|| at.map(|a| a.translation() * CM)) else {
                continue;
            };
            // The contact runs once for each pushed part.
            for &part in parts.iter() {
                let hit = DomeHit {
                    offset: at - centre,
                    scale: 1.0,
                    caster: false,
                    limp,
                    struck_flag: limp && strikes(&body, part),
                };
                let Some(contact) = live.barrier.contact(&hit, now, &mut roll.0) else {
                    continue;
                };
                let mut swing = holder.and_then(|h| swings.get_mut(h).ok());
                apply_contact(
                    &contact,
                    &mut body,
                    routes.get_mut(other).ok().as_deref_mut(),
                    swing.as_deref_mut(),
                    BARRIER_BALANCE_STEP,
                    now,
                );
                if let Some([volume, pitch]) = contact.sound {
                    live.sounds.push(
                        DomeSet::Hit,
                        Sound {
                            at: shell_point,
                            volume,
                            pitch,
                        },
                    );
                }
            }
        }
    }
}

/// A barrier for `caster` at its place `at` (cm), paying `cost` a tick.
pub fn new_barrier(
    body: &mut CastBody,
    caster: Entity,
    cost: f32,
    at: Vec3,
    now: u32,
    draws: &mut EffectDraws,
) -> LiveBarrier {
    let hooked = start_barrier(body);
    let generation = body.cast_generation;
    let (mut barrier, sound) = Barrier::cast(BARRIER_RADIUS, at, at.y, now, &mut draws.0);
    // The hum's start draws after the cast sound's pitch.
    let hum_roll = draws.0.unit();
    barrier.dome.attach_node();
    // The creator's GI light and the cast's hum voice, which the destructor
    // frees: the light is registered while the entity lives
    // (`super::effect_light`) and the voice is held by `super::dome_sound`.
    barrier.dome.light = true;
    barrier.dome.hum = true;
    barrier.rate = cost;
    LiveBarrier {
        barrier,
        caster,
        hooked,
        generation,
        sounds: DomeSounds::cast(sound, hum_roll),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::arena::Mt19937;

    fn live(body: &mut CastBody) -> LiveBarrier {
        let mut draws = EffectDraws(Mt19937::new(1));
        new_barrier(body, Entity::PLACEHOLDER, 0.07, Vec3::ZERO, 0, &mut draws)
    }

    #[test]
    fn the_start_plays_the_motion_and_takes_the_hook() {
        let mut body = CastBody::default();
        let barrier = live(&mut body);
        assert!(barrier.hooked);
        assert_eq!(body.casting, BARRIER);
        assert_eq!(body.cast_motion, Some(BARRIER_MOTION));
        assert_eq!(body.cast_hook, Some(BARRIER_HOOK));
        assert_eq!(barrier.barrier.dome.phase, Phase::Growing);
        assert_eq!(barrier.barrier.dome.radius, BARRIER_RADIUS);
        // The dome rises to the ground plus 100.
        assert_eq!(barrier.barrier.dome.at, Vec3::new(0.0, 100.0, 0.0));
        // The cast queues `thmfrc_ss` at 0.2 there.
        let cue = barrier.sounds.cues[0];
        assert_eq!(cue.set, DomeSet::Cast);
        assert_eq!(
            (cue.sound.at, cue.sound.volume),
            (barrier.barrier.dome.at, 0.2)
        );
    }

    #[test]
    fn a_later_cast_or_the_release_ends_it() {
        let mut body = CastBody::default();
        let mut barrier = live(&mut body);
        assert!(!hook_called(&mut barrier, &mut body));
        assert!(casting::start_cast(&mut body, 0x22, 0x201));
        assert!(hook_called(&mut barrier, &mut body));
        assert_eq!(barrier.barrier.dome.phase, Phase::Ending);

        let mut body = CastBody::default();
        let mut barrier = live(&mut body);
        body.release = true;
        assert!(casting::release_cast(&mut body));
        assert!(hook_called(&mut barrier, &mut body));
        assert_eq!(
            (body.casting, body.cast_hook, body.release),
            (0, None, false)
        );
        assert_eq!(barrier.barrier.dome.phase, Phase::Ending);
    }

    #[test]
    fn it_pays_its_rate_and_ends_when_focus_runs_short() {
        let mut body = CastBody {
            focus: 0.15,
            ..CastBody::default()
        };
        let mut barrier = live(&mut body);
        pay(&mut barrier, &mut body);
        assert_eq!(body.spent, 0.07);
        barrier.barrier.dome.phase = Phase::Standing;
        pay(&mut barrier, &mut body);
        assert!((body.spent - 0.105).abs() < 1e-6);
        assert_eq!(barrier.barrier.dome.phase, Phase::Standing);
        // 0.15 − 0.14 is short of the next 0.035: the hook ends the barrier.
        pay(&mut barrier, &mut body);
        assert_eq!(barrier.barrier.dome.phase, Phase::Ending);
        assert_eq!((body.casting, body.cast_hook), (0, None));
        assert_eq!(body.condition, 0);
        assert!(casting::take_stance_return(&mut body), "the stance return");
        let spent = body.spent;
        pay(&mut barrier, &mut body);
        assert_eq!(body.spent, spent);
    }
}
