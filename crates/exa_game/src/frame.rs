//! The original's frame loop and platform pacing, as pure rule models.
//!
//! The Win32 message loop is replaced by winit and Bevy's schedule, and the
//! waitable-timer sleeps by the swap chain's present. What is left of the
//! original's frame tick is the arithmetic: how long a frame is held, how
//! game time is separated from real time, when the periodic timer table
//! fires, which deferred calls run again, how long an idle loop may wait and
//! how the logger presents a message. Those rules live here, away from any
//! window, and are pinned by tests. `exarust::frame_loop` applies the pacing
//! to the running app; everything else is the model the game layers use.
//!
//! Rules: `docs/PERFORMANCE.md` (frame pacing) and `docs/ARCHITECTURE.md`
//! (schedules).

use std::collections::VecDeque;
use std::time::Duration;

/// How long an unfocused window's frame is held: about 31 frames a second.
pub const UNFOCUSED_INTERVAL_MS: i32 = 32;
/// The refresh rate an automatic cap uses when the display reports less.
pub const AUTO_MIN_REFRESH_HZ: i32 = 120;
/// Milliseconds taken off a refresh-rate interval, leaving the loop just
/// inside the next vertical blank.
pub const VSYNC_SPARE_MS: i32 = 1;
/// The original's time multiplier for the periodic timer table: intervals
/// and elapsed times are in hundredths of its unit.
pub const TIMER_MULTIPLIER: u32 = 100;
/// Slots in the deferred-call ring.
pub const DEFERRED_SLOTS: usize = 128;
/// Entries in the log's status/history line.
pub const LOG_HISTORY: usize = 16;
/// Entries in the on-screen log overlay.
pub const LOG_OVERLAY: usize = 16;
/// How long an overlay line stays before its per-character time is added.
pub const LOG_OVERLAY_BASE_MS: u32 = 2000;
/// The extra an overlay line stays for each character.
pub const LOG_OVERLAY_PER_CHAR_MS: u32 = 50;

/// The frame-rate cap the original holds itself to.
///
/// A positive value is a fixed rate in frames a second, zero disables the
/// cap and a negative value is automatic: the display's refresh rate, or
/// [`AUTO_MIN_REFRESH_HZ`] when that is lower. `focused` is the window's
/// focus flag, `vsync` whether the swap waits for the display, and
/// `refresh_hz` the primary display's current rate. The interval is measured
/// from frame start to frame start with the display's full precision
/// rounded to whole milliseconds, as the original's conversion does
/// (nearest, ties to even, not a floor).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FramePacer {
    pub fps_cap: i32,
    pub focused: bool,
    pub vsync: bool,
    pub refresh_hz: u32,
}

impl FramePacer {
    /// The interval the loop aims for, or `None` while the cap is off.
    ///
    /// This is the original's general pacing rule: an unfocused window is
    /// held to [`UNFOCUSED_INTERVAL_MS`]; otherwise an automatic cap asks
    /// for the refresh rate (at least [`AUTO_MIN_REFRESH_HZ`]) when the swap
    /// does not wait, a fixed cap below the refresh rate asks for its own
    /// rate, and a cap at or above the refresh rate asks for the refresh
    /// rate. Vsync-enabled intervals come out
    /// [`VSYNC_SPARE_MS`] early.
    pub fn target_interval(&self) -> Option<u32> {
        if self.fps_cap == 0 {
            return None;
        }
        if !self.focused {
            return Some(UNFOCUSED_INTERVAL_MS as u32);
        }
        if self.refresh_hz == 0 {
            // Exarust guard: the original would divide by zero here.
            return None;
        }
        if self.vsync {
            if self.fps_cap < 0 || self.refresh_hz as i32 <= self.fps_cap {
                Some(
                    (interval_from_rate(self.refresh_hz as i32) as i32 - VSYNC_SPARE_MS).max(0)
                        as u32,
                )
            } else {
                Some((interval_from_rate(self.fps_cap) as i32 - 1).max(0) as u32)
            }
        } else {
            let rate = if self.fps_cap < 0 {
                (self.refresh_hz as i32).max(AUTO_MIN_REFRESH_HZ)
            } else {
                self.fps_cap
            };
            Some((interval_from_rate(rate) as i32 - 1).max(0) as u32)
        }
    }

