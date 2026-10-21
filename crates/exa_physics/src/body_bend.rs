//! The standing leg bend: each leg folds onto its bend target by the law
//! of cosines, and the muscles follow that fold on the next tick.
//!
//! The correction from the clip's angle advances at most 0.02 rad a tick
//! (0.2 while a recovery step is on), plus a stored rate, and never past
//! the target. The rate then becomes half of that sum, within ±0.1 rad.
//! From a zero correction and a zero rate, one tick toward a larger
//! correction stores 0.02 rad. The pose this tick's muscles already follow
//! is left as it was; the next aim multiplies the stored correction in.
//!
//! An anchored foot's target is the midpoint of the foot and its anchor,
//! at the anchor's height. An unanchored foot's is its anchor, lowered by
//! twice the lift term stored before clamping: 2.25 cm when a standing
//! body is at its target height. While the leg is not stepping, the reach
//! still measures its height from the anchor and its horizontal reach from
//! the clip, which is how the original builds the standing triangle.
//! (Included from body.rs: `#[path = "body_bend.rs"] mod bend;`)

use bevy::math::Affine3A;
use bevy::prelude::*;

use super::balance::cm;
use super::feet::FootState;
use super::{CM, Leg, Part, Ragdoll};

/// Above this cosine the knee is held at 30°.
const FOLD_COS: f32 = 0.8660254;
/// 30°, and a straight knee.
const FOLD: f32 = 0.5235988;
const FLAT: f32 = 3.1415927;
/// The most the correction advances in one tick (rad): standing, and
/// while a recovery step is on.
const EASE_STANDING: f32 = 0.02;
const EASE_STEPPING: f32 = 0.2;
/// The stored rate is kept by half, and never past this (rad).
const RATE_KEEP: f32 = 0.5;
const RATE_CAP: f32 = 0.1;
/// The reach is at least this share of the leg, and never past the leg.
const REACH_MIN: f32 = 0.25;
/// While stepping, the thigh turns this share of the way to the target.
const THIGH_SHARE: f32 = 0.75;
/// Not stepping, the extra turn loses this share of itself a tick.
const TURN_DECAY: f32 = 0.1;
/// A direction pair shorter than this (the product of the squared
/// lengths) has no turn, and a cosine below it is a right angle.
const TURN_TINY: f32 = 1.1920929e-7;
const RIGHT_ANGLE: f32 = 1.5707964;
/// The step lift. s is 0.4 of the horizontal distance, and no more than
/// 30 cm. An unanchored lift keeps 0.8 of itself; an anchored one that
/// is not stepping keeps 0.9.
const LIFT_HORIZONTAL: f32 = 0.4;
const LIFT_CAP: f32 = 30.0;
const LIFT_FREE: f32 = 0.8;
const LIFT_HELD: f32 = 0.9;
const STANCE_BASE: f32 = 1.5;

/// The knee angle (rad) for bones of length `thigh` and `shin` (m) and a
/// hip-to-target reach `reach` (m). The cosine above [`FOLD_COS`] is 30°,
/// and a cosine below −1 is 180°.
pub(super) fn knee_angle(thigh: f32, shin: f32, reach: f32) -> f32 {
    let denom = 2.0 * thigh * shin;
    if denom <= 0.0 {
        return FLAT;
    }
    let cos = (thigh * thigh + shin * shin - reach * reach) / denom;
    if cos <= -1.0 {
        FLAT
    } else if cos > FOLD_COS {
        FOLD
    } else {
        cos.acos()
    }
}

/// The hip angle (rad) of the same triangle: 0 when the thigh already
/// points along the reach, 180° when the cosine is below −1.
pub(super) fn hip_angle(thigh: f32, shin: f32, reach: f32) -> f32 {
    let denom = 2.0 * reach * thigh;
    if denom <= 0.0 {
        return 0.0;
    }
    let cos = (thigh * thigh + reach * reach - shin * shin) / denom;
    if cos <= -1.0 {
        FLAT
    } else if cos >= 1.0 {
        0.0
    } else {
        cos.acos()
    }
}

