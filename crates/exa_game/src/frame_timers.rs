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
//! The same tick keeps the game's clocks first ([`GameClock`]): the real
//! millisecond clock, the paused-time accumulator (a frame while the game
//! is paused adds to it) and the game clock, the real clock less the
//! accumulator. The scene task then steps its own clock from them
//! ([`step_scene_clock`]): its step is the game clock's since the last
//! frame, capped at 1/60 s with a time scale for a longer frame, started
//! when a level comes in; the scene's readers take it from
//! [`exa_world::scene_frame::SceneFrame`].
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

/// The main tick's clocks ([`crate::frame::FrameClock`]), advanced once a
/// frame: the real clock read after the frame's pacing, the paused-time
/// accumulator and the game clock.
#[derive(Resource, Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct GameClock {
    clock: crate::frame::FrameClock,
    now_ms: u32,
    step: crate::frame::FrameStep,
}

impl GameClock {
    /// Advances to the real clock `now_ms`; `paused` is the pause counter
    /// above zero.
    pub fn advance(&mut self, now_ms: u32, paused: bool) -> crate::frame::FrameStep {
        // Nothing in the shipped game sets the fixed rate.
        self.step = self.clock.advance(now_ms, 0, paused);
        self.now_ms = now_ms;
        self.step
    }

    /// The real clock of this frame.
    pub fn now_ms(&self) -> u32 {
        self.now_ms
    }

    /// The paused-time accumulator.
    pub fn paused_ms(&self) -> u32 {
        self.clock.paused_ms()
    }

    /// The game clock: the real clock less the paused time.
    pub fn game_ms(&self) -> u32 {
        self.clock.game_ms()
    }

    /// This frame's step.
    pub fn step(&self) -> crate::frame::FrameStep {
        self.step
    }
}

/// Advances the game clocks on the app's real clock. The pause counter is
/// the simulation's hold ([`exa_physics::sim_app::SimPause`]): the screens
/// and the teardown that pause the game raise it.
pub fn advance_game_clock(
    time: Res<Time<Real>>,
    pause: Option<Res<exa_physics::sim_app::SimPause>>,
    mut clock: ResMut<GameClock>,
) {
    let paused = pause.is_some_and(|p| p.paused());
    clock.advance(time.elapsed().as_millis() as u32, paused);
}

/// The scene task's clock ([`crate::scene_editor::FrameClock`]), live.
#[derive(Resource, Debug, Clone, Copy, Default, PartialEq)]
pub struct SceneClock {
    pub clock: crate::scene_editor::FrameClock,
    pub running: bool,
}

/// Steps the scene task's clock after the game clocks: started when a
/// level is in (the loading screen gone), as the task's start stamps the
/// game clock, then stepped once a frame on the real clock less the paused
/// time; the step, scale and audio mark go to
/// [`exa_world::scene_frame::SceneFrame`]. The audio mark takes half the
/// step off while the view super-samples. While a level loads there is no
/// scene: the step is 0 and the scale 1.
pub fn step_scene_clock(
    game: Res<GameClock>,
    loading: Option<Res<exa_ui::LoadingStatus>>,
    hardware: Option<Res<crate::hw_config::HardwareConfig>>,
    mut scene: ResMut<SceneClock>,
    mut frame: ResMut<exa_world::scene_frame::SceneFrame>,
) {
    if loading.is_some() {
        scene.running = false;
        *frame = exa_world::scene_frame::SceneFrame::default();
        return;
    }
    if !scene.running {
        scene.clock = crate::scene_editor::FrameClock::started(game.now_ms(), game.paused_ms());
        scene.running = true;
        *frame = exa_world::scene_frame::SceneFrame {
            audio_mark: game.game_ms(),
            running: true,
            ..Default::default()
        };
        return;
    }
    let passes = hardware.map_or(0, |h| h.live.supersample);
    let step = scene
        .clock
        .step(game.now_ms(), game.paused_ms(), game.game_ms(), passes);
    *frame = exa_world::scene_frame::SceneFrame {
        dt_ms: step.dt,
        scale: step.scale,
        audio_mark: step.audio_mark,
        running: true,
    };
}

/// Adds the timer table, with the GUI timer registered, and runs it each
/// frame before the game's systems, after the game clocks and the scene
/// clock.
pub struct FrameTimersPlugin;

impl Plugin for FrameTimersPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<FrameTimers>()
            .init_resource::<GameClock>()
            .init_resource::<SceneClock>()
            .init_resource::<exa_world::scene_frame::SceneFrame>()
            .add_systems(
                First,
                (advance_game_clock, step_scene_clock, run_frame_timers)
                    .chain()
                    .after(bevy::time::TimeSystems),
            );
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

    /// The live clocks in an app: the game clock holds while the
    /// simulation's pause is up, and the scene clock's step follows it
    /// (0 while held, capped at 1/60 s with a scale for a long frame).
    #[test]
    fn the_scene_clock_steps_on_the_game_clock_and_holds_with_the_pause() {
        use bevy::time::{TimePlugin, TimeUpdateStrategy};
        use exa_world::scene_frame::SceneFrame;
        use std::time::Duration;

        let mut app = App::new();
        app.add_plugins((TimePlugin, FrameTimersPlugin))
            .init_resource::<exa_physics::sim_app::SimPause>()
            .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
                10,
            )));
        app.update();
        app.update();
        // The first frames start the scene; then a 10 ms frame is 10 ms.
        app.update();
        let frame = *app.world().resource::<SceneFrame>();
        assert!(frame.running);
        assert_eq!(frame.dt_ms, 10.0);
        assert_eq!(frame.scale, 1.0);
        let game = app.world().resource::<GameClock>().game_ms();
        // The pause holds the game clock and the scene's step.
        app.world_mut()
            .resource_mut::<exa_physics::sim_app::SimPause>()
            .raise();
        app.update();
        app.update();
        assert_eq!(app.world().resource::<SceneFrame>().dt_ms, 0.0);
        assert_eq!(app.world().resource::<GameClock>().game_ms(), game);
        app.world_mut()
            .resource_mut::<exa_physics::sim_app::SimPause>()
            .lower();
        // A 50 ms frame is capped: 1/60 s and a scale of the cap over 50.
        app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
            50,
        )));
        app.update();
        let frame = *app.world().resource::<SceneFrame>();
        assert_eq!(frame.dt_ms, crate::scene_editor::FRAME_CAPPED_MS);
        assert_eq!(frame.scale, (16.666_666_666_666_668 / 50.0) as f32);
        // A level load stops the scene; the next level starts it again.
        app.insert_resource(exa_ui::LoadingStatus::default());
        app.update();
        assert!(!app.world().resource::<SceneFrame>().running);
        app.world_mut().remove_resource::<exa_ui::LoadingStatus>();
        app.update();
        let frame = *app.world().resource::<SceneFrame>();
        assert!(frame.running);
        assert_eq!(frame.dt_ms, 0.0);
    }

    #[test]
    fn without_the_table_the_gui_work_runs_every_frame() {
        assert!(gui_tick(None));
        let timers = FrameTimers::new(0);
        assert!(!gui_tick(Some(&timers)));
    }
}
