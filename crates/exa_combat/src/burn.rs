//! Burning: the damage-over-time a fire particle puts on a body. Each tick
//! the burn's strength raises a timer by 0.03 of itself (up to 20, or takes
//! 0.1 off it when the strength is gone); the body's yellow health falls by
//! the timer × 0.00004, its red by (timer × 0.002)², and yellow is then
//! scaled by the red's new share of its old value. A timer over 1 makes a
//! sound every 800 ms at most; yellow under 0 collapses the body (yellow
//! and red 0). The body's condition bit 0x200 (down) and 0x800 (dying) stop
//! it.
//!
//! The original's fields are [`Burn`] here (`+0x10d4` strength, `+0x10dc`
//! timer, `+0x10d8` the sound's clock, `+0xd58`/`+0xd5c` yellow and red).
//! Evidence: the burning routine's assembly and the damage model notes.
//! Constants are the original's.

/// The timer gains this share of the strength a tick, up to 20; gone, it
/// loses 0.1.
const TIMER_GAIN: f32 = 0.03;
const TIMER_MOST: f32 = 20.0;
const TIMER_FALL: f32 = 0.1;
/// Yellow loses timer × this, red (timer × 0.002)².
const YELLOW_LOSS: f32 = 0.000_04;
const RED_LOSS: f32 = 0.002;
/// A sound after this many ms at most.
pub const BURN_SOUND_MS: u64 = 800;
/// The timer over which the burn crackles.
const SOUND_FROM: f32 = 1.0;

/// A body's burn.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Burn {
    /// The burn's strength this tick (`+0x10d4`).
    pub strength: f32,
    /// The timer it builds (`+0x10dc`).
    pub timer: f32,
    /// The next sound's earliest game clock (`+0x10d8`).
    pub next_sound: u64,
}

impl Default for Burn {
    fn default() -> Self {
        Self {
            strength: 0.0,
            timer: 0.0,
            next_sound: 0,
        }
    }
}

/// What one burning tick did: the body's new yellow and red, whether the
/// crackle sounds (its game clock), and whether the body collapses.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BurnStep {
    pub yellow: f32,
    pub red: f32,
    pub sound_at: Option<u64>,
    pub collapse: bool,
}

/// One burning tick of a body not off its feet (`condition` & 0xA00 clear),
/// with `yellow`, `red`, the burn and the game clock `now_ms`; `roll` is the
/// sound cooldown's random draw (0..1).
pub fn burn_tick(
    condition: u32,
    yellow: f32,
    red: f32,
    burn: &mut Burn,
    now_ms: u64,
    roll: f32,
) -> Option<BurnStep> {
    if condition & 0xa00 != 0 {
        return None;
    }
    let mut sound_at = None;
    if burn.strength > 0.0 {
        burn.timer += burn.strength * TIMER_GAIN;
        if burn.timer > TIMER_MOST {
            burn.timer = TIMER_MOST;
        }
        let mut yellow = yellow - burn.timer * YELLOW_LOSS;
        let old_red = red;
        let mut red = red;
        if burn.timer > SOUND_FROM {
            let loss = burn.timer * RED_LOSS;
            red -= loss * loss;
        }
        if burn.timer > SOUND_FROM && now_ms > burn.next_sound {
            sound_at = Some(now_ms);
            burn.next_sound = now_ms + (roll * BURN_SOUND_MS as f32) as u64;
        }
        let mut collapse = false;
        if yellow < 0.0 {
            yellow = 0.0;
            red = 0.0;
            collapse = true;
        } else if red <= 1.192_09e-7 {
            yellow = 0.0;
        } else {
            yellow *= red / old_red;
        }
        if yellow < 0.0 {
            yellow = 0.0;
            red = 0.0;
            collapse = true;
        }
        burn.strength = 0.0;
        return Some(BurnStep {
            yellow,
            red,
            sound_at,
            collapse,
        });
    }
    burn.timer = (burn.timer - TIMER_FALL).max(0.0);
    burn.strength = 0.0;
    Some(BurnStep {
        yellow,
        red,
        sound_at,
        collapse: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A strength of 1 raises the timer to 0.03 and takes 1.2e-6 off
    /// yellow; a timer over 1 takes (timer·0.002)² off red and scales
    /// yellow; no strength takes 0.1 off the timer.
    #[test]
    fn the_burn_is_the_originals() {
        let mut b = Burn {
            strength: 1.0,
            ..Burn::default()
        };
        let step = burn_tick(0, 0.9, 1.0, &mut b, 0, 0.0).unwrap();
        assert!((b.timer - 0.03).abs() < 1e-7);
        assert!((step.yellow - (0.9 - 0.03 * 0.000_04)).abs() < 1e-6);
        assert_eq!(step.red, 1.0);
        assert_eq!(b.strength, 0.0);
        let mut b = Burn {
            strength: 1000.0,
            timer: 1.5,
            ..Burn::default()
        };
        let step = burn_tick(0, 1.0, 1.0, &mut b, 100, 0.5).unwrap();
        assert_eq!(b.timer, 20.0);
        let red_loss = (20.0f32 * 0.002).powi(2);
        assert!((step.red - (1.0 - red_loss)).abs() < 1e-6);
        assert!(step.sound_at == Some(100));
        assert_eq!(b.next_sound, 100 + 400);
        let mut b = Burn {
            strength: 0.0,
            timer: 0.25,
            ..Burn::default()
        };
        let step = burn_tick(0, 1.0, 1.0, &mut b, 0, 0.0).unwrap();
        assert!((b.timer - 0.15).abs() < 1e-6);
        assert_eq!((step.yellow, step.red), (1.0, 1.0));
    }

    /// A yellow under 0 collapses (both 0); a down or dying body takes no
    /// burn.
    #[test]
    fn the_burn_collapses_and_stops() {
        let mut b = Burn {
            strength: 0.001,
            timer: 19.9,
            ..Burn::default()
        };
        let step = burn_tick(0, 0.0005, 1.0, &mut b, 0, 0.0).unwrap();
        assert!(step.collapse);
        assert_eq!((step.yellow, step.red), (0.0, 0.0));
        let mut b = Burn::default();
        assert!(burn_tick(0x200, 1.0, 1.0, &mut b, 0, 0.0).is_none());
        assert!(burn_tick(0x800, 1.0, 1.0, &mut b, 0, 0.0).is_none());
    }
}
