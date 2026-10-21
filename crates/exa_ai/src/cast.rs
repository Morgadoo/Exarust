//! Choosing a power to cast.
//!
//! A fighter has ten power slots, each with its own cooldown. A slot whose
//! cooldown is still running is skipped. An empty slot's cooldown is pushed
//! 30 s out. A power the power system refuses outright (its low byte is 0)
//! is put away for 30 s when the refusal says so; one whose low byte is
//! 1–3 is put away for 3 s but still weighed. A power answers a word: the
//! low byte is its weight, bits 8–15 its cost, [`USABLE`] marks it usable
//! and [`DENIED`] its refusal.
//!
//! The pool gives `round(pool · 200)` (ties to even); the costliest usable
//! power's cost is taken off it, and a power whose own cost passes that
//! budget weighs nothing. Every power weighed keeps a flat 20 of the draw
//! and five times its low byte. A draw below the flat part casts nothing;
//! the rest walks the cumulative table to the first power whose running
//! total is not below it (a dropped first power is taken by a draw of
//! exactly the flat part). A chosen power whose own share (its running
//! total less the entry before it) is under 11 puts away the slot whose
//! number is the chosen power's place in the table, not its own slot, for
//! 6 s after the cast.
//!
//! The combat state asks for a cast at its first place when the body is not
//! attacking and has a foe; a cast running for more than a second is torn
//! down once the power system is done with it ([`crate::combat::tick`]).
//!
//! Evidence: the original's cast chooser and casting stop, read from the
//! instructions (draws on the engine's generator, the pool compared after
//! a round to even, the walk's unsigned compare, the rest written by the
//! table's index).

use crate::combat::Body;

/// The power system's word: the power may be cast.
pub const USABLE: u32 = 0x0800_0000;
/// The power system's word: the power is refused; with no weight, it is put
/// away for 30 s.
pub const DENIED: u32 = 0x0400_0000;
/// Power slots a character has.
pub const SLOTS: usize = 10;
/// Cooldown of an empty slot, and of a refused power (ms).
pub const UNUSABLE_MS: u32 = 30_000;
/// Cooldown of a power too weak to weigh (ms).
pub const WEAK_MS: u32 = 3_000;
/// Cooldown after a cast whose power weighed under [`LIGHT_WEIGHT`] (ms).
pub const CAST_MS: u32 = 6_000;
/// A chosen power under this weight rests longer after its cast.
pub const LIGHT_WEIGHT: i32 = 11;
/// The pool is multiplied by this before it is rounded to a cost budget.
pub const POOL_SCALE: f32 = 200.0;
/// Each weighed power's flat share of the draw.
pub const FLAT_SHARE: i32 = 20;
/// A power's low byte is worth this much of the draw.
pub const WEIGHT_SCALE: i32 = 5;
/// A power whose low byte is under this is too weak to weigh now.
pub const WEAK_BELOW: u32 = 4;

/// The power system the chooser asks: a power's word, and the cast itself.
pub trait Powers {
    /// The word a power answers: [`USABLE`], [`DENIED`], its cost in bits
    /// 8–15 and its weight in bits 0–7.
    fn query(&mut self, power: u32) -> u32;
    /// Casts the power.
    fn cast(&mut self, power: u32);
}

/// The per-slot cooldowns (the cast in progress is the combat mind's).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Casting {
    /// Each slot's cooldown ends at this clock (ms).
    pub cooldowns: [u32; SLOTS],
}

/// A power that may be weighed: its slot, its weight and its cost.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Candidate {
    pub slot: usize,
    pub weight: i32,
    /// Its own cost, or 0 when the power is usable now.
    pub cost: u32,
}

/// What a cast chose.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Choice {
    pub slot: usize,
    pub power: u32,
    /// The power's own weight, for the shorter rest after a light cast.
    pub weight: i32,
}

/// The engine's draw below `n`.
fn draw(roll: &mut u32, n: i32) -> i32 {
    *roll = roll.wrapping_mul(0x0808_8405).wrapping_add(1);
    ((u64::from(*roll) * n as u64) >> 32) as i32
}

