//! The player behaviour's screen cursor for the gamepad's aim stick, as the
//! original keeps it. Rules: `docs/PLAYER.md` (Gamepad aim).
//!
//! **The behaviour tick's aim read** ([`aim_cursor`], each behaviour tick,
//! before the steering) has two branches, picked by the controls' mouse
//! capture (the orbit or Cursor Look, [`crate::player::CursorLook`]):
//!
//! - While the capture holds: a screen cursor the pad was driving is let go,
//!   the pointer warped to its drawn point; the aim stick's raw pair (the
//!   reader stores it unturned while the capture holds) past a squared
//!   length of 0.01 (double) sets the pad latch, which stays until the
//!   capture ends. Latched, Cursor Look's kind turns the cursor follower's
//!   target by `−0.04·x` a tick (double precision) and aligns the camera
//!   behind the raw cursor direction; the orbit's kind turns the camera by
//!   the pair as the camera stick's look does ([`crate::gamepad::look_input`]).
//! - Otherwise: the latch clears. A stick past 0.01 (its `x`, `z` in the
//!   world, squared in double) places its point `250·|aim| + 50` cm out from
//!   the player (the cursor's distance takes that value), and the screen
//!   cursor, starting from the pointer the first time
//!   ([`PadCursor::set`]), eases toward the point's screen projection by
//!   `(0.4·AimSpeed + 0.1)²` (double, then single). A stick back under the
//!   engine's epsilon lets the screen cursor go: the pointer is warped to its
//!   eased point. Between the two the cursor eases toward the raw cursor
//!   direction at its last distance. The point lies 80 cm over the ground.
//!   While the pad drives it the GUI's pointer is the eased point, rounded
//!   (what the cursor's pick reads, [`PadCursor::pointer`]).
//!
//! **The frame interpolation pair** (the behaviour's class slots 19 and 20):
//! each simulation step's post-step copies the eased point into the pair
//! (the previous takes the current, the current the eased point,
//! [`advance_pad_cursor`]), and each frame draws the pair at the render
//! fraction ([`show_pad_cursor`]). The world cursor is drawn at the drawn
//! point, rounded, while the pad drives it.
//!
//! Evidence: the behaviour's aim read, its slots 19 and 20, its screen
//! cursor setter, the controls' getter of the drawn point, the GUI's world
//! cursor draw and the controls' mouse capture, read in the instructions.
//! The gate on the binding table's first byte (the primary button's code,
//! never zero in a loaded table) is kept. [inf: the original projects with
//! the view the frame last drew; Exarust takes the view the frame hands the
//! step ([`PadView`]).]

use bevy::prelude::*;

use crate::player::{ChaseCamera, LookCapture, Player, PlayerBody};
use crate::player_behaviour::Interpolated2;

/// The engine's single-precision epsilon: a stick under it is let go.
const STICK_RELEASE: f32 = crate::player_behaviour::EPSILON;
/// A stick past this squared length (double) drives the cursor.
const STICK_ENGAGED: f64 = 0.01;
/// Cursor Look's pad turn: the stick's `x` times this, a tick (rad).
const PAD_LOOK_TURN: f64 = 0.04;
/// The aim plane over the player's ground (cm).
const AIM_PLANE_CM: f32 = 80.0;
const CM: f32 = 100.0;

/// The behaviour's screen cursor for the pad (logical window pixels).
#[derive(Resource, Debug, Clone, Default, PartialEq)]
pub(crate) struct PadCursor {
    /// The pad drives the screen cursor (the stick moved it outside a
    /// capture).
    pub(crate) active: bool,
    /// The aim stick moved while the controls held the mouse; cleared once
    /// the capture is over.
    pub(crate) latched: bool,
    /// The eased point the frame interpolation advances to.
    pub(crate) target: Vec2,
    /// The frame interpolation pair: the previous and current eased points
    /// and the drawn one.
    pub(crate) pair: Interpolated2,
    /// Latched under Cursor Look: the cursor follower's target the pad
    /// turned it to this tick.
    pub(crate) turned: Option<f32>,
    /// Latched under the orbit: the camera's look the pair asks this tick.
    pub(crate) look: Option<Vec2>,
}

impl PadCursor {
    /// The behaviour's screen cursor setter: the eased point and all three
    /// copies of the pair are `at`.
    pub(crate) fn set(&mut self, at: Vec2) {
        self.target = at;
        self.pair = Interpolated2::at(at);
    }

