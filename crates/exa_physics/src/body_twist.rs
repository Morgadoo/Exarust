//! The torso twist the original's body tick computes before its sampler:
//! the look direction in the character frame, its yaw clamped to a radian
//! and followed at a rate (0.05 rad a tick standing, 0.02 locomoting, the
//! weapon's handling × 0.12 × the swing strength while attacking), lowered
//! by the crouch, made a look rotation whose forward is that direction;
//! the identity while the condition is helpless or the tone is 0.85 or
//! less. The sampler turns the spine by it ([`twist_spine`]).
//!
//! Evidence: reverse engineering of the body tick's twist block and the
//! living sampler's midline step (`docs/formats/RSQ.md`, Torso twist).
//! (Included from body_locomotion.rs: `#[path = "body_twist.rs"] mod twist;`)

// The original's compares send NaN down one branch; the negated compares
// say which.
#![allow(clippy::neg_cmp_op_on_partial_ord)]

use bevy::math::{Affine3A, Mat3A, Vec3A};
use bevy::prelude::*;

use super::{Locomotor, forward};
pub use crate::engine_math::{acos_approx, asin_approx, euler_of, from_euler, look_rotation, sign};

/// The twist holds below this tone (single precision, its compare).
const TWIST_TONE: f32 = 0.85;
/// The twist's yaw is clamped to this (rad).
const TWIST_REACH: f32 = 1.0;
/// It follows its target this much a tick: locomoting, standing; while
/// attacking the weapon's handling × 0.12 (double) × the swing strength.
const FOLLOW_MOVING: f32 = 0.02;
const FOLLOW_STANDING: f32 = 0.05;
const FOLLOW_ATTACK: f64 = 0.12;
/// The crouch lowers the direction by this × the crouch while attacking
/// (double), else by the crouch itself.
const CROUCH_ATTACK: f64 = 0.3;
const CROUCH_LOWER: f32 = 1.0;
/// The lowered direction's height is then × 0.75 (casting, a lowered one
/// is −y² instead).
const HEIGHT_SHARE: f32 = 0.75;

/// What the twist reads of the character this tick.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TwistBy {
    /// Its condition is helpless (down, getting up, dormant, collapsing,
    /// dead or shocked): no twist.
    pub helpless: bool,
    pub tone: f32,
    /// Where it looks (a yaw).
    pub look: f32,
    /// The character frame's yaw: the body's yaw before this tick's turn
    /// (the original rebuilds its frame after the sampler).
    pub frame: f32,
    /// The decision moves it.
    pub moving: bool,
    /// Its condition is attacking: the twist follows at the weapon's
    /// `handling` × 0.12 × the swing `strength`, and the crouch lowers it
    /// by 0.3 of itself.
    pub attack: Option<(f32, f32)>,
    /// Its condition is casting a power.
    pub casting: bool,
    pub crouch: f32,
    /// The direction's height before the crouch (the original's run lean:
    /// the body's persistent head tilt).
    pub tilt: f32,
}

/// The original's yaw of a direction: acos(z/|xz|) by its approximation
/// ([`acos_approx`]), negative for a negative x; 0 when its horizontal
/// length² is under 1e-8; π at a cosine of −1 or less, 0 above 1 (a NaN
/// length or cosine goes to the approximation, as the original's compares
/// send it).
pub fn heading_of(d: Vec3) -> f32 {
    crate::engine_math::heading(d)
}

/// `v` less its component along the unit `n` (the dot product summed x
/// and z, then y, as the original's).
fn without(v: Vec3A, n: Vec3A) -> Vec3A {
    let d = v.x * n.x + v.z * n.z + v.y * n.y;
    v - n * d
}

