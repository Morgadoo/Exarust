//! The main unit's periodic timer table, run once a frame.
//!
//! The original's frame tick runs a table of periodic timers after it has
//! paced the frame: every record whose due time has passed the frame's
//! real-time clock runs its callback and is armed one interval (scaled by
//! 100) from that clock. Three units register records in 0.9.5:
//!
//! - the GUI, at its start: one unit (100 ms), for its whole life. Its
//!   callback is the GUI timer: the delayed tooltip, the tooltip's
//!   auto-hide, the pressed and the timer form's repeat and the deletion of
//!   closed forms. A press on a scroll arrow reschedules it six units
//!   further out (700 ms), so the repeat starts after that pause.
//! - the item container's delayed card: two units, freed once it fired.
//! - the arena's dormant auto rematch: forty units after the return to the
//!   hub, freed by its own callback.
//!
//! [`FrameTimers`] holds the table ([`crate::frame::PeriodicTimers`]) with
//! an owner tag per record; [`run_frame_timers`] runs it in `First` on the
//! app's real clock and keeps the records that came due this frame, which
//! their owners read ([`FrameTimers::fired`]). The GUI's record is
//! registered when the plugin is built, as the GUI's start does; the GUI
//! screens run their timer work only on its ticks
//! ([`crate::form_input::drive`]) and the arena rematch registers its own
//! record. The item card keeps its delay as an inline due time
//! ([`crate::item_container::CardTimer`]): a one-shot record of two units
//! checked once a frame against the frame clock is the same arithmetic.
//!
//! Rules: `docs/PERFORMANCE.md` (Frame pacing).

use bevy::prelude::*;

use crate::frame::PeriodicTimers;

/// The GUI timer's interval, in timer units (100 ms each).
pub const GUI_TIMER_INTERVAL: u32 = 1;
/// How far an arrow press pushes the GUI timer out: the reschedule's
/// "elapsed" units, −6, so the next run is seven units (700 ms) away.
pub const GUI_TIMER_ARROW_PAUSE: i32 = -6;

/// Who registered a timer record.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TimerUser {
    /// The GUI timer (tooltips, form repeats, closed-form deletion).
    Gui,
    /// The arena's auto rematch.
    ArenaRematch,
}

/// The live timer table and this frame's due records.
#[derive(Resource, Debug, Clone)]
pub struct FrameTimers {
    table: PeriodicTimers<TimerUser>,
    fired: Vec<(u32, TimerUser)>,
    /// The frame's real-time clock in milliseconds (wrapping, as the
    /// original's 32-bit clock).
    now_ms: u32,
    /// The GUI timer's record.
    pub gui: u32,
}

impl Default for FrameTimers {
    fn default() -> Self {
        Self::new(0)
    }
}

impl FrameTimers {
    /// A table at `now_ms` with the GUI timer registered as its first
    /// record.
    pub fn new(now_ms: u32) -> Self {
        let mut table = PeriodicTimers::new();
        let gui = table.add(GUI_TIMER_INTERVAL, now_ms, TimerUser::Gui);
        Self {
            table,
            fired: Vec::new(),
            now_ms,
            gui,
        }
    }

    /// Runs the table at the frame time `now_ms` and keeps the records that
    /// came due.
    pub fn run(&mut self, now_ms: u32) {
        self.now_ms = now_ms;
        self.fired = self.table.run(now_ms);
    }

    /// The records that came due this frame, in record order.
    pub fn fired_records(&self) -> &[(u32, TimerUser)] {
        &self.fired
    }

    /// Registers `user` every `interval` units from this frame's time.
    pub fn add(&mut self, interval: u32, user: TimerUser) -> u32 {
        self.table.add(interval, self.now_ms, user)
    }

    /// Frees record `id`.
    pub fn free(&mut self, id: u32) {
        self.table.free(id);
    }

