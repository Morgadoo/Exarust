//! The collapse before death: a lethal wound does not kill at once. As in
//! the original, the fighter's condition becomes collapsing (0x800) and it
//! starts Fall Backward with a blend of round((1.2 − E) × 300) frames, E
//! its energy after the blow, eased smoothly; it no longer stands on its
//! controller, and its lower legs leave the living lower legs' collision
//! layer. Each tick its dying tick runs in place of the posture
//! decision ([`dying_tick`]): its heading wanders, it throws its arms wide
//! with Rebalance (20 frames, linear) once its balance is under 0.8 (not a
//! body of size 1.2 or more), its tone falls 0.003 and its support pins'
//! gain 0.0003; its body meanwhile follows its pelvis with its frame
//! exactly, its lift fades by its tone squared and its arms' stiffness is
//! 0.2 (`exa_physics::body::Bearing::collapsing`). At a tone of 0.25,
//! 250 ticks (3.0 s) after the blow, it dies: dead (0xA00), a plain ragdoll
//! that keeps the motion it has. Rules: `docs/formats/RSQ.md` (Condition,
//! The collapse).

use bevy::prelude::*;
use exa_anim::{Animator, Ease};
use exa_physics::body::{GoLimp, Ragdoll, pairs};

use crate::condition::{Condition, PostureTick, StateFlags, track_condition};
use crate::{Fighter, Roll};

/// The clip a lethal wound starts.
pub const FALL_BACKWARD: &str = "Fall Backward";
/// The clip a collapsing body throws its arms wide with.
pub const REBALANCE: &str = "Rebalance";
/// Fall Backward starts with round((1.2 − E) × 300) frames of blend,
/// computed in double precision.
const BLEND_FROM: f64 = 1.2;
const BLEND_SCALE: f64 = 300.0;
/// Tone lost each dying tick (subtracted in double precision).
const TONE_LOSS: f64 = 0.003;
/// At or under this tone (half the original's 0.5) the body dies; its
/// muscles off, its tone becomes 0.01.
pub const DEATH_TONE: f32 = 0.25;
const DEAD_TONE: f32 = 0.01;
/// The support pins' gain lost each dying tick (in double precision); once
/// under 0 it is set to 0.01.
const PIN_LOSS: f64 = 0.0003;
const PIN_RESET: f32 = 0.01;
/// Bodies of this size or more (in double precision) never Rebalance.
const BIG_FROM: f64 = 1.2;
/// Under this balance (double) a collapsing body Rebalances, with this
/// blend.
const REBALANCE_BELOW: f64 = 0.8;
const REBALANCE_BLEND: u32 = 20;
/// Each tick the heading's x and z each move by (random − 0.5) × 0.5.
const JITTER: f32 = 0.5;

/// The blend (frames) Fall Backward starts with at `energy`, the energy
/// after the killing blow: round((1.2 − E) × 300), ties to even (60 at
/// full energy, 345 at the daze's floor of 0.05).
pub fn collapse_blend(energy: f32) -> u32 {
    ((BLEND_FROM - f64::from(energy)) * BLEND_SCALE)
        .round_ties_even()
        .max(0.0) as u32
}

/// The heading (x, z, a unit vector across the ground) after one dying
/// tick, from two random draws `a` and `b` in 0..1: each component moves
/// by (draw − 0.5) × 0.5, at most a quarter, then it is a unit vector
/// again.
pub fn jitter_heading(heading: Vec2, a: f32, b: f32) -> Vec2 {
    let moved = Vec2::new(
        heading.x + (a - 0.5) * JITTER,
        heading.y + (b - 0.5) * JITTER,
    );
    moved.try_normalize().unwrap_or(heading)
}

/// One dying tick of the tone: 0.003 less; at 0.25 or less the muscles go
/// and the tone is 0.01. Returns the tone and whether the body dies.
pub fn lose_tone(tone: f32) -> (f32, bool) {
    let tone = (f64::from(tone) - TONE_LOSS) as f32;
    if tone <= DEATH_TONE {
        (DEAD_TONE, true)
    } else {
        (tone, false)
    }
}

/// One dying tick of the support pins' gain: 0.0003 less, and 0.01 once
/// it went under 0.
pub fn fade_pins(gain: f32) -> f32 {
    let gain = (f64::from(gain) - PIN_LOSS) as f32;
    if gain < 0.0 { PIN_RESET } else { gain }
}

/// Whether a collapsing body of `size` with `balance` throws its arms wide
/// (Rebalance): under size 1.2 and balance 0.8, both in double precision.
pub fn rebalances(size: f32, balance: f32) -> bool {
    f64::from(size) < BIG_FROM && f64::from(balance) < REBALANCE_BELOW
}

