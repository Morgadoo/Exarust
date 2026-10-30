//! The live scene editor's task (`RF_SceneEditor`, `TSceneTaskInt`): the
//! camera-mode switch, the frame step with its 1/60 s cap, the task tick's
//! repaint and message rules, the run-scene set-up and the scene node
//! despawn.
//!
//! Nothing here opens a window: the model holds the state the original's
//! task object, viewport and scene keep, so the timing rules and the order
//! of the set-up can be tested without an app. The original's global
//! message posts become the flags of [`TaskTick`] that a host turns into
//! Bevy events.
//!
//! Not modelled: the task tick's copy of the camera pose into the audio
//! listener (and its mid-point blend of two positions), the weather
//! option calls of the run-scene set-up, and the handful of option bytes
//! whose callee was not identified (see [`SceneRunOptions`]).

use exa_world::scene_nodes::{Released, Scene};

use crate::nav_camera::Attachment;

/// The frame cap, 1/60 s in milliseconds, as the double the original
/// compares and divides with.
pub const FRAME_CAP_MS: f64 = 16.666_666_666_666_668;
/// What a capped step stores: the cap as the `f32` the original keeps in
/// its constant table (`0x41855555`).
pub const FRAME_CAPPED_MS: f32 = f32::from_bits(0x4185_5555);
/// The viewport rectangle the run-scene set-up writes: `(100, 100, 800, 600)`.
pub const VIEWPORT: (i32, i32, i32, i32) = (100, 100, 800, 600);
/// The viewport flag bits the set-up ors in.
pub const VIEWPORT_FLAGS: u32 = 0x2000 | 0x8 | 0x4;
/// How many characters of the scene name the viewport title copies.
pub const TITLE_CHARS: usize = 15;
/// The first camera mode with a leave callback: switching away from a mode
/// at or above it calls that mode's callback.
pub const FIRST_CALLBACK_MODE: u32 = 3;
/// The viewport mode word whose low two bits mark a maximised viewport.
pub const MAXIMISED: u32 = 2;
/// The weather word bits the run options may replace.
pub const WEATHER_OPTION_BITS: u32 = 0x60;
/// The audio-listener flag bits a new listener starts with.
pub const LISTENER_ACTIVE: u32 = 0x3;

/// The editor's camera state during a session. The mode is a plain word in
/// the original; the values below [`FIRST_CALLBACK_MODE`] have no callback
/// and the meaning of the individual modes is not recovered.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CameraSwitch {
    /// The scene's camera mode word.
    pub mode: u32,
    /// The navigation controller exists (it is created on the first record).
    pub controller: bool,
    /// The followed camera record, when there is one.
    pub target: Option<u32>,
    /// The camera object's mode word, kept equal to the scene's.
    pub viewport_mode: u32,
    /// The scene's listeners were notified.
    pub notified: bool,
    /// The mode whose leave callback ran on the last switch.
    pub left: Option<u32>,
    /// The navigation controller's viewport attachment: a followed record
    /// binds it and dropping the record detaches it.
    pub attachment: Attachment,
}

impl CameraSwitch {
    /// Switches the camera. The previous mode's callback runs when it is
    /// [`FIRST_CALLBACK_MODE`] or above; the mode is stored in the scene and
    /// the camera object. An existing controller drops its target and, given
    /// a record, follows it; without one it is detached. With no controller,
    /// a record creates one. The scene's listeners are notified last.
    /// Returns whether the controller was created.
    pub fn switch(&mut self, mode: u32, data: Option<u32>) -> bool {
        self.left = (self.mode >= FIRST_CALLBACK_MODE).then_some(self.mode);
        self.mode = mode;
        self.viewport_mode = mode;
        let created = match (self.controller, data) {
            (false, Some(record)) => {
                self.controller = true;
                self.target = Some(record);
                true
            }
            (true, record) => {
                self.target = record;
                self.attachment.set_detached(record.is_none());
                false
            }
            (false, None) => false,
        };
        self.notified = true;
        created
    }
}

/// The count of maximised viewports: a viewport-class object whose mode
/// word is [`MAXIMISED`] adds one, any other mode word removes one.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MaximisedViewports {
    pub count: i32,
}

