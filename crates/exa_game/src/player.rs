//! The player character, steered as the original steers it, and the
//! original's chase camera. Rules: `docs/PLAYER.md`.
//!
//! Exploration uses tank controls: W walks along the character's heading,
//! A and D steer it (or turn a quarter in place when standing), S brakes
//! and backpedals, and the right mouse button walks toward the cursor. The
//! camera follows the character's position only: it never turns by itself,
//! the middle mouse button orbits it and the wheel zooms by field of view.
//! Combat mode (Tab) turns toward the cursor and steps relative to it; the
//! left button attacks with the clips of the moveset the held items pick
//! (`exa_character::held`).

use bevy::input::mouse::{AccumulatedMouseMotion, AccumulatedMouseScroll};
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use exa_anim::Animator;
use exa_character::held::{Hands, HeldMoveset, class};
use exa_combat::{Attacking, Fighter};
use exa_physics::body::Fallen;
use exa_physics::{CharacterBody, Physics};

/// Walking speed (m/s): where the original's walk drive settles.
const WALK_SPEED: f32 = 1.4;
/// Running speed (m/s) at a full run blend.
const RUN_SPEED: f32 = 3.6;
/// Backpedalling speed (m/s): the walk scaled by the gait table's thrusts
/// (77.8 against 88.9).
const BACK_SPEED: f32 = WALK_SPEED * 77.8 / 88.9;
/// The behaviour tick (the fixed step), per second.
const TICK_HZ: f32 = exa_physics::STEP_HZ as f32;
/// A and D turn the heading this much per tick while moving (rad).
const STEER: f32 = 0.025;
/// Standing, A and D turn a quarter, at most this often (s).
const QUARTER_TURN_EVERY: f32 = 1.0;
/// The body's yaw follows the heading at up to this rate (rad/s).
const BODY_TURN: f32 = 4.188_79;
/// The body turn's rate shrinks over the last part of a turn (rad).
const TURN_EASE: f32 = 0.6;
/// The run blend rises and falls this much per tick.
const RUN_RISE: f32 = 0.05;
const RUN_FALL: f32 = 0.015;
/// The right mouse button walks to a cursor this close (m) and runs to
/// one further away.
const CURSOR_RUN_FROM: f32 = 3.0;
/// A cursor closer than this (m) gives no direction.
const CURSOR_MIN: f32 = 0.5;
/// S faster than this (m/s) brakes before backpedalling (1 cm per tick).
const BRAKE_ABOVE: f32 = 0.01 * TICK_HZ;
/// The cursor's ground plane lies this far above the feet (hip height, m).
const HIP: f32 = 0.9;
/// Combat footwork held longer than this (s) walks on.
const STEP_HOLD: f32 = 0.2;
/// In combat mode the heading turns to the cursor once it is further off
/// than this (rad, 30°).
const FACE_CURSOR_BEYOND: f32 = std::f32::consts::FRAC_PI_6;
/// A click this soon (s) after the one before is a double click.
const DOUBLE_CLICK: f32 = 0.3;
/// A cursor further left of the facing than this (rad) attacks on the
/// other side: 20°, or 15° for weapon class 0.
const OTHER_SIDE: f32 = 20.0 * std::f32::consts::PI / 180.0;
const OTHER_SIDE_EMPTY: f32 = 15.0 * std::f32::consts::PI / 180.0;
/// The attacks of each side, picked among at random.
const MAIN_SIDE: [&str; 3] = ["Attack Mid", "Attack Down", "Attack Up"];
const OTHER_SIDE_ATTACKS: [&str; 3] = ["Attack Combo 1", "Attack Combo 2", "Attack Combo 3"];

/// The player's physics root (origin at the feet).
#[derive(Component)]
pub struct Player;

