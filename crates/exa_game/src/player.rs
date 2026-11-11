//! The player character, steered as the original steers it, and the
//! original's chase camera. Rules: `docs/PLAYER.md`.
//!
//! Exploration uses tank controls: W walks along the character's heading,
//! A and D steer it (or turn a quarter in place when standing), S brakes
//! and backpedals, and the right mouse button walks toward the cursor
//! (`crate::steering`, which writes the body's locomotion request; the
//! body tick's movement decision walks, stops and turns the body,
//! `crate::stages::movement`). The steering starts only the walk, run and
//! backpedal; what a standing character plays is its stance's. The
//! camera follows the character's pelvis only. It turns toward the body
//! only while CamFollow is set, which starts clear; otherwise the middle
//! mouse button orbits it and the wheel zooms by field of view.
//! Combat mode (Tab, or the start of a fight) turns toward the cursor and
//! steps relative to it; the left button requests attacks (held, it
//! repeats them; released, the swing checks) with the clips of the moveset
//! the held items pick (`exa_character::held`), started by the original's
//! attack executor (`exa_combat::attack`) and run by its swing clock
//! (`exa_combat::swing`). In combat mode the
//! player stands in its guard until a target comes close or it attacks
//! (`exa_combat::stance`).

use bevy::input::mouse::{AccumulatedMouseMotion, AccumulatedMouseScroll, MouseScrollUnit};
use bevy::prelude::*;
use bevy::window::{CursorGrabMode, CursorOptions, PrimaryWindow};
use exa_anim::Stature;
use exa_anim::{Animator, Ease, Motions};
use exa_character::held::{Hands, HeldMoveset, class};
use exa_combat::aim::{self, HeadTrack};
use exa_combat::attack::{self, AttackKind, AttackMotion, Attacker, Busy, Request, Side, Slot};
use exa_combat::stance::{self, Stance};
use exa_combat::swing::{self, Swing, SwingAt, SwingState};
use exa_combat::{AttackStart, Attacking, Blade, Condition, Fighter};
use exa_physics::body::locomotion::{Locomotion, Locomotor, flags, forward, turn_between, yaw_of};
use exa_physics::body::{BodyTicks, Fallen, GoLimp, MAX_ENERGY, Ragdoll};
use exa_physics::{CharacterBody, Physics};
use exa_ui::CursorCapture;

use crate::bindings::{Action, Bindings};
use crate::nav_camera::{
    align_behind, clamp_pitch, orbit_per_pixel, orbit_pitch, orbit_yaw, pitch_floor, pitch_held,
};

use crate::footwork::{self, StepGate};
use crate::player_behaviour;
pub(crate) use crate::steering::CURSOR_RUN_FROM;
use crate::steering::{Keys, LineAsked, LineWalk, Steering, key_segment};

/// The behaviour tick (the fixed step), per second.
#[cfg(test)]
const TICK_HZ: f32 = exa_physics::STEP_HZ as f32;
/// The crouch falls by this much a tick once its key is let go.
const CROUCH_FALL: f64 = 0.02;
/// Cursor Look turns the facing by 3.6 rad per screen width of horizontal
/// mouse motion.
pub const CURSOR_LOOK_SENSITIVITY: f32 = 3.6;

/// The controls' mouse capture: the orbit (middle button) and Cursor Look
/// (V) are one routine in the original. Holding either captures the mouse
/// and re-centres the pointer every frame; the kind is fixed at the start
/// (Cursor Look's when its key is down then, else the orbit's). Letting
/// both go ends it: the camera's drag gets its close message and the
/// pointer is warped, with the OS layer's hold, to the screen point of the
/// player's aim point, or back where it was when the capture began over
/// the GUI.
#[derive(Resource, Debug, Default)]
pub struct CursorLook {
    /// The key or button is held and the mouse is captured.
    pub active: bool,
    /// The capture is Cursor Look's: the mouse turns the look. Else it is
    /// the orbit's: the mouse drags the camera.
    pub turns: bool,
    /// The capture began this frame (the orbit drags from the next).
    pub began: bool,
    /// The capture ended this frame.
    pub ended: bool,
    /// The window pointer before capture.
    saved: Option<Vec2>,
    /// The capture began over the GUI: the pointer goes back to `saved`.
    over_gui: bool,
    /// Cursor Look's target yaw: the cursor follower's target, turned by
    /// the mouse.
    target: Option<f32>,
    /// Spectating, the look's flat direction (a yaw) and distance (m) from
    /// the camera's look-at point, measured at the start.
    spectate: Option<(f32, f32)>,
    /// The GUI's pointer while the capture holds: the aim point's screen
    /// point, or `saved`.
    pointer: Option<Vec2>,
    /// The view and the reference point (the player's feet, or the
    /// spectating camera's look-at point) at the end of the last frame.
    last: Option<(GlobalTransform, Vec3)>,
}

impl CursorLook {
    /// The facing turn for `dx` pixels across a window `width` wide (rad).
    pub fn turn(dx: f32, width: f32) -> f32 {
        dx * CURSOR_LOOK_SENSITIVITY / width.max(1.0)
    }

    /// The orbit holds the mouse: the camera is dragged.
    pub fn orbits(&self) -> bool {
        self.active && !self.turns
    }

    /// What the behaviour tick reads of the capture.
    pub(crate) fn capture(&self) -> crate::steering::Capture {
        crate::steering::Capture {
            held: self.active,
            look: self.target.filter(|_| self.active && self.turns),
        }
    }
}

/// The controls' capture as the simulation's behaviour tick reads it,
/// written by the frame when it changes.
#[derive(Resource, Debug, Default, Clone, Copy, PartialEq)]
pub(crate) struct LookCapture(pub(crate) crate::steering::Capture);

/// The original's aim point (m): the point at the aim plane's height
/// (`AIM_HEIGHT` over the ground under `base`) that lies `distance` along
/// the flat direction `yaw` from `base`.
pub(crate) fn aim_point(base: Vec3, ground: f32, yaw: f32, distance: f32) -> Vec3 {
    Vec3::new(base.x, ground + AIM_HEIGHT, base.z) + forward(yaw) * distance
}

/// Where a ray from `origin` along `dir` meets the level plane at `height`
/// (m), as the original's pick computes it: the parameter is the plane's
/// height less the origin's over the direction's height, with no test of
/// its sign. None when it is not finite.
pub(crate) fn level_plane_hit(origin: Vec3, dir: Vec3, height: f32) -> Option<Vec3> {
    let t = (height - origin.y) / dir.y;
    let at = origin + dir * t;
    at.is_finite().then_some(at)
}

/// The flat direction (a yaw) and length (m) from `base` to `point`, the
/// height ignored; none for a zero length.
pub(crate) fn flat_offset(base: Vec3, point: Vec3) -> Option<(f32, f32)> {
    let d = (point - base).with_y(0.0);
    let length = d.length();
    (length > 0.0 && length.is_finite()).then(|| (yaw_of(d), length))
}

/// Without a navigation grid the look-ahead's line walk casts from this
/// far above the feet (hip height, m), and clamps its goal a grid cell (m)
/// more than [`NO_GRID_CLEARANCE`] short of what it meets.
const HIP: f32 = 0.9;
const GRID_CELL: f32 = 0.3;
/// The cursor's point lies on the plane this far above the player's ground
/// (m): the original's aim plane, 80 cm.
pub(crate) const AIM_HEIGHT: f32 = 0.8;
/// A press this soon (ms of the game clock) after the one before is a
/// double click.
const DOUBLE_CLICK_MS: u64 = 300;
/// A cursor further left of the facing than this (rad) attacks on the
/// other side: 20°, or 15° for weapon class 0.
const OTHER_SIDE: f32 = 20.0 * std::f32::consts::PI / 180.0;
const OTHER_SIDE_EMPTY: f32 = 15.0 * std::f32::consts::PI / 180.0;
/// A released main-side attack returns to the stance with this blend
/// (frames) when the fighter has no primary striker, else twice its stance
/// blend.
const RETURN_BLEND: u32 = 30;

/// The player's physics root (origin at the feet). The audio listener is
/// anchored on it, as the original anchors its listener on the player.
#[derive(Component, Debug, Clone)]
#[require(exa_audio::ListenerAnchor)]
pub struct Player;

/// The visual body under the player root. The player is human and is
/// kept in underwear, as the original keeps its player's record; what its
/// hands hold picks its moveset, and it keeps a combat stance. Its heading
/// and look are its locomotion request's, its yaw and run blend its body
/// tick's (`exa_physics::body::locomotion`).
#[derive(Component, Debug, Clone)]
#[require(
    exa_character::WearsUnderwear,
    Hands,
    Stance,
    Locomotion,
    exa_items::possession::Possessions,
    crate::player_practice::PlayerPractice,
    exa_ai::combatant::PlayerContacts
)]
pub struct PlayerBody {
    pub combat: bool,
    /// In combat mode, the cursor's direction from the player (a yaw): the
    /// footwork steps along and across it.
    pub aim: f32,
    /// The attack request from the left button, and the attack picks.
    attack: AttackInput,
    /// The exploration behaviour's state between ticks.
    steering: Steering,
    /// The combat footwork's movement word: a tap moves for up to 200 ms
    /// after the press, and a word held over 200 ms is flagged a tap.
    move_word: crate::steering::MoveWord,
    /// The combat footwork's view of its body's step, read from the body
    /// before the dispatcher and written back after it.
    step: crate::footwork::StepRecord,
    /// The behaviour's combat mode word and the button branch it took last.
    mode: player_behaviour::CombatMode,
    mode_button: player_behaviour::ModeButton,
    /// When its last attack last ticked (s, on the fixed clock).
    last_attack: Option<f32>,
    /// The opponent the behaviour's auto-target picked this tick: what the
    /// aim point's bone lock reads. Only in combat mode.
    target: Option<Entity>,
    /// How far it crouches, 0 to 1.
    crouch: f32,
    /// The head tracker's two relaxed angles, kept between the stabs that
    /// run it.
    head: HeadTrack,
    /// The body's race hovers (the spirit's wraith): combat mode stays off.
    hovers: bool,
    /// The player's controls were off at its last behaviour tick: their
    /// switch off clears the move request once.
    controls_off: bool,
    /// The player's controls were blocked at its last behaviour tick: the
    /// block clears the locomotion flags and the moving byte once.
    controls_blocked: bool,
}

impl PlayerBody {
    /// The behaviour's last line walk on the navigation grid (the look-ahead,
    /// a soft stop or a footwork step), for the navigation overlay.
    pub(crate) fn line_asked(&self) -> Option<LineAsked> {
        self.steering.line
    }

    /// A body whose combat aim starts along `yaw` (its locomotion's facing
    /// is [`Locomotion::facing`]'s).
    pub fn new(yaw: f32) -> Self {
        Self {
            combat: false,
            aim: yaw,
            attack: AttackInput::default(),
            steering: Steering::default(),
            move_word: crate::steering::MoveWord::default(),
            step: crate::footwork::StepRecord::default(),
            mode: player_behaviour::CombatMode::Idle,
            mode_button: player_behaviour::ModeButton::None,
            last_attack: None,
            target: None,
            crouch: 0.0,
            head: HeadTrack { yaw, pitch: 0.0 },
            hovers: false,
            controls_off: false,
            controls_blocked: false,
        }
    }

    /// A body whose race hovers (the spirit's wraith): the combat toggle
    /// keeps it out of combat mode, as the original's does for a player
    /// body with the hover flag.
    pub(crate) fn hovering(mut self) -> Self {
        self.hovers = true;
        self
    }

    /// The opponent the behaviour's auto-target picked this tick (the
    /// original's stored target), which the aim point's bone lock reads.
    pub(crate) fn aim_target(&self) -> Option<Entity> {
        self.target
    }

    /// How far it crouches, 0 to 1.
    pub fn crouch(&self) -> f32 {
        self.crouch
    }

    /// The opponent the auto-target is holding, if it has one.
    pub(crate) fn foe(&self) -> Option<Entity> {
        self.target
    }

    /// Drops the stored combat target. The next fight's behaviour reset does
    /// this while combat mode stays on; the stance tick only drops the target
    /// when combat mode is off.
    pub(crate) fn forget_target(&mut self) {
        self.target = None;
    }

    /// A body whose stored combat target is `target`.
    #[cfg(test)]
    pub(crate) fn with_target(mut self, target: Entity) -> Self {
        self.target = Some(target);
        self
    }

    /// A body whose behaviour has already counted `ticks` toward the spawn
    /// snap: the spirit body's behaviour starts at 4, past the third tick's
    /// snap, so it is never snapped onto the grid
    /// ([`crate::mind::runtime`]).
    pub(crate) fn with_snap_ticks(mut self, ticks: u8) -> Self {
        self.steering.ticks = ticks;
        self
    }

    /// The behaviour's spawn-snap count.
    #[cfg(test)]
    pub(crate) fn snap_ticks(&self) -> u8 {
        self.steering.ticks
    }

    /// A body crouching by `crouch`.
    #[cfg(test)]
    pub(crate) fn crouching(mut self, crouch: f32) -> Self {
        self.crouch = crouch;
        self
    }

    /// The walk to the cursor is asked by the right button (not the keys'
    /// walk, nor a released walk finishing its stride): the cursor shows its
    /// chevrons.
    pub(crate) fn walking_to_cursor(&self) -> bool {
        self.steering.requested && self.steering.mode != crate::steering::CursorMode::Other
    }

    /// The cursor's last direction (a yaw) and distance (m) as the
    /// behaviour read it, once it was 50 cm or more away.
    pub(crate) fn cursor(&self) -> Option<(f32, f32)> {
        self.steering.raw
    }

    /// The left button starts or stops holding an object
    /// ([`crate::hand`]): while it holds one, the standing quarter turns
    /// wait.
    pub(crate) fn hold_object(&mut self, holds: bool) {
        self.steering.holds_object = holds;
    }

    /// Turns combat mode on or off, as Tab does. True when this enters
    /// combat mode.
    pub fn toggle_combat(&mut self) -> bool {
        self.set_combat(!self.combat)
    }

    /// Turns combat mode on (as the original does at the start of every
    /// practice and arena fight) or off. True when this enters combat mode:
    /// the double-click clock is dropped, and the caller clears the
    /// locomotion bits [`Locomotion::entered_combat`] does. A mode already
    /// on is left as it is, clock included. A hovering body's request is
    /// always off.
    pub fn set_combat(&mut self, on: bool) -> bool {
        let on = on && !self.hovers;
        let entered = on && !self.combat;
        if entered {
            self.attack.last_press = None;
        }
        self.combat = on;
        entered
    }
}

/// The original's attack request from the left button in combat mode, kept
/// each behaviour tick: a press asks for an attack (an overhead when it
/// comes within 300 ms of the press before, the other side when the cursor
/// lies left of the facing), holding the button keeps the request standing
/// and releasing it drops it.
#[derive(Debug, Default, Clone)]
struct AttackInput {
    /// The button was down last tick.
    down: bool,
    /// An attack is requested: pressed in combat mode and still held.
    requested: bool,
    /// The request is a double click's (an overhead) while it stands.
    double: bool,
    /// Alt asks for a thrust (only with a held weapon).
    thrust: bool,
    /// The other side, as the cursor lay at the press; every attack
    /// started clears it.
    other_side: bool,
    /// The game clock (ms) of the last press that was not a double click's
    /// second.
    last_press: Option<u64>,
    /// The standing request waits for the energy to come back (logged once).
    dazed: bool,
}

impl AttackInput {
    /// One behaviour tick of the button, down or not (`down`), at `now`
    /// (ms of the game clock); `other_side` is where the cursor lies
    /// against the facing. A new press requests an attack and, within
    /// 300 ms of the press before (which then no longer counts), an
    /// overhead; the side is taken at the press; a release drops the
    /// request.
    fn tick(&mut self, down: bool, now: u64, other_side: bool) {
        if !down {
            *self = Self {
                last_press: self.last_press,
                ..Self::default()
            };
            return;
        }
        if !self.down {
            let double = self.last_press.is_some_and(|t| now < t + DOUBLE_CLICK_MS);
            self.last_press = if double { None } else { Some(now) };
            self.double |= double;
            self.other_side = other_side;
            self.requested = true;
        }
        self.down = true;
    }

    /// What the standing request asks of the executor.
    fn request(&self) -> Request {
        Request {
            overhead: self.double,
            thrust: self.thrust,
            other_side: self.other_side,
        }
    }
}

/// The left button as the behaviour ticks see it: a press since the last
/// tick counts even when it was let go before a tick ran (a click within a
/// frame that ran no tick).
#[derive(Resource, Default, Debug, Clone)]
struct AttackButton {
    pressed_since_tick: bool,
}

/// Before the frame's ticks: remembers a press for the next tick.
fn latch_attack_button(
    buttons: Res<ButtonInput<MouseButton>>,
    bindings: Res<Bindings>,
    status: Option<Res<crate::gamepad::PadStatus>>,
    pad: Option<ResMut<crate::gamepad::PlayPad>>,
    mut latch: ResMut<AttackButton>,
    mut fixed: Option<ResMut<exa_physics::sim_app::FixedCommands>>,
    control: Option<Res<crate::player_controls::PlayerControl>>,
) {
    // The controls off, a press is the open form's; blocked, the
    // behaviour that takes it is suspended.
    if crate::player_controls::PlayerControl::input_suspended(control.as_deref()) {
        return;
    }
    let mut pressed = bindings.mouse_just_pressed(&buttons, Action::Primary);
    if status.as_ref().is_some_and(|s| s.active())
        && let Some(mut pad) = pad
    {
        pressed |= pad.take_attack_press();
    }
    // The latch is the player tick's (it clears it): the press reaches it
    // as a posted write.
    if pressed {
        exa_physics::sim_app::write_resource(fixed.as_deref_mut(), &mut latch, |l| {
            l.pressed_since_tick = true;
        });
    }
}

/// The combat-mode heading toward a cursor at yaw `aim`: the cursor's
/// direction once [`crate::player_behaviour::look_snaps`] (strictly over
/// 30°, the gap wrapped into ±π in double precision), or at once while the
/// left button is held (an attack request stands); else unchanged.
fn combat_heading(heading: f32, aim: f32, requesting: bool) -> f32 {
    if requesting || crate::player_behaviour::look_snaps(heading, aim) {
        aim
    } else {
        heading
    }
}

/// Whether a cursor at yaw `cursor` lies far enough left of a body facing
/// `yaw` to attack on the other side: more than 20°, or 15° for weapon
/// class 0 (a body holding nothing at all, or a shield or an off-hand
/// light beside the fists, whose class has no hands bits).
fn other_side(cursor: f32, yaw: f32, weapon_class: u8) -> bool {
    let beyond = if weapon_class == class::NONE {
        OTHER_SIDE_EMPTY
    } else {
        OTHER_SIDE
    };
    turn_between(yaw, cursor) > beyond
}

/// The point under the cursor on the player's aim plane, for the right
/// mouse button, combat facing and the world cursor.
#[derive(Resource, Default, Debug, Clone)]
pub(crate) struct CursorGround(pub(crate) Option<Vec3>);

/// The cursor's aim pick reaches this far along its ray (m), as the
/// navigation pick does.
const AIM_PICK_RANGE: f32 = 40.0;

/// The skeleton's bone numbers the aim point's bone lock reads: the head,
/// the belly and the pelvis, three points away from the chest it locks
/// from. [guess: the original reads the target's head record's own points;
/// Exarust stands these three part origins in for them.]
const AIM_BONES: [usize; 3] = [10, 8, 1];

/// The aim point with a locked target: the target's chest as the lock's
/// root and three of its bones, projected to the window, with the pixel
/// radius 80 cm makes at the root's depth. Returns the locked height when a
/// bone lies within twice the radius of the cursor.
fn aim_locked_height(
    plane: f32,
    cursor: Vec2,
    camera: &Camera,
    eye: &GlobalTransform,
    window: &Window,
    projection: &Projection,
    target: (&GlobalTransform, &Ragdoll),
) -> Option<f32> {
    let (gt, ragdoll) = target;
    let root = stance::chest_point(gt, Some(ragdoll));
    let root_screen = window_point(camera, eye, window, root)?;
    let Projection::Perspective(perspective) = projection else {
        return None;
    };
    // The projection's x scale, 1 / (aspect·tan(fov/2)): the pixels one
    // unit makes at depth one, as the original's projection scale.
    let aspect = perspective.aspect_ratio;
    let aspect = if aspect.is_finite() && aspect > 0.0 {
        aspect
    } else {
        window.width() / window.height().max(1.0)
    };
    let scale = 1.0 / (aspect * (perspective.fov * 0.5).tan());
    let depth = (root - eye.translation()).dot(*eye.forward());
    let radius = crate::player_behaviour::bone_lock_radius(window.width(), scale, depth);
    let bones: Vec<crate::player_behaviour::AimBone> = AIM_BONES
        .iter()
        .filter_map(|bone| {
            let part = ragdoll.parts.iter().find(|p| p.bone == *bone)?;
            let at = Vec3::from(part.transform().translation);
            Some(crate::player_behaviour::AimBone {
                screen: window_point(camera, eye, window, at)?,
                height: at.y,
            })
        })
        .collect();
    let (height, locked) = crate::player_behaviour::aim_lock_bones(
        plane,
        crate::player_behaviour::AimBone {
            screen: root_screen,
            height: root.y,
        },
        &bones,
        cursor,
        radius,
    );
    locked.then_some(height)
}

/// The aim point each frame: the cursor's ray on the plane 80 cm over the
/// player's ground, its height locked onto the auto-target's bones when one
/// shows within twice the pixel radius 80 cm makes at the target's depth,
/// else raised onto the surface the cursor's pick meets when that lies
/// above the plane's point and under the player's ground plus 200 cm.
///
/// The aim plane's normal is the world's up, as the original's; the pick is
/// the level's fixed geometry (the original's navigation pick also admits
/// non-solid nodes; Exarust's fixed colliders stand in).
#[allow(clippy::too_many_arguments)]
pub(crate) fn cursor_ground(
    mut ground: ResMut<CursorGround>,
    scripted: Res<exa_ui::ScriptedPointer>,
    look: Res<CursorLook>,
    windows: Query<&Window, With<PrimaryWindow>>,
    cameras: Query<(&Camera, &GlobalTransform, &Projection), With<ChaseCamera>>,
    players: Query<(&GlobalTransform, Option<&Children>), With<Player>>,
    bodies: Query<&PlayerBody>,
    targets: Query<(&GlobalTransform, Option<&Ragdoll>)>,
    physics: Option<Res<exa_physics::collision_share::FramePhysics>>,
) {
    ground.0 = None;
    let (Ok(window), Ok((camera, eye, projection)), Some((player, children))) =
        (windows.single(), cameras.single(), players.iter().next())
    else {
        return;
    };
    let body = children.and_then(|c| c.iter().find_map(|child| bodies.get(child).ok()));
    // While the controls hold the mouse the GUI's pointer is the aim
    // point's screen point (`cursor_look`), not the re-centred one.
    let pointer = look.pointer.filter(|_| look.active);
    let Some(cursor) = pointer.or_else(|| scripted.position(window)) else {
        return;
    };
    let Some(ray) = window_ray(camera, eye, window, cursor) else {
        return;
    };
    let ground_y = player.translation().y;
    let at = |height: f32| {
        let hit = crate::nav_pick::plane_hit(
            crate::nav_pick::Ray::new(ray.origin, *ray.direction),
            crate::nav_pick::Plane::new(Vec3::Y, height),
        );
        (hit.distance > 0.0 && hit.point.is_finite()).then_some(hit.point)
    };
    let plane = ground_y + AIM_HEIGHT;
    let Some(plane_point) = at(plane) else {
        return;
    };
    let locked = body.and_then(|pb| {
        let target = pb.aim_target()?;
        let (gt, ragdoll) = targets.get(target).ok()?;
        aim_locked_height(
            plane,
            cursor,
            camera,
            eye,
            window,
            projection,
            (gt, ragdoll?),
        )
    });
    if let Some(height) = locked {
        ground.0 = at(height);
        return;
    }
    let hit = physics
        .as_ref()
        .and_then(|physics| physics.fixed_hit(ray.origin, *ray.direction, AIM_PICK_RANGE))
        .filter(|distance| *distance > 0.0)
        .map(|distance| ray.origin + *ray.direction * distance);
    // The raise rule's 200 cm is in centimetres: the points go to the
    // rule's own units and back.
    const PER_M: f32 = 100.0;
    let raised = crate::player_behaviour::aim_raised(
        plane_point * PER_M,
        hit.map(|at| at * PER_M),
        ground_y * PER_M,
    );
    ground.0 = Some(raised / PER_M);
}