impl MaximisedViewports {
    /// Notifies a mode change of an object; only viewports count.
    pub fn notify(&mut self, is_viewport_class: bool, mode: u32) {
        if !is_viewport_class {
            return;
        }
        if mode == MAXIMISED {
            self.count += 1;
        } else {
            self.count -= 1;
        }
    }
}

/// The scene's audio listener slot.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Listener {
    /// No listener yet.
    #[default]
    None,
    /// The scene's own listener with its flag word.
    Own(u32),
    /// The shared global listener.
    Global,
}

/// The scene state the run-scene set-up changes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SceneSettings {
    /// The weather word (bits [`WEATHER_OPTION_BITS`] are the options').
    pub weather_flags: u32,
    /// The renderer's minimum quality word; the options can only raise it.
    pub renderer_minimum: u32,
    pub listener: Listener,
    /// The scene's camera mode word.
    pub camera_mode: u32,
}

/// What the run-scene set-up reads from its options record. The bytes whose
/// callee was not identified are kept as they are, named by what they gate.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SceneRunOptions {
    /// The camera record to follow: seven words copied into the task.
    pub camera: Option<[u64; 7]>,
    /// The option bits: [`WEATHER_OPTION_BITS`] replace the weather word's;
    /// two more groups drive the weather step (not modelled).
    pub weather: u32,
    /// The renderer's minimum quality word.
    pub renderer_minimum: u32,
    /// Create the audio listener.
    pub listener: bool,
    /// Gates a scene preparation call (callee not identified).
    pub prepare_scene: bool,
    /// Gates a viewport call (callee not identified).
    pub viewport_call: bool,
    /// Gates a viewport flag, with a second byte (callees not identified).
    pub viewport_flag: bool,
    /// Gates a renderer toggle (callee not identified).
    pub renderer_toggle: bool,
}

/// A scene the editor has opened in a viewport.
#[derive(Clone, Debug, PartialEq)]
pub struct SceneRun {
    pub viewport: (i32, i32, i32, i32),
    /// The viewport flag bits the set-up ors in.
    pub viewport_flags: u32,
    /// The viewport's title: the scene name cut to [`TITLE_CHARS`]
    /// characters (the copy's argument order was not confirmed).
    pub title: String,
    /// The task's copy of the camera record.
    pub camera_record: Option<[u64; 7]>,
    /// A navigation controller was created for the record.
    pub controller: bool,
}

impl SceneRun {
    /// Opens a scene in a new viewport. The viewport gets its rectangle,
    /// flags and title; the renderer minimum is raised if the options ask
    /// for more; a camera record is copied, creates the controller and sets
    /// the scene's camera mode to 1 directly; option bits replace the
    /// weather word's; a requested listener is created, or an existing one
    /// without its flag bits falls back to the global listener.
    pub fn open(scene_name: &str, options: &SceneRunOptions, scene: &mut SceneSettings) -> Self {
        let title = scene_name.chars().take(TITLE_CHARS).collect();
        if options.renderer_minimum > scene.renderer_minimum {
            scene.renderer_minimum = options.renderer_minimum;
        }
        if options.camera.is_some() {
            scene.camera_mode = 1;
        }
        if options.weather & WEATHER_OPTION_BITS != 0 {
            scene.weather_flags = (scene.weather_flags & !WEATHER_OPTION_BITS)
                | (options.weather & WEATHER_OPTION_BITS);
        }
        if options.listener {
            scene.listener = match scene.listener {
                Listener::None => Listener::Own(LISTENER_ACTIVE),
                Listener::Own(flags) if flags & LISTENER_ACTIVE == 0 => Listener::Global,
                other => other,
            };
        }
        Self {
            viewport: VIEWPORT,
            viewport_flags: VIEWPORT_FLAGS,
            title,
            camera_record: options.camera,
            controller: options.camera.is_some(),
        }
    }
}

/// A scene's frame clock: the stamp of the previous step and the two step
/// lengths the scene keeps.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FrameClock {
    /// The previous step's `now - base`, a signed 64-bit value.
    pub stamp: i64,
    /// The step before the last one (ms).
    pub previous_dt: f32,
    /// The last step after the cap (ms).
    pub dt: f32,
    /// The scale of the last step: 1, or the cap over the step when capped.
    pub scale: f32,
}

impl Default for FrameClock {
    fn default() -> Self {
        Self {
            stamp: 0,
            previous_dt: 0.0,
            dt: 0.0,
            scale: 1.0,
        }
    }
}

