//! The hit veto a body may carry: the original's damage routine asks the
//! struck body's veto callback, when it has one, about every weapon contact
//! on a living limb (kind 1) whose landing point is short of the weapon's
//! tip, after the struck body's regeneration ramp is reset; a veto that
//! answers yes ends the routine there, so the blow deals nothing.
//!
//! The only callback 0.9.5 installs is Constant Displacement's phasing
//! ([`PhaseVeto`]): its streak `s` lets a blow through when the engine
//! generator's percentage is at most `100 >> (s + 1)` (`s ≥ 0`) or
//! `100 − (100 >> (1 − s))` (`s < 0`); a pass raises a non-negative streak
//! (a negative one restarts at 0), a refusal lowers a non-positive one (a
//! positive one restarts at 0). A pass draws the phasing sound's pitch,
//! `0.9 + 0.2·r` in double, and the power system does the rest
//! ([`BlowPhased`]).
//!
//! The original installs the callback when the power starts and never
//! takes it away: once a body has run Constant Displacement its blows keep
//! being asked [the callback then reads the ended power's freed record;
//! Exarust keeps the streak on the body].
//!
//! Evidence: the damage routine's veto call and the phasing callback, read
//! from `Exanima.exe`.

use bevy::prelude::*;

use crate::Roll;

/// The struck body's veto: a constant displacement's phasing streak.
#[derive(Component, Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PhaseVeto {
    pub streak: i32,
}

/// Whether the routine reaches the veto for a blow landing at `share` of
/// the weapon's length: anything short of the tip, a non-number too (the
/// original's two ordered compares both refuse it, and it is taken as 0).
pub fn short_of_tip(share: f32) -> bool {
    share.partial_cmp(&1.0) != Some(std::cmp::Ordering::Greater)
        && share.partial_cmp(&1.0) != Some(std::cmp::Ordering::Equal)
}

/// The highest percentage that lets a blow through at streak `s`.
pub fn phase_threshold(s: i32) -> i32 {
    if s < 0 {
        100 - (100u32 >> ((1 - s) as u32 & 0x1f)) as i32
    } else {
        (100u32 >> ((s + 1) as u32 & 0x1f)) as i32
    }
}

/// Decides one blow with the percentage `percent` (0 to 99): true when it
/// passes through, the streak moved on either way.
pub fn phase_decide(streak: &mut i32, percent: i32) -> bool {
    if phase_threshold(*streak) < percent {
        *streak = if *streak < 1 { *streak - 1 } else { 0 };
        false
    } else {
        *streak = if *streak < 0 { 0 } else { *streak + 1 };
        true
    }
}

/// The phasing sound's pitch from the generator's unit draw: `r·0.2 + 0.9`
/// in double, narrowed.
pub fn phase_pitch(unit: f32) -> f32 {
    (f64::from(unit) * 0.2 + 0.9) as f32
}

impl PhaseVeto {
    /// Asks the veto about one blow: the generator's percentage, then on a
    /// pass its unit draw for the sound's pitch. `Some(pitch)` vetoes it.
    pub fn ask(&mut self, roll: &mut Roll) -> Option<f32> {
        let percent = roll.below(100) as i32;
        phase_decide(&mut self.streak, percent).then(|| phase_pitch(roll.next()))
    }
}

/// A blow the struck body's veto let through: the power system plays the
/// phasing sound at the struck body (volume 0.6, `pitch`), phases the
/// striker (collision bits `0xC000` on it and its descendants), hangs a
/// tendril from the striker at the contact and gives the striker its
/// collisions back 500 ms later.
#[derive(Message, Debug, Clone, Copy, PartialEq)]
pub struct BlowPhased {
    /// The phased body.
    pub target: Entity,
    pub attacker: Entity,
    /// The body that struck (the weapon, or the fist's blade).
    pub striker: Entity,
    /// The contact (world, metres).
    pub point: Vec3,
    pub pitch: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_blow_short_of_the_tip_is_asked() {
        assert!(short_of_tip(0.99));
        assert!(short_of_tip(-3.0));
        assert!(short_of_tip(f32::NAN));
        assert!(!short_of_tip(1.0));
        assert!(!short_of_tip(1.5));
    }

    #[test]
    fn the_threshold_halves_with_the_streak() {
        assert_eq!(phase_threshold(0), 50);
        assert_eq!(phase_threshold(1), 25);
        assert_eq!(phase_threshold(2), 12);
        assert_eq!(phase_threshold(-1), 75);
        assert_eq!(phase_threshold(-2), 88);
        assert_eq!(phase_threshold(-6), 100);
    }

    #[test]
    fn a_pass_raises_and_a_refusal_lowers_the_streak() {
        let mut s = 0;
        assert!(phase_decide(&mut s, 50));
        assert_eq!(s, 1);
        assert!(!phase_decide(&mut s, 26));
        assert_eq!(s, 0);
        assert!(!phase_decide(&mut s, 51));
        assert_eq!(s, -1);
        assert!(!phase_decide(&mut s, 76));
        assert_eq!(s, -2);
        assert!(phase_decide(&mut s, 88));
        assert_eq!(s, 0);
    }

    #[test]
    fn the_veto_draws_the_percentage_then_the_pitch() {
        // The generator from 0: x1 = 1, x2 = 0x0808_8406.
        let mut veto = PhaseVeto::default();
        let mut roll = Roll::default();
        // 1·100 >> 32 = 0: a pass; the pitch takes the second draw.
        let pitch = veto.ask(&mut roll).unwrap();
        let x2 = 0x0808_8406u32;
        let r = (f64::from(x2) * 2f64.powi(-32)) as f32;
        assert_eq!(pitch, (f64::from(r) * 0.2 + 0.9) as f32);
        assert_eq!(veto.streak, 1);
    }
}
