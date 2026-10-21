//! The live scene editor's task (`RF_SceneEditor`, `TSceneTaskInt`): the
//! per-session 3D view, its camera-mode switch, the RFC import by pack
//! name, the paper-doll viewport's frame step, the frame-timing cap and the
//! scene node despawn.
//!
//! Nothing here opens a window: the model holds the state the original's
//! task object and viewport keep, so the timing rules and the switch order
//! can be tested without an app. `exarust::viewer` supplies the actual
//! camera and the level; [`crate::player`] the camera controllers. The
//! original's global message posts and its window title become the
//! [`TaskTick`] flags a host turns into Bevy events.

use exa_world::scene_nodes::{NodeOwner, Scene};

/// The frame-timing cap: 1/60 s in milliseconds, as the original stores it.
pub const FRAME_CAP_MS: f32 = 16.666_666;
/// The stored capped value (the `f32` the original writes back).
pub const FRAME_CAPPED_MS: f32 = 16.666_666;
/// The scene editor's viewport rectangle, as the original opens it.
pub const VIEWPORT: (i32, i32, i32, i32) = (100, 100, 800, 600);
/// The longest scene title the original copies (characters).
pub const TITLE_CHARS: usize = 15;

/// A camera mode the editor can switch to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CameraMode {
    /// A free-flying view of the level.
    Fly,
    /// The player's chase camera.
    Chase,
    /// A fixed camera.
    Fixed,
    /// A camera driven by game code.
    Scripted,
}

impl CameraMode {
    /// The original's mode word.
    pub fn word(self) -> u32 {
        match self {
            Self::Fly => 0,
            Self::Chase => 1,
            Self::Fixed => 2,
            Self::Scripted => 3,
        }
    }
}

/// The editor's camera state during a session.
#[derive(Clone, Debug, PartialEq)]
pub struct CameraSwitch {
    pub mode: CameraMode,
    /// The navigation controller exists (the original creates it once).
    pub controller: bool,
    /// The followed camera record, when the new mode needs one.
    pub target: Option<u32>,
    /// The viewport's mode word (`+0x1d8`).
    pub viewport_mode: u32,
    /// The GUI was told about the switch.
    pub notified: bool,
    /// The previous mode was detached (the original calls the old mode's
    /// leave callback above mode 2).
    pub detached: bool,
}

impl Default for CameraSwitch {
    fn default() -> Self {
        Self {
            mode: CameraMode::Fly,
            controller: false,
            target: None,
            viewport_mode: 0,
            notified: false,
            detached: false,
        }
    }
}

impl CameraSwitch {
    /// Switches the viewport's camera. `data` is the camera record to
    /// follow (none for the fly mode). An existing controller drops its
    /// target; a new one is created for a mode with data and initialised
    /// with the viewport; the viewport's mode word and the GUI follow.
    pub fn switch(&mut self, mode: CameraMode, data: Option<u32>) -> bool {
        self.detached = self.mode.word() > 2;
        self.mode = mode;
        self.viewport_mode = mode.word();
        let created = match (self.controller, data) {
            (false, Some(target)) => {
                self.controller = true;
                self.target = Some(target);
                true
            }
            (true, Some(target)) => {
                self.target = Some(target);
                false
            }
            (true, None) => {
                self.target = None;
                false
            }
            (false, None) => false,
        };
        self.notified = true;
        created
    }
}

/// The maximised-viewport count the editor keeps: a viewport that becomes
/// maximised (mode 2) adds one, any other mode word removes one.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MaximisedViewports {
    pub count: i32,
}

impl MaximisedViewports {
    /// Notifies a viewport class's mode change.
    pub fn notify(&mut self, is_viewport_class: bool, mode: u32) {
        if !is_viewport_class {
            return;
        }
        if mode == 2 {
            self.count += 1;
        } else {
            self.count -= 1;
        }
    }
}