    /// The GUI's pointer while the pad drives the cursor: the eased point
    /// rounded to the nearest pixel (ties to even, as the conversion the
    /// original uses).
    pub(crate) fn pointer(&self) -> Option<Vec2> {
        self.active.then(|| round_point(self.target))
    }

    /// Where the world cursor is drawn while the pad drives it: the drawn
    /// point, rounded (the controls' getter).
    pub(crate) fn drawn(&self) -> Option<Vec2> {
        self.active.then(|| round_point(self.pair.shown))
    }
}

fn round_point(p: Vec2) -> Vec2 {
    Vec2::new(p.x.round_ties_even(), p.y.round_ties_even())
}

/// What the frame hands the behaviour tick for the screen cursor: the
/// window's pointer (logical pixels, the scripted one when set) and the
/// view the frame last drew (the camera's projection, its transform and
/// the window's size).
#[derive(Resource, Debug, Clone, Default, PartialEq)]
pub(crate) struct PadView {
    pub(crate) pointer: Option<Vec2>,
    pub(crate) view: Option<(Mat4, GlobalTransform, Vec2)>,
}

impl PadView {
    /// A world point (m) on the window (logical pixels), as the game view's
    /// projection puts it.
    fn project(&self, point: Vec3) -> Option<Vec2> {
        let (clip, eye, size) = self.view.as_ref()?;
        crate::player::project_to_window(*clip, eye, *size, point)
    }
}

/// The aim read's pad part, one behaviour tick (see the module).
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub(crate) fn aim_cursor(
    config: Option<Res<crate::hw_config::HardwareConfig>>,
    bindings: Res<crate::bindings::Bindings>,
    pad: Option<Res<crate::gamepad::PlayPad>>,
    status: Option<Res<crate::gamepad::PadStatus>>,
    capture: Option<Res<LookCapture>>,
    view: Option<Res<PadView>>,
    players: Query<(&GlobalTransform, &Children), With<Player>>,
    mut bodies: Query<(&mut PlayerBody, &exa_physics::body::locomotion::Locomotion)>,
    mut cameras: Query<(Entity, &mut ChaseCamera)>,
    mut cursor: ResMut<PadCursor>,
    mut frame: ResMut<exa_physics::sim_app::FrameCommands>,
) {
    cursor.turned = None;
    cursor.look = None;
    // The binding table's first byte: the primary button's code.
    if bindings.key(crate::bindings::Action::Primary) & 0xff == 0 {
        return;
    }
    let Some((feet, children)) = players.iter().next() else {
        return;
    };
    let Some(child) = children.iter().find(|c| bodies.contains(*c)) else {
        return;
    };
    let Ok((mut pb, req)) = bodies.get_mut(child) else {
        return;
    };
    let reads_pad = status.as_ref().is_some_and(|s| s.active()) && pad.is_some();
    let stick = pad
        .as_deref()
        .filter(|_| reads_pad)
        .map_or(Vec3::ZERO, |p| p.frame().aim_world);
    let capture = capture.map_or_else(Default::default, |c| c.0);
    let (raw_yaw, distance) = pb.cursor().unwrap_or((req.desired, 0.0));
    if capture.held {
        if cursor.active {
            cursor.active = false;
            // The pointer goes to the drawn point (the frame's own copy).
            frame.send(|world: &mut World| {
                let Some(to) = world
                    .get_resource::<PadCursor>()
                    .map(|c| round_point(c.pair.shown))
                else {
                    return;
                };
                warp_pointer(world, to);
            });
        }
        // The reader keeps the raw pair while the capture holds.
        let pair = Vec2::new(stick.x, stick.y);
        if f64::from(pair.y * pair.y + pair.x * pair.x) > STICK_ENGAGED {
            cursor.latched = true;
        }
        if cursor.latched {
            if capture.turns {
                let from = pb.follower_target().unwrap_or(req.heading);
                cursor.turned = Some((f64::from(from) - f64::from(pair.x) * PAD_LOOK_TURN) as f32);
                if let Ok((entity, mut camera)) = cameras.single_mut() {
                    let yaw = raw_yaw + std::f32::consts::PI;
                    frame.write(entity, &mut camera, move |c| c.align(yaw));
                }
            } else {
                cursor.look = crate::gamepad::look_input(pair);
            }
        }
        return;
    }
    cursor.latched = false;
    let ground = feet.translation();
    let flat = Vec3::new(stick.x, 0.0, stick.z);
    let length2 = (0.0 + flat.x * flat.x) + flat.z * flat.z;
    let mut want = if f64::from(length2) > STICK_ENGAGED {
        let length = length2.sqrt();
        let reach = crate::player_behaviour::aim_distance(length);
        pb.set_cursor_distance(raw_yaw, reach / CM);
        let point = ground * CM + flat * (1.0 / length) * reach;
        if !cursor.active {
            // The screen cursor starts from the pointer.
            let at = view
                .as_ref()
                .and_then(|v| v.pointer)
                .unwrap_or(cursor.target);
            cursor.set(round_point(at));
            cursor.active = true;
        }
        point
    } else {
        if length2 < STICK_RELEASE && cursor.active {
            cursor.active = false;
            let to = round_point(cursor.target);
            frame.send(move |world: &mut World| warp_pointer(world, to));
        }
        ground * CM + exa_physics::body::locomotion::forward(raw_yaw) * (distance * CM)
    };
    want.y = ground.y * CM + AIM_PLANE_CM;
    if cursor.active
        && let Some(at) = view.as_ref().and_then(|v| v.project(want / CM))
    {
        let speed = config.map_or(0.29, |c| c.live.aim_speed);
        let target = cursor.target;
        cursor.target = crate::player_behaviour::aim_cursor_step(target, at, speed);
    }
}