/// The visual body under the player root. The player is human and is
/// kept in underwear, as the original keeps its player's record; what its
/// hands hold picks its moveset.
#[derive(Component, Debug)]
#[require(exa_character::WearsUnderwear, Hands)]
pub struct PlayerBody {
    /// The body's yaw (rad; the model faces +Z).
    pub yaw: f32,
    /// The heading the body turns toward and walks along.
    pub heading: f32,
    pub combat: bool,
    /// In combat mode, the cursor's direction from the player (a yaw): the
    /// footwork steps along and across it.
    pub aim: f32,
    /// The last click and attack picks, for the attack choice.
    attack: AttackChoice,
    /// 0 walks, 1 runs.
    run: f32,
    /// Seconds until A or D may turn a quarter again.
    quarter_wait: f32,
    /// Seconds the combat footwork keys have been held.
    held: f32,
}

impl PlayerBody {
    /// A body facing `yaw`.
    pub fn new(yaw: f32) -> Self {
        Self {
            yaw,
            heading: yaw,
            combat: false,
            aim: yaw,
            attack: AttackChoice::default(),
            run: 0.0,
            quarter_wait: 0.0,
            held: 0.0,
        }
    }

    /// The run blend: 0 walks, 1 runs.
    pub fn run(&self) -> f32 {
        self.run
    }

    /// What is left of the body's turn toward its heading (rad).
    pub fn turn_left(&self) -> f32 {
        turn_between(self.yaw, self.heading)
    }
}

/// What a left click in combat mode asks for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Strike {
    /// One of the main side's attacks (the right fist, the weapon's
    /// forehand).
    Main,
    /// One of the other side's (the left fist for the fists).
    Other,
    /// The overhead swing of a double click.
    Overhead,
}

/// The original's attack choice: the time of the last click (for double
/// clicks), and each side's last pick with the generator that picks.
#[derive(Debug, Default)]
struct AttackChoice {
    last_click: Option<f32>,
    seed: u32,
    /// The last pick of each side, 0 until the first (as the original's
    /// zeroed fields, so the first pick of a side is never its first clip).
    last: [usize; 2],
}

impl AttackChoice {
    /// A click at `now` (s): an overhead when it comes within 300 ms of the
    /// click before (which then no longer counts), else the side the cursor
    /// gives.
    fn click(&mut self, now: f32, other_side: bool) -> Strike {
        let double = self.last_click.is_some_and(|t| now < t + DOUBLE_CLICK);
        self.last_click = if double { None } else { Some(now) };
        match (double, other_side) {
            (true, _) => Strike::Overhead,
            (false, false) => Strike::Main,
            (false, true) => Strike::Other,
        }
    }

    /// One of a side's three attacks at random, never the one it picked
    /// last, by the original's 32-bit generator.
    fn pick(&mut self, side: usize) -> usize {
        loop {
            self.seed = self.seed.wrapping_mul(0x0808_8405).wrapping_add(1);
            let i = ((self.seed as u64 * 3) >> 32) as usize;
            if i != self.last[side] {
                self.last[side] = i;
                return i;
            }
        }
    }
}

/// The combat-mode heading toward a cursor at yaw `aim`: the cursor's
/// direction once it is more than 30° off, or at once while the left
/// button is held (an attack request stands); else unchanged.
fn combat_heading(heading: f32, aim: f32, requesting: bool) -> f32 {
    if requesting || turn_between(heading, aim).abs() > FACE_CURSOR_BEYOND {
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

/// The point under the cursor on the player's hip plane, for the right
/// mouse button.
#[derive(Resource, Default)]
struct CursorGround(Option<Vec3>);

/// A cursor position (logical pixels in the window) that stands in for the
/// pointer, for unattended runs (`--cursor`).
#[derive(Resource, Clone, Copy, Debug)]
pub struct ScriptedCursor(pub Vec2);

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CursorGround>()
            .add_systems(FixedPreUpdate, steer_player)
            .add_systems(
                Update,
                (cursor_ground, toggle_combat, player_combat, chase_camera).chain(),
            );
    }
}