/// The gamepad aim's eased cursor point in the world (m): the stick's
/// direction places the point `250·|aim| + 50` cm out, and it eases toward
/// it by `(0.4·AimSpeed + 0.1)²` a behaviour tick (the original eases the
/// screen cursor and picks its point on the aim plane; Exarust eases the
/// world point by the same factor).
#[derive(Resource, Default, Debug, Clone)]
pub(crate) struct AimCursor(pub(crate) Option<Vec3>);

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        exa_physics::sim_app::add_frame_commands(app);
        bridge_player_types(app);
        app.init_resource::<CursorGround>()
            .init_resource::<AttackButton>()
            .init_resource::<Bindings>()
            .init_resource::<CursorLook>()
            .init_resource::<LookCapture>()
            .init_resource::<crate::gamepad::PlayPad>()
            .init_resource::<crate::gamepad::PadStatus>()
            .init_resource::<AimCursor>()
            .init_resource::<CameraKickRng>()
            .init_resource::<crate::player_controls::PlaySession>()
            // The controls unit's mode records (Character and Spectate),
            // registered at unit init in the original.
            .init_resource::<crate::player_controls::GameModes>()
            .add_message::<exa_combat::AttackAskedHook>()
            .add_message::<exa_combat::pain::BlowFelt>()
            .add_message::<exa_combat::pain::ClashFelt>()
            .add_message::<exa_ai::struck::Taught>()
            .add_systems(
                PostUpdate,
                crate::player_practice::take_lessons.after(exa_ai::struck::HitReactions),
            )
            .add_systems(
                RunFixedMainLoop,
                latch_attack_button.in_set(RunFixedMainLoopSystems::BeforeFixedMainLoop),
            )
            .add_systems(
                FixedPreUpdate,
                pick_player_target.before(exa_combat::parry::aim_foe_parries),
            )
            .add_systems(
                FixedPreUpdate,
                (
                    pad_aim_cursor,
                    crate::player_practice::tick_player_practice,
                    request_attacks,
                    steer_player,
                    release_held_wake,
                    take_player_stance,
                )
                    .chain()
                    .after(exa_combat::parry::aim_foe_parries),
            )
            .add_systems(FixedPreUpdate, (follow_activation_centre, follow_body_camera))
            .add_systems(Update, (begin_play_session, end_play_session))
            .add_systems(
                Update,
                (
                    cursor_look,
                    cursor_ground,
                    player_combat,
                    camera_hits,
                    feel_blows,
                    chase_camera,
                )
                    .chain(),
            );
    }
}

/// The data this plugin's fixed-step systems touch, for the simulation's
/// bridge (`exa_physics::sim_bridge`).
fn bridge_player_types(app: &mut App) {
    use exa_physics::sim_bridge::SimBridgeAppExt;
    app.bridge_copy::<Player>()
        .bridge_copy_resource::<bevy::input::ButtonInput<bevy::input::keyboard::KeyCode>>()
        .bridge_copy_resource::<bevy::input::ButtonInput<bevy::input::mouse::MouseButton>>()
        .bridge_move_resource::<exa_ai::presence::ActivationArea>()
        .bridge_copy::<exa_character::Wielded>()
        .bridge_copy_in_resource::<crate::bindings::Bindings>()
        .bridge_copy_in_resource::<crate::experience_live::StoryExperience>()
        .bridge_copy_in_resource::<crate::gamepad::PadStatus>()
        .bridge_copy_in_resource::<crate::gamepad::PlayPad>()
        .bridge_copy_in_resource::<crate::hand::Hand>()
        .bridge_copy_in_resource::<crate::hw_config::HardwareConfig>()
        .bridge_copy_in_resource::<crate::idle_orbit::IdleWatch>()
        .bridge_copy_in_resource::<crate::player::LookCapture>()
        .bridge_snapshot_resource::<crate::player::AimCursor>()
        .bridge_copy_in_resource::<crate::spectate::SpectateSession>()
        .bridge_snapshot_resource::<crate::player::AttackButton>()
        .bridge_copy_in::<crate::player::ChaseCamera>()
        .bridge_copy_in_resource::<crate::player_controls::PlaySession>()
        .bridge_copy_in_resource::<crate::player_controls::PlayerControl>()
        .bridge_copy::<crate::player_practice::PlayerPractice>()
        .bridge_copy::<exa_ai::combatant::PlayerContacts>();
}

/// The start of play: the controls run and the AI's activation centre is
/// the player, its radius 40 m. The player is spawned by the viewer; the
/// first frame it exists is the start.
fn begin_play_session(
    mut session: ResMut<crate::player_controls::PlaySession>,
    players: Query<&Transform, Added<Player>>,
) {
    let Some(at) = players.iter().next() else {
        return;
    };
    session.begin(at.translation);
    info!("start of play: activation radius 40 m");
}

/// The end of play: the controls stop, the activation centre goes back to the
/// origin with its 200 m radius.
fn end_play_session(
    mut session: ResMut<crate::player_controls::PlaySession>,
    mut removed: RemovedComponents<Player>,
) {
    if removed.read().next().is_none() || !session.playing {
        return;
    }
    session.end();
    info!("end of play: activation radius 200 m");
}

/// The player controller's tick: while play runs, the AI's activation
/// centre follows the player, and the AI takes the session's area.
fn follow_activation_centre(
    mut session: ResMut<crate::player_controls::PlaySession>,
    mut frame: ResMut<exa_physics::sim_app::FrameCommands>,
    area: Option<ResMut<exa_ai::presence::ActivationArea>>,
    players: Query<&GlobalTransform, With<Player>>,
) {
    if session.playing
        && let Some(at) = players.iter().next()
        && session.activation_centre != at.translation()
    {
        // The session is the frame's: the centre is written there too.
        let centre = at.translation();
        frame.write_resource(&mut session, move |s| s.activation_centre = centre);
    }
    if let Some(mut area) = area {
        area.centre_cm = session.activation_centre * 100.0;
        area.radius2 = session.activation_radius2;
    }
}

/// One tick of the crouch: it falls by 0.02 while above 0 (the body
/// tick's part, first), then the Crouch key `held` while the body is not
/// `moving`, or while it is `down`, sets it to 1.
fn crouch_tick(crouch: f32, held: bool, moving: bool, down: bool) -> f32 {
    let eased = if crouch > 0.0 {
        (f64::from(crouch) - CROUCH_FALL) as f32
    } else {
        crouch
    };
    if held && (!moving || down) {
        1.0
    } else {
        eased
    }
}

/// Camera viewport pixels per window pixel: a supersampled camera draws
/// into an image larger than the window.
fn viewport_scale(camera: &Camera, window: &Window) -> Vec2 {
    camera
        .logical_viewport_size()
        .map_or(Vec2::ONE, |v| v / window.size().max(Vec2::ONE))
}

/// The camera's ray through a point of the window (logical pixels).
pub(crate) fn window_ray(
    camera: &Camera,
    eye: &GlobalTransform,
    window: &Window,
    at: Vec2,
) -> Option<Ray3d> {
    camera
        .viewport_to_world(eye, at * viewport_scale(camera, window))
        .ok()
}

/// Where a world point shows in the window (logical pixels).
pub(crate) fn window_point(
    camera: &Camera,
    eye: &GlobalTransform,
    window: &Window,
    point: Vec3,
) -> Option<Vec2> {
    camera
        .world_to_viewport(eye, point)
        .ok()
        .map(|p| p / viewport_scale(camera, window))
}

/// Whether the session is a Wayland one, where the pointer can be moved
/// only while it is locked.
fn wayland() -> bool {
    crate::os_pointer::wayland()
}

/// The players the controls' capture reads: the root (feet) and the
/// body's behaviour state and request.
type LookPlayers<'w, 's> =
    Query<'w, 's, (&'static GlobalTransform, &'static Children), With<Player>>;
type LookBodies<'w, 's> = Query<'w, 's, (&'static PlayerBody, &'static Locomotion)>;
type LookCamera<'w, 's> = Query<
    'w,
    's,
    (
        &'static mut ChaseCamera,
        &'static Camera,
        &'static GlobalTransform,
    ),
>;

/// The player's aim point and the yaw the camera aligns behind: the feet,
/// the smoothed cursor direction and the cursor's distance (the original's
/// aim point reads the drawn cursor direction and its last distance).
fn player_aim(players: &LookPlayers, bodies: &LookBodies) -> Option<(Vec3, f32, f32)> {
    let (feet, children) = players.iter().next()?;
    let (pb, req) = children.iter().find_map(|c| bodies.get(c).ok())?;
    let distance = pb.steering.raw.map_or(0.0, |(_, d)| d);
    Some((feet.translation(), req.cursor, distance))
}

/// The controls' frame routine for the mouse capture (the original's one
/// routine for the orbit and Cursor Look), each frame. See [`CursorLook`].
///
/// - Held, the first frame: the kind is fixed, the pointer saved, and
///   whether it began over the GUI; spectating, the look's direction and
///   distance from the camera's look-at point are measured from the
///   pointer's point on the plane 80 cm over it.
/// - Held, later frames: the orbit's drag is the camera's
///   (`chase_camera`); Cursor Look turns its target by 3.6 × Δx / width
///   rad (not spectating; the behaviour tick turns the cursor follower to
///   it) and aligns the camera behind the drawn cursor direction (the
///   spectating look's direction while spectating), every frame.
/// - Every held frame the pointer is put back at the window's centre and
///   the GUI's pointer is the aim point's screen point (or the saved one).
/// - Let go: the drag's close message (`chase_camera`) and the warp.
/// - Neither held nor captured, while the camera still turns to an
///   alignment: the pointer is moved so it stays over the same flat offset
///   from the player as on the view of the frame before.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn cursor_look(
    mut look: ResMut<CursorLook>,
    mut sim_capture: ResMut<LookCapture>,
    keys: Res<ButtonInput<KeyCode>>,
    buttons: Res<ButtonInput<MouseButton>>,
    bindings: Res<Bindings>,
    motion: Res<AccumulatedMouseMotion>,
    pad: Option<Res<crate::gamepad::PlayPad>>,
    pad_status: Option<Res<crate::gamepad::PadStatus>>,
    mut windows: Query<(&mut Window, &mut CursorOptions), With<PrimaryWindow>>,
    (players, bodies): (LookPlayers, LookBodies),
    mut cameras: LookCamera,
    mut capture: ResMut<CursorCapture>,
    mut warp: Option<ResMut<crate::os_pointer::PointerWarp>>,
    (over_gui, spectate, control): (
        Option<Res<crate::panel_live::PointerOverGui>>,
        Option<Res<crate::spectate::SpectateSession>>,
        Option<Res<crate::player_controls::PlayerControl>>,
    ),
) {
    look.began = false;
    look.ended = false;
    let Ok((mut window, mut options)) = windows.single_mut() else {
        return;
    };
    // The pad's camera look takes the frame: the mouse capture is not
    // started or kept while the stick looks, and the pad's own branch saves
    // the view's basis and returns (the stick turns the camera in
    // `chase_camera`).
    let pad_look = pad_status.as_ref().is_some_and(|s| s.active())
        && pad
            .as_ref()
            .is_some_and(|p| crate::gamepad::look_takes_over(true, p.frame().look));
    if pad_look {
        if look.active {
            look.active = false;
            options.grab_mode = CursorGrabMode::None;
            options.visible = true;
            if let Some(saved) = look.saved.take() {
                window.set_cursor_position(Some(saved));
            }
        }
        capture.0 = false;
        sim_capture.set_if_neq(LookCapture(look.capture()));
        return;
    }
    // The original drops the capture without a release while its GUI's
    // focus is off the game view; Exarust's nearest is the window's focus.
    // The controls off, their mouse hook is dropped with its capture (the
    // same drop, with no release); the block pair drops it too.
    if !window.focused || crate::player_controls::PlayerControl::input_suspended(control.as_deref())
    {
        if look.active {
            look.active = false;
            look.target = None;
            look.pointer = None;
            options.grab_mode = CursorGrabMode::None;
            options.visible = true;
        }
        capture.0 = false;
        sim_capture.set_if_neq(LookCapture(look.capture()));
        return;
    }
    let spectating = spectate.as_deref().filter(|s| s.active);
    let look_key = bindings.pressed(&keys, Action::CursorLook);
    let orbit_key =
        bindings.mouse_pressed(&buttons, Action::Orbit) || bindings.pressed(&keys, Action::Orbit);
    let held = look_key || orbit_key;
    let camera = cameras.single_mut().ok();
    let (mut cam, view_camera, view) = match camera {
        Some((cam, camera, view)) => (Some(cam), Some(camera), Some(*view)),
        None => (None, None, None),
    };
    let aim = player_aim(&players, &bodies);
    // The reference point the aim point is measured from and the plane's
    // ground: the player's feet, or spectating the camera's look-at point.
    let reference = match spectating {
        Some(session) => session.focus,
        None => aim.map(|(feet, ..)| feet),
    };
    // The aim point now, as the original's screen point of it reads it.
    let aim_now = |look: &CursorLook| -> Option<Vec3> {
        match spectating {
            Some(session) => {
                let focus = session.focus?;
                let (yaw, distance) = look.spectate?;
                Some(aim_point(focus, focus.y, yaw, distance))
            }
            None => {
                let (feet, yaw, distance) = aim?;
                Some(aim_point(feet, feet.y, yaw, distance))
            }
        }
    };
    if held {
        if !look.active {
            look.active = true;
            look.began = true;
            look.turns = look_key;
            look.saved = window.cursor_position();
            look.over_gui = over_gui.as_deref().is_some_and(|o| o.0);
            look.target = None;
            look.spectate = None;
            if let (Some(session), Some(saved), Some(camera), Some(view)) =
                (spectating, look.saved, view_camera, view.as_ref())
                && let Some(focus) = session.focus
                && let Some(ray) = window_ray(camera, view, &window, saved)
                && let Some(hit) = level_plane_hit(ray.origin, *ray.direction, focus.y + AIM_HEIGHT)
            {
                look.spectate = flat_offset(focus, hit);
            }
            if look.turns && spectating.is_none() {
                // The look's target starts at the follower's own.
                look.target = players.iter().next().and_then(|(_, children)| {
                    children.iter().find_map(|c| {
                        let (pb, req) = bodies.get(c).ok()?;
                        Some(pb.steering.follower.map_or(req.heading, |f| f.target()))
                    })
                });
            }
            // Locked falls back to confined where it is unsupported (X11,
            // Windows). Wayland locks it in place and only moves it while
            // locked.
            options.grab_mode = CursorGrabMode::Locked;
            options.visible = false;
        } else if look.turns && spectating.is_none() {
            let turn = CursorLook::turn(motion.delta.x, window.width());
            if let Some(target) = look.target.as_mut() {
                *target += turn;
            }
        }
        if look.turns
            && let Some(cam) = cam.as_mut()
        {
            // Behind the drawn cursor direction (spectating, the look's),
            // kept within half a turn of the camera's present yaw.
            let yaw = match spectating {
                Some(_) => look.spectate.map(|(yaw, _)| yaw),
                None => aim.map(|(_, yaw, _)| yaw),
            };
            if let Some(yaw) = yaw {
                cam.align(yaw + std::f32::consts::PI);
            }
        }
        if !wayland() {
            let centre = window.size() * 0.5;
            window.set_cursor_position(Some(centre));
        }
        look.pointer = if look.over_gui {
            look.saved
        } else {
            aim_now(&look)
                .zip(view_camera.zip(view.as_ref()))
                .and_then(|(point, (camera, view))| window_point(camera, view, &window, point))
                .or(look.saved)
        };
    } else if look.active {
        look.active = false;
        look.ended = true;
        look.target = None;
        look.pointer = None;
        options.grab_mode = CursorGrabMode::None;
        options.visible = true;
        // The aim point's screen point, unless the capture began over the
        // GUI (or there is no aim point): then where the pointer was.
        let to = if look.over_gui {
            look.saved
        } else {
            aim_now(&look)
                .zip(view_camera.zip(view.as_ref()))
                .and_then(|(point, (camera, view))| window_point(camera, view, &window, point))
                .or(look.saved)
        };
        look.saved = None;
        // The window's position is applied before the grab is released, so
        // Wayland takes it as the lock's hint and leaves the pointer there.
        // Elsewhere the pointer is warped with the original's hold
        // (`os_pointer`), which puts back the moves still on their way.
        if let Some(to) = to {
            match warp.as_deref_mut().filter(|_| !wayland()) {
                Some(warp) => warp.warp(&mut window, to),
                None => window.set_cursor_position(Some(to)),
            }
        }
    } else if cam.as_ref().is_some_and(|c| c.aligning)
        && !wayland()
        && let (Some((last_view, last_reference)), Some(reference)) = (look.last, reference)
        && let (Some(camera), Some(view)) = (view_camera, view.as_ref())
        && let Some(pointer) = window.cursor_position()
        && let Some(ray) = window_ray(camera, &last_view, &window, pointer)
        && let Some(hit) = level_plane_hit(ray.origin, *ray.direction, reference.y + AIM_HEIGHT)
        && let Some((yaw, distance)) = flat_offset(last_reference, hit)
        && let Some(to) = window_point(
            camera,
            view,
            &window,
            aim_point(reference, reference.y, yaw, distance),
        )
    {
        // The camera still swings to the alignment Cursor Look left: the
        // pointer keeps its flat offset from the player.
        window.set_cursor_position(Some(to));
    }
    look.last = view.zip(reference);
    capture.0 = look.active;
    sim_capture.set_if_neq(LookCapture(look.capture()));
}

/// The player's behaviour's release of a held knock-out: while the
/// player's state flags hold the wake (0x40, which the story's revive
/// sets), a mouse button (left or right), a movement key (forward, back,
/// left, right) or the move stick deflected past a squared length of 0.1
/// (compared in double precision) clears it, so the wake can finish and
/// the body get up. The original's behaviour tick tests them in that order
/// every tick, whatever the body does.
#[allow(clippy::too_many_arguments)]
pub(crate) fn release_held_wake(
    keys: Res<ButtonInput<KeyCode>>,
    buttons: Res<ButtonInput<MouseButton>>,
    bindings: Res<Bindings>,
    pad: Option<Res<crate::gamepad::PlayPad>>,
    pad_status: Option<Res<crate::gamepad::PadStatus>>,
    players: Query<&Children, With<Player>>,
    mut flags: Query<&mut exa_combat::StateFlags, With<PlayerBody>>,
    control: Option<Res<crate::player_controls::PlayerControl>>,
) {
    // The controls off, the behaviour reads no key, button or stick; the
    // block pair suspends the behaviour.
    if crate::player_controls::PlayerControl::input_suspended(control.as_deref()) {
        return;
    }
    let held = |action| bindings.pressed(&keys, action);
    let pressed = buttons.pressed(MouseButton::Left)
        || buttons.pressed(MouseButton::Right)
        || held(Action::Forward)
        || held(Action::Back)
        || held(Action::Left)
        || held(Action::Right)
        || match (pad_status.as_deref(), pad.as_deref()) {
            (Some(status), Some(pad)) if status.active() => {
                f64::from(pad.frame().move_world.length_squared()) > HELD_WAKE_STICK2
            }
            _ => false,
        };
    if !pressed {
        return;
    }
    for children in &players {
        for child in children.iter() {
            if let Ok(mut word) = flags.get_mut(child)
                && word.has(exa_combat::StateFlags::HELD)
            {
                word.0 &= !exa_combat::StateFlags::HELD;
                info!("player input releases its held knock-out wake");
            }
        }
    }
}

/// The squared move-stick length past which the stick releases a held
/// knock-out wake (the original's double 0.1).
const HELD_WAKE_STICK2: f64 = 0.1;

