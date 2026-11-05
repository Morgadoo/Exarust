//! The original's simulation step driver, as a clock with no thread of its
//! own.
//!
//! The original runs every scene's simulation on one thread at time-critical
//! priority. Each pass of that thread first runs the calls other threads
//! queued for it (a ring of 128 deferred calls), then looks for a
//! simulation whose next step is due on a millisecond clock. A due step
//! runs at once, so a thread that is behind steps back to back until it has
//! caught up. A step more than four steps late first moves the
//! simulation's base so that this step ends now: the backlog is dropped and
//! game time slows instead of spiralling. When nothing is due the thread
//! waits one millisecond less than the nearest step's wait (at most 100 ms)
//! and looks again. A paused driver sleeps 11 ms a pass and leaves the base
//! alone, so the first step after a pause finds itself late and drops the
//! pause's time the same way.
//!
//! The clock arithmetic is the original's: 32-bit milliseconds that wrap,
//! the step's due time compared unsigned with the time since the base, and
//! the lateness compared with four steps in 64 bits.
//!
//! [`StepDriver`] makes these decisions; whoever owns the simulation (the
//! frame loop today, the simulation thread once the fixed step has its own
//! world: OpenSpec `plan-the-simulation-thread`) runs the step it asks for.

/// Steps of lateness past which the driver drops its backlog.
pub const DROP_AFTER_STEPS: u32 = 4;
/// The longest the driver waits before looking at the clock again (ms).
pub const LONGEST_WAIT_MS: u32 = 100;
/// The paused driver's sleep a pass (ms).
pub const PAUSED_SLEEP_MS: u32 = 11;

/// What the driver asks of its thread.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Poll {
    /// Run one step now. `dropped_ms` is the backlog dropped first (zero
    /// when the step was on time or less than four steps late).
    Step { dropped_ms: u32 },
    /// Nothing is due: wait this many milliseconds, then poll again.
    Wait { ms: u32 },
}

/// One simulation's step clock.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StepDriver {
    /// The step (ms): 12 for the original's 83 Hz.
    pub step_ms: u32,
    /// Steps taken.
    pub ticks: u32,
    /// The simulation's time (ms): `ticks` steps, less nothing (the dropped
    /// backlog moves the base, not the time).
    pub time_ms: u32,
    /// The clock reading the simulation's time counts from.
    pub base_ms: u32,
    /// Keep the backlog however late the steps are (the original's switch
    /// per simulation; off in play).
    pub keep_backlog: bool,
}

impl StepDriver {
    /// A driver whose first step is due one step after `now_ms`.
    pub fn new(step_ms: u32, now_ms: u32) -> Self {
        Self {
            step_ms,
            ticks: 0,
            time_ms: 0,
            base_ms: now_ms,
            keep_backlog: false,
        }
    }

    /// Looks at the clock: either the next step is due (and, if it is more
    /// than four steps late, the base has moved so that it ends now), or
    /// the thread should wait.
    pub fn poll(&mut self, now_ms: u32) -> Poll {
        let due = self.time_ms.wrapping_add(self.step_ms);
        let elapsed = now_ms.wrapping_sub(self.base_ms);
        if elapsed >= due {
            let late = u64::from(elapsed) - u64::from(due);
            let mut dropped_ms = 0;
            if late > u64::from(self.step_ms << 2) && !self.keep_backlog {
                // The base moves by the lateness: this step ends now.
                self.base_ms = elapsed.wrapping_add(self.base_ms).wrapping_sub(due);
                dropped_ms = late as u32;
            }
            return Poll::Step { dropped_ms };
        }
        let left = (due - elapsed).min(LONGEST_WAIT_MS);
        // The original waits one millisecond less than the time left.
        Poll::Wait {
            ms: left.saturating_sub(1),
        }
    }

    /// Records the step the last [`StepDriver::poll`] asked for as taken.
    pub fn stepped(&mut self) {
        self.ticks = self.ticks.wrapping_add(1);
        self.time_ms = self.time_ms.wrapping_add(self.step_ms);
    }

    /// Runs every step due at `now_ms` through `step`, as the original's
    /// thread does when the step itself takes no time, and returns how
    /// many ran. A step that takes time is driven with [`StepDriver::poll`].
    pub fn catch_up(&mut self, now_ms: u32, mut step: impl FnMut(&Self)) -> u32 {
        let mut ran = 0;
        while let Poll::Step { .. } = self.poll(now_ms) {
            step(self);
            self.stepped();
            ran += 1;
        }
        ran
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_first_step_is_due_one_step_after_the_start() {
        let mut d = StepDriver::new(12, 1000);
        assert_eq!(d.poll(1000), Poll::Wait { ms: 11 });
        assert_eq!(d.poll(1011), Poll::Wait { ms: 0 });
        assert_eq!(d.poll(1012), Poll::Step { dropped_ms: 0 });
    }

    #[test]
    fn a_late_thread_steps_back_to_back() {
        let mut d = StepDriver::new(12, 0);
        // 47 ms late at most: three and a bit steps, none dropped.
        assert_eq!(d.catch_up(12 * 4 + 11, |_| {}), 4);
        assert_eq!(d.time_ms, 48);
        assert_eq!(d.base_ms, 0);
    }

    #[test]
    fn exactly_four_steps_late_keeps_the_backlog() {
        let mut d = StepDriver::new(12, 0);
        // Due at 12, now 60: 48 ms late, not more than four steps.
        assert_eq!(d.poll(60), Poll::Step { dropped_ms: 0 });
        assert_eq!(d.base_ms, 0);
    }

    #[test]
    fn more_than_four_steps_late_drops_the_backlog() {
        let mut d = StepDriver::new(12, 0);
        // Due at 12, now 61: 49 ms late. The base moves 49 ms on, so this
        // step ends now and no other is due.
        assert_eq!(d.poll(61), Poll::Step { dropped_ms: 49 });
        assert_eq!(d.base_ms, 49);
        d.stepped();
        assert_eq!(d.poll(61), Poll::Wait { ms: 11 });
    }

    #[test]
    fn the_wait_is_at_most_a_hundred_milliseconds() {
        let mut d = StepDriver::new(500, 0);
        assert_eq!(d.poll(0), Poll::Wait { ms: 99 });
    }

    #[test]
    fn keeping_the_backlog_never_moves_the_base() {
        let mut d = StepDriver::new(12, 0);
        d.keep_backlog = true;
        assert_eq!(d.catch_up(1200, |_| {}), 100);
        assert_eq!(d.base_ms, 0);
    }

    #[test]
    fn the_clock_wraps() {
        let start = u32::MAX - 5;
        let mut d = StepDriver::new(12, start);
        assert_eq!(d.poll(start.wrapping_add(12)), Poll::Step { dropped_ms: 0 });
        d.stepped();
        assert_eq!(d.poll(start.wrapping_add(13)), Poll::Wait { ms: 10 });
    }

    #[test]
    fn a_pause_is_dropped_by_the_first_step_after_it() {
        let mut d = StepDriver::new(12, 0);
        d.catch_up(12, |_| {});
        // Paused for a second: the base was left alone.
        assert_eq!(d.poll(1012), Poll::Step { dropped_ms: 988 });
        d.stepped();
        assert_eq!(d.poll(1012), Poll::Wait { ms: 11 });
    }
}
