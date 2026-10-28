//! The original's frame loop and platform pacing, as pure rule models.
//!
//! The Win32 message loop is replaced by winit and Bevy's schedule, and the
//! waitable-timer sleeps by the swap chain's present. What is left of the
//! original's frame tick is the arithmetic: how long a frame is held, how
//! game time is separated from real time, when the periodic timer table
//! fires, which deferred calls run again, how long the message loop may
//! wait and how the logger presents a message. Those rules live here, away
//! from any window, and are pinned by tests. `exarust::frame_loop` applies
//! the pacing to the running app; everything else is the model the game
//! layers use.
//!
//! Rules: `docs/PERFORMANCE.md` (Frame pacing).

use std::time::Duration;

/// How long an unfocused window's frame is held: about 31 frames a second.
pub const UNFOCUSED_INTERVAL_MS: i32 = 32;
/// The refresh rate an automatic cap uses without vertical sync when the
/// display reports less (or no rate at all).
pub const AUTO_MIN_REFRESH_HZ: i32 = 120;
/// Milliseconds taken off the display's period when vertical sync paces the
/// frame, leaving the loop just inside the next vertical blank.
pub const VSYNC_SPARE_MS: i32 = 1;
/// The original's time multiplier for the periodic timer table: intervals
/// are scaled by it before they are added to the frame time.
pub const TIMER_MULTIPLIER: u32 = 100;
/// Slots in the deferred-call ring; one is always left empty, so it holds
/// at most 127 calls.
pub const DEFERRED_SLOTS: usize = 128;
/// Entries in the log's status/history line.
pub const LOG_HISTORY: usize = 16;
/// Slots in the on-screen log overlay's ring; one is always left empty, so
/// at most fifteen lines show.
pub const LOG_OVERLAY_SLOTS: usize = 16;
/// How long an overlay line stays before its per-character time is added.
pub const LOG_OVERLAY_BASE_MS: u32 = 2000;
/// The extra an overlay line stays for each byte of its text.
pub const LOG_OVERLAY_PER_CHAR_MS: u32 = 50;
/// What joins an overlay line's title and message.
pub const LOG_OVERLAY_SEPARATOR: &str = " : ";

/// The processor's double-to-integer conversion under the default rounding
/// mode: nearest, ties to even, and the "integer indefinite" value for a
/// NaN or a result out of range.
fn convert_rounded(value: f64) -> i64 {
    if value.is_nan()
        || value >= 9.223_372_036_854_775_808e18
        || value < -9.223_372_036_854_775_808e18
    {
        i64::MIN
    } else {
        value.round_ties_even() as i64
    }
}

/// `1000 / rate` in double precision, converted by [`convert_rounded`]: a
/// rate of zero divides to infinity and converts to the indefinite value.
fn period_ms(rate: i32) -> i64 {
    convert_rounded(1000.0 / f64::from(rate))
}

/// The frame-rate cap the original holds itself to, from its `FPSLimit`
/// setting.
///
/// A positive value is a frame rate, zero disables the cap and a negative
/// value (the default, −1) is automatic. `focused` is the window's focus
/// flag, `vsync` the vertical-sync setting and `refresh_hz` the primary
/// display's current rate, 0 when unknown (below 30 Hz, see
/// `exa_game::window::display_refresh_hz`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FramePacer {
    pub fps_cap: i32,
    pub focused: bool,
    pub vsync: bool,
    pub refresh_hz: u32,
}