/// The player's behaviour each fixed tick, before its body's tick (as the
/// original's behaviour runs before its movement decision): on its third
/// tick with a navigation grid the spawn snap onto the grid's nearest
/// walkable point (across the ground: its height is the capsule's) and
/// the request for the placement on the ground from its floor; the
/// crouch; Shift's sprint latch, the movement request, the cursor and the
/// exploration steering (`crate::steering`), or in combat mode the cursor,
/// the combat branch's part and Exarust's interim footwork; then the clip
/// the steering starts, by the body's last movement decision. Its line
/// walks are the grid's (a ray from the hip without one). While it falls
/// (Exarust's own) or its condition is helpless it is not steered: it asks
/// for no move.
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub(crate) fn steer_player(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    buttons: Res<ButtonInput<MouseButton>>,
    bindings: Res<Bindings>,
    cursor: Res<CursorGround>,
    pad: Option<Res<crate::gamepad::PlayPad>>,
    pad_status: Option<Res<crate::gamepad::PadStatus>>,
    ticks: Option<Res<BodyTicks>>,
    physics: Res<Physics>,
    nav: Option<Res<exa_world::nav::NavGrid>>,
    motions: Option<Res<Motions>>,
    hand: Option<Res<crate::hand::Hand>>,
    idle: Option<Res<crate::idle_orbit::IdleWatch>>,
    mut players: Query<(&CharacterBody, &mut Transform, &Children), With<Player>>,
    mut bodies: Query<(
        &mut PlayerBody,
        &mut Locomotion,
        &Locomotor,
        &mut Animator,
        &Stance,
        Option<&Condition>,
        Has<Fallen>,
        Option<&Fighter>,
        Option<&mut Ragdoll>,
        Has<Attacking>,
    )>,
    (targets, spectate, look, control): (
        Query<(&GlobalTransform, Option<&Ragdoll>, Option<&Stature>), Without<PlayerBody>>,
        Option<Res<crate::spectate::SpectateSession>>,
        Option<Res<LookCapture>>,
        Option<Res<crate::player_controls::PlayerControl>>,
    ),
) {
    let now = ticks.map_or(0, |t| t.millis());
    let capture = look.map_or_else(Default::default, |l| l.0);
    // Spectating, the controls' frame does nothing: no key steers.
    let spectating = crate::spectate::SpectateSession::spectating(spectate.as_deref());
    // The controls off, a form has the keyboard and the mouse: the movement
    // request does not run and the behaviour reads no key, no button and no
    // pad ([`crate::controls_live`]).
    let switched_off = crate::player_controls::PlayerControl::is_off(control.as_deref());
    // Blocked (or second-blocked) the request does not run and the
    // behaviour is suspended, so no input reaches it either.
    let off = crate::player_controls::PlayerControl::input_suspended(control.as_deref());
    let blocked = crate::player_controls::PlayerControl::is_blocked(control.as_deref());
    let suspended = crate::player_controls::PlayerControl::behaviour_suspended(control.as_deref());
    let held = |action| !off && bindings.pressed(&keys, action);
    let input = Keys {
        w: held(Action::Forward),
        a: held(Action::Left),
        s: held(Action::Back),
        d: held(Action::Right),
        shift: held(Action::Run),
        mouse: !off && buttons.pressed(MouseButton::Right) && cursor.0.is_some(),
    };
    let gamepad = match (pad_status.as_deref(), pad.as_deref()) {
        (Some(status), Some(pad)) if status.active() && !off => Some(*pad.frame()),
        _ => None,
    };
    let nearest = |at: Vec3| {
        nav.as_deref()
            .map_or(at, |g| exa_world::nav::nearest_walkable(g, at))
    };
    for (capsule, mut at, children) in &mut players {
        for child in children.iter() {
            let Ok((
                mut pb,
                mut req,
                body,
                mut anim,
                &stance,
                condition,
                fallen,
                fighter,
                mut body_mut,
                swinging,
            )) = bodies.get_mut(child)
            else {
                continue;
            };
            let ragdoll = body_mut.as_deref();
            let state = condition.copied().unwrap_or_default();
            // The controls' switch off clears the behaviour's move request
            // and the locomotion flags, once.
            if switched_off && !pb.controls_off {
                pb.steering.requested = false;
                req.flags = 0;
            }
            pb.controls_off = switched_off;
            // The block clears the locomotion flags and the moving byte,
            // once (the second block leaves them).
            if blocked && !pb.controls_blocked {
                pb.steering.requested = false;
                req.flags = 0;
                req.moving = false;
            }
            pb.controls_blocked = blocked;
            // Where its feet are in the simulation, as its next step starts
            // from.
            let mut standing = capsule.feet(&at);
            // The original's grid exists before its first behaviour tick;
            // Exarust's is built after the level loads, so the snap counts
            // the ticks it has.
            // The frame's origin is the pelvis's height over where it stands.
            let pelvis = ragdoll
                .and_then(|r| r.parts.first())
                .map_or(standing.y, |p| p.transform().translation.y);
            // The original's behaviour ticks are its character's, whose body
            // exists from the spawn on; Exarust's body is built a few frames
            // after the character, so the count waits for it.
            if nav.is_some()
                && ragdoll.is_some()
                && let Some(to) = pb.steering.snap(state, standing.with_y(pelvis), &nearest)
            {
                let by = (to - standing).with_y(0.0);
                info!(
                    "spawn snap: from ({:.3}, {:.3}, {:.3}) {:.3} m across the ground to the nearest walkable point, its floor at {:.3}",
                    standing.x,
                    standing.y,
                    standing.z,
                    by.length(),
                    to.y
                );
                // Then the original places the character on the ground
                // from the floor ([`crate::limp::GroundPlacement`]).
                commands.entity(child).insert(crate::limp::GroundPlacement {
                    floor: to.y,
                    yaw: None,
                });
                if by != Vec3::ZERO {
                    standing += by;
                    at.translation = standing;
                }
            }
            // Exploring, the pad drives the same request the keys do
            // (`gamepad::pad_keys`); in combat mode its stick vector is the
            // footwork's analog word and only its sprint reaches the keys.
            let mut input = input;
            let pad_crouch = gamepad.is_some_and(|g| g.crouch);
            let pad_stick = gamepad.map_or(Vec3::ZERO, |g| g.move_world);
            if let Some(g) = gamepad {
                if pb.combat {
                    input.shift |= g.sprint;
                } else {
                    input = crate::gamepad::pad_keys(input, &g, req.heading);
                }
            }
            // The crouch is kept while down, so it is set before the
            // helpless bodies are left alone.
            let down = fallen || state.off_feet();
            pb.crouch = crouch_tick(
                pb.crouch,
                held(Action::Crouch) || pad_crouch,
                body.moving(),
                down,
            );
            // The block pair suspends the behaviour: its tick stops after
            // its head, before any steering.
            if suspended {
                continue;
            }
            // Not steered while it falls (Exarust's own) or its condition is
            // helpless: down, getting up, collapsing, dead.
            if fallen || state.helpless() || spectating {
                req.moving = false;
                continue;
            }
            let line = |from: Vec3, to: Vec3| line_walk(nav.as_deref(), &physics, from, to);
            // While the idle camera turns the cursor is not read.
            let to_cursor = cursor
                .0
                .filter(|_| idle.as_ref().is_none_or(|i| i.follows_cursor))
                .map(|c| c - standing);
            let mut steering = pb.steering;
            steering.capture = capture;
            steering.body_stepping = ragdoll.is_some_and(Ragdoll::stepping);
            Steering::latch(&mut req, &input);
            if pb.combat {
                steering.aim(&mut req, to_cursor, state.off_feet());
                // The behaviour runs its head tracking only while the body
                // plays the stab (action 11). The eye is the character
                // frame times (−20, 10, 0) cm and the look-at is the aim
                // point; the tracked flat direction becomes the look the
                // sampler reads.
                if swinging
                    && anim.action == "Stab"
                    && let Some(frame) = ragdoll.as_ref().and_then(|r| r.drive.as_ref()?.frame())
                {
                    // The frame and the aim point are in metres; the
                    // tracker works in the original's centimetres, so
                    // both go across and the direction comes back.
                    let offset = Vec3::from(aim::EYE_IN_FRAME_CM);
                    let eye = Vec3::from(frame.translation) * 100.0
                        + Vec3::from(frame.matrix3.x_axis) * offset.x
                        + Vec3::from(frame.matrix3.y_axis) * offset.y
                        + Vec3::from(frame.matrix3.z_axis) * offset.z;
                    let look_at = cursor.0.unwrap_or(standing + forward(req.heading)) * 100.0;
                    let look = aim::track_head(
                        &mut pb.head,
                        forward(req.look).to_array(),
                        eye.to_array(),
                        forward(req.heading).to_array(),
                        look_at.to_array(),
                    );
                    req.look = yaw_of(Vec3::from(look.flat));
                }
                steering.fight(&mut req, body, standing, &line);
                // A tap step's length (m) and the long step a held key aims.
                // The reach already includes one √scale. The legs' extension
                // scales it, then the same √scale again.
                let extension = ragdoll
                    .and_then(|r| r.drive.as_ref())
                    .map(|d| d.balance().extension())
                    .unwrap_or(1.0);
                let reach_cm = fighter.map_or(STEP_LENGTH * 100.0, |f| {
                    exa_formats::weapon_record::step_reach(
                        f.stats.scale,
                        f.stats.feral,
                        f.stats.encumbrance,
                    )
                });
                let encumbrance = fighter.map_or(0.0, |f| f.stats.encumbrance);
                let scale = fighter.map_or(1.0, |f| f.stats.scale);
                let chest = ragdoll
                    .and_then(|r| r.drive.as_ref())
                    .and_then(|d| d.balance().chest())
                    .unwrap_or(standing + Vec3::Y * stance::CHEST_HEIGHT);
                let energy = ragdoll
                    .and_then(|r| r.drive.as_ref())
                    .map_or(MAX_ENERGY, |d| d.energy);
                // The behaviour reads its body's step fields as they are.
                let body_step = ragdoll.and_then(Ragdoll::footwork_step);
                if let Some(fw) = &body_step {
                    pb.step.read_body(fw, now);
                }
                let feet_centre = body_step.map_or(chest, |fw| fw.feet_centre);
                let footing = Footwork {
                    chest,
                    feet_centre,
                    ground: standing.y,
                    reach_cm,
                    extension,
                    encumbrance,
                    scale,
                    scale_root: scale.sqrt(),
                    gate: StepGate {
                        condition: state.0,
                        energy,
                        swinging,
                    },
                    // The behaviour's auto-target stands for the body's
                    // target the goal reads.
                    target: pb.target.and_then(|t| targets.get(t).ok()).map(
                        |(gt, body, stature)| {
                            (stance::chest_point(gt, body), stature.map_or(1.0, |s| s.0))
                        },
                    ),
                    line: &line,
                    nearest: &nearest,
                };
                // The right button walks toward the cursor in combat mode
                // too, however near it is; the keys then keep the footwork.
                // The behaviour's mode word: the right button walks only
                // while the ladder lets it (not down, dazed under 0.85
                // energy, switching or casting). Exarust's own inputs: the
                // pad reaches the footwork as keys (no stick walk), and the
                // player has no dash latch nor the kept action.
                let (mode, button) = player_behaviour::combat_mode(
                    pb.mode,
                    pb.mode_button,
                    &player_behaviour::CombatModeInput {
                        condition: state.0,
                        energy,
                        right: input.mouse,
                        left: bindings.mouse_pressed(&buttons, Action::Primary),
                        swinging,
                        ..Default::default()
                    },
                );
                pb.mode = mode;
                pb.mode_button = button;
                let walks = mode == player_behaviour::CombatMode::Walk;
                steering.combat_request(walks);
                let walked = if walks && let Some(to) = to_cursor {
                    steering.combat_walk(&mut req, body, standing, to, &line)
                } else {
                    false
                };
                if walked {
                    // Exarust's own: the cursor walk owns the capsule, so
                    // it drops the footwork's word and step (the original
                    // keeps ticking its word under the walk, whose walking
                    // condition then lays no step).
                    pb.move_word = crate::steering::MoveWord::default();
                    if anim.looping {
                        let gait = gait_of(body, &req, &anim.action);
                        explore_clip(&mut anim, gait);
                    }
                } else {
                    let started = combat_steps(
                        &input,
                        pad_stick,
                        &mut steering,
                        &mut pb,
                        &mut anim,
                        stance,
                        body.moving(),
                        &footing,
                        now,
                    );
                    // And writes them back: the body walks the step. The
                    // body's target is the behaviour's auto-target, which
                    // its queued-step taker bends a step at.
                    if body_step.is_some()
                        && let Some(r) = body_mut.as_deref_mut()
                    {
                        r.write_footwork(&pb.step.to_body(), started);
                        r.aim_steps(footing.target);
                    }
                }
                pb.steering = steering;
                continue;
            }
            // The movement word is the combat footwork's: exploration reads
            // the keys as they are.
            pb.move_word = crate::steering::MoveWord::default();
            let segment = motions
                .as_deref()
                .filter(|_| GAITS.contains(&anim.action.as_str()))
                .and_then(|m| key_segment(&anim, m));
            // A refused press or a lift leaves the request as it was.
            if !hand.as_ref().is_some_and(|h| h.keeps_the_request()) {
                steering.request(&mut req, body, &input, segment, state);
            }
            steering.aim(&mut req, to_cursor, state.off_feet());
            steering.explore(&mut req, body, &input, now, standing, state, &line);
            pb.steering = steering;
            let gait = gait_of(body, &req, &anim.action);
            explore_clip(&mut anim, gait);
        }
    }
}

/// The look-ahead's line walk from `from` toward `to`: the navigation
/// grid's ([`exa_world::nav::line_walk`]), which ends before the first cell
/// that blocks; without a grid (Exarust's own) a ray from hip height over
/// `from`, a fixed collider on the way blocking the walk and clamping the
/// goal [`NO_GRID_CLEARANCE`] and a grid cell short of it (never behind
/// `from`), at the goal's height as the grid keeps it (a footwork step
/// asks from the ground toward a target at chest height). Returns the
/// goal, clamped, and whether something blocked it.
/// Without a grid the look-ahead stops this far (m) short of a wall, half
/// a body's width (Exarust's own).
const NO_GRID_CLEARANCE: f32 = 0.28;

fn line_walk(
    nav: Option<&exa_world::nav::NavGrid>,
    physics: &Physics,
    from: Vec3,
    to: Vec3,
) -> (Vec3, bool) {
    if let Some(grid) = nav {
        let hit = exa_world::nav::line_walk(grid, from, to);
        return (hit.end, hit.blocked);
    }
    let way = (to - from).with_y(0.0);
    let length = way.length();
    if length < 1e-4 {
        return (to, false);
    }
    let dir = way / length;
    match physics.fixed_hit(from + Vec3::Y * HIP, dir, length) {
        Some(hit) => (
            (from + dir * (hit - NO_GRID_CLEARANCE - GRID_CELL).max(0.0)).with_y(to.y),
            true,
        ),
        None => (to, false),
    }
}

/// The gait clip of a body by its last movement decision: none standing;
/// braking, the gait it plays; else the walk, forward and backwards (the
/// run is an overlay on the walk, and backing runs the walk's clock
/// backwards).
fn gait_of(body: &Locomotor, req: &Locomotion, playing: &str) -> Option<&'static str> {
    if !body.moving() {
        None
    } else if let Some(gait) = GAITS
        .iter()
        .copied()
        .find(|g| *g == playing)
        .filter(|_| req.has(flags::BRAKE))
    {
        Some(gait)
    } else {
        Some("Walk")
    }
}

/// The exploration steering's part in the clip, as the original's: moving,
/// the gait starts from any looping clip; standing, a gait ends in Idle,
/// the exploration stance. A walk or a run blends into it over 20 frames. The steering
/// starts no other clip: the stance does, through the helper that sets it
/// ([`stance::set_stance`]) and the body tick's posture switch, which never
/// leaves Cmbt - Idle. So a player that turns combat mode off in the combat
/// idle with the Defence stance keeps it until it walks.
fn explore_clip(anim: &mut Animator, gait: Option<&str>) {
    if !anim.looping {
        return;
    }
    match gait {
        Some(gait) if anim.action != gait => anim.play_loop(gait),
        None if GAITS.contains(&anim.action.as_str()) => {
            end_gait(anim, Stance::Idle.action());
        }
        _ => {}
    }
}

/// A tap step's length (m) for a character without stats: a human's step
/// reach.
const STEP_LENGTH: f32 = 0.8;

/// Where the footwork steps from, the stride's parts, the dispatcher's
/// gate and the grid's line walk and snap that clamp the goal.
struct Footwork<'a> {
    /// The chest point a step is measured from (m).
    chest: Vec3,
    /// The feet centre (m) a balance step's target is measured from.
    feet_centre: Vec3,
    /// The ground height the line walk starts from (m).
    ground: f32,
    /// The stored step reach (cm): 80·√scale shortened by feral level and
    /// encumbrance, so it already includes one √scale.
    reach_cm: f32,
    /// The legs' extension, which scales the stride.
    extension: f32,
    /// The encumbrance the tapped stride reads.
    encumbrance: f32,
    /// The body's scale and its root, the stride's other factor.
    scale: f32,
    scale_root: f32,
    /// What may lay a step.
    gate: StepGate,
    /// The body's target: its chest (m) and scale, which a lone forward
    /// word's goal bends at ([`footwork::approach_bend`]).
    target: Option<(Vec3, f32)>,
    line: LineWalk<'a>,
    /// The grid's nearest walkable point.
    nearest: &'a dyn Fn(Vec3) -> Vec3,
}

/// Combat footwork, the original's step machine on the body's feet: W, S,
/// A and D (opposing keys cancel) form the movement word along the cursor
/// direction, whose persistence keeps a tap for 200 ms and flags a word
/// held over 200 ms (with the crouch under 0.1) a tap. The dispatcher
/// ([`crate::footwork::dispatch`]) then starts a step toward the movement
/// goal, re-lays a young step at the long stride, or queues the next step
/// while one is under way; a step already started finishes after the keys
/// are let go. The goal is the chest plus the word's direction (a side key
/// at 0.8 of the facing, summed W, A, D, S) scaled by
/// `(stride·extension·√scale)/|dir|` cm, the stride the reach or, tapped,
/// `(1.5 − 0.3·encumbrance)` of it in double (the step's long kind); the
/// line walk from the chest's x/z at the ground clamps it, and it is taken
/// only when over 400 cm² from the chest, then snapped onto the grid.
/// `pb.step` is the body's step as the caller read it, and the caller
/// writes it back; the body then walks it (its stance moves to the target,
/// its feet step there, its chest pins lead toward it, the character
/// follows its frame), ends it and takes a queued step at a landing
/// through its queue rule. The stance clip plays throughout. A walk, run
/// or backpedal of the exploration steering still playing when combat
/// mode turned on ends in the stance once the body stands (`moving`
/// false), a walk or a run over 20 frames. Returns whether a step started.
#[allow(clippy::too_many_arguments)]
fn combat_steps(
    input: &Keys,
    stick: Vec3,
    steering: &mut Steering,
    pb: &mut PlayerBody,
    anim: &mut Animator,
    stance: Stance,
    moving: bool,
    footing: &Footwork,
    now: u64,
) -> bool {
    let facing = forward(pb.aim);
    if pb.step.queue < 0 {
        pb.step.target =
            steering.queued_target(footing.chest, pb.step.target, footing.line, footing.nearest);
    }
    pb.move_word.take_queue(now, &mut pb.step.queue);
    // The pad's stick vector, once its squared length is over 0.01, is the
    // analog word in place of the keys; the vector it was read with is
    // the direction the word's goal and queued step take.
    pb.move_word
        .tick(now, [input.w, input.s, input.a, input.d], stick, pb.crouch);
    let analog = pb.move_word.analog;
    if pb.move_word.queue_expired {
        pb.step.queue = 0;
    }
    let under_way = pb.step.under_way(footing.feet_centre, footing.scale);
    let raw = pb.move_word.raw;
    let mut started = false;
    match footwork::dispatch(&pb.move_word, raw, now, &pb.step, under_way, footing.gate) {
        footwork::StepAsk::Nothing => {}
        footwork::StepAsk::Queue(word) => {
            let (reach, root) = (footing.reach_cm, footing.scale_root);
            pb.step
                .queue_word(&mut pb.move_word, word, facing, analog, reach, root);
        }
        footwork::StepAsk::Start(word) => {
            if lay_goal(pb, footing, facing, analog, word, steering) {
                pb.step.start(now);
                started = true;
            }
        }
        footwork::StepAsk::Relay(word) => {
            lay_goal(pb, footing, facing, analog, word, steering);
        }
    }
    if anim.looping && GAITS.contains(&anim.action.as_str()) && !moving {
        end_gait(anim, stance.action());
    }
    started
}

/// Lays the movement goal for `word`, as the original's goal routine: the
/// word becomes the last laid word; a tapped word lays the long stride and
/// marks the step long; the target is the chest plus the direction scaled
/// by `(stride·extension·√scale)/|dir|` cm, clamped by the line walk and,
/// once over 400 cm² from the chest, snapped onto the grid. Returns
/// whether it was taken; the target is the body's step target either way,
/// as the original's routine writes it in place.
fn lay_goal(
    pb: &mut PlayerBody,
    footing: &Footwork,
    facing: Vec3,
    analog: Vec3,
    word: u32,
    steering: &mut Steering,
) -> bool {
    let dir = footwork::goal_direction(facing, word, analog);
    let length = dir.length();
    pb.move_word.last_word = word;
    let tap = pb.move_word.tap;
    if tap {
        pb.step.long = true;
    }
    let stride = crate::steering::goal_stride(
        footing.reach_cm,
        footing.extension,
        footing.scale_root,
        footing.encumbrance,
        tap,
    );
    let mut step = dir * (stride / length);
    // A lone forward word with a target bends at the target's circle.
    if word == crate::steering::MOVE_FORWARD
        && let Some((target, scale)) = footing.target
    {
        step = footwork::approach_bend(step, footing.chest * 100.0, target * 100.0, scale);
    }
    let wanted = footing.chest + step / 100.0;
    let (target, taken) = steering.step_target(
        footing.chest,
        footing.ground,
        wanted,
        footing.line,
        footing.nearest,
    );
    pb.step.target = target;
    taken
}

/// A walk or a run (the actions that set the walking condition) returns to
/// `action` over 20 frames with the fast ease, as the body tick does when
/// it leaves that condition. A backpedal ends at once.
fn end_gait(anim: &mut Animator, action: &str) {
    if anim.action == "Walk" || anim.action == "Run" {
        anim.start(action, true, None, WALK_END_BLEND, Ease::Fast);
    } else {
        anim.play_loop(action);
    }
}

/// Frames a walk or a run blends into the stance it ends in.
const WALK_END_BLEND: u32 = 20;
/// The exploration steering's looping gaits, which only it ends.
const GAITS: [&str; 3] = ["Walk", "Run", "Backpedal"];

/// The behaviour's auto-target: of the `candidates` (entity, scale, chest
/// point), the visible opponent within 5 m of the player's chest `from`
/// with the best (500 − d)·(cursor·dir + 0.5), the cursor along `aim`. Its
/// visibility is a ray at the fixed level between the two chests; the
/// original also asks its behaviour's own can-see callback, which Exarust
/// does not have.
fn player_auto_target(
    physics: Option<&Physics>,
    from: Vec3,
    aim: f32,
    candidates: &[(Entity, f32, Vec3)],
) -> Option<Entity> {
    let cursor = forward(aim);
    crate::player_behaviour::auto_target(
        from * 100.0,
        cursor,
        candidates.iter().map(|(_, _, chest)| {
            let to = *chest - from;
            let visible = to.length() < 1e-4
                || physics.is_none_or(|physics| {
                    physics
                        .fixed_hit(from, to.normalize_or_zero(), to.length())
                        .is_none()
                });
            crate::player_behaviour::AimCandidate {
                at: *chest * 100.0,
                visible,
                skipped: false,
            }
        }),
    )
    .map(|index| candidates[index].0)
}

/// The other living fighters the player's stance and auto-target choose
/// from (the dead a level places lie limp): each with its body's scale and
/// its chest point.
#[allow(clippy::type_complexity)]
fn target_candidates(
    me: Entity,
    others: &Query<
        (
            Entity,
            &Condition,
            &GlobalTransform,
            Option<&Ragdoll>,
            Option<&Stature>,
        ),
        (With<Fighter>, Without<GoLimp>),
    >,
) -> Vec<(Entity, f32, Vec3)> {
    others
        .iter()
        .filter(|(e, c, _, b, _)| *e != me && !c.is_dead() && b.is_none_or(|b| b.drive.is_some()))
        .map(|(e, _, gt, b, scale)| (e, scale.map_or(1.0, |s| s.0), stance::chest_point(gt, b)))
        .collect()
}

/// The player's combat target as its behaviour tick picks it, before the
/// attack starters aim: the original's behaviour tick runs its auto-target
/// right before its attack executor, so an attack started this tick aims
/// at this tick's pick. It is written to the player's
/// [`exa_combat::CombatTarget`], which the starters' aim reads
/// ([`exa_combat::parry::aim_foe_parries`]); the behaviour's own stored
/// target, which its steering reads before the pick, is still taken with
/// the stance ([`take_player_stance`]). None out of combat mode.
#[allow(clippy::type_complexity)]
pub(crate) fn pick_player_target(
    physics: Option<Res<Physics>>,
    mut bodies: Query<(
        Entity,
        &PlayerBody,
        &Condition,
        &GlobalTransform,
        Option<&Ragdoll>,
        Option<&mut exa_combat::CombatTarget>,
    )>,
    others: Query<
        (
            Entity,
            &Condition,
            &GlobalTransform,
            Option<&Ragdoll>,
            Option<&Stature>,
        ),
        (With<Fighter>, Without<GoLimp>),
    >,
    mut commands: Commands,
) {
    for (me, pb, condition, gt, body, target) in &mut bodies {
        if condition.dying() {
            continue;
        }
        let picked = if pb.combat {
            let from = stance::chest_point(gt, body);
            let candidates = target_candidates(me, &others);
            player_auto_target(physics.as_deref(), from, pb.aim, &candidates)
        } else {
            None
        };
        match target {
            Some(mut target) => {
                if target.0 != picked {
                    target.0 = picked;
                }
            }
            None => {
                commands.entity(me).insert(exa_combat::CombatTarget(picked));
            }
        }
    }
}

/// The player's stance, each behaviour tick: Idle out of combat mode (the
/// clip follows when it was the old stance's); in combat mode the guard,
/// or the combat idle with a target within 3.5 m or within 3 s of its last
/// attack's last tick. The target is the best of the other living fighters
/// within 5 m of the cursor's direction (`exa_combat::stance`); the
/// behaviour's own auto-target picks the same list by the original's score
/// and keeps the entity for the aim point's bone lock. The clip changes in
/// the body tick, never from the combat idle.
#[allow(clippy::type_complexity)]
fn take_player_stance(
    time: Res<Time>,
    physics: Option<Res<Physics>>,
    mut bodies: Query<(
        Entity,
        &mut PlayerBody,
        &mut Stance,
        &mut Animator,
        &Fighter,
        &Condition,
        &GlobalTransform,
        Option<&Ragdoll>,
        Has<Attacking>,
    )>,
    others: Query<
        (
            Entity,
            &Condition,
            &GlobalTransform,
            Option<&Ragdoll>,
            Option<&Stature>,
        ),
        (With<Fighter>, Without<GoLimp>),
    >,
) {
    let now = time.elapsed_secs();
    for (me, mut pb, mut stance, mut anim, fighter, condition, gt, body, attacking) in &mut bodies {
        if condition.dying() {
            continue;
        }
        if attacking {
            pb.last_attack = Some(now);
        }
        if !pb.combat {
            pb.target = None;
            if *stance != Stance::Idle {
                info!("player stance: Idle");
            }
            stance::set_stance(&mut stance, &mut anim, Stance::Idle);
            continue;
        }
        let from = stance::chest_point(gt, body);
        let candidates = target_candidates(me, &others);
        let target = stance::pick_target(
            from,
            forward(pb.aim),
            candidates
                .iter()
                .map(|(e, scale, chest)| ((*e, *scale), *chest)),
        )
        .map(|((_, scale), d)| (d, scale));
        // The behaviour's auto-target ([`player_auto_target`]); the
        // attack starters read the same pick, taken earlier in the step
        // ([`pick_player_target`]).
        pb.target = player_auto_target(physics.as_deref(), from, pb.aim, &candidates);
        // Dazed, its energy more than 0.05 under its most, it guards.
        let dazed = body
            .and_then(|b| b.drive.as_ref())
            .is_some_and(|d| d.dazed());
        let chosen = stance::player_stance(
            true,
            fighter.health.yellow,
            dazed,
            target,
            pb.last_attack.map(|t| now - t),
        );
        // In combat mode the stance is written as it is: the body tick
        // switches the clip.
        if stance.set_if_neq(chosen) {
            info!(
                "player stance: {chosen:?} (target {}, {} s since an attack)",
                target.map_or("none".into(), |(d, _)| format!("at {d:.2} m")),
                pb.last_attack
                    .map_or("never".into(), |t| format!("{:.1}", now - t)),
            );
        }
    }
}

/// The gamepad aim's eased cursor, each behaviour tick while a stick asks
/// for it (its `z·z + x·x` over 0.01): its direction and magnitude place a
/// point `250·|aim| + 50` cm out from the player, which the cursor eases
/// toward by the AimSpeed factor. With no stick the resource is cleared and
/// the mouse's point is used.
fn pad_aim_cursor(
    config: Option<Res<crate::hw_config::HardwareConfig>>,
    pad: Option<Res<crate::gamepad::PlayPad>>,
    status: Option<Res<crate::gamepad::PadStatus>>,
    ground: Res<CursorGround>,
    players: Query<&GlobalTransform, With<Player>>,
    mut aim: ResMut<AimCursor>,
) {
    let active = status.as_ref().is_some_and(|s| s.active()) && pad.is_some();
    let Some(player) = players.iter().next() else {
        aim.0 = None;
        return;
    };
    let stick = active
        .then(|| pad.as_deref().map(|p| p.frame().aim_world))
        .flatten()
        .filter(|aim| crate::gamepad::aim_engaged(*aim));
    let Some(stick) = stick else {
        aim.0 = None;
        return;
    };
    let magnitude = (stick.x * stick.x + stick.z * stick.z).sqrt();
    let want = player.translation()
        + stick.with_y(0.0)
            * (crate::player_behaviour::aim_distance(magnitude) / magnitude / 100.0);
    let speed = config.map_or(0.29, |c| c.live.aim_speed);
    // The first tick starts the ease from the mouse's point, as the
    // original starts its screen cursor from the pointer.
    let current = aim.0.or(ground.0).unwrap_or(want);
    aim.0 = Some(crate::player_behaviour::aim_point_step(
        current, want, speed,
    ));
}

