//! A body's power ramp and the start of a power's clip.
//!
//! The body keeps one ramp that a power raises, one limit at which the
//! registered power callback is told to end (signal 0x80) and cleared, and
//! at most one such callback. Starting a power's clip first asks the callback
//! (signal 0x20: an answer of 0 or more leaves the cast for another tick),
//! otherwise forgets it, resets the effect flag, ends a clinch when the body
//! is parrying, starts the motion, and sets the casting condition 0x20.
//! Ending the callback while casting returns the body to condition 0 and
//! restarts its motion with a blend of 30.
//!
//! These reproduce the original's ramp and cast-start rules without a power
//! system to drive them; the tests pin the numbers. Evidence: the body's
//! ramp routines and the cast starter.

/// The blend of the motion restarted when a cast's callback ends.
pub const END_BLEND: u32 = 30;
/// The condition while casting.
pub const CASTING: u32 = 0x20;

/// A ramp raised by `amount` (the original's plain add).
pub fn raise(ramp: f32, amount: f32) -> f32 {
    ramp + amount
}

/// The result of a power's pulse.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Pulse {
    /// The raised ramp.
    pub ramp: f32,
    /// The callback is told to end (signal 0x80) and cleared.
    pub ends: bool,
}

/// A power's pulse of `amount`: raises the ramp, and ends the registered
/// callback when `amount` is more than what is left below the limit. A NaN
/// never ends it, and with no callback nothing does.
pub fn pulse(ramp: f32, limit: f32, amount: f32, has_callback: bool) -> Pulse {
    let ramp = ramp + amount;
    Pulse {
        ramp,
        ends: has_callback && amount > limit - ramp,
    }
}

/// What clearing the callback does: only a body casting (condition exactly
/// 0x20) restarts its motion, blended by [`END_BLEND`], and drops to
/// condition 0.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CastEnd {
    pub restarts_motion: bool,
    pub condition: u32,
}

/// Clears the callback of a body in `condition`.
pub fn end_cast(condition: u32) -> CastEnd {
    if condition == CASTING {
        CastEnd {
            restarts_motion: true,
            condition: 0,
        }
    } else {
        CastEnd {
            restarts_motion: false,
            condition,
        }
    }
}

/// What starting a power's clip does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CastStart {
    /// The cast began (the original returns true); false when the callback
    /// said to wait and nothing else changed.
    pub started: bool,
    /// The parry/clinch is ended before the clip.
    pub ends_clinch: bool,
    /// The motion starts and the casting condition 0x20 is set.
    pub casting: bool,
}

/// The cast start for a motion `motion`, `parrying` or not, and the answer
/// of the registered callback to signal 0x20 (`None` with no callback). An
/// answer of 0 or more waits. Otherwise a positive motion starts the clip
/// (ending a parry first) and sets condition 0x20; none only clears the
/// callback and its data.
pub fn cast_start(callback: Option<i32>, motion: i32, parrying: bool) -> CastStart {
    if matches!(callback, Some(answer) if answer >= 0) {
        return CastStart {
            started: false,
            ends_clinch: false,
            casting: false,
        };
    }
    CastStart {
        started: true,
        ends_clinch: motion > 0 && parrying,
        casting: motion > 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A pulse raises the ramp; an amount over what is left below the limit
    /// ends the callback, only when there is one.
    #[test]
    fn the_ramp_pulses_and_ends() {
        assert_eq!(raise(0.5, 0.25), 0.75);
        let p = pulse(0.0, 1.0, 0.25, true);
        assert_eq!(p.ramp, 0.25);
        assert!(!p.ends);
        // 0.25 against 1.0 − 0.5 = 0.5: not yet; against 1.0 − 0.9: ends.
        assert!(!pulse(0.25, 1.0, 0.25, true).ends);
        assert!(pulse(0.65, 1.0, 0.25, true).ends);
        assert!(!pulse(0.65, 1.0, 0.25, false).ends);
        assert!(!pulse(0.0, f32::NAN, 0.25, true).ends);
        // Exactly what is left does not end it.
        assert!(!pulse(0.5, 1.0, 0.25, true).ends);
    }

    /// Only a casting body restarts its motion when the callback ends.
    #[test]
    fn ending_a_cast_restarts_the_motion() {
        assert_eq!(
            end_cast(CASTING),
            CastEnd {
                restarts_motion: true,
                condition: 0
            }
        );
        assert_eq!(
            end_cast(8),
            CastEnd {
                restarts_motion: false,
                condition: 8
            }
        );
    }

    /// A callback answering 0 or more waits; otherwise a positive motion
    /// casts and ends a parry; none only starts nothing.
    #[test]
    fn a_cast_starts_the_clip_and_ends_a_parry() {
        assert_eq!(
            cast_start(None, 0x0E, true),
            CastStart {
                started: true,
                ends_clinch: true,
                casting: true
            }
        );
        assert!(!cast_start(None, 0, true).casting);
        assert!(!cast_start(None, 0x0E, false).ends_clinch);
        assert!(!cast_start(Some(0), 0x0E, true).started);
        assert!(!cast_start(Some(3), 0x0E, true).casting);
        assert!(cast_start(Some(-1), 0x0E, true).casting);
    }
}
