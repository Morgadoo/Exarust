//! An unarmed parry, chosen from the blow that is already coming in.
//!
//! A helpless fighter, a break-tangle motion, or yellow health under 0.01
//! parries nothing. Otherwise the attacker must be attacking, not cleaving,
//! in front, and within its reach plus 150 cm. Its clock must sit from 0.3
//! through 0.9, unless the combo flag is set. An attacker of size 1.3 or
//! more is refused unless the defender has a shield. An unarmed parry then
//! succeeds. Its speed is 1 and its blend is 20 frames. The clip is the
//! height table for the attack motion: left for motions 5 through 7, right
//! for 8 through 10, the down clip when the table height is at or below the
//! chest difference.
//!
//! Evidence: the unarmed parry decision, the height table, and the armed grip.

use std::f32::consts::PI;

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
/// falls to 0 at 250 cm, and stays 0 past that. A weapon's strength is
/// (1.4 − handling) times that falloff times 0.75 times 0.6, and its speed is
/// the handling. A shield's speed is 1 minus half the weight times stamina,
/// and its strength is stamina times weight times the falloff times 0.75
/// times 0.4, with flags 0x800. The off hand is slot 1 only while attacking
/// with a motion outside 8 through 10 and two weapons are held.
pub fn armed_grip(arm: &Arm) -> Option<Grip> {
    if arm.cleaving {
        return None;
    }
    let falloff = parry_falloff(arm.horizontal_cm);
    let slot = grip_slot(arm);
    if arm.passed {
        let speed = if arm.shield {
            arm.shield_weight * arm.stamina
        } else {
            arm.handling * arm.handling
        };
        return Some(grip(speed, 0.0, slot, 0x100000, false, true));
    }
    if arm.shield {
        let speed = 1.0 - arm.shield_weight * arm.stamina * 0.5_f32;
        if !arm.has_grip {
            return Some(grip(speed, 0.0, slot, 0, false, false));
        }
        let strength = arm.stamina * arm.shield_weight * falloff * 0.75_f32 * 0.4_f32;
        return Some(grip(speed, strength, slot, 0x800, true, arm.other_held));
    }
    let speed = arm.handling;
    if !arm.has_grip {
        return Some(grip(speed, 0.0, slot, 0, false, false));
    }
    let strength = (1.4_f32 - arm.handling) * falloff * 0.75_f32 * 0.6_f32;
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
        blend: (20.0 * speed).round() as u32,
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
    } else if distance <= 250.0 {
        1.0 - (distance - 80.0) * (1.0_f32 / 170.0)
    } else {
        0.0
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

/// A parry in progress on the defender: the original's parrying flag
/// (`+0x12f3`) and the attack motion it counters (`+0xf5c`).
///
/// Exarust raises a guard only in tests, so a fighter whose guard is a parry
/// is parrying until the parry ends; a counter-parry ends it and leaves the
/// motion it countered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParryState {
    /// It is parrying (`+0x12f3`).
    pub active: bool,
    /// The attack motion it counters (`+0xf5c`), as the original's motion
    /// number, or none.
    pub counter_motion: Option<usize>,
}

impl Default for ParryState {
    fn default() -> Self {
        Self {
            active: true,
            counter_motion: None,
        }
    }
}

impl ParryState {
    /// Ends the parry (`0x100162e80`): it counters no motion and is no
    /// longer parrying. The original also disables the grip links of every
    /// held item and relaxes both hands; Exarust has no grip links to
    /// disable yet.
    pub fn end(&mut self) {
        self.active = false;
        self.counter_motion = None;
    }
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
        };
        assert_eq!(clash_branch(parrying, 5, false), Clash::Parrying);
        let countering = ParryState {
            active: false,
            counter_motion: Some(5),
        };
        assert_eq!(clash_branch(countering, 5, false), Clash::Countering);
        assert_eq!(clash_branch(countering, 6, false), Clash::Plain);
        assert_eq!(clash_branch(countering, 5, true), Clash::Plain);
        assert_eq!(
            clash_branch(ParryState::default(), 5, false),
            Clash::Parrying
        );
    }

    #[test]
    fn a_parry_ends() {
        let mut parry = ParryState {
            active: true,
            counter_motion: Some(8),
        };
        parry.end();
        assert_eq!(
            parry,
            ParryState {
                active: false,
                counter_motion: None,
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
        assert_eq!(passed.flags, 0x100000);
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
    }
}