/// What the editor starts a scene with, from the run-scene options record.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SceneRunOptions {
    /// Start the simulation at once (`+0x24`).
    pub run_physics: bool,
    /// Start with the viewport maximised (`+0x26`).
    pub maximised: bool,
    /// Hide the task bar (`+0x27`).
    pub hide_interface: bool,
    /// Switch the renderer to its wireframe/heat view (`+0x29`).
    pub wireframe: bool,
    /// Create the audio environment (`+0x2a`).
    pub sound: bool,
    /// The camera record to follow (`param_2[1]`), if any.
    pub camera: Option<u32>,
    /// The renderer's minimum quality word (`param_2[4]`).
    pub renderer_minimum: u32,
    /// The render flags (`param_2[2]`): sun and sky bits.
    pub render_flags: u32,
}

impl Default for SceneRunOptions {
    fn default() -> Self {
        Self {
            run_physics: true,
            maximised: false,
            hide_interface: false,
            wireframe: false,
            sound: false,
            camera: None,
            renderer_minimum: 0,
            render_flags: 0,
        }
    }
}

/// A scene the editor has opened in a viewport.
#[derive(Clone, Debug, PartialEq)]
pub struct SceneRun {
    /// The viewport rectangle the original opens.
    pub viewport: (i32, i32, i32, i32),
    /// The viewport's title: the scene name cut to 15 characters.
    pub title: String,
    /// The camera state after the options' camera record was applied.
    pub camera: CameraSwitch,
    /// The maximised viewports after this one asked for the task bar.
    pub maximised: MaximisedViewports,
    /// The simulation was started or reset.
    pub physics_started: bool,
    /// The audio environment was created.
    pub audio: bool,
    /// The renderer's minimum quality was applied.
    pub renderer_minimum: u32,
    /// The scene's sun mode/sky flags were applied.
    pub render_flags: u32,
    /// The object database manager was created (the original keeps one per
    /// scene).
    pub object_database: bool,
}

impl SceneRun {
    /// Opens a scene: the original's run-scene set-up, in its order, without
    /// the GUI objects. A camera record in the options starts the chase
    /// mode; no record leaves the fly view.
    pub fn open(scene_name: &str, options: &SceneRunOptions) -> Self {
        let title: String = scene_name.chars().take(TITLE_CHARS).collect();
        let mut camera = CameraSwitch::default();
        if let Some(record) = options.camera {
            camera.switch(CameraMode::Chase, Some(record));
        }
        let mut maximised = MaximisedViewports::default();
        if options.maximised {
            maximised.notify(true, 2);
        }
        Self {
            viewport: VIEWPORT,
            title,
            camera,
            maximised,
            physics_started: options.run_physics,
            audio: options.sound,
            renderer_minimum: options.renderer_minimum,
            render_flags: options.render_flags,
            object_database: true,
        }
    }
}

/// The name an RFC import looks up: the model name with `.rfc` appended
/// (the original's tile-set and model import both build this entry name).
pub fn rfc_entry_name(name: &str) -> String {
    format!("{name}.rfc")
}

/// A scene task's frame timing: the capped step, the previous step and the
/// audio mark the viewport asks for.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FrameClock {
    /// The previous frame's elapsed time (ms).
    pub previous: f32,
    /// The last elapsed time after the cap (ms).
    pub dt: f32,
    /// The scale applied to time-based work this frame.
    pub scale: f32,
}

impl Default for FrameClock {
    fn default() -> Self {
        Self {
            previous: 0.0,
            dt: 0.0,
            scale: 1.0,
        }
    }
}

/// The result of one frame step.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FrameStep {
    /// The elapsed time after the cap.
    pub dt: f32,
    pub scale: f32,
    /// The game time the audio mark takes.
    pub audio_mark: i64,
    /// The viewport was active (the original's audio environment exists).
    pub active: bool,
}

