//! The in-game object editor's look roll: a pair of palette colours, a
//! tone, a style, a material set and finish, and a flag, drawn from the
//! engine's two random streams in the original's order. The editor itself
//! is not part of Exarust; this keeps its rule, with the original's
//! constants, for the editor to call.
//!
//! Evidence: `docs/formats/OTHER.md` (Object database, The editor's look
//! roll).

use crate::particles::Draws;

/// The palette families the first colour prefers (a colour byte is family ×
/// 8 + shade).
const PREFERRED_FAMILIES: [u8; 7] = [0, 4, 8, 14, 19, 25, 28];

/// The material set and finish pairs the roll can pick: `(set, finish)`.
const SET_FINISHES: [(u8, u8); 7] = [
    (0x07, 0x0C),
    (0x24, 0x18),
    (0x6C, 0x40),
    (0x9B, 0x18),
    (0xCC, 0x20),
    (0xE4, 0x18),
    (0xFD, 0x20),
];

/// A rolled editor look. The colour bytes are palette bytes (family × 8 +
/// shade); the tone, style, set and finish feed the editor's material
/// fields. The original writes the fields to editor globals; the meanings
/// beyond the colour encoding and the set/finish pair are not established
/// (`[guess]` names by role).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EditorLook {
    /// The first material's colour.
    pub primary: u8,
    /// The second material's colour.
    pub secondary: u8,
    /// A tone: 3 or 4.
    pub tone: u8,
    /// A style byte: `S`–`U` or 3–4.
    pub style: u8,
    /// The material set.
    pub set: u8,
    /// The finish within the set.
    pub finish: u8,
    /// Whether the first colour keeps the preferred families; the editor's
    /// use of it is not established.
    pub flag: bool,
    /// Two fields the original always writes as zero; their use is not
    /// established.
    pub cleared: [u8; 2],
}

/// Rolls a look. `fresh` is the caller's mode: a fresh roll may pick a
/// set/finish pair, another only takes the default set 3 and finish 0. The
/// draws are, in order: the first colour's family branch, its family (one
/// of seven preferred families, or one of 32), a unit draw for its shade,
/// the second colour's family and shade, the tone, (a fresh roll's
/// five-way test and possibly a set/finish pair), the flag, and the style's
/// three-way test with its value. A non-fresh roll does not draw the
/// five-way test. The shade is `base − round(unit² · 6)`, the round being
/// the processor's to-nearest-even conversion, and the byte wraps.
pub fn roll(rng: &mut impl Draws, fresh: bool) -> EditorLook {
    let shade = |base: u16, rng: &mut dyn Draws| {
        let unit = rng.unit();
        let drop = (unit * unit * 6.0).round_ties_even() as u8;
        (base as u8).wrapping_sub(drop)
    };
    let primary = if rng.int(2) == 0 {
        let family = u16::from(PREFERRED_FAMILIES[rng.int(7) as usize]);
        shade(family * 8 + 7, rng)
    } else {
        let family = rng.int(32) as u16;
        shade(family * 8 + 7, rng)
    };
    let secondary_family = u16::from(rng.int(32) as u8);
    let secondary = (secondary_family * 8 + 7).wrapping_sub(u16::from(rng.int(2) as u8)) as u8;
    let tone = rng.int(2) as u8 + 3;
    let (set, finish) = if !fresh || rng.int(5) != 0 {
        (3, 0)
    } else {
        SET_FINISHES[rng.int(7) as usize]
    };
    let flag = rng.int(2) == 0;
    let style = if rng.int(3) <= 1 {
        rng.int(3) as u8 + 0x53
    } else {
        rng.int(2) as u8 + 3
    };
    EditorLook {
        primary,
        secondary,
        tone,
        style,
        set,
        finish,
        flag,
        cleared: [0; 2],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;

    /// A scripted source that records the bounds it was asked for.
    struct Script {
        ints: VecDeque<u32>,
        units: VecDeque<f64>,
        calls: Vec<(char, u32)>,
    }

    impl Script {
        fn new(ints: &[u32], units: &[f64]) -> Self {
            Self {
                ints: ints.iter().copied().collect(),
                units: units.iter().copied().collect(),
                calls: Vec::new(),
            }
        }
    }

    impl Draws for Script {
        fn int(&mut self, n: u32) -> u32 {
            self.calls.push(('i', n));
            self.ints.pop_front().expect("a scripted draw")
        }

        fn unit(&mut self) -> f64 {
            self.calls.push(('u', 0));
            self.units.pop_front().expect("a scripted unit")
        }
    }

    #[test]
    fn a_fresh_roll_can_pick_a_finish() {
        let mut rng = Script::new(&[1, 31, 0, 0, 0, 0, 3, 1, 2, 1], &[0.0]);
        let look = roll(&mut rng, true);
        assert_eq!(
            look,
            EditorLook {
                primary: 255,
                secondary: 7,
                tone: 3,
                style: 4,
                set: 0x9B,
                finish: 0x18,
                flag: false,
                cleared: [0; 2],
            }
        );
        assert_eq!(
            rng.calls,
            [
                ('i', 2),
                ('i', 32),
                ('u', 0),
                ('i', 32),
                ('i', 2),
                ('i', 2),
                ('i', 5),
                ('i', 7),
                ('i', 2),
                ('i', 3),
                ('i', 2),
            ]
        );
        assert!(rng.ints.is_empty() && rng.units.is_empty());
    }

    #[test]
    fn a_default_roll_takes_the_preferred_family_and_set_three() {
        // The first colour's top bit is clear: the family draw is seven-way
        // and indexes the preferred families; the shade of 0.5 squares to
        // 0.25, times 6 is 1.5 and rounds to even, 2.
        let mut rng = Script::new(&[0, 6, 31, 1, 1, 0, 0, 2], &[0.5]);
        let look = roll(&mut rng, false);
        assert_eq!(look.primary, 231 - 2);
        assert_eq!(look.secondary, 254);
        assert_eq!(look.tone, 4);
        assert_eq!((look.set, look.finish), (3, 0));
        assert!(look.flag);
        // Two draws of three: the first picks the letter branch and the
        // second the letter.
        assert_eq!(look.style, 0x53 + 2);
        assert_eq!(
            rng.calls,
            [
                ('i', 2),
                ('i', 7),
                ('u', 0),
                ('i', 32),
                ('i', 2),
                ('i', 2),
                ('i', 2),
                ('i', 3),
                ('i', 3),
            ]
        );
    }

    #[test]
    fn a_fresh_roll_that_keeps_the_default_finish() {
        let mut rng = Script::new(&[1, 0, 0, 0, 0, 2, 1, 2, 1], &[0.0]);
        let look = roll(&mut rng, true);
        assert_eq!(look.primary, 7);
        assert_eq!((look.set, look.finish), (3, 0));
        assert!(!look.flag);
        assert_eq!(look.style, 4);
        assert!(rng.ints.is_empty());
    }
}
