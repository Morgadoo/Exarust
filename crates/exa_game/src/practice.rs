//! The practice ladder: the next sparring partner, and the health restored
//! between fights.
//!
//! A fight count advances by one and stops at 8. Novice names that count
//! `opponent01` through `opponent08`. Expert adds 8, so the same counts
//! name `opponent09` through `opponent16`. The eighth fight is the last.
//! Red gains a tenth of the maximum, in double precision, and is kept
//! within the maximum. Yellow then copies red.

/// How many fights a practice ladder has.
pub const PRACTICE_FIGHTS: i32 = 8;
/// The share of maximum health restored to red between fights, the double 0.1.
pub const PRACTICE_HEAL_BITS: u64 = 0x3fb9_9999_9999_999a;

/// Which practice ladder the next partner comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ladder {
    Novice,
    Expert,
}

/// One step along the ladder.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NextFight {
    /// The fight count after the step, at most [`PRACTICE_FIGHTS`].
    pub fight: i32,
    /// The partner's name, ten bytes, as `opponent` plus two digits.
    pub name: [u8; 10],
    /// The eighth fight, the last on the ladder.
    pub boss: bool,
    /// Yellow after the heal, the same bits as red.
    pub yellow_bits: u32,
    /// Red after the heal.
    pub red_bits: u32,
}

/// The next practice fight after `fight`. Red gains a tenth of `max` and
/// yellow copies that red.
pub fn next_practice(fight: i32, ladder: Ladder, red: f32, max: f32) -> NextFight {
    let fight = fight.wrapping_add(1).min(PRACTICE_FIGHTS);
    let number = match ladder {
        Ladder::Novice => fight,
        Ladder::Expert => fight.wrapping_add(PRACTICE_FIGHTS),
    };
    let mut name = *b"opponent00";
    let tens = number / 10;
    let ones = number % 10;
    if (0..10).contains(&tens) {
        name[8] = b'0' + tens as u8;
    }
    if (0..10).contains(&ones) {
        name[9] = b'0' + ones as u8;
    }
    let red = healed_red(red, max);
    NextFight {
        fight,
        name,
        boss: fight == PRACTICE_FIGHTS,
        yellow_bits: red.to_bits(),
        red_bits: red.to_bits(),
    }
}

/// Red plus a tenth of the maximum, stored as a float, and not above the
/// maximum. A non-number is left as it is.
fn healed_red(red: f32, max: f32) -> f32 {
    let red = (f64::from(max) * f64::from_bits(PRACTICE_HEAL_BITS) + f64::from(red)) as f32;
    if red > max { max } else { red }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_practice_ladder_names_the_partner_and_heals() {
        let step = |fight, ladder, red| next_practice(fight, ladder, f32::from_bits(red), 0.25);
        let first = step(0, Ladder::Novice, 0x3e4c_cccd);
        assert_eq!(first.fight, 1);
        assert_eq!(&first.name, b"opponent01");
        assert!(!first.boss);
        assert_eq!(
            (first.yellow_bits, first.red_bits),
            (0x3e66_6667, 0x3e66_6667)
        );
        let last = step(7, Ladder::Novice, 0x3e4c_cccd);
        assert_eq!(last.fight, 8);
        assert_eq!(&last.name, b"opponent08");
        assert!(last.boss);
        let stayed = step(8, Ladder::Novice, 0x3e80_0000);
        assert_eq!(stayed.fight, 8);
        assert_eq!(&stayed.name, b"opponent08");
        assert_eq!(stayed.red_bits, 0x3e80_0000);
        let expert = step(0, Ladder::Expert, 0x3e4c_cccd);
        assert_eq!(&expert.name, b"opponent09");
        assert!(!expert.boss);
        let boss = step(7, Ladder::Expert, 0x3e75_c28f);
        assert_eq!(boss.fight, 8);
        assert_eq!(&boss.name, b"opponent16");
        assert!(boss.boss);
        assert_eq!(boss.red_bits, 0x3e80_0000);
        assert_eq!(boss.yellow_bits, boss.red_bits);
    }
}