impl FrameClock {
    /// Steps the clock: `elapsed = now − base − offset`; when it is over
    /// 16.666666 ms the step is capped there and the scale is the cap over
    /// the elapsed time; otherwise the scale is 1 and a negative elapsed
    /// time becomes 0. The audio mark is the frame's game time less half
    /// the step, rounded down, when the viewport is active, else the frame
    /// time itself.
    pub fn step(&mut self, now: i64, base: i64, offset: i64, active: bool) -> FrameStep {
        let elapsed = now - base - offset;
        let mut dt = elapsed as f32 - self.previous;
        if dt <= FRAME_CAP_MS {
            self.scale = 1.0;
            if dt < 0.0 {
                dt = 0.0;
            }
        } else {
            self.scale = FRAME_CAPPED_MS / dt;
            dt = FRAME_CAPPED_MS;
        }
        self.previous = elapsed as f32;
        self.dt = dt;
        let audio_mark = if active {
            now - (dt * 0.5).floor() as i64
        } else {
            now
        };
        FrameStep {
            dt,
            scale: self.scale,
            audio_mark,
            active,
        }
    }
}

/// What a scene task's tick asks the host to do.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TaskTick {
    pub update: bool,
    pub redraw: bool,
    /// The scene's redraw word was set.
    pub repaint: bool,
    /// A task message was posted.
    pub posted: bool,
    pub step: Option<FrameStep>,
}

/// A running scene's GUI task: its scene and viewport, whether it is
/// paused, its frame marks and whether the tick is hooked.
#[derive(Clone, Debug, PartialEq)]
pub struct SceneTask {
    pub scene: u32,
    pub viewport: u32,
    /// The task's start stamp (`+0xc8`), subtracted from the frame time.
    pub offset: u32,
    /// The tick was hooked into the task interface.
    pub hooked: bool,
    /// The task is paused (`+0xd4`): its tick does nothing.
    pub paused: bool,
    /// The viewport's audio environment exists (the audio mark is used).
    pub audio_active: bool,
    pub frame: FrameClock,
    /// The scene's pending render count and limit (`+0x288`, `+0x284`).
    pub pending: i32,
    pub pending_limit: i32,
    /// The task's input/redraw word (`+0x190` and `+400` in the original).
    pub flags: u32,
    /// The viewport's mode word (its low two bits 2 mean maximised).
    pub viewport_mode: u32,
}

impl SceneTask {
    /// The constructor's task.
    pub fn new(scene: u32, viewport: u32) -> Self {
        Self {
            scene,
            viewport,
            offset: 0,
            hooked: false,
            paused: false,
            audio_active: false,
            frame: FrameClock::default(),
            pending: 0,
            pending_limit: 0,
            flags: 0,
            viewport_mode: 0,
        }
    }

    /// Starts the task: remembers the frame stamp and hooks the tick.
    pub fn start(&mut self, now: u32) {
        self.offset = now;
        self.hooked = true;
    }

    /// Computes the frame step (the original's time, its 1/60 cap and the
    /// audio mark) and asks for the view update.
    pub fn tick_frame(&mut self, now: u32, base: u32) -> FrameStep {
        self.frame.step(
            i64::from(now),
            i64::from(base),
            i64::from(self.offset),
            self.audio_active,
        )
    }

    /// The frame tick: paused tasks do nothing; otherwise the frame step
    /// runs, the view and scene update, the weather step's redraw request
    /// repaints the scene, and a pending message is posted when there is
    /// room and no modal input is active. The original's flags: the low
    /// three bits all set force a repaint; `0xf0` blocks the message.
    pub fn tick(&mut self, now: u32, base: u32, weather_redraw: bool, modal: bool) -> TaskTick {
        if self.paused {
            return TaskTick::default();
        }
        let step = self.tick_frame(now, base);
        let mut tick = TaskTick {
            update: true,
            step: Some(step),
            ..TaskTick::default()
        };
        if weather_redraw {
            tick.redraw = true;
        }
        if self.flags & 7 == 7 || weather_redraw {
            tick.repaint = true;
        }
        if self.pending < self.pending_limit && !modal {
            tick.posted = true;
        }
        tick
    }

    /// Destroys the task: unhooks the tick and reports whether the
    /// maximised-viewport count must drop (the viewport mode word's low two
    /// bits are 2).
    pub fn destroy(&mut self) -> bool {
        self.hooked = false;
        self.viewport_mode & 3 == 2
    }
}

