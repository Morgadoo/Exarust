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
/// The horizontal length² under which a direction has no yaw.
const NO_YAW: f32 = 1e-8;

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
    /// The direction's height before the crouch (the original's run lean,
    /// from an exertion Exarust does not keep: 0).
    pub tilt: f32,
}

/// The original's yaw of a direction: acos(z/|xz|) by its approximation
/// ([`acos_approx`]), negative for a negative x; 0 when its horizontal
/// length² is under 1e-8; π at a cosine of −1 or less, 0 above 1 (a NaN
/// length or cosine goes to the approximation, as the original's compares
/// send it).
pub fn heading_of(d: Vec3) -> f32 {
    let flat = d.z * d.z + d.x * d.x;
    if flat < NO_YAW {
        return 0.0;
    }
    let c = d.z / flat.sqrt();
    let yaw = if !(c <= -1.0) {
        if !(c > 1.0) { acos_approx(c) } else { 0.0 }
    } else {
        std::f32::consts::PI
    };
    if d.x < 0.0 { -yaw } else { yaw }
}

/// The original's arc cosine, a polynomial (within about 4e-4 rad):
/// (π/2 − 0.2075·x + 0.0535·x²)·√(1 − x) for x ≥ 0 (or NaN), and π less
/// the same of −x (in double precision) below 0.
pub fn acos_approx(x: f32) -> f32 {
    const HALF_PI: f32 = std::f32::consts::FRAC_PI_2;
    const LINEAR: f32 = 0.2075;
    const SQUARE: f32 = 0.0535;
    if !(x < 0.0) {
        let a = HALF_PI - x * LINEAR;
        (x * x * SQUARE + a) * (1.0 - x).sqrt()
    } else {
        let a = x * LINEAR + HALF_PI;
        let r = (x * x * SQUARE + a) * (1.0 + x).sqrt();
        (std::f64::consts::PI - f64::from(r)) as f32
    }
}

/// The original's sign: 1 or −1 by the sign bit, 0 for zero.
fn sign(x: f32) -> f32 {
    if x == 0.0 { 0.0 } else { 1.0_f32.copysign(x) }
}

/// `v` less its component along the unit `n` (the dot product summed x
/// and z, then y, as the original's).
fn without(v: Vec3A, n: Vec3A) -> Vec3A {
    let d = v.x * n.x + v.z * n.z + v.y * n.y;
    v - n * d
}

/// The look rotation along `dir` (unit): its forward (z) is `dir`, its up
/// the vertical less its part along `dir`, its side their cross product.
pub fn look_rotation(dir: Vec3) -> Mat3A {
    let z = Vec3A::from(dir);
    let y = without(Vec3A::Y, z).normalize_or_zero();
    let x = y.cross(z).normalize_or_zero();
    Mat3A::from_cols(x, y, z)
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

/// The original's arc sine, the same polynomial: π/2 − (π/2 − (0.2075 −
/// 0.0535·x)·x)·√(1 − x) for x ≥ 0 (or NaN), ((0.0535·x + 0.2075)·x +
/// π/2)·√(1 + x) − π/2 below 0.
pub fn asin_approx(x: f32) -> f32 {
    const HALF_PI: f32 = std::f32::consts::FRAC_PI_2;
    const LINEAR: f32 = 0.2075;
    const SQUARE: f32 = 0.0535;
    if !(x < 0.0) {
        let a = HALF_PI - (LINEAR - x * SQUARE) * x;
        HALF_PI - a * (1.0 - x).sqrt()
    } else {
        ((SQUARE * x + LINEAR) * x + HALF_PI) * (1.0 + x).sqrt() - HALF_PI
    }
}

/// The original's angles of a turn ([`from_euler`] composes them back):
/// the roll z = asin(x axis's y), and with |cos z| over 0.005 (double) the
/// pitch x = atan2(−z axis's y, y axis's y) and the yaw y = atan2(−x axis's
/// z, x axis's x), each over cos z; otherwise no pitch and the yaw
/// atan2(z axis's x, z axis's z). Double-precision functions, as the
/// original's.
pub fn euler_of(m: &Mat3A) -> Vec3 {
    let roll = f64::from(m.x_axis.y).asin() as f32;
    let c = f64::from(roll).cos() as f32;
    if f64::from(c.abs()) > ROLL_LIMIT {
        let pitch = f64::from(-m.z_axis.y / c).atan2(f64::from(m.y_axis.y / c)) as f32;
        let yaw = f64::from(-m.x_axis.z / c).atan2(f64::from(m.x_axis.x / c)) as f32;
        Vec3::new(pitch, yaw, roll)
    } else {
        let yaw = f64::from(m.z_axis.x).atan2(f64::from(m.z_axis.z)) as f32;
        Vec3::new(0.0, yaw, roll)
    }
}

/// Above this |cos roll| (double) the pitch and the yaw are told apart.
const ROLL_LIMIT: f64 = 0.005;

/// The turn of the original's angles (x pitch, y yaw, z roll): its x axis
/// (cz·cy, sz, −cz·sy), y axis (sx·sy − cx·sz·cy, cx·cz, sx·cy + cx·sz·sy),
/// z axis (cx·sy + sx·sz·cy, −sx·cz, cx·cy − sx·sz·sy).
pub fn from_euler(e: Vec3) -> Mat3A {
    let (sx, cx) = e.x.sin_cos();
    let (sy, cy) = e.y.sin_cos();
    let (sz, cz) = e.z.sin_cos();
    let cxsz = cx * sz;
    let sxsz = sx * sz;
    Mat3A::from_cols(
        Vec3A::new(cz * cy, sz, -cz * sy),
        Vec3A::new(sx * sy + -cxsz * cy, cx * cz, sx * cy + cxsz * sy),
        Vec3A::new(cx * sy + sxsz * cy, -sx * cz, cx * cy + -sxsz * sy),
    )
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
