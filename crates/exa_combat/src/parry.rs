//! An unarmed parry, chosen from the blow that is already coming in, and
//! the rule that runs it in the character tick.
//!
//! A helpless fighter, a break-tangle motion, or yellow health under 0.01
//! parries nothing. Otherwise the attacker must be attacking, not cleaving,
//! in front, and within its reach plus 150 cm. Its clock must sit from 0.3
//! through 0.9, unless the swing is a continuation. An attacker of size 1.3
//! or more is refused unless the defender has a shield. An unarmed parry
//! then succeeds. Its speed is 1 and its blend is 20 frames. The clip is
//! the height table for the attack motion: left for motions 5 through 7,
//! right for 8 through 10, the down clip when the table height is at or
//! below the chest difference.
//!
//! [`run_parries`] places the rule where the original's body tick calls it:
//! after the swing routine, for the ticked fighter, from the combat idle
//! (muscle tone 0.85 or more, the auto-parry deadline passed, something in
//! the hands) and then on every tick the fighter parries. The armed parry
//! (its speed and grips from the weapon's handling) is not modelled, so a
//! fighter holding a weapon does not parry.
//!
//! Evidence: the unarmed parry decision, the height table, the body tick's
//! calls of the check, and the armed grip.

// The original compares floats so that a NaN takes the other branch; the
// negated compares here keep that.
#![allow(clippy::neg_cmp_op_on_partial_ord)]

use bevy::math::{Vec3, Vec4};
use bevy::prelude::*;
use exa_anim::{Animator, Ease, Motions};
use exa_physics::body::{BodyTicks, Ragdoll, TickedCharacter};
use exa_physics::engine_math;
use exa_physics::motile::auto_parry_deadline;
use std::f32::consts::PI;

use crate::attack::Attacker;
use crate::condition::Condition;
use crate::damage::Guard;
use crate::fighter::{Fighter, failing};
use crate::stance::{self, POSTURE_TONE, Stance};
use crate::swing::SwingState;
use crate::{Attacking, Blade};

/// Parry Left.
const LEFT: u8 = 0x14;
/// Parry Right.
const RIGHT: u8 = 0x15;
/// Parry Up.
const UP: u8 = 0x16;
/// Parry Left Dn.
const LEFT_DN: u8 = 0x17;
/// Parry Right Dn.
const RIGHT_DN: u8 = 0x18;
/// Attack motions whose height row is (low side, high side), in centimetres.
const HEIGHTS: [(u8, f32, f32); 6] = [
    (5, -20.0, -20.0),
    (6, 40.0, -80.0),
    (7, -60.0, 20.0),
    (8, -20.0, -20.0),
    (9, -20.0, 50.0),
    (10, 20.0, -60.0),
];

/// The blow a defender is looking at, before any parry has been chosen.
#[derive(Debug, Clone, Copy)]
pub struct Threat {
    pub attacking: bool,
    pub clock: f32,
    pub combo: bool,
    pub cleaving: bool,
    pub in_front: bool,
    pub distance_cm: f32,
    pub reach_cm: f32,
    pub size: f32,
    pub defender_has_shield: bool,
    pub yellow: f32,
    pub helpless: bool,
    pub break_tangle: bool,
    pub motion: u8,
    /// Yaw of the attacker's aim, minus the yaw of the defender's chest
    /// seen from the attacker.
    pub yaw: f32,
    pub defender_chest_cm: f32,
    pub attacker_chest_cm: f32,
    /// Stab when the motion is not one of 5 through 10.
    pub stab: bool,
    pub overhead: bool,
}

/// The parry clip and the blend it starts with.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Parry {
    pub clip: u8,
    pub blend: u32,
    pub speed: f32,
}

/// What an armed defender brings to the grip.
#[derive(Debug, Clone, Copy)]
pub struct Arm {
    pub handling: f32,
    pub horizontal_cm: f32,
    pub passed: bool,
    pub cleaving: bool,
    pub shield: bool,
    pub shield_weight: f32,
    pub stamina: f32,
    pub attacking: bool,
    pub motion: u8,
    pub held: u8,
    pub has_grip: bool,
    pub other_held: bool,
}

/// The grip a parry enables, and the speed the clip blends by.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Grip {
    pub speed: f32,
    pub blend: u32,
    pub strength: f32,
    pub slot: u8,
    pub flags: u32,
    pub enabled: bool,
    pub other_disabled: bool,
}

/// The armed parry's grip. A cleaving attacker is refused. A blow that has
/// passed sets the speed to the handling squared, or the shield weight times
/// stamina, and disables the grips. Otherwise the falloff is 1 inside 80 cm,
/// falls to 0 at 250 cm, and stays 0 past that (a NaN distance falls
/// through to the line and gives NaN); it is scaled by 0.75 at once. A
/// weapon's strength is (1.4 − handling) times that, then times 0.6, each
/// product in double and stored as a float, and its speed is the handling.
/// A shield's speed is 1 minus half the weight times stamina, and its
/// strength stamina × (weight × (falloff·0.75 × 0.4)) in double, with flags
/// 0x800. The off hand is slot 1 only while attacking with a motion outside
/// 8 through 10 and two weapons are held. The blend is 20 × the speed,
/// rounded half to even.
pub fn armed_grip(arm: &Arm) -> Option<Grip> {
    if arm.cleaving {
        return None;
    }
    let falloff = parry_falloff(arm.horizontal_cm) * 0.75_f32;
    let slot = grip_slot(arm);
    if arm.passed {
        let speed = if arm.shield {
            arm.shield_weight * arm.stamina
        } else {
            arm.handling * arm.handling
        };
        return Some(grip(speed, 0.0, slot, 0x10_0000, false, true));
    }
    if arm.shield {
        let speed = 1.0 - arm.shield_weight * arm.stamina * 0.5_f32;
        if !arm.has_grip {
            return Some(grip(speed, 0.0, slot, 0, false, false));
        }
        let strength = (f64::from(arm.stamina)
            * (f64::from(arm.shield_weight) * (f64::from(falloff) * 0.4)))
            as f32;
        return Some(grip(speed, strength, slot, 0x800, true, arm.other_held));
    }
    let speed = arm.handling;
    if !arm.has_grip {
        return Some(grip(speed, 0.0, slot, 0, false, false));
    }
    let gain = ((1.4 - f64::from(arm.handling)) * f64::from(falloff)) as f32;
    let strength = (f64::from(gain) * 0.6) as f32;
    Some(grip(speed, strength, slot, 0, true, arm.other_held))
}

fn grip(
    speed: f32,
    strength: f32,
    slot: u8,
    flags: u32,
    enabled: bool,
    other_disabled: bool,
) -> Grip {
    Grip {
        speed,
        blend: engine_math::round_to_i64(20.0 * speed) as u32,
        strength,
        slot,
        flags,
        enabled,
        other_disabled,
    }
}

fn parry_falloff(distance: f32) -> f32 {
    if distance < 80.0 {
        1.0
    } else if distance > 250.0 {
        0.0
    } else {
        1.0 - (distance - 80.0) * (1.0_f32 / 170.0)
    }
}

fn grip_slot(arm: &Arm) -> u8 {
    let slot = if arm.attacking && !(8..=10).contains(&arm.motion) {
        1
    } else {
        0
    };
    if arm.held < slot + 1 { 0 } else { slot }
}

/// The attack motions the armed parry's travel is not scaled for: the stab
/// and the overhead.
const STAB_MOTION: i32 = 0x0B;
/// The overhead, whose parry point sits 80 cm back instead of 60.
const OVERHEAD_MOTION: i32 = 0x0C;

/// How far up the attacker's blade (cm from its origin, along its local y)
/// the armed parry meets it. The gap is the distance between the two chest
/// points less 20 cm. Beyond the attacker's reach the meeting point is the
/// blade length less 20; within it, the blade length less the part of the
/// reach not used and less 20, floored at 0 (a NaN gap takes this branch,
/// and a NaN height is kept).
pub fn parry_blade_height(
    defender_chest: Vec3,
    attacker_chest: Vec3,
    reach_cm: f32,
    blade_length_cm: f32,
) -> f32 {
    let gap = engine_math::length3(defender_chest - attacker_chest) - 20.0;
    if gap > reach_cm {
        return blade_length_cm - 20.0;
    }
    let height = blade_length_cm - (reach_cm - gap) - 20.0;
    if height < 0.0 { 0.0 } else { height }
}

