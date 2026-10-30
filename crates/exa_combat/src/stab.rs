//! The stab's aim and its arm drive.
//!
//! A stab does not only play its clip. While the clip runs, the fighter's
//! behaviour keeps a stab aim, and the body tick pulls the weapon onto the
//! line from a point by the shoulder to that aim and drives it along the
//! line, so the point of a long weapon arrives first.
//!
//! The AI's aim ([`StabAim`]) starts on the attack's aim yaw, level. Every
//! behaviour tick of the stab it turns toward the target point by at most
//! 0.0314 rad in yaw and in pitch, looks up by no more than 0.6 rad and
//! keeps within 45° of the body's heading. The direction is taken from an
//! anchor 20 cm to the side of and 10 cm above the character frame's
//! origin. A target closer than 50 cm across the ground keeps the last aim,
//! and one between 50 and 100 cm has its height replaced by
//! `(10000 − h²)·0.0001`. The aim point lies 250 cm out along the aim.
//!
//! The drive ([`stab_drive`]) runs from clock 0.1 while the stab has a held
//! weapon and has not landed a blow. With `s = S((c − 0.1)·1.65)` (`S(u) =
//! (3 − 2u)·u²`, floored at 0), the weapon point 0.9 up the first held item
//! is pushed across onto the line by `6·c·s` of its distance from it, and
//! the left hand by `c²·s` of its own. Past clock 0.4, with `g = 4·(1 − c)²·
//! (3 − 2t)·t` (`t = c − 0.4`, floored at 0), both are driven along the line
//! as well: the weapon by `48·g·(W7 + 0.8)` cm and the hand by `12·g·
//! (W7 + 0.8)·(1 − c)²` cm, W7 the weapon's balance.
//!
//! Evidence: the AI behaviour's stab-aim slot and the body tick's stab arm
//! drive (REA, `Exanima.exe`).

use bevy::prelude::*;
use exa_anim::{quick_asin, quick_direction, quick_yaw};
use exa_physics::engine_math::normalize_approx;

/// The aim turns this far a tick at most, in yaw and in pitch (radians).
pub const AIM_TURN: f32 = 0.031_415_93;
/// The aim keeps within this angle of the body's heading (radians).
pub const AIM_SPREAD: f32 = 0.785_398_2;
/// The aim looks up by no more than this (a pitch below it is raised).
pub const AIM_HIGHEST: f32 = -0.6;
/// The aim point lies this far out along the aim (cm).
pub const AIM_POINT_CM: f32 = 250.0;
/// The anchor the aim and the drive measure from, in the character
/// frame (cm): 20 cm to the side, 10 cm above the frame's origin.
pub const ANCHOR_CM: Vec3 = Vec3::new(-20.0, 10.0, 0.0);
/// Below this squared distance across the ground (cm²) the aim stays.
const NEAR2: f32 = 2500.0;
/// Below this squared distance across the ground (cm²) the height is
/// replaced.
const CLOSE2: f32 = 10000.0;
/// The drive starts past this clock.
pub const DRIVE_FROM: f32 = 0.1;
/// The drive pushes along the line past this clock.
pub const ALONG_FROM: f32 = 0.4;
/// The across ramp's rate.
const ACROSS_RATE: f32 = 1.65;
/// The weapon point's across gain, times the clock and the ramp.
const ITEM_ACROSS: f32 = 6.0;
/// The weapon point's push along the line (cm), times the along ramp and
/// W7 + 0.8.
const ITEM_ALONG_CM: f32 = 48.0;
/// The hand's push along the line (cm), times the along ramp, W7 + 0.8 and
/// (1 − c)².
const HAND_ALONG_CM: f32 = 12.0;
/// Added to the weapon's balance for the along pushes.
const BALANCE_PLUS: f32 = 0.8;
/// The weapon point sits this share of the item's length up from its grip.
pub const ITEM_POINT_SHARE: f32 = 0.9;

const PI: f32 = 3.141_592_7;
const TURN: f32 = 6.283_185_5;

/// An angle brought back within ±π once, as the original wraps.
fn wrap(a: f32) -> f32 {
    if a < -PI {
        a + TURN
    } else if PI < a {
        a - TURN
    } else {
        a
    }
}

/// `from` moved toward `to` by at most `most`, wrapped.
fn turn_toward(from: f32, to: f32, most: f32) -> f32 {
    let gap = wrap(to - from);
    if most < gap {
        wrap(from + most)
    } else if gap < -most {
        wrap(from - most)
    } else {
        to
    }
}

/// A fighter's stab aim, which its behaviour keeps while a stab plays and
/// its body's stab drive reads.
#[derive(Component, Debug, Clone, Copy, Default, PartialEq)]
pub struct StabAim {
    /// The aim's yaw (radians, 0 along +z, toward +x).
    pub yaw: f32,
    /// The aim's pitch (radians, positive down).
    pub pitch: f32,
    /// The aim point the drive pulls toward (world, cm), once aimed.
    pub point_cm: Option<Vec3>,
}

