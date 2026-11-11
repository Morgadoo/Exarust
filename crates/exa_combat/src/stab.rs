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
//! The drive runs from clock 0.1 while the stab has a held weapon and has
//! not landed a blow: the body tick's handling pushes
//! ([`crate::weapon::handling_pushes`]) move the weapon point 0.9 up the
//! first held item and the right hand across onto the line from the anchor
//! to the aim point, and past clock 0.4 along it.
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
}