/// The blade's point at `height` on its local y axis, now and a tick ago:
/// its barycentric weights in the blade's rest tetrahedron (the fast
/// variant, with the approximate reciprocal) applied to the tetrahedron's
/// corners now and before.
pub fn blade_point(
    height: f32,
    rest: &[[f32; 3]; 4],
    now: &[[f32; 3]; 4],
    before: &[[f32; 3]; 4],
) -> (Vec3, Vec3) {
    let weights = Vec4::from_array(engine_math::barycentric_fast(
        Vec3::new(0.0, height, 0.0),
        rest,
    ));
    (
        engine_math::vec4_mul_4x3(weights, now),
        engine_math::vec4_mul_4x3(weights, before),
    )
}

/// Where the armed parry takes the blade, and whether the blade has
/// already passed the defender.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ParryPoint {
    pub point: Vec3,
    pub passed: bool,
}

/// The armed parry's point. The blade point's travel this tick (now less
/// before) has its height scaled by 16 − 16·clock unless the motion is a
/// stab or an overhead; the travel less its part along the (approximately
/// normalised) direction from the attacker's chest to the blade point,
/// normalised the same way, is the sweep direction. When the blade point
/// lies further along it than the defender's chest the blow has passed and
/// the point is 10 cm on along it. Otherwise the point is moved along it by
/// the chest's lead less 60 cm (80 for an overhead), floored at 0 (a NaN
/// lead is kept). Dots sum x, then y, then z.
pub fn parry_point(
    blade_now: Vec3,
    blade_before: Vec3,
    attacker_chest: Vec3,
    defender_chest: Vec3,
    motion: i32,
    progress: f32,
) -> ParryPoint {
    let mut travel = blade_now - blade_before;
    if motion != OVERHEAD_MOTION && motion != STAB_MOTION {
        travel.y *= 16.0 - progress * 16.0;
    }
    let toward = engine_math::normalize_approx(blade_now - attacker_chest);
    let sweep = engine_math::normalize_approx(engine_math::remove_component(travel, toward));
    let dot = |v: Vec3| (v.x * sweep.x + v.y * sweep.y) + v.z * sweep.z;
    let blade_side = dot(blade_now);
    let chest_side = dot(defender_chest);
    if blade_side > chest_side {
        return ParryPoint {
            point: blade_now + sweep * 10.0,
            passed: true,
        };
    }
    let back = if motion == OVERHEAD_MOTION {
        80.0
    } else {
        60.0
    };
    let mut lead = (chest_side - blade_side) - back;
    if lead < 0.0 {
        lead = 0.0;
    }
    ParryPoint {
        point: blade_now + sweep * lead,
        passed: false,
    }
}

/// The parry point's distance from the defender's chest across the ground
/// (the height difference zeroed, then the length summed x and z first), the
/// distance the grip's falloff reads.
pub fn parry_distance(point: Vec3, defender_chest: Vec3) -> f32 {
    let mut d = point - defender_chest;
    d.y = 0.0;
    engine_math::length3(d)
}

/// The unarmed parry for `threat`, or nothing when the blow is not one an
/// unarmed fighter answers.
pub fn unarmed_parry(threat: &Threat) -> Option<Parry> {
    if threat.helpless || threat.break_tangle || threat.yellow < 0.01 {
        return None;
    }
    let in_window = threat.combo || (0.3..=0.9).contains(&threat.clock);
    let size_ok = threat.size < 1.3 || threat.defender_has_shield;
    let in_reach = threat.distance_cm <= threat.reach_cm + 150.0;
    if !threat.attacking
        || threat.cleaving
        || !threat.in_front
        || !in_window
        || !size_ok
        || !in_reach
    {
        return None;
    }
    let speed = 1.0_f32;
    Some(Parry {
        clip: clip_for(threat),
        blend: (20.0 * speed).round() as u32,
        speed,
    })
}

fn clip_for(threat: &Threat) -> u8 {
    let theta = wrap_pi(threat.yaw);
    let s = theta / PI + 0.5;
    if let Some((_, low, high)) = HEIGHTS
        .iter()
        .find(|(motion, _, _)| *motion == threat.motion)
    {
        let height = if s < 0.0 {
            *low
        } else if s > 1.0 {
            *high
        } else {
            *high * s + *low * (1.0 - s)
        };
        let chests = threat.defender_chest_cm - threat.attacker_chest_cm;
        let down = height <= chests;
        return if threat.motion <= 7 {
            if down { LEFT_DN } else { LEFT }
        } else if down {
            RIGHT_DN
        } else {
            RIGHT
        };
    }
    if threat.overhead {
        return UP;
    }
    if threat.stab {
        return if s >= 0.0 { RIGHT } else { LEFT };
    }
    LEFT
}

fn wrap_pi(yaw: f32) -> f32 {
    let turn = yaw.rem_euclid(PI * 2.0);
    if turn > PI { turn - PI * 2.0 } else { turn }
}

/// A parry in progress on the defender: the original's parrying flag, the
/// chosen parry clip (0x14–0x18), the attack motion the clip was picked for
/// and the parry counter.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ParryState {
    /// It is parrying (the original's parrying flag, zero on a fresh body).
    pub active: bool,
    /// The chosen parry clip 0x14–0x18, none before the first pick.
    pub clip: Option<u8>,
    /// The attack motion the clip was picked for, as the original's motion
    /// number, or none.
    pub counter_motion: Option<usize>,
    /// The counter: Left and Right rise to 20, Up falls from above 0 and
    /// rises from below, the downs fall while over 20.
    pub counter: i32,
    /// The game clock (ms) of the last tick a check kept the parry up, none
    /// before the first.
    pub last_ms: Option<u64>,
}

impl ParryState {
    /// Ends the parry: it counters no motion and is no longer parrying.
    /// The original also disables the grip links of every held item and
    /// relaxes both hands; Exarust has no grip links to disable yet.
    pub fn end(&mut self) {
        self.active = false;
        self.clip = None;
        self.counter_motion = None;
    }
}

/// The blend (frames) a parry ends into its stance with, eased smoothly.
pub const END_BLEND: u32 = 0x28;

/// The clip (an action of the fighter's moveset) of a parry code, 0x14
/// through 0x18.
pub fn clip_action(clip: u8) -> Option<&'static str> {
    Some(match clip {
        LEFT => "Parry Left",
        RIGHT => "Parry Right",
        UP => "Parry Up",
        LEFT_DN => "Parry Left Dn",
        RIGHT_DN => "Parry Rgt Dn",
        _ => return None,
    })
}

/// Whether `action` is one of the parry clips.
pub fn is_parry_action(action: &str) -> bool {
    matches!(
        action,
        "Parry Left" | "Parry Right" | "Parry Up" | "Parry Left Dn" | "Parry Rgt Dn"
    )
}

/// The branch a weapon's clash takes on the victim.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Clash {
    /// The victim parries: the attacker recoils, the victim is kept from
    /// attacking 600 ms unless it has Riposte, and its stance sags.
    Parrying,
    /// The victim counters this very motion with a weapon that is not its
    /// primary: the attacker recoils and the parry ends into its stance,
    /// with no lock and no sag.
    Countering,
    /// Anything else: neither side changes.
    Plain,
}

/// Which branch of a clash the victim takes: parrying first, then a counter
/// of the attacker's own motion by a weapon other than the victim's
/// primary, then nothing.
pub fn clash_branch(parry: ParryState, attacker_motion: usize, primary_is_struck: bool) -> Clash {
    if parry.active {
        Clash::Parrying
    } else if parry.counter_motion == Some(attacker_motion) && !primary_is_struck {
        Clash::Countering
    } else {
        Clash::Plain
    }
}

#[cfg(test)]
mod clash_tests {
    use super::*;

    #[test]
    fn the_clash_branches() {
        let parrying = ParryState {
            active: true,
            counter_motion: Some(5),
            ..ParryState::default()
        };
        assert_eq!(clash_branch(parrying, 5, false), Clash::Parrying);
        let countering = ParryState {
            active: false,
            counter_motion: Some(5),
            ..ParryState::default()
        };
        assert_eq!(clash_branch(countering, 5, false), Clash::Countering);
        assert_eq!(clash_branch(countering, 6, false), Clash::Plain);
        assert_eq!(clash_branch(countering, 5, true), Clash::Plain);
        // A fresh body is not parrying.
        assert_eq!(clash_branch(ParryState::default(), 5, false), Clash::Plain);
    }

