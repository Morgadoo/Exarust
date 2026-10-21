//! The hire list: who is offered, how the list renews each day, and how a
//! candidate's skills and loadouts are prepared.
//!
//! A new career offers ten candidates. Each candidate seeds the roll from a
//! performance counter, steps it by `× 0x08088405 + 1`, and is a physician
//! when the high half of the word times 20 is 0, else (one more step) a
//! trainer when times 15 is 0, else (one more step) a merchant when times 10
//! is 0, else a combatant. A trainer's level is a unit draw squared times 2,
//! rounded to the nearest even integer, plus 4; a combatant's is the draw
//! squared times 5, rounded the same way; the others are level 0. Each day
//! the oldest candidate leaves, and one new candidate comes (one or two
//! while fewer than eight are listed), newest first. A trainer's skills are
//! snapped down to their levels and the technique being learnt is
//! forgotten. A combatant's six loadouts copy its kit; those of the ranks
//! below its level empty the hands and keep only the apparel the rank
//! allows.
//!
//! Evidence: the retail arena's candidate, list and loadout routines read
//! from `Exanima.exe`, arithmetic from the instructions
//! (`context/re/arena-match.md`).

use crate::career::Profession;

/// Candidates a new career lists.
pub const POOL_SIZE: usize = 10;
/// Below this many candidates a day may add two.
pub const POOL_GROWS_BELOW: usize = 8;
/// Loadouts a combatant keeps, one per rank.
pub const LOADOUTS: usize = 6;
/// Skill experience in one level.
pub const SKILL_STEP: u16 = 5000;
/// A trainer snaps a skill only below this level.
pub const SNAP_LEVELS: u16 = 5;

/// The profession's high nibble in a character's role word.
pub fn role_word(profession: Profession) -> u32 {
    match profession {
        Profession::Combatant => 0x2000_0000,
        Profession::Trainer => 0x3000_0000,
        Profession::Merchant => 0x4000_0000,
        Profession::Physician => 0x5000_0000,
    }
}

/// A candidate as the list draws it, before its body is generated.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Candidate {
    pub profession: Profession,
    /// The level the body generator receives (its experience is the level
    /// times 10000 plus a draw under 9999).
    pub level: i32,
    /// The generator's older flag (0x20000): physicians and trainers.
    pub older: bool,
    /// The five bytes a specialist's outfit draw receives; a combatant
    /// draws a kit for its level instead.
    pub outfit: Option<[u8; 5]>,
}

fn step(roll: &mut u32) -> u32 {
    *roll = roll.wrapping_mul(0x0808_8405).wrapping_add(1);
    *roll
}

fn below(roll: u32, count: u32) -> u32 {
    ((u64::from(roll) * u64::from(count)) >> 32) as u32
}

/// The processor's double-to-integer conversion: ties to even, NaN and
/// out-of-range values the 64-bit minimum.
fn convert(value: f64) -> i64 {
    let rounded = value.round_ties_even();
    if rounded.is_nan()
        || rounded >= 9.223_372_036_854_776e18
        || rounded < -9.223_372_036_854_776e18
    {
        i64::MIN
    } else {
        rounded as i64
    }
}

/// Draw one candidate. The roll is the counter's low word with its bytes
/// reversed; `unit` is the run-time library's unit draw, asked only for a
/// trainer or a combatant. Returns the candidate and the roll after it.
pub fn draw_candidate(counter: u64, unit: impl FnOnce() -> f64) -> (Candidate, u32) {
    let mut roll = (counter as u32).swap_bytes();
    step(&mut roll);
    let candidate = if below(roll, 20) == 0 {
        Candidate {
            profession: Profession::Physician,
            level: 0,
            older: true,
            outfit: Some([0x14, 0, 100, 10, 10]),
        }
    } else if below(step(&mut roll), 15) == 0 {
        let draw = unit();
        Candidate {
            profession: Profession::Trainer,
            level: (convert(draw * draw * 2.0) as i32).wrapping_add(4),
            older: true,
            outfit: Some([4, 0, 100, 10, 10]),
        }
    } else if below(step(&mut roll), 10) == 0 {
        Candidate {
            profession: Profession::Merchant,
            level: 0,
            older: false,
            outfit: Some([4, 0, 100, 10, 10]),
        }
    } else {
        let draw = unit();
        Candidate {
            profession: Profession::Combatant,
            level: convert(draw * draw * 5.0) as i32,
            older: false,
            outfit: None,
        }
    };
    (candidate, roll)
}

/// The kit share a combatant's level asks of its gear draw: the level
/// times the double 0.05, stored as single.
pub fn kit_share(level: i32) -> f32 {
    (f64::from(level) * 0.05) as f32
}