/// The result of one frame step.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FrameStep {
    pub dt: f32,
    pub scale: f32,
    /// The scene's audio mark.
    pub audio_mark: u32,
}

impl FrameClock {
    /// A clock started at the given time (the task's start stamp).
    pub fn started(now: u32, base: u32) -> Self {
        Self {
            stamp: elapsed(now, base),
            ..Self::default()
        }
    }

    /// Steps the clock. The step is `now - base` less the previous stamp,
    /// as a signed 64-bit value converted to `f32`. Compared in double, a
    /// step over [`FRAME_CAP_MS`] stores [`FRAME_CAPPED_MS`] and a scale of
    /// the cap over the step; otherwise the scale is 1 and a negative step
    /// becomes 0 (NaN stays). The audio mark is the audio clock less half
    /// the step rounded to even when the viewport count is positive, the
    /// audio clock itself otherwise; it is a 32-bit word.
    pub fn step(
        &mut self,
        now: u32,
        base: u32,
        audio_clock: u32,
        viewport_count: i32,
    ) -> FrameStep {
        let now_minus_base = elapsed(now, base);
        self.previous_dt = self.dt;
        let mut dt = now_minus_base.wrapping_sub(self.stamp) as f32;
        if f64::from(dt) > FRAME_CAP_MS {
            self.scale = (FRAME_CAP_MS / f64::from(dt)) as f32;
            dt = FRAME_CAPPED_MS;
        } else {
            self.scale = 1.0;
            if dt < 0.0 {
                dt = 0.0;
            }
        }
        self.stamp = now_minus_base;
        self.dt = dt;
        let audio_mark = if viewport_count > 0 {
            let half = (dt * 0.5).round_ties_even() as i64;
            u64::from(audio_clock).wrapping_sub(half as u64) as u32
        } else {
            audio_clock
        };
        FrameStep {
            dt,
            scale: self.scale,
            audio_mark,
        }
    }
}

/// `now - base` as the original computes it: two zero-extended 32-bit
/// clocks subtracted as 64-bit words, read back as signed.
fn elapsed(now: u32, base: u32) -> i64 {
    u64::from(now).wrapping_sub(u64::from(base)) as i64
}

/// What one task tick reads from the outside.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TickInputs {
    pub now: u32,
    pub base: u32,
    pub audio_clock: u32,
    /// The viewport's count that gates the audio mark (signed).
    pub viewport_count: i32,
    /// The byte the scene update leaves in its flag (it starts as 1).
    pub scene_update_flag: bool,
    /// The weather step reported work, which clears the flag.
    pub weather_busy: bool,
    /// The message's two input words.
    pub primary: u32,
    pub secondary: u32,
}

/// What a scene task's tick asks the host to do.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TaskTick {
    /// The frame step that ran (none for a paused task).
    pub step: Option<FrameStep>,
    /// Message `0x47` goes to the global GUI task.
    pub post_gui_task: bool,
    /// The viewport's repaint byte is set.
    pub repaint: bool,
    /// Message `2` goes to the message.
    pub post_message: bool,
}

/// The message word bits that, set in either input word, force a repaint.
pub const REPAINT_BITS: u32 = 0x7;
/// The primary message word bits that block the follow-up message.
pub const BLOCKING_BITS: u32 = 0xf0;

/// A running scene's GUI task.
#[derive(Clone, Debug, PartialEq)]
pub struct SceneTask {
    pub scene: u32,
    pub viewport: u32,
    /// The tick was hooked into the task table.
    pub hooked: bool,
    /// The task is paused: its tick does nothing.
    pub paused: bool,
    pub frame: FrameClock,
    /// The viewport's pending and limit counts (signed).
    pub pending: i32,
    pub pending_limit: i32,
    /// The viewport's flag word (its low two bits 2 mean maximised).
    pub viewport_flags: u32,
}

impl SceneTask {
    pub fn new(scene: u32, viewport: u32) -> Self {
        Self {
            scene,
            viewport,
            hooked: false,
            paused: false,
            frame: FrameClock::default(),
            pending: 0,
            pending_limit: 0,
            viewport_flags: 0,
        }
    }

    /// Starts the task: hooks the tick and stamps `now - base`.
    pub fn start(&mut self, now: u32, base: u32) {
        self.hooked = true;
        self.frame = FrameClock::started(now, base);
    }