impl FramePacer {
    /// The interval the loop aims for in whole milliseconds, or `None` while
    /// the cap is off. A negative interval never holds a frame.
    ///
    /// An unfocused window is held to [`UNFOCUSED_INTERVAL_MS`]. With
    /// vertical sync an automatic cap, or a cap at or above the refresh
    /// rate, asks for the display's period less [`VSYNC_SPARE_MS`]; a cap
    /// below it for its own period less one. Without vertical sync an
    /// automatic cap asks for the refresh rate's period, at least
    /// [`AUTO_MIN_REFRESH_HZ`] (the comparison is signed, so an unknown rate
    /// of 0 gives 120), and a fixed cap for its own period, each less one.
    /// Every period is `1000 / rate` in double precision rounded to nearest,
    /// ties to even. An unknown rate under vertical sync divides by zero:
    /// the conversion's indefinite value less one keeps its low 32 bits,
    /// −1, and the frame is not held.
    pub fn target_interval(&self) -> Option<i32> {
        if self.fps_cap == 0 {
            return None;
        }
        if !self.focused {
            return Some(UNFOCUSED_INTERVAL_MS);
        }
        let refresh = self.refresh_hz as i32;
        let interval = if self.vsync {
            if self.fps_cap < 0 || refresh <= self.fps_cap {
                period_ms(refresh).wrapping_sub(i64::from(VSYNC_SPARE_MS))
            } else {
                period_ms(self.fps_cap).wrapping_sub(1)
            }
        } else if self.fps_cap < 0 {
            period_ms(refresh.max(AUTO_MIN_REFRESH_HZ)).wrapping_sub(1)
        } else {
            period_ms(self.fps_cap).wrapping_sub(1)
        };
        // The interval is stored as a 32-bit integer.
        Some(interval as i32)
    }

    /// How long the tick sleeps at `now_ms` with the previous frame start at
    /// `last_ms`, in whole milliseconds; zero means no sleep.
    ///
    /// The elapsed time is the difference of the two clock readings taken as
    /// 64-bit values, compared signed with the interval.
    pub fn wait_ms(&self, now_ms: u32, last_ms: u32) -> u32 {
        let Some(target) = self.target_interval() else {
            return 0;
        };
        let elapsed = i64::from(now_ms) - i64::from(last_ms);
        if elapsed < i64::from(target) {
            (target as u32).wrapping_sub(now_ms.wrapping_sub(last_ms))
        } else {
            0
        }
    }
}

/// The interval of the frame tick's fixed-rate path: `1000 / fps` by
/// truncating integer division, less one, as a 32-bit value (so a rate
/// above 1000 gives −1 and never holds a frame).
///
/// The tick takes this path, instead of [`FramePacer`], when its fixed-rate
/// variable is positive and the game is not paused. Nothing in the shipped
/// executable writes that variable, so the path is never taken in play; it
/// is kept for the clock model ([`FrameClock::advance`]). 60 frames a
/// second is 15 ms here and 16 ms through [`FramePacer`].
pub fn fixed_rate_interval(fps: i32) -> Option<i32> {
    if fps < 1 {
        return None;
    }
    Some(1000 / fps - 1)
}

/// What the message loop does after a tick: wait for input up to
/// `wait_ms`, or run the next tick at once.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IdleWait {
    /// The wait's time-out, in milliseconds. The original keeps it between
    /// ticks: a tick that does not choose one keeps the last.
    pub wait_ms: u32,
    /// Whether the loop waits at all.
    pub waits: bool,
}

/// The redraw flags the original ORs from every view and the GUI root.
pub mod redraw {
    /// The frame must be presented (the original's swap).
    pub const PRESENT: u32 = 0x1;
    /// The frame has something new to show.
    pub const DRAW: u32 = 0x2;
    /// A short animation is running.
    pub const ANIMATION: u32 = 0x100;
    /// The interface is active (hover, caret, a held control).
    pub const ACTIVE: u32 = 0x200;
    /// Draw now, without waiting.
    pub const IMMEDIATE: u32 = 0x400;
    /// What the tick forces when its "always redraw" switch is set.
    pub const FORCED: u32 = 0x447;

    /// Asks for a draw and a present, as every changed GUI control does.
    pub fn request(flags: &mut u32) {
        *flags |= DRAW | PRESENT;
    }

    /// The flags after the tick's promotion: bits 4–7 also set bits 8–11,
    /// so a view's 0x10, 0x20 and 0x40 count as an animation, an active
    /// interface and an immediate redraw.
    pub fn promote(flags: u32) -> u32 {
        ((flags & 0xf0) << 4) | flags
    }
}

