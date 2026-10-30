//! The legs in the animated pose: the standing leg IK and the locomotion
//! leg IK, the foot pitch and the thigh posture.
//!
//! The original runs one of the two IK routines from its feet routine: the
//! standing one while the body does not locomote, the locomotion one while
//! it does. Both write, for each leg, a knee correction about the lower
//! leg's lateral axis, a hip correction about the thigh's, and (standing)
//! a turn of the thigh toward its target; the sampler multiplies them into
//! the clip's pose ([`Ragdoll::fold_leg_ik`]).
//!
//! **Standing**: the target is the anchor, lowered by twice the lift
//! term ([`bend_target`]), or, for an anchored foot, the midpoint of the
//! contact and its anchor at the anchor's height. The reach is the hip's
//! height over the anchor (plus the lift term) with, while the recovery
//! step runs, the horizontal distance to the target (anchored) or the
//! anchor, and otherwise the clip's own horizontal hip-to-ankle offset,
//! plus the step lift, clamped to a quarter of the leg through the whole
//! leg; the knee and the hip are the law of cosines of that triangle, and
//! the corrections are the clip's angles less the target's, eased at
//! 0.2 rad a tick while the step runs and 0.02 otherwise (the feet routine
//! never snaps them; the placement runs the same routine with its snap set,
//! [`Ragdoll::snap_legs`]). The step also turns the thigh 75 % of the way toward
//! the target; not stepping, that turn loses a tenth of itself a tick.
//!
//! **Locomoting**: the target leg reaches from the physical hip across to
//! the foot's contact and down to its anchor (plus a per-leg lag); it acts
//! only where the clip's leg is longer than the hip's height over the
//! anchor (the ground is higher than the clip expects), or while a
//! touching foot does not run. Its knee is
//! the law of cosines of the target triangle; its hip is the angle between
//! the clip's leg and the target leg, signed by whether the clip's foot
//! is ahead of the character frame's origin (behind or level negates), less
//! the target triangle's hip angle and plus the clip triangle's. The
//! smoother runs at 0.04 × the speed plus 0.01 while moving. A leg whose
//! target has no length, or whose correction is switched off, keeps the
//! identity correction for that part while the stored angle still eases.
//!
//! Lying down (down, dormant or dead: off its feet, and not getting up)
//! does not retarget: the stored knee, hip and foot pitch ease toward 0 by
//! the standing step, 0.02 rad a tick, and the bend target, the step lift
//! and the thigh's extra turn stay as they were. A get-up still folds.
//!
//! The foot pitch ([`foot_pitch_target`]) levels a toe 20 cm times the
//! body scale ahead of the ankle; the height it works from is the foot's
//! height over its floor, which each IK routine writes ([`Ragdoll`]'s legs
//! keep it as `pitch_height`).
//! (Included from body.rs: `#[path = "body_bend.rs"] mod bend;`)

use bevy::math::Affine3A;
use bevy::prelude::*;

use crate::engine_math::acos_approx;

use super::balance::cm;
use super::feet::FootState;
use super::locomotion::LegMode;
use super::{CM, Leg, Part, Ragdoll};

/// Above this cosine (compared in double, as the original does) the knee
/// is held at 30°.
const FOLD_COS: f64 = 0.866_025_403_784_438_6;
/// 30°, and a straight knee.
const FOLD: f32 = 0.5235988;
const FLAT: f32 = 3.1415927;
/// The most the correction advances in one tick (rad): standing, and
/// while a foot is stepping.
const EASE_STANDING: f32 = 0.02;
const EASE_STEPPING: f32 = 0.2;
/// Moving, the locomotion IK's rate is 0.04 × the gait speed (cm a tick)
/// plus 0.01, in double.
const EASE_MOVING: f64 = 0.04;
const EASE_MOVING_BASE: f64 = 0.01;
/// The foot pitch eases at most this far (rad) a tick, plus its rate, and
/// ten times that while a recovery step is on.
pub(super) const PITCH_STEP: f32 = 0.01;
pub(super) const PITCH_STEP_STEPPING: f32 = 0.1;
/// The toe the pitch levels sits this far (cm) ahead of the ankle, times
/// the body scale.
const PITCH_REACH: f32 = 20.0;
/// The correction (cm) is held inside this, and under 0.1 it is none.
const PITCH_LIMIT: f32 = 30.0;
const PITCH_DEAD: f64 = 0.1;
/// Squared lengths under this (the original's epsilon) are none.
const PITCH_EPS: f32 = f32::from_bits(0x33ff_ffd7);
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

