//! The look roll of the dormant object editor: the dice it throws before a
//! new object is placed, which fill the editor's colour and flag fields.
//!
//! Nothing in the shipped game reads the fields it fills (the editor module
//! is not entered), and their meanings are not established, so the fields
//! are named after the arithmetic that produces them. The roll draws from
//! two process-wide streams, exactly as the rest of the engine unit does:
//! the engine's integer generator and the run-time library's `Random`, which
//! is the Mersenne twister scaled by 2⁻³² (the caller supplies it, since
//! this crate holds no generator of its own).

use super::packed::round_f64;

/// The families a first colour may come from (the procedural palette's
/// primary families; the original keeps the same seven values beside the
/// editor's roll).
const FAMILIES: [u8; 7] = [0, 4, 8, 14, 19, 25, 28];

/// The colour/shade pairs of the trim row.
const TRIM: [(u8, u8); 7] = [
    (7, 12),
    (36, 24),
    (108, 64),
    (155, 24),
    (204, 32),
    (228, 24),
    (253, 32),
];

/// The editor's fields one roll fills.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LookRoll {
    /// The first colour: a family from [`FAMILIES`] with shade
    /// `7 − round(6r²)`, or a family 0–31 the same way, `r` the unit draw
    /// (the procedural palette's cloth-colour rule).
    pub primary: u8,
    /// The second colour: a family 0–31 with shade 7 or 6.
    pub secondary: u8,
    /// A shade of 3 or 4.
    pub tertiary: u8,
    /// A colour and its shade from the seven trim rows, or `(3, 0)` when
    /// the roll is not the full one or the one-in-five chance passes.
    pub trim: (u8, u8),
    /// Two of the editor's bytes the roll always clears (one after the
    /// third colour, one after the light).
    pub cleared: [u8; 2],
    /// One when the roll's draw had its top bit clear, else zero.
    pub light: u8,
    /// One in three: a shade of 3 or 4 from a top-bit draw; otherwise
    /// `0x53` plus a draw below three (83–85).
    pub finish: u8,
}

/// One step of the engine's integer generator: `state ← state·0x0808_8405 +
/// 1`, wrapping.
fn step(state: &mut u32) -> u32 {
    *state = state.wrapping_mul(0x0808_8405).wrapping_add(1);
    *state
}

/// The high half of `state · n`, drawn as one generator step.
fn below(state: &mut u32, n: u32) -> u32 {
    ((u64::from(step(state)) * u64::from(n)) >> 32) as u32
}