    #[test]
    fn a_parry_ends() {
        let mut parry = ParryState {
            active: true,
            counter_motion: Some(8),
            ..ParryState::default()
        };
        parry.end();
        assert_eq!(
            parry,
            ParryState {
                active: false,
                counter_motion: None,
                ..ParryState::default()
            }
        );
        assert_eq!(clash_branch(parry, 8, false), Clash::Plain);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mid_swing() -> Threat {
        Threat {
            attacking: true,
            clock: 0.5,
            combo: false,
            cleaving: false,
            in_front: true,
            distance_cm: 100.0,
            reach_cm: 80.0,
            size: 1.0,
            defender_has_shield: false,
            yellow: 0.25,
            helpless: false,
            break_tangle: false,
            motion: 5,
            yaw: 0.0,
            defender_chest_cm: 140.0,
            attacker_chest_cm: 140.0,
            stab: false,
            overhead: false,
        }
    }

    #[test]
    fn an_unarmed_fighter_parries_a_swing_that_is_already_coming() {
        let parry = unarmed_parry(&mid_swing()).unwrap();
        assert_eq!(parry.clip, LEFT_DN);
        assert_eq!(parry.blend, 20);
        assert_eq!(parry.speed, 1.0);
        let mut early = mid_swing();
        early.clock = 0.2;
        assert!(unarmed_parry(&early).is_none());
        early.combo = true;
        assert!(unarmed_parry(&early).is_some());
        let mut low = mid_swing();
        low.yellow = 0.009;
        assert!(unarmed_parry(&low).is_none());
        let mut big = mid_swing();
        big.size = 1.3;
        assert!(unarmed_parry(&big).is_none());
        big.defender_has_shield = true;
        assert!(unarmed_parry(&big).is_some());
        let mut behind = mid_swing();
        behind.in_front = false;
        assert!(unarmed_parry(&behind).is_none());
        let mut far = mid_swing();
        far.distance_cm = far.reach_cm + 151.0;
        assert!(unarmed_parry(&far).is_none());
        let mut down = mid_swing();
        down.motion = 6;
        down.defender_chest_cm = 190.0;
        assert_eq!(unarmed_parry(&down).unwrap().clip, LEFT_DN);
        down.defender_chest_cm = 100.0;
        assert_eq!(unarmed_parry(&down).unwrap().clip, LEFT);
    }

    fn arm() -> Arm {
        Arm {
            handling: 0.5,
            horizontal_cm: 40.0,
            passed: false,
            cleaving: false,
            shield: false,
            shield_weight: 0.8,
            stamina: 0.5,
            attacking: false,
            motion: 5,
            held: 1,
            has_grip: true,
            other_held: false,
        }
    }

    #[test]
    fn an_armed_parry_sets_the_grip() {
        assert!(
            armed_grip(&Arm {
                cleaving: true,
                ..arm()
            })
            .is_none()
        );
        let near = armed_grip(&arm()).unwrap();
        assert_eq!(near.speed.to_bits(), 0x3f00_0000);
        assert_eq!(near.blend, 10);
        assert_eq!(near.strength.to_bits(), 0x3ecf_5c29);
        assert_eq!(near.slot, 0);
        assert_eq!(near.flags, 0);
        assert!(near.enabled);
        assert!(!near.other_disabled);
        let mid = armed_grip(&Arm {
            horizontal_cm: 165.0,
            ..arm()
        })
        .unwrap();
        assert_eq!(mid.strength.to_bits(), 0x3e4f_5c29);
        let edge = armed_grip(&Arm {
            horizontal_cm: 250.0,
            ..arm()
        })
        .unwrap();
        assert_eq!(edge.strength.to_bits(), 0);
        let past = armed_grip(&Arm {
            horizontal_cm: 251.0,
            ..arm()
        })
        .unwrap();
        assert_eq!(past.strength.to_bits(), 0);
        let passed = armed_grip(&Arm {
            passed: true,
            ..arm()
        })
        .unwrap();
        assert_eq!(passed.speed.to_bits(), 0x3e80_0000);
        assert_eq!(passed.blend, 5);
        assert_eq!(passed.flags, 0x10_0000);
        assert!(!passed.enabled);
        assert!(passed.other_disabled);
        let shield = armed_grip(&Arm {
            shield: true,
            other_held: true,
            ..arm()
        })
        .unwrap();
        assert_eq!(shield.speed.to_bits(), 0x3f4c_cccd);
        assert_eq!(shield.blend, 16);
        assert_eq!(shield.strength.to_bits(), 0x3df5_c290);
        assert_eq!(shield.flags, 0x800);
        assert!(shield.other_disabled);
        let off = armed_grip(&Arm {
            attacking: true,
            held: 2,
            other_held: true,
            ..arm()
        })
        .unwrap();
        assert_eq!(off.slot, 1);
        let combo = armed_grip(&Arm {
            attacking: true,
            motion: 9,
            held: 2,
            ..arm()
        })
        .unwrap();
        assert_eq!(combo.slot, 0);
        let one = armed_grip(&Arm {
            attacking: true,
            held: 1,
            ..arm()
        })
        .unwrap();
        assert_eq!(one.slot, 0);
        // The weapon's strength takes its products in double: a float chain
        // would give 0x3edf_9f9f here.
        let doubled = armed_grip(&Arm {
            handling: 0.3,
            horizontal_cm: 100.0,
            ..arm()
        })
        .unwrap();
        assert_eq!(doubled.strength.to_bits(), 0x3edf_9fa0);
        let shield_doubled = armed_grip(&Arm {
            shield: true,
            stamina: 0.3,
            shield_weight: 0.7,
            horizontal_cm: 100.0,
            ..arm()
        })
        .unwrap();
        assert_eq!(shield_doubled.strength.to_bits(), 0x3d63_b07e);
        // 20 × 0.625 is a half: the blend rounds to even.
        let half = armed_grip(&Arm {
            handling: 0.625,
            ..arm()
        })
        .unwrap();
        assert_eq!(half.blend, 12);
        // A NaN distance falls through to the line.
        let lost = armed_grip(&Arm {
            horizontal_cm: f32::NAN,
            ..arm()
        })
        .unwrap();
        assert!(lost.strength.is_nan());
    }

    fn near(a: Vec3, b: Vec3, tolerance: f32) -> bool {
        (a - b).abs().max_element() < tolerance
    }

    #[test]
    fn the_blade_height_follows_the_reach() {
        let at = |z: f32| parry_blade_height(Vec3::new(0.0, 0.0, z), Vec3::ZERO, 100.0, 90.0);
        assert_eq!(at(200.0), 70.0);
        assert_eq!(at(100.0), 50.0);
        assert_eq!(at(30.0), 0.0);
        let lost = parry_blade_height(Vec3::splat(f32::NAN), Vec3::ZERO, 100.0, 90.0);
        assert!(lost.is_nan());
    }

    #[test]
    fn the_blade_point_is_carried_by_the_tetrahedron() {
        let rest = [
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 1.0],
        ];
        let now = rest.map(|[x, y, z]| [x + 5.0, y, z]);
        let (at, was) = blade_point(0.5, &rest, &now, &rest);
        assert!(near(at, Vec3::new(5.0, 0.5, 0.0), 1e-2));
        assert!(near(was, Vec3::new(0.0, 0.5, 0.0), 1e-2));
    }

    #[test]
    fn the_parry_point_waits_ahead_of_the_chest() {
        let blade = Vec3::new(-50.0, 0.0, 0.0);
        let before = blade - Vec3::new(10.0, 0.0, 20.0);
        let ahead = parry_point(
            blade,
            before,
            Vec3::ZERO,
            Vec3::new(0.0, 0.0, 100.0),
            0x0B,
            0.5,
        );
        assert!(!ahead.passed);
        assert!(near(ahead.point, Vec3::new(-50.0, 0.0, 40.0), 0.1));
        let overhead = parry_point(
            blade,
            before,
            Vec3::ZERO,
            Vec3::new(0.0, 0.0, 100.0),
            0x0C,
            0.5,
        );
        assert!(near(overhead.point, Vec3::new(-50.0, 0.0, 20.0), 0.1));
        let close = parry_point(
            blade,
            before,
            Vec3::ZERO,
            Vec3::new(0.0, 0.0, 30.0),
            0x0B,
            0.5,
        );
        assert!(!close.passed);
        assert!(near(close.point, blade, 1e-3));
        let gone = parry_point(
            blade,
            before,
            Vec3::ZERO,
            Vec3::new(0.0, 0.0, -5.0),
            0x0B,
            0.5,
        );
        assert!(gone.passed);
        assert!(near(gone.point, Vec3::new(-50.0, 0.0, 10.0), 0.1));
        // A side swing's travel height is scaled by 16 − 16·0.75 = 4.
        let rising = blade - Vec3::new(0.0, 1.0, 1.0);
        let side = parry_point(
            blade,
            rising,
            Vec3::ZERO,
            Vec3::new(0.0, -100.0, -100.0),
            5,
            0.75,
        );
        assert!(side.passed);
        let sweep = Vec3::new(0.0, 4.0, 1.0) / 17.0_f32.sqrt();
        assert!(near(side.point, blade + sweep * 10.0, 0.1));
        assert_eq!(
            parry_distance(Vec3::new(30.0, 50.0, 40.0), Vec3::ZERO),
            50.0
        );
    }
}