/// Degrees in a radian, single precision, as the foot rest turn's table
/// stores it.
const DEGREE: f32 = 0.017_453_292;
/// The foot rest turn's angle in combat: (1 − the record's height) × this
/// many degrees.
const COMBAT_PITCH: f32 = -1.5;
/// The foot rest turn's angle when splayed: this many degrees.
const SPLAY_PITCH: f32 = 5.0;
/// The foot rest turn's axis: the lerp of the original's two vectors at
/// 0.4, normalised, is (−1, 0, 0).
const FOOT_AXIS: Vec3 = Vec3::new(-1.0, 0.0, 0.0);

/// The rest turn the body tick gives a foot, from the original's foot
/// rest routine: an `-1.5° × (1 − the record's height)` turn about (−1,
/// 0, 0) while its behaviour is in combat and it is not down or walking,
/// else a 5° turn while its state word has the flat-floor (splay) bit,
/// else the identity. The mode is the thigh posture's, so the same
/// decision picks both. Each leg takes the turn the opposite way (the
/// first positive, the second negative). The sampler applies it to the
/// foot's sampled turn.
pub fn foot_rest_turn(posture: FemurPosture, leg: usize) -> Quat {
    let angle = match posture {
        FemurPosture::Level => return Quat::IDENTITY,
        FemurPosture::Splayed => SPLAY_PITCH * DEGREE,
        FemurPosture::Combat { height } => (1.0 - height) * COMBAT_PITCH * DEGREE,
    };
    let angle = if leg.is_multiple_of(2) { angle } else { -angle };
    Quat::from_axis_angle(FOOT_AXIS, angle)
}

/// The knee angle (rad) for bones of length `thigh` and `shin` (m) and a
/// hip-to-target reach `reach` (m). A cosine at or below −1 is 180°, and
/// one above the double 0.8660254 is 30°; otherwise the engine's arc
/// cosine (a NaN stays).
pub(super) fn knee_angle(thigh: f32, shin: f32, reach: f32) -> f32 {
    let denom = 2.0 * thigh * shin;
    if denom <= 0.0 {
        return FLAT;
    }
    let cos = (thigh * thigh + shin * shin - reach * reach) / denom;
    if cos <= -1.0 {
        FLAT
    } else if f64::from(cos) > FOLD_COS {
        FOLD
    } else {
        acos_approx(cos)
    }
}

/// The hip angle (rad) of the same triangle: 0 when the cosine is at or
/// above 1, 180° at or below −1, otherwise the engine's arc cosine.
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
        acos_approx(cos)
    }
}

/// The hip angle of the locomotion IK's target triangle: 0 when the
/// cosine compares at or above 1 (a NaN takes the arc cosine), otherwise
/// the engine's arc cosine.
fn target_hip_angle(cos: f32) -> f32 {
    if cos < 1.0 || cos.is_nan() {
        acos_approx(cos)
    } else {
        0.0
    }
}

