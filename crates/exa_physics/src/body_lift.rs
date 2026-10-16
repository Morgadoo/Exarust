//! The carry of a living body, as the original's: what holds it up and
//! where it stands, in one vector that every limb particle takes in place
//! of gravity.
//!
//! The character tick resets the carry to gravity, then a pelvis-height
//! servo (the lift) takes g·f·S off it, f growing as the pelvis sinks
//! under its target height: f = 0.075(h + 15) standing, at most 1.5, or
//! (0.4·run + 0.2)(h + 5) moving, at most 2, h the target less the pelvis
//! height (cm), damped by the pelvis's vertical speed and scaled by the
//! tone over a half. Each particle of a part whose tone is over a half then
//! moves by g(1 − a) + carry·a: a = tone × the part's support share
//! (pelvis and thighs 1, lower legs 0.5, feet 0.25) or tone × energy (the
//! upper body). Standing at f = 1, 1.7 cm over its target, a body floats on
//! its legs; without support the carry is gravity.
//!
//! The target is the lowest foot's ground plus the clip root's height over
//! the ankle and the foot's ground offset (its sole height standing),
//! blended with the other foot's, capped at the ground plus sole plus leg
//! length, plus 5 cm. The rules are the original's, in its centimetres and
//! ticks (evidence: `docs/formats/RSQ.md`, Physical body).
//! (Included from body.rs: `#[path = "body_lift.rs"] mod lift;`)

use bevy::math::Affine3A;
use bevy::prelude::*;

use super::balance::cm;
use super::{BODY_HZ, Bearing, CM, GRAVITY, Ragdoll};

/// Standing, f = 0.075 per cm of h + 15, at most 1.5.
const LIFT_PER_CM: f32 = 0.075;
const STANDING_LEAD: f32 = 15.0;
const MAX_STANDING_LIFT: f32 = 1.5;
/// Moving, f = (0.4·run + 0.2)(h + 5), at most 2.
const RUN_LIFT: f32 = 0.4;
const WALK_LIFT: f32 = 0.2;
const MOVING_LEAD: f32 = 5.0;
const MAX_MOVING_LIFT: f32 = 2.0;
/// The damping takes (1 − |f|/(|f| + 2))·E²·v_y off f.
const DAMPING_SPREAD: f32 = 2.0;
/// Crouched deeper than this a standing body's f may go down to −10;
/// otherwise a negative f leaves the carry at gravity.
const DEEP_CROUCH: f64 = 0.9;
const MIN_CROUCHED_LIFT: f32 = -10.0;
/// Over this tone a part takes the carry, and the lift scales by twice the
/// tone above it.
const TONE_FLOOR: f32 = 0.5;
/// A crouching body's target is this high over its ground at full crouch
/// (cm).
const CROUCHED_HEIGHT: f32 = 60.0;
/// The other foot's target is blended in with this weight standing, by
/// none once the feet's spread (cm) reaches the second, by all of it under
/// the first; between, by 0.05 a cm.
const STANDING_BLEND: f32 = 0.5;
const BLEND_FULL: f32 = 40.0;
const BLEND_NONE: f32 = 60.0;
const BLEND_SLOPE: f64 = 0.05;
/// A standing target is raised this much (cm) once capped.
const TARGET_RAISE: f32 = 5.0;
/// A foot is the lowest only with ground under this height over the
/// pelvis (cm).
const LOWEST_ABOVE: f32 = 100.0;
/// The target when no foot has ground (cm): no lift.
pub(super) const NO_TARGET: f32 = -100_000.0;

/// Gravity per tick (m a tick²): what each particle falls by.
pub(super) fn gravity_per_tick() -> Vec3 {
    let dt = 1.0 / BODY_HZ;
    Vec3::Y * GRAVITY * dt * dt
}

/// The share of the carry a part of `bone` takes, of its tone: the pelvis
/// and thighs 1, the lower legs 0.5, the feet 0.25; none for the upper
/// body, which takes its energy instead.
pub(super) fn support_share(bone: usize) -> Option<f32> {
    match bone {
        1 | 2 | 5 => Some(1.0),
        3 | 6 => Some(0.5),
        4 | 7 => Some(0.25),
        _ => None,
    }
}

/// How much of the carry a part of `bone` takes at `tone` and `energy`:
/// a = tone × its support share, or tone × energy; none at a tone of a
/// half or less (it falls by gravity alone).
pub(super) fn carry_share(bone: usize, tone: f32, energy: f32) -> Option<f32> {
    (tone > TONE_FLOOR).then(|| tone * support_share(bone).unwrap_or(energy))
}

/// The lift f before the tone, in the original's cm and ticks: `h` the
/// target less the pelvis height, `rise` the pelvis's vertical displacement
/// over the last tick, `energy` E. Standing, f = 0.075(h + 15), none when
/// negative (but down to −10 crouched deeper than 0.9), at most 1.5; moving,
/// f = (0.4·run + 0.2)(h + 5), none when negative, at most 2. Then f −=
/// (1 − |f|/(|f| + 2))·E²·rise.
pub(super) fn lift_factor(h: f32, rise: f32, energy: f32, bearing: &Bearing) -> Option<f32> {
    let f = if bearing.moving {
        let f = (RUN_LIFT * bearing.run + WALK_LIFT) * (h + MOVING_LEAD);
        if f < 0.0 || f.is_nan() {
            return None;
        }
        f.min(MAX_MOVING_LIFT)
    } else {
        let f = LIFT_PER_CM * (h + STANDING_LEAD);
        let f = if f64::from(bearing.crouch) <= DEEP_CROUCH {
            if f < 0.0 || f.is_nan() {
                return None;
            }
            f
        } else {
            f.max(MIN_CROUCHED_LIFT)
        };
        f.min(MAX_STANDING_LIFT)
    };
    let spread = f.abs() / (f.abs() + DAMPING_SPREAD);
    Some(f - (1.0 - spread) * energy * energy * rise)
}