/// The original's scene-node despawn: it releases what the node's geometry
/// owns (a character's data or a database slot) and queues the removal, so
/// the caller gets the owner back to free it. An instance is replaced by
/// its master first.
pub fn despawn_scene_node(scene: &mut Scene, node: usize) -> NodeOwner {
    let released = scene.release_node(node);
    released.owner
}

#[cfg(test)]
mod tests {
    use super::*;
    use exa_world::scene_nodes::{NodeKind, NodeOwner};

    #[test]
    fn the_frame_cap_and_audio_mark() {
        let mut clock = FrameClock::default();
        let step = clock.step(1000, 900, 0, true);
        assert_eq!(step.dt, 16.666_666);
        assert_eq!(step.scale, 16.666_666 / 100.0);
        assert_eq!(step.audio_mark, 1000 - 8);
        // The next frame is measured from the previous elapsed time.
        let step = clock.step(1000, 900, 0, false);
        assert_eq!(step.dt, 0.0);
        assert_eq!(step.scale, 1.0);
        assert_eq!(step.audio_mark, 1000);
    }

    #[test]
    fn a_negative_elapsed_time_becomes_zero() {
        let mut clock = FrameClock::default();
        let step = clock.step(10, 20, 0, false);
        assert_eq!(step.dt, 0.0);
        assert_eq!(step.scale, 1.0);
    }

    #[test]
    fn the_camera_switch_creates_once_and_notifies() {
        let mut c = CameraSwitch::default();
        assert!(c.switch(CameraMode::Chase, Some(4)));
        assert!(c.controller);
        assert_eq!(c.target, Some(4));
        assert_eq!(c.viewport_mode, 1);
        assert!(c.notified);
        assert!(!c.switch(CameraMode::Fly, None));
        assert_eq!(c.target, None);
        // A scripted mode detaches the previous one.
        c.switch(CameraMode::Scripted, None);
        c.switch(CameraMode::Chase, None);
        assert!(c.detached);
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
            camera: Some(9),
            maximised: true,
            sound: true,
            renderer_minimum: 3,
            ..SceneRunOptions::default()
        };
        let run = SceneRun::open("exanima_level_one.rfc", &options);
        assert_eq!(run.viewport, VIEWPORT);
        assert_eq!(run.title, "exanima_level_o");
        assert_eq!(run.camera.mode, CameraMode::Chase);
        assert_eq!(run.maximised.count, 1);
        assert!(run.physics_started && run.audio && run.object_database);
        assert_eq!(run.renderer_minimum, 3);
    }

    #[test]
    fn the_rfc_entry_appends_the_extension() {
        assert_eq!(rfc_entry_name("gews_uwa01"), "gews_uwa01.rfc");
        assert_eq!(rfc_entry_name("door"), "door.rfc");
    }

    #[test]
    fn a_paused_task_tick_does_nothing() {
        let mut task = SceneTask::new(1, 2);
        task.start(100);
        task.paused = true;
        let tick = task.tick(150, 0, true, false);
        assert_eq!(tick, TaskTick::default());
        task.paused = false;
        task.pending_limit = 2;
        let tick = task.tick(150, 0, true, false);
        assert!(tick.update && tick.redraw && tick.repaint && tick.posted);
        assert_eq!(tick.step.unwrap().dt, FRAME_CAP_MS);
        task.viewport_mode = 2;
        assert!(task.destroy() && !task.hooked);
        task.viewport_mode = 1;
        assert!(!task.destroy());
    }

    #[test]
    fn a_node_despawn_reports_its_owner() {
        let mut scene = Scene::new((0, 0));
        let node = scene.add_node(NodeKind::Mesh, None);
        scene.nodes[node].owner = NodeOwner::Character(42);
        assert_eq!(
            despawn_scene_node(&mut scene, node),
            NodeOwner::Character(42)
        );
        assert_eq!(scene.removals.len(), 1);
    }
}