/// The clip's hip angle (the knee of the clip's own triangle): 0 while
/// the leg's reach covers the clip's length, 180° for a clip of no length,
/// otherwise the engine's arc cosine of the clip's triangle. `reach2` is
/// the whole leg squared and `clip2` the clip's hip-to-ankle distance
/// squared, both cm².
fn clip_hip_angle(thigh: f32, shin: f32, reach2: f32, clip2: f32) -> f32 {
    // The original's unordered compares: a NaN falls through to the arc
    // cosine.
    if reach2 <= clip2 {
        0.0
    } else if clip2 < PITCH_EPS {
        FLAT
    } else {
        // The original takes the engine's arc cosine unclamped; a cosine
        // at or over 1 (a clip shorter than thigh less shin) is held at 0
        // here rather than becoming a NaN in the pose.
        let cos = ((thigh * thigh + clip2) - shin * shin) / (clip2.sqrt() * 2.0 * thigh);
        target_hip_angle(cos)
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

/// The pitch (rad) that levels a toe `20·scale` cm ahead of the ankle.
/// `up_y` and `forward_y` are the shin basis heights, `local_forward_y`
/// the foot's own, `height` how far the contact sits over its ground and
/// sole (cm). A correction of 0.1 cm or under, or a NaN, is none. The
/// angle is the engine's arc cosine of the isosceles cosine, negated into
/// the correction's sign.
pub fn foot_pitch_target(
    up_y: f32,
    forward_y: f32,
    local_forward_y: f32,
    height: f32,
    scale: f32,
) -> f32 {
    let length = PITCH_REACH * scale;
    let length2 = length * length;
    let remain = length2 - height * height;
    let mut raw = if remain > PITCH_EPS {
        let horiz = up_y * up_y + forward_y * forward_y;
        if horiz > PITCH_EPS {
            let along = (remain / horiz).sqrt();
            -up_y * height - forward_y * along
        } else {
            0.0
        }
    } else {
        -up_y * height
    };
    // Ordered compares: a NaN correction stays NaN and the dead zone drops it.
    raw = raw.clamp(-PITCH_LIMIT, PITCH_LIMIT);
    raw -= local_forward_y * length;
    if f64::from(raw.abs()) > PITCH_DEAD {
        let denom = 2.0 * length2;
        let cos = (denom - raw * raw) / denom;
        let angle = if cos <= -1.0 {
            std::f32::consts::PI
        } else if cos >= 1.0 {
            0.0
        } else {
            acos_approx(cos)
        };
        -angle.copysign(raw)
    } else {
        0.0
    }
}

/// The bend target (m). Anchored: the midpoint of the contact and the
/// anchor, at the anchor's height. Unanchored: the anchor, lowered by
/// `drop` (m), twice the lift term stored before clamping.
pub(super) fn bend_target(contact: Vec3, anchor: Vec3, anchored: bool, drop: f32) -> Vec3 {
    if anchored {
        let mid = contact.lerp(anchor, 0.5);
        Vec3::new(mid.x, anchor.y, mid.z)
    } else {
        anchor - Vec3::Y * drop
    }
}

/// The step lift (cm), negative while an anchored stepping foot rises.
/// `horizontal` and `distance` are centimetres to the anchor, `crouch` is
/// 0 to 1 and `stance` is the race's stance factor (a human's 0.5).
pub(super) fn step_lift(horizontal: f32, distance: f32, crouch: f32, stance: f32) -> f32 {
    // Under a centimetre of horizontal travel the scale is the squared
    // distance itself; beyond, 0.4 of it, at most 30 cm.
    let s = if horizontal * horizontal <= 1.0 {
        horizontal * horizontal
    } else {
        (LIFT_HORIZONTAL * horizontal).min(LIFT_CAP)
    };
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
        acos_approx(cos)
    };
    let axis = rest.cross(toward);
    if axis.length_squared() <= TURN_TINY {
        return Quat::IDENTITY;
    }
    Quat::from_axis_angle(axis.normalize(), angle * THIGH_SHARE)
}

/// Not stepping, the extra turn loses 10% of its angle.
pub(super) fn decay_turn(turn: Quat) -> Quat {
    turn.slerp(Quat::IDENTITY, TURN_DECAY).normalize()
}

/// What one tick of the IK reads off the body, the same for every leg.
struct IkTick {
    /// The standing smoother's rate (0.02 or 0.2 rad).
    step: f32,
    /// The locomotion smoother's rate.
    moving_step: f32,
    /// Twice the lift term before clamping (m), which lowers an
    /// unanchored standing target.
    drop: f32,
    /// The lift term before clamping (cm), which the locomotion IK adds
    /// to the target's vertical difference.
    lift_cm: f32,
    /// Whether the balance routine runs a recovery step (its angle
    /// smoother's 0.2 rad rate and its thigh's turn toward the target).
    stepping: bool,
    /// The placement's snap: write each leg's correction at once instead
    /// of easing toward it, and drop the thigh's extra turn.
    snap: bool,
    /// The run flag (the locomotion flags' bit 4).
    running: bool,
    stance: f32,
    crouch: f32,
    frame: Affine3A,
    sole: f32,
}

impl Ragdoll {
    /// The sampler's IK step, on `chain` (the sampled parts in the body
    /// root's frame, the clip's pose before the IK): records each leg's
    /// clip hip and ankle from it, multiplies last tick's leg bend into the
    /// thigh and shin, and recomposes the parts under the shin (the foot)
    /// on the bent shin, as the original's IK chain places every part by
    /// its parent's. The next [`Ragdoll::aim`] puts the record in the world
    /// after its carry. A body without muscles is left alone.
    pub fn fold_leg_ik(&mut self, chain: &mut [Affine3A]) {
        let Ragdoll {
            parts, legs, drive, ..
        } = self;
        if drive.is_none() || chain.len() != parts.len() {
            return;
        }
        for leg in legs.iter_mut() {
            let Some(thigh) = parts[leg.shin].parent else {
                continue;
            };
            let hip = Vec3::from(chain[thigh].translation);
            let ankle = chain[leg.shin].transform_point3(leg.point);
            leg.sampled_clip = Some((hip, ankle));
            let shin_before = chain[leg.shin];
            fold_pose(chain, thigh, leg.shin, leg);
            // Parts come after their parents: each descendant of the shin
            // keeps its clip placement under its re-placed parent.
            let mut before: Vec<(usize, Affine3A)> = vec![(leg.shin, shin_before)];
            for i in leg.shin + 1..parts.len() {
                let Some(parent) = parts[i].parent else {
                    continue;
                };
                let Some(&(_, parent_before)) = before.iter().find(|(j, _)| *j == parent) else {
                    continue;
                };
                let old = chain[i];
                chain[i] = chain[parent] * (parent_before.inverse() * old);
                before.push((i, old));
            }
        }
    }

    /// The placement's reset of the legs' IK corrections: the knee's and
    /// the hip's (with the thigh's step turn) are the identity until the
    /// IK writes them again, as the original's placement resets every
    /// part's correction turns before it samples the first clip; the
    /// smoothed angles are kept.
    pub fn clear_leg_corrections(&mut self) {
        for leg in &mut self.legs {
            leg.knee_shown = false;
            leg.hip_shown = false;
        }
    }

    /// Puts the clip's hip-to-ankle record of this tick in the world
    /// (`root`, after the carry): the sampler's record when its IK step
    /// folded the pose; otherwise, for a pose given without a sample, the
    /// record is read from the targets and last tick's bend folded into
    /// them here. A correction the IK holds at the identity is not
    /// multiplied in.
    pub(super) fn pose_leg_bend(&mut self, root: Affine3A) {
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
            if let Some((hip, ankle)) = leg.sampled_clip.take() {
                let (hip, ankle) = (root.transform_point3(hip), root.transform_point3(ankle));
                leg.posed_offset = ankle - hip;
                leg.anim_contact = ankle;
                leg.anim_hip = hip;
                continue;
            }
            let hip = Vec3::from(drive.targets[thigh].translation);
            let ankle = drive.targets[leg.shin].transform_point3(leg.point);
            leg.posed_offset = ankle - hip;
            leg.anim_contact = ankle;
            leg.anim_hip = hip;
            fold_pose(&mut drive.targets, thigh, leg.shin, leg);
        }
    }

    /// One tick of the leg IK, from where the solver left the body. The
    /// pose the muscles just followed is not rewritten. Lying down (off
    /// its feet, and not getting up) eases the stored knee, hip and foot
    /// pitch toward 0 by 0.02 rad and leaves the target, the lift and the
    /// thigh's extra turn alone.
    pub(super) fn bend_legs(&mut self) {
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
        if drive.targets.len() != parts.len() {
            return;
        }
        // The feet routine's flat-floor and hovering modes call no leg IK:
        // the stored corrections stay as they were.
        if drive.feet_mode != super::FeetMode::Walking {
            return;
        }
        if drive.bearing.off_feet && !drive.bearing.getting_up {
            for leg in legs.iter_mut() {
                let (knee, knee_rate) = ease(leg.knee, leg.knee_rate, 0.0, EASE_STANDING);
                leg.knee = knee;
                leg.knee_rate = knee_rate;
                let (hip, hip_rate) = ease(leg.hip, leg.hip_rate, 0.0, EASE_STANDING);
                leg.hip = hip;
                leg.hip_rate = hip_rate;
                let (pitch, pitch_rate) = ease(leg.pitch, leg.pitch_rate, 0.0, EASE_STANDING);
                leg.pitch = pitch;
                leg.pitch_rate = pitch_rate;
            }
            return;
        }
        let moving = drive.bearing.moving;
        let tick = IkTick {
            step: if drive.steps.stepping {
                EASE_STEPPING
            } else {
                EASE_STANDING
            },
            moving_step: (f64::from(drive.bearing.speed) * EASE_MOVING + EASE_MOVING_BASE) as f32,
            drop: 2.0 * drive.lift_raw / CM,
            lift_cm: drive.lift_raw,
            stepping: drive.steps.stepping,
            snap: false,
            running: drive.bearing.legs == LegMode::Run,
            stance: drive.stance,
            crouch: drive.bearing.crouch,
            frame: drive.frame.unwrap_or(Affine3A::IDENTITY),
            sole: *sole,
        };
        for leg in legs.iter_mut() {
            if moving {
                locomoting_leg(parts, leg, &tick);
            } else {
                standing_leg(parts, leg, &tick);
            }
        }
    }

    /// The placement's leg IK: the standing IK with its snap set, so every
    /// leg takes the correction its anchor asks for at once (the
    /// original's placement calls the same routine with its snap argument
    /// set). The corrections are folded into the next sampled pose.
    pub(super) fn snap_legs(&mut self, frame: Affine3A) {
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
        if drive.targets.len() != parts.len() {
            return;
        }
        let tick = IkTick {
            step: EASE_STANDING,
            moving_step: EASE_STANDING,
            drop: 2.0 * drive.lift_raw / CM,
            lift_cm: drive.lift_raw,
            stepping: false,
            snap: true,
            running: false,
            stance: drive.stance,
            crouch: drive.bearing.crouch,
            frame,
            sole: *sole,
        };
        for leg in legs.iter_mut() {
            standing_leg(parts, leg, &tick);
        }
    }
}

