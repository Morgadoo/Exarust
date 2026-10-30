//! The body tick's per-tick timers beside the ones its movement, balance
//! and combat stages already keep: the two adapters the original derives
//! from last tick's support at the top of the tick and moves with the
//! balance and the energy, the eased rate of the look point's follow, and
//! the two counters the tick steps (a countdown and the weapon switch).
//!
//! The adapters' original consumer is a part field nothing reads; they are
//! kept as the original's tested rules, not as a behaviour Exarust shows.
//! The look follow's rate is 0.0125 a tick standing, or rising 0.0025 a
//! tick to 0.15 while locomoting; Exarust's head look uses its own cursor
//! smoothing, so this rate is data until a caller reads it.
//!
//! (Included from body.rs: `#[path = "body_timers.rs"] pub mod timers;`)

/// The look follow's rate standing.
pub const LOOK_RATE_STANDING: f32 = 0.0125;
/// The rate rises this much a tick (in double) while locomoting, to
/// [`LOOK_RATE_MOVING`].
pub const LOOK_RATE_RISE: f64 = 0.0025;
pub const LOOK_RATE_MOVING: f32 = 0.15;
/// A get-up under this share of its clip (compared in double) holds the
/// rest adapter at one.
pub const GET_UP_HOLD_PROGRESS: f64 = 0.9;

/// What the body tick's timers read this tick.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TimerInput {
    /// Last tick's support S, 0..=1.
    pub support: f32,
    /// The balance B, 0..=1.
    pub balance: f32,
    /// Energy E, 0..=1.
    pub energy: f32,
    /// It locomotes this tick.
    pub moving: bool,
    /// It is getting up (the original's get-up flag; a reading of the
    /// body-feet notes, it could be the tangle flag).
    pub getting_up: bool,
    /// Its clip progress, 0..1.
    pub progress: f32,
}

/// The timers the body tick keeps from tick to tick.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BodyTimers {
    /// The rest adapter: 1 − 0.01·S, raised by 0.05·(1 − B) and then by
    /// 0.3·(1 − E), each capped at 1; one during a get-up's first nine
    /// tenths.
    pub rest: f32,
    /// The support adapter: 0.9 + 0.09·S, lowered by 0.6·(1 − E) and then
    /// by 0.1·(1 − B), each floored at 0.
    pub support: f32,
    /// The look follow's rate.
    pub look_rate: f32,
    /// A countdown the tick decrements every tick.
    pub countdown: i32,
    /// The weapon switch counter: the switch rules move it within its
    /// range, and the tick steps it one toward zero.
    pub weapon_switch: i32,
}

impl Default for BodyTimers {
    fn default() -> Self {
        Self {
            rest: 1.0,
            support: 0.9,
            look_rate: LOOK_RATE_STANDING,
            countdown: 0,
            weapon_switch: 0,
        }
    }
}

/// A dormant body's link-iteration count, set every body tick.
pub const DORMANT_LINK_ITERATIONS: i32 = 5;

/// One body tick of the root's link-iteration count
/// ([`crate::body::Ragdoll::link_iterations`]): a dormant statue or golem
/// holds [`DORMANT_LINK_ITERATIONS`]; any other living body climbs one a
/// tick toward its target and drops to it at once, the target being
/// `round((1 − 0.25·feral)·1660 / 83)` in single precision, rounded half
/// to even (`motile::cadence_target`): 20 for a body that is not feral,
/// 15 for a fully feral one.
///
/// The original's dormant branch also keeps a mark bit and raises a
/// scene-graph flag on the owning object depending on a word of the owner;
/// neither has a reader in Exarust and they are not modelled.
pub fn link_iteration_step(count: i32, dormant: bool, feral: f32) -> i32 {
    if dormant {
        return DORMANT_LINK_ITERATIONS;
    }
    let target = crate::motile::cadence_target(feral);
    if count < target {
        count + 1
    } else if count > target {
        target
    } else {
        count
    }
}