/// One tick of the angle smoother. `step` is 0.02 or 0.2 (rad). The
/// advance is the stored `rate` plus `step` (or plus the whole remaining
/// error, when that is smaller), never past `target` and never back.
/// The rate becomes half of (rate + advance), within ±0.1.
pub(super) fn ease(angle: f32, rate: f32, target: f32, step: f32) -> (f32, f32) {
    let delta = target - angle;
    let advance = if delta >= 0.0 {
        let candidate = if delta <= step {
            rate + delta
        } else {
            rate + step
        };
        if candidate >= 0.0 && delta < candidate {
            delta
        } else if candidate >= 0.0 {
            candidate
        } else {
            0.0
        }
    } else {
        let candidate = if -step <= delta {
            rate + delta
        } else {
            rate - step
        };
        if candidate <= 0.0 && candidate < delta {
            delta
        } else if candidate <= 0.0 {
            candidate
        } else {
            0.0
        }
    };
    let angle = angle + advance;
    let rate = ((rate + advance) * RATE_KEEP).clamp(-RATE_CAP, RATE_CAP);
    (angle, rate)
}

/// The bend target (m). Anchored: the midpoint of `foot` and `anchor`, at
/// the anchor's height. Unanchored: the anchor, lowered by `drop` (m),
/// twice the lift term stored before clamping.
pub(super) fn bend_target(foot: Vec3, anchor: Vec3, anchored: bool, drop: f32) -> Vec3 {
    if anchored {
        let mid = foot.lerp(anchor, 0.5);
        Vec3::new(mid.x, anchor.y, mid.z)
    } else {
        anchor - Vec3::Y * drop
    }
}

/// The step lift (cm), negative while an anchored stepping foot rises.
/// `horizontal` and `distance` are centimetres to the anchor, `crouch` is
/// 0 to 1 and `stance` is the race's stance factor (a human's 0.5).
pub(super) fn step_lift(horizontal: f32, distance: f32, crouch: f32, stance: f32) -> f32 {
    let s = (LIFT_HORIZONTAL * horizontal).min(LIFT_CAP);
    let f = if s > 0.0 && distance <= 3.0 * s {
        let span = 3.0 * s;
        let r = (distance * distance) / (span * span);
        ((2.0 - r) * r * 3.0 - 2.0) * s
    } else {
        0.5 * s
    };
    -(1.0 - crouch) * f * (STANCE_BASE - stance)
}

/// The lift carried to the next tick (m): a fresh step lift while the
/// foot is anchored and stepping, otherwise 0.8 of it (free) or 0.9
/// (anchored, not stepping).
pub(super) fn next_lift(lift: f32, anchored: bool, stepping: bool, fresh: f32) -> f32 {
    if anchored && stepping {
        fresh
    } else if anchored {
        lift * LIFT_HELD
    } else {
        lift * LIFT_FREE
    }
}

/// The thigh's extra turn, 75% of the angle from `rest` to `toward`.
/// A pair with no length, or a cosine below the tiny floor, is a right
/// angle's 75% only in the second case; a zero vector does not turn.
pub(super) fn extra_turn(rest: Vec3, toward: Vec3) -> Quat {
    let len2 = rest.length_squared() * toward.length_squared();
    if len2 <= TURN_TINY {
        return Quat::IDENTITY;
    }
    let cos = rest.dot(toward) / len2.sqrt();
    let angle = if cos <= TURN_TINY {
        RIGHT_ANGLE
    } else if cos >= 1.0 {
        0.0
    } else {
        cos.acos()
    };
    let axis = rest.cross(toward);
    if axis.length_squared() <= TURN_TINY {
        return Quat::IDENTITY;
    }
    Quat::from_axis_angle(axis.normalize(), angle * THIGH_SHARE)
}

/// Not stepping, the extra turn loses 10% of its angle.
pub(super) fn decay_turn(turn: Quat) -> Quat {
    turn.slerp(Quat::IDENTITY, TURN_DECAY)
}

/// The reach (m) clamped to a quarter of the leg through the whole leg,
/// and the knee it gives. At the leg or past it the knee is straight.
fn reached(thigh: f32, shin: f32, mut reach: f32) -> (f32, f32) {
    let full = thigh + shin;
    if full <= 0.0 {
        return (0.0, FLAT);
    }
    if reach >= full {
        return (full, FLAT);
    }
    let least = full * REACH_MIN;
    if reach < least {
        reach = least;
    }
    (reach, knee_angle(thigh, shin, reach))
}

