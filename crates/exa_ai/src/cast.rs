//! Choosing a power to cast, and the stop of a cast that goes down.
//!
//! A fighter has ten power slots, each with its own cooldown. A slot whose
//! cooldown is still running is skipped. An empty slot's cooldown is pushed
//! 30 s out. A power the power system refuses outright (its low byte is 0)
//! is put away for 30 s when the refusal says so; one whose low byte is
//! 1–3 is put away for 3 s but still weighed. A power answers a word: the
//! low byte is its weight, bits 8–15 its cost, [`USABLE`] marks it usable
//! and [`DENIED`] its refusal.
//!
//! The pool gives `round(pool · 200)`; the costliest usable power's cost is
//! taken off it, and a power whose own cost passes that budget is dropped.
//! Every power left keeps a flat 20 of weight and five times its low byte.
//! The last of them is not in the cumulative table the draw walks: it never
//! wins, though its flat 20 still widens the draw. A draw below the flat
//! part casts nothing; the rest walks the cumulative table. A chosen power
//! that carries under 11 of its own weight is put away for 6 s after the
//! cast.
//!
//! A body that goes down while casting stands in its guard facing the way
//! its body faces, and holds that heading; a body that was already lying
//! and is late in its fall keeps the pose it had.
//!
//! Evidence: the original's cast chooser and casting stop, read from the
//! instructions (draws on the engine's generator, the pool compared after
//! a round to even, the cumulative table's last entry left out).

use bevy::math::Vec3;

use crate::combat::{Body, CombatMind, GUARD, yaw};

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
/// The body word that stops casting (busy).
pub const CAST_BUSY: u32 = 0x10;
/// The body condition bit of a downed body.
pub const DOWN: u32 = 0x200;
/// The condition word of a body lying down.
pub const LYING: u32 = 0x300;
/// The clock past which a lying body keeps its pose.
pub const KEEP_POSE_CLOCK: f32 = 0.9;
/// The pose a downed caster holds, and the pose a late fall keeps.
pub const DOWN_POSE: i32 = 4;
pub const KEPT_POSE: i32 = 0;

/// The power system the chooser asks: a power's word, and the cast itself.
pub trait Powers {
    /// The word a power answers: [`USABLE`], [`DENIED`], its cost in bits
    /// 8–15 and its weight in bits 0–7.
    fn query(&mut self, power: u32) -> u32;
    /// Casts the power.
    fn cast(&mut self, power: u32);
}

/// Per-slot cooldowns and the cast in progress.
#[derive(Debug, Clone, PartialEq)]
pub struct Casting {
    /// Each slot's cooldown ends at this clock (ms).
    pub cooldowns: [u32; SLOTS],
    /// A cast is running.
    pub casting: bool,
    /// When it started (ms).
    pub started_ms: u32,
}