/// When the camera-follow option is set, a yaw more than a tenth of a
/// turn from the body's is put back to the body's yaw, that tenth toward
/// the camera's own side. The option starts clear, so the orbit is left
/// alone.
fn follow_body_camera(
    config: Option<Res<crate::hw_config::HardwareConfig>>,
    mut cameras: Query<(Entity, &mut ChaseCamera)>,
    bodies: Query<&Locomotor, With<PlayerBody>>,
    mut frame: ResMut<exa_physics::sim_app::FrameCommands>,
) {
    if !config.is_some_and(|config| config.live.cam_follow) {
        return;
    }
    let Some(body) = bodies.iter().next() else {
        return;
    };
    let Ok((entity, mut camera)) = cameras.single_mut() else {
        return;
    };
    // The camera is the frame's: its yaw is turned there too.
    if let Some(yaw) = crate::player_behaviour::camera_follow_yaw(camera.yaw, body.yaw) {
        frame.write(entity, &mut camera, move |c| c.yaw = yaw);
    }
}

/// Combat mode (Tab), each frame: the cursor's direction is the aim, and
/// the heading turns to it once it is more than 30° off, or follows it
/// while an attack is requested.
pub(crate) fn player_combat(
    cursor: Res<CursorGround>,
    aim_cursor: Res<AimCursor>,
    look: Res<CursorLook>,
    pad_status: Option<Res<crate::gamepad::PadStatus>>,
    players: Query<&GlobalTransform, With<Player>>,
    mut bodies: Query<(
        &mut PlayerBody,
        &mut Locomotion,
        Option<&Condition>,
        Has<Fallen>,
    )>,
) {
    let Some(player) = players.iter().next() else {
        return;
    };
    let pad_active = pad_status.as_ref().is_some_and(|s| s.active());
    // While the controls hold the mouse the cursor's point is not read:
    // Cursor Look's target is the cursor's direction, the orbit leaves the
    // aim as it was.
    let captured = look.active && !pad_active;
    let look_aim = look.capture().look;
    if captured && look_aim.is_none() {
        return;
    }
    for (mut pb, mut req, condition, fallen) in &mut bodies {
        if fallen || condition.is_some_and(|c| c.helpless()) || !pb.combat {
            continue;
        }
        // The eased aim cursor's direction, once a stick asks for it; the
        // mouse's point under the cursor otherwise.
        let aim = if captured {
            look_aim
        } else {
            aim_cursor
                .0
                .and_then(|point| aim_from(player.translation(), point))
                .or_else(|| {
                    cursor
                        .0
                        .and_then(|point| aim_from(player.translation(), point))
                })
        };
        if let Some(aim) = aim {
            pb.aim = aim;
            req.heading = combat_heading(req.heading, aim, pb.attack.requested);
        }
    }
}

/// The point under the cursor is kept as the combat aim once it is 50 cm or
/// more from the player across the ground (the original's aim direction is
/// only rewritten from 50 cm out, so nearer points keep the last aim).
const AIM_MIN_CM: f32 = 50.0;

/// The combat aim's yaw from a point under the cursor (m), or none while the
/// point is nearer than [`AIM_MIN_CM`].
fn aim_from(from: Vec3, to: Vec3) -> Option<f32> {
    let to = (to - from).with_y(0.0);
    (to.length() >= AIM_MIN_CM / 100.0).then(|| yaw_of(to))
}

/// The player's attack request, each behaviour tick, as the original's: in
/// combat mode a left press requests an attack (on the main side, the
/// other side with the cursor far enough left of the facing, or overhead
/// on a double click; Alt with a held weapon a stab), holding the button
/// keeps it standing (so the attacks repeat) and releasing drops it. While
/// it stands the heading is the cursor's direction, and with at least 80 %
/// of its energy the fighter's executor starts what it asks
/// ([`exa_combat::attack::execute`]). Without a request ([`leave_swing`]),
/// an attack playing is released (its swing stalls) and a released
/// main-side attack returns to the stance once its stall is over.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn request_attacks(
    mut commands: Commands,
    ticks: Option<Res<BodyTicks>>,
    motions: Option<Res<Motions>>,
    keys: Res<ButtonInput<KeyCode>>,
    buttons: Res<ButtonInput<MouseButton>>,
    bindings: Res<Bindings>,
    pad: Option<Res<crate::gamepad::PlayPad>>,
    pad_status: Option<Res<crate::gamepad::PadStatus>>,
    mut latch: ResMut<AttackButton>,
    blades: Query<(&Blade, &GlobalTransform)>,
    mut bodies: Query<(
        Entity,
        (&mut PlayerBody, &mut Locomotion, &Locomotor),
        &mut Animator,
        (&mut Attacker, &mut SwingState, Option<&Swing>),
        &Stance,
        Option<&Fighter>,
        Option<&HeldMoveset>,
        Option<&mut Condition>,
        Option<&Ragdoll>,
        Option<&Attacking>,
        (Has<Fallen>, Option<&exa_combat::AttackHook>),
    )>,
    (mut asked, control): (
        MessageWriter<exa_combat::AttackAskedHook>,
        Option<Res<crate::player_controls::PlayerControl>>,
    ),
) {
    let now = ticks.map_or(0, |t| t.millis());
    // The controls off, the behaviour's button word is empty; the block
    // pair suspends the behaviour that reads it.
    let off = crate::player_controls::PlayerControl::input_suspended(control.as_deref());
    let pad_active = pad_status.as_ref().is_some_and(|s| s.active()) && pad.is_some();
    let down = !off
        && (bindings.mouse_pressed(&buttons, Action::Primary)
            || latch.pressed_since_tick
            || (pad_active && pad.as_deref().is_some_and(|p| p.attack_held())));
    latch.pressed_since_tick = false;
    for (
        entity,
        (mut pb, mut req, body_turn),
        mut anim,
        (mut attacker, mut state, swing),
        stance,
        fighter,
        held,
        mut condition,
        body,
        attacking,
        (fallen, hook),
    ) in &mut bodies
    {
        let state_now = condition.as_deref().copied().unwrap_or_default();
        // A helpless condition engages no attack (the original's 0xAC0), and
        // a casting one hands the buttons to the cast hook instead.
        let able = !fallen && !state_now.helpless() && state_now != Condition::CASTING && pb.combat;
        let weapon_class = held.map_or(class::FISTS, |h| h.weapon_class);
        let other = other_side(pb.aim, body_turn.yaw, weapon_class);
        pb.attack.tick(able && down, now, other);
        let swing = swing.copied().unwrap_or(Swing::FISTS);
        let base = body
            .and_then(|b| b.drive.as_ref())
            .map_or(1.0, |d| d.race.clip_rate);
        if !pb.attack.requested {
            if state_now != Condition::GETTING_UP {
                let primary_blend = attacker
                    .primary_record(entity, blades.iter().map(|(b, _)| b))
                    .map(|r| r.blend);
                let at = motions
                    .as_deref()
                    .and_then(|m| SwingAt::of(now, &anim, m, base));
                let left = leave_swing(
                    &mut state,
                    &mut anim,
                    &swing,
                    at,
                    state_now,
                    primary_blend,
                    stance.action(),
                );
                if left {
                    if let Some(c) = condition.as_deref_mut() {
                        *c = Condition::NORMAL;
                    }
                    commands.entity(entity).remove::<Attacking>();
                }
            }
            continue;
        }
        pb.attack.thrust = attack::may_thrust(weapon_class)
            && !off
            && (bindings.pressed(&keys, Action::Alternate)
                || (pad_active && pad.as_deref().is_some_and(|p| p.frame().alternate)));
        req.heading = pb.aim;
        let (energy, most) = body
            .and_then(|b| b.drive.as_ref())
            .map_or((MAX_ENERGY, MAX_ENERGY), |d| (d.energy, d.max_energy()));
        let dazed = !attack::has_energy(energy, most);
        if dazed != pb.attack.dazed {
            pb.attack.dazed = dazed;
            if dazed {
                info!("attack waits: energy {energy:.2}, under 80 % of {most:.2}");
            }
        }
        if dazed {
            continue;
        }
        // The attack start asks a running cast's hook first: a hook that
        // keeps its cast refuses the attack.
        let (ask, go) = exa_combat::AttackHook::gate(hook);
        if ask {
            asked.write(exa_combat::AttackAskedHook(entity));
        }
        if !go {
            continue;
        }
        let cooldown = attacker.cooldown_ms(entity, blades.iter().map(|(b, _)| b));
        let busy = Busy {
            now_ms: now,
            last_tick_ms: attacker.last_tick_ms,
            cooldown_ms: cooldown,
            playing: attack::playing(attacking, &anim),
            released: state.has(swing::flags::RELEASED),
            stall_until_ms: state.end_ms,
            renew_ms: state.renew_ms,
            side_started_ms: attacker.side_started_ms,
            feint: fighter.is_some_and(Fighter::feint),
            strikers: attack::held_strikers(entity, blades.iter().map(|(b, _)| b)),
        };
        let Some(start) = attack::execute(pb.attack.request(), busy) else {
            continue;
        };
        // The starters aim at the foe's parry (a shield fighter's or one
        // with two weapons) and mark the foe's motion they counter.
        let aim = attacker.aim(fighter, pb.crouch > 0.0, false, now);
        let mut countered = None;
        let mut side_pick = |side: Side| {
            let (pick, c) = attacker.pick_aimed(side, &aim);
            countered = c;
            side.motion(pick)
        };
        let motion = match start.kind {
            AttackKind::Main => side_pick(Side::Main),
            AttackKind::Other => side_pick(Side::Other),
            AttackKind::Overhead => AttackMotion::Overhead,
            AttackKind::Stab => {
                // The held weapon's long axis, forward or not far back.
                let ahead = blades
                    .iter()
                    .find(|(b, _)| b.wielder == entity && b.slot == Slot::First)
                    .map(|(_, gt)| gt.up().dot(forward(body_turn.yaw)));
                if !ahead.is_some_and(attack::stab_faces) {
                    continue;
                }
                AttackMotion::Stab
            }
        };
        info!(
            "attack: {} in {} with {} at {now} ms (cursor {:.0}° left of the facing, \
             energy {energy:.2}; last attack tick {}, cooldown {cooldown} ms{})",
            motion.action(),
            anim.moveset,
            anim.config,
            turn_between(body_turn.yaw, pb.aim).to_degrees(),
            attacker
                .last_tick_ms
                .map_or("none".into(), |t| format!("at {t} ms")),
            if busy.released {
                format!(", after a release, stall until {} ms", state.end_ms)
            } else {
                String::new()
            },
        );
        pb.attack.other_side = false;
        // It ends in the stance the player holds by then.
        exa_combat::start_attack(
            &mut commands,
            entity,
            &mut anim,
            &mut attacker,
            &mut state,
            blades.iter().map(|(b, _)| b),
            AttackStart {
                motion,
                blend: start.blend,
                weapon_class,
                remise: fighter.is_some_and(Fighter::remise),
                now_ms: now,
                countered: if motion.is_side() {
                    countered
                } else {
                    exa_combat::attack::countered_by(motion, &aim)
                },
            },
            stance.action(),
        );
    }
}

/// The original's behaviour tick without an attack request, for a fighter
/// whose condition is `condition`, its clip where `at` says: an attack
/// playing (attacking) is released unless it recoiled, landed a cleaving
/// blow or was released already ([`SwingState::release`]); not attacking,
/// its state keeps only the released, recoiled and 0x40000 bits. A
/// main-side attack whose stall is over returns to `stance` with twice
/// its primary's stance blend (`primary_blend`, W19; 30 frames without a
/// primary), eased smoothly. Returns whether it did (the caller makes the
/// condition normal).
fn leave_swing(
    state: &mut SwingState,
    anim: &mut Animator,
    swing: &Swing,
    at: Option<SwingAt>,
    condition: Condition,
    primary_blend: Option<i32>,
    stance: &str,
) -> bool {
    if condition == Condition::ATTACKING {
        if !state.has(swing::flags::NO_RELEASE)
            && let Some(at) = at
        {
            state.release(swing, &at);
            info!(
                "attack released at {:.2} of its clip ({} ms): stall {:.3} a tick, stance at {} ms, \
                 renewal at {} ms",
                at.progress, at.now_ms, state.stall, state.end_ms, state.renew_ms
            );
        }
    } else {
        state.flags &= swing::flags::KEPT_IDLE;
    }
    let main_side = matches!(at.and_then(|a| a.motion), Some(AttackMotion::Main(_)));
    let now = at.map_or(0, |a| a.now_ms);
    if !main_side || state.end_ms >= now {
        return false;
    }
    let blend = primary_blend.map_or(RETURN_BLEND, |w| 2 * w.max(0) as u32);
    anim.start(stance, true, None, blend, Ease::Smooth);
    // The stance's start shuts the window, as every clip start.
    state.open = [false; 2];
    info!("released attack back to {stance} at {now} ms (blend {blend})");
    true
}

// ---- Camera ---------------------------------------------------------------

/// The camera's distance from the player (m): fixed, as the original's;
/// zoom narrows the field of view instead.
const CAMERA_DISTANCE: f32 = 20.0;
// The yaw spring's stiffness and retention per tick are the game camera
// preset's view (250/59²) and retention (the 59th root of 3.94e-13):
// [`ChaseCamera::controller`].
/// The yaw leads the shown yaw by at most this per tick (rad).
const MAX_YAW_LEAD: f32 = std::f32::consts::FRAC_PI_4;
/// Zoom stops: 800 + 240·level, in the original's units (2000 = 45°): the
/// game camera preset's smallest and largest distance, and its first one
/// the zoom starts from (`nav_camera::GAME_CAMERA_PRESET`).
const ZOOM_MIN: f32 = crate::nav_camera::GAME_CAMERA_PRESET.min_distance;
const ZOOM_MAX: f32 = crate::nav_camera::GAME_CAMERA_PRESET.max_distance;
const ZOOM_START: f32 = crate::nav_camera::GAME_CAMERA_PRESET.distance;
/// The steepest pitch the orbit drag reaches (rad below the horizon): the
/// preset's default upper bound.
const MAX_PITCH: f32 = 1.5;
/// A shake offset shows while its squared length is above the original's
/// 1e-5 cm² (here in m²).
const SHAKE_FLOOR: f32 = 1e-9;
/// Each tick the shake offset keeps this share of its last change, and then
/// this share of itself: a damped wobble of about a second.
const SHAKE_CARRY: f32 = 0.96;
const SHAKE_KEEP: f32 = 0.99;
/// The shake stops once its change in a tick squared is within the
/// original's 1e-7 cm² and the offset squared within 1e-4 cm² (here in m²).
const SHAKE_STILL_CHANGE: f32 = 1e-11;
const SHAKE_STILL: f32 = 1e-8;
/// The original's centimetre, in metres: the wall probe's sizes are in
/// its units.
const CM: f32 = 0.01;
/// The stepped camera's extent in the original's units: its eye distance
/// and the zoom that is 45°.
const CAMERA_EXTENT: f32 = 2000.0;

#[derive(Component, Debug, Clone)]
pub struct ChaseCamera {
    /// The orbit's target yaw, and the shown yaw: the heading of the
    /// backward vector as the last tick read it (rad).
    pub yaw: f32,
    shown_yaw: f32,
    /// The pitch asked for by the orbit drag, and the shown pitch (rad
    /// below the horizon).
    user_pitch: f32,
    pitch: f32,
    /// Zoom: the target (800–2000) and the stored shown value, which a
    /// frame moves to the zoom it draws once they lie 0.01 apart.
    zoom: f32,
    shown_zoom: f32,
    /// The zoom spring a frame draws between its last two values.
    zoom_spring: crate::nav_camera::ZoomSpring,
    /// The real time since the camera started (ms), rounded once to the
    /// frame stamp the tick driver compares against.
    elapsed_ms: f64,
    /// The original's integer tick clock.
    frame: crate::nav_camera::FrameDriver,
    /// The backward unit vector from the focus to the eye, and the one the
    /// last tick started from: each tick springs it toward the axis of the
    /// camera node turned to the pitch and yaw, and a frame shows it
    /// between the two.
    back: Vec3,
    back_before: Vec3,
    /// The eye's shake offset (m) and the tick before's: `kick` moves them
    /// apart, each tick carries the offset on, and a frame adds it between
    /// the two.
    shake: Vec3,
    shake_before: Vec3,
    /// The wall avoidance: the step count (0–4), whether the last probe
    /// tested the present step (a climbing probe) or the one below, the
    /// tilt the axis takes (rad, the controller's sign: negative raises
    /// the eye) and the tilt the tick before left.
    wall_steps: i32,
    wall_climb: bool,
    wall_tilt: f32,
    wall_before: f32,
    /// The camera controller the level's view builds from the game camera
    /// preset: the backward vector's stiffness (the preset's view) and
    /// retention, and the zoom spring's rate and step, are its numbers.
    controller: crate::nav_camera::NavController,
    /// The yaw was set by an alignment (Cursor Look's) and the shown yaw
    /// has not yet come within [`ALIGNED`] of it; an orbit drag ends it.
    pub(crate) aligning: bool,
    /// The looked-at point of a camera that follows nothing (the original's
    /// centimetres), which the free branch of each tick pans, lifts over
    /// the terrain and holds in height; `None` while the camera follows a
    /// body.
    free: Option<crate::nav_camera::FreeFocus>,
}

/// What the free branch of a tick reads: the movement keys, the camera's
/// drawn right and forward axes, and the terrain's height under a point
/// (both in the original's centimetres; `None` where it has no ground).
pub(crate) struct FreeInput<'a> {
    pub keys: crate::nav_camera::PanKeys,
    pub right: Vec3,
    pub forward: Vec3,
    pub ground: &'a dyn Fn(Vec3) -> Option<f32>,
}

/// An alignment is over once the shown yaw is this near its target (rad,
/// compared in double precision).
const ALIGNED: f64 = 0.01;

impl Default for ChaseCamera {
    fn default() -> Self {
        let controller =
            crate::nav_camera::NavController::new(crate::nav_camera::GAME_CAMERA_PRESET, 0, false);
        let preset = controller.preset;
        let mut cam = Self {
            yaw: 0.0,
            shown_yaw: 0.0,
            user_pitch: 0.0,
            pitch: min_pitch(ZOOM_START),
            zoom: ZOOM_START,
            shown_zoom: ZOOM_START,
            zoom_spring: crate::nav_camera::ZoomSpring {
                motion: ZOOM_START,
                previous: ZOOM_START,
                target: ZOOM_START,
                step: preset.zoom_step,
                rate: preset.zoom_speed,
                half_range: preset.min_distance,
            },
            elapsed_ms: 0.0,
            frame: crate::nav_camera::FrameDriver::default(),
            back: Vec3::Z,
            back_before: Vec3::Z,
            shake: Vec3::ZERO,
            shake_before: Vec3::ZERO,
            aligning: false,
            wall_steps: 0,
            wall_climb: false,
            wall_tilt: 0.0,
            wall_before: 0.0,
            controller,
            free: None,
        };
        cam.settle_back();
        cam
    }
}

/// The random stream of the camera's hard-impact kick, seeded once per
/// match like the other effect streams.
#[derive(Resource)]
pub(crate) struct CameraKickRng(pub crate::arena::Mt19937);

impl Default for CameraKickRng {
    fn default() -> Self {
        Self(crate::arena::Mt19937::new(5489))
    }
}

/// Kicks the chase camera when a hard overhead lands within 18 m: the
/// original's `(2000 − distance)·0.001` cm strength along +Y, with its
/// random offset. The distance is over 3 240 000 cm² (18 m) it is silent.
fn camera_hits(
    mut hits: MessageReader<exa_combat::impact::CameraHit>,
    mut rng: ResMut<CameraKickRng>,
    mut cameras: Query<(&GlobalTransform, &mut ChaseCamera)>,
) {
    let Ok((eye, mut camera)) = cameras.single_mut() else {
        return;
    };
    for hit in hits.read() {
        // The distance is measured in the original's centimetres.
        let distance2 = hit.at.distance_squared(eye.translation()) * 10_000.0;
        let Some(strength) = exa_physics::part::camera_shake(distance2) else {
            continue;
        };
        let mut draw = || rng.0.unit() as f32;
        camera.kick_random(Vec3::Y, Vec3::X, strength * 0.01, &mut draw);
    }
}

/// The player's feedback to a blow dealt on flesh, as the damage routine's
/// tail gives it: a struck player's camera jolts along the blow by the
/// square root of its strength (`7Y + 7L`, a big striker's by its speed)
/// when that passes 1, in cm, with the kick's random offset, and its pad
/// rumbles by the strength; a striking player's pad rumbles by half the
/// yellow it dealt. The kick's side axis is the world's x, as
/// [`camera_hits`] takes it.
fn feel_blows(
    mut blows: MessageReader<exa_combat::pain::BlowFelt>,
    mut clashes: MessageReader<exa_combat::pain::ClashFelt>,
    players: Query<(), With<PlayerBody>>,
    mut rng: ResMut<CameraKickRng>,
    mut cameras: Query<&mut ChaseCamera>,
    mut pad: Option<ResMut<crate::gamepad::PlayPad>>,
) {
    for blow in blows.read() {
        if players.contains(blow.target)
            && let Ok(mut camera) = cameras.single_mut()
        {
            let jolt = exa_combat::pain::struck_player_jolt(blow.wound, blow.yellow, &blow.striker);
            if let Some(size) = jolt.camera {
                let mut draw = || rng.0.unit() as f32;
                camera.kick_random(blow.direction, Vec3::X, size * 0.01, &mut draw);
            }
            if let Some(pad) = pad.as_mut() {
                crate::gamepad::rumble_hit(pad, jolt.rumble);
            }
            info!(
                "player struck: jolt {:.3} (camera {}), rumble {:.3}",
                jolt.rumble,
                jolt.camera.map_or("none".into(), |c| format!("{c:.3} cm")),
                jolt.rumble
            );
        } else if players.contains(blow.attacker) {
            let rumble = exa_combat::pain::striking_player_rumble(blow.yellow);
            if let Some(pad) = pad.as_mut() {
                crate::gamepad::rumble_hit(pad, rumble);
            }
            debug!("player's blow: rumble {rumble:.3}");
        }
    }
    // A swing's first clash or block rumbles the pad by half the blow's
    // volume when the player struck or was struck.
    for clash in clashes.read() {
        if players.contains(clash.attacker) || players.contains(clash.target) {
            let rumble = exa_combat::pain::clash_rumble(clash.volume);
            if let Some(pad) = pad.as_mut() {
                crate::gamepad::rumble_hit(pad, rumble);
            }
            debug!("player's clash: rumble {rumble:.3}");
        }
    }
}

impl ChaseCamera {
    /// A camera behind a character facing `yaw`.
    pub fn behind(yaw: f32) -> Self {
        let yaw = yaw + std::f32::consts::PI;
        let mut cam = Self {
            yaw,
            shown_yaw: yaw,
            ..default()
        };
        cam.settle_back();
        cam
    }

    /// The axis of the camera node turned to the present pitch with the
    /// wall tilt (held to 1.54 rad) and `yaw`: the backward vector the
    /// spring pulls toward.
    fn node_axis(&self, yaw: f32) -> Vec3 {
        let pitch = crate::nav_camera::tilted_pitch(-self.pitch, self.wall_tilt);
        crate::nav_camera::camera_axis(pitch, yaw)
    }

    /// Puts the backward vector on the node's axis at the target yaw, still.
    fn settle_back(&mut self) {
        self.back = self.node_axis(self.yaw);
        self.back_before = self.back;
    }

    /// The zoom the camera eases to (800–2000).
    pub fn zoom(&self) -> f32 {
        self.zoom
    }

    /// Places the camera for a shot from `eye` to `focus` (any units), as
    /// the original's placement routine does
    /// ([`crate::nav_camera::place`]): the target and shown yaw become the
    /// engine's planar yaw of the backward vector from the focus to the eye,
    /// and the orbit stops. A shot with a `distance` (a fast shot's 1300)
    /// starts the zoom again from it: the shown zoom takes the distance and
    /// the zoom raised to the smallest distance and, the camera being the
    /// stepped one, lowered to the largest. A shot without one keeps the
    /// zoom [guess: the original passes the controller's own last placed
    /// distance, which Exarust does not keep apart from the wheel's zoom].
    /// The backward vector takes the shot's and stops, and the wall
    /// avoidance starts over (no step, no tilt); the pitch stays the
    /// camera's own rules', toward which the next ticks spring the vector.
    /// A shot with no horizontal offset leaves the camera.
    pub fn face_shot(&mut self, eye: Vec3, focus: Vec3, distance: Option<f32>) {
        let back = eye - focus;
        if back.x == 0.0 && back.z == 0.0 || !back.is_finite() {
            return;
        }
        let preset = self.controller.preset;
        let limits = crate::nav_camera::HeightLimits {
            min: preset.min_height,
            max: preset.max_height,
        };
        let placed = crate::nav_camera::place(focus, eye, None, limits);
        self.yaw = placed.yaw;
        self.shown_yaw = placed.yaw;
        self.back = placed.back;
        self.back_before = placed.back;
        self.wall_steps = 0;
        self.wall_climb = false;
        self.wall_tilt = 0.0;
        self.wall_before = 0.0;
        if let Some(distance) = distance {
            self.zoom = crate::nav_camera::clamp_distance(
                distance,
                preset.min_distance,
                preset.max_distance,
            );
            // The shown zoom and both spring values take the distance as
            // it is.
            self.shown_zoom = distance;
            self.zoom_spring.motion = distance;
            self.zoom_spring.previous = distance;
        }
    }