impl Ragdoll {
    /// Multiplies last tick's leg bend into this tick's pose, and remembers
    /// the clip's hip-to-ankle offset for the bend computed after the
    /// solver. The muscles sample the pose after this, so they follow the
    /// correction on the tick after it was computed.
    pub(super) fn pose_leg_bend(&mut self) {
        let Ragdoll {
            parts, legs, drive, ..
        } = self;
        let Some(drive) = drive.as_mut() else {
            return;
        };
        if drive.targets.len() != parts.len() {
            return;
        }
        for leg in legs.iter_mut() {
            let Some(thigh) = parts[leg.shin].parent else {
                continue;
            };
            if thigh >= drive.targets.len() {
                continue;
            }
            let hip = Vec3::from(drive.targets[thigh].translation);
            let ankle = drive.targets[leg.shin].transform_point3(leg.point);
            leg.posed_offset = ankle - hip;
            fold_pose(&mut drive.targets, thigh, leg.shin, leg);
        }
    }

    /// One tick of the leg bend, from where the solver left the body.
    /// The pose the muscles just followed is not rewritten.
    pub(super) fn bend_legs(&mut self) {
        let Ragdoll {
            parts, legs, drive, ..
        } = self;
        let Some(drive) = drive.as_mut() else {
            return;
        };
        if drive.targets.len() != parts.len() {
            return;
        }
        let stepping = drive.steps.stepping;
        let rate = if stepping {
            EASE_STEPPING
        } else {
            EASE_STANDING
        };
        let drop = 2.0 * drive.lift_raw / CM;
        let stance = drive.stance;
        let crouch = drive.bearing.crouch;
        let frame = drive.frame.unwrap_or(Affine3A::IDENTITY);
        let tick = BendTick {
            stepping,
            rate,
            drop,
            stance,
            crouch,
            frame,
        };
        for leg in legs.iter_mut() {
            bend_leg(parts, leg, &tick);
        }
    }
}

/// What one tick of the bend reads off the body, the same for every leg.
struct BendTick {
    stepping: bool,
    rate: f32,
    drop: f32,
    stance: f32,
    crouch: f32,
    frame: Affine3A,
}

/// Folds one leg. Lengths are the two bones, not the lift's padded leg.
fn bend_leg(parts: &[Part], leg: &mut Leg, tick: &BendTick) {
    let BendTick {
        stepping,
        rate,
        drop,
        stance,
        crouch,
        frame,
    } = *tick;
    let Some(thigh) = parts[leg.shin].parent else {
        return;
    };
    let shin_len = leg.point.length();
    let thigh_len = parts[leg.shin].origin.length();
    if shin_len <= 0.0 || thigh_len <= 0.0 {
        return;
    }
    let foot = parts[leg.shin].point(&leg.contact);
    let anchored = leg.state.has(FootState::ANCHORED);
    let to_anchor = leg.anchor - foot;
    let fresh = step_lift(
        cm(to_anchor.with_y(0.0)).length(),
        cm(to_anchor).length(),
        crouch,
        stance,
    );
    leg.lift = next_lift(leg.lift, anchored, stepping, fresh / CM);
    let goal = bend_target(foot, leg.anchor, anchored, drop);
    leg.goal = goal;
    let hip = Vec3::from(parts[thigh].transform().translation);
    let horizontal = if stepping {
        let at = if anchored { goal } else { leg.anchor };
        (at - hip).with_y(0.0)
    } else {
        leg.posed_offset.with_y(0.0)
    };
    let vertical = hip.y - leg.anchor.y + drop * 0.5;
    let reach = (vertical * vertical + horizontal.length_squared()).sqrt() + leg.lift;
    if !reach.is_finite() {
        return;
    }
    let (reach, knee) = reached(thigh_len, shin_len, reach);
    leg.reach_knee = knee;
    let posed = leg.posed_offset.length();
    let rest_knee = if posed > 0.0 {
        knee_angle(thigh_len, shin_len, posed)
    } else {
        FLAT
    };
    let rest_hip = if posed > 0.0 {
        hip_angle(thigh_len, shin_len, posed)
    } else {
        0.0
    };
    let (knee_corr, knee_rate) = ease(leg.knee, leg.knee_rate, rest_knee - knee, rate);
    leg.knee = knee_corr;
    leg.knee_rate = knee_rate;
    let hip_now = hip_angle(thigh_len, shin_len, reach);
    let (hip_corr, hip_rate) = ease(leg.hip, leg.hip_rate, rest_hip - hip_now, rate);
    leg.hip = hip_corr;
    leg.hip_rate = hip_rate;
    if stepping {
        let into = frame.inverse();
        let rest = into.transform_vector3(leg.posed_offset);
        let toward = into.transform_vector3(goal - hip);
        leg.turn = extra_turn(rest, toward);
    } else {
        leg.turn = decay_turn(leg.turn);
    }
}