impl StabAim {
    /// The aim as an attack starts: on the attack's aim yaw, level, with
    /// no point yet.
    pub fn start(aim_yaw: f32) -> Self {
        Self {
            yaw: aim_yaw,
            pitch: 0.0,
            point_cm: None,
        }
    }

    /// The aim's unit direction.
    pub fn direction(&self) -> Vec3 {
        quick_direction(self.pitch, self.yaw)
    }

    /// One behaviour tick toward `target_cm` from the anchor `anchor_cm`
    /// (world, cm), the body heading `heading_yaw`. Returns the aim point.
    pub fn step(&mut self, target_cm: Vec3, anchor_cm: Vec3, heading_yaw: f32) -> Vec3 {
        let mut v = target_cm - anchor_cm;
        let across2 = v.z * v.z + v.x * v.x;
        if NEAR2 <= across2 {
            if across2 < CLOSE2 {
                v.y = (CLOSE2 - across2) * 0.0001;
            }
            v = normalize_approx(v);
        } else {
            v = self.direction();
        }
        let yaw = turn_toward(self.yaw, quick_yaw(v.x, v.z), AIM_TURN);
        let mut pitch_to = -quick_asin(v.y);
        if pitch_to < AIM_HIGHEST {
            pitch_to = AIM_HIGHEST;
        }
        let pitch = turn_toward(self.pitch, pitch_to, AIM_TURN);
        let off = wrap(yaw - heading_yaw);
        let yaw = if AIM_SPREAD < off {
            wrap(heading_yaw + AIM_SPREAD)
        } else if off < -AIM_SPREAD {
            wrap(heading_yaw - AIM_SPREAD)
        } else {
            yaw
        };
        self.yaw = yaw;
        self.pitch = pitch;
        let point = anchor_cm + self.direction() * AIM_POINT_CM;
        self.point_cm = Some(point);
        point
    }
}

/// The two pushes of one tick of the stab drive (cm).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StabPushes {
    /// On the weapon point: toward the line and along it.
    pub item: Vec3,
    /// The weapon point's offset to the line (from the point to its
    /// nearest point on the line).
    pub item_to_line: Vec3,
    /// On the left hand.
    pub hand: Vec3,
    /// The line's direction (unit).
    pub dir: Vec3,
}

/// The across ramp at clock `c`: `S((c − 0.1)·1.65)`, 0 until 0.1.
fn across_ramp(c: f32) -> f32 {
    if c <= DRIVE_FROM {
        return 0.0;
    }
    let u = (c - DRIVE_FROM) * ACROSS_RATE;
    ((3.0 - u * 2.0) * u * u).max(0.0)
}

/// The along ramp at clock `c`: `4·(1 − c)²·(3 − 2t)·t`, `t = c − 0.4`, 0
/// until 0.4.
fn along_ramp(c: f32) -> f32 {
    if c <= ALONG_FROM {
        return 0.0;
    }
    let t = (c - ALONG_FROM) * 1.0;
    let d = 1.0 - c;
    (d * d * (3.0 - t * 2.0) * t * 4.0).max(0.0)
}

/// From `point` to the line through `anchor` along the unit `dir`: the
/// anchor's offset less its part along the line (the dot taken z, y, x).
fn across(anchor: Vec3, point: Vec3, dir: Vec3) -> Vec3 {
    let v = anchor - point;
    let along = v.z * dir.z + v.y * dir.y + v.x * dir.x;
    v + dir * -along
}