    /// The unit vector from the focus to the eye the last tick left.
    fn backward(&self) -> Vec3 {
        self.back
    }

    /// Shakes the camera: the eye's offset moves by `by` (m) and the tick
    /// before's the other way, so the next ticks carry it on.
    pub fn kick(&mut self, by: Vec3) {
        self.shake += by;
        self.shake_before -= by;
    }

    /// A kick from the original's routine: the kick's direction (the
    /// camera's right axis, jittered, when it is too short) plus a small
    /// random offset, scaled to `size` (m). The draws come from the game's
    /// generator, in the original's order.
    pub fn kick_random(
        &mut self,
        direction: Vec3,
        right: Vec3,
        size: f32,
        draw: &mut impl FnMut() -> f32,
    ) {
        let offset = crate::nav_camera::kick_offset(direction, right, size, draw);
        self.kick(offset);
    }

    /// The wall probe that ends each tick of the stepped camera. It looks
    /// from 50 cm below `focus` along the camera's yaw (the planar yaw of
    /// the backward vector) and a pitch 0.05 rad higher than the camera's,
    /// less 0.15 rad per step: alternately at the present step with a
    /// 30 cm ball (a hit climbs, to four) and at the step below with a
    /// 20 cm ball (a miss comes down). `obstructed` answers the probe
    /// (start, unit direction, radius and length in metres; the length is
    /// twice the extent less 1.5 m, 38.5 m). The tilt then eases toward
    /// `−0.15` per step.
    fn probe_wall(
        &mut self,
        focus: Vec3,
        obstructed: &mut impl FnMut(Vec3, Vec3, f32, f32) -> bool,
    ) {
        use crate::nav_camera as nav;
        let probe = nav::wall_probe(-self.pitch, self.wall_steps, self.wall_climb);
        self.wall_climb = probe.climb;
        let direction = nav::camera_axis(probe.pitch, nav::planar_yaw(self.backward()));
        let start = focus - Vec3::Y * (nav::PROBE_DROP * CM);
        let length = nav::probe_length(CAMERA_EXTENT) * CM;
        let hit = obstructed(start, direction, probe.radius * CM, length);
        self.wall_steps = nav::wall_steps(self.wall_steps, probe.climb, hit);
        let target = nav::wall_tilt(self.wall_steps);
        // The ease scales by the controller's zoom-spring rate.
        let rate = self.controller.preset.zoom_speed;
        let tilt = nav::wall_tilt_ease(self.wall_tilt, self.wall_before, target, rate);
        self.wall_before = self.wall_tilt;
        self.wall_tilt = tilt;
    }

    /// The original's camera alignment: the target yaw becomes `yaw`,
    /// kept within half a turn of the shown yaw, and the camera is marked
    /// aligning.
    pub(crate) fn align(&mut self, yaw: f32) {
        self.yaw = align_behind(yaw, self.shown_yaw);
        self.aligning = true;
    }

    /// One 17 ms tick, in the controller's order: the zoom spring (or the
    /// shown zoom's snap to its target), the shake, the pitch ease, then
    /// the heading: the shown yaw becomes the backward vector's heading
    /// (the target turning a whole turn with it across ±π), an alignment
    /// ends within 0.01 rad, and the vector springs toward the camera node's
    /// axis at the target yaw, led at most π/4 ahead of the heading.
    fn tick(&mut self, free: Option<&FreeInput>) {
        use crate::nav_camera as nav;
        if nav::zoom_settles(self.shown_zoom, self.zoom) {
            self.shown_zoom = self.zoom;
        } else {
            self.zoom_spring.target = self.zoom;
            self.zoom_spring.step();
        }
        self.free_tick(free);
        let change = self.shake - self.shake_before;
        if change.length_squared() > SHAKE_STILL_CHANGE || self.shake.length_squared() > SHAKE_STILL
        {
            self.shake_before = self.shake;
            self.shake = (self.shake + change * SHAKE_CARRY) * SHAKE_KEEP;
        } else {
            self.shake = Vec3::ZERO;
            self.shake_before = Vec3::ZERO;
        }
        let target_pitch = pitch_held(self.user_pitch, pitch_floor(self.shown_zoom));
        self.pitch = nav::ease_pitch(self.pitch, target_pitch);
        let heading = nav::planar_yaw(self.back);
        self.yaw = nav::wrap_target_yaw(self.yaw, self.shown_yaw, heading);
        self.shown_yaw = heading;
        // The controller's update ends an alignment once the shown yaw is
        // within 0.01 rad of the target, before the spring's step.
        if self.aligning && f64::from((self.yaw - self.shown_yaw).abs()) < ALIGNED {
            self.aligning = false;
        }
        let yaw = nav::lead_yaw(self.yaw, heading, MAX_YAW_LEAD);
        let target = self.node_axis(yaw);
        let preset = &self.controller.preset;
        (self.back, self.back_before) = nav::spring_back(
            self.back,
            self.back_before,
            target,
            preset.view,
            preset.retention,
        );
    }

    /// The free branch of a tick, for a camera that follows nothing: the
    /// movement keys make the pan vector along the drawn right and forward
    /// axes (the preset's `0x800` reads them, its `0x20000` flattens and
    /// normalises the vector), and the looked-at point is lifted over the
    /// terrain, pulled along the pan by the zoom's pull and moved by the
    /// preset's rate within its height limits
    /// ([`crate::nav_camera::FreeFocus::tick`]).
    fn free_tick(&mut self, input: Option<&FreeInput>) {
        use crate::nav_camera as nav;
        let (Some(free), Some(input)) = (self.free.as_mut(), input) else {
            return;
        };
        let preset = &self.controller.preset;
        let keys = if preset.flags & 0x800 != 0 {
            input.keys
        } else {
            nav::PanKeys::default()
        };
        let pan = nav::pan_input(
            input.right,
            input.forward,
            keys,
            Vec3::ZERO,
            preset.flags & 0x2_0000 != 0,
        );
        let pull = nav::free_pull(self.shown_zoom, preset.max_distance, preset.extent);
        let limits = nav::HeightLimits {
            min: preset.min_height,
            max: preset.max_height,
        };
        let ground = (input.ground)(free.focus);
        free.tick(
            ground,
            pan,
            pull,
            preset.rate,
            preset.flags & 0x2000 != 0,
            limits,
        );
    }

    /// Frees the camera at `at` (m): it follows nothing and its looked-at
    /// point starts there, standing still. A camera already free keeps its
    /// point.
    pub(crate) fn free_at(&mut self, at: Vec3) {
        if self.free.is_none() {
            self.free = Some(crate::nav_camera::FreeFocus::at(at / CM));
        }
    }

    /// The camera follows a body again: its free point is dropped.
    pub(crate) fn follow(&mut self) {
        self.free = None;
    }

    /// Whether the camera follows nothing.
    pub(crate) fn is_free(&self) -> bool {
        self.free.is_some()
    }

    /// The looked-at point of a free camera a frame draws `t` of the way
    /// into the last tick (m).
    pub(crate) fn free_focus(&self, t: f32) -> Option<Vec3> {
        self.free.map(|f| f.shown(t) * CM)
    }

    /// Places a free camera for a shot from `eye` looking at `at` (m), as
    /// the original's placement routine does: the backward vector and yaw
    /// as [`Self::face_shot`] sets them, and the looked-at point at `at`
    /// with its height held within the preset's limits.
    pub(crate) fn place_free(&mut self, eye: Vec3, at: Vec3) {
        self.face_shot(eye, at, None);
        let preset = self.controller.preset;
        let limits = crate::nav_camera::HeightLimits {
            min: preset.min_height,
            max: preset.max_height,
        };
        self.free = Some(crate::nav_camera::FreeFocus::at(limits.apply(at / CM)));
    }

    /// Runs the ticks whose start the frame has reached, `dt` (s) after the
    /// last frame: at most nine, the rest of a backlog dropped. Returns how
    /// far the frame is into the last tick (0–1). No wall probe answers.
    #[cfg(test)]
    fn advance(&mut self, dt: f32) -> f32 {
        self.advance_probing(dt, Some(Vec3::ZERO), &mut |_, _, _, _| false, None)
    }

    /// [`Self::advance`] with each tick's wall probe asked of
    /// `obstructed` from `probe_from` (the followed body's hip point; a
    /// camera that follows nothing runs no probe, as the original's wall
    /// routine returns without a follow object), and the free branch fed
    /// by `free`.
    fn advance_probing(
        &mut self,
        dt: f32,
        probe_from: Option<Vec3>,
        obstructed: &mut impl FnMut(Vec3, Vec3, f32, f32) -> bool,
        free: Option<&FreeInput>,
    ) -> f32 {
        // The original compares the frame's millisecond stamp with the
        // tick clock; the elapsed real time is kept unrounded so the
        // rounding happens once, against the absolute clock.
        self.elapsed_ms += f64::from(dt) * 1000.0;
        // The stamp wraps at 2³² ms as the original's tick count does.
        let (ran, t) = self.frame.step(self.elapsed_ms.round() as u64 as u32);
        for _ in 0..ran {
            self.tick(free);
            if let Some(from) = probe_from {
                self.probe_wall(from, obstructed);
            }
        }
        t
    }

    /// The backward vector, zoom and eye offset to show `t` of the way from
    /// the tick before the last to the last.
    fn shown(&self, t: f32) -> (Vec3, f32, Vec3) {
        // The zoom is the spring's two values blended; the vectors' blend
        // is normalised (the zoom `interpolate_eye` gives is not used).
        let (backward, _) =
            crate::nav_camera::interpolate_eye((self.back_before, 0.0), (self.back, 0.0), t);
        let zoom = crate::nav_camera::frame_zoom(&self.zoom_spring, t);
        // The present offset decides whether there is a shake at all, as the
        // original tests it: one ending drops out at once.
        let shake = if self.shake.length_squared() > SHAKE_FLOOR {
            self.shake_before.lerp(self.shake, t)
        } else {
            Vec3::ZERO
        };
        (backward, zoom, shake)
    }
}

/// The shallowest pitch at a zoom: 0.8 rad, steeper when zoomed out.
fn min_pitch(zoom: f32) -> f32 {
    pitch_floor(zoom)
}

/// The field of view (rad) at a zoom, for the window's aspect.
fn zoom_fov(zoom: f32, aspect: f64) -> f32 {
    crate::nav_camera::fov_from_zoom(zoom, CAMERA_EXTENT, aspect)
}

/// Where the camera looks: the player's pelvis, drawn between its last two
/// body ticks as its mesh is (`alpha` of the way), or the player's feet
/// while it has no body.
fn camera_focus(feet: Vec3, body: Option<&Ragdoll>, alpha: f32) -> Vec3 {
    body.and_then(|b| b.parts.iter().find(|p| p.parent.is_none()))
        .map_or(feet, |pelvis| pelvis.drawn(alpha).translation.into())
}