/// One leg as the target height reads it, in cm: its ground (none out of
/// reach), its contact's height, the clip root's height over its animated
/// ankle plus its ground offset, and its length.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct LegHeight {
    pub ground: Option<f32>,
    pub contact: f32,
    pub stance: f32,
    pub length: f32,
}

/// The target pelvis height (cm) from the first two legs, crouching by
/// `crouch`, with the feet's sole height `sole` and the pelvis at `pelvis`:
/// each leg's is crouch·60 + (1 − crouch)·stance + ground. It is the lowest
/// foot L's, blended toward the other O's by half (none when O has no
/// ground), fully while O's contact height less its ground plus L's
/// contact height is under 40 cm, by 0.05 a cm of it less over 40, not at
/// all from 60 (the original's test, as read). Standing, it is capped at
/// L's ground plus sole plus leg length and raised 5 cm; moving, neither.
/// [`NO_TARGET`] when no foot has ground.
pub(super) fn target_height(
    legs: &[LegHeight],
    sole: f32,
    crouch: f32,
    pelvis: f32,
    moving: bool,
) -> f32 {
    let mut lowest = pelvis + LOWEST_ABOVE;
    let mut low = None;
    for (i, leg) in legs.iter().take(2).enumerate() {
        if let Some(g) = leg.ground
            && g < lowest
        {
            lowest = g;
            low = Some(i);
        }
    }
    let Some(i) = low else {
        return NO_TARGET;
    };
    let value = |leg: &LegHeight, ground: f32| {
        CROUCHED_HEIGHT * crouch + (1.0 - crouch) * leg.stance + ground
    };
    let l = &legs[i];
    let mut target = value(l, lowest);
    if let Some(o) = legs.get(1 - i) {
        let (weight, ground) = match o.ground {
            Some(g) => (STANDING_BLEND, g),
            None => (0.0, lowest),
        };
        let spread = o.contact - ground + l.contact;
        if spread < BLEND_NONE {
            let weight = if spread > BLEND_FULL {
                (f64::from((BLEND_NONE - spread) * weight) * BLEND_SLOPE) as f32
            } else {
                weight
            };
            target = value(o, ground) * weight + (1.0 - weight) * target;
        }
    }
    if !moving {
        target = target.min(lowest + sole + l.length) + TARGET_RAISE;
    }
    target
}

impl Ragdoll {
    /// The character tick's carry, once the body is aimed: gravity, less
    /// the lift. Each leg's ground offset is set (its sole height standing;
    /// moving, its animated ankle's height over the walk's lowest, plus the
    /// sole), then the target pelvis height from the ground under the feet
    /// and the clip root's height over each ankle, then the lift servo, with
    /// support only.
    pub(super) fn lift(&mut self) {
        let measured = self.measures_balance();
        let root = self.parts.iter().position(|p| p.parent.is_none());
        let Ragdoll {
            parts,
            legs,
            drive,
            sole,
            ..
        } = self;
        let Some(drive) = drive.as_mut() else {
            return;
        };
        let gravity = gravity_per_tick();
        drive.carry = gravity;
        drive.lift = 0.0;
        drive.lift_target = None;
        let (Some(root), true) = (root, measured) else {
            return;
        };
        let bearing = drive.bearing;
        let posed = drive.targets.len() == parts.len();
        let pelvis_target = posed.then(|| Vec3::from(drive.targets[root].translation));
        let floor = drive.root.map(|r: Affine3A| r.translation.y);
        let heights: Vec<LegHeight> = legs
            .iter_mut()
            .map(|leg| {
                let ankle = posed.then(|| drive.targets[leg.shin].transform_point3(leg.point));
                // The original's ground offset: the sole height standing;
                // moving, the ankle's height over the walk's lowest.
                leg.offset = match (bearing.moving, bearing.gait_floor, ankle, floor) {
                    (true, Some(lowest), Some(a), Some(f)) => a.y - f - lowest + *sole,
                    _ => *sole,
                };
                let over_ankle = pelvis_target.zip(ankle).map_or(0.0, |(p, a)| p.y - a.y);
                LegHeight {
                    ground: leg.ground.map(cm),
                    contact: cm(parts[leg.shin].point(&leg.contact).y),
                    stance: cm(over_ankle + leg.offset),
                    length: cm(leg.length),
                }
            })
            .collect();
        let pelvis = &parts[root];
        let height = cm(pelvis.transform().translation.y);
        let target = target_height(&heights, cm(*sole), bearing.crouch, height, bearing.moving);
        drive.lift_target = Some(target / CM);
        if !drive.balance.has_support() {
            return;
        }
        let w = pelvis.own_origin;
        let rise: f32 = (0..4)
            .map(|k| (pelvis.positions[k].y - pelvis.previous[k].y) * w[k])
            .sum();
        let Some(f) = lift_factor(target - height, cm(rise), drive.energy, &bearing) else {
            return;
        };
        if drive.tone > TONE_FLOOR {
            let f = 2.0 * (drive.tone - TONE_FLOOR) * f;
            drive.lift = f;
            drive.carry -= gravity * (f * drive.support);
        }
    }
}

#[cfg(test)]
#[path = "body_lift_test.rs"]
mod tests;