/// A new career's list: ten candidates, made in order.
pub fn fill_pool<T>(make: impl FnMut() -> T) -> Vec<T> {
    std::iter::repeat_with(make).take(POOL_SIZE).collect()
}

/// Renew the list for a new day. The last (oldest) candidate leaves when
/// any is listed. Then, while fewer than eight are listed, the roll steps
/// and one more than the high half of the word times 2 come, else one;
/// the listed move back and the new ones go first, made in order. Returns
/// how many came and who left.
pub fn renew_pool<T>(
    pool: &mut Vec<T>,
    roll: &mut u32,
    mut make: impl FnMut() -> T,
) -> (usize, Option<T>) {
    let left = pool.pop();
    let coming = if pool.len() < POOL_GROWS_BELOW {
        below(step(roll), 2) as usize + 1
    } else {
        1
    };
    let fresh: Vec<T> = std::iter::repeat_with(&mut make).take(coming).collect();
    pool.splice(0..0, fresh);
    (coming, left)
}

/// A trainer's skills: each experience word snaps down to its level times
/// 5000, and below level 5 the technique at that level, the one being
/// learnt, is forgotten.
pub fn snap_trainer(experience: &mut [u16; 6], techniques: &mut [[u8; 6]; 6]) {
    for (points, slots) in experience.iter_mut().zip(techniques.iter_mut()) {
        let level = *points / SKILL_STEP;
        *points = level.wrapping_mul(SKILL_STEP);
        if level < SNAP_LEVELS && slots[level as usize] != 0 {
            slots[level as usize] = 0;
        }
    }
}

/// A loadout: the two hands and the worn items.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Loadout {
    pub hands: [u32; 2],
    pub items: Vec<u32>,
}

/// Trim a loadout to `rank`. Both hands are emptied first, so their rank
/// test (a weapon above the rank is dropped) never finds a weapon. Then a
/// worn item whose apparel rank (the top three bits of its rank byte) is
/// above the rank is dropped, in order; an item that is not apparel stays.
pub fn trim_loadout(
    loadout: &mut Loadout,
    rank: i32,
    apparel_rank_byte: impl Fn(u32) -> Option<u8>,
) {
    loadout.hands = [0, 0];
    loadout.items.retain(|&item| match apparel_rank_byte(item) {
        Some(byte) => rank >= i32::from(byte >> 5),
        None => true,
    });
}