/// What stops a parry before the attacker is looked at: a helpless
/// condition, the break-tangle motion (0x1C), or yellow health under 0.01
/// (compared in double, so a yellow of `0.01_f32` is under it).
pub fn parry_ends(condition_helpless: bool, motion: u8, yellow: f32) -> bool {
    condition_helpless || motion == 0x1C || f64::from(yellow) < 0.01
}

/// The attacker as the parry trigger reads it.
#[derive(Debug, Clone, Copy)]
pub struct Trigger {
    /// The attacker's condition is exactly attacking.
    pub attacker_attacking: bool,
    pub attacker_size: f32,
    /// The defender holds a shield.
    pub defender_has_shield: bool,
    pub strike_window: bool,
    pub cleaving: bool,
    pub progress: f32,
    pub reach_cm: f32,
    /// Attacker's chest less the defender's chest (cm, world).
    pub chest_delta: Vec3,
    /// The defender's forward across the ground (unit).
    pub facing: Vec3,
}

/// The parry trigger's attacker test (the routine's gate): the attacker is
/// attacking and either smaller than 1.3 (in double) or facing a defender
/// with a shield; unless its strike window is open, its clip is from 0.3
/// through 0.9; it is not cleaving; it is not behind (the chest delta's dot
/// with the facing is not negative) and within its reach plus 150 cm. NaN
/// never rules a blow out of the compares.
pub fn parry_trigger(t: &Trigger) -> bool {
    if !t.attacker_attacking || !(f64::from(t.attacker_size) < 1.3 || t.defender_has_shield) {
        return false;
    }
    let progress = f64::from(t.progress);
    if !t.strike_window && (progress < 0.3 || progress > 0.9) {
        return false;
    }
    if t.cleaving {
        return false;
    }
    let d = t.chest_delta;
    let f = t.facing;
    // The original sums the z product onto the sum of the y and x ones.
    if d.z * f.z + (d.y * f.y + d.x * f.x) < 0.0 {
        return false;
    }
    let reach = t.reach_cm + 150.0;
    let length_squared = d.z * d.z + (d.y * d.y + d.x * d.x);
    !(reach * reach < length_squared)
}

/// The parry counter after choosing `clip` (0x14 Left, 0x15 Right,
/// 0x16 Up, 0x17 Left Dn, 0x18 Right Dn): Left and Right rise to 20; Up
/// falls from above 0 to it and rises from below; the two downs fall while
/// over 20. The original passes the last clip it chose, every tick, even
/// when this tick chose none.
pub fn parry_counter(counter: i32, clip: u8) -> i32 {
    match clip {
        0x14 | 0x15 if counter < 20 => counter + 1,
        0x16 if counter >= 1 => counter - 1,
        0x16 if counter < 0 => counter + 1,
        0x17 | 0x18 if counter > 20 => counter - 1,
        _ => counter,
    }
}

/// What a successful parry starts: the clip, its blend (round(recovery·20))
/// and the ease (2), and the attack motion the body is left in.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ParryStart {
    pub clip: u8,
    pub blend: u32,
    pub ease: u32,
    pub attack_motion: u8,
}

/// The parry start when the chosen clip differs from the one playing:
/// `chosen` is the re-picked clip from the height table (none below 0),
/// `current` the clip playing, `recovery` the body's recovery state and
/// `attack_motion` the motion it is attacking with, which a start moves
/// from an attack (5–12) to 4.
pub fn parry_start(
    chosen: i32,
    current: i32,
    recovery: f32,
    attack_motion: u8,
) -> Option<ParryStart> {
    if chosen < 0 || chosen == current {
        return None;
    }
    Some(ParryStart {
        clip: chosen as u8,
        blend: (recovery * 20.0).round_ties_even() as u32,
        ease: 2,
        attack_motion: if (5..=12).contains(&attack_motion) {
            4
        } else {
            attack_motion
        },
    })
}

#[cfg(test)]
mod trigger_tests {
    use super::*;

    fn attacker() -> Trigger {
        Trigger {
            attacker_attacking: true,
            attacker_size: 1.0,
            defender_has_shield: false,
            strike_window: true,
            cleaving: false,
            progress: 0.5,
            reach_cm: 100.0,
            chest_delta: Vec3::Z * 100.0,
            facing: Vec3::Z,
        }
    }

    /// The trigger's gates: helpless, break tangle and yellow under 0.01
    /// end it; the attacker must be attacking (or big against no shield),
    /// in its window, not cleaving, in front and within reach + 150 cm.
    #[test]
    fn the_parry_trigger_is_the_originals() {
        assert!(parry_ends(true, 5, 1.0));
        assert!(parry_ends(false, 0x1C, 1.0));
        assert!(parry_ends(false, 5, 0.0099));
        // The compare is in double: 0.01 as a float is just under it.
        assert!(parry_ends(false, 5, 0.01));
        assert!(!parry_ends(false, 5, 0.0100001));
        assert!(parry_trigger(&attacker()));
        let mut t = attacker();
        t.attacker_attacking = false;
        assert!(!parry_trigger(&t));
        t.attacker_attacking = true;
        // Size 1.3 as a float is under 1.3 in double; the next float is not
        // unless the defender holds a shield.
        t.attacker_size = 1.3;
        assert!(parry_trigger(&t));
        t.attacker_size = 1.300_000_1;
        assert!(!parry_trigger(&t));
        t.defender_has_shield = true;
        assert!(parry_trigger(&t));
        let mut t = attacker();
        t.strike_window = false;
        t.progress = 0.3;
        assert!(parry_trigger(&t));
        t.progress = 0.9;
        assert!(parry_trigger(&t));
        t.progress = 0.29;
        assert!(!parry_trigger(&t));
        t.progress = 0.91;
        assert!(!parry_trigger(&t));
        t.progress = f32::NAN;
        assert!(parry_trigger(&t));
        let mut t = attacker();
        t.cleaving = true;
        assert!(!parry_trigger(&t));
        let mut t = attacker();
        t.chest_delta = Vec3::NEG_Z * 100.0;
        assert!(!parry_trigger(&t));
        // Level with the defender (a zero dot) is not behind it.
        t.chest_delta = Vec3::X * 100.0;
        assert!(parry_trigger(&t));
        let mut t = attacker();
        t.chest_delta = Vec3::Z * 250.0;
        assert!(parry_trigger(&t));
        t.chest_delta = Vec3::Z * 250.01;
        assert!(!parry_trigger(&t));
    }

    /// The counter rises to 20 on the sides, falls from above 0 on Up and
    /// falls while over 20 on the downs; a start needs a new clip (0 or above)
    /// and blends round(recovery·20) with ease 2.
    #[test]
    fn the_parry_counter_and_start_are_the_originals() {
        assert_eq!(parry_counter(3, 0x14), 4);
        assert_eq!(parry_counter(20, 0x15), 20);
        assert_eq!(parry_counter(3, 0x16), 2);
        assert_eq!(parry_counter(-1, 0x16), 0);
        assert_eq!(parry_counter(22, 0x17), 21);
        assert_eq!(parry_counter(20, 0x18), 20);
        let start = parry_start(0x15, 0x14, 1.05, 6).unwrap();
        assert_eq!(
            start,
            ParryStart {
                clip: 0x15,
                blend: 21,
                ease: 2,
                attack_motion: 4,
            }
        );
        // 20 · 0.025 is a half: it rounds to even.
        assert_eq!(parry_start(0x15, 0x14, 0.025, 3).unwrap().blend, 0);
        assert_eq!(
            parry_start(0x15, 0x14, 0.075, 13).unwrap().attack_motion,
            13
        );
        // Clip 0 is a start too.
        assert!(parry_start(0, 0x14, 1.0, 0).is_some());
        assert!(parry_start(0x14, 0x14, 1.0, 0).is_none());
        assert!(parry_start(-1, 0x14, 1.0, 0).is_none());
    }
}