/// One leg's standing IK: the original's targets, triangle and corrections.
/// The feet routine runs it with its snap argument clear, so the
/// corrections always ease; the placement runs it with the snap set
/// ([`Ragdoll::snap_legs`]).
fn standing_leg(parts: &[Part], leg: &mut Leg, tick: &IkTick) {
    let Some(thigh) = parts[leg.shin].parent else {
        return;
    };
    let shin_len = leg.point.length();
    let thigh_len = parts[leg.shin].origin.length();
    if shin_len <= 0.0 || thigh_len <= 0.0 {
        return;
    }
    let contact = parts[leg.shin].point(&leg.contact);
    let anchored = leg.state.has(FootState::ANCHORED);
    // The standing IK clears the per-leg lag before it reads it.
    leg.contact_lag = 0.0;
    // The original's foot height for the pitch: the contact less the sole
    // and the foot's floor.
    leg.pitch_height = (contact.y - tick.sole - leg.floor) * CM;
    let to_anchor = leg.anchor - contact;
    let fresh = step_lift(
        cm(to_anchor.with_y(0.0)).length(),
        cm(to_anchor).length(),
        tick.crouch,
        tick.stance,
    );
    leg.lift = if leg.ground.is_none() {
        // A foot with no ground in reach keeps no step lift.
        0.0
    } else {
        // The feet routine's lift: fresh for an anchored foot while the
        // body's recovery step runs.
        next_lift(leg.lift, anchored, tick.stepping, fresh / CM)
    };
    let goal = bend_target(contact, leg.anchor, anchored, tick.drop);
    leg.goal = goal;
    let hip = Vec3::from(parts[thigh].transform().translation);
    // Stepping, the horizontal reach is to the target (anchored) or the
    // anchor; otherwise the clip's own hip-to-ankle offset.
    let horizontal = if tick.stepping {
        let at = if anchored { goal } else { leg.anchor };
        (at - hip).with_y(0.0)
    } else {
        leg.posed_offset.with_y(0.0)
    };
    // The hip over the anchor (plus the lag, 0 here), plus the lift term.
    let vertical = hip.y - leg.anchor.y + tick.drop * 0.5;
    let reach = (vertical * vertical + horizontal.length_squared()).sqrt() + leg.lift;
    if !reach.is_finite() {
        return;
    }
    // The reach triangle and the clip's own (lengths in cm, as the
    // original's eps compares are).
    let full = thigh_len + shin_len;
    let reach = if reach >= full {
        full
    } else {
        (full * REACH_MIN).max(reach)
    };
    leg.reach_knee = knee_angle(thigh_len, shin_len, reach);
    let clip = leg.posed_offset.length();
    let clip_knee = if clip > 0.0 {
        knee_angle(thigh_len, shin_len, clip)
    } else {
        FLAT
    };
    let (full_cm, clip_cm) = (full * CM, clip * CM);
    let clip_hip = clip_hip_angle(
        thigh_len * CM,
        shin_len * CM,
        full_cm * full_cm,
        clip_cm * clip_cm,
    );
    let target_hip = hip_angle(thigh_len, shin_len, reach);
    leg.knee_shown = true;
    leg.hip_shown = true;
    let (knee, knee_rate) = if tick.snap {
        (clip_knee - leg.reach_knee, 0.0)
    } else {
        ease(
            leg.knee,
            leg.knee_rate,
            clip_knee - leg.reach_knee,
            tick.step,
        )
    };
    leg.knee = knee;
    leg.knee_rate = knee_rate;
    let (hip_corr, hip_rate) = if tick.snap {
        (clip_hip - target_hip, 0.0)
    } else {
        ease(leg.hip, leg.hip_rate, clip_hip - target_hip, tick.step)
    };
    leg.hip = hip_corr;
    leg.hip_rate = hip_rate;
    if tick.snap {
        leg.turn = Quat::IDENTITY;
    } else if tick.stepping {
        let into = tick.frame.inverse();
        let rest = into.transform_vector3(leg.posed_offset);
        let toward = into.transform_vector3(goal - hip);
        leg.turn = extra_turn(rest, toward);
    } else {
        leg.turn = decay_turn(leg.turn);
    }
}