    /// How long the tick sleeps at `now_ms` with the previous frame start at
    /// `last_ms`, in whole milliseconds; zero means no sleep.
    pub fn wait_ms(&self, now_ms: u32, last_ms: u32) -> u32 {
        let Some(target) = self.target_interval() else {
            return 0;
        };
        let elapsed = now_ms.wrapping_sub(last_ms);
        target.saturating_sub(elapsed)
    }
}

/// The interval of a fixed frame rate setting: whole milliseconds by
/// truncating integer division, one less than the rate's period.
///
/// The original's single-window fixed-rate path uses integer division here,
/// while [`FramePacer::target_interval`] converts the floating-point period;
/// 60 frames a second is 15 ms on this path and 16 on the other.
pub fn fixed_rate_interval(fps: i32) -> u32 {
    if fps < 1 {
        return 0;
    }
    (1000 / fps).max(0) as u32 - 1
}

/// A whole-millisecond interval from a frame rate, rounded to nearest with
/// ties to even.
fn interval_from_rate(rate: i32) -> u32 {
    if rate < 1 {
        return 0;
    }
    (1000.0f64 / rate as f64).round_ties_even() as u32
}

/// When the logger shows a message, from the level the engine passes with
/// it.
///
/// 1 is the status line's title, 2 and 3 write the status/history line, 4
/// and 5 the timed on-screen overlay, 6 an error message box and 7 a
/// warning one. Every message reaches the log file; level 8 is the system
/// information that is not presented, and so is any level below 1 or above
/// 8.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LogPresentation {
    StatusLine,
    Overlay,
    MessageBox,
    FileOnly,
}

/// The icon a message box carries.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MessageBoxIcon {
    Error,
    Warning,
}

impl LogPresentation {
    /// The presentation of a message at `level`.
    pub fn for_level(level: i32) -> Self {
        match level {
            1..=3 => Self::StatusLine,
            4 | 5 => Self::Overlay,
            6 | 7 => Self::MessageBox,
            _ => Self::FileOnly,
        }
    }

    /// The box icon for a message box level: 6 is the error icon, 7 the
    /// warning icon.
    pub fn icon(level: i32) -> Option<MessageBoxIcon> {
        match level {
            6 => Some(MessageBoxIcon::Error),
            7 => Some(MessageBoxIcon::Warning),
            _ => None,
        }
    }
}

/// How long one line of the on-screen overlay stays up: two seconds plus
/// fifty milliseconds for each character.
pub fn overlay_lifetime_ms(text: &str) -> u32 {
    LOG_OVERLAY_BASE_MS + LOG_OVERLAY_PER_CHAR_MS * text.chars().count() as u32
}

/// The redraw flags the original ORs from every view and the GUI root.
pub mod redraw {
    /// The frame has something new to show.
    pub const DRAW: u32 = 0x2;
    /// The frame must be presented (the original's swap).
    pub const PRESENT: u32 = 0x1;
    /// A short animation is running.
    pub const ANIMATION: u32 = 0x100;
    /// The interface is active (hover, caret, a held control).
    pub const ACTIVE: u32 = 0x200;
    /// Draw now, without waiting.
    pub const IMMEDIATE: u32 = 0x400;
}