/// A dying tick's Rebalance: when the body [`rebalances`], Rebalance (a
/// looping held pose) starts with 20 frames of blend, linear, unless it
/// already plays (the clip start then does nothing). Returns whether it
/// started.
pub fn throw_arms_wide(anim: &mut Animator, size: f32, balance: f32) -> bool {
    if !rebalances(size, balance) || anim.action == REBALANCE {
        return false;
    }
    anim.start(REBALANCE, true, None, REBALANCE_BLEND, Ease::Linear);
    true
}

/// On a collapsing fighter (its body model root): the heading its dying
/// ticks drift (x, z; taken from where the body faces at the first) and
/// how many it has had.
#[derive(Component, Debug, Clone, Copy, Default, PartialEq)]
pub struct Collapse {
    pub heading: Option<Vec2>,
    pub ticks: u32,
}

/// Starts a lethal wound's collapse, unless the fighter collapses or is
/// dead already: no longer waking, condition collapsing, Fall Backward
/// (a looping held pose) starting from where the body is with
/// [`collapse_blend`] frames at `energy` (the clip start does nothing when
/// it already plays), eased smoothly either way; its body no longer stands
/// on its controller, and its lower legs leave the living lower legs'
/// layer, so they meet other characters' again. Returns whether it
/// started.
pub fn start_collapse(
    condition: &mut Condition,
    flags: &mut StateFlags,
    anim: &mut Animator,
    body: Option<&mut Ragdoll>,
    energy: f32,
) -> bool {
    if condition.dying() {
        return false;
    }
    flags.0 &= !StateFlags::WAKING;
    *condition = Condition::COLLAPSING;
    if anim.action != FALL_BACKWARD {
        let blend = collapse_blend(energy);
        anim.start(FALL_BACKWARD, true, None, blend, Ease::Smooth);
    } else {
        anim.looping = true;
        anim.then = None;
    }
    anim.ease = Ease::Smooth;
    if let Some(body) = body {
        body.fall();
        pairs::set_leg_layers(&mut body.parts, 0);
    }
    true
}

pub struct CollapsePlugin;

impl Plugin for CollapsePlugin {
    fn build(&self, app: &mut App) {
        // The collapsing branch of the posture stage, in place of the
        // posture decision (which leaves the collapsing alone).
        app.add_systems(
            FixedUpdate,
            dying_tick.in_set(PostureTick).after(track_condition),
        );
    }
}

/// The dying tick of every collapsing body, once a fixed tick: its heading
/// drifts by two draws of the original's generator; under balance 0.8 a
/// body under size 1.2 starts Rebalance (once: the start does nothing
/// while it plays); its tone falls 0.003 and its pins' gain 0.0003. At a
/// tone of 0.25 it dies there and then: dead, its muscles gone, keeping
/// its motion; the frame strips its clip and controller.
#[allow(clippy::type_complexity)]
pub(crate) fn dying_tick(
    mut commands: Commands,
    mut roll: Local<Roll>,
    mut bodies: Query<(
        Entity,
        &mut Condition,
        &mut Collapse,
        &mut Ragdoll,
        &mut Animator,
        &Fighter,
        &GlobalTransform,
    )>,
) {
    for (entity, mut condition, mut collapse, mut body, mut anim, fighter, gt) in &mut bodies {
        if !condition.is_collapsing() {
            continue;
        }
        let Some(drive) = body.drive.as_mut() else {
            continue;
        };
        let heading = collapse.heading.unwrap_or_else(|| {
            (gt.rotation() * Vec3::Z)
                .xz()
                .try_normalize()
                .unwrap_or(Vec2::Y)
        });
        let (a, b) = (roll.next(), roll.next());
        collapse.heading = Some(jitter_heading(heading, a, b));
        if throw_arms_wide(&mut anim, fighter.stats.size, drive.balance().value) {
            info!(
                "{entity:?} rebalances {} ticks into its collapse",
                collapse.ticks
            );
        }
        let (tone, dies) = lose_tone(drive.tone);
        drive.tone = tone;
        drive.pin_gain = fade_pins(drive.pin_gain);
        collapse.ticks += 1;
        if dies {
            condition.die();
            body.go_limp();
            commands
                .entity(entity)
                .insert(GoLimp {
                    velocity: None,
                    delay: 0,
                })
                .remove::<Collapse>();
            info!(
                "{entity:?} dies {} ticks after its collapse",
                collapse.ticks
            );
        }
    }
}

#[cfg(test)]
#[path = "collapse_test.rs"]
mod tests;