/// How long the message loop may wait after a tick.
///
/// The flags are promoted first ([`redraw::promote`]). An immediate redraw
/// does not wait and leaves the time-out as it was (`previous_ms`); an
/// active interface waits 1 ms while focused and 32 ms otherwise; an
/// animation 32 ms while focused and 64 ms otherwise; and an idle window
/// 32 ms in the foreground or 1024 ms in the background. A minimised window
/// always waits, at least 32 ms, even after an immediate redraw, and its
/// frame is not drawn ([`draws`]).
pub fn idle_wait(flags: u32, focused: bool, minimized: bool, previous_ms: u32) -> IdleWait {
    let flags = redraw::promote(flags);
    let mut result = if flags & redraw::IMMEDIATE != 0 {
        IdleWait {
            wait_ms: previous_ms,
            waits: false,
        }
    } else {
        let wait_ms = if flags & redraw::ACTIVE != 0 {
            if focused { 1 } else { 32 }
        } else if flags & redraw::ANIMATION != 0 {
            if focused { 32 } else { 64 }
        } else if focused {
            32
        } else {
            1024
        };
        IdleWait {
            wait_ms,
            waits: true,
        }
    };
    if minimized {
        // The comparison is signed; the time-outs are all small.
        if (result.wait_ms as i32) < 32 {
            result.wait_ms = 32;
        }
        result.waits = true;
    }
    result
}

/// Whether a tick with `flags` draws its frame: the draw flag, which a
/// minimised window clears.
pub fn draws(flags: u32, minimized: bool) -> bool {
    !minimized && flags & redraw::DRAW != 0
}

/// When the logger shows a message, from the level the engine passes with
/// it.
///
/// 1 to 3 write the status/history line, 4 and 5 the timed on-screen
/// overlay, 6 an error message box and 7 a warning one; any other level is
/// only written to the log file.
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

    /// Whether a message at `level` is written to the log file: every level
    /// but 5, an overlay-only message. (The original also skips the file
    /// while it logs its system information block.)
    pub fn writes_file(level: i32) -> bool {
        level != 5
    }

    /// The status-line kind a level stores with its line: 0 for level 1,
    /// 1 for levels 2 and 3.
    pub fn status_kind(level: i32) -> Option<u8> {
        match level {
            1 => Some(0),
            2 | 3 => Some(1),
            _ => None,
        }
    }
}

/// The text of one overlay line: the title, `" : "` and the message.
pub fn overlay_text(title: &str, message: &str) -> String {
    format!("{title}{LOG_OVERLAY_SEPARATOR}{message}")
}

/// How long one overlay line stays up: two seconds plus fifty milliseconds
/// for each byte of its text (the original's single-byte strings).
pub fn overlay_lifetime_ms(text: &str) -> u32 {
    LOG_OVERLAY_BASE_MS.wrapping_add(LOG_OVERLAY_PER_CHAR_MS.wrapping_mul(text.len() as u32))
}

/// One line of the status/history display.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StatusEntry {
    /// The kind the level gave the line ([`LogPresentation::status_kind`]).
    pub kind: u8,
    /// The message (the title is not stored).
    pub text: String,
}

/// The status/history line's newest-first list of at most sixteen entries.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StatusHistory {
    entries: Vec<StatusEntry>,
}

impl StatusHistory {
    /// Adds a message at the front, dropping the oldest past sixteen.
    pub fn push(&mut self, kind: u8, text: impl Into<String>) {
        self.entries.insert(
            0,
            StatusEntry {
                kind,
                text: text.into(),
            },
        );
        self.entries.truncate(LOG_HISTORY);
    }

    /// The entries, newest first.
    pub fn entries(&self) -> &[StatusEntry] {
        &self.entries
    }
}

/// One line of the timed on-screen log overlay.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OverlayLine {
    pub text: String,
    /// The game time the line goes away; it stays while `now <= expiry`.
    pub expires_ms: u32,
}

/// The timed on-screen log overlay: a ring of sixteen slots holding at most
/// fifteen lines, oldest first.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LogOverlay {
    lines: std::collections::VecDeque<OverlayLine>,
}

impl LogOverlay {
    /// Adds the line `title : message` if the ring has room and returns
    /// whether it did; its expiry is measured from the game time `now_ms`.
    /// The original requests a redraw either way.
    pub fn push(&mut self, title: &str, message: &str, now_ms: u32) -> bool {
        if self.lines.len() + 1 >= LOG_OVERLAY_SLOTS {
            return false;
        }
        let text = overlay_text(title, message);
        self.lines.push_back(OverlayLine {
            expires_ms: now_ms.wrapping_add(overlay_lifetime_ms(&text)),
            text,
        });
        true
    }