/// One leg's locomotion IK. The negated compares keep the original's
/// unordered jumps on a NaN.
#[allow(clippy::neg_cmp_op_on_partial_ord)]
fn locomoting_leg(parts: &[Part], leg: &mut Leg, tick: &IkTick) {
    let Some(thigh) = parts[leg.shin].parent else {
        return;
    };
    let shin_len = leg.point.length();
    let thigh_len = parts[leg.shin].origin.length();
    if shin_len <= 0.0 || thigh_len <= 0.0 {
        return;
    }
    let full = thigh_len + shin_len;
    let (thigh_cm, shin_cm) = (thigh_len * CM, shin_len * CM);
    let full2 = (full * CM) * (full * CM);
    let contact = parts[leg.shin].point(&leg.contact);
    let touching = leg.state.has(FootState::TOUCHING);
    let lifted = leg.state.has(FootState::LIFTED);
    // The original's per-leg lag: zeroed by the standing IK and only ever
    // lowered further, so from zero it stays zero (a guess about the value
    // the feet routine leaves it at).
    if !touching {
        leg.contact_lag = 0.0;
    }
    let lag = leg.contact_lag / CM;
    // The foot's height: planted, over its sole floor; lifted, over its
    // smoothed ground and its ground offset.
    leg.pitch_height = if touching {
        (contact.y - tick.sole - leg.floor) * CM
    } else {
        (contact.y - leg.offset - leg.smoothed) * CM
    };
    let hip = Vec3::from(parts[thigh].transform().translation);
    let anim_contact = leg.anim_contact;
    let anim_hip = leg.anim_hip;
    // The vertical difference from the physical hip down to the anchor
    // (plus the lag), and the clip's own hip over its ankle. cm, in the
    // original's order.
    let d = (hip.y - (leg.anchor.y + lag)) * CM;
    let clip_len = (anim_hip.y - anim_contact.y) * CM;
    // The IK acts where the clip's leg is longer than the physical hip's
    // height over the anchor (the ground is higher than the clip
    // expects), or while a touching foot does not run. A NaN difference
    // takes the IK, as the original's unordered jump does.
    let acts = !(clip_len <= d) || (touching && !tick.running);
    if !acts {
        // Both corrections reset at once while their angles ease down.
        let (knee, knee_rate) = ease(leg.knee, leg.knee_rate, 0.0, tick.moving_step);
        leg.knee = knee;
        leg.knee_rate = knee_rate;
        leg.knee_shown = false;
        let (hip_corr, hip_rate) = ease(leg.hip, leg.hip_rate, 0.0, tick.moving_step);
        leg.hip = hip_corr;
        leg.hip_rate = hip_rate;
        leg.hip_shown = false;
        return;
    }
    // The target leg: from the physical hip across the ground to the
    // foot's contact, and down by `d`. Its length squared, capped at the
    // whole leg's.
    let horizontal = (contact - hip).with_y(0.0) * CM;
    let horizontal2 = horizontal.length_squared();
    let half = 0.5 / (thigh_cm * shin_cm);
    let (distance2, knee_target, hip_target) = if horizontal2 <= PITCH_EPS {
        (d * d, FLAT, 0.0)
    } else {
        let raw = d * d + horizontal2;
        // Unordered: a NaN takes the triangle.
        if !(full2 <= raw) {
            let cos_knee = (thigh_cm * thigh_cm + shin_cm * shin_cm - raw) * half;
            let knee = if cos_knee <= -1.0 {
                FLAT
            } else if f64::from(cos_knee) > FOLD_COS {
                FOLD
            } else {
                acos_approx(cos_knee)
            };
            let cos_hip =
                (thigh_cm * thigh_cm + raw - shin_cm * shin_cm) / (raw.sqrt() * 2.0 * thigh_cm);
            (raw, knee, target_hip_angle(cos_hip))
        } else {
            (full2, FLAT, 0.0)
        }
    };
    // The clip's triangle, from the clip's own hip-to-ankle vector.
    let clip2 = (anim_contact - anim_hip).length_squared() * CM * CM;
    let cos_clip_knee = (thigh_cm * thigh_cm + shin_cm * shin_cm - clip2) * half;
    let clip_knee = if cos_clip_knee <= -1.0 {
        FLAT
    } else if f64::from(cos_clip_knee) > FOLD_COS {
        FOLD
    } else {
        acos_approx(cos_clip_knee)
    };
    let clip_hip = clip_hip_angle(thigh_cm, shin_cm, full2, clip2);
    let (knee, knee_rate) = ease(
        leg.knee,
        leg.knee_rate,
        clip_knee - knee_target,
        tick.moving_step,
    );
    leg.knee = knee;
    leg.knee_rate = knee_rate;
    leg.knee_shown = true;
    // The angle between the clip's leg and the target leg: their length
    // and the vertical difference of their ends give the third side. The
    // original takes the anchor's height plus the lag (less twice the lift
    // term while the foot touches or is lifted) off the hip's, then that
    // off the clip's own difference, and its magnitude.
    let target_y = leg.anchor.y + lag;
    let mut physical = (target_y - hip.y) * CM;
    if touching || lifted {
        physical -= 2.0 * tick.lift_cm;
    }
    let side = ((anim_contact.y - anim_hip.y) * CM - physical).abs();
    // Unordered: a NaN resets the hip too.
    if !(distance2 > PITCH_EPS) {
        let (hip_corr, hip_rate) = ease(leg.hip, leg.hip_rate, 0.0, tick.moving_step);
        leg.hip = hip_corr;
        leg.hip_rate = hip_rate;
        leg.hip_shown = false;
        return;
    }
    let cos = (distance2 + clip2 - side * side) / (distance2.sqrt() * (clip2.sqrt() * 2.0));
    let between = if cos <= -1.0 {
        FLAT
    } else if cos >= 1.0 {
        0.0
    } else {
        acos_approx(cos)
    };
    // The sign: the original negates the angle unless the clip's foot is
    // ahead of the character frame's origin (its stance-local z above 0;
    // a NaN negates).
    let ahead = tick.frame.inverse().transform_point3(anim_contact).z;
    let signed = if ahead > 0.0 { between } else { -between };
    let (hip_corr, hip_rate) = ease(
        leg.hip,
        leg.hip_rate,
        signed - hip_target + clip_hip,
        tick.moving_step,
    );
    leg.hip = hip_corr;
    leg.hip_rate = hip_rate;
    leg.hip_shown = true;
    leg.turn = decay_turn(leg.turn);
}

