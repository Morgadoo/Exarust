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
//! Evidence: the unarmed parry decision and the height table.

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
}