    /// The tick (the original always returns 1). A paused task does
    /// nothing. Otherwise the frame step runs; the flag byte starts as 1,
    /// takes the scene update's value and is cleared by a busy weather
    /// step. A cleared flag posts the GUI message and repaints; so do both
    /// input words together carrying every [`REPAINT_BITS`] bit. A repaint
    /// sets the viewport's byte. With the pending count below its limit and
    /// no [`BLOCKING_BITS`] in the primary word, message 2 is posted.
    pub fn tick(&mut self, input: &TickInputs) -> TaskTick {
        if self.paused {
            return TaskTick::default();
        }
        let step = self.frame.step(
            input.now,
            input.base,
            input.audio_clock,
            input.viewport_count,
        );
        let flag = input.scene_update_flag && !input.weather_busy;
        let mut tick = TaskTick {
            step: Some(step),
            post_gui_task: !flag,
            repaint: !flag,
            post_message: false,
        };
        if (input.primary | input.secondary) & REPAINT_BITS == REPAINT_BITS {
            tick.repaint = true;
        }
        if self.pending < self.pending_limit && input.primary & BLOCKING_BITS == 0 {
            tick.post_message = true;
        }
        tick
    }

    /// Destroys the task: unhooks the tick and reports whether the
    /// maximised-viewport count must drop (the viewport flag word's low two
    /// bits are [`MAXIMISED`]).
    pub fn destroy(&mut self) -> bool {
        self.hooked = false;
        self.viewport_flags & 3 == MAXIMISED
    }
}

/// The original's scene-node despawn: a climbing node is replaced by the
/// node under its sector, a character's data and a live database slot are
/// reported for release, and the node is queued for removal. See
/// [`Scene::release_node`].
pub fn despawn_scene_node(scene: &mut Scene, node: usize) -> Released {
    scene.release_node(node)
}

#[cfg(test)]
mod tests {
    use super::*;
    use exa_world::scene_nodes::{NodeKind, PAYLOAD_CHARACTER};

    fn input(now: u32, base: u32) -> TickInputs {
        TickInputs {
            now,
            base,
            audio_clock: 5000,
            viewport_count: 1,
            scene_update_flag: true,
            weather_busy: false,
            primary: 0,
            secondary: 0,
        }
    }

    #[test]
    fn the_cap_constants_are_the_stored_words() {
        assert_eq!(FRAME_CAPPED_MS.to_bits(), 0x4185_5555);
        assert_eq!(FRAME_CAP_MS, 50.0 / 3.0);
        // The stored f32 is below the double cap.
        assert!(f64::from(FRAME_CAPPED_MS) < FRAME_CAP_MS);
    }

    #[test]
    fn a_long_step_is_capped_and_scaled_by_the_double_ratio() {
        let mut clock = FrameClock::default();
        let step = clock.step(1000, 900, 5000, 1);
        assert_eq!(step.dt, FRAME_CAPPED_MS);
        assert_eq!(step.scale, (FRAME_CAP_MS / 100.0) as f32);
        assert!((step.scale - 0.166_666_67).abs() < 1e-7);
        // Half of 16.666666 rounds to 8.
        assert_eq!(step.audio_mark, 5000 - 8);
        // The stamp moved: the same time again is a zero step, and the
        // capped step becomes the previous one.
        let step = clock.step(1000, 900, 5000, 0);
        assert_eq!(step.dt, 0.0);
        assert_eq!(step.scale, 1.0);
        assert_eq!(step.audio_mark, 5000);
        assert_eq!(clock.previous_dt, FRAME_CAPPED_MS);
    }

    #[test]
    fn a_step_just_over_the_cap_is_capped() {
        let mut clock = FrameClock::default();
        // 16 ms passes unchanged; 17 ms does not.
        let step = clock.step(16, 0, 0, 0);
        assert_eq!((step.dt, step.scale), (16.0, 1.0));
        let mut clock = FrameClock::default();
        let step = clock.step(17, 0, 0, 0);
        assert_eq!(step.dt, FRAME_CAPPED_MS);
        assert!((step.scale - 0.980_392).abs() < 1e-6);
    }