/// Rolls the editor's look. `full` is the flag the editor's new-object step
/// passes as 1: without it the trim keeps its default and the state does not
/// advance for it. `state` is the engine's integer generator, `random` the
/// run-time library's `Random` (a draw in `[0, 1)`); the original throws
/// exactly this many draws in this order.
pub fn look_roll(full: bool, state: &mut u32, random: &mut dyn FnMut() -> f64) -> LookRoll {
    // The first colour: half the time a family from the seven-name table,
    // else any of the 32 families; then a shade 7 less round(6r²), rounded
    // in double precision as the original's conversion does.
    let first = step(state);
    let family = if first >> 31 == 0 {
        i64::from(FAMILIES[below(state, 7) as usize])
    } else {
        i64::from(step(state) >> 27)
    };
    let r = random();
    let primary = (family * 8 + 7).wrapping_sub(round_f64(r * r * 6.0)) as u8;

    let family = step(state) >> 27;
    let shade = step(state) >> 31;
    let secondary = (family * 8 + 7 - shade) as u8;
    let tertiary = ((step(state) >> 31) + 3) as u8;

    let trim = if !full || below(state, 5) != 0 {
        (3, 0)
    } else {
        TRIM[below(state, 7) as usize]
    };

    let light = u8::from(step(state) >> 31 == 0);

    let kind = below(state, 3);
    let finish = if kind > 1 {
        ((step(state) >> 31) + 3) as u8
    } else {
        (below(state, 3) + 0x53) as u8
    };

    LookRoll {
        primary,
        secondary,
        tertiary,
        trim,
        cleared: [0; 2],
        light,
        finish,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A stand-in for the run-time library's `Random`: a fixed list of
    /// draws, recorded so a test can prove how many were taken.
    struct Draws {
        values: Vec<f64>,
        at: usize,
    }

    impl Draws {
        fn new(values: &[f64]) -> Self {
            Self {
                values: values.to_vec(),
                at: 0,
            }
        }

        fn next(&mut self) -> f64 {
            let v = self.values.get(self.at).copied().unwrap_or(0.0);
            self.at += 1;
            v
        }
    }

    #[test]
    fn a_full_roll_from_seed_zero() {
        // The first family draw is 0, so the first colour is family 0 with
        // shade 7 − round(6·0.36²) = 7 − 1 = 6. The trim's one-in-five test
        // passes (its draw is non-zero), so the row is not taken.
        let mut state = 0;
        let mut draws = Draws::new(&[0.36]);
        let look = look_roll(true, &mut state, &mut || draws.next());
        assert_eq!(
            look,
            LookRoll {
                primary: 6,
                secondary: 223,
                tertiary: 3,
                trim: (3, 0),
                cleared: [0; 2],
                light: 1,
                finish: 84,
            }
        );
        // One draw from the run-time library; nine generator steps: the
        // family, the second colour's two, the third, the one-in-five test,
        // the light, the finish test and the finish.
        assert_eq!(draws.at, 1);
        assert_eq!(state, 0x5F4B_0359);
    }

    #[test]
    fn a_roll_without_the_full_flag_keeps_the_trim() {
        let mut with = 0u32;
        let mut without = 0u32;
        let full = look_roll(true, &mut with, &mut || 0.36);
        let plain = look_roll(false, &mut without, &mut || 0.36);
        assert_eq!(plain.trim, (3, 0));
        assert_eq!(plain.primary, full.primary);
        assert_eq!(plain.secondary, full.secondary);
        assert_eq!(plain.tertiary, full.tertiary);
        // Without the flag the trim's own step is not drawn at all, so every
        // later field comes from the step before the full roll's.
        assert_eq!(plain.light, 0);
        assert_eq!(plain.finish, 83);
        assert_eq!(without, 0x296B_6D78);
    }

    #[test]
    fn the_trim_row_is_drawn_one_time_in_five() {
        // Seed 10: the one-in-five test draws zero, so the trim row is taken
        // and its own draw picks the sixth row. The finish's three-way draw
        // lands in its high branch, giving a shade of 4.
        let mut state = 10;
        let look = look_roll(true, &mut state, &mut || 0.0);
        assert_eq!(look.trim, (228, 24));
        assert_eq!(look.finish, 4);
        assert_eq!(look.light, 1);

        // Seed 16 has the first step's top bit set, so the first colour is
        // any family 0–31 (9) rather than one of the seven; with a draw of
        // 0.5 the shade is 7 − round(1.5) = 5.
        let mut state = 16;
        let look = look_roll(true, &mut state, &mut || 0.5);
        assert_eq!(look.primary, 77);
        assert_eq!(look.secondary, 87);
        assert_eq!(look.tertiary, 4);
        assert_eq!(look.trim, (228, 24));
        assert_eq!(look.finish, 85);
    }

    #[test]
    fn the_colours_wrap_to_a_byte() {
        // 6·0.9² = 4.86 rounds to 5, leaving family 0 at shade 2.
        let mut state = 0;
        let look = look_roll(false, &mut state, &mut || 0.9);
        assert_eq!(look.primary, 2);

        // A NaN draw converts to the integer indefinite, whose low byte is
        // zero: the colour keeps the family's shade 7.
        let mut state = 0;
        let look = look_roll(false, &mut state, &mut || f64::NAN);
        assert_eq!(look.primary, 7);
    }
}