/// Weighs the power slots and casts one, or returns `None`. `slots` are the
/// record's power ids, `body` the caster's body.
pub fn choose(
    state: &mut Casting,
    slots: &[u32; SLOTS],
    body: &Body,
    powers: &mut dyn Powers,
    roll: &mut u32,
) -> Option<Choice> {
    if body.power_busy {
        return None;
    }
    let now = body.now_ms;
    let mut weighed: Vec<Candidate> = Vec::with_capacity(SLOTS);
    let mut most_cost = 0u32;
    for (slot, &power) in slots.iter().enumerate() {
        if state.cooldowns[slot] > now {
            continue;
        }
        if power == 0 {
            state.cooldowns[slot] = now.wrapping_add(UNUSABLE_MS);
            continue;
        }
        let answer = powers.query(power);
        if answer & USABLE != 0 {
            most_cost = most_cost.max((answer >> 8) & 0xff);
        }
        let weight = answer & 0xff;
        if weight == 0 {
            if answer & DENIED != 0 {
                state.cooldowns[slot] = now.wrapping_add(UNUSABLE_MS);
            }
            continue;
        }
        if weight < WEAK_BELOW {
            state.cooldowns[slot] = now.wrapping_add(WEAK_MS);
        }
        weighed.push(Candidate {
            slot,
            weight: (weight * WEIGHT_SCALE as u32) as i32,
            cost: if answer & USABLE != 0 {
                0
            } else {
                (answer >> 8) & 0xff
            },
        });
    }
    if weighed.is_empty() {
        return None;
    }
    // The pool converts to a 64-bit integer; the compare reads the low 32
    // bits of the difference.
    let budget = i64::from(
        ((body.power_pool * POOL_SCALE).round_ties_even() as i64).wrapping_sub(i64::from(most_cost))
            as i32,
    );
    // The cumulative table: a power whose cost passes the budget is zeroed
    // and adds nothing.
    let mut total = 0;
    for candidate in &mut weighed {
        if i64::from(candidate.cost) <= budget {
            total += candidate.weight;
            candidate.weight = total;
        } else {
            candidate.weight = 0;
        }
    }
    let flat = (weighed.len() as i32).wrapping_mul(FLAT_SHARE);
    let drawn = draw(roll, total.wrapping_add(flat));
    if drawn < flat {
        return None;
    }
    // The draw is under the total here, which the last kept power's running
    // total reaches, so the walk stops inside the table.
    let target = drawn - flat;
    let mut at = 0;
    while weighed[at].weight < target {
        at += 1;
    }
    let own = if at == 0 {
        weighed[0].weight
    } else {
        weighed[at].weight - weighed[at - 1].weight
    };
    let chosen = weighed[at];
    if own < LIGHT_WEIGHT {
        // The original writes the rest by the table's index.
        state.cooldowns[at] = now.wrapping_add(CAST_MS);
    }
    powers.cast(slots[chosen.slot]);
    Some(Choice {
        slot: chosen.slot,
        power: slots[chosen.slot],
        weight: own,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A power system that answers from a table and records the cast.
    struct Table {
        answers: [u32; SLOTS],
        cast: Option<u32>,
    }

    impl Powers for Table {
        fn query(&mut self, power: u32) -> u32 {
            self.answers[power as usize - 1]
        }

        fn cast(&mut self, power: u32) {
            self.cast = Some(power);
        }
    }

    fn body(pool: f32, now: u32) -> Body {
        Body {
            now_ms: now,
            power_pool: pool,
            ..Body::default()
        }
    }

    /// The generator state whose next draw below `n` is `want`.
    fn seed_for(n: i32, want: i32) -> u32 {
        (0..u32::MAX)
            .find(|&s| {
                let mut r = s;
                draw(&mut r, n) == want
            })
            .unwrap()
    }

    /// A lone usable power of weight 5 weighs 25 over a flat 20: a draw
    /// under 20 casts nothing, any other casts it.
    #[test]
    fn a_lone_power_is_cast_past_the_flat_share() {
        let mut slots = [0; SLOTS];
        slots[0] = 1;
        let mut answers = [0; SLOTS];
        answers[0] = USABLE | 5;
        let mut powers = Table {
            answers,
            cast: None,
        };
        let mut state = Casting::default();
        let mut roll = seed_for(45, 19);
        assert_eq!(
            choose(&mut state, &slots, &body(1.0, 0), &mut powers, &mut roll),
            None
        );
        let mut roll = seed_for(45, 20);
        let choice = choose(&mut state, &slots, &body(1.0, 0), &mut powers, &mut roll).unwrap();
        assert_eq!((choice.slot, choice.power, choice.weight), (0, 1, 25));
        assert_eq!(powers.cast, Some(1));
        // A share of 25 rests no longer.
        assert_eq!(state.cooldowns[0], 0);
    }

    /// Two powers of 25: the cumulative table holds 25 and 50 over a flat
    /// 40. The walk takes the first total not below the draw's rest, so a
    /// rest of 25 is still the first power and 26 the second.
    #[test]
    fn the_walk_takes_the_first_total_not_below_the_draw() {
        let mut slots = [0; SLOTS];
        slots[0] = 1;
        slots[1] = 2;
        let mut answers = [0; SLOTS];
        answers[0] = USABLE | 5;
        answers[1] = USABLE | 5;
        let mut powers = Table {
            answers,
            cast: None,
        };
        let mut state = Casting::default();
        let pick = |rest: i32, state: &mut Casting, powers: &mut Table| {
            let mut roll = seed_for(90, 40 + rest);
            choose(state, &slots, &body(1.0, 0), powers, &mut roll).map(|c| c.slot)
        };
        assert_eq!(pick(25, &mut state, &mut powers), Some(0));
        assert_eq!(pick(26, &mut state, &mut powers), Some(1));
        assert_eq!(pick(49, &mut state, &mut powers), Some(1));
    }

    /// A light power (share under 11) rests 6 s after its cast, written to
    /// the slot numbered by its place in the table: the power in slot 3 is
    /// the table's second entry, so slot 1 rests.
    #[test]
    fn a_light_cast_rests_the_slot_of_its_table_place() {
        let mut slots = [0; SLOTS];
        slots[0] = 1;
        slots[3] = 4;
        let mut answers = [0; SLOTS];
        answers[0] = USABLE | 4;
        answers[3] = USABLE | 2;
        let mut powers = Table {
            answers,
            cast: None,
        };
        let mut state = Casting::default();
        // Table 20 and 30 (shares 20 and 10), flat 40: a rest of 21 takes
        // the second.
        let mut roll = seed_for(70, 61);
        let choice = choose(&mut state, &slots, &body(1.0, 500), &mut powers, &mut roll).unwrap();
        assert_eq!((choice.slot, choice.power, choice.weight), (3, 4, 10));
        assert_eq!(state.cooldowns[1], 500 + CAST_MS);
        // Slot 3's own rest is the 3 s of a weak power (low byte under 4).
        assert_eq!(state.cooldowns[3], 500 + WEAK_MS);
    }

    /// An empty slot rests 30 s, a refused power with no weight also does,
    /// a weightless power without the refusal does not, and a weak power
    /// rests 3 s and still weighs.
    #[test]
    fn empty_refused_and_weak_powers_rest() {
        let mut state = Casting::default();
        let mut slots = [0; SLOTS];
        slots[0] = 1;
        slots[1] = 2;
        slots[2] = 3;
        let mut answers = [0; SLOTS];
        answers[0] = USABLE | 2;
        answers[2] = DENIED;
        let mut powers = Table {
            answers,
            cast: None,
        };
        let mut roll = 0;
        let _ = choose(&mut state, &slots, &body(1.0, 100), &mut powers, &mut roll);
        assert_eq!(state.cooldowns[0], 100 + WEAK_MS);
        assert_eq!(state.cooldowns[1], 0);
        assert_eq!(state.cooldowns[2], 100 + UNUSABLE_MS);
        assert_eq!(state.cooldowns[3], 100 + UNUSABLE_MS);
        // A slot still resting is not asked.
        let before = state.clone();
        let _ = choose(&mut state, &slots, &body(1.0, 200), &mut powers, &mut roll);
        assert_eq!(state.cooldowns[2], before.cooldowns[2]);
    }

    /// The pool's budget zeroes the powers whose cost passes it; with every
    /// power zeroed the draw is all flat and nothing casts.
    #[test]
    fn the_pool_budget_drops_the_costliest_powers() {
        let mut slots = [0; SLOTS];
        slots[0] = 1;
        slots[1] = 2;
        let mut answers = [0; SLOTS];
        // Usable with cost 50 (its own cost counts 0), and an unusable
        // power of cost 10.
        answers[0] = USABLE | (50 << 8) | 5;
        answers[1] = (10 << 8) | 5;
        let mut powers = Table {
            answers,
            cast: None,
        };
        // Pool 0.1 → 20; less the costliest usable cost 50 → budget −30:
        // the usable power (cost 0) is dropped too.
        let mut state = Casting::default();
        let mut roll = 0;
        for _ in 0..100 {
            assert_eq!(
                choose(&mut state, &slots, &body(0.1, 0), &mut powers, &mut roll),
                None
            );
        }
        // Pool 0.25 → 50: budget 0 keeps the usable power only (25 of 65).
        let mut roll = seed_for(65, 40 + 24);
        let choice = choose(&mut state, &slots, &body(0.25, 0), &mut powers, &mut roll);
        assert_eq!(choice.map(|c| c.slot), Some(0));
    }

    /// A body whose power word is busy asks for no power at all.
    #[test]
    fn a_busy_body_does_not_choose() {
        let mut state = Casting::default();
        let mut slots = [0; SLOTS];
        slots[0] = 1;
        let mut answers = [0; SLOTS];
        answers[0] = USABLE | 5;
        let mut powers = Table {
            answers,
            cast: None,
        };
        let mut roll = 0;
        let mut me = body(1.0, 0);
        me.power_busy = true;
        assert_eq!(
            choose(&mut state, &slots, &me, &mut powers, &mut roll),
            None
        );
        assert_eq!(roll, 0);
    }
}