/// Whether the parry clip is picked again (the clash bookkeeping): when the
/// clip playing is outside the parry clips 0x14–0x18, or the attack motion
/// differs from the countered motion `f5c`.
pub fn parry_repick(current_clip: i32, current_motion: i32, attack_motion: i32) -> bool {
    !(0x14..=0x18).contains(&current_clip) || attack_motion != current_motion
}

#[cfg(test)]
mod repick_tests {
    use super::*;

    #[test]
    fn the_parry_is_repicked_for_a_new_motion() {
        assert!(!parry_repick(0x14, 5, 5));
        assert!(parry_repick(0x14, 5, 6));
        assert!(parry_repick(0x13, 5, 5));
        assert!(parry_repick(0x19, 5, 5));
    }
}

/// The stance a parry ends into: the Defence stance for a body dying (yellow
/// health under 0.01, in double precision) or dazed (its energy more than
/// 0.05 under its most), else the combat idle.
pub fn parry_stance(yellow: f32, dazed: bool) -> Stance {
    if failing(yellow) || dazed {
        Stance::Defence
    } else {
        Stance::CombatIdle
    }
}

/// The blow a parry answers: the attacker's motion (the counter's), and the
/// parry clip the height table gives for it.
#[derive(Clone, Copy, Debug)]
struct Pick {
    motion: u8,
    clip: u8,
}

/// What the defender holds: whether it may parry at all (it holds something,
/// a fist included) and whether what it holds is a weapon, whose parry (the
/// grip, its speed from the weapon's handling) is not modelled yet.
#[derive(Clone, Copy, Debug, Default)]
struct Held {
    allowed: bool,
    armed: bool,
}

fn held(wielder: Entity, blades: &Query<&Blade>) -> Held {
    let mut held = Held::default();
    for blade in blades.iter().filter(|b| b.wielder == wielder) {
        held.allowed = true;
        held.armed |= blade.record.armed();
    }
    held
}

/// A fighter's reach in centimetres, as the equip refresh sets it: its
/// longest held weapon's record reach, else its race's unarmed reach.
fn reach_cm(wielder: Entity, blades: &Query<&Blade>) -> f32 {
    blades
        .iter()
        .filter(|b| b.wielder == wielder && b.record.armed())
        .map(|b| b.record.reach_metres() * 100.0)
        .fold(f32::NEG_INFINITY, f32::max)
        .max(stance::UNARMED_REACH * 100.0)
}

/// The ground-plane yaw of `v` (0 along +Z, positive toward +X).
fn yaw(v: Vec3) -> f32 {
    v.x.atan2(v.z)
}

/// The body's forward across the ground.
fn forward(gt: &GlobalTransform) -> Vec3 {
    (gt.rotation() * Vec3::Z).with_y(0.0).normalize_or_zero()
}

/// Whether the combat idle's auto-parry deadline has passed: 400 ms after
/// the last attack tick, or after the recoil's stamp when the swing was
/// recoiled ([`auto_parry_deadline`], in 32-bit arithmetic, the deadline
/// signed and the clock not).
fn deadline_passed(swing: &SwingState, attacker: &Attacker, now: u64) -> bool {
    let deadline = auto_parry_deadline(
        swing.flags,
        0,
        swing.end_ms as u32,
        attacker.last_tick_ms.unwrap_or(0) as u32,
    );
    i64::from(deadline) < i64::from(now as u32)
}

/// The muscle tone the combat idle's auto-parry needs (the posture switch's).
fn toned(body: Option<&Ragdoll>) -> bool {
    match body {
        None => true,
        Some(b) => b.drive.as_ref().is_some_and(|d| d.tone >= POSTURE_TONE),
    }
}

/// The parry guard follows the parry: raised while it parries or still
/// counters a motion, never over a shield's.
fn sync_guard(fighter: &mut Fighter) {
    let up = fighter.parry.active || fighter.parry.counter_motion.is_some();
    match fighter.guard {
        Guard::Open if up => fighter.guard = Guard::Parry,
        Guard::Parry if !up => fighter.guard = Guard::Open,
        _ => {}
    }
}

/// The defender's side of a parry check.
struct Defender<'a> {
    fighter: &'a mut Fighter,
    anim: &'a mut Animator,
    stance: &'a mut Stance,
    condition: Condition,
    dazed: bool,
    /// The motion number of the clip it plays.
    motion: u8,
    now: u64,
    held: Held,
}

impl Defender<'_> {
    /// The parry's clear: it counters no motion and is no longer parrying;
    /// a parry that was on returns to its stance (the Defence stance when
    /// dying or dazed, else the combat idle) with a blend of 40 frames
    /// eased smoothly, unless the condition's clip stands still.
    fn clear(&mut self) {
        let was = self.fighter.parry.active;
        self.fighter.parry.end();
        sync_guard(self.fighter);
        if was && !self.condition.clip_stands_still() {
            self.restart_stance();
        }
    }

    fn restart_stance(&mut self) {
        let new = parry_stance(self.fighter.health.yellow, self.dazed);
        *self.stance = new;
        if self.anim.action != new.action() {
            self.anim
                .start(new.action(), true, None, END_BLEND, Ease::Smooth);
        }
    }

    /// One parry check against the blow `pick` (none: no foe's blow
    /// passes the trigger). Returns whether it started a parry from an
    /// attack, which the body leaves.
    fn check(&mut self, pick: Option<Pick>) -> bool {
        let f = &*self.fighter;
        if parry_ends(self.condition.helpless(), self.motion, f.health.yellow) {
            self.clear();
            return false;
        }
        if !self.held.allowed {
            // Not allowed to parry: only the flag goes.
            self.fighter.parry.active = false;
            sync_guard(self.fighter);
            return false;
        }
        let Some(pick) = pick.filter(|_| !self.held.armed) else {
            self.clear();
            return false;
        };
        let parry = &mut self.fighter.parry;
        parry.active = true;
        parry.last_ms = Some(self.now);
        let repick = parry_repick(
            parry.clip.map_or(-1, i32::from),
            parry.counter_motion.map_or(-1, |m| m as i32),
            i32::from(pick.motion),
        );
        // The clip stored for the counter: the one just picked, else the
        // one it keeps.
        let mut chosen = -1;
        if repick {
            parry.counter_motion = Some(usize::from(pick.motion));
            chosen = i32::from(pick.clip);
        }
        if let Some(stored) = if repick { Some(pick.clip) } else { parry.clip } {
            parry.counter = parry_counter(parry.counter, stored);
        }
        let start = parry_start(chosen, parry.clip.map_or(-1, i32::from), 1.0, self.motion);
        let mut left_attack = false;
        if let Some(start) = start
            && let Some(action) = clip_action(start.clip)
        {
            parry.clip = Some(start.clip);
            self.anim.start(action, true, None, start.blend, Ease::Fast);
            left_attack = start.attack_motion != self.motion;
        }
        sync_guard(self.fighter);
        left_attack
    }

    /// A body that is not parrying: the counter drifts back to 0, and a
    /// parry clip left playing (the parry ended by a blow, in the hit path)
    /// returns to the stance.
    fn rest(&mut self) {
        let counter = self.fighter.parry.counter;
        if counter != 0 {
            self.fighter.parry.counter = counter - counter.signum();
        }
        if self.anim.looping && is_parry_action(&self.anim.action) {
            if !self.condition.clip_stands_still() {
                self.restart_stance();
            }
            sync_guard(self.fighter);
        } else if matches!(self.fighter.guard, Guard::Parry)
            && !self.fighter.parry.active
            && self.fighter.parry.counter_motion.is_none()
        {
            self.fighter.guard = Guard::Open;
        }
    }
}

