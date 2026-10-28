//! The body tick's per-tick timers beside the ones its movement, balance
//! and combat stages already keep: the two size adapters the original
//! derives from the body scale and lowers while the body is tired or dazed
//! (`+0xaec` and `+0xaf0`), the eased speed of the look point's follow
//! (`+0xb64`), and the two countdowns the tick decrements every tick
//! (`+0xe98` and the weapon switch's `+0xf68`).
//!
//! The size adapters' original consumer is the part's fatigue field
//! (`part+0x598`), which nothing reads; they are kept as the original's
//! tested rules, not as a behaviour Exarust shows. The look follow's rate
//! is 0.0125 a tick standing and dead, or rising 0.0025 a tick to 0.15
//! while locomoting; Exarust's head look uses its own cursor smoothing, so
//! this rate is data until a caller reads it.
//!
//! (Included from body.rs: `#[path = "body_timers.rs"] pub mod timers;`)

/// The look follow's rate standing and dead (the original's `+0xb64`).
pub const LOOK_RATE_STANDING: f32 = 0.0125;
/// The rate rises this much a tick while locomoting, to [`LOOK_RATE_MOVING`].
pub const LOOK_RATE_RISE: f32 = 0.0025;
pub const LOOK_RATE_MOVING: f32 = 0.15;
/// A clip this far through its get-up keeps the height adapter at one.
pub const GET_UP_HEIGHT_PROGRESS: f32 = 0.9;

/// What the body tick's timers read this tick.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TimerInput {
    /// The body scale (the limbs' 1.25 and the size adapters' base).
    pub scale: f32,
    /// Muscle tone, 0..=1.
    pub tone: f32,
    /// Energy (daze), 0..=1.
    pub energy: f32,
    /// It locomotes this tick.
    pub moving: bool,
    /// Its condition is off its feet (the dead, down and dormant family).
    pub off_feet: bool,
    /// It is getting up, whose first nine tenths keep the height adapter
    /// at one (the original's flag at `+0x12f3`; a guess that the get-up
    /// is what it marks).
    pub getting_up: bool,
    /// Its clip progress, 0..1.
    pub progress: f32,
}

/// The timers the body tick keeps from tick to tick.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BodyTimers {
    /// The height adapter (the original's `+0xaec`): scale·0.09 + 0.9,
    /// lowered toward 1 by the energy and tone misses.
    pub height: f32,
    /// The speed adapter (`+0xaf0`): 1 − scale·0.01, lowered by the same
    /// misses, floor 0.
    pub speed: f32,
    /// The look follow's rate (`+0xb64`).
    pub look_rate: f32,
    /// The countdown at `+0xe98`, decremented every tick.
    pub countdown: i32,
    /// The weapon switch counter (`+0xf68`), clamped to its range by the
    /// switch rules and decremented here.
    pub weapon_switch: i32,
}

impl Default for BodyTimers {
    fn default() -> Self {
        Self {
            height: 0.9,
            speed: 1.0,
            look_rate: LOOK_RATE_STANDING,
            countdown: 0,
            weapon_switch: 0,
        }
    }
}