impl BodyTimers {
    /// One tick of the timers, as the body tick's first lines run them:
    /// each sum in double, narrowed once, then the processor's minimum
    /// (rest, against 1) or maximum (support, against 0), so a NaN gives
    /// the bound.
    pub fn tick(&mut self, input: &TimerInput) {
        let s = f64::from(input.support);
        let miss = |v: f32| f64::from(1.0 - v);
        let support = (s * 0.09 + 0.9) as f32;
        let rest = (1.0 - s * 0.01) as f32;
        let up = |v: f32| if v < 1.0 { v } else { 1.0 };
        let down = |v: f32| if v > 0.0 { v } else { 0.0 };
        let rest = up((f64::from(rest) + miss(input.balance) * 0.05) as f32);
        let mut rest = up((f64::from(rest) + miss(input.energy) * 0.3) as f32);
        let support = down((f64::from(support) - miss(input.energy) * 0.6) as f32);
        let support = down((f64::from(support) - miss(input.balance) * 0.1) as f32);
        if input.getting_up && f64::from(input.progress) < GET_UP_HOLD_PROGRESS {
            rest = 1.0;
        }
        // The original's mode bit that forces the standing rate is not
        // modelled; standing it is the same rate.
        self.look_rate = if input.moving {
            let next = (f64::from(self.look_rate) + LOOK_RATE_RISE) as f32;
            if f64::from(next) > 0.15 {
                LOOK_RATE_MOVING
            } else {
                next
            }
        } else {
            LOOK_RATE_STANDING
        };
        self.rest = rest;
        self.support = support;
        self.countdown = self.countdown.wrapping_sub(1);
        self.weapon_switch -= self.weapon_switch.signum();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input() -> TimerInput {
        TimerInput {
            support: 1.0,
            balance: 1.0,
            energy: 1.0,
            moving: false,
            getting_up: false,
            progress: 0.0,
        }
    }

    /// A supported, balanced, rested body: rest 1 − 0.01 = 0.99, support
    /// 0.9 + 0.09 = 0.99, the look rate 0.0125, the countdown down one and
    /// the weapon switch at rest.
    #[test]
    fn a_steady_body_keeps_its_adapters() {
        let mut t = BodyTimers::default();
        t.tick(&input());
        assert!((t.rest - 0.99).abs() < 1e-6, "{}", t.rest);
        assert!((t.support - 0.99).abs() < 1e-6, "{}", t.support);
        assert_eq!(t.look_rate, LOOK_RATE_STANDING);
        assert_eq!(t.countdown, -1);
        assert_eq!(t.weapon_switch, 0);
        let mut t = BodyTimers::default();
        t.tick(&TimerInput {
            support: 0.0,
            ..input()
        });
        assert_eq!((t.rest, t.support), (1.0, 0.9));
    }

    /// Half balance raises rest by 0.025 (0.99 → 1, capped) and lowers
    /// support by 0.05; half energy raises rest by 0.15 and lowers support
    /// by 0.3. No energy and no balance floor support at 0.99 − 0.6 − 0.1.
    #[test]
    fn imbalance_and_fatigue_move_the_adapters() {
        let mut t = BodyTimers::default();
        t.tick(&TimerInput {
            balance: 0.5,
            ..input()
        });
        assert_eq!(t.rest, 1.0);
        assert!((t.support - 0.94).abs() < 1e-6, "{}", t.support);
        let mut t = BodyTimers::default();
        t.tick(&TimerInput {
            support: 0.5,
            energy: 0.5,
            ..input()
        });
        assert_eq!(t.rest, 1.0);
        let support = 0.945 - 0.3;
        assert!((t.support - support).abs() < 1e-6, "{}", t.support);
        let mut t = BodyTimers::default();
        t.tick(&TimerInput {
            energy: 0.0,
            balance: 0.0,
            ..input()
        });
        assert!((t.support - 0.29).abs() < 1e-6, "{}", t.support);
        t.tick(&TimerInput {
            support: f32::NAN,
            ..input()
        });
        assert_eq!((t.rest, t.support), (1.0, 0.0));
    }

    /// A get-up under nine tenths of its clip holds rest at one; past that
    /// it does not. The look rate rises 0.0025 a tick moving, at most
    /// 0.15, and is 0.0125 standing. The weapon switch steps toward zero
    /// from either side.
    #[test]
    fn the_get_up_look_and_switch_timers() {
        let mut t = BodyTimers::default();
        t.tick(&TimerInput {
            getting_up: true,
            progress: 0.2,
            ..input()
        });
        assert_eq!(t.rest, 1.0);
        t.tick(&TimerInput {
            getting_up: true,
            progress: 0.95,
            ..input()
        });
        assert!((t.rest - 0.99).abs() < 1e-6, "{}", t.rest);
        let mut t = BodyTimers::default();
        t.tick(&TimerInput {
            moving: true,
            ..input()
        });
        assert!((t.look_rate - 0.015).abs() < 1e-6);
        for _ in 0..200 {
            t.tick(&TimerInput {
                moving: true,
                ..input()
            });
        }
        assert_eq!(t.look_rate, LOOK_RATE_MOVING);
        t.tick(&input());
        assert_eq!(t.look_rate, LOOK_RATE_STANDING);
        t.weapon_switch = 2;
        t.tick(&input());
        assert_eq!(t.weapon_switch, 1);
        t.weapon_switch = -3;
        t.tick(&input());
        assert_eq!(t.weapon_switch, -2);
    }

    /// The link-iteration ramp: dormant holds 5; otherwise one up a tick
    /// to the target (20 not feral, round(0.875·20) = 17.5 → 18 half-even
    /// at feral 0.5, 15 fully feral) and down to it at once.
    #[test]
    fn the_link_iterations_ramp_toward_the_feral_target() {
        assert_eq!(link_iteration_step(20, true, 0.0), 5);
        assert_eq!(link_iteration_step(5, false, 0.0), 6);
        assert_eq!(link_iteration_step(19, false, 0.0), 20);
        assert_eq!(link_iteration_step(20, false, 0.0), 20);
        assert_eq!(link_iteration_step(20, false, 0.5), 18);
        assert_eq!(link_iteration_step(17, false, 0.5), 18);
        assert_eq!(link_iteration_step(20, false, 1.0), 15);
        // A NaN feral level's target is 0 (the 64-bit indefinite's low
        // word), so the count drops to 0.
        assert_eq!(link_iteration_step(20, false, f32::NAN), 0);
    }
}