/// One tick of the stab drive at clock `clock`: the anchor, the aim point,
/// the weapon point and the left hand's centre (world, cm) and the
/// weapon's balance W7. None at or before clock 0.1 or with no direction
/// to the aim.
pub fn stab_drive(
    clock: f32,
    anchor_cm: Vec3,
    aim_point_cm: Vec3,
    item_cm: Vec3,
    hand_cm: Vec3,
    balance: f32,
) -> Option<StabPushes> {
    if clock <= DRIVE_FROM {
        return None;
    }
    let dir = normalize_approx(aim_point_cm - anchor_cm);
    if dir == Vec3::ZERO || !dir.is_finite() {
        return None;
    }
    let g = along_ramp(clock);
    let s = across_ramp(clock);
    let item_to_line = across(anchor_cm, item_cm, dir);
    let mut item = item_to_line * (s * clock * ITEM_ACROSS);
    let mut hand = across(anchor_cm, hand_cm, dir) * (clock * clock * s);
    if ALONG_FROM < clock {
        let rest = 1.0 - clock;
        item += dir * (g * (balance + BALANCE_PLUS) * ITEM_ALONG_CM);
        hand += dir * (g * (balance + BALANCE_PLUS) * rest * rest * HAND_ALONG_CM);
    }
    Some(StabPushes {
        item,
        item_to_line,
        hand,
        dir,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_aim_turns_a_little_a_tick_and_keeps_near_the_heading() {
        // A target straight ahead and level with the anchor: the aim is
        // already there, and its point lies 250 cm out.
        let mut aim = StabAim::start(0.0);
        let p = aim.step(Vec3::new(0.0, 0.0, 300.0), Vec3::ZERO, 0.0);
        assert_eq!((aim.yaw, aim.pitch), (0.0, 0.0));
        assert!((p - Vec3::new(0.0, 0.0, 250.0)).length() < 0.2, "{p}");
        // A target a right angle off: the yaw moves by 0.0314 rad a tick.
        let mut aim = StabAim::start(0.0);
        aim.step(Vec3::new(300.0, 0.0, 0.0), Vec3::ZERO, 0.0);
        assert_eq!(aim.yaw, AIM_TURN);
        for _ in 0..100 {
            aim.step(Vec3::new(300.0, 0.0, 0.0), Vec3::ZERO, 0.0);
        }
        // ... and stops 45° from the heading.
        assert_eq!(aim.yaw, AIM_SPREAD);
        // A target high above looks up by 0.6 rad at most.
        let mut aim = StabAim::start(0.0);
        for _ in 0..100 {
            aim.step(Vec3::new(0.0, 500.0, 200.0), Vec3::ZERO, 0.0);
        }
        assert_eq!(aim.pitch, AIM_HIGHEST);
        // A low target looks down.
        let mut aim = StabAim::start(0.0);
        aim.step(Vec3::new(0.0, -100.0, 200.0), Vec3::ZERO, 0.0);
        assert_eq!(aim.pitch, AIM_TURN);
    }

    #[test]
    fn a_near_target_keeps_the_aim_and_a_close_one_flattens() {
        // Inside 50 cm across the ground the last aim stays.
        let mut aim = StabAim {
            yaw: 0.3,
            pitch: 0.2,
            point_cm: None,
        };
        aim.step(Vec3::new(-30.0, -200.0, 30.0), Vec3::ZERO, 0.0);
        // (The engine's quick trigonometry round-trips to about 1e-3.)
        assert!((aim.yaw - 0.3).abs() < 2e-3 && (aim.pitch - 0.2).abs() < 2e-3);
        // Between 50 and 100 cm the height is (10000 − h²)·0.0001: at 60 cm
        // ahead that is 0.64 up, whatever the target's own height.
        let mut aim = StabAim::start(0.0);
        let mut low = aim;
        for _ in 0..40 {
            aim.step(Vec3::new(0.0, 500.0, 60.0), Vec3::ZERO, 0.0);
            low.step(Vec3::new(0.0, -500.0, 60.0), Vec3::ZERO, 0.0);
        }
        assert_eq!(aim.pitch, low.pitch);
        let up = -quick_asin(normalize_approx(Vec3::new(0.0, 0.64, 60.0)).y);
        assert!((aim.pitch - up).abs() < 1e-6, "{} {up}", aim.pitch);
    }

    #[test]
    fn the_drive_pulls_across_from_a_tenth_and_along_from_four_tenths() {
        let anchor = Vec3::ZERO;
        let aim = Vec3::new(0.0, 0.0, 250.0);
        // The weapon point 30 cm to the side of the anchor, the hand 10 cm
        // (level with it, so the across push has no part along the line).
        let item = Vec3::new(30.0, 0.0, 0.0);
        let hand = Vec3::new(10.0, 0.0, 0.0);
        assert_eq!(stab_drive(0.1, anchor, aim, item, hand, 0.5), None);
        // Clock 0.3: u = 0.33, s = (3 − 0.66)·0.1089 = 0.254826; across only.
        let p = stab_drive(0.3, anchor, aim, item, hand, 0.5).unwrap();
        let s = (3.0f32 - 0.33 * 2.0) * 0.33 * 0.33;
        assert!((p.item.x - -30.0 * (s * 0.3 * 6.0)).abs() < 1e-3, "{p:?}");
        assert!((p.hand.x - -10.0 * (0.09 * s)).abs() < 1e-4, "{p:?}");
        assert!(p.item.z.abs() < 1e-3 && p.hand.z.abs() < 1e-4, "{p:?}");
        // Clock 0.6: t = 0.2, g = 4·0.16·2.6·0.2 = 0.3328; the weapon goes
        // 48·g·1.3 = 20.77 cm along, the hand 12·g·1.3·0.16 = 0.831 cm.
        let p = stab_drive(0.6, anchor, aim, item, hand, 0.5).unwrap();
        let g = 0.4f32 * 0.4 * (3.0 - 0.2 * 2.0) * 0.2 * 4.0;
        // (The direction is normalised as the engine does, to 12 bits.)
        assert!((p.item.z - g * 1.3 * 48.0).abs() < 0.02, "{p:?}");
        assert!((p.hand.z - g * 1.3 * 0.16 * 12.0).abs() < 1e-3, "{p:?}");
        assert!((p.item.z - 20.77).abs() < 0.02 && (p.hand.z - 0.831).abs() < 0.001);
        // Past the clip's end the across ramp has fallen to nothing.
        let late = stab_drive(1.01, anchor, aim, item, hand, 0.5).unwrap();
        assert!(late.item.x.abs() < 0.05, "{late:?}");
    }
}