impl BodyTimers {
    /// One tick of the timers. The height adapter starts at scale·0.09 +
    /// 0.9 and the speed at 1 − scale·0.01; the height takes (1 − E)·0.05
    /// and then (1 − E)·0.3, each clamped up to 1, and the speed loses
    /// (1 − E)·0.6 then (1 − tone)·0.1, floored at 0. A get-up before nine
    /// tenths of its clip sets the height to 1. The look rate is 0.0125
    /// off the feet, 0.0125 standing and 0.0025 higher a tick moving, at
    /// most 0.15. The countdowns each lose one.
    pub fn tick(&mut self, input: &TimerInput) {
        let scale = f64::from(input.scale);
        let energy = f64::from(input.energy);
        let tone = f64::from(input.tone);
        let mut height = (scale * 0.09 + 0.9) as f32;
        let mut speed = (1.0 - scale * 0.01) as f32;
        let up = |value: f64| -> f32 {
            let v = value as f32;
            if v < 1.0 { v } else { 1.0 }
        };
        height = up(f64::from(height) + (1.0 - energy) * 0.05);
        height = up(f64::from(height) + (1.0 - energy) * 0.3);
        let down = |value: f64| -> f32 {
            let v = value as f32;
            if v > 0.0 { v } else { 0.0 }
        };
        speed = down(f64::from(speed) - (1.0 - energy) * 0.6);
        speed = down(f64::from(speed) - (1.0 - tone) * 0.1);
        if input.getting_up && input.progress < GET_UP_HEIGHT_PROGRESS {
            height = 1.0;
        }
        let look_rate = if input.off_feet {
            LOOK_RATE_STANDING
        } else if input.moving {
            let next = self.look_rate + LOOK_RATE_RISE;
            if next < LOOK_RATE_MOVING {
                next
            } else {
                LOOK_RATE_MOVING
            }
        } else {
            LOOK_RATE_STANDING
        };
        self.height = height;
        self.speed = speed;
        self.look_rate = look_rate;
        self.countdown = self.countdown.wrapping_sub(1);
        self.weapon_switch = self.weapon_switch.wrapping_sub(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input() -> TimerInput {
        TimerInput {
            scale: 1.0,
            tone: 1.0,
            energy: 1.0,
            moving: false,
            off_feet: false,
            getting_up: false,
            progress: 0.0,
        }
    }

    /// A fresh human at full tone and energy: height 0.09 + 0.9 = 0.99,
    /// speed 1 − 0.01 = 0.99, the look rate 0.0125, the countdowns down
    /// one.
    #[test]
    fn a_fresh_body_keeps_the_size_adapters() {
        let mut t = BodyTimers::default();
        t.tick(&input());
        assert!((t.height - 0.99).abs() < 1e-6, "{}", t.height);
        assert!((t.speed - 0.99).abs() < 1e-6, "{}", t.speed);
        assert_eq!(t.look_rate, LOOK_RATE_STANDING);
        assert_eq!(t.countdown, -1);
        assert_eq!(t.weapon_switch, -1);
    }

    /// At half energy the height takes 0.05 and 0.3 of the half (0.175 in
    /// all, capped at 1: 0.99 + 0.175 = 1) and the speed loses 0.3; at half
    /// tone the speed loses 0.05 more. The original's sums are double.
    #[test]
    fn a_tired_body_lowers_its_adapters() {
        let mut t = BodyTimers::default();
        t.tick(&TimerInput {
            energy: 0.5,
            tone: 1.0,
            ..input()
        });
        assert_eq!(t.height, 1.0);
        let speed = 0.99 - 0.5 * 0.6;
        assert!((t.speed - speed as f32).abs() < 1e-6, "{}", t.speed);
        let mut t = BodyTimers::default();
        t.tick(&TimerInput {
            energy: 0.5,
            tone: 0.5,
            ..input()
        });
        let speed = (0.99 - 0.3 - 0.05) as f32;
        assert!((t.speed - speed).abs() < 1e-6, "{}", t.speed);
        let mut t = BodyTimers::default();
        t.tick(&TimerInput {
            energy: 0.0,
            tone: 0.0,
            ..input()
        });
        assert_eq!(t.height, 1.0);
        assert!((t.speed - 0.29).abs() < 1e-6, "{}", t.speed);
    }

    /// A get-up under nine tenths of its clip holds the height at one;
    /// past that it does not. The look rate rises 0.0025 a tick moving, at
    /// most 0.15, and is 0.0125 off the feet or standing.
    #[test]
    fn the_get_up_and_look_timers() {
        let mut t = BodyTimers::default();
        t.tick(&TimerInput {
            scale: 0.5,
            getting_up: true,
            progress: 0.2,
            ..input()
        });
        assert_eq!(t.height, 1.0);
        let mut t = BodyTimers::default();
        t.tick(&TimerInput {
            scale: 0.5,
            getting_up: true,
            progress: 0.95,
            ..input()
        });
        assert!((t.height - 0.945).abs() < 1e-6, "{}", t.height);
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
        t.tick(&TimerInput {
            off_feet: true,
            ..input()
        });
        assert_eq!(t.look_rate, LOOK_RATE_STANDING);
    }
}