/// The behaviour's slot 19, each step's post-step: the pair's previous copy
/// takes the current and the current the eased point.
pub(crate) fn advance_pad_cursor(mut cursor: ResMut<PadCursor>) {
    let target = cursor.target;
    cursor.pair.advance(target);
}

/// The behaviour's slot 20, each frame: the drawn point is the pair at the
/// render fraction (no clamp).
pub(crate) fn show_pad_cursor(time: Res<Time<Fixed>>, mut cursor: ResMut<PadCursor>) {
    let t = time.overstep_fraction();
    let shown = cursor.pair.lerp(t);
    cursor.bypass_change_detection().pair.shown = shown;
}

/// The frame's view and pointer for the next step's aim read.
pub(crate) fn take_pad_view(
    scripted: Res<exa_ui::ScriptedPointer>,
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    cameras: Query<(&Camera, &GlobalTransform), With<ChaseCamera>>,
    mut view: ResMut<PadView>,
) {
    let Ok(window) = windows.single() else {
        return;
    };
    let next = PadView {
        pointer: scripted.position(window),
        view: cameras
            .single()
            .ok()
            .map(|(camera, eye)| (camera.clip_from_view(), *eye, window.size())),
    };
    view.set_if_neq(next);
}

/// The OS layer's pointer move (no hold).
fn warp_pointer(world: &mut World, to: Vec2) {
    let mut windows = world.query_filtered::<&mut Window, With<bevy::window::PrimaryWindow>>();
    if let Ok(mut window) = windows.single_mut(world) {
        window.set_cursor_position(Some(to));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gamepad::{PadStatus, PlayPad};
    use exa_physics::body::locomotion::Locomotion;

    /// A player at the origin with a camera 10 m back looking at it, the
    /// pad connected in controller mode, and the aim read and the post-step
    /// advance as the step runs them.
    fn app() -> (App, Entity) {
        let mut app = App::new();
        app.init_resource::<crate::bindings::Bindings>()
            .init_resource::<PadCursor>()
            .init_resource::<LookCapture>()
            .init_resource::<PlayPad>()
            .insert_resource(PadStatus {
                connected: true,
                controller: true,
                auto: false,
            })
            .init_resource::<exa_physics::sim_app::FrameCommands>()
            .add_systems(FixedPreUpdate, aim_cursor)
            .add_systems(FixedPostUpdate, advance_pad_cursor);
        let eye = GlobalTransform::from(
            Transform::from_xyz(0.0, 5.0, -10.0).looking_at(Vec3::ZERO, Vec3::Y),
        );
        app.insert_resource(PadView {
            pointer: Some(Vec2::new(100.0, 200.0)),
            view: Some((
                Mat4::from_cols(
                    Vec4::new(1.6, 0.0, 0.0, 0.0),
                    Vec4::new(0.0, 2.4, 0.0, 0.0),
                    Vec4::new(0.0, 0.0, 0.0, -1.0),
                    Vec4::new(0.0, 0.0, 0.1, 0.0),
                ),
                eye,
                Vec2::new(1200.0, 800.0),
            )),
        });
        let body = app
            .world_mut()
            .spawn((PlayerBody::new(0.0), Locomotion::facing(0.0).0))
            .id();
        app.world_mut()
            .spawn((Player, GlobalTransform::IDENTITY))
            .add_child(body);
        (app, body)
    }

    fn tick(app: &mut App) {
        app.world_mut().run_schedule(FixedPreUpdate);
        app.world_mut().run_schedule(FixedPostUpdate);
    }

    fn aim(app: &mut App, aim: Vec3) {
        let mut frame = crate::gamepad::PlayFrame::default();
        frame.aim_world = aim;
        app.world_mut().resource_mut::<PlayPad>().set_frame(frame);
    }

    /// The stick starts the screen cursor at the pointer, which eases
    /// toward the stick's point by the AimSpeed factor each tick; the step's
    /// post-step moves the pair along; the GUI's pointer is the eased point
    /// rounded; a centred stick lets it go.
    #[test]
    fn the_stick_eases_the_screen_cursor_from_the_pointer() {
        let (mut app, _) = app();
        aim(&mut app, Vec3::new(0.0, 0.0, 1.0));
        tick(&mut app);
        let cursor = app.world().resource::<PadCursor>().clone();
        assert!(cursor.active);
        let view = app.world().resource::<PadView>().clone();
        // 300 cm ahead, 80 cm up.
        let want = view.project(Vec3::new(0.0, 0.8, 3.0)).unwrap();
        let start = Vec2::new(100.0, 200.0);
        let factor = crate::player_behaviour::aim_factor(0.29);
        let expected = start + (want - start) * factor;
        assert!(
            (cursor.target - expected).length() < 1e-3,
            "{cursor:?} {expected}"
        );
        assert_eq!(cursor.pair.current, cursor.target);
        assert_eq!(cursor.pair.previous, start);
        assert_eq!(cursor.pointer(), Some(round_point(expected)));
        // Ticks later it is nearer the stick's point.
        for _ in 0..30 {
            tick(&mut app);
        }
        let near = app.world().resource::<PadCursor>().target;
        assert!((near - want).length() < (expected - want).length());
        aim(&mut app, Vec3::ZERO);
        tick(&mut app);
        assert!(!app.world().resource::<PadCursor>().active);
    }

    /// While the controls hold the mouse the pad's screen cursor is let go
    /// and a moving stick latches; latched under Cursor Look the follower's
    /// target turns by −0.04·x; the latch clears when the capture ends.
    #[test]
    fn a_capture_lets_the_screen_cursor_go_and_latches_the_stick() {
        let (mut app, body) = app();
        aim(&mut app, Vec3::new(0.0, 0.0, 1.0));
        tick(&mut app);
        assert!(app.world().resource::<PadCursor>().active);
        app.world_mut()
            .get_mut::<PlayerBody>(body)
            .unwrap()
            .placed(0.5);
        app.insert_resource(LookCapture(crate::steering::Capture {
            held: true,
            turns: true,
            look: Some(0.5),
        }));
        aim(&mut app, Vec3::new(0.5, 0.0, 0.0));
        tick(&mut app);
        let cursor = app.world().resource::<PadCursor>().clone();
        assert!(!cursor.active && cursor.latched);
        let turned = cursor.turned.unwrap();
        assert_eq!(turned, (0.5f64 - 0.5f64 * 0.04) as f32);
        // The orbit's kind turns the camera instead.
        app.insert_resource(LookCapture(crate::steering::Capture {
            held: true,
            turns: false,
            look: None,
        }));
        aim(&mut app, Vec3::new(1.0, 0.0, 0.0));
        tick(&mut app);
        let cursor = app.world().resource::<PadCursor>().clone();
        assert_eq!(cursor.turned, None);
        assert_eq!(cursor.look, crate::gamepad::look_input(Vec2::new(1.0, 0.0)));
        app.insert_resource(LookCapture::default());
        aim(&mut app, Vec3::ZERO);
        tick(&mut app);
        assert!(!app.world().resource::<PadCursor>().latched);
    }

    /// The frame draws the pair at the render fraction.
    #[test]
    fn the_drawn_point_is_the_pair_at_the_render_fraction() {
        let mut cursor = PadCursor::default();
        cursor.set(Vec2::new(10.0, 10.0));
        cursor.target = Vec2::new(20.0, 30.0);
        cursor.pair.advance(cursor.target);
        cursor.active = true;
        cursor.pair.shown = cursor.pair.lerp(0.25);
        assert_eq!(cursor.pair.shown, Vec2::new(12.5, 15.0));
        // Rounded ties to even.
        assert_eq!(cursor.drawn(), Some(Vec2::new(12.0, 15.0)));
    }
}