/// Writes one leg's stored correction into the pose: the thigh's turn and
/// hip hinge, then the shin's knee hinge, each in the part's own axes.
fn fold_pose(targets: &mut [Affine3A], thigh: usize, shin: usize, leg: &Leg) {
    let hip_q = leg.turn * Quat::from_rotation_x(leg.hip);
    let knee_q = Quat::from_rotation_x(leg.knee);
    let parent = targets[thigh];
    let local = parent.inverse() * targets[shin];
    targets[thigh] = parent * Affine3A::from_quat(hip_q);
    targets[shin] = targets[thigh] * local * Affine3A::from_quat(knee_q);
}

#[cfg(test)]
#[path = "body_bend_test.rs"]
mod tests;

/// How the thighs are posed over the clip.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FemurPosture {
    /// The clip's turns are kept.
    Level,
    /// The thighs splay: 5° about the posture axis, the first leg positive
    /// and the others alternating (a body with the splay flag).
    Splayed,
    /// A body in combat behaviour out of the walking and helpless
    /// conditions: (1 − `height`)·−1.5° (the race record's height), the
    /// first leg negative and the others alternating.
    Combat { height: f32 },
}

/// The turn the original multiplies into leg `leg`'s thigh over the clip's
/// (after the clip's turn, `clip * turn`): about the axis
/// normalized((0, 0, 1) lerped 0.4 toward (−1, 0, 0)), the angle by
/// `posture`, its sign alternating by leg.
pub fn femur_posture_turn(posture: FemurPosture, leg: usize) -> Quat {
    const DEGREE: f32 = 0.017_453_292;
    let angle = match posture {
        FemurPosture::Level => return Quat::IDENTITY,
        FemurPosture::Splayed => 5.0 * DEGREE,
        FemurPosture::Combat { height } => (1.0 - height) * -1.5 * DEGREE,
    };
    let axis = Vec3::new(0.0, 0.0, 1.0).lerp(Vec3::new(-1.0, 0.0, 0.0), 0.4);
    let sign = if leg.is_multiple_of(2) { 1.0 } else { -1.0 };
    Quat::from_axis_angle(axis.normalize(), sign * angle)
}

#[cfg(test)]
mod posture_tests {
    use super::*;

    /// 5° splayed, (1 − height)·−1.5° in combat, none level, alternating
    /// sign by leg about the axis 0.4 of the way from forward to left.
    #[test]
    fn the_femur_posture_is_the_originals() {
        let axis = Vec3::new(-0.4, 0.0, 0.6).normalize();
        let a = femur_posture_turn(FemurPosture::Splayed, 0);
        let b = femur_posture_turn(FemurPosture::Splayed, 1);
        let (got, angle) = a.to_axis_angle();
        assert!((angle - 5.0 * 0.017_453_292).abs() < 1e-6);
        assert!((got - axis).length() < 1e-6);
        assert!(b.abs_diff_eq(a.conjugate(), 1e-6));
        let c = femur_posture_turn(FemurPosture::Combat { height: 0.5 }, 0);
        let expected = Quat::from_axis_angle(axis, -0.75 * 0.017_453_292);
        assert!(c.abs_diff_eq(expected, 1e-6));
        assert_eq!(femur_posture_turn(FemurPosture::Level, 0), Quat::IDENTITY);
    }
}