impl Default for Casting {
    fn default() -> Self {
        Self {
            cooldowns: [0; SLOTS],
            casting: false,
            started_ms: 0,
        }
    }
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
    if body.condition & CAST_BUSY != 0 {
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
    let budget = (body.power_pool * POOL_SCALE).round_ties_even() as i32
        - i32::try_from(most_cost).unwrap_or(i32::MAX);
    // The cumulative table is built over every power but the last, so the
    // last can never be drawn.
    let mut total = 0;
    let last = weighed.len() - 1;
    for candidate in &mut weighed[..last] {
        if i32::try_from(candidate.cost).unwrap_or(i32::MAX) <= budget {
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
    let target = drawn - flat;
    let mut at = 0;
    while at < weighed.len() - 1 && weighed[at].weight < target {
        at += 1;
    }
    let own = if at == 0 {
        weighed[0].weight
    } else {
        weighed[at].weight - weighed[at - 1].weight
    };
    let chosen = weighed[at];
    if own < LIGHT_WEIGHT {
        state.cooldowns[chosen.slot] = now.wrapping_add(CAST_MS);
    }
    powers.cast(slots[chosen.slot]);
    state.casting = true;
    state.started_ms = now;
    Some(Choice {
        slot: chosen.slot,
        power: slots[chosen.slot],
        weight: own,
    })
}

/// A caster that goes down: the pose it holds, and the heading it faces.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Stopped {
    /// The pose the body's base holds: 4 for a fresh down, 0 for a late
    /// lying fall [guess: what the field drives].
    pub pose: i32,
    /// The body motion the stop sets: the guard.
    pub motion: i32,
    /// The heading, aim and look, all the body's facing yaw.
    pub yaw: f32,
}

/// Stops a cast (and the tick) for a downed body: the guard motion, the
/// look, heading and aim set to the body's facing, and the caster's aim and
/// yaw follow. Returns `None` when the body is not down.
pub fn stop(mind: &mut CombatMind, body: &Body) -> Option<Stopped> {
    if body.condition & DOWN == 0 {
        return None;
    }
    let pose = if body.condition != LYING || body.clock <= KEEP_POSE_CLOCK {
        DOWN_POSE
    } else {
        KEPT_POSE
    };
    let facing = yaw(body.facing);
    mind.heading = facing;
    mind.aim = facing;
    mind.look_timer = 0.0;
    Some(Stopped {
        pose,
        motion: GUARD,
        yaw: facing,
    })
}

/// The direction a body faces, as the stop leaves it on the body's look.
pub fn facing_of(body: &Body) -> Vec3 {
    body.facing
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

    /// Slot 1 is a usable power of weight 5, slot 2 a usable power of
    /// weight 1; slot 3 is empty and slot 4 refused.
    fn table() -> Table {
        let mut answers = [0; SLOTS];
        answers[0] = USABLE | 5;
        answers[1] = USABLE | 1;
        answers[3] = DENIED;
        Table {
            answers,
            cast: None,
        }
    }

    /// A single weighed power is the last entry of the draw's table, so it
    /// never wins: the draw's flat part always covers the whole range.
    #[test]
    fn a_lone_power_is_never_cast() {
        let mut state = Casting::default();
        let mut slots = [0; SLOTS];
        slots[0] = 1;
        let mut powers = table();
        let mut roll = 0;
        for _ in 0..1000 {
            assert_eq!(
                choose(&mut state, &slots, &body(1.0, 0), &mut powers, &mut roll),
                None
            );
        }
        assert_eq!(powers.cast, None);
        assert!(!state.casting);
    }

    /// Two usable powers: the first is the only one the cumulative table
    /// holds, so every cast picks it (never the last). The draw's flat part
    /// (40 of 65) casts nothing.
    #[test]
    fn the_draw_casts_nothing_below_the_flat_share_and_never_the_last() {
        let mut state = Casting::default();
        let mut slots = [0; SLOTS];
        slots[0] = 1;
        slots[1] = 2;
        let mut powers = table();
        powers.answers[1] = USABLE | 5;
        let mut roll = 0;
        // Seed 0 draws 0: below the 40 of flat share.
        assert_eq!(
            choose(&mut state, &slots, &body(1.0, 0), &mut powers, &mut roll),
            None
        );
        assert_eq!(roll, 1);
        let mut casts = 0;
        for _ in 0..1000 {
            if let Some(choice) = choose(&mut state, &slots, &body(1.0, 0), &mut powers, &mut roll)
            {
                assert_eq!((choice.slot, choice.power, choice.weight), (0, 1, 25));
                casts += 1;
            }
            state.casting = false;
        }
        assert!(casts > 0);
    }

    /// An empty slot rests 30 s; a refused one also does when the power
    /// system says so; a weak power rests 3 s and still weighs.
    #[test]
    fn empty_refused_and_weak_powers_rest() {
        let mut state = Casting::default();
        let mut slots = [0; SLOTS];
        slots[0] = 1;
        slots[1] = 2;
        slots[2] = 3;
        let mut answers = [0; SLOTS];
        answers[0] = USABLE | 2;
        answers[2] = USABLE | 2;
        let mut powers = Table {
            answers,
            cast: None,
        };
        let mut roll = 0;
        // Slot 0 is the weak usable power, slot 1 has no power id, slot 2 a
        // refused power, slot 3 an empty one.
        let _ = choose(&mut state, &slots, &body(1.0, 100), &mut powers, &mut roll);
        assert_eq!(state.cooldowns[0], 100 + WEAK_MS);
        assert_eq!(state.cooldowns[1], 0);
        assert_eq!(state.cooldowns[2], 100 + WEAK_MS);
        assert_eq!(state.cooldowns[3], 100 + UNUSABLE_MS);
        // A weak usable power: 3 s of rest, and it still weighs.
        let mut state = Casting::default();
        let mut slots = [0; SLOTS];
        slots[0] = 1;
        let mut answers = [0; SLOTS];
        answers[0] = USABLE | 2;
        let mut powers = Table {
            answers,
            cast: None,
        };
        let mut roll = 0;
        for _ in 0..100 {
            let _ = choose(&mut state, &slots, &body(1.0, 0), &mut powers, &mut roll);
            state.casting = false;
        }
        assert_eq!(state.cooldowns[0], WEAK_MS);
    }

    /// The pool's budget drops the powers whose cost passes it; with every
    /// power dropped the draw is all flat and nothing casts.
    #[test]
    fn the_pool_budget_drops_the_costliest_powers() {
        let mut state = Casting::default();
        let mut slots = [0; SLOTS];
        slots[0] = 1;
        slots[1] = 2;
        let mut answers = [0; SLOTS];
        // Usable with cost 50, and a costly power of cost 10.
        answers[0] = USABLE | (50 << 8) | 5;
        answers[1] = (10 << 8) | 5;
        let mut powers = Table {
            answers,
            cast: None,
        };
        let mut roll = 0;
        // Pool 0.1 → 20; less the costliest usable cost 50 → budget −30.
        for _ in 0..100 {
            assert_eq!(
                choose(&mut state, &slots, &body(0.1, 0), &mut powers, &mut roll),
                None
            );
        }
        // Pool 1.0 → 200; budget 150 keeps both: the first weighs 25 of 65.
        let mut state = Casting::default();
        let mut casts = 0;
        for _ in 0..100 {
            if choose(&mut state, &slots, &body(1.0, 0), &mut powers, &mut roll).is_some() {
                casts += 1;
            }
            state.casting = false;
        }
        assert!(casts > 0);
    }

    /// A busy body asks for no power at all.
    #[test]
    fn a_busy_body_does_not_choose() {
        let mut state = Casting::default();
        let mut slots = [0; SLOTS];
        slots[0] = 1;
        let mut powers = table();
        let mut roll = 0;
        let mut me = body(1.0, 0);
        me.condition = CAST_BUSY;
        assert_eq!(
            choose(&mut state, &slots, &me, &mut powers, &mut roll),
            None
        );
        assert_eq!(roll, 0);
    }

    /// A downed caster holds the guard facing its body's facing, its aim and
    /// heading the same; a late lying fall keeps the old pose.
    #[test]
    fn a_fall_stops_the_cast_facing_the_body() {
        let mut mind = CombatMind {
            heading: 1.0,
            aim: 2.0,
            casting: true,
            ..CombatMind::default()
        };
        let me = Body {
            condition: DOWN,
            clock: 0.5,
            facing: Vec3::NEG_Z,
            ..Body::default()
        };
        let held = stop(&mut mind, &me).unwrap();
        assert_eq!((held.pose, held.motion), (DOWN_POSE, GUARD));
        assert_eq!(held.yaw, yaw(Vec3::NEG_Z));
        assert_eq!((mind.heading, mind.aim), (held.yaw, held.yaw));
        let lying = Body {
            condition: LYING,
            clock: 0.95,
            ..me
        };
        assert_eq!(stop(&mut mind, &lying).unwrap().pose, KEPT_POSE);
        let early = Body {
            clock: 0.9,
            ..lying
        };
        assert_eq!(stop(&mut mind, &early).unwrap().pose, DOWN_POSE);
        let standing = Body::default();
        assert_eq!(stop(&mut mind, &standing), None);
    }
}