/// The parry rule on every living fighter, in its attack stage after the
/// swing routine, as the original's body tick runs it:
///
/// - a fighter in the combat idle, not parrying, muscles toned, whose
///   auto-parry deadline has passed and that may parry, checks the blow
///   coming at it;
/// - a fighter that parries checks it on every tick of its own, right after,
///   so a parry that starts takes its counter twice that tick;
/// - one that does not parry lets its counter drift back to 0.
///
/// A check ends the parry for a helpless condition, the break-tangle clip or
/// yellow under 0.01 (into its stance), and for a blow that fails the
/// trigger. The attacker is the nearest foe attacking with a swing the
/// trigger accepts (the original keeps an explicit target; the player's is
/// the cursor's, which Exarust does not keep as a component). A body that
/// does not fight, or has no foe attacking, does no more than read its own
/// state.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub(crate) fn run_parries(
    mut commands: Commands,
    ticked: Res<TickedCharacter>,
    ticks: Option<Res<BodyTicks>>,
    motions: Option<Res<Motions>>,
    blades: Query<&Blade>,
    mut bodies: ParamSet<(
        Query<(
            &mut Fighter,
            &mut Animator,
            &SwingState,
            &mut Stance,
            &Condition,
            &Attacker,
            Option<&Ragdoll>,
        )>,
        Query<(
            Entity,
            &Condition,
            &Fighter,
            &Animator,
            &SwingState,
            Option<&Attacking>,
            &GlobalTransform,
            Option<&Ragdoll>,
        )>,
    )>,
    shields: Query<(), With<crate::block::ShieldHold>>,
) {
    let Some(motions) = motions else { return };
    let now = ticks.as_ref().map_or(0, |t| t.millis());
    let entity = ticked.body;
    // Whether the body checks a blow this tick: it parries, or it stands in
    // the combat idle ready to. `rests` is a body that only lets its counter
    // drift or its stance return.
    let (parries, enters, rests, held) = {
        let own = bodies.p0();
        let Ok((fighter, anim, swing, _, _, attacker, body)) = own.get(entity) else {
            return;
        };
        let parries = fighter.parry.active;
        let ready = !parries
            && anim.looping
            && anim.action == Stance::CombatIdle.action()
            && toned(body)
            && deadline_passed(swing, attacker, now);
        let held = if parries || ready {
            held(entity, &blades)
        } else {
            Held::default()
        };
        let rests = !parries
            && (fighter.parry.counter != 0
                || is_parry_action(&anim.action)
                || matches!(fighter.guard, Guard::Parry));
        (parries, ready && held.allowed, rests, held)
    };
    if !(parries || enters || rests) {
        return;
    }
    let mut pick = None;
    if parries || enters {
        let (def_chest, def_facing, shield, yellow, helpless, motion) = {
            let own = bodies.p1();
            let Ok((_, condition, fighter, anim, _, _, gt, body)) = own.get(entity) else {
                return;
            };
            (
                stance::chest_point(gt, body),
                forward(gt),
                matches!(fighter.guard, Guard::Shield(_)) || shields.contains(entity),
                fighter.health.yellow,
                condition.helpless(),
                motions.0.action_index(&anim.action).unwrap_or(0) as u8,
            )
        };
        // The nearest foe attacking with a swing the trigger accepts.
        let mut best = f32::INFINITY;
        let foes = bodies.p1();
        for (e, condition, fighter, anim, swing, attacking, gt, body) in foes.iter() {
            if e == entity || *condition != Condition::ATTACKING {
                continue;
            }
            let Some(attacking) = attacking else { continue };
            let chest = stance::chest_point(gt, body);
            let delta = chest - def_chest;
            let distance = delta.length();
            if distance >= best {
                continue;
            }
            let to_def = (def_chest - chest).with_y(0.0);
            let progress = motions
                .clip(anim, &anim.action)
                .map_or(0.0, |c| anim.clip_progress(c));
            // A chained attack (the continuation flag) skips the clock's
            // window, as the original's combo flag does.
            let chained = swing.has(crate::swing::flags::CONTINUATION);
            let cleaving = swing.has(crate::swing::flags::CLEAVING);
            let reach_cm = reach_cm(e, &blades);
            let chest_delta = delta * 100.0;
            let trigger = Trigger {
                attacker_attacking: true,
                attacker_size: fighter.stats.size,
                defender_has_shield: shield,
                strike_window: chained,
                cleaving,
                progress,
                reach_cm,
                chest_delta,
                facing: def_facing,
            };
            if !parry_trigger(&trigger) {
                continue;
            }
            let motion_index = attacking.motion.index() as u8;
            let threat = Threat {
                attacking: true,
                clock: progress,
                combo: chained,
                cleaving,
                in_front: chest_delta.dot(def_facing) >= 0.0,
                distance_cm: chest_delta.length(),
                reach_cm,
                size: fighter.stats.size,
                defender_has_shield: shield,
                yellow,
                helpless,
                break_tangle: motion == 0x1C,
                motion: motion_index,
                yaw: yaw(forward(gt)) - yaw(to_def),
                defender_chest_cm: def_chest.y * 100.0,
                attacker_chest_cm: chest.y * 100.0,
                stab: matches!(attacking.motion, crate::attack::AttackMotion::Stab),
                overhead: matches!(attacking.motion, crate::attack::AttackMotion::Overhead),
            };
            best = distance;
            pick = Some(Pick {
                motion: motion_index,
                clip: clip_for(&threat),
            });
        }
    }
    // A body with no blow to check and nothing to settle is left untouched
    // (no change flagged on its components).
    if !(parries || (enters && pick.is_some()) || rests) {
        return;
    }
    let mut own = bodies.p0();
    let Ok((mut fighter, mut anim, _, mut stance, condition, _, body)) = own.get_mut(entity) else {
        return;
    };
    let mut defender = Defender {
        dazed: body
            .and_then(|b| b.drive.as_ref())
            .is_some_and(|d| d.dazed()),
        motion: motions.0.action_index(&anim.action).unwrap_or(0) as u8,
        condition: *condition,
        now,
        held,
        fighter: &mut fighter,
        anim: &mut anim,
        stance: &mut stance,
    };
    let mut left_attack = false;
    if enters && pick.is_some() {
        left_attack |= defender.check(pick);
    }
    if defender.fighter.parry.active {
        left_attack |= defender.check(pick);
    } else {
        defender.rest();
    }
    if left_attack {
        commands.entity(entity).remove::<Attacking>();
    }
}

#[cfg(test)]
mod stance_tests {
    use super::*;

    /// A parry ends into the Defence stance when the fighter is dying (yellow
    /// under 0.01 in double precision, so the float 0.01 is under it) or
    /// dazed, else into the combat idle.
    #[test]
    fn a_parry_ends_into_the_stance_the_body_needs() {
        assert_eq!(parry_stance(1.0, false), Stance::CombatIdle);
        assert_eq!(parry_stance(0.01, false), Stance::Defence);
        assert_eq!(parry_stance(0.0101, false), Stance::CombatIdle);
        assert_eq!(parry_stance(1.0, true), Stance::Defence);
    }
}

#[cfg(test)]
mod wiring_tests {
    use super::*;
    use crate::CombatPlugin;
    use crate::attack::AttackMotion;
    use crate::damage;
    use crate::swing::{Swing, flags};
    use exa_formats::rsq::{Action, Clip, Key, MotionSet, Moveset};
    use exa_formats::weapon_record::{Holder, Lengths, Section, WeaponRecord};
    use exa_formats::weapon_stats::{StatBytes, at};
    use exa_physics::body::{BodyPlugin, run_character_ticks};

    /// A synthetic motion set whose action numbers are the original's:
    /// Idle 2, Defence stance 3, Cmbt - Idle 4, Attack Mid 5, the parry
    /// clips 0x14 through 0x18 and Break Tangle 0x1C, each 100 frames.
    fn motions() -> Motions {
        let key = |time| Key {
            time,
            root: [0.0; 3],
            rotations: Vec::new(),
            flags: Vec::new(),
        };
        let clip = || Clip {
            mode: 1,
            keys: vec![key(0.0), key(100.0)],
        };
        let mut names: Vec<String> = (0..0x1D).map(|i| format!("Spare {i}")).collect();
        for (at, name) in [
            (2, "Idle"),
            (3, "Defence stance"),
            (4, "Cmbt - Idle"),
            (5, "Attack Mid"),
            (0x14, "Parry Left"),
            (0x15, "Parry Right"),
            (0x16, "Parry Up"),
            (0x17, "Parry Left Dn"),
            (0x18, "Parry Rgt Dn"),
            (0x1C, "Break Tangle"),
        ] {
            names[at] = name.into();
        }
        let set = MotionSet {
            bone_count: 0,
            actions: names
                .iter()
                .map(|n| Action {
                    name: n.clone(),
                    params: [0; 2],
                })
                .collect(),
            movesets: vec![Moveset {
                name: "Default".into(),
                parent: String::new(),
                clips: names.iter().map(|_| Some(clip())).collect(),
            }],
        };
        Motions::new(std::sync::Arc::new(set), std::sync::Arc::default(), 0.0)
    }

    fn fist() -> WeaponRecord {
        WeaponRecord::build(
            &Section::FISTS,
            false,
            Lengths::hand(-8.0, 9.66),
            &Holder::AVERAGE,
        )
    }