    #[test]
    fn the_audio_half_step_rounds_to_even() {
        let mut clock = FrameClock::default();
        // 7 ms: half is 3.5, which rounds to 4 (a floor would give 3).
        assert_eq!(clock.step(7, 0, 100, 1).audio_mark, 96);
        // 5 ms: half is 2.5, which rounds to 2.
        let mut clock = FrameClock::default();
        assert_eq!(clock.step(5, 0, 100, 1).audio_mark, 98);
        // A zero count uses the audio clock as it is.
        let mut clock = FrameClock::default();
        assert_eq!(clock.step(5, 0, 100, 0).audio_mark, 100);
        // Underflow wraps in 32 bits.
        let mut clock = FrameClock::default();
        assert_eq!(clock.step(8, 0, 2, 1).audio_mark, 2u32.wrapping_sub(4));
    }

    #[test]
    fn a_negative_step_becomes_zero() {
        let mut clock = FrameClock::default();
        // now - base is -10 as a signed 64-bit value.
        let step = clock.step(10, 20, 0, 0);
        assert_eq!(step.dt, 0.0);
        assert_eq!(step.scale, 1.0);
        assert_eq!(clock.stamp, -10);
    }

    #[test]
    fn the_camera_switch_creates_once_and_notifies() {
        let mut c = CameraSwitch::default();
        assert!(c.switch(1, Some(4)));
        assert!(c.controller);
        assert_eq!(c.target, Some(4));
        assert_eq!(c.viewport_mode, 1);
        assert!(c.notified);
        assert_eq!(c.left, None);
        // The controller follows a new record, bound to the viewport.
        assert!(!c.switch(2, Some(7)));
        assert_eq!(c.target, Some(7));
        assert!(!c.attachment.editing && c.attachment.bound);
        // Dropping the record detaches the controller, and a new record
        // binds it again.
        assert!(!c.switch(0, None));
        assert_eq!(c.target, None);
        assert!(c.attachment.editing && !c.attachment.bound);
        assert!(!c.switch(1, Some(9)));
        assert!(!c.attachment.editing && c.attachment.bound);
        // Only modes from 3 up have a leave callback.
        c.switch(5, None);
        assert_eq!(c.left, None);
        c.switch(1, None);
        assert_eq!(c.left, Some(5));
        // Without a record nothing is created.
        let mut bare = CameraSwitch::default();
        assert!(!bare.switch(2, None));
        assert!(!bare.controller);
    }

    #[test]
    fn maximised_viewports_count_by_class_and_mode() {
        let mut m = MaximisedViewports::default();
        m.notify(false, 2);
        assert_eq!(m.count, 0);
        m.notify(true, 2);
        assert_eq!(m.count, 1);
        m.notify(true, 1);
        assert_eq!(m.count, 0);
    }

    #[test]
    fn running_a_scene_applies_its_options() {
        let options = SceneRunOptions {
            camera: Some([1, 2, 3, 4, 5, 6, 7]),
            weather: 0x40 | 0x10,
            renderer_minimum: 3,
            listener: true,
            ..SceneRunOptions::default()
        };
        let mut scene = SceneSettings {
            weather_flags: 0x2f,
            renderer_minimum: 5,
            ..SceneSettings::default()
        };
        let run = SceneRun::open("exanima_level_one", &options, &mut scene);
        assert_eq!(run.viewport, VIEWPORT);
        assert_eq!(run.viewport_flags, 0x200c);
        assert_eq!(run.title, "exanima_level_o");
        assert!(run.controller);
        assert_eq!(run.camera_record.unwrap()[6], 7);
        // The minimum only rises, so 5 stays; bits 0x60 are replaced.
        assert_eq!(scene.renderer_minimum, 5);
        assert_eq!(scene.weather_flags, 0x4f);
        assert_eq!(scene.camera_mode, 1);
        assert_eq!(scene.listener, Listener::Own(3));
    }

    #[test]
    fn a_higher_minimum_and_no_options_leave_the_rest_alone() {
        let options = SceneRunOptions {
            renderer_minimum: 9,
            ..SceneRunOptions::default()
        };
        let mut scene = SceneSettings {
            weather_flags: 0x60,
            camera_mode: 4,
            ..SceneSettings::default()
        };
        let run = SceneRun::open("x", &options, &mut scene);
        assert_eq!(scene.renderer_minimum, 9);
        assert_eq!(scene.weather_flags, 0x60);
        assert_eq!(scene.camera_mode, 4);
        assert_eq!(scene.listener, Listener::None);
        assert!(!run.controller && run.camera_record.is_none());
    }