/// Writes one leg's stored correction into the pose: the thigh's turn and
/// hip hinge, then the shin's knee hinge, each local to the clip's turn
/// (the original's `q × turn × hinge` and `q × hinge`).
fn fold_pose(targets: &mut [Affine3A], thigh: usize, shin: usize, leg: &Leg) {
    if thigh >= targets.len() || shin >= targets.len() {
        return;
    }
    let thigh_turn = if leg.hip_shown {
        leg.turn * Quat::from_rotation_x(leg.hip)
    } else {
        Quat::IDENTITY
    };
    let knee_turn = if leg.knee_shown {
        Quat::from_rotation_x(leg.knee)
    } else {
        Quat::IDENTITY
    };
    let clip_thigh = targets[thigh];
    let local = clip_thigh.inverse() * targets[shin];
    targets[thigh] = clip_thigh * Affine3A::from_quat(thigh_turn);
    targets[shin] = targets[thigh] * local * Affine3A::from_quat(knee_turn);
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

    /// The foot rest turn is (−1, 0, 0) by (1 − height)·−1.5° in combat,
    /// 5° splayed, none otherwise, and the second leg takes it back.
    #[test]
    fn the_foot_rest_turn_is_the_originals() {
        let axis = Vec3::NEG_X;
        assert_eq!(foot_rest_turn(FemurPosture::Level, 0), Quat::IDENTITY);
        let splay = foot_rest_turn(FemurPosture::Splayed, 0);
        assert!(splay.abs_diff_eq(Quat::from_axis_angle(axis, 5.0 * 0.017_453_292), 1e-6));
        assert!(foot_rest_turn(FemurPosture::Splayed, 1).abs_diff_eq(splay.conjugate(), 1e-6));
        let combat = foot_rest_turn(FemurPosture::Combat { height: 0.5 }, 0);
        assert!(combat.abs_diff_eq(Quat::from_axis_angle(axis, -0.75 * 0.017_453_292), 1e-6));
        // A NaN height keeps a non-finite turn, as the original's
        // arithmetic does.
        assert!(!foot_rest_turn(FemurPosture::Combat { height: f32::NAN }, 0).is_finite());
    }
}