/// How long the message loop may wait before the next frame, in
/// milliseconds; `None` means no wait.
///
/// The original picks the wait from the redraw flags, the focus state and
/// whether the window is minimised: an immediately redrawing frame waits not
/// at all, an active interface waits 1 ms while focused, an animation 32 ms
/// while focused, and an idle window 32 ms in the foreground or 1024 ms in
/// the background. A minimised window is never held for less than 32 ms.
pub fn idle_wait_ms(flags: u32, focused: bool, minimized: bool) -> Option<u32> {
    if flags & redraw::IMMEDIATE != 0 {
        return None;
    }
    let wait = if flags & redraw::ACTIVE != 0 {
        if focused { 1 } else { 32 }
    } else if flags & redraw::ANIMATION != 0 {
        if focused { 32 } else { 64 }
    } else if focused {
        32
    } else {
        1024
    };
    Some(if minimized { wait.max(32) } else { wait })
}

/// The frame and game clocks of one tick.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FrameStep {
    /// The real time this tick took.
    pub frame_ms: u32,
    /// The game time this tick advanced.
    pub game_delta_ms: u32,
    /// The game clock after the tick.
    pub game_ms: u32,
}

/// The original's frame and game clocks: real time from the millisecond
/// timer, game time with a paused-time accumulator subtracted.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FrameClock {
    last_ms: u32,
    paused_ms: u32,
    last_game_ms: u32,
}

impl FrameClock {
    /// Advances to `now_ms`.
    ///
    /// With a fixed frame-rate setting (`fixed_fps` above zero) the time the
    /// limiter waits is added to the paused accumulator, so game time
    /// advances by exactly one capped interval a frame. A paused game adds
    /// the whole frame. Both additions are signed: a frame shorter than the
    /// capped interval gives game time back.
    pub fn advance(&mut self, now_ms: u32, fixed_fps: i32, paused: bool) -> FrameStep {
        let elapsed = now_ms.wrapping_sub(self.last_ms);
        if fixed_fps > 0 {
            let interval = (1000 / fixed_fps) as i64;
            self.paused_ms = (self.paused_ms as i64 + elapsed as i64 - interval) as u32;
        } else if paused {
            self.paused_ms = self.paused_ms.wrapping_add(elapsed);
        }
        self.last_ms = now_ms;
        let game_ms = now_ms.wrapping_sub(self.paused_ms);
        let game_delta_ms = game_ms.wrapping_sub(self.last_game_ms);
        self.last_game_ms = game_ms;
        FrameStep {
            frame_ms: elapsed,
            game_delta_ms,
            game_ms,
        }
    }

    /// The game clock after the last tick.
    pub fn game_ms(&self) -> u32 {
        self.last_game_ms
    }

    /// The paused-time accumulator after the last tick.
    pub fn paused_ms(&self) -> u32 {
        self.paused_ms
    }
}

/// One record of the original's periodic timer table.
struct Timer {
    interval_ms: u32,
    due_ms: u32,
    free: bool,
    action: Box<dyn FnMut()>,
}

/// The periodic timer table: fixed intervals, a next-due time and a callback
/// for each record, re-armed from the current frame time so the table never
/// drifts.
#[derive(Default)]
pub struct PeriodicTimers {
    slots: Vec<Timer>,
    /// The multiplier applied to intervals and to reschedules, 100 by
    /// default (the original's time scale unit).
    pub multiplier: u32,
}

impl PeriodicTimers {
    /// An empty table at the original's multiplier.
    pub fn new() -> Self {
        Self {
            slots: Vec::new(),
            multiplier: TIMER_MULTIPLIER,
        }
    }

    /// Registers `action` to run every `interval_ms` from `now_ms`, taking
    /// the first free record or adding one. The returned id is the record's
    /// one-based position.
    pub fn add(&mut self, interval_ms: u32, now_ms: u32, action: Box<dyn FnMut()>) -> u32 {
        let slot = Timer {
            interval_ms,
            due_ms: self
                .multiplier
                .wrapping_mul(interval_ms)
                .wrapping_add(now_ms),
            free: false,
            action,
        };
        if let Some(index) = self.slots.iter().position(|t| t.free) {
            self.slots[index] = slot;
            index as u32 + 1
        } else {
            self.slots.push(slot);
            self.slots.len() as u32
        }
    }