    /// Moves record `id`'s next run `elapsed` units closer to this frame's
    /// time (a negative `elapsed` pushes it out).
    pub fn reschedule(&mut self, id: u32, elapsed: i32) {
        self.table.reschedule(id, elapsed as u32, self.now_ms);
    }

    /// Whether record `id` came due this frame.
    pub fn fired(&self, id: u32) -> bool {
        self.fired.iter().any(|&(fired, _)| fired == id)
    }

    /// Whether the GUI timer came due this frame.
    pub fn gui_tick(&self) -> bool {
        self.fired(self.gui)
    }

    /// The frame time the table last ran at.
    pub fn now_ms(&self) -> u32 {
        self.now_ms
    }

    /// The table's records, free ones included.
    pub fn len(&self) -> usize {
        self.table.len()
    }

    /// Whether the table has no records.
    pub fn is_empty(&self) -> bool {
        self.table.is_empty()
    }
}

/// Whether the GUI timer's work runs this frame: when it came due, or every
/// frame in an app without the table (headless tools and tests).
pub fn gui_tick(timers: Option<&FrameTimers>) -> bool {
    timers.is_none_or(FrameTimers::gui_tick)
}

/// Runs the timer table on the app's real clock. With `EXA_TIMER_TRACE`
/// set, every record that comes due is logged with the frame time.
pub fn run_frame_timers(
    time: Res<Time<Real>>,
    mut timers: ResMut<FrameTimers>,
    mut trace: Local<Option<bool>>,
) {
    timers.run(time.elapsed().as_millis() as u32);
    if *trace.get_or_insert_with(|| std::env::var_os("EXA_TIMER_TRACE").is_some()) {
        for (id, user) in timers.fired_records() {
            info!(
                "timer table: record {id} ({user:?}) at {} ms",
                timers.now_ms()
            );
        }
    }
}

/// Adds the timer table, with the GUI timer registered, and runs it each
/// frame before the game's systems.
pub struct FrameTimersPlugin;

impl Plugin for FrameTimersPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<FrameTimers>()
            .add_systems(First, run_frame_timers.after(bevy::time::TimeSystems));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_gui_timer_is_the_first_record_and_ticks_every_100_ms() {
        let mut timers = FrameTimers::new(0);
        assert_eq!(timers.gui, 1);
        timers.run(99);
        assert!(!timers.gui_tick());
        timers.run(100);
        assert!(timers.gui_tick());
        // A late frame runs it once and arms it 100 ms from that frame.
        timers.run(260);
        assert!(timers.gui_tick());
        timers.run(359);
        assert!(!timers.gui_tick());
        timers.run(360);
        assert!(timers.gui_tick());
    }

    #[test]
    fn an_arrow_press_pushes_the_gui_timer_700_ms_out() {
        let mut timers = FrameTimers::new(0);
        timers.run(130);
        timers.reschedule(timers.gui, GUI_TIMER_ARROW_PAUSE);
        timers.run(829);
        assert!(!timers.gui_tick());
        timers.run(830);
        assert!(timers.gui_tick());
    }

    #[test]
    fn a_one_shot_record_fires_once_when_its_owner_frees_it() {
        let mut timers = FrameTimers::new(0);
        timers.run(1_000);
        let rematch = timers.add(40, TimerUser::ArenaRematch);
        assert_eq!(rematch, 2);
        timers.run(4_999);
        assert!(!timers.fired(rematch));
        timers.run(5_000);
        assert!(timers.fired(rematch));
        timers.free(rematch);
        timers.run(9_000);
        assert!(!timers.fired(rematch));
        // The freed record is taken by the next registration.
        assert_eq!(timers.add(2, TimerUser::ArenaRematch), rematch);
    }

    #[test]
    fn without_the_table_the_gui_work_runs_every_frame() {
        assert!(gui_tick(None));
        let timers = FrameTimers::new(0);
        assert!(!gui_tick(Some(&timers)));
    }
}