    /// Drops the oldest line when its expiry is strictly before the game
    /// time `now_ms` and returns whether it went; the original requests a
    /// redraw when one does. At most one line goes per tick.
    pub fn expire(&mut self, now_ms: u32) -> bool {
        match self.lines.front() {
            Some(front) if front.expires_ms < now_ms => {
                self.lines.pop_front();
                true
            }
            _ => false,
        }
    }

    /// The live lines, oldest first.
    pub fn lines(&self) -> impl Iterator<Item = &OverlayLine> {
        self.lines.iter()
    }

    /// How many lines wait.
    pub fn len(&self) -> usize {
        self.lines.len()
    }

    /// Whether the overlay is empty.
    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }
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
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FrameClock {
    last_ms: u32,
    paused_ms: u32,
    last_game_ms: u32,
    /// Whether a paused game holds its clock (the original's switch, on by
    /// default).
    pub pause_holds_time: bool,
}

impl Default for FrameClock {
    fn default() -> Self {
        Self {
            last_ms: 0,
            paused_ms: 0,
            last_game_ms: 0,
            pause_holds_time: true,
        }
    }
}

impl FrameClock {
    /// Advances to `now_ms`, the clock read after the tick's pacing sleep.
    ///
    /// While the game is paused (`paused`, the pause counter above zero) the
    /// whole frame is added to the paused accumulator when
    /// [`FrameClock::pause_holds_time`] is set. Otherwise, with the fixed
    /// rate `fixed_fps` above zero, the frame less `1000 / fixed_fps`
    /// (integer division, without the one taken off the wait) is added, so
    /// game time advances by exactly that a frame; the addition is signed, a
    /// short frame gives game time back. Without either nothing is added.
    pub fn advance(&mut self, now_ms: u32, fixed_fps: i32, paused: bool) -> FrameStep {
        let elapsed = now_ms.wrapping_sub(self.last_ms);
        if paused {
            if self.pause_holds_time {
                self.paused_ms = self.paused_ms.wrapping_add(elapsed);
            }
        } else if fixed_fps > 0 {
            let period = (1000 / fixed_fps) as u32;
            self.paused_ms = self.paused_ms.wrapping_add(elapsed).wrapping_sub(period);
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
    interval: u32,
    due_ms: u32,
    free: bool,
    action: Box<dyn FnMut()>,
}

/// The periodic timer table: fixed intervals, a next-due time and a callback
/// for each record, re-armed from the current frame time so the table never
/// drifts.
pub struct PeriodicTimers {
    slots: Vec<Timer>,
    /// The multiplier applied to intervals, 100 (the original's time scale
    /// unit).
    pub multiplier: u32,
}

impl Default for PeriodicTimers {
    fn default() -> Self {
        Self::new()
    }
}

impl PeriodicTimers {
    /// An empty table at the original's multiplier.
    pub fn new() -> Self {
        Self {
            slots: Vec::new(),
            multiplier: TIMER_MULTIPLIER,
        }
    }

    /// Registers `action` to run every `interval` units from the frame time
    /// `now_ms`, taking the first free record or adding one. The returned id
    /// is the record's one-based position.
    pub fn add(&mut self, interval: u32, now_ms: u32, action: Box<dyn FnMut()>) -> u32 {
        let slot = Timer {
            interval,
            due_ms: self.multiplier.wrapping_mul(interval).wrapping_add(now_ms),
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

    /// Moves a record's next run `elapsed` units closer: the next due time
    /// is `(interval − elapsed) · multiplier` from `now_ms`.
    pub fn reschedule(&mut self, id: u32, elapsed: u32, now_ms: u32) {
        if let Some(slot) = id
            .checked_sub(1)
            .and_then(|i| self.slots.get_mut(i as usize))
        {
            slot.due_ms = slot
                .interval
                .wrapping_sub(elapsed)
                .wrapping_mul(self.multiplier)
                .wrapping_add(now_ms);
        }
    }

    /// Runs every record that is due at `now_ms`, in record order, and arms
    /// each one a full interval from now. The comparison is unsigned.
    pub fn run(&mut self, now_ms: u32) {
        for slot in &mut self.slots {
            if slot.free || now_ms < slot.due_ms {
                continue;
            }
            (slot.action)();
            slot.due_ms = self
                .multiplier
                .wrapping_mul(slot.interval)
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
pub type Deferred = Box<dyn FnMut() -> bool>;

/// One queued call and the identities it was registered with. The original
/// stores a procedure and a data word beside every entry so a caller can
/// cancel all work for an object or a routine; a call queued without tags
/// matches neither.
struct QueuedCall {
    procedure: Option<u64>,
    data: Option<u64>,
    call: Deferred,
}

/// The original's ring of calls that run again next frame until they return
/// false.
///
/// The ring has [`DEFERRED_SLOTS`] slots and a head and a tail index; a push
/// whose next tail would meet the head is dropped, so it holds at most 127
/// calls. A run walks from the head up to the tail as it was when the run
/// began: a call that returns false is removed by moving the calls between
/// the head and it one slot on and advancing the head, so the survivors keep
/// their order. The walk stops when the index is no longer below that tail,
/// compared as plain integers, so once the calls wrap past the ring's end
/// only the call at the head runs in a tick (the original's quirk, kept).
pub struct DeferredCalls {
    slots: Vec<Option<QueuedCall>>,
    head: usize,
    tail: usize,
}

impl Default for DeferredCalls {
    fn default() -> Self {
        Self::new()
    }
}

impl DeferredCalls {
    /// An empty ring.
    pub fn new() -> Self {
        Self {
            slots: (0..DEFERRED_SLOTS).map(|_| None).collect(),
            head: 0,
            tail: 0,
        }
    }

    /// Pushes a call for the next run; returns false when the ring is full
    /// and the call is dropped.
    pub fn push(&mut self, call: Deferred) -> bool {
        self.push_tagged(None, None, call)
    }

    /// Pushes a call tagged with the procedure and data identities its
    /// cancel routines match; `None` for either means it cannot be found by
    /// that identity (the untagged [`DeferredCalls::push`]).
    pub fn push_tagged(
        &mut self,
        procedure: Option<u64>,
        data: Option<u64>,
        call: Deferred,
    ) -> bool {
        let next = (self.tail + 1) % DEFERRED_SLOTS;
        if next == self.head {
            return false;
        }
        self.slots[self.tail] = Some(QueuedCall {
            procedure,
            data,
            call,
        });
        self.tail = next;
        true
    }

    /// Removes every queued call with this data identity and returns how
    /// many went. Removal rotates the ring and advances its head, so the
    /// remaining calls keep their order.
    pub fn remove_data(&mut self, data: u64) -> usize {
        self.remove_where(|call| call.data == Some(data))
    }

    /// Removes every queued call with this procedure identity and returns
    /// how many went.
    pub fn remove_procedure(&mut self, procedure: u64) -> usize {
        self.remove_where(|call| call.procedure == Some(procedure))
    }

    /// The original's removal walk: every live slot from the head is
    /// visited once; a match is removed in place (which advances the head)
    /// and the scan moves on to the next physical slot.
    fn remove_where(&mut self, mut matches: impl FnMut(&QueuedCall) -> bool) -> usize {
        let mut removed = 0;
        let mut at = self.head;
        while at != self.tail {
            let remove = self.slots[at].as_ref().is_some_and(&mut matches);
            if remove {
                self.remove(at);
                removed += 1;
            }
            at += 1;
            if at >= DEFERRED_SLOTS {
                at = 0;
            }
        }
        removed
    }

    /// Runs the pending calls once and keeps the ones that ask for another
    /// frame.
    pub fn run(&mut self) {
        if self.head == self.tail {
            return;
        }
        let end = self.tail;
        let mut index = self.head;
        loop {
            let keep = match self.slots[index].as_mut() {
                Some(queued) => (queued.call)(),
                None => false,
            };
            if !keep {
                self.remove(index);
            }
            index += 1;
            if index >= DEFERRED_SLOTS {
                index = 0;
            }
            if end <= index {
                break;
            }
        }
    }

    /// Removes the call at `index`: the calls from the head up to it move
    /// one slot on and the head advances.
    fn remove(&mut self, index: usize) {
        self.slots[index] = None;
        if index >= self.head {
            for k in (self.head + 1..=index).rev() {
                self.slots[k] = self.slots[k - 1].take();
            }
        } else {
            for k in (1..=index).rev() {
                self.slots[k] = self.slots[k - 1].take();
            }
            self.slots[0] = self.slots[DEFERRED_SLOTS - 1].take();
            for k in (self.head + 1..DEFERRED_SLOTS).rev() {
                self.slots[k] = self.slots[k - 1].take();
            }
        }
        self.head = if self.head >= DEFERRED_SLOTS - 1 {
            0
        } else {
            self.head + 1
        };
    }

    /// How many calls wait for the next run.
    pub fn len(&self) -> usize {
        (self.tail + DEFERRED_SLOTS - self.head) % DEFERRED_SLOTS
    }

    /// Whether no call waits.
    pub fn is_empty(&self) -> bool {
        self.head == self.tail
    }
}

/// The original's sleep: zero yields the processor, anything else sleeps on
/// a high-resolution waitable timer (or `Sleep` with a one-millisecond timer
/// period where that timer is missing). Exarust uses
/// [`std::thread::sleep`], which is high-resolution on its platforms.
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
        // A fixed rate below the refresh rate rounds to nearest:
        // 1000/60 = 16.67 -> 17, minus one.
        assert_eq!(focused(false, 144, 60).target_interval(), Some(16));
        // 1000/80 = 12.5 rounds to even, 12, minus one.
        assert_eq!(focused(false, 144, 80).target_interval(), Some(11));
        // An automatic cap with no vsync holds the display's rate, at least
        // 120: 1000/120 = 8.33 -> 8 - 1; 1000/165 = 6.06 -> 6 - 1.
        assert_eq!(focused(false, 60, -1).target_interval(), Some(7));
        assert_eq!(focused(false, 165, -1).target_interval(), Some(5));
        // An unknown rate is below 120 in the signed comparison.
        assert_eq!(focused(false, 0, -1).target_interval(), Some(7));
        // Vsync asks for the refresh rate, one millisecond early:
        // 1000/144 = 6.94 -> 7 - 1.
        assert_eq!(focused(true, 144, -1).target_interval(), Some(6));
        // A fixed cap at or above the refresh rate still asks for it.
        assert_eq!(focused(true, 144, 240).target_interval(), Some(6));
        assert_eq!(focused(true, 144, 144).target_interval(), Some(6));
        assert_eq!(focused(true, 144, 60).target_interval(), Some(16));
        // An unknown rate under vsync divides by zero: the indefinite
        // integer minus one keeps its low 32 bits, -1, and nothing waits.
        assert_eq!(focused(true, 0, -1).target_interval(), Some(-1));
        assert_eq!(focused(true, 0, -1).wait_ms(1_000, 999), 0);
        // A cap above 2000 rounds its period to 0: -1, no wait.
        assert_eq!(focused(false, 144, 3000).target_interval(), Some(-1));
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
        // 1000/60 = 16 by integer division, one less: 15 ms.
        assert_eq!(fixed_rate_interval(60), Some(15));
        assert_eq!(fixed_rate_interval(83), Some(11));
        assert_eq!(fixed_rate_interval(144), Some(5));
        // Above 1000 the period is 0 and the interval -1.
        assert_eq!(fixed_rate_interval(2000), Some(-1));
        assert_eq!(fixed_rate_interval(0), None);
    }

    #[test]
    fn the_tick_sleeps_only_while_ahead() {
        // A 16 ms interval, 10 ms in: 6 ms to go.
        let pacer = focused(false, 144, 60);
        assert_eq!(pacer.wait_ms(1_000, 990), 6);
        assert_eq!(pacer.wait_ms(1_016, 1_000), 0);
        assert_eq!(pacer.wait_ms(1_030, 1_000), 0);
    }

    #[test]
    fn game_time_ignores_the_paused_and_fixed_rate_time() {
        let mut clock = FrameClock::default();
        // A free-running frame advances the game clock by its length.
        assert_eq!(clock.advance(30, 0, false).game_ms, 30);
        assert_eq!(clock.advance(55, 0, false).game_delta_ms, 25);
        // Paused, the same real time is held back.
        assert_eq!(clock.advance(80, 0, true).game_ms, 55);
        assert_eq!(clock.game_ms(), 55);
        // A pause takes precedence over the fixed rate.
        assert_eq!(clock.advance(100, 60, true).game_ms, 55);
        // Without the switch a pause holds nothing back.
        clock.pause_holds_time = false;
        assert_eq!(clock.advance(130, 0, true).game_delta_ms, 30);
        // At 60 fixed frames the game advances by 1000/60 = 16 ms a frame,
        // whatever the real frame took: the credit is signed, so a 21 ms
        // frame holds 5 ms back and a 15 ms frame gives 1 ms of it back.
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
        // 20 units at multiplier 100 fall due 2000 ms on, at 3000.
        timers.run(2_999);
        assert_eq!(*count.borrow(), 0);
        timers.run(3_000);
        assert_eq!(*count.borrow(), 1);
        // Re-armed from now, not from the old due time.
        timers.run(4_999);
        assert_eq!(*count.borrow(), 1);
        timers.run(5_000);
        assert_eq!(*count.borrow(), 2);
        // A reschedule by 15 of the 20 units: (20 - 15) * 100 = 500 ms on.
        timers.reschedule(id, 15, 5_000);
        timers.run(5_499);
        assert_eq!(*count.borrow(), 2);
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

    fn counting(log: &std::rc::Rc<std::cell::RefCell<Vec<u32>>>, id: u32, runs: u32) -> Deferred {
        let log = log.clone();
        let mut left = runs;
        Box::new(move || {
            log.borrow_mut().push(id);
            left -= 1;
            left > 0
        })
    }

    #[test]
    fn the_deferred_ring_holds_127_and_keeps_asking_calls() {
        let mut calls = DeferredCalls::new();
        let mut pushed = 0;
        for _ in 0..DEFERRED_SLOTS + 3 {
            if calls.push(Box::new(|| false)) {
                pushed += 1;
            }
        }
        assert_eq!(pushed, DEFERRED_SLOTS - 1);
        assert_eq!(calls.len(), DEFERRED_SLOTS - 1);
        calls.run();
        assert!(calls.is_empty());
        let log = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
        let mut calls = DeferredCalls::new();
        calls.push(counting(&log, 1, 3));
        calls.push(counting(&log, 2, 1));
        calls.push(counting(&log, 3, 2));
        calls.run();
        // Call 2 asked to stop; the others keep their order.
        assert_eq!(*log.borrow(), [1, 2, 3]);
        assert_eq!(calls.len(), 2);
        log.borrow_mut().clear();
        calls.run();
        assert_eq!(*log.borrow(), [1, 3]);
        assert_eq!(calls.len(), 1);
        log.borrow_mut().clear();
        calls.run();
        assert_eq!(*log.borrow(), [1]);
        assert!(calls.is_empty());
    }

    #[test]
    fn a_wrapped_ring_runs_only_its_head() {
        let log = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
        let mut calls = DeferredCalls::new();
        // Move the head to slot 126 with calls that finish at once.
        for _ in 0..126 {
            calls.push(Box::new(|| false));
        }
        calls.run();
        assert!(calls.is_empty());
        // Four calls in slots 126, 127, 0 and 1: the tail is 2.
        for id in 1..=4 {
            calls.push(counting(&log, id, 2));
        }
        calls.run();
        // The walk stops at once: 127 is not below the tail 2.
        assert_eq!(*log.borrow(), [1]);
        assert_eq!(calls.len(), 4);
    }

    #[test]
    fn the_idle_wait_follows_the_redraw_flags() {
        use redraw::*;
        let wait = |flags, focused, minimized| idle_wait(flags, focused, minimized, 7);
        let waits = |ms| IdleWait {
            wait_ms: ms,
            waits: true,
        };
        assert_eq!(wait(0, true, false), waits(32));
        assert_eq!(wait(0, false, false), waits(1024));
        assert_eq!(wait(ANIMATION, true, false), waits(32));
        assert_eq!(wait(ANIMATION, false, false), waits(64));
        assert_eq!(wait(ACTIVE, true, false), waits(1));
        assert_eq!(wait(ACTIVE, false, false), waits(32));
        // An immediate redraw does not wait and keeps the last time-out.
        assert_eq!(
            wait(IMMEDIATE, true, false),
            IdleWait {
                wait_ms: 7,
                waits: false
            }
        );
        // Bits 4-7 count as bits 8-11.
        assert_eq!(wait(0x20, true, false), waits(1));
        assert!(!wait(0x40, true, false).waits);
        // A minimised window always waits, at least 32 ms.
        assert_eq!(wait(ACTIVE, true, true), waits(32));
        assert_eq!(wait(IMMEDIATE, true, true), waits(32));
        assert_eq!(wait(0, false, true), waits(1024));
        // The strongest flag decides.
        assert_eq!(wait(DRAW | PRESENT | ANIMATION, true, false), waits(32));
        assert!(draws(DRAW, false));
        assert!(!draws(DRAW, true));
        assert!(!draws(PRESENT, false));
        let mut flags = 0;
        request(&mut flags);
        assert_eq!(flags, DRAW | PRESENT);
    }

    #[test]
    fn the_status_line_keeps_sixteen_newest_first() {
        let mut history = StatusHistory::default();
        for i in 0..20 {
            history.push(1, format!("line {i}"));
        }
        assert_eq!(history.entries().len(), LOG_HISTORY);
        assert_eq!(history.entries()[0].text, "line 19");
        assert_eq!(history.entries()[15].text, "line 4");
        history.push(0, "Title");
        assert_eq!(history.entries()[0].kind, 0);
    }

    #[test]
    fn overlay_lines_expire_one_a_tick_after_their_time() {
        let mut overlay = LogOverlay::default();
        // "Error : abcd" is 12 bytes: 2000 + 600 ms.
        assert!(overlay.push("Error", "abcd", 1_000));
        assert_eq!(overlay.lines().next().unwrap().text, "Error : abcd");
        assert_eq!(overlay.lines().next().unwrap().expires_ms, 3_600);
        assert!(overlay.push("Error", "abcd", 1_000));
        assert!(!overlay.expire(3_600));
        // Both are due, but one goes a tick.
        assert!(overlay.expire(3_601));
        assert_eq!(overlay.len(), 1);
        assert!(overlay.expire(3_601));
        assert!(overlay.is_empty());
        // Fifteen lines at most; the sixteenth is dropped.
        for i in 0..LOG_OVERLAY_SLOTS - 1 {
            assert!(overlay.push("", &format!("{i}"), 0), "{i}");
        }
        assert!(!overlay.push("", "sixteen", 0));
        assert_eq!(overlay.len(), LOG_OVERLAY_SLOTS - 1);
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
        assert!(LogPresentation::writes_file(4));
        assert!(!LogPresentation::writes_file(5));
        assert_eq!(LogPresentation::status_kind(1), Some(0));
        assert_eq!(LogPresentation::status_kind(3), Some(1));
        assert_eq!(LogPresentation::status_kind(4), None);
        assert_eq!(overlay_lifetime_ms("abcd"), 2_200);
        assert_eq!(overlay_lifetime_ms(""), 2_000);
    }

    #[test]
    fn tagged_calls_cancel_by_data_or_procedure() {
        let log = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
        let mut calls = DeferredCalls::new();
        calls.push_tagged(Some(10), Some(100), counting(&log, 1, 1));
        calls.push_tagged(Some(11), Some(100), counting(&log, 2, 1));
        calls.push_tagged(Some(10), Some(200), counting(&log, 3, 1));
        calls.push(counting(&log, 4, 1));
        assert_eq!(calls.remove_data(100), 2);
        assert_eq!(calls.len(), 2);
        // An untagged call was not registered with data 0.
        assert_eq!(calls.remove_data(0), 0);
        assert_eq!(calls.remove_procedure(10), 1);
        assert_eq!(calls.len(), 1);
        calls.run();
        assert_eq!(*log.borrow(), [4]);
        assert!(calls.is_empty());
    }

    #[test]
    fn cancellation_keeps_the_remaining_calls_in_order() {
        let log = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
        let mut calls = DeferredCalls::new();
        for id in 1..=5u32 {
            calls.push_tagged(Some(1), Some(u64::from(id)), counting(&log, id, 2));
        }
        // Cancel the middle one; the rest still run in their order.
        assert_eq!(calls.remove_data(3), 1);
        calls.run();
        assert_eq!(*log.borrow(), [1, 2, 4, 5]);
        assert_eq!(calls.len(), 4);
        log.borrow_mut().clear();
        calls.run();
        assert_eq!(*log.borrow(), [1, 2, 4, 5]);
        assert!(calls.is_empty());
    }
}