/// A combatant's six loadouts: each a copy of its kit. The loadouts of
/// ranks below the combatant's level are trimmed to their rank; the rest
/// keep the whole kit, hands included (the original's trim loop runs for
/// the first `level` slots only, so a level-0 combatant trims none).
pub fn ranked_loadouts(
    kit: &Loadout,
    level: i32,
    apparel_rank_byte: impl Fn(u32) -> Option<u8>,
) -> Vec<Loadout> {
    (0..LOADOUTS as i32)
        .map(|rank| {
            let mut loadout = kit.clone();
            if rank < level {
                trim_loadout(&mut loadout, rank, &apparel_rank_byte);
            }
            loadout
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_role_words_carry_the_profession_nibble() {
        assert_eq!(role_word(Profession::Combatant), 0x2000_0000);
        assert_eq!(role_word(Profession::Trainer), 0x3000_0000);
        assert_eq!(role_word(Profession::Merchant), 0x4000_0000);
        assert_eq!(role_word(Profession::Physician), 0x5000_0000);
    }

    fn never() -> f64 {
        panic!("no unit draw expected")
    }

    #[test]
    fn a_candidate_is_drawn_from_the_counter() {
        // Counter 0: the first step is 1, under a twentieth: a physician.
        let (physician, roll) = draw_candidate(0, never);
        assert_eq!(physician.profession, Profession::Physician);
        assert_eq!((physician.level, physician.older), (0, true));
        assert_eq!(physician.outfit, Some([0x14, 0, 100, 10, 10]));
        assert_eq!(roll, 1);
        let (merchant, roll) = draw_candidate(23, never);
        assert_eq!(merchant.profession, Profession::Merchant);
        assert_eq!((merchant.level, merchant.older), (0, false));
        assert_eq!(merchant.outfit, Some([4, 0, 100, 10, 10]));
        assert_eq!(roll, 0x176d_ac1f);
        // A trainer: a half squared times 2 is 0.5, which ties to 0: level 4.
        let (trainer, roll) = draw_candidate(10, || 0.5);
        assert_eq!(trainer.profession, Profession::Trainer);
        assert_eq!((trainer.level, trainer.older), (4, true));
        assert_eq!(roll, 0x0208_8406);
        assert_eq!(draw_candidate(10, || 0.75).0.level, 5);
        assert_eq!(draw_candidate(10, || 0.9).0.level, 6);
        // A combatant: 0.5 squared times 5 is 1.25, level 1.
        let (combatant, roll) = draw_candidate(3, || 0.5);
        assert_eq!(combatant.profession, Profession::Combatant);
        assert_eq!((combatant.level, combatant.outfit), (1, None));
        assert!(!combatant.older);
        assert_eq!(roll, 0x536d_ac1f);
        assert_eq!(draw_candidate(3, || 0.0).0.level, 0);
        assert_eq!(draw_candidate(3, || 0.999_999).0.level, 5);
        assert_eq!(kit_share(3).to_bits(), 0x3e19_999a);
    }

    #[test]
    fn the_list_renews_oldest_out_newest_first() {
        let mut next = 100;
        let mut make = || {
            next += 1;
            next
        };
        let mut pool = fill_pool(&mut make);
        assert_eq!(pool, (101..=110).collect::<Vec<_>>());
        // Nine left after the oldest goes: one comes.
        let mut roll = 0;
        assert_eq!(renew_pool(&mut pool, &mut roll, &mut make), (1, Some(110)));
        assert_eq!(pool[..3], [111, 101, 102]);
        assert_eq!(pool.len(), 10);
        assert_eq!(roll, 0);
        // Four left: the roll steps to 1, a half under 1: one comes.
        let mut short = vec![1, 2, 3, 4, 5];
        let mut roll = 0;
        assert_eq!(renew_pool(&mut short, &mut roll, &mut make), (1, Some(5)));
        assert_eq!(short, vec![112, 1, 2, 3, 4]);
        assert_eq!(roll, 1);
        // The roll steps to 0x80000001: two come, the first made goes first.
        let mut roll = 0x8000_0000;
        assert_eq!(renew_pool(&mut short, &mut roll, &mut make), (2, Some(4)));
        assert_eq!(short, vec![113, 114, 112, 1, 2, 3]);
        assert_eq!(roll, 0x8000_0001);
        // An empty list loses nobody and grows.
        let mut empty: Vec<i32> = Vec::new();
        let mut roll = 0;
        assert_eq!(renew_pool(&mut empty, &mut roll, &mut make), (1, None));
        assert_eq!(empty, vec![115]);
    }

    #[test]
    fn a_trainer_snaps_its_skills() {
        let mut experience = [4500, 5000, 12_345, 27_000, 0, 65_535];
        let mut techniques = [[7u8; 6]; 6];
        snap_trainer(&mut experience, &mut techniques);
        assert_eq!(experience, [0, 5000, 10_000, 25_000, 0, 65_000]);
        // The technique at each level below 5 is forgotten.
        assert_eq!(techniques[0], [0, 7, 7, 7, 7, 7]);
        assert_eq!(techniques[1], [7, 0, 7, 7, 7, 7]);
        assert_eq!(techniques[2], [7, 7, 0, 7, 7, 7]);
        assert_eq!(techniques[3], [7; 6]);
        assert_eq!(techniques[4], [0, 7, 7, 7, 7, 7]);
        assert_eq!(techniques[5], [7; 6]);
    }

    #[test]
    fn loadouts_hold_the_kit_the_rank_allows() {
        // Item 1 is rank-2 apparel (byte 0x40), item 2 is rank-0 apparel,
        // item 3 is not apparel, item 4 is rank-7 apparel (byte 0xe0).
        let rank = |item: u32| match item {
            1 => Some(0x40),
            2 => Some(0x1f),
            4 => Some(0xe0),
            _ => None,
        };
        let kit = Loadout {
            hands: [9, 10],
            items: vec![1, 2, 3, 4],
        };
        // A level-3 combatant trims the loadouts of ranks 0, 1 and 2.
        let loadouts = ranked_loadouts(&kit, 3, rank);
        assert_eq!(loadouts.len(), 6);
        assert_eq!(loadouts[0].items, vec![2, 3]);
        assert_eq!(loadouts[1].items, vec![2, 3]);
        assert_eq!(loadouts[2].items, vec![1, 2, 3]);
        assert!(loadouts[..3].iter().all(|l| l.hands == [0, 0]));
        assert_eq!(loadouts[3], kit);
        assert_eq!(loadouts[5], kit);
        // A level-0 combatant trims none.
        assert!(ranked_loadouts(&kit, 0, rank).iter().all(|l| *l == kit));
        let mut top = kit.clone();
        trim_loadout(&mut top, 7, rank);
        assert_eq!(top.items, vec![1, 2, 3, 4]);
    }
}