/// The camera ticks on real time, so it keeps orbiting while the
/// simulation is paused or slowed (`docs/DEBUGGING.md`).
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub(crate) fn chase_camera(
    time: Res<Time<Real>>,
    fixed: Res<Time<Fixed>>,
    motion: Res<AccumulatedMouseMotion>,
    scroll: Res<AccumulatedMouseScroll>,
    pad: Option<Res<crate::gamepad::PlayPad>>,
    pad_status: Option<Res<crate::gamepad::PadStatus>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    players: Query<&Transform, (With<Player>, Without<ChaseCamera>)>,
    bodies: Query<&Ragdoll, With<PlayerBody>>,
    mut cameras: Query<(&mut ChaseCamera, &mut Transform, &mut Projection)>,
    (mut spectate, every_body, mut cut_away, look, obstacles, views, keys, terrain, mut logged): (
        Option<ResMut<crate::spectate::SpectateSession>>,
        Query<&Ragdoll>,
        Option<ResMut<crate::cut_away_live::CutAwayCamera>>,
        Res<CursorLook>,
        Query<(
            &exa_world::cut_away::WallObstacle,
            &GlobalTransform,
            Option<&exa_world::cut_away::CutAway>,
        )>,
        Query<&ViewVisibility>,
        Res<ButtonInput<KeyCode>>,
        Option<Res<exa_world::terrain::ground::TerrainGround>>,
        Local<Option<Vec3>>,
    ),
) {
    let player_focus = players.iter().next().map(|target| {
        camera_focus(
            target.translation,
            bodies.iter().next(),
            fixed.overstep_fraction(),
        )
    });
    // Spectating, the camera follows the body the session picked (its
    // hip, drawn as its mesh is), else it is free: its own tick pans and
    // lifts the point it looks at, from where it last looked
    // (`crate::spectate`). The arena's spectated start places the camera
    // once at the original's pose (the spawn spot raised by its offset,
    // looking at the spot) and frees it there.
    let (followed, free_from, pose) = match spectate.as_deref_mut().filter(|s| s.active) {
        Some(session) => {
            let hip =
                crate::spectate::followed_hip(session, &every_body, fixed.overstep_fraction());
            (
                hip,
                session.focus.or(player_focus).unwrap_or(Vec3::ZERO),
                session.pose.take(),
            )
        }
        None => match player_focus {
            Some(focus) => (Some(focus), focus, None),
            None => return,
        },
    };
    let mut looked = None;
    let (width, aspect) = windows.single().map_or((1600.0, 1.6), |w| {
        (
            w.width().max(1.0),
            f64::from(w.width()) / f64::from(w.height().max(1.0)),
        )
    });
    for (mut cam, mut t, mut projection) in &mut cameras {
        // The controls' capture drags while it is the orbit's, from the
        // frame after it began (`cursor_look`); a drag ends an alignment.
        if look.orbits() && !look.began {
            cam.aligning = false;
            let per_pixel = orbit_per_pixel(width);
            cam.yaw = orbit_yaw(cam.yaw, motion.delta.x, per_pixel);
            cam.user_pitch = orbit_pitch(
                cam.user_pitch,
                motion.delta.y,
                per_pixel,
                pitch_floor(cam.shown_zoom),
                MAX_PITCH,
            );
        } else if look.ended {
            // The capture's end (the orbit's or Cursor Look's) sends the
            // drag's close message: it snaps the target a quarter turn from
            // the shown yaw when it turned past an eighth (`nav_drag`).
            cam.yaw = crate::nav_drag::free_drag_release(cam.yaw, cam.shown_yaw);
        }
        if scroll.delta.y != 0.0 {
            // The wheel's handler reads the zoom as a stop level, subtracts
            // the notch in 120-unit steps and holds it below the sixth stop.
            // A line is one notch; a touchpad's pixels are read at 24 to the
            // notch (Exarust's guess, the map panel's line height) so a
            // pixel delta does not jump the zoom to an end stop.
            let notches = match scroll.unit {
                MouseScrollUnit::Line => scroll.delta.y,
                MouseScrollUnit::Pixel => scroll.delta.y / 24.0,
            };
            let units = (notches * crate::nav_drag::WHEEL_UNITS_PER_NOTCH).round() as i32;
            cam.zoom = crate::nav_drag::zoom_detents(cam.zoom, units, ZOOM_MIN, ZOOM_MAX);
        }
        if pad_status.as_ref().is_some_and(|s| s.active())
            && let Some(pad) = pad.as_deref()
        {
            let frame = pad.frame();
            if let Some(look) = frame.look {
                (cam.yaw, cam.user_pitch) = crate::gamepad::look_turn(
                    cam.yaw,
                    cam.user_pitch,
                    pitch_floor(cam.shown_zoom),
                    MAX_PITCH,
                    look,
                    orbit_per_pixel(width),
                );
            }
            for (held, steps) in [
                (frame.zoom_in, crate::gamepad::ZOOM_IN_STEPS),
                (frame.zoom_out, -crate::gamepad::ZOOM_IN_STEPS),
            ] {
                if held {
                    cam.zoom = crate::gamepad::zoom_step(cam.zoom, steps, 800.0, 2000.0);
                }
            }
        }
        let floor = pitch_floor(cam.shown_zoom);
        cam.user_pitch = clamp_pitch(cam.user_pitch, floor, MAX_PITCH);
        // The wall probe at the end of each tick asks the meshes the
        // original's scene walk asks: those marked as its obstacles (the
        // outdoor levels' walls, trees and buildings) that a view drew last
        // frame, in spawn order; the first that meets the probe answers. A
        // mesh the cut-away fade has marked (the node's mark bit the walk
        // reads) is skipped.
        let mut seen: Vec<_> = obstacles
            .iter()
            .filter(|(o, _, cut)| {
                !cut.is_some_and(|c| c.cut)
                    && o.parts.iter().any(|&p| views.get(p).is_ok_and(|v| v.get()))
            })
            .map(|(o, frame, _)| (o, frame))
            .collect();
        seen.sort_unstable_by_key(|(o, _)| o.parts.first().copied());
        if let Some((eye, at)) = pose {
            cam.place_free(eye, at);
        }
        let was_free = cam.is_free();
        match followed {
            Some(_) => cam.follow(),
            None => cam.free_at(free_from),
        }
        if cam.is_free() != was_free {
            match cam.free_focus(1.0) {
                Some(at) => info!("camera: free, looking at {at:.2}"),
                None => info!("camera: follows a body"),
            }
        }
        // The free branch's keys (A/D along the drawn right axis, S/W
        // along the forward one) and the terrain's height query, in the
        // original's centimetres.
        let ground = |at: Vec3| {
            terrain
                .as_deref()
                .and_then(|g| g.height(at.x * CM, at.z * CM))
                .map(|h| h / CM)
        };
        let free_input = FreeInput {
            keys: crate::nav_camera::PanKeys {
                left: keys.pressed(KeyCode::KeyA),
                right: keys.pressed(KeyCode::KeyD),
                back: keys.pressed(KeyCode::KeyS),
                forward: keys.pressed(KeyCode::KeyW),
            },
            right: *t.right(),
            forward: *t.forward(),
            ground: &ground,
        };
        let steps_before = cam.wall_steps;
        let along = cam.advance_probing(
            time.delta_secs(),
            followed,
            &mut |start, direction, radius, length| {
                seen.iter()
                    .any(|(o, frame)| o.meets(frame, start, direction, length, radius))
            },
            Some(&free_input),
        );
        // A followed camera looks at the hip; a free one at its own point
        // drawn into the last tick.
        let focus = cam.free_focus(along).or(followed).unwrap_or(free_from);
        looked = Some(focus);
        // A free camera's point is logged each time it has moved half a
        // metre from the last logged place.
        if cam.is_free() {
            if logged.is_none_or(|was| was.distance(focus) > 0.5 || !focus.is_finite()) {
                info!(
                    "camera: free point {focus:.2}, backward {:.2}",
                    cam.backward()
                );
                *logged = Some(focus);
            }
        } else {
            *logged = None;
        }
        if cam.wall_steps != steps_before {
            info!(
                "camera wall probe: {} step(s), tilt {:.3} rad ({} obstacle(s) drawn)",
                cam.wall_steps,
                cam.wall_tilt,
                seen.len()
            );
        }
        let (backward, zoom, shake) = cam.shown(along);
        // The stored shown zoom (the pitch floor's) follows the drawn one
        // once they lie 0.01 apart.
        cam.shown_zoom = crate::nav_camera::frame_zoom_kept(cam.shown_zoom, zoom);
        // The cut-away driver casts from the looked-at point back along the
        // camera's backward vector, by the camera's distance.
        if let Some(probe) = cut_away.as_deref_mut() {
            *probe = crate::cut_away_live::CutAwayCamera {
                focus,
                backward,
                distance: CAMERA_DISTANCE,
                ready: true,
            };
        }
        t.translation = focus + backward * CAMERA_DISTANCE;
        t.look_at(focus, Vec3::Y);
        t.translation += shake;
        if let Projection::Perspective(p) = projection.as_mut() {
            p.fov = zoom_fov(zoom, aspect);
        }
    }
    if let Some(session) = spectate.as_deref_mut().filter(|s| s.active)
        && looked.is_some()
    {
        session.focus = looked;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::system::RunSystemOnce;

    #[test]
    fn steering_with_w_and_a_walks_a_circle() {
        // 0.025 rad per tick at walking speed: a circle of v / ω.
        let walk = exa_physics::body::locomotion::CapsuleGaits::default().walk;
        let radius = (walk / TICK_HZ) / crate::steering::STEER;
        assert!((radius - 0.67).abs() < 0.05, "{radius}");
    }

    #[test]
    fn player_body_wears_underwear() {
        let mut world = World::new();
        let body = world.spawn(PlayerBody::new(0.0)).id();
        assert!(
            world
                .entity(body)
                .contains::<exa_character::WearsUnderwear>()
        );
    }

    #[test]
    fn the_player_keeps_a_stance() {
        let mut world = World::new();
        let body = world.spawn(PlayerBody::new(0.0)).id();
        assert_eq!(world.entity(body).get::<Stance>(), Some(&Stance::Idle));
    }

    /// The behaviour's auto-target is taken before the attack starters aim
    /// and written to the player's combat target: an opponent 2 m ahead is
    /// the target in the same step; out of combat mode there is none.
    #[test]
    fn the_target_is_picked_before_the_starters_aim() {
        let mut app = App::new();
        app.add_systems(Update, pick_player_target);
        let mut pb = PlayerBody::new(0.0);
        pb.set_combat(true);
        let player = app
            .world_mut()
            .spawn((pb, Fighter::default(), GlobalTransform::IDENTITY))
            .id();
        let foe = app
            .world_mut()
            .spawn((
                Fighter::default(),
                GlobalTransform::from_translation(Vec3::new(0.0, 0.0, 2.0)),
            ))
            .id();
        app.update();
        let target = |app: &App| {
            app.world()
                .get::<exa_combat::CombatTarget>(player)
                .map(|t| t.0)
        };
        assert_eq!(target(&app), Some(Some(foe)));
        app.world_mut()
            .get_mut::<PlayerBody>(player)
            .unwrap()
            .set_combat(false);
        app.update();
        assert_eq!(target(&app), Some(None));
    }

    #[test]
    fn the_player_holds_its_fists() {
        let mut world = World::new();
        let body = world.spawn(PlayerBody::new(0.0)).id();
        let held = world.entity(body).get::<HeldMoveset>().unwrap();
        assert_eq!(
            (held.config, held.weapon_class),
            (exa_anim::MovesetConfig::NONE, class::FISTS)
        );
    }

    /// An app running the player's stance rule, with the player at the
    /// origin facing +Z in combat mode and an opponent `distance` m ahead.
    fn stance_app(distance: f32) -> (App, Entity, Entity) {
        let mut app = App::new();
        app.init_resource::<Time>()
            .add_systems(Update, take_player_stance);
        let mut pb = PlayerBody::new(0.0);
        pb.set_combat(true);
        let player = app
            .world_mut()
            .spawn((
                pb,
                Animator::new("Default", "Idle"),
                Fighter::default(),
                GlobalTransform::IDENTITY,
            ))
            .id();
        let foe = app
            .world_mut()
            .spawn((
                Fighter::default(),
                GlobalTransform::from_translation(Vec3::new(0.0, 0.0, distance)),
            ))
            .id();
        (app, player, foe)
    }

    fn stance_of(app: &App, player: Entity) -> Stance {
        *app.world().get::<Stance>(player).unwrap()
    }

    fn place(app: &mut App, foe: Entity, at: Vec3) {
        *app.world_mut().get_mut::<GlobalTransform>(foe).unwrap() =
            GlobalTransform::from_translation(at);
    }

    /// In combat mode the player guards with its opponent 4 m ahead, takes
    /// the combat idle once it is within 3.5 m, and guards again (as a
    /// stance) when it leaves; behind it (beyond 120° of the cursor), dead
    /// or limp the opponent is no target. Out of combat mode: Idle.
    #[test]
    fn the_player_guards_until_its_target_comes_close() {
        let (mut app, player, foe) = stance_app(4.0);
        app.update();
        assert_eq!(stance_of(&app, player), Stance::Defence);
        place(&mut app, foe, Vec3::new(0.0, 0.0, 3.4));
        app.update();
        assert_eq!(stance_of(&app, player), Stance::CombatIdle);
        place(&mut app, foe, Vec3::new(0.0, 0.0, 3.6));
        app.update();
        assert_eq!(stance_of(&app, player), Stance::Defence);
        place(&mut app, foe, Vec3::new(0.0, 0.0, -2.0));
        app.update();
        assert_eq!(stance_of(&app, player), Stance::Defence);
        place(&mut app, foe, Vec3::new(0.0, 0.0, 2.0));
        *app.world_mut().get_mut::<Condition>(foe).unwrap() = Condition::DEAD;
        app.update();
        assert_eq!(stance_of(&app, player), Stance::Defence);
        // A limp body (a corpse the level placed) is no target either.
        *app.world_mut().get_mut::<Condition>(foe).unwrap() = Condition::NORMAL;
        app.update();
        assert_eq!(stance_of(&app, player), Stance::CombatIdle);
        app.world_mut().entity_mut(foe).insert(GoLimp::default());
        place(&mut app, foe, Vec3::new(0.0, 0.0, 4.0));
        app.update();
        place(&mut app, foe, Vec3::new(0.0, 0.0, 2.0));
        app.update();
        assert_eq!(stance_of(&app, player), Stance::Defence);
        app.world_mut()
            .get_mut::<PlayerBody>(player)
            .unwrap()
            .set_combat(false);
        app.update();
        assert_eq!(stance_of(&app, player), Stance::Idle);
    }

    /// The behaviour's auto-target stores the opponent the aim lock reads:
    /// the one ahead within 5 m, cleared out of combat mode and for one
    /// behind the cursor; a nearer one wins.
    #[test]
    fn the_aim_target_is_the_opponent_ahead() {
        let (mut app, player, foe) = stance_app(4.0);
        app.update();
        assert_eq!(
            app.world().get::<PlayerBody>(player).unwrap().aim_target(),
            Some(foe)
        );
        place(&mut app, foe, Vec3::new(0.0, 0.0, -2.0));
        app.update();
        assert_eq!(
            app.world().get::<PlayerBody>(player).unwrap().aim_target(),
            None
        );
        place(&mut app, foe, Vec3::new(0.0, 0.0, 2.0));
        let near = app
            .world_mut()
            .spawn((
                Fighter::default(),
                GlobalTransform::from_translation(Vec3::new(0.0, 0.0, 1.0)),
            ))
            .id();
        app.update();
        assert_eq!(
            app.world().get::<PlayerBody>(player).unwrap().aim_target(),
            Some(near)
        );
        app.world_mut()
            .get_mut::<PlayerBody>(player)
            .unwrap()
            .set_combat(false);
        app.update();
        assert_eq!(
            app.world().get::<PlayerBody>(player).unwrap().aim_target(),
            None
        );
    }

    /// While attacking, and for 3 s after its attack's last tick, the
    /// player stays in the combat idle with no target near.
    #[test]
    fn the_player_stays_ready_3_s_after_an_attack() {
        let (mut app, player, _) = stance_app(9.0);
        app.world_mut()
            .entity_mut(player)
            .insert(Attacking::default());
        app.update();
        assert_eq!(stance_of(&app, player), Stance::CombatIdle);
        app.world_mut().entity_mut(player).remove::<Attacking>();
        let time = |app: &mut App, s: f32| {
            app.world_mut()
                .resource_mut::<Time>()
                .advance_by(std::time::Duration::from_secs_f32(s));
            app.update();
        };
        time(&mut app, 2.9);
        assert_eq!(stance_of(&app, player), Stance::CombatIdle);
        time(&mut app, 0.2);
        assert_eq!(stance_of(&app, player), Stance::Defence);
    }

    /// The combat aim keeps a cursor point 50 cm or more out across the
    /// ground, drops its height, and keeps the last aim below 50 cm.
    #[test]
    fn the_combat_aim_needs_a_fifty_centimetre_cursor() {
        // +X 60 cm out is a quarter turn from +Z.
        let aim = aim_from(Vec3::ZERO, Vec3::new(0.6, 3.0, 0.0)).unwrap();
        assert!((aim - std::f32::consts::FRAC_PI_2).abs() < 1e-6);
        // Exactly 50 cm is kept; 49.9 cm is not; a point straight above the
        // player is not.
        assert!(aim_from(Vec3::ZERO, Vec3::new(0.0, 0.0, 0.5)).is_some());
        assert!(aim_from(Vec3::ZERO, Vec3::new(0.0, 0.0, 0.499_9)).is_none());
        assert!(aim_from(Vec3::ZERO, Vec3::new(0.0, 10.0, 0.0)).is_none());
        // A point behind is the half turn.
        let behind = aim_from(Vec3::ZERO, Vec3::new(0.0, 0.0, -1.0)).unwrap();
        assert!((behind.abs() - std::f32::consts::PI).abs() < 1e-6);
    }

    /// A footing on open ground for the footwork tests: the chest (and the
    /// feet centre) at `chest`, a human's 80 cm reach, scale 1, a gate that
    /// lets steps through.
    fn footing<'a>(
        chest: Vec3,
        line: LineWalk<'a>,
        nearest: &'a dyn Fn(Vec3) -> Vec3,
    ) -> Footwork<'a> {
        Footwork {
            chest,
            feet_centre: chest,
            ground: 0.0,
            reach_cm: STEP_LENGTH * 100.0,
            extension: 1.0,
            encumbrance: 0.0,
            scale: 1.0,
            scale_root: 1.0,
            gate: StepGate {
                condition: 0,
                energy: 1.0,
                swinging: false,
            },
            target: None,
            line,
            nearest,
        }
    }

    /// The line walk that ends where asked, and the grid snap that keeps
    /// the point.
    fn open(_: Vec3, to: Vec3) -> (Vec3, bool) {
        (to, false)
    }
    fn stay(at: Vec3) -> Vec3 {
        at
    }

    /// One footwork tick at `now` with `footing`, the body standing:
    /// whether a step started.
    fn step_with(
        input: &Keys,
        pb: &mut PlayerBody,
        anim: &mut Animator,
        footing: &Footwork,
        now: u64,
    ) -> bool {
        let mut steering = pb.steering;
        let started = combat_steps(
            input,
            Vec3::ZERO,
            &mut steering,
            pb,
            anim,
            Stance::Defence,
            false,
            footing,
            now,
        );
        pb.steering = steering;
        started
    }

    /// The footwork, one behaviour tick at `now`, on open ground with the
    /// chest at the origin.
    fn steps(input: &Keys, pb: &mut PlayerBody, anim: &mut Animator, now: u64) -> bool {
        let f = footing(Vec3::ZERO, &open, &stay);
        step_with(input, pb, anim, &f, now)
    }

    const W: Keys = Keys {
        w: true,
        a: false,
        s: false,
        d: false,
        shift: false,
        mouse: false,
    };

    fn guard() -> Animator {
        Animator::new("Default", "Defence stance")
    }

    /// A footwork step whose target the grid's line walk clamps within
    /// 20 cm is not taken; 21 cm away it is, and the step's target is
    /// recorded for the overlay. The stance clip plays throughout.
    #[test]
    fn footwork_stops_where_its_step_target_is_clamped() {
        let mut pb = PlayerBody::new(0.0);
        pb.set_combat(true);
        let mut anim = guard();
        let mut walk = |near: f32, now: u64, anim: &mut Animator| {
            let wall = move |from: Vec3, to: Vec3| (from + (to - from).normalize() * near, true);
            let f = footing(Vec3::ZERO, &wall, &stay);
            let started = step_with(&W, &mut pb, anim, &f, now);
            (started, pb.steering.line, pb.step)
        };
        let (started, asked, step) = walk(0.2, 1000, &mut anim);
        assert!(!started && !step.active);
        assert!(
            (step.target - Vec3::Z * 0.2).length() < 1e-6,
            "the target is written"
        );
        let asked = asked.unwrap();
        assert!(asked.blocked && (asked.to - Vec3::Z * 0.8).length() < 1e-6);
        let (started, _, step) = walk(0.21, 1012, &mut anim);
        assert!(started && step.active && !step.balance_step && step.started_ms == 1012);
        assert_eq!(anim.action, "Defence stance");
    }

    /// A word held over 200 ms becomes a tap, and a step younger than
    /// 300 ms is then re-laid at (1.5 − 0.3·encumbrance) of the reach,
    /// narrowed once from double: 120 cm from 80 cm, 97.2 cm from a reach
    /// of 72 cm at encumbrance 0.5; the step becomes the long kind. A crouch
    /// of 0.1 (above the double 0.1) is no tap; the float just under it is.
    #[test]
    fn a_tapped_word_relays_a_young_step_long() {
        let long = |reach: f32, encumbrance: f32, crouch: f32| {
            let mut pb = PlayerBody::new(0.0).crouching(crouch);
            pb.set_combat(true);
            let mut anim = guard();
            let mut f = footing(Vec3::ZERO, &open, &stay);
            f.reach_cm = reach;
            f.encumbrance = encumbrance;
            for now in [1000, 1200, 1201] {
                step_with(&W, &mut pb, &mut anim, &f, now);
                if now == 1000 {
                    assert_eq!(pb.step.target.z, reach / 100.0);
                    assert!(!pb.step.long);
                }
            }
            pb.step.target.z
        };
        assert_eq!(long(80.0, 0.0, 0.0).to_bits(), 0x3f99_999a);
        assert_eq!(long(72.0, 0.5, 0.0).to_bits(), 0x3f78_d4fd);
        assert_eq!(long(80.0, 0.0, 0.1), 0.8);
        let under = f32::from_bits(0.1f32.to_bits() - 1);
        assert_eq!(long(80.0, 0.0, under).to_bits(), 0x3f99_999a);
    }

    /// A step 300 ms old queues the held raw word (its full step vector)
    /// for the body. Once the body took it (the queue its countdown) the
    /// next tick clamps the taken target by the line walk from the chest
    /// and the grid's snap, the word takes the queued word back and the
    /// queue clears; the old step then queues the held word again.
    #[test]
    fn an_old_step_queues_the_next_and_takes_it_back() {
        let mut pb = PlayerBody::new(0.0);
        pb.set_combat(true);
        let mut anim = guard();
        assert!(steps(&W, &mut pb, &mut anim, 1000));
        steps(&W, &mut pb, &mut anim, 1299);
        assert_eq!(pb.step.queue, 0);
        steps(&W, &mut pb, &mut anim, 1300);
        assert_eq!(pb.step.queue, 1);
        assert_eq!(pb.move_word.queued_word, crate::steering::MOVE_FORWARD);
        assert!((pb.step.queued - Vec3::Z * 0.8).length() < 1e-6);
        assert!(pb.move_word.tap);
        // The body took it at a landing (its rule: the chest plus the
        // vector, the countdown −1).
        pb.step.queue = -1;
        pb.step.target = Vec3::new(0.0, 0.3, 1.6);
        pb.step.started_ms = 1390;
        let snap = |at: Vec3| at + Vec3::X * 0.05;
        let wall = |_: Vec3, to: Vec3| (to * 0.5, true);
        let f = footing(Vec3::Z * 0.8, &wall, &snap);
        step_with(&W, &mut pb, &mut anim, &f, 1400);
        assert!(
            (pb.step.target - Vec3::new(0.05, 0.15, 0.8)).length() < 1e-6,
            "{}",
            pb.step.target
        );
        assert!(!pb.move_word.tap);
        assert_eq!(pb.move_word.held, crate::steering::MOVE_FORWARD);
        assert_eq!(
            pb.step.queue, 0,
            "a young step does not queue the same word"
        );
        step_with(&W, &mut pb, &mut anim, &f, 1700);
        assert_eq!(pb.step.queue, 1);
    }

    /// No step is laid while walking (condition 1), under 0.7 energy (or a
    /// NaN energy) or during a swing.
    #[test]
    fn the_gate_holds_the_footwork() {
        for gate in [
            StepGate {
                condition: 1,
                energy: 1.0,
                swinging: false,
            },
            StepGate {
                condition: 0,
                energy: 0.69,
                swinging: false,
            },
            StepGate {
                condition: 0,
                energy: f32::NAN,
                swinging: false,
            },
            StepGate {
                condition: 0,
                energy: 1.0,
                swinging: true,
            },
        ] {
            let mut pb = PlayerBody::new(0.0);
            let mut anim = guard();
            let mut f = footing(Vec3::ZERO, &open, &stay);
            f.gate = gate;
            assert!(!step_with(&W, &mut pb, &mut anim, &f, 1000), "{gate:?}");
            assert!(!pb.step.active, "{gate:?}");
        }
    }

    /// A taken goal is moved onto the grid's nearest walkable point across
    /// the ground, keeping its height; an untaken one is not.
    #[test]
    fn a_taken_goal_snaps_onto_the_grid() {
        let mut pb = PlayerBody::new(0.0);
        let mut anim = guard();
        let shift = |at: Vec3| at + Vec3::new(0.1, -5.0, 0.0);
        let f = footing(Vec3::ZERO, &open, &shift);
        step_with(&W, &mut pb, &mut anim, &f, 1000);
        assert!((pb.step.target - Vec3::new(0.1, 0.0, 0.8)).length() < 1e-6);
        let mut pb = PlayerBody::new(0.0);
        let short = |_: Vec3, _: Vec3| (Vec3::Z * 0.1, true);
        let f = footing(Vec3::ZERO, &short, &shift);
        step_with(&W, &mut pb, &mut anim, &f, 1000);
        assert_eq!(pb.step.target, Vec3::Z * 0.1);
    }

    /// The legs' extension and the √scale scale the stride after the long
    /// step is narrowed: full extension keeps 80 cm and 120 cm, half
    /// extension halves both, and √4 doubles a 160 cm reach's half.
    #[test]
    fn the_legs_extension_scales_the_footwork_stride() {
        use crate::steering::goal_stride;
        assert_eq!(goal_stride(80.0, 1.0, 1.0, 0.0, false), 80.0);
        assert_eq!(goal_stride(80.0, 1.0, 1.0, 0.0, true), 120.0);
        assert_eq!(goal_stride(80.0, 0.5, 1.0, 0.0, false), 40.0);
        assert_eq!(goal_stride(80.0, 0.5, 1.0, 0.0, true), 60.0);
        assert_eq!(goal_stride(160.0, 0.5, 2.0, 0.0, false), 160.0);
        assert!(goal_stride(80.0, f32::NAN, 1.0, 0.0, false).is_nan());
    }

    /// W and A together, facing +Z, aim 0.8 as far sideways as forward. A
    /// side key alone still aims a full step: the stride is divided by the
    /// direction's own length.
    #[test]
    fn a_diagonal_step_weighs_the_side_by_0_8() {
        let aim = |w: bool, a: bool, d: bool| {
            let mut pb = PlayerBody::new(0.0);
            let mut anim = guard();
            let keys = Keys {
                w,
                a,
                d,
                ..Keys::default()
            };
            steps(&keys, &mut pb, &mut anim, 1000);
            pb.steering.line.unwrap().to
        };
        let diagonal = aim(true, true, false);
        assert!((diagonal.x / diagonal.z - 0.8).abs() < 1e-6);
        let side = aim(false, false, true);
        assert!((side - Vec3::new(-STEP_LENGTH, 0.0, 0.0)).length() < 1e-6);
    }

    /// A clamp 15 cm ahead and 15 cm up is 450 cm² and starts a step. The
    /// same 15 cm on the level is 225 cm² and starts none.
    #[test]
    fn footwork_takes_a_raised_clamp_inside_20_cm() {
        for (end, taken) in [
            (Vec3::new(0.0, 0.15, 0.15), true),
            (Vec3::new(0.0, 0.0, 0.15), false),
        ] {
            let mut pb = PlayerBody::new(0.0);
            let mut anim = guard();
            let line = move |_: Vec3, _: Vec3| (end, true);
            let f = footing(Vec3::ZERO, &line, &stay);
            step_with(&W, &mut pb, &mut anim, &f, 1000);
            assert_eq!(pb.step.active, taken, "{end}");
        }
    }

    /// A balance step whose target is within 20 cm of the feet is not under
    /// way: the held word starts a footwork step over it.
    #[test]
    fn a_settled_balance_step_gives_way_to_the_footwork() {
        let mut pb = PlayerBody::new(0.0);
        pb.set_combat(true);
        let mut anim = guard();
        pb.step = crate::footwork::StepRecord {
            active: true,
            balance_step: true,
            started_ms: 900,
            target: Vec3::X * 0.1,
            ..Default::default()
        };
        assert!(steps(&W, &mut pb, &mut anim, 1000));
        assert!(!pb.step.balance_step);
        pb.step.balance_step = true;
        pb.step.target = Vec3::X * 0.3;
        assert!(
            !steps(&W, &mut pb, &mut anim, 1012),
            "30 cm off is under way"
        );
    }

    /// Tab pressed while walking: the exploration walk (or run, or
    /// backpedal) ends in the stance once the body stands, instead of
    /// walking on the spot.
    #[test]
    fn an_exploration_walk_ends_in_the_stance() {
        let mut pb = PlayerBody::new(0.0);
        pb.set_combat(true);
        let f = footing(Vec3::ZERO, &open, &stay);
        for gait in ["Walk", "Run", "Backpedal"] {
            let mut anim = Animator::new("Default", gait);
            // Still moving (stopping softly) it walks on.
            let mut steering = pb.steering;
            combat_steps(
                &Keys::default(),
                Vec3::ZERO,
                &mut steering,
                &mut pb,
                &mut anim,
                Stance::Defence,
                true,
                &f,
                0,
            );
            assert_eq!(anim.action, gait);
            steps(&Keys::default(), &mut pb, &mut anim, 0);
            assert_eq!(anim.action, "Defence stance", "{gait}");
            let (blend, ease) = if gait == "Backpedal" {
                (0.0, Ease::Linear)
            } else {
                (20.0, Ease::Fast)
            };
            assert_eq!((anim.blend, anim.ease), (blend, ease), "{gait}");
        }
    }

    /// When the body leaves the walking condition the original starts the
    /// stored stance with a 20-frame blend eased mode 2 (the fast ease),
    /// not linearly. This end of the walk is the exploration clip's.
    #[test]
    fn an_ended_walk_keeps_the_originals_fast_ease() {
        for gait in ["Walk", "Run"] {
            let mut anim = Animator::new("Default", gait);
            explore_clip(&mut anim, None);
            assert_eq!(anim.action, "Idle", "{gait}");
            assert_eq!((anim.blend, anim.ease), (20.0, Ease::Fast), "{gait}");
        }
    }

    /// A player carrying the torch ("Torch") in combat mode, playing
    /// `action` with `stance`, under its root, with the steering, the
    /// stance rule, the body tick's stance part and its movement decision
    /// (as the ticked character, at body tick 1000); [`behaviour_tick`]
    /// runs them as the fixed step does.
    fn steered_app(action: &str, stance: Stance) -> (App, Entity) {
        let mut app = App::new();
        app.init_resource::<Time>()
            .insert_resource(BodyTicks(1000))
            .init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<ButtonInput<MouseButton>>()
            .init_resource::<Bindings>()
            .init_resource::<CursorGround>()
            .init_resource::<AimCursor>()
            .init_resource::<CameraKickRng>()
            .init_resource::<crate::player_controls::PlaySession>()
            .init_resource::<Physics>()
            .add_plugins(exa_combat::stance::StancePlugin)
            .add_systems(FixedPreUpdate, (steer_player, take_player_stance).chain())
            .add_systems(FixedUpdate, crate::stages::movement::decide_movement);
        let mut pb = PlayerBody::new(0.0);
        pb.set_combat(true);
        let body = app
            .world_mut()
            .spawn((
                pb,
                Transform::default(),
                GlobalTransform::IDENTITY,
                Animator::new("Torch", action),
                stance,
                Fighter::default(),
            ))
            .id();
        let root = app
            .world_mut()
            .spawn((
                Player,
                CharacterBody::default(),
                Transform::default(),
                GlobalTransform::IDENTITY,
            ))
            .add_child(body)
            .id();
        app.insert_resource(exa_physics::body::TickedCharacter {
            body,
            character: Some(root),
        });
        (app, body)
    }

    /// The behaviour runs the head tracker only while the body plays the
    /// stab. A look-at well off the heading moves the look by one step;
    /// any other action leaves the look where it is.
    #[test]
    fn the_head_tracks_only_while_the_player_stabs() {
        use bevy::math::Affine3A;
        use exa_combat::Attacking;

        let (mut app, body) = steered_app("Stab", Stance::CombatIdle);
        let mut ragdoll = Ragdoll::new(&[]).with_drive();
        ragdoll
            .drive
            .as_mut()
            .unwrap()
            .set_frame(Affine3A::IDENTITY);
        {
            let mut entity = app.world_mut().entity_mut(body);
            entity.insert((
                ragdoll,
                Attacking::new(AttackMotion::Stab, class::ONE_HANDED),
            ));
            let mut anim = entity.get_mut::<Animator>().unwrap();
            anim.looping = false;
        }
        // Two metres to the body's right, far enough that the tracker
        // takes the direction rather than the previous one.
        app.insert_resource(CursorGround(Some(Vec3::new(2.0, 1.0, 0.0))));
        behaviour_tick(&mut app);
        let tracked = app.world().get::<Locomotion>(body).unwrap().look;
        assert!(
            tracked > 0.02 && tracked < 0.05,
            "a stab moves the look one step toward the point, got {tracked}"
        );
        {
            let mut entity = app.world_mut().entity_mut(body);
            entity.remove::<Attacking>();
            entity.get_mut::<Animator>().unwrap().action = "Attack Mid".into();
        }
        behaviour_tick(&mut app);
        let kept = app.world().get::<Locomotion>(body).unwrap().look;
        assert_eq!(kept, tracked, "another action does not run the tracker");
    }

    /// A held knock-out wake (0x40) stays until the player gives input: a
    /// movement key or a mouse button clears that bit alone.
    #[test]
    fn input_releases_a_held_knock_out_wake() {
        let mut world = World::new();
        world.init_resource::<ButtonInput<KeyCode>>();
        world.init_resource::<ButtonInput<MouseButton>>();
        world.init_resource::<Bindings>();
        let body = world
            .spawn((PlayerBody::new(0.0), exa_combat::StateFlags(0x70)))
            .id();
        world.spawn(Player).add_child(body);
        world.run_system_once(release_held_wake).unwrap();
        assert_eq!(world.get::<exa_combat::StateFlags>(body).unwrap().0, 0x70);
        world
            .resource_mut::<ButtonInput<MouseButton>>()
            .press(MouseButton::Left);
        world.run_system_once(release_held_wake).unwrap();
        assert_eq!(world.get::<exa_combat::StateFlags>(body).unwrap().0, 0x30);
    }

    fn behaviour_tick(app: &mut App) {
        app.world_mut().run_schedule(FixedPreUpdate);
        app.world_mut().run_schedule(FixedUpdate);
    }

    fn clip_and_stance(app: &App, body: Entity) -> (String, Stance) {
        (
            app.world().get::<Animator>(body).unwrap().action.clone(),
            *app.world().get::<Stance>(body).unwrap(),
        )
    }

    fn set_key(app: &mut App, key: KeyCode, down: bool) {
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        if down {
            keys.press(key);
        } else {
            keys.release(key);
        }
    }

    /// The original's exploration steering starts no clip but the walk:
    /// a player that turns combat mode off in Cmbt - Idle with the Defence
    /// stance (3 s after its attack, or its target gone) keeps the combat
    /// idle, with the torch the compact carry before the chest, standing
    /// and turning in place, until it walks; the walk then ends in Idle.
    #[test]
    fn the_combat_idle_stays_after_combat_mode_until_the_player_walks() {
        let (mut app, body) = steered_app("Cmbt - Idle", Stance::Defence);
        behaviour_tick(&mut app);
        assert_eq!(
            clip_and_stance(&app, body),
            ("Cmbt - Idle".into(), Stance::Defence)
        );
        app.world_mut()
            .get_mut::<PlayerBody>(body)
            .unwrap()
            .set_combat(false);
        for _ in 0..200 {
            behaviour_tick(&mut app);
        }
        assert_eq!(
            clip_and_stance(&app, body),
            ("Cmbt - Idle".into(), Stance::Idle)
        );
        // A quarter turn in place starts no clip.
        set_key(&mut app, KeyCode::KeyD, true);
        behaviour_tick(&mut app);
        set_key(&mut app, KeyCode::KeyD, false);
        for _ in 0..90 {
            behaviour_tick(&mut app);
        }
        let req = app.world().get::<Locomotion>(body).unwrap();
        assert!((req.heading + std::f32::consts::FRAC_PI_2).abs() < 1e-5);
        assert_eq!(clip_and_stance(&app, body).0, "Cmbt - Idle");
        // W walks in the body tick that decides it moves: its locomotion
        // starts the walk; released, it stops softly (exploration reads the
        // keys as they are: the 200 ms word persistence is the combat
        // footwork's), its capsule braking from the speed the tap reached
        // (49 ticks to Idle here, where no controller moves it), and the
        // walk ends in Idle once the body tick has stopped it.
        set_key(&mut app, KeyCode::KeyW, true);
        behaviour_tick(&mut app);
        assert_eq!(clip_and_stance(&app, body).0, "Walk");
        behaviour_tick(&mut app);
        assert_eq!(clip_and_stance(&app, body).0, "Walk");
        set_key(&mut app, KeyCode::KeyW, false);
        // The clock advances a tick at a time, as the game's does: the
        // capsule brakes from the speed it reached; the walk ends in Idle
        // 49 ticks after the release.
        let mut ticks = 0;
        while clip_and_stance(&app, body).0 == "Walk" && ticks < 220 {
            app.world_mut().resource_mut::<BodyTicks>().0 += 1;
            behaviour_tick(&mut app);
            ticks += 1;
        }
        assert_eq!(ticks, 49);
        assert_eq!(clip_and_stance(&app, body), ("Idle".into(), Stance::Idle));
    }

    /// Combat mode turned off while the stance's own clip plays: the helper
    /// starts Idle at once, from the guard and from the combat idle held
    /// as the stance (within 3 s of an attack, or with a target near).
    #[test]
    fn the_stance_clip_turns_to_idle_with_combat_mode() {
        for (action, stance) in [
            ("Cmbt - Idle", Stance::CombatIdle),
            ("Defence stance", Stance::Defence),
        ] {
            let (mut app, body) = steered_app(action, stance);
            app.world_mut()
                .get_mut::<PlayerBody>(body)
                .unwrap()
                .set_combat(false);
            behaviour_tick(&mut app);
            assert_eq!(
                clip_and_stance(&app, body),
                ("Idle".into(), Stance::Idle),
                "{action}"
            );
        }
    }

    /// Standing in exploration, a gait ends in Idle; any other looping clip is the stance's to change, and a clip
    /// played once is left to end; moving starts the gait from any loop.
    #[test]
    fn the_exploration_steering_only_ends_walks() {
        for gait in ["Walk", "Run", "Backpedal"] {
            let mut anim = Animator::new("Torch", gait);
            explore_clip(&mut anim, None);
            assert_eq!(anim.action, "Idle", "{gait}");
            let (blend, ease) = if gait == "Walk" || gait == "Run" {
                (20.0, Ease::Fast)
            } else {
                (0.0, Ease::Linear)
            };
            assert_eq!((anim.blend, anim.ease), (blend, ease), "{gait}");
        }
        for kept in ["Cmbt - Idle", "Defence stance", "Idle"] {
            let mut anim = Animator::new("Torch", kept);
            explore_clip(&mut anim, None);
            assert_eq!(anim.action, kept);
            explore_clip(&mut anim, Some("Run"));
            assert_eq!(anim.action, "Run", "{kept}");
        }
        let mut anim = Animator::new("Torch", "Cmbt - Idle");
        anim.play_once("Attack Mid", "Cmbt - Idle");
        explore_clip(&mut anim, Some("Walk"));
        explore_clip(&mut anim, None);
        assert_eq!(anim.action, "Attack Mid");
    }

    #[test]
    fn the_cursor_left_of_the_facing_attacks_on_the_other_side() {
        let deg = |d: f32| d.to_radians();
        // Positive turns go to the body's left (+X from +Z; its right is −X).
        assert!(turn_between(0.0, deg(25.0)) > 0.0);
        assert_eq!(forward(deg(90.0)).x.signum(), 1.0);
        for yaw in [0.0, 3.0, -3.0] {
            assert!(!other_side(yaw + deg(19.5), yaw, class::FISTS));
            assert!(other_side(yaw + deg(20.5), yaw, class::FISTS));
            assert!(other_side(yaw + deg(20.5), yaw, class::ONE_HANDED));
            // Only the left: far to the right stays on the main side.
            assert!(!other_side(yaw - deg(90.0), yaw, class::FISTS));
            // Nothing held at all: from 15°.
            assert!(!other_side(yaw + deg(14.5), yaw, class::NONE));
            assert!(other_side(yaw + deg(15.5), yaw, class::NONE));
        }
    }

    /// A press requests an attack, held it stands, released it is gone; a
    /// second press within 300 ms (of the game clock) of the one before
    /// asks for the overhead while it is held, and uses both presses up.
    #[test]
    fn a_second_press_within_300_ms_asks_for_an_overhead() {
        let mut input = AttackInput::default();
        let press = |input: &mut AttackInput, at: u64, other: bool| {
            input.tick(true, at, other);
            let request = input.request();
            input.tick(false, at + 12, false);
            (input.requested, request)
        };
        input.tick(true, 10_000, false);
        assert!(input.requested && input.request() == Request::default());
        input.tick(true, 10_500, true);
        // Held: still the same request, its side taken at the press.
        assert!(input.requested && !input.request().other_side);
        input.tick(false, 10_512, false);
        assert!(!input.requested);
        let overhead = |r: Request| r.overhead;
        // 299 ms after the press at 10 000.
        assert!(overhead(press(&mut input, 10_299, false).1));
        // The double click used both presses up.
        let (_, r) = press(&mut input, 10_400, true);
        assert!(!r.overhead && r.other_side);
        assert!(!overhead(press(&mut input, 10_750, false).1));
        assert!(overhead(press(&mut input, 11_049, false).1));
        assert!(!input.requested);
    }

    /// Entering combat mode drops the double-click clock, so a press 100 ms
    /// later is not an overhead. A mode already on keeps the clock.
    #[test]
    fn entering_combat_drops_the_double_click_clock() {
        let mut body = PlayerBody::new(0.0);
        body.attack.tick(true, 1_000, false);
        body.attack.tick(false, 1_012, false);
        assert!(body.set_combat(true));
        body.attack.tick(true, 1_100, false);
        assert!(!body.attack.request().overhead);
        body.attack.tick(false, 1_112, false);
        body.attack.tick(true, 2_000, false);
        body.attack.tick(false, 2_012, false);
        assert!(!body.set_combat(true));
        body.attack.tick(true, 2_200, false);
        assert!(body.attack.request().overhead);
    }

    /// A synthetic motion set: the "Default" moveset's attacks and combat
    /// idle, 100 frames each.
    fn attack_clips() -> Motions {
        use exa_formats::rsq::{Action, Clip, Key, MotionSet, Moveset};
        let key = |time| Key {
            time,
            root: [0.0; 3],
            rotations: Vec::new(),
            flags: Vec::new(),
        };
        let clip = Clip {
            mode: 0,
            keys: vec![key(0.0), key(100.0)],
        };
        let actions = [
            "Attack Mid",
            "Attack Down",
            "Attack Up",
            "Attack Combo 1",
            "Attack Combo 2",
            "Attack Combo 3",
            "Overhead Swing",
            "Cmbt - Idle",
        ];
        let set = MotionSet {
            bone_count: 0,
            actions: actions
                .iter()
                .map(|n| exa_formats::rsq::Action {
                    name: (*n).into(),
                    params: [0; 2],
                })
                .collect::<Vec<Action>>(),
            movesets: vec![Moveset {
                name: "Default".into(),
                parent: String::new(),
                clips: vec![Some(clip); actions.len()],
            }],
        };
        Motions::new(std::sync::Arc::new(set), std::sync::Arc::default(), 0.0)
    }

    /// The unarmed player in combat mode, its two fists its strikers, at
    /// body tick 1000 (12 000 ms of the game clock); [`request_tick`] runs
    /// one behaviour tick of the attack request.
    fn attack_app() -> (App, Entity) {
        let mut app = App::new();
        app.insert_resource(BodyTicks(1000))
            .insert_resource(attack_clips())
            .init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<ButtonInput<MouseButton>>()
            .init_resource::<Bindings>()
            .init_resource::<AttackButton>()
            .add_message::<exa_combat::AttackAskedHook>()
            .add_systems(FixedPreUpdate, request_attacks);
        let mut pb = PlayerBody::new(0.0);
        pb.set_combat(true);
        let body = app
            .world_mut()
            .spawn((
                pb,
                Animator::new("Default", "Cmbt - Idle"),
                Stance::CombatIdle,
                Fighter::default(),
            ))
            .id();
        use exa_formats::weapon_record::{Holder, Lengths, Section, WeaponRecord};
        let fist = WeaponRecord::build(
            &Section::FISTS,
            false,
            Lengths::hand(-8.0, 9.66),
            &Holder::AVERAGE,
        );
        for slot in [Slot::First, Slot::Second] {
            app.world_mut().spawn((
                Blade::new(body, 0.1766, fist).in_slot(slot),
                GlobalTransform::IDENTITY,
            ));
        }
        (app, body)
    }

    /// The pad's emulated left button requests the same attack the left
    /// button does.
    #[test]
    fn the_pad_trigger_requests_an_attack() {
        use crate::gamepad::{PadSample, PadStatus, PlayContext, PlayPad};
        let (mut app, body) = attack_app();
        let mut sample = PadSample::default();
        sample.connected = true;
        sample.mouse_left = true;
        let ctx = PlayContext::default();
        let mut pad = PlayPad::default();
        pad.update(0, &sample, ctx);
        pad.update(3, &sample, ctx);
        pad.update(4, &sample, ctx);
        assert!(pad.attack_held());
        app.insert_resource(pad).insert_resource(PadStatus {
            connected: true,
            controller: true,
            ..PadStatus::default()
        });
        app.world_mut().run_schedule(FixedPreUpdate);
        let anim = app.world().get::<Animator>(body).unwrap();
        assert!(!anim.looping, "{}", anim.action);
        assert!(anim.action.starts_with("Attack"), "{}", anim.action);
    }

    /// One behaviour tick with the left button down or up, the game clock
    /// one tick on; the clip it plays after.
    fn request_tick(app: &mut App, body: Entity, down: bool) -> (String, bool) {
        {
            let mut buttons = app.world_mut().resource_mut::<ButtonInput<MouseButton>>();
            if down {
                buttons.press(MouseButton::Left);
            } else {
                buttons.release(MouseButton::Left);
            }
        }
        app.world_mut().run_schedule(FixedPreUpdate);
        app.world_mut().resource_mut::<BodyTicks>().0 += 1;
        let anim = app.world().get::<Animator>(body).unwrap();
        (anim.action.clone(), anim.looping)
    }

    /// Ends the attack playing in its stance at the game clock's `ms`, as
    /// its last tick.
    fn end_attack(app: &mut App, body: Entity, ms: u64) {
        let mut e = app.world_mut().entity_mut(body);
        e.get_mut::<Animator>().unwrap().play_loop("Cmbt - Idle");
        e.get_mut::<Attacker>().unwrap().last_tick_ms = Some(ms);
        e.remove::<Attacking>();
    }

    /// A press throws a right straight at once with the right fist as its
    /// primary striker. Held, the request stands: nothing while the attack
    /// plays, nothing until 330 ms (the fist's 300 × 0.1 + 300) after its
    /// last tick, then the next main-side attack, never the same pick.
    /// Released, nothing more.
    #[test]
    fn holding_the_button_repeats_the_attack_after_its_cooldown() {
        let (mut app, body) = attack_app();
        assert_eq!(
            request_tick(&mut app, body, true),
            ("Attack Up".into(), false)
        );
        let first = app.world().get::<Attacking>(body).unwrap().motion;
        assert_eq!(first, AttackMotion::Main(2));
        let attacker = *app.world().get::<Attacker>(body).unwrap();
        assert_eq!(attacker.primary, Some(Slot::First));
        for _ in 0..50 {
            request_tick(&mut app, body, true);
        }
        assert_eq!(app.world().get::<Attacking>(body).unwrap().motion, first);
        // The attack's last tick at 13 000 ms: the cooldown ends at 13 330.
        end_attack(&mut app, body, 13_000);
        app.world_mut().resource_mut::<BodyTicks>().0 = 1110; // 13 320 ms
        assert_eq!(
            request_tick(&mut app, body, true),
            ("Cmbt - Idle".into(), true)
        );
        // 1111 ticks: 13 332 ms.
        let (action, looping) = request_tick(&mut app, body, true);
        assert!(!looping && action != "Attack Up", "{action}");
        assert!(matches!(
            app.world().get::<Attacking>(body).unwrap().motion,
            AttackMotion::Main(_)
        ));
        end_attack(&mut app, body, 14_000);
        request_tick(&mut app, body, false);
        app.world_mut().resource_mut::<BodyTicks>().0 = 1300;
        for _ in 0..10 {
            assert_eq!(request_tick(&mut app, body, false).0, "Cmbt - Idle");
        }
    }

    /// Its condition as the posture stage would write it this tick.
    fn set_condition(app: &mut App, body: Entity, c: Condition) {
        *app.world_mut().get_mut::<Condition>(body).unwrap() = c;
    }

    fn swing_of(app: &App, body: Entity) -> SwingState {
        *app.world().get::<SwingState>(body).unwrap()
    }

    /// A click (pressed one tick) checks the punch: the next tick, the
    /// button up, releases its swing (stall 0.3 a tick, the stance due at
    /// 12 054 ms, Feint's renewal at 12 122); the punch plays on while the
    /// stall lasts and returns to the stance (the fists' blend 0) on the
    /// first tick after it, its condition normal, no longer attacking.
    #[test]
    fn a_click_checks_the_swing() {
        let (mut app, body) = attack_app();
        assert_eq!(
            request_tick(&mut app, body, true),
            ("Attack Up".into(), false)
        );
        set_condition(&mut app, body, Condition::ATTACKING);
        request_tick(&mut app, body, false);
        let s = swing_of(&app, body);
        assert!(s.has(swing::flags::STALLING) && s.has(swing::flags::RELEASED));
        assert!((s.stall - 0.3).abs() < 1e-6);
        assert_eq!((s.end_ms, s.renew_ms), (12_054, 12_122));
        for _ in 0..3 {
            assert_eq!(request_tick(&mut app, body, false).0, "Attack Up");
        }
        // 12 060 ms: past the stall's end.
        assert_eq!(
            request_tick(&mut app, body, false),
            ("Cmbt - Idle".into(), true)
        );
        let anim = app.world().get::<Animator>(body).unwrap();
        assert_eq!((anim.blend, anim.ease), (0.0, Ease::Smooth));
        assert_eq!(
            *app.world().get::<Condition>(body).unwrap(),
            Condition::NORMAL
        );
        app.update();
        assert!(app.world().get::<Attacking>(body).is_none());
        // Not attacking, its state keeps the release.
        request_tick(&mut app, body, false);
        assert_eq!(swing_of(&app, body).flags, swing::flags::RELEASED);
    }

    /// A casting body's buttons go to its cast hook: no attack starts.
    #[test]
    fn a_casting_body_asks_no_attack() {
        let (mut app, body) = attack_app();
        set_condition(&mut app, body, Condition::CASTING);
        let (clip, _) = request_tick(&mut app, body, true);
        assert_ne!(clip, "Attack Up");
        assert!(app.world().get::<Attacking>(body).is_none());
    }

    /// The attack start asks a cast hook with event 0x100: one that keeps
    /// its cast refuses the attack, one that does not lets it start.
    #[test]
    fn an_attack_start_asks_the_cast_hook() {
        let asked = |app: &mut App| {
            app.world_mut()
                .resource_mut::<Messages<exa_combat::AttackAskedHook>>()
                .drain()
                .count()
        };
        let (mut app, body) = attack_app();
        app.world_mut()
            .entity_mut(body)
            .insert(exa_combat::AttackHook(Some(true)));
        let (clip, _) = request_tick(&mut app, body, true);
        assert_ne!(clip, "Attack Up");
        assert_eq!(asked(&mut app), 1);
        let (mut app, body) = attack_app();
        app.world_mut()
            .entity_mut(body)
            .insert(exa_combat::AttackHook(Some(false)));
        assert_eq!(request_tick(&mut app, body, true).0, "Attack Up");
        assert_eq!(asked(&mut app), 1);
        let (mut app, body) = attack_app();
        app.world_mut()
            .entity_mut(body)
            .insert(exa_combat::AttackHook(None));
        assert_eq!(request_tick(&mut app, body, true).0, "Attack Up");
        assert_eq!(asked(&mut app), 0, "no hook, nothing asked");
    }

    /// Held, the punch is never released: its swing does not stall.
    #[test]
    fn a_held_button_keeps_the_swing() {
        let (mut app, body) = attack_app();
        request_tick(&mut app, body, true);
        set_condition(&mut app, body, Condition::ATTACKING);
        for _ in 0..10 {
            assert_eq!(request_tick(&mut app, body, true).0, "Attack Up");
        }
        assert!(!swing_of(&app, body).has(swing::flags::STALLING));
    }

    /// A tap that releases the punch, then a new press 216 ms after the
    /// punch's start (324 ms after the press before: no double click)
    /// throws the overhead with no blend, though the cooldown since the
    /// last attack tick has not passed; without the release the press
    /// would wait for it.
    #[test]
    fn a_press_soon_after_a_released_punch_throws_the_overhead() {
        let (mut app, body) = attack_app();
        app.world_mut()
            .get_mut::<Attacker>(body)
            .unwrap()
            .last_tick_ms = Some(11_700);
        // Pressed at 12 000, the punch waits 400 ms from 11 700: 12 108.
        assert_eq!(request_tick(&mut app, body, true).0, "Cmbt - Idle");
        app.world_mut().resource_mut::<BodyTicks>().0 = 1009;
        assert!(request_tick(&mut app, body, true).0.starts_with("Attack"));
        assert_eq!(
            app.world().get::<Attacker>(body).unwrap().side_started_ms,
            Some(12_108)
        );
        set_condition(&mut app, body, Condition::ATTACKING);
        app.world_mut()
            .get_mut::<Attacker>(body)
            .unwrap()
            .last_tick_ms = Some(12_108);
        request_tick(&mut app, body, false);
        assert!(swing_of(&app, body).has(swing::flags::RELEASED));
        // 12 324 ms: the cooldown (330 ms from 12 108) has not passed.
        app.world_mut().resource_mut::<BodyTicks>().0 = 1027;
        assert_eq!(
            request_tick(&mut app, body, true),
            ("Overhead Swing".into(), false)
        );
        let anim = app.world().get::<Animator>(body).unwrap();
        assert_eq!(anim.blend, 0.0);
        assert_eq!(
            app.world().get::<Attacking>(body).unwrap().motion,
            AttackMotion::Overhead
        );
    }

    /// Feint: after a released punch has returned to its stance, a new
    /// press past its renewal time (12 122 ms) and past the overhead's
    /// 300 ms starts the next punch at once, the cooldown waived; without
    /// the perk it waits for the cooldown.
    #[test]
    fn feint_renews_a_released_punch() {
        for feint in [true, false] {
            let (mut app, body) = attack_app();
            if feint {
                let mut perks = exa_formats::characters::Perks::default();
                perks.insert(exa_formats::characters::Perk::Feint);
                let f = app.world().get::<Fighter>(body).unwrap().clone();
                app.world_mut().entity_mut(body).insert(f.with_perks(perks));
            }
            request_tick(&mut app, body, true);
            set_condition(&mut app, body, Condition::ATTACKING);
            request_tick(&mut app, body, false);
            for _ in 0..4 {
                request_tick(&mut app, body, false);
            }
            set_condition(&mut app, body, Condition::COMBAT_IDLE);
            // Its last attack tick at 12 300: the cooldown lasts to 12 630.
            app.world_mut()
                .get_mut::<Attacker>(body)
                .unwrap()
                .last_tick_ms = Some(12_300);
            app.world_mut().resource_mut::<BodyTicks>().0 = 1034; // 12 408
            let (action, looping) = request_tick(&mut app, body, true);
            assert_eq!(!looping, feint, "{action}");
        }
    }

    /// The first unarmed attack waits 400 ms after an earlier attack's last
    /// tick (no primary striker yet); the cursor 25° to the left at the
    /// press throws the other side's attack with the left fist as primary.
    #[test]
    fn the_cursor_left_throws_the_left_fist() {
        let (mut app, body) = attack_app();
        app.world_mut()
            .get_mut::<Attacker>(body)
            .unwrap()
            .last_tick_ms = Some(11_700);
        app.world_mut().get_mut::<PlayerBody>(body).unwrap().aim = 25f32.to_radians();
        // 12 000 ms: 300 after the last tick.
        assert_eq!(request_tick(&mut app, body, true).0, "Cmbt - Idle");
        app.world_mut().resource_mut::<BodyTicks>().0 = 1009; // 12 108 ms
        assert_eq!(request_tick(&mut app, body, true).0, "Attack Combo 3");
        let a = app.world().get::<Attacker>(body).unwrap();
        assert_eq!(a.primary, Some(Slot::Second));
        let req = app.world().get::<Locomotion>(body).unwrap();
        assert_eq!(req.heading, 25f32.to_radians());
    }

    /// Below 80 % of its energy (dazed) the player's request stands but
    /// starts nothing; at 80 % it attacks.
    #[test]
    fn no_attack_below_80_percent_energy() {
        let (mut app, body) = attack_app();
        let part = exa_physics::body::PartSetup {
            bone: 1,
            entity: Entity::PLACEHOLDER,
            parent: None,
            world: bevy::math::Affine3A::IDENTITY,
            centre: Vec3::ZERO,
            half: Vec3::splat(0.08),
            origin: Vec3::ZERO,
            links: vec![],
            foot: None,
            pins: Vec::new(),
            spheres: None,
        };
        let mut ragdoll = Ragdoll::new(&[part]).with_drive();
        ragdoll.drive.as_mut().unwrap().energy = 0.79;
        app.world_mut().entity_mut(body).insert(ragdoll);
        for _ in 0..5 {
            assert_eq!(request_tick(&mut app, body, true).0, "Cmbt - Idle");
        }
        assert!(
            app.world()
                .get::<PlayerBody>(body)
                .unwrap()
                .attack
                .requested
        );
        let mut e = app.world_mut().entity_mut(body);
        e.get_mut::<Ragdoll>()
            .unwrap()
            .drive
            .as_mut()
            .unwrap()
            .energy = 0.8;
        assert!(request_tick(&mut app, body, true).0.starts_with("Attack"));
    }

    /// A feral player's gate is 80 % of its own most energy: with most
    /// 0.9 it waits at 0.71 and attacks at 0.72.
    #[test]
    fn a_feral_attack_waits_for_80_percent_of_its_most() {
        let (mut app, body) = attack_app();
        let part = exa_physics::body::PartSetup {
            bone: 1,
            entity: Entity::PLACEHOLDER,
            parent: None,
            world: bevy::math::Affine3A::IDENTITY,
            centre: Vec3::ZERO,
            half: Vec3::splat(0.08),
            origin: Vec3::ZERO,
            links: vec![],
            foot: None,
            pins: Vec::new(),
            spheres: None,
        };
        let feral = exa_physics::body::BodyStats {
            max_energy: 0.9,
            ..exa_physics::body::BodyStats::HUMAN
        };
        let mut ragdoll = Ragdoll::new(&[part]).with_drive().with_stats(feral);
        ragdoll.drive.as_mut().unwrap().energy = 0.71;
        app.world_mut().entity_mut(body).insert(ragdoll);
        assert_eq!(request_tick(&mut app, body, true).0, "Cmbt - Idle");
        let mut e = app.world_mut().entity_mut(body);
        e.get_mut::<Ragdoll>()
            .unwrap()
            .drive
            .as_mut()
            .unwrap()
            .energy = 0.72;
        assert!(request_tick(&mut app, body, true).0.starts_with("Attack"));
    }

    /// A click that came and went within a frame that ran no tick still
    /// reaches the next tick as a press.
    #[test]
    fn a_click_between_ticks_still_attacks() {
        let (mut app, body) = attack_app();
        app.add_systems(PreUpdate, latch_attack_button);
        let mut buttons = app.world_mut().resource_mut::<ButtonInput<MouseButton>>();
        buttons.press(MouseButton::Left);
        app.world_mut().run_schedule(PreUpdate);
        let mut buttons = app.world_mut().resource_mut::<ButtonInput<MouseButton>>();
        buttons.clear();
        buttons.release(MouseButton::Left);
        app.world_mut().run_schedule(FixedPreUpdate);
        assert!(!app.world().get::<Animator>(body).unwrap().looping);
    }

    #[test]
    fn the_combat_heading_turns_to_the_cursor_beyond_30_degrees() {
        let deg = |d: f32| d.to_radians();
        for heading in [0.0, 3.0, -3.0] {
            // Within 30° the heading stays, either side.
            for off in [29.5, -29.5] {
                let aim = heading + deg(off);
                assert_eq!(combat_heading(heading, aim, false), heading);
                // A held button (a standing attack request) follows it.
                assert_eq!(combat_heading(heading, aim, true), aim);
            }
            for off in [30.5, -30.5] {
                let aim = heading + deg(off);
                assert_eq!(combat_heading(heading, aim, false), aim);
            }
        }
        // An unordered aim fails the ordered compare and the heading stays.
        assert_eq!(combat_heading(1.0, f32::NAN, false), 1.0);
    }

    /// The camera follows the body only while the option is set. Off, or
    /// with no configuration, the orbit yaw is left alone.
    #[test]
    fn the_camera_follows_the_body_only_when_asked() {
        let run = |follow: bool| {
            let mut app = App::new();
            app.add_systems(Update, follow_body_camera)
                .init_resource::<exa_physics::sim_app::FrameCommands>();
            let body = Locomotor {
                yaw: 0.0,
                ..Default::default()
            };
            app.world_mut().spawn((PlayerBody::new(0.0), body));
            let camera_state = ChaseCamera {
                yaw: 1.0,
                ..Default::default()
            };
            let camera = app.world_mut().spawn(camera_state).id();
            if follow {
                app.insert_resource(crate::hw_config::HardwareConfig::new(
                    exa_formats::hw_config::Options {
                        cam_follow: true,
                        ..exa_formats::hw_config::Options::default()
                    },
                ));
            }
            app.update();
            app.world().get::<ChaseCamera>(camera).unwrap().yaw
        };
        assert_eq!(run(false), 1.0);
        let expect = crate::player_behaviour::camera_follow_yaw(1.0, 0.0).unwrap();
        assert!((run(true) - expect).abs() < 1e-5);
    }

    #[test]
    fn a_shot_places_the_camera_by_the_engines_yaw_and_distance() {
        let mut cam = ChaseCamera {
            zoom: 1900.0,
            shown_zoom: 1850.0,
            back_before: Vec3::X,
            wall_steps: 3,
            wall_tilt: -0.2,
            ..default()
        };
        // The story shot: the eye 600, 600, 1300 cm off the player.
        let focus = Vec3::new(10.0, 20.0, 30.0);
        cam.face_shot(focus + Vec3::new(600.0, 600.0, 1300.0), focus, Some(1300.0));
        let yaw = crate::nav_camera::planar_yaw(Vec3::new(600.0, 600.0, 1300.0).normalize());
        assert!((cam.yaw - yaw).abs() < 1e-6, "{} {yaw}", cam.yaw);
        // The engine's arc cosine, not atan2: about 5e-4 rad apart here.
        assert!((cam.yaw - 600f32.atan2(1300.0)).abs() < 2e-3);
        // The vector takes the shot's and stops; the wall avoidance starts
        // over.
        let back = Vec3::new(600.0, 600.0, 1300.0).normalize();
        assert!((cam.back - back).length() < 1e-6);
        assert_eq!((cam.shown_yaw, cam.back_before), (cam.yaw, cam.back));
        assert_eq!((cam.wall_steps, cam.wall_tilt), (0, 0.0));
        assert_eq!((cam.zoom, cam.shown_zoom), (1300.0, 1300.0));
        assert_eq!(
            (cam.zoom_spring.motion, cam.zoom_spring.previous),
            (1300.0, 1300.0)
        );
        // A distance outside the stops is held to them; the shown zoom
        // takes it as it is.
        cam.face_shot(focus + Vec3::X, focus, Some(5000.0));
        assert_eq!((cam.zoom, cam.shown_zoom), (2000.0, 5000.0));
        // A shot without a distance keeps the zoom.
        cam.zoom = 900.0;
        cam.face_shot(focus - Vec3::Z, focus, None);
        assert_eq!(cam.zoom, 900.0);
        assert!((cam.yaw.abs() - std::f32::consts::PI).abs() < 1e-6);
    }

    #[test]
    fn the_camera_ticks_every_17_ms_and_at_most_nine_a_frame() {
        let mut cam = ChaseCamera::behind(0.0);
        cam.zoom = 2000.0;
        // The first frame starts a tick that ends 7 ms after it: 10 ms in.
        let t = cam.advance(0.010);
        assert!((t - 0.010 / 0.017).abs() < 1e-5);
        assert_eq!((cam.frame.clock, cam.frame.frame), (17, 10));
        // 7 ms more reach its end: no new tick, all the way in.
        let zoom = cam.zoom_spring;
        assert!((cam.advance(0.007) - 1.0).abs() < 1e-4);
        assert_eq!(cam.zoom_spring, zoom);
        // A second's stall runs nine ticks and drops the rest.
        let mut nine = ChaseCamera::behind(0.0);
        nine.zoom = 2000.0;
        nine.zoom_spring = cam.zoom_spring;
        assert_eq!(cam.advance(1.0), 1.0);
        for _ in 0..9 {
            nine.tick(None);
        }
        assert_eq!(cam.zoom_spring, nine.zoom_spring);
        assert_eq!(cam.frame.clock, cam.frame.frame);
    }

    #[test]
    fn a_wall_probe_climbs_on_hits_comes_down_on_misses_and_raises_the_eye() {
        let focus = Vec3::new(1.0, 2.0, 3.0);
        // Nothing in the way: no step and no tilt, so the view is as it
        // was without the probe.
        let mut cam = ChaseCamera::behind(0.3);
        for _ in 0..5 {
            cam.tick(None);
            cam.probe_wall(focus, &mut |_, _, _, _| false);
        }
        assert_eq!((cam.wall_steps, cam.wall_tilt), (0, 0.0));
        let plain = cam.node_axis(cam.yaw);
        assert!((cam.backward() - plain).length() < 1e-6);
        // The first probe tests the present step: 30 cm, from 50 cm under
        // the focus, 38.5 m long, looking 0.05 rad higher than the view.
        let mut cam = ChaseCamera::behind(0.3);
        let back = cam.backward();
        let mut asked = Vec::new();
        cam.probe_wall(focus, &mut |start, direction, radius, length| {
            asked.push((start, direction, radius, length));
            true
        });
        let (start, direction, radius, length) = asked[0];
        assert_eq!(start, focus - Vec3::Y * 0.5);
        assert!((radius - 0.3).abs() < 1e-7 && (length - 38.5).abs() < 1e-5);
        let lift = direction.y.asin() - back.y.asin();
        assert!((lift - 0.05).abs() < 1e-4, "{lift}");
        // The heading is the engine's planar yaw, whose fast arc cosine is
        // a few 1e-4 rad off the true one.
        let across = |v: Vec3| Vec3::new(v.x, 0.0, v.z).normalize();
        assert!((across(direction) - across(back)).length() < 1e-3);
        // Hits climb every other probe to four; misses come down the same
        // way to none, never below.
        let mut steps = Vec::new();
        for _ in 0..9 {
            cam.probe_wall(focus, &mut |_, _, _, _| true);
            steps.push(cam.wall_steps);
        }
        assert_eq!(steps, [1, 2, 2, 3, 3, 4, 4, 4, 4]);
        // The tilt has eased upward and the node's axis it springs the
        // vector toward lifts the eye over the plain view.
        assert!(cam.wall_tilt < 0.0);
        let plain = crate::nav_camera::camera_axis(-cam.pitch, cam.yaw);
        assert!(cam.node_axis(cam.yaw).y > plain.y);
        let mut lifted = cam.clone();
        for _ in 0..60 {
            lifted.tick(None);
        }
        assert!(lifted.backward().y > plain.y);
        steps.clear();
        for _ in 0..9 {
            cam.probe_wall(focus, &mut |_, _, _, _| false);
            steps.push(cam.wall_steps);
        }
        assert_eq!(steps, [4, 3, 3, 2, 2, 1, 1, 0, 0]);
        // Each tick probes once.
        let mut cam = ChaseCamera::behind(0.0);
        let mut probes = 0;
        cam.advance_probing(
            0.05,
            Some(focus),
            &mut |_, _, _, _| {
                probes += 1;
                false
            },
            None,
        );
        assert_eq!(probes, 3);
        // A camera that follows nothing runs no probe.
        let mut free = ChaseCamera::behind(0.0);
        free.free_at(focus);
        let mut asked = 0;
        free.advance_probing(
            0.05,
            None,
            &mut |_, _, _, _| {
                asked += 1;
                true
            },
            None,
        );
        assert_eq!(asked, 0);
    }

    /// A free camera pans its looked-at point with W along the drawn
    /// forward axis, flattened, and it coasts to a stop once the key is
    /// let go; a followed camera has no free point.
    #[test]
    fn a_free_camera_pans_with_the_keys() {
        let mut cam = ChaseCamera::behind(0.0);
        assert!(!cam.is_free());
        cam.free_at(Vec3::new(1.0, 1.0, 1.0));
        assert!(cam.is_free());
        let no_ground = |_: Vec3| None;
        let input = |w: bool| FreeInput {
            keys: crate::nav_camera::PanKeys {
                forward: w,
                ..Default::default()
            },
            right: Vec3::X,
            // A forward axis tilted down: the pan is flattened.
            forward: Vec3::new(0.0, -0.7, -0.7),
            ground: &no_ground,
        };
        let held = input(true);
        for _ in 0..30 {
            cam.tick(Some(&held));
        }
        let at = cam.free_focus(1.0).unwrap();
        assert!(
            at.z < 0.0 && (at.x - 1.0).abs() < 1e-6 && (at.y - 1.0).abs() < 1e-6,
            "{at}"
        );
        let still = input(false);
        let mut last = at;
        for _ in 0..400 {
            cam.tick(Some(&still));
        }
        let rest = cam.free_focus(1.0).unwrap();
        assert!(rest.z < last.z);
        last = rest;
        cam.tick(Some(&still));
        assert_eq!(cam.free_focus(1.0).unwrap(), last);
        cam.follow();
        assert!(cam.free_focus(0.5).is_none());
    }

    /// The backward vector springs toward the node's axis: a turn the
    /// short way across ±π carries the target a whole turn with the shown
    /// yaw, the node leads the heading by at most π/4, and the vector comes
    /// to rest on the axis.
    #[test]
    fn the_backward_vector_springs_across_the_seam_to_the_node_axis() {
        use std::f32::consts::PI;
        let mut cam = ChaseCamera::behind(PI - 0.1 - PI);
        assert!((cam.shown_yaw - (PI - 0.1)).abs() < 1e-6);
        // Turn 0.3 rad past π: the shown yaw crosses to −π and the target
        // follows a whole turn down.
        cam.yaw = PI + 0.2;
        let mut crossed = false;
        let mut first = None;
        for _ in 0..200 {
            let before = cam.shown_yaw;
            cam.tick(None);
            first.get_or_insert(cam.back);
            if cam.shown_yaw < 0.0 && before > 0.0 {
                crossed = true;
            }
        }
        assert!(crossed);
        assert!(
            (cam.yaw - (PI + 0.2 - std::f32::consts::TAU)).abs() < 1e-5,
            "{}",
            cam.yaw
        );
        assert!(
            (cam.shown_yaw - cam.yaw).abs() < 1e-3,
            "{} {}",
            cam.shown_yaw,
            cam.yaw
        );
        assert!((cam.back - cam.node_axis(cam.yaw)).length() < 1e-3);
        // The first tick pulls by the stiffness times the retention.
        let start = crate::nav_camera::camera_axis(-cam.pitch, PI - 0.1);
        let preset = cam.controller.preset;
        let target = crate::nav_camera::camera_axis(-min_pitch(ZOOM_START), PI + 0.2);
        let expect = crate::thaum::engine::normalize(
            start + (target - start) * preset.view * preset.retention,
        );
        assert!((first.unwrap() - expect).length() < 1e-5);
    }

    /// A zoom step eases on the original's spring: its pull grows by at
    /// most 5 a tick, and a frame draws between its last two values; the
    /// stored shown zoom follows once 0.01 away.
    #[test]
    fn the_zoom_eases_on_its_capped_spring() {
        let mut cam = ChaseCamera::behind(0.0);
        cam.zoom = 2000.0;
        let mut moves = Vec::new();
        for _ in 0..4 {
            let was = cam.zoom_spring.motion;
            cam.tick(None);
            moves.push(cam.zoom_spring.motion - was);
        }
        // The pull (700 × 5/59 ≈ 59) is held to 5 more than the last move,
        // and the move carried at 0.998.
        let expect = [4.99, 9.97, 14.94, 19.9];
        for (got, want) in moves.iter().zip(expect) {
            assert!((got - want).abs() < 0.01, "{moves:?}");
        }
        // Nothing moved the stored shown zoom: frames do.
        assert_eq!(cam.shown_zoom, ZOOM_START);
        let drawn = cam.shown(0.5).1;
        assert_eq!(
            crate::nav_camera::frame_zoom_kept(cam.shown_zoom, drawn),
            drawn
        );
        for _ in 0..400 {
            cam.tick(None);
        }
        assert_eq!(cam.zoom_spring.motion, 2000.0);
    }

    #[test]
    fn the_camera_shows_its_last_two_ticks_blended() {
        let mut cam = ChaseCamera::behind(0.0);
        cam.zoom = 2000.0;
        cam.yaw += 0.5;
        cam.advance(0.017);
        cam.advance(0.017);
        let (b0, z0) = (cam.back_before, cam.zoom_spring.previous);
        let (b1, z1) = (cam.backward(), cam.zoom_spring.motion);
        assert!(z1 > z0);
        assert!(b1 != b0);
        let (b, z, _) = cam.shown(0.5);
        assert!((z - (z0 + z1) / 2.0).abs() < 1e-3);
        assert!((b - (b0 + b1).normalize()).length() < 1e-5);
        assert_eq!(cam.shown(1.0).1, z1);
        // The shake is shown between the tick before's offset and the
        // present one.
        cam.shake_before = Vec3::new(0.01, 0.0, 0.0);
        cam.shake = Vec3::new(0.03, 0.0, 0.0);
        assert!((cam.shown(0.5).2.x - 0.02).abs() < 1e-7);
        // The present offset decides: one ending drops out at once.
        cam.shake = Vec3::new(2e-5, 0.0, 0.0);
        assert_eq!(cam.shown(0.5).2, Vec3::ZERO);
    }

    /// A struck player's camera kicks by √(7Y + 7L) cm and its pad rumbles
    /// by 7Y + 7L; a blow too light (strength 1 or under) rumbles without
    /// a kick; a striking player's pad rumbles by half the yellow.
    #[test]
    fn blows_jolt_the_struck_player_and_rumble_the_striker() {
        let mut app = App::new();
        app.init_resource::<CameraKickRng>()
            .init_resource::<crate::gamepad::PlayPad>()
            .add_message::<exa_combat::pain::BlowFelt>()
            .add_message::<exa_combat::pain::ClashFelt>()
            .add_systems(Update, feel_blows);
        let player = app.world_mut().spawn(PlayerBody::new(0.0)).id();
        let other = app.world_mut().spawn_empty().id();
        let camera = app.world_mut().spawn(ChaseCamera::behind(0.0)).id();
        let blow = |target, attacker, wound: f32, yellow: f32| exa_combat::pain::BlowFelt {
            target,
            attacker,
            wound,
            yellow,
            striker: exa_combat::pain::Striker {
                scale: 1.0,
                speed: 10.0,
                overhead: false,
            },
            direction: Vec3::new(0.0, 0.0, 5.0),
        };
        // 7 × 0.2 + 7 × 0.3 = 3.5: a √3.5 cm kick and a full rumble.
        app.world_mut().write_message(blow(player, other, 0.3, 0.2));
        app.update();
        let cam = app.world().get::<ChaseCamera>(camera).unwrap();
        assert!((cam.shake.length() - 3.5f32.sqrt() * 0.01).abs() < 1e-6);
        assert_eq!(cam.shake_before, -cam.shake);
        let pad = app.world().resource::<crate::gamepad::PlayPad>();
        assert_eq!(pad.rumble.first, 1.0);
        // 7 × 0.1 = 0.7 kicks nothing.
        let mut fresh = app.world_mut().get_mut::<ChaseCamera>(camera).unwrap();
        *fresh = ChaseCamera::behind(0.0);
        app.world_mut()
            .resource_mut::<crate::gamepad::PlayPad>()
            .rumble = crate::gamepad::Rumble::default();
        app.world_mut().write_message(blow(player, other, 0.0, 0.1));
        app.update();
        assert_eq!(
            app.world().get::<ChaseCamera>(camera).unwrap().shake,
            Vec3::ZERO
        );
        let rumble = app.world().resource::<crate::gamepad::PlayPad>().rumble;
        assert!((rumble.first - 0.7).abs() < 1e-6);
        // The player strikes: half the yellow.
        app.world_mut()
            .resource_mut::<crate::gamepad::PlayPad>()
            .rumble = crate::gamepad::Rumble::default();
        app.world_mut().write_message(blow(other, player, 0.5, 0.4));
        app.update();
        let rumble = app.world().resource::<crate::gamepad::PlayPad>().rumble;
        assert_eq!(rumble.first, 0.2);
    }

    #[test]
    fn a_kick_rings_the_camera_down() {
        let mut cam = ChaseCamera::behind(0.0);
        // A 1 cm kick: the offset at 1 cm, the tick before's at −1 cm.
        cam.kick(Vec3::new(0.01, 0.0, 0.0));
        assert!((cam.shown(0.75).2.x - 0.005).abs() < 1e-7);
        // A tick keeps 0.96 of the 2 cm change, then 0.99 of the whole:
        // 0.99 × (1 + 0.96 × 2) = 2.8908 cm.
        cam.advance(0.017);
        assert!((cam.shake.x - 0.028_908).abs() < 1e-7, "{}", cam.shake.x);
        assert_eq!(cam.shake_before.x, 0.01);
        assert!((cam.shown(0.5).2.x - 0.019_454).abs() < 1e-7);
        // It wobbles back through nothing, still moving after 4 s and
        // stopped for good by 6 s.
        let mut crossed = false;
        for _ in 1..240 {
            cam.tick(None);
            crossed |= cam.shake.x < 0.0;
        }
        assert!(crossed);
        assert_ne!(cam.shake, Vec3::ZERO);
        for _ in 240..360 {
            cam.tick(None);
        }
        assert_eq!((cam.shake, cam.shake_before), (Vec3::ZERO, Vec3::ZERO));
        // Under the 1e-5 cm² floor a kick is not shown; one too small to
        // move (change within 1e-7 cm², offset within 1e-4 cm²) is dropped
        // by the next tick.
        cam.kick(Vec3::splat(1e-5));
        assert_eq!(cam.shown(1.0).2, Vec3::ZERO);
        let mut still = ChaseCamera::behind(0.0);
        still.kick(Vec3::new(1e-6, 0.0, 0.0));
        still.tick(None);
        assert_eq!((still.shake, still.shake_before), (Vec3::ZERO, Vec3::ZERO));
    }

    #[test]
    fn the_camera_follows_the_pelvis_or_the_feet_before_a_body() {
        let feet = Vec3::new(1.0, 0.0, 2.0);
        assert_eq!(camera_focus(feet, None, 0.5), feet);
        // A pelvis 1 m over the feet and a thigh under it: the focus is the
        // pelvis, the part without a parent.
        let part = |bone, parent, at: Vec3| exa_physics::body::PartSetup {
            bone,
            entity: Entity::PLACEHOLDER,
            parent,
            world: bevy::math::Affine3A::from_translation(at),
            centre: Vec3::ZERO,
            half: Vec3::splat(0.08),
            origin: Vec3::ZERO,
            links: vec![],
            foot: None,
            pins: Vec::new(),
            spheres: None,
        };
        let body = Ragdoll::new(&[
            part(1, None, feet + Vec3::Y),
            part(2, Some(0), feet + Vec3::new(0.1, 0.6, 0.0)),
        ]);
        let focus = camera_focus(feet, Some(&body), 0.5);
        assert!((focus - (feet + Vec3::Y)).length() < 1e-5, "{focus}");
    }

    #[test]
    fn zoom_is_field_of_view() {
        assert!((zoom_fov(2000.0, 1.6).to_degrees() - 45.0).abs() < 1e-3);
        assert!((zoom_fov(800.0, 1.6).to_degrees() - 18.0).abs() < 1e-3);
        assert!((min_pitch(2000.0) - 0.925).abs() < 1e-6);
        assert_eq!(min_pitch(900.0), 0.8);
    }

    /// Held standing, the crouch is 1 and stays 1; walking, the key does
    /// not raise it and it falls by 0.02 a tick, reaching 0 within 50
    /// ticks and staying there; down, the key holds it while moving.
    #[test]
    fn the_crouch_is_held_standing_and_eases_out() {
        let mut c = crouch_tick(0.0, true, false, false);
        assert_eq!(c, 1.0);
        c = crouch_tick(c, true, false, false);
        assert_eq!(c, 1.0);
        assert_eq!(crouch_tick(0.0, true, true, false), 0.0);
        assert_eq!(crouch_tick(0.3, true, true, true), 1.0);
        c = crouch_tick(c, true, true, false);
        assert!((c - 0.98).abs() < 1e-6);
        for _ in 1..50 {
            c = crouch_tick(c, false, false, false);
        }
        assert!(c.abs() < 1e-5, "{c}");
        // Rounding may leave a sliver above 0, which the next tick takes
        // below; from there it stays.
        let rest = crouch_tick(c, false, false, false);
        assert!(rest <= 0.0 && rest > -0.02, "{rest}");
        assert_eq!(crouch_tick(rest, false, false, false), rest);
    }

    #[test]
    fn cursor_look_turns_three_point_six_radians_across_the_window() {
        assert!((CursorLook::turn(1600.0, 1600.0) - 3.6).abs() < 1e-6);
        assert!((CursorLook::turn(800.0, 1600.0) - 1.8).abs() < 1e-6);
        assert!((CursorLook::turn(-800.0, 1600.0) + 1.8).abs() < 1e-6);
        // A zero width is clamped to one pixel rather than dividing by 0.
        assert_eq!(CursorLook::turn(10.0, 0.0), 36.0);
    }

    #[test]
    fn the_aim_point_lies_on_the_aim_plane_along_the_cursor() {
        let at = aim_point(
            Vec3::new(1.0, 5.0, 2.0),
            0.5,
            std::f32::consts::FRAC_PI_2,
            3.0,
        );
        assert!((at - Vec3::new(4.0, 1.3, 2.0)).length() < 1e-6, "{at}");
        // The plane hit takes no sign test: behind the origin counts.
        let hit = level_plane_hit(Vec3::new(0.0, 2.0, 0.0), Vec3::new(0.0, 1.0, 1.0), 1.0).unwrap();
        assert_eq!(hit, Vec3::new(0.0, 1.0, -1.0));
        assert_eq!(level_plane_hit(Vec3::Y, Vec3::X, 0.0), None);
        let (yaw, length) =
            flat_offset(Vec3::new(1.0, 9.0, 1.0), Vec3::new(1.0, 0.0, 3.0)).unwrap();
        assert_eq!((yaw, length), (0.0, 2.0));
        assert_eq!(flat_offset(Vec3::ZERO, Vec3::Y), None);
    }

    #[test]
    fn an_alignment_holds_until_the_shown_yaw_is_within_a_hundredth() {
        let mut cam = ChaseCamera::default();
        cam.align(1.0);
        assert!(cam.aligning);
        assert_eq!(cam.yaw, 1.0);
        // Within half a turn of the shown yaw.
        cam.align(std::f32::consts::TAU - 0.5);
        assert!((cam.yaw + 0.5).abs() < 1e-6, "{}", cam.yaw);
        for _ in 0..400 {
            cam.tick(None);
        }
        assert!(!cam.aligning);
        assert!((cam.yaw - cam.shown_yaw).abs() < 0.01);
    }

    /// The capture's state through a Cursor Look hold and an orbit hold
    /// (no camera, so no aim point: the warp goes back where the pointer
    /// was).
    #[test]
    fn the_orbit_and_cursor_look_share_one_capture() {
        let mut app = App::new();
        app.init_resource::<CursorLook>()
            .init_resource::<LookCapture>()
            .init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<ButtonInput<MouseButton>>()
            .init_resource::<Bindings>()
            .init_resource::<AccumulatedMouseMotion>()
            .init_resource::<CursorCapture>()
            .init_resource::<crate::panel_live::PointerOverGui>()
            .add_systems(Update, cursor_look);
        let window = app
            .world_mut()
            .spawn((
                Window {
                    resolution: bevy::window::WindowResolution::new(800, 600),
                    focused: true,
                    ..default()
                },
                CursorOptions::default(),
                PrimaryWindow,
            ))
            .id();
        app.world_mut()
            .entity_mut(window)
            .get_mut::<Window>()
            .unwrap()
            .set_cursor_position(Some(Vec2::new(100.0, 50.0)));
        let body = app.world_mut().spawn(PlayerBody::new(0.0)).id();
        app.world_mut()
            .spawn((Player, GlobalTransform::default()))
            .add_child(body);
        let press = |app: &mut App, key: Option<KeyCode>, button: Option<MouseButton>, dx: f32| {
            let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            keys.reset_all();
            keys.release_all();
            if let Some(key) = key {
                keys.press(key);
            }
            let mut buttons = app.world_mut().resource_mut::<ButtonInput<MouseButton>>();
            buttons.release_all();
            if let Some(button) = button {
                buttons.press(button);
            }
            app.world_mut()
                .resource_mut::<AccumulatedMouseMotion>()
                .delta = Vec2::new(dx, 0.0);
            app.update();
        };
        // Cursor Look: the first frame fixes the kind and starts the
        // target at the follower's (the heading, 0).
        press(&mut app, Some(KeyCode::KeyV), None, 400.0);
        let look = app.world().resource::<CursorLook>();
        assert!(look.active && look.turns && look.began);
        assert_eq!(look.target, Some(0.0));
        assert_eq!(
            app.world().resource::<LookCapture>().0,
            crate::steering::Capture {
                held: true,
                look: Some(0.0)
            }
        );
        // Later frames turn it by 3.6 × Δx / width; the orbit's button
        // does not change the kind.
        press(
            &mut app,
            Some(KeyCode::KeyV),
            Some(MouseButton::Middle),
            400.0,
        );
        let look = app.world().resource::<CursorLook>();
        assert!(look.turns && !look.began);
        assert!((look.target.unwrap() - 1.8).abs() < 1e-6);
        press(&mut app, None, Some(MouseButton::Middle), 0.0);
        assert!(app.world().resource::<CursorLook>().active);
        // Both let go: the capture ends and the pointer goes back.
        press(&mut app, None, None, 0.0);
        let look = app.world().resource::<CursorLook>();
        assert!(!look.active && look.ended && look.target.is_none());
        assert_eq!(app.world().resource::<LookCapture>().0, Default::default());
        let window = app.world().entity(window).get::<Window>().unwrap();
        assert_eq!(window.cursor_position(), Some(Vec2::new(100.0, 50.0)));
        // The orbit's capture: no target, the cursor only frozen.
        press(&mut app, None, Some(MouseButton::Middle), 10.0);
        let look = app.world().resource::<CursorLook>();
        assert!(look.orbits() && look.target.is_none());
        assert_eq!(
            app.world().resource::<LookCapture>().0,
            crate::steering::Capture {
                held: true,
                look: None
            }
        );
    }
}