    fn sword() -> WeaponRecord {
        let mut stats = StatBytes::default();
        stats.set(at::WEIGHT, 30);
        stats.set(at::BALANCE, 60);
        WeaponRecord::of_one_part(0x11, &stats, false, 90.0)
    }

    /// A case of the readiness table: its name, whether the defender
    /// parries, and the setup that makes it differ from the ready one.
    type Try = (&'static str, bool, fn(&mut Duel));

    /// An unarmed defender in the combat idle at the origin facing +Z, and
    /// a foe attacking it with Attack Mid at `progress`, standing at
    /// `foe_at`, the game clock at 12 000 ms (body tick 1000). The
    /// defender's character tick runs through the combat plugin's real
    /// schedule, one tick at a time.
    struct Duel {
        app: App,
        foe: Entity,
        me: Entity,
    }

    impl Duel {
        fn new(foe_at: Vec3, progress: f32) -> Self {
            let mut app = App::new();
            app.add_plugins((MinimalPlugins, BodyPlugin, CombatPlugin))
                .insert_resource(exa_physics::Physics::default())
                .insert_resource(motions());
            app.world_mut().resource_mut::<BodyTicks>().0 = 1000;
            let foe = app
                .world_mut()
                .spawn((
                    Fighter::default(),
                    {
                        let mut anim = Animator::new("Default", "Attack Mid");
                        anim.time = progress * 100.0;
                        anim.looping = false;
                        anim
                    },
                    Condition::ATTACKING,
                    Attacking::new(AttackMotion::Main(0), 0x80),
                    SwingState::default(),
                    GlobalTransform::from_translation(foe_at),
                ))
                .id();
            let me = app
                .world_mut()
                .spawn((
                    Fighter::default(),
                    Animator::new("Default", "Cmbt - Idle"),
                    Stance::CombatIdle,
                    SwingState::default(),
                    Condition::COMBAT_IDLE,
                    Attacker::default(),
                    GlobalTransform::default(),
                    Ragdoll::new(&[]).with_drive(),
                ))
                .id();
            app.world_mut().spawn(Blade::new(me, 0.18, fist()));
            Self { app, foe, me }
        }

        /// A foe 1 m ahead and 0.5 m higher: the table picks Parry Left.
        fn high() -> Self {
            Self::new(Vec3::new(0.0, 0.5, 1.0), 0.5)
        }

        /// One character tick of the defender, then the game clock's.
        fn tick(&mut self) {
            run_character_ticks(
                self.app.world_mut(),
                [TickedCharacter {
                    body: self.me,
                    character: None,
                }],
            );
            self.app.world_mut().resource_mut::<BodyTicks>().0 += 1;
        }

        fn get<T: Component>(&self) -> &T {
            self.app.world().get::<T>(self.me).unwrap()
        }

        fn edit<T: Component<Mutability = bevy::ecs::component::Mutable>>(
            &mut self,
            f: impl FnOnce(&mut T),
        ) {
            f(&mut *self.app.world_mut().get_mut::<T>(self.me).unwrap());
        }

        fn parrying(&self) -> bool {
            self.get::<Fighter>().parry.active
        }

        /// The foe stops attacking.
        fn foe_rests(&mut self) {
            *self.app.world_mut().get_mut::<Condition>(self.foe).unwrap() = Condition::COMBAT_IDLE;
        }
    }

    /// An attacker mid-swing 1 m in front within reach + 150 cm starts the
    /// defender's parry from the combat idle: the height table gives
    /// Parry Left against a foe 0.5 m higher, the blend is round(1 · 20)
    /// frames with the fast ease, the guard becomes the parry the hit path
    /// reads and the game clock is kept. The start tick takes the counter
    /// twice (the idle's check, then the parrying body's own), a held parry
    /// once a tick; the foe's blow ending ends the parry into the combat
    /// idle with the 40-frame smooth blend, and the counter drifts back.
    #[test]
    fn a_parry_starts_holds_and_ends_in_the_character_tick() {
        let mut d = Duel::high();
        d.tick();
        let f = d.get::<Fighter>();
        assert!(f.parry.active);
        assert_eq!(f.parry.clip, Some(LEFT));
        assert_eq!(f.parry.counter_motion, Some(5));
        assert_eq!(f.parry.counter, 2);
        assert_eq!(f.parry.last_ms, Some(12_000));
        assert!(matches!(f.guard, Guard::Parry));
        let anim = d.get::<Animator>();
        assert_eq!(anim.action, "Parry Left");
        assert_eq!(anim.blend, 20.0);
        assert_eq!(anim.ease, Ease::Fast);
        assert!(anim.looping);

        d.tick();
        d.tick();
        let f = d.get::<Fighter>();
        assert_eq!((f.parry.counter, f.parry.last_ms), (4, Some(12_024)));
        assert_eq!(d.get::<Animator>().blend, 20.0);

        d.foe_rests();
        d.tick();
        let f = d.get::<Fighter>();
        assert!(!f.parry.active);
        assert_eq!((f.parry.clip, f.parry.counter_motion), (None, None));
        assert!(matches!(f.guard, Guard::Open));
        let anim = d.get::<Animator>();
        assert_eq!(anim.action, "Cmbt - Idle");
        assert_eq!((anim.blend, anim.ease), (40.0, Ease::Smooth));
        assert_eq!(*d.get::<Stance>(), Stance::CombatIdle);
        assert_eq!(d.get::<Fighter>().parry.counter, 4);
        d.tick();
        assert_eq!(d.get::<Fighter>().parry.counter, 3);
    }

    /// A foe level with the defender is answered by Parry Left Dn; a parry
    /// that is kept does not start the clip again, and a new motion of the
    /// foe picks again.
    #[test]
    fn the_clip_follows_the_height_and_a_new_motion_picks_again() {
        let mut d = Duel::new(Vec3::new(0.0, 0.0, 1.0), 0.5);
        d.tick();
        assert_eq!(d.get::<Fighter>().parry.clip, Some(LEFT_DN));
        assert_eq!(d.get::<Animator>().action, "Parry Left Dn");
        // A kept parry leaves its clip playing (the clock does not run in
        // this schedule, so a restart would show as a blend reset).
        d.edit::<Animator>(|a| a.blend = 7.0);
        d.tick();
        assert_eq!(d.get::<Animator>().blend, 7.0);
        // The foe attacks with the other side: the clip is picked again.
        let foe = d.foe;
        d.app
            .world_mut()
            .entity_mut(foe)
            .insert(Attacking::new(AttackMotion::Other(0), 0x80));
        d.tick();
        let f = d.get::<Fighter>();
        assert_eq!(f.parry.counter_motion, Some(8));
        assert_eq!(f.parry.clip, Some(RIGHT_DN));
        assert_eq!(d.get::<Animator>().action, "Parry Rgt Dn");
        assert_eq!(d.get::<Animator>().blend, 20.0);
    }

    /// The combat idle's check needs its gates: the auto-parry deadline
    /// (400 ms after the last attack tick, strictly passed), muscle tone of
    /// 0.85, hands holding something, nothing in them that is a weapon (its
    /// grip is not modelled), and the combat idle itself.
    #[test]
    fn the_idle_checks_only_when_ready() {
        let tries: [Try; 8] = [
            ("ready", true, |_| {}),
            ("deadline met", false, |d| {
                d.edit::<Attacker>(|a| a.last_tick_ms = Some(11_600));
            }),
            ("deadline just past", true, |d| {
                d.edit::<Attacker>(|a| a.last_tick_ms = Some(11_599));
            }),
            ("tone 0.84", false, |d| {
                d.edit::<Ragdoll>(|b| b.drive.as_mut().unwrap().tone = 0.84);
            }),
            ("tone 0.85", true, |d| {
                d.edit::<Ragdoll>(|b| b.drive.as_mut().unwrap().tone = 0.85);
            }),
            ("empty hands", false, |d| {
                let me = d.me;
                let blades: Vec<Entity> = d
                    .app
                    .world_mut()
                    .query::<(Entity, &Blade)>()
                    .iter(d.app.world())
                    .filter(|(_, b)| b.wielder == me)
                    .map(|(e, _)| e)
                    .collect();
                for e in blades {
                    d.app.world_mut().despawn(e);
                }
            }),
            ("armed", false, |d| {
                let me = d.me;
                assert!(sword().armed());
                d.app.world_mut().spawn(Blade::new(me, 0.9, sword()));
            }),
            ("not in the combat idle", false, |d| {
                d.edit::<Animator>(|a| a.action = "Attack Mid".into());
            }),
        ];
        for (name, parries, setup) in tries {
            let mut d = Duel::high();
            setup(&mut d);
            d.tick();
            assert_eq!(d.parrying(), parries, "{name}");
        }
    }

    /// A recoiled swing waits out its recoil's stamp instead of the last
    /// attack tick, and a defender that is parrying checks without any of
    /// the idle's gates (muscles under 0.85 included).
    #[test]
    fn a_recoil_moves_the_deadline_and_a_parry_holds_through_a_weak_tick() {
        let mut d = Duel::high();
        d.edit::<SwingState>(|s| {
            s.flags = flags::RECOILED;
            s.end_ms = 11_700;
        });
        d.tick();
        assert!(!d.parrying(), "11 700 + 400 is after 12 000");
        let mut d = Duel::high();
        d.edit::<SwingState>(|s| {
            s.flags = flags::RECOILED;
            s.end_ms = 11_500;
        });
        d.tick();
        assert!(d.parrying());
        d.edit::<Ragdoll>(|b| b.drive.as_mut().unwrap().tone = 0.5);
        d.tick();
        assert!(d.parrying());
        assert_eq!(d.get::<Fighter>().parry.counter, 3);
    }

    /// The trigger's clock window is 0.3 through 0.9 of the foe's clip,
    /// unless the foe's swing is a continuation; an open strike window is
    /// not that flag.
    #[test]
    fn the_foes_clock_window_is_skipped_only_for_a_continuation() {
        let at = Vec3::new(0.0, 0.5, 1.0);
        for (progress, expected) in [(0.29, false), (0.3, true), (0.9, true), (0.91, false)] {
            let mut d = Duel::new(at, progress);
            d.tick();
            assert_eq!(d.parrying(), expected, "progress {progress}");
        }
        let mut d = Duel::new(at, 0.2);
        let foe = d.foe;
        d.app.world_mut().get_mut::<SwingState>(foe).unwrap().open = [true, true];
        d.tick();
        assert!(!d.parrying());
        d.app.world_mut().get_mut::<SwingState>(foe).unwrap().flags = flags::CONTINUATION;
        d.tick();
        assert!(d.parrying());
    }

    /// A parry ends into the Defence stance for a dying or dazed body, with
    /// the same 40-frame blend; a condition whose clip stands still leaves
    /// the clip and the stance alone.
    #[test]
    fn a_parry_ends_into_the_stance_by_the_bodys_state() {
        // Dying: yellow health under 0.01 ends it at the check.
        let mut d = Duel::high();
        d.tick();
        d.edit::<Fighter>(|f| f.health.yellow = 0.005);
        d.tick();
        assert!(!d.parrying());
        assert_eq!(d.get::<Animator>().action, "Defence stance");
        assert_eq!(d.get::<Animator>().blend, 40.0);
        assert_eq!(*d.get::<Stance>(), Stance::Defence);
        // Dazed: the foe's blow ends, the energy is 0.1 under its most.
        let mut d = Duel::high();
        d.tick();
        d.edit::<Ragdoll>(|b| {
            let drive = b.drive.as_mut().unwrap();
            drive.energy = drive.stats.max_energy - 0.1;
        });
        d.foe_rests();
        d.tick();
        assert_eq!(d.get::<Animator>().action, "Defence stance");
        assert_eq!(*d.get::<Stance>(), Stance::Defence);
        // A shocked body (helpless, its clip stands still, and the posture
        // stage keeps the condition): the parry goes, the clip stays and so
        // does the stance, tick after tick.
        let mut d = Duel::high();
        d.tick();
        d.edit::<Condition>(|c| *c = Condition::SHOCKED);
        d.tick();
        d.tick();
        assert!(!d.parrying());
        assert_eq!(d.get::<Animator>().action, "Parry Left");
        assert_eq!(*d.get::<Stance>(), Stance::CombatIdle);
        // Break Tangle, the clip number 0x1C, ends it as well.
        let mut d = Duel::high();
        d.tick();
        d.edit::<Animator>(|a| a.action = "Break Tangle".into());
        d.tick();
        assert!(!d.parrying());
        assert_eq!(d.get::<Animator>().action, "Cmbt - Idle");
    }

    /// A parry the hit path ended (the flag and counter motion cleared,
    /// the clip still playing) returns to the stance on the body's next
    /// tick, and a stale parry guard opens; a shield's guard is never
    /// replaced by the parry's.
    #[test]
    fn a_parry_ended_by_a_blow_returns_to_the_stance() {
        let mut d = Duel::high();
        d.tick();
        d.edit::<Fighter>(|f| f.parry.end());
        d.tick();
        assert_eq!(d.get::<Animator>().action, "Cmbt - Idle");
        assert_eq!(d.get::<Animator>().blend, 40.0);
        assert!(matches!(d.get::<Fighter>().guard, Guard::Open));

        let mut d = Duel::high();
        let shield = Guard::Shield(damage::ShieldGuard::default());
        d.edit::<Fighter>(|f| f.guard = shield);
        d.tick();
        assert!(d.parrying());
        assert!(matches!(d.get::<Fighter>().guard, Guard::Shield(_)));
    }

    /// A defender that has nothing to parry (no foe attacking) or only
    /// rests is left as it is: the tick flags none of its stance or clip as
    /// changed.
    #[test]
    fn a_body_with_nothing_to_parry_is_not_touched() {
        let mut d = Duel::high();
        d.foe_rests();
        d.tick();
        let before = d.app.world().read_change_tick();
        d.tick();
        let world = d.app.world();
        let now = world.read_change_tick();
        let me = world.entity(d.me);
        assert!(
            !me.get_change_ticks::<Stance>()
                .unwrap()
                .is_changed(before, now)
        );
        assert!(
            !me.get_change_ticks::<Animator>()
                .unwrap()
                .is_changed(before, now)
        );
        assert!(!d.parrying());
    }

    /// A fist-swinging fighter in the swing's stall (a blow bogged it
    /// down) paces its clip by the stall's decay through the real
    /// character tick, as the swing's own numbers do (the clip rate 1.62
    /// times 0.7 and then 0.4 of it, 0 at the stall's end), and ends its
    /// attack in the stance once the end passes; the parry rule does not
    /// disturb a body that is attacking.
    #[test]
    fn a_stalled_swing_slows_and_ends_in_the_character_tick() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, BodyPlugin, CombatPlugin))
            .insert_resource(exa_physics::Physics::default())
            .insert_resource(motions());
        app.world_mut().resource_mut::<BodyTicks>().0 = 1000;
        let mut anim = Animator::new("Default", "Cmbt - Idle");
        anim.play_once("Attack Mid", "Cmbt - Idle");
        anim.time = 5.0;
        let fighter = app
            .world_mut()
            .spawn((
                Fighter::default(),
                anim,
                Attacking::new(AttackMotion::Main(0), 0x80),
                Condition::ATTACKING,
                GlobalTransform::IDENTITY,
                Swing::FISTS,
                Stance::CombatIdle,
                Attacker::default(),
                Ragdoll::new(&[]).with_drive(),
            ))
            .id();
        *app.world_mut().get_mut::<SwingState>(fighter).unwrap() = SwingState {
            flags: flags::STALLING | flags::RELEASED | flags::NO_CHAIN,
            stall: 0.3,
            end_ms: 12_036,
            ..SwingState::default()
        };
        let mut rates = Vec::new();
        for _ in 0..4 {
            run_character_ticks(
                app.world_mut(),
                [TickedCharacter {
                    body: fighter,
                    character: None,
                }],
            );
            app.world_mut().resource_mut::<BodyTicks>().0 += 1;
            rates.push(app.world().get::<SwingState>(fighter).unwrap().rate);
        }
        let close = |a: f32, b: f32| (a - b).abs() < 1e-4;
        assert!(close(rates[0], 1.62 * 0.7), "{rates:?}");
        assert!(close(rates[1], 1.62 * 0.4), "{rates:?}");
        assert!(rates[2] < 0.17 && close(rates[3], 0.0), "{rates:?}");
        assert!(app.world().get::<Attacking>(fighter).is_some());
        run_character_ticks(
            app.world_mut(),
            [TickedCharacter {
                body: fighter,
                character: None,
            }],
        );
        assert!(app.world().get::<Attacking>(fighter).is_none());
        let anim = app.world().get::<Animator>(fighter).unwrap();
        assert_eq!(anim.action, "Cmbt - Idle");
        assert!(!app.world().get::<Fighter>(fighter).unwrap().parry.active);
    }
}
