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
    /// The weapon switch timer (the original's `0x444`): 5 with an item in
    /// the hands, else moved one a tick toward the record field's cadence
    /// target (`motile::cadence_target`).
    pub weapon_switch: i32,
    /// The weapon switch's flags (the original's `0x109C`): bit 8 is set
    /// while the hands are empty and the state flag is to be raised.
    pub switch_flags: u32,
}

impl Default for BodyTimers {
    fn default() -> Self {
        Self {
            rest: 1.0,
            support: 0.9,
            look_rate: LOOK_RATE_STANDING,
            countdown: 0,
            weapon_switch: 0,
            switch_flags: 0,
        }
    }
}

/// One tick of the weapon switch block. With an item in the hands
/// (`hands`, the low 24 bits of the held word) the timer is 5; empty, it
/// is 5 and bit 8 of the flags is set, raising the state flag unless
/// `state_flag4` is already set. Otherwise (flags 6 clear) the timer moves
/// one toward the step/breath cadence target of the character record's
/// field (`motile::cadence_target`, the original's own formula) and bit 8
/// clears. Returns the timer, the flags and whether the state flag is to
/// be raised.
pub fn weapon_switch_step(
    timer: i32,
    flags: u32,
    hands: u32,
    state_flag4: bool,
    field: f32,
) -> (i32, u32, bool) {
    if flags & 6 != 0 {
        if hands != 0 {
            (5, flags & !8, false)
        } else if state_flag4 {
            (5, flags | 8, false)
        } else {
            (5, flags | 8, true)
        }
    } else {
        let target = crate::motile::cadence_target(field);
        let timer = if timer < target {
            timer + 1
        } else if timer > target {
            target
        } else {
            timer
        };
        (timer, flags & !8, false)
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
    }

    /// Runs the weapon switch block, which the posture stage calls when it
    /// knows the hands and the record's field.
    pub fn tick_switch(&mut self, hands: u32, state_flag4: bool, field: f32) -> bool {
        let (timer, flags, raise) = weapon_switch_step(
            self.weapon_switch,
            self.switch_flags,
            hands,
            state_flag4,
            field,
        );
        self.weapon_switch = timer;
        self.switch_flags = flags;
        raise
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
    }

    /// The weapon switch block: an item of 5; empty with the flags 6 clear
    /// moves toward the target and clears bit 8; empty with a flag set
    /// raises the state flag once.
    #[test]
    fn the_weapon_switch_timer() {
        assert_eq!(crate::motile::cadence_target(0.0), 20);
        assert_eq!(crate::motile::cadence_target(1.0), 15);
        assert_eq!(crate::motile::cadence_target(0.5), 18);
        assert_eq!(weapon_switch_step(10, 0, 0x30, false, 0.0), (11, 0, false));
        assert_eq!(weapon_switch_step(20, 0, 0, false, 0.0), (20, 0, false));
        assert_eq!(weapon_switch_step(25, 0, 0, false, 0.0), (20, 0, false));
        assert_eq!(weapon_switch_step(10, 0xa, 0x30, false, 1.0), (5, 2, false));
        assert_eq!(weapon_switch_step(10, 0xa, 0, false, 1.0), (5, 0xa, true));
        assert_eq!(weapon_switch_step(10, 0xa, 0, true, 1.0), (5, 0xa, false));
        let mut t = BodyTimers {
            weapon_switch: 18,
            switch_flags: 8,
            ..BodyTimers::default()
        };
        assert!(!t.tick_switch(0, false, 0.5));
        assert_eq!(t.weapon_switch, 18);
        assert_eq!(t.switch_flags, 0);
    }
}