    #[test]
    fn a_listener_without_its_flags_falls_back_to_the_global_one() {
        let options = SceneRunOptions {
            listener: true,
            ..SceneRunOptions::default()
        };
        let mut scene = SceneSettings {
            listener: Listener::Own(0x5),
            ..SceneSettings::default()
        };
        SceneRun::open("x", &options, &mut scene);
        assert_eq!(scene.listener, Listener::Own(0x5));
        scene.listener = Listener::Own(0x0);
        SceneRun::open("x", &options, &mut scene);
        assert_eq!(scene.listener, Listener::Global);
    }

    #[test]
    fn starting_a_task_stamps_the_time() {
        let mut task = SceneTask::new(1, 2);
        task.start(150, 50);
        assert!(task.hooked);
        assert_eq!(task.frame.stamp, 100);
        // The first tick measures from that stamp.
        let tick = task.tick(&input(160, 50));
        assert_eq!(tick.step.unwrap().dt, 10.0);
    }

    #[test]
    fn a_paused_task_tick_does_nothing() {
        let mut task = SceneTask::new(1, 2);
        task.paused = true;
        assert_eq!(task.tick(&input(150, 0)), TaskTick::default());
        assert_eq!(task.frame, FrameClock::default());
    }

    #[test]
    fn a_cleared_flag_posts_to_the_gui_and_repaints() {
        let mut task = SceneTask::new(1, 2);
        let tick = task.tick(&input(10, 0));
        assert!(!tick.post_gui_task && !tick.repaint && !tick.post_message);
        // The weather step clears the flag.
        let mut busy = input(20, 0);
        busy.weather_busy = true;
        let tick = task.tick(&busy);
        assert!(tick.post_gui_task && tick.repaint);
        // So does the scene update clearing its own byte.
        let mut cleared = input(30, 0);
        cleared.scene_update_flag = false;
        assert!(task.tick(&cleared).post_gui_task);
    }

    #[test]
    fn the_message_bits_force_a_repaint_without_a_gui_post() {
        let mut task = SceneTask::new(1, 2);
        let mut a = input(10, 0);
        a.primary = 0x3;
        a.secondary = 0x4;
        let tick = task.tick(&a);
        assert!(tick.repaint && !tick.post_gui_task);
        a.secondary = 0x0;
        assert!(!task.tick(&a).repaint);
    }

    #[test]
    fn the_follow_up_message_needs_room_and_no_blocking_bits() {
        let mut task = SceneTask::new(1, 2);
        task.pending_limit = 2;
        assert!(task.tick(&input(10, 0)).post_message);
        let mut blocked = input(20, 0);
        blocked.primary = 0x10;
        assert!(!task.tick(&blocked).post_message);
        // Only the primary word blocks.
        let mut other = input(30, 0);
        other.secondary = 0xf0;
        assert!(task.tick(&other).post_message);
        // The compare is signed.
        task.pending = -1;
        task.pending_limit = 0;
        assert!(task.tick(&input(40, 0)).post_message);
        task.pending = 2;
        assert!(!task.tick(&input(50, 0)).post_message);
    }

    #[test]
    fn destroying_a_maximised_task_asks_for_the_count_to_drop() {
        let mut task = SceneTask::new(1, 2);
        task.start(0, 0);
        task.viewport_flags = 0x6;
        assert!(task.destroy() && !task.hooked);
        task.viewport_flags = 0x5;
        assert!(!task.destroy());
    }

    #[test]
    fn a_node_despawn_reports_what_it_released() {
        let mut scene = Scene::new((0, 0));
        let top = scene.add_node(NodeKind::Mesh, None);
        let node = scene.add_node(NodeKind::Mesh, Some(top));
        // A character payload also climbs, so the node under the sector
        // is the one released.
        scene.nodes[node].payload_flags = PAYLOAD_CHARACTER;
        scene.nodes[top].payload_flags = PAYLOAD_CHARACTER;
        scene.nodes[top].character = Some(42);
        scene.nodes[top].object_slot = 0x8000_0001;
        let released = despawn_scene_node(&mut scene, node);
        assert_eq!(released.node, top);
        assert_eq!(released.character, Some(42));
        assert_eq!(released.object_slot, Some(0x8000_0001));
        assert!(released.queued);
    }
}