    /// Frees the record `id`; a later [`PeriodicTimers::add`] may take it.
    pub fn free(&mut self, id: u32) {
        if let Some(slot) = id
            .checked_sub(1)
            .and_then(|i| self.slots.get_mut(i as usize))
        {
            slot.free = true;
        }
    }

    /// Moves a record's next run `elapsed_ms` closer, measured on its own
    /// interval: the next due time is `(interval - elapsed) * multiplier`
    /// from `now_ms`.
    pub fn reschedule(&mut self, id: u32, elapsed_ms: u32, now_ms: u32) {
        if let Some(slot) = id
            .checked_sub(1)
            .and_then(|i| self.slots.get_mut(i as usize))
        {
            slot.due_ms = slot
                .interval_ms
                .wrapping_sub(elapsed_ms)
                .wrapping_mul(self.multiplier)
                .wrapping_add(now_ms);
        }
    }

    /// Runs every record that is due at `now_ms`, in record order, and arms
    /// each one a full interval from now.
    ///
    /// Records added by a callback are not run in the same pass. The
    /// comparison is unsigned, as the original's is.
    pub fn run(&mut self, now_ms: u32) {
        for slot in &mut self.slots {
            if slot.free || now_ms < slot.due_ms {
                continue;
            }
            (slot.action)();
            slot.due_ms = self
                .multiplier
                .wrapping_mul(slot.interval_ms)
                .wrapping_add(now_ms);
        }
    }

    /// The number of records, free ones included.
    pub fn len(&self) -> usize {
        self.slots.len()
    }

    /// Whether the table has no records.
    pub fn is_empty(&self) -> bool {
        self.slots.is_empty()
    }
}

/// A deferred call: returns whether it wants to run again next frame.
type Deferred = Box<dyn FnMut() -> bool>;

/// The original's ring of calls that run again next frame until they return
/// false.
///
/// The ring holds [`DEFERRED_SLOTS`] calls. A push into a full ring is
/// dropped silently. Running calls each pending call once; the ones that
/// return true stay for the next frame in their original order, and newly
/// pushed calls follow them (a re-entrancy case the original resolves with
/// its ring compaction; this order is the equivalent for the engine's own
/// callers).
#[derive(Default)]
pub struct DeferredCalls {
    pending: VecDeque<Deferred>,
}

impl DeferredCalls {
    /// An empty ring.
    pub fn new() -> Self {
        Self::default()
    }

    /// Pushes a call for the next run; a full ring drops it.
    pub fn push(&mut self, call: Deferred) {
        if self.pending.len() < DEFERRED_SLOTS {
            self.pending.push_back(call);
        }
    }

    /// Runs every pending call once and keeps the ones that ask for another
    /// frame.
    pub fn run(&mut self) {
        let pending: Vec<Deferred> = self.pending.drain(..).collect();
        let mut kept = VecDeque::new();
        for mut call in pending {
            if call() {
                kept.push_back(call);
            }
        }
        // Calls pushed by the run itself follow the survivors.
        kept.append(&mut self.pending);
        self.pending = kept;
    }

    /// How many calls wait for the next run.
    pub fn len(&self) -> usize {
        self.pending.len()
    }

    /// Whether no call waits.
    pub fn is_empty(&self) -> bool {
        self.pending.is_empty()
    }
}

