//! Burning: the damage-over-time a fire particle puts on a body. Each tick
//! the burn's strength raises a timer by 0.03 of itself (up to 20, or takes
//! 0.1 off it when the strength is gone); the body's yellow health falls by
//! the timer × 0.00004, its red by (timer × 0.002)², and yellow is then
//! scaled by the red's new share of its old value. A timer over 1 makes a
//! sound once the clock is 800 ms past the one drawn after the last (a random
//! share of 800 ms past that sound); yellow under 0, after red has scaled it,
//! collapses the body (yellow and red 0). A red already near 0 takes all of
//! yellow. The body's condition bit 0x200 (down) and 0x800 (dying) stop
//! it.
//!
//! The original keeps these as the burning body's strength, timer and sound
//! clock, and the yellow and red flame levels; [`Burn`] holds the first three.
//! Evidence: the burning routine's assembly and the damage model notes.
//! Constants are the original's.

/// The timer gains this share of the strength a tick, up to 20; gone, it
/// loses 0.1.
const TIMER_GAIN: f64 = 0.03;
const TIMER_MOST: f32 = 20.0;
const TIMER_FALL: f64 = 0.1;
/// Yellow loses timer × this, red (timer × 0.002)².
const YELLOW_LOSS: f64 = 0.000_04;
const RED_LOSS: f64 = 0.002;
/// The red below which yellow is taken whole instead of scaled.
const RED_LEAST: f32 = 1.192_09e-7;
/// A sound waits this many ms past the clock the last one drew, which is
/// itself a random share of this many ms.
pub const BURN_SOUND_MS: u64 = 800;
/// The timer over which the burn crackles.
const SOUND_FROM: f32 = 1.0;

/// A body's burn.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Burn {
    /// The burn's strength this tick.
    pub strength: f32,
    /// The timer it builds.
    pub timer: f32,
    /// The next sound's earliest game clock.
    pub next_sound: u32,
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
/// sound cooldown's random draw (0..1). The sound's own volume and pitch
/// draws, and the body's staggering reset the original makes here, are not
/// reproduced.
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
        // The original works these in double and narrows each result.
        burn.timer += (f64::from(burn.strength) * TIMER_GAIN) as f32;
        if burn.timer > TIMER_MOST {
            burn.timer = TIMER_MOST;
        }
        let timer = f64::from(burn.timer);
        let mut yellow = (f64::from(yellow) - timer * YELLOW_LOSS) as f32;
        let old_red = red;
        let mut red = red;
        if burn.timer > SOUND_FROM {
            let loss = timer * RED_LOSS;
            red = (f64::from(red) - loss * loss) as f32;
            if u64::from(burn.next_sound) + BURN_SOUND_MS < now_ms {
                sound_at = Some(now_ms);
                burn.next_sound = (now_ms + (roll * BURN_SOUND_MS as f32) as u64) as u32;
            }
        }
        // The share of red left scales yellow, by the red the tick began
        // with: none worth dividing by takes all of it.
        if old_red > RED_LEAST {
            yellow = red / old_red * yellow;
        } else {
            yellow = 0.0;
        }
        let mut collapse = false;
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
    burn.timer = (f64::from(burn.timer) - TIMER_FALL) as f32;
    if burn.timer < 0.0 {
        burn.timer = 0.0;
    }
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
        let step = burn_tick(0, 1.0, 1.0, &mut b, 1000, 0.5).unwrap();
        assert_eq!(b.timer, 20.0);
        let red_loss = (20.0f32 * 0.002).powi(2);
        assert!((step.red - (1.0 - red_loss)).abs() < 1e-6);
        assert!(step.sound_at == Some(1000));
        assert_eq!(b.next_sound, 1000 + 400);
        // Too soon after the draw: 1400 + 800 is not yet past 2000.
        let mut quiet = Burn {
            strength: 1000.0,
            timer: 1.5,
            next_sound: 1400,
        };
        let step = burn_tick(0, 1.0, 1.0, &mut quiet, 2000, 0.5).unwrap();
        assert_eq!(step.sound_at, None);
        assert_eq!(quiet.next_sound, 1400);
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
        // A red the tick began with near 0 takes yellow whole, without a
        // collapse, even when the timer's loss already left yellow below 0.
        let mut b = Burn {
            strength: 1.0,
            timer: 19.0,
            ..Burn::default()
        };
        let step = burn_tick(0, 0.0001, 0.0, &mut b, 0, 0.0).unwrap();
        assert!(!step.collapse);
        assert_eq!(step.yellow, 0.0);
        let mut b = Burn::default();
        assert!(burn_tick(0x200, 1.0, 1.0, &mut b, 0, 0.0).is_none());
        assert!(burn_tick(0x800, 1.0, 1.0, &mut b, 0, 0.0).is_none());
    }
}