/// The yaw (model facing +Z) of a horizontal direction.
fn yaw_of(d: Vec3) -> f32 {
    d.x.atan2(d.z)
}

/// The horizontal direction of a yaw.
fn forward(yaw: f32) -> Vec3 {
    Vec3::new(yaw.sin(), 0.0, yaw.cos())
}

/// The signed shortest turn from `from` to `to` (rad).
fn turn_between(from: f32, to: f32) -> f32 {
    (to - from + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI
}

/// One tick of the body's turn toward the heading.
fn body_turn(yaw: f32, heading: f32, keyboard: bool, speed: f32) -> f32 {
    let error = turn_between(yaw, heading);
    let mut rate = BODY_TURN / TICK_HZ;
    // Slower while moving fast (speed in cm per tick), and with the keys.
    rate *= (1.0 - 0.1 * speed * 100.0 / TICK_HZ).max(0.2);
    if keyboard {
        rate *= 0.9;
    }
    rate *= (error.abs() / TURN_EASE).min(1.0);
    yaw + error.clamp(-rate, rate)
}

fn cursor_ground(
    mut ground: ResMut<CursorGround>,
    scripted: Option<Res<ScriptedCursor>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    cameras: Query<(&Camera, &GlobalTransform), With<ChaseCamera>>,
    players: Query<&GlobalTransform, With<Player>>,
) {
    ground.0 = None;
    let (Ok(window), Ok((camera, eye)), Some(player)) =
        (windows.single(), cameras.single(), players.iter().next())
    else {
        return;
    };
    let Some(ray) = scripted
        .map(|s| s.0)
        .or_else(|| window.cursor_position())
        .and_then(|c| camera.viewport_to_world(eye, c).ok())
    else {
        return;
    };
    let plane = player.translation().y + HIP;
    if ray.direction.y.abs() > 1e-4 {
        let d = (plane - ray.origin.y) / ray.direction.y;
        if d > 0.0 {
            ground.0 = Some(ray.origin + *ray.direction * d);
        }
    }
}

#[allow(clippy::type_complexity)]
fn steer_player(
    keys: Res<ButtonInput<KeyCode>>,
    buttons: Res<ButtonInput<MouseButton>>,
    cursor: Res<CursorGround>,
    physics: Res<Physics>,
    mut players: Query<(&mut CharacterBody, &GlobalTransform, &Children), With<Player>>,
    mut bodies: Query<(
        &mut PlayerBody,
        &mut Transform,
        &mut Animator,
        Option<&Fighter>,
        Has<Fallen>,
    )>,
) {
    let dt = 1.0 / TICK_HZ;
    for (mut body, at, children) in &mut players {
        for child in children.iter() {
            let Ok((mut pb, mut t, mut anim, fighter, fallen)) = bodies.get_mut(child) else {
                continue;
            };
            if fallen || fighter.is_some_and(|f| f.down) {
                body.desired_velocity = Vec3::ZERO;
                continue;
            }
            let feet = at.translation();
            let speed = body.desired_velocity.length();
            if pb.combat {
                // The body turns toward the heading, which the cursor sets.
                pb.yaw = body_turn(pb.yaw, pb.heading, false, speed);
                t.rotation = Quat::from_rotation_y(pb.yaw);
                combat_steps(&keys, &mut body, &mut pb, &mut anim, dt);
                continue;
            }
            let held = |k| keys.pressed(k);
            let to_cursor = cursor
                .0
                .map(|c| c - (feet + Vec3::Y * HIP))
                .filter(|d| d.xz().length() >= CURSOR_MIN);
            let mouse = buttons.pressed(MouseButton::Right) && to_cursor.is_some();
            let ahead = held(KeyCode::KeyW) || mouse;
            let back = held(KeyCode::KeyS) && !mouse;
            let steer = held(KeyCode::KeyA) as i32 - held(KeyCode::KeyD) as i32;
            pb.quarter_wait = (pb.quarter_wait - dt).max(0.0);
            let mut running = held(KeyCode::ShiftLeft);
            if mouse && let Some(d) = to_cursor {
                pb.heading = yaw_of(d);
                running |= d.xz().length() > CURSOR_RUN_FROM;
            } else if ahead || back {
                // W walks the heading; A and D steer it.
                pb.heading += steer as f32 * STEER;
            } else if steer != 0 && pb.quarter_wait == 0.0 {
                // Standing: a quarter turn in place.
                pb.heading += steer as f32 * std::f32::consts::FRAC_PI_2;
                pb.quarter_wait = QUARTER_TURN_EVERY;
            }
            pb.run = if running && ahead {
                (pb.run + RUN_RISE).min(1.0)
            } else {
                (pb.run - RUN_FALL).max(0.0)
            };
            pb.yaw = body_turn(pb.yaw, pb.heading, !mouse, speed);
            t.rotation = Quat::from_rotation_y(pb.yaw);
            // A reversal of more than 120° turns before walking.
            let facing_away =
                turn_between(pb.yaw, pb.heading).abs() > 2.0 * std::f32::consts::FRAC_PI_3;
            let heading = forward(pb.heading);
            let mut velocity = if ahead && !facing_away {
                heading * (WALK_SPEED + (RUN_SPEED - WALK_SPEED) * pb.run)
            } else if back {
                if speed > BRAKE_ABOVE && body.desired_velocity.dot(heading) > 0.0 {
                    Vec3::ZERO
                } else {
                    -heading * BACK_SPEED
                }
            } else {
                Vec3::ZERO
            };
            // A look ahead: blocked within reach, the character stops
            // rather than sliding along the wall.
            if velocity != Vec3::ZERO {
                let dir = velocity.normalize();
                let v = velocity.length() * 100.0 / TICK_HZ;
                let reach = 0.75 + 0.06 * v * v;
                if physics
                    .fixed_hit(feet + Vec3::Y * HIP, dir, reach.min(1.0))
                    .is_some()
                {
                    velocity = Vec3::ZERO;
                }
            }
            body.desired_velocity = velocity;
            let action = if velocity == Vec3::ZERO {
                "Idle"
            } else if back {
                "Backpedal"
            } else if pb.run > 0.5 {
                "Run"
            } else {
                "Walk"
            };
            if anim.looping && anim.action != action {
                anim.play_loop(action);
            }
        }
    }
}

/// Combat footwork: W, S, A and D step along and across the cursor
/// direction; held, they walk on.
fn combat_steps(
    keys: &ButtonInput<KeyCode>,
    body: &mut CharacterBody,
    pb: &mut PlayerBody,
    anim: &mut Animator,
    dt: f32,
) {
    let look = forward(pb.aim);
    let side = Vec3::new(look.z, 0.0, -look.x);
    let mut dir = Vec3::ZERO;
    for (key, d) in [
        (KeyCode::KeyW, look),
        (KeyCode::KeyS, -look),
        (KeyCode::KeyA, side),
        (KeyCode::KeyD, -side),
    ] {
        if keys.pressed(key) {
            dir += d;
        }
    }
    let dir = dir.normalize_or_zero();
    pb.held = if dir == Vec3::ZERO { 0.0 } else { pb.held + dt };
    let walking = dir != Vec3::ZERO && pb.held >= STEP_HOLD;
    body.desired_velocity = if dir != Vec3::ZERO {
        dir * WALK_SPEED
    } else {
        Vec3::ZERO
    };
    let action = if walking || dir != Vec3::ZERO {
        "Combat Walk"
    } else {
        "Cmbt - Idle"
    };
    if anim.looping && anim.action != action {
        anim.play_loop(action);
    }
}

fn toggle_combat(
    keys: Res<ButtonInput<KeyCode>>,
    mut bodies: Query<(&mut PlayerBody, &mut Animator)>,
) {
    if !keys.just_pressed(KeyCode::Tab) {
        return;
    }
    for (mut pb, mut anim) in &mut bodies {
        pb.combat = !pb.combat;
        pb.aim = pb.heading;
        anim.play_loop(if pb.combat { "Cmbt - Idle" } else { "Idle" });
    }
}

/// Combat mode (Tab): the heading turns to the cursor once it is more than
/// 30° off, and follows it while the left button is held. A left click
/// attacks: on the main side, or on the other side when the cursor is far
/// enough left of the body's facing, or overhead on a double click.
#[allow(clippy::type_complexity)]
fn player_combat(
    mut commands: Commands,
    time: Res<Time>,
    buttons: Res<ButtonInput<MouseButton>>,
    cursor: Res<CursorGround>,
    players: Query<&GlobalTransform, With<Player>>,
    mut bodies: Query<(
        Entity,
        &mut PlayerBody,
        &mut Animator,
        Option<&HeldMoveset>,
        Option<&Fighter>,
        Option<&Attacking>,
        Has<Fallen>,
    )>,
) {
    let Some(player) = players.iter().next() else {
        return;
    };
    for (entity, mut pb, mut anim, held, fighter, attacking, fallen) in &mut bodies {
        if fallen || fighter.is_some_and(|f| f.down) || !pb.combat {
            continue;
        }
        let aim = cursor
            .0
            .map(|c| c - player.translation())
            .filter(|to| to.xz().length() > 0.2)
            .map(yaw_of);
        if let Some(aim) = aim {
            pb.aim = aim;
            pb.heading = combat_heading(pb.heading, aim, buttons.pressed(MouseButton::Left));
        }
        if !buttons.just_pressed(MouseButton::Left) {
            continue;
        }
        let weapon_class = held.map_or(class::FISTS, |h| h.weapon_class);
        let other = aim.is_some_and(|a| other_side(a, pb.yaw, weapon_class));
        let strike = pb.attack.click(time.elapsed_secs(), other);
        let resting = attacking.is_none() && anim.looping;
        let action = match strike {
            // An overhead cuts a side's attack short, but not a stab or
            // another overhead.
            Strike::Overhead => (!matches!(anim.action.as_str(), "Stab" | "Overhead Swing"))
                .then_some("Overhead Swing"),
            _ if !resting => None,
            Strike::Main => Some(MAIN_SIDE[pb.attack.pick(0)]),
            Strike::Other => Some(OTHER_SIDE_ATTACKS[pb.attack.pick(1)]),
        };
        if let Some(action) = action {
            info!(
                "attack: {action} in {} (cursor {:.0}° left of the facing)",
                anim.moveset,
                aim.map_or(0.0, |a| turn_between(pb.yaw, a).to_degrees())
            );
            exa_combat::start_attack(&mut commands, entity, &mut anim, action, "Cmbt - Idle");
        }
    }
}

// ---- Camera ---------------------------------------------------------------

/// The camera's distance from the player (m): fixed, as the original's;
/// zoom narrows the field of view instead.
const CAMERA_DISTANCE: f32 = 20.0;
/// The camera's tick (s) and the spring constants per tick.
const CAMERA_TICK: f32 = 0.017;
const ORBIT_STIFFNESS: f32 = 0.0718;
const ORBIT_RETENTION: f32 = 0.616;
/// The yaw leads the shown yaw by at most this per tick (rad).
const MAX_YAW_LEAD: f32 = std::f32::consts::FRAC_PI_4;
/// Orbit drag (middle mouse), rad per screen width.
const ORBIT_DRAG: f32 = 4.8;
/// Zoom stops: 800 + 240·level, in the original's units (2000 = 45°).
const ZOOM_STOPS: usize = 6;
const ZOOM_START: f32 = 1300.0;
/// The steepest pitch the orbit drag reaches (rad below the horizon).
const MAX_PITCH: f32 = 1.5;

#[derive(Component, Debug)]
pub struct ChaseCamera {
    /// The orbit's target yaw and the shown yaw (rad).
    pub yaw: f32,
    shown_yaw: f32,
    yaw_speed: f32,
    /// The pitch asked for by the orbit drag, and the shown pitch (rad
    /// below the horizon).
    user_pitch: f32,
    pitch: f32,
    /// Zoom: the target and the shown value (800–2000).
    zoom: f32,
    shown_zoom: f32,
    /// Time left over from the last camera tick (s).
    carry: f32,
}

impl Default for ChaseCamera {
    fn default() -> Self {
        Self {
            yaw: 0.0,
            shown_yaw: 0.0,
            yaw_speed: 0.0,
            user_pitch: 0.0,
            pitch: min_pitch(ZOOM_START),
            zoom: ZOOM_START,
            shown_zoom: ZOOM_START,
            carry: 0.0,
        }
    }
}

impl ChaseCamera {
    /// A camera behind a character facing `yaw`.
    pub fn behind(yaw: f32) -> Self {
        let yaw = yaw + std::f32::consts::PI;
        Self {
            yaw,
            shown_yaw: yaw,
            ..default()
        }
    }
}

/// The shallowest pitch at a zoom: 0.8 rad, steeper when zoomed out.
fn min_pitch(zoom: f32) -> f32 {
    if zoom < 1000.0 {
        0.8
    } else {
        0.8 + (zoom - 1000.0) * 0.000_125
    }
}

/// The field of view (rad) at a zoom, for the window's aspect.
fn zoom_fov(zoom: f32, aspect: f32) -> f32 {
    (zoom / 2000.0 * 45.0 * (1.6 / aspect.max(0.1)).sqrt())
        .clamp(10.0, 100.0)
        .to_radians()
}

/// The camera ticks on real time, so it keeps orbiting while the
/// simulation is paused or slowed (`docs/DEBUGGING.md`).
#[allow(clippy::type_complexity)]
fn chase_camera(
    time: Res<Time<Real>>,
    buttons: Res<ButtonInput<MouseButton>>,
    motion: Res<AccumulatedMouseMotion>,
    scroll: Res<AccumulatedMouseScroll>,
    windows: Query<&Window, With<PrimaryWindow>>,
    players: Query<&Transform, (With<Player>, Without<ChaseCamera>)>,
    mut cameras: Query<(&mut ChaseCamera, &mut Transform, &mut Projection)>,
) {
    let Some(target) = players.iter().next() else {
        return;
    };
    let (width, aspect) = windows.single().map_or((1600.0, 1.6), |w| {
        (w.width().max(1.0), w.width() / w.height().max(1.0))
    });
    for (mut cam, mut t, mut projection) in &mut cameras {
        if buttons.pressed(MouseButton::Middle) {
            cam.yaw -= motion.delta.x * ORBIT_DRAG / width;
            cam.user_pitch += motion.delta.y * ORBIT_DRAG / width;
        }
        if scroll.delta.y != 0.0 {
            let level = ((cam.zoom - 800.0) / 240.0).round() - scroll.delta.y.signum();
            cam.zoom = 800.0 + 240.0 * level.clamp(0.0, (ZOOM_STOPS - 1) as f32);
        }
        let floor = min_pitch(cam.shown_zoom);
        cam.user_pitch = cam.user_pitch.clamp(floor, MAX_PITCH);
        // The camera's own tick: springs and eases at 17 ms.
        cam.carry += time.delta_secs();
        while cam.carry >= CAMERA_TICK {
            cam.carry -= CAMERA_TICK;
            let lead = turn_between(cam.shown_yaw, cam.yaw).clamp(-MAX_YAW_LEAD, MAX_YAW_LEAD);
            cam.yaw_speed = ORBIT_RETENTION * (cam.yaw_speed + ORBIT_STIFFNESS * lead);
            cam.shown_yaw += cam.yaw_speed;
            let target_pitch = cam.user_pitch.max(min_pitch(cam.shown_zoom));
            cam.pitch += (target_pitch - cam.pitch) * 0.1;
            let ease = 5.0 / 59.0;
            cam.shown_zoom += (cam.zoom - cam.shown_zoom) * ease;
        }
        let focus = target.translation;
        let rot = Quat::from_euler(EulerRot::YXZ, cam.shown_yaw, -cam.pitch, 0.0);
        t.translation = focus + rot * Vec3::new(0.0, 0.0, CAMERA_DISTANCE);
        t.look_at(focus, Vec3::Y);
        if let Projection::Perspective(p) = projection.as_mut() {
            p.fov = zoom_fov(cam.shown_zoom, aspect);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn steering_with_w_and_a_walks_a_circle() {
        // 0.025 rad per tick at walking speed: a circle of v / ω.
        let radius = (WALK_SPEED / TICK_HZ) / STEER;
        assert!((radius - 0.67).abs() < 0.05, "{radius}");
    }

    #[test]
    fn the_body_turns_toward_the_heading() {
        let mut yaw = 0.0;
        for _ in 0..83 {
            yaw = body_turn(yaw, std::f32::consts::FRAC_PI_2, true, 0.0);
        }
        assert!((yaw - std::f32::consts::FRAC_PI_2).abs() < 0.05, "{yaw}");
        // At most 240°/s × 0.9 in the first tick.
        let first = body_turn(0.0, 3.0, true, 0.0);
        assert!((first - BODY_TURN * 0.9 / TICK_HZ).abs() < 1e-4);
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
    fn the_player_holds_its_fists() {
        let mut world = World::new();
        let body = world.spawn(PlayerBody::new(0.0)).id();
        let held = world.entity(body).get::<HeldMoveset>().unwrap();
        assert_eq!(
            (held.moveset.as_str(), held.weapon_class),
            ("Default", class::FISTS)
        );
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

    #[test]
    fn a_second_click_within_300_ms_is_an_overhead() {
        let mut choice = AttackChoice::default();
        assert_eq!(choice.click(10.0, false), Strike::Main);
        assert_eq!(choice.click(10.29, false), Strike::Overhead);
        // The double click used both clicks up.
        assert_eq!(choice.click(10.4, true), Strike::Other);
        assert_eq!(choice.click(10.75, false), Strike::Main);
        assert_eq!(choice.click(11.1, false), Strike::Main);
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
    }

    #[test]
    fn the_picks_follow_the_original_generator() {
        // × 0x08088405 + 1 from 0, clip = state × 3 >> 32, redrawn while
        // it equals the last pick (0 at first).
        let mut choice = AttackChoice::default();
        let picks: Vec<usize> = (0..8).map(|_| choice.pick(0)).collect();
        assert_eq!(picks, [2, 0, 2, 0, 1, 0, 1, 0]);
        assert_eq!(choice.seed, 303_092_965);
    }

    #[test]
    fn a_side_never_repeats_its_last_attack() {
        let mut choice = AttackChoice::default();
        let picks: Vec<usize> = (0..200).map(|_| choice.pick(0)).collect();
        // The first pick is never the first clip: the last pick starts at 0.
        assert_ne!(picks[0], 0);
        assert!(picks.windows(2).all(|w| w[0] != w[1]));
        for i in 0..3 {
            let n = picks.iter().filter(|&&p| p == i).count();
            assert!((50..=84).contains(&n), "clip {i}: {n} of 200");
        }
        // Each side keeps its own last pick.
        assert_ne!(choice.pick(1), 0);
    }

    #[test]
    fn zoom_is_field_of_view() {
        assert!((zoom_fov(2000.0, 1.6).to_degrees() - 45.0).abs() < 1e-3);
        assert!((zoom_fov(800.0, 1.6).to_degrees() - 18.0).abs() < 1e-3);
        assert!((min_pitch(2000.0) - 0.925).abs() < 1e-6);
        assert_eq!(min_pitch(900.0), 0.8);
    }
}