/// One tick of the torso twist (`body.twist_yaw` the yaw it follows,
/// `body.twist` the rotation; the identity when it holds).
pub fn twist(body: &mut Locomotor, by: &TwistBy) {
    if by.helpless || !(by.tone > TWIST_TONE) {
        body.twist = Quat::IDENTITY;
        return;
    }
    let local = Quat::from_rotation_y(-by.frame) * forward(by.look);
    let mut yaw = heading_of(local);
    if yaw.abs() > TWIST_REACH {
        yaw = sign(yaw) * TWIST_REACH;
    }
    let step = if by.moving {
        FOLLOW_MOVING
    } else if let Some((handling, strength)) = by.attack {
        (f64::from(strength) * (f64::from(handling) * FOLLOW_ATTACK)) as f32
    } else {
        FOLLOW_STANDING
    };
    let kept = body.twist_yaw;
    body.twist_yaw = if (yaw - kept).abs() > step {
        if kept < yaw { kept + step } else { kept - step }
    } else {
        yaw
    };
    let flat = forward(body.twist_yaw);
    let mut y = by.tilt;
    y = if by.attack.is_some() {
        (f64::from(y) - f64::from(by.crouch) * CROUCH_ATTACK) as f32
    } else {
        y - CROUCH_LOWER * by.crouch
    };
    // The first normalisation's length (the height is set after it).
    let n = Vec3::new(flat.x, y, flat.z).normalize_or_zero();
    let y = if by.casting && !(0.0 < n.y) {
        -(n.y * n.y)
    } else {
        n.y * HEIGHT_SHARE
    };
    let d = Vec3::new(n.x, y, n.z);
    let length = (d.y * d.y + 0.0 + d.x * d.x + d.z * d.z).sqrt();
    let dir = d * (1.0 / length);
    body.twist = Quat::from_mat3a(&look_rotation(dir));
}

/// The sampler's midline step: each spine part's turn (in the body's
/// frame) is turned by the twist; the one whose parent is the root (the
/// lumbar) is then taken back halfway: its forward axis midway between
/// twisted and not, its up made square to it, its side their cross
/// product. Positions keep their pose (the support pins read them).
pub fn twist_spine(parts: &mut [Affine3A], spine: &[(usize, bool)], twist: Quat) {
    if twist == Quat::IDENTITY {
        return;
    }
    let turn = Mat3A::from_quat(twist);
    for &(i, half) in spine {
        let Some(p) = parts.get_mut(i) else {
            continue;
        };
        let pose = p.matrix3;
        let mut m = turn * pose;
        if half {
            let z = (m.z_axis + (pose.z_axis - m.z_axis) * 0.5).normalize_or_zero();
            let y = without(m.y_axis, z).normalize_or_zero();
            let x = y.cross(z).normalize_or_zero();
            m = Mat3A::from_cols(x, y, z);
        }
        p.matrix3 = m;
    }
}

/// The head's look yaw is clamped to this (rad, double), × 0.8, and
/// followed at most this much a tick (double); held, it decays × 0.09.
const LOOK_REACH: f64 = 1.256_637_061_435_917_2;
const LOOK_SHARE: f64 = 0.8;
const LOOK_RATE: f64 = 0.031_415_926_535_897_934;
const LOOK_DECAY: f64 = 0.09;
/// The roll is this share of the look yaw (double).
const LOOK_ROLL: f64 = 0.1;

/// The head look: the head's turn (in the character frame) `head` blended
/// by `weight` toward the look along `dir` (unit, in the character frame):
/// its yaw by the original's arc cosine, within ±1.2566 rad, × 0.8,
/// followed by `yaw` (the kept look yaw) at most 0.0314 rad a tick; its
/// pitch −asin(dir's height); its roll a tenth of the yaw. While the
/// condition is `helpless` the kept yaw decays × 0.09 and the pitch is 0.
/// Each of the head's angles moves `weight` of the way to the look's.
pub fn head_look(head: &Mat3A, dir: Vec3, weight: f32, yaw: &mut f32, helpless: bool) -> Mat3A {
    let clip = euler_of(head);
    let pitch = if helpless {
        *yaw = (f64::from(*yaw) * LOOK_DECAY) as f32;
        0.0
    } else {
        let mut look = heading_of(dir);
        if f64::from(look.abs()) > LOOK_REACH {
            look = (f64::from(sign(look)) * LOOK_REACH) as f32;
        }
        look = (f64::from(look) * LOOK_SHARE) as f32;
        let change = look - *yaw;
        if f64::from(change.abs()) > LOOK_RATE {
            look = (f64::from(sign(change)) * LOOK_RATE + f64::from(*yaw)) as f32;
        }
        *yaw = look;
        -asin_approx(dir.y)
    };
    let roll = (f64::from(*yaw) * LOOK_ROLL) as f32;
    let rest = 1.0 - weight;
    from_euler(Vec3::new(
        rest * clip.x + pitch * weight,
        rest * clip.y + *yaw * weight,
        rest * clip.z + roll * weight,
    ))
}

#[cfg(test)]
#[path = "body_twist_test.rs"]
mod tests;