/// The original's waitable-timer sleep, in the forms an application can
/// take: a zero-duration sleep yields the processor, a duration sleeps to
/// the millisecond timer. Exarust uses [`std::thread::sleep`], the nearest
/// equivalent; the original's high-resolution timer and its one-millisecond
/// timer-period fallback have no separate Rust equivalent.
pub fn sleep_yield(ms: u32) {
    if ms == 0 {
        std::thread::yield_now();
    } else {
        std::thread::sleep(Duration::from_millis(ms as u64));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn focused(vsync: bool, refresh_hz: u32, fps_cap: i32) -> FramePacer {
        FramePacer {
            fps_cap,
            focused: true,
            vsync,
            refresh_hz,
        }
    }

    #[test]
    fn the_cap_picks_the_interval() {
        // No cap: the loop is not held.
        assert_eq!(focused(true, 144, 0).target_interval(), None);
        // A fixed rate below the refresh rate: floor-free conversion to the
        // nearest millisecond, 1000/60 = 16.67 -> 17, minus one.
        assert_eq!(focused(false, 144, 60).target_interval(), Some(16));
        // An automatic cap with no vsync holds the display's rate up to 120.
        assert_eq!(focused(false, 60, -1).target_interval(), Some(7));
        assert_eq!(focused(false, 165, -1).target_interval(), Some(5));
        // Vsync asks for the refresh rate, one millisecond early.
        assert_eq!(focused(true, 144, -1).target_interval(), Some(6));
        // A fixed cap at or above the refresh rate still waits for vsync.
        assert_eq!(focused(true, 144, 240).target_interval(), Some(6));
        assert_eq!(focused(true, 144, 60).target_interval(), Some(16));
        // An unfocused window is held whatever the cap says.
        for cap in [1, 60, -1, 1000] {
            let pacer = FramePacer {
                focused: false,
                ..focused(true, 144, cap)
            };
            assert_eq!(pacer.target_interval(), Some(32), "{cap}");
        }
    }

    #[test]
    fn the_fixed_rate_path_truncates() {
        // The single-window fixed-rate setting divides whole milliseconds:
        // 60 frames a second is 16 ms a frame, one less than its period.
        assert_eq!(fixed_rate_interval(60), 15);
        assert_eq!(fixed_rate_interval(83), 11);
        assert_eq!(fixed_rate_interval(144), 5);
        assert_eq!(fixed_rate_interval(0), 0);
    }

    #[test]
    fn the_tick_sleeps_only_while_ahead() {
        let pacer = focused(false, 144, 60);
        assert_eq!(pacer.wait_ms(1_000, 990), 6);
        assert_eq!(pacer.wait_ms(1_016, 1_000), 0);
        assert_eq!(pacer.wait_ms(1_030, 1_000), 0);
    }

    #[test]
    fn game_time_ignores_the_fixed_rate_wait() {
        let mut clock = FrameClock::default();
        // A free-running frame advances the game clock by its length.
        assert_eq!(clock.advance(30, 0, false).game_ms, 30);
        assert_eq!(clock.advance(55, 0, false).game_delta_ms, 25);
        // Paused, the same real time is held back.
        assert_eq!(clock.advance(80, 0, true).game_ms, 55);
        assert_eq!(clock.game_ms(), 55);
        // At 60 fixed frames the game advances by the 16 ms interval each
        // frame, whatever the real frame took: the credit is signed, so a
        // 21 ms frame holds 5 ms back and a 15 ms frame gives 1 ms of it
        // back.
        let mut clock = FrameClock::default();
        clock.advance(1_000, 60, false);
        assert_eq!(clock.paused_ms(), 984);
        let step = clock.advance(1_021, 60, false);
        assert_eq!((step.game_delta_ms, step.game_ms), (16, 32));
        let step = clock.advance(1_036, 60, false);
        assert_eq!((step.game_delta_ms, step.game_ms), (16, 48));
        assert_eq!(clock.paused_ms(), 988);
    }

    #[test]
    fn the_timer_table_rearms_from_the_frame_time() {
        use std::cell::RefCell;
        use std::rc::Rc;
        let count = Rc::new(RefCell::new(0u32));
        let mut timers = PeriodicTimers::new();
        let n = count.clone();
        let id = timers.add(
            20,
            1_000,
            Box::new(move || {
                *n.borrow_mut() += 1;
            }),
        );
        assert_eq!(id, 1);
        assert_eq!(timers.len(), 1);
        // 20 units at multiplier 100 fall due at 3 000 ms.
        timers.run(2_999);
        assert_eq!(*count.borrow(), 0);
        timers.run(3_000);
        assert_eq!(*count.borrow(), 1);
        // Re-armed from now, not from the old due time.
        timers.run(4_999);
        assert_eq!(*count.borrow(), 1);
        timers.run(5_000);
        assert_eq!(*count.borrow(), 2);
        // A reschedule moves the next run closer by the given elapsed time.
        timers.reschedule(id, 15, 5_000);
        timers.run(5_500);
        assert_eq!(*count.borrow(), 3);
        timers.free(id);
        timers.run(9_000);
        assert_eq!(*count.borrow(), 3);
        // A freed record is reused, keeping its one-based id.
        let again = timers.add(5, 9_000, Box::new(|| {}));
        assert_eq!(again, 1);
        assert_eq!(timers.len(), 1);
    }

    #[test]
    fn the_deferred_ring_drops_when_full_and_keeps_asking_calls() {
        let mut calls = DeferredCalls::new();
        for _ in 0..DEFERRED_SLOTS + 3 {
            calls.push(Box::new(|| false));
        }
        assert_eq!(calls.len(), DEFERRED_SLOTS);
        calls.run();
        assert!(calls.is_empty());
        let runs = std::rc::Rc::new(std::cell::Cell::new(0u32));
        let r = runs.clone();
        calls.push(Box::new(move || {
            r.set(r.get() + 1);
            r.get() < 3
        }));
        calls.run();
        assert_eq!(calls.len(), 1);
        calls.run();
        assert_eq!(calls.len(), 1);
        calls.run();
        assert_eq!(calls.len(), 0);
        assert_eq!(runs.get(), 3);
    }

    #[test]
    fn the_idle_wait_follows_the_redraw_flags() {
        use redraw::*;
        assert_eq!(idle_wait_ms(0, true, false), Some(32));
        assert_eq!(idle_wait_ms(0, false, false), Some(1024));
        assert_eq!(idle_wait_ms(ANIMATION, true, false), Some(32));
        assert_eq!(idle_wait_ms(ANIMATION, false, false), Some(64));
        assert_eq!(idle_wait_ms(ACTIVE, true, false), Some(1));
        assert_eq!(idle_wait_ms(ACTIVE, false, false), Some(32));
        assert_eq!(idle_wait_ms(IMMEDIATE, true, false), None);
        // A minimised window never waits less than 32 ms.
        assert_eq!(idle_wait_ms(ACTIVE, true, true), Some(32));
        assert_eq!(idle_wait_ms(0, false, true), Some(1024));
        // The flags can be combined; the strongest shortens the wait.
        assert_eq!(
            idle_wait_ms(DRAW | PRESENT | ANIMATION, true, false),
            Some(32)
        );
    }

    #[test]
    fn log_levels_pick_their_presentation() {
        assert_eq!(LogPresentation::for_level(1), LogPresentation::StatusLine);
        assert_eq!(LogPresentation::for_level(3), LogPresentation::StatusLine);
        assert_eq!(LogPresentation::for_level(4), LogPresentation::Overlay);
        assert_eq!(LogPresentation::for_level(5), LogPresentation::Overlay);
        assert_eq!(LogPresentation::for_level(6), LogPresentation::MessageBox);
        assert_eq!(LogPresentation::icon(6), Some(MessageBoxIcon::Error));
        assert_eq!(LogPresentation::icon(7), Some(MessageBoxIcon::Warning));
        assert_eq!(LogPresentation::for_level(8), LogPresentation::FileOnly);
        assert_eq!(LogPresentation::for_level(0), LogPresentation::FileOnly);
        assert_eq!(overlay_lifetime_ms("abcd"), 2_200);
        assert_eq!(overlay_lifetime_ms(""), 2_000);
    }
}
