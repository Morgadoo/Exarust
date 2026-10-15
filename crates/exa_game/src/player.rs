//! The player character, steered as the original steers it, and the
//! original's chase camera. Rules: `docs/PLAYER.md`.
//!
//! Exploration uses tank controls: W walks along the character's heading,
//! A and D steer it (or turn a quarter in place when standing), S brakes
//! and backpedals, and the right mouse button walks toward the cursor. The
//! camera follows the character's position only: it never turns by itself,
//! the middle mouse button orbits it and the wheel zooms by field of view.
//! Combat mode (Tab) faces the cursor and steps relative to it.

use bevy::input::mouse::{AccumulatedMouseMotion, AccumulatedMouseScroll};
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use exa_anim::Animator;
use exa_combat::{Attacking, Fighter};
use exa_physics::body::Fallen;
use exa_physics::{CharacterBody, Physics};

use crate::torch::CarriesTorch;

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

/// The player's physics root (origin at the feet).
#[derive(Component)]
pub struct Player;

/// The visual body under the player root. The player is human and is
/// kept in underwear, as the original keeps its player's record.
#[derive(Component, Debug)]
#[require(exa_character::WearsUnderwear)]
pub struct PlayerBody {
    /// The body's yaw (rad; the model faces +Z).
    pub yaw: f32,
    /// The heading the body turns toward and walks along.
    pub heading: f32,
    pub combat: bool,
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
            run: 0.0,
            quarter_wait: 0.0,
            held: 0.0,
        }
    }
}

/// Moveset and idle used in combat mode with a one-handed weapon.
pub const COMBAT_MOVESET: &str = "One Handed";
/// The exploration moveset carrying a torch in the left hand.
pub const TORCH_MOVESET: &str = "1H & Torch";

/// The point under the cursor on the player's hip plane, for the right
/// mouse button.
#[derive(Resource, Default)]
struct CursorGround(Option<Vec3>);

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
    let Some(ray) = window
        .cursor_position()
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
        Has<CarriesTorch>,
    )>,
) {
    let dt = 1.0 / TICK_HZ;
    for (mut body, at, children) in &mut players {
        for child in children.iter() {
            let Ok((mut pb, mut t, mut anim, fighter, fallen, torch)) = bodies.get_mut(child)
            else {
                continue;
            };
            if fallen || fighter.is_some_and(|f| f.down) {
                body.desired_velocity = Vec3::ZERO;
                continue;
            }
            let feet = at.translation();
            let speed = body.desired_velocity.length();
            if pb.combat {
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
            let moveset = if torch { TORCH_MOVESET } else { "Default" };
            if anim.looping && (anim.action != action || anim.moveset != moveset) {
                anim.moveset = moveset.into();
                anim.play_loop(action);
            }
        }
    }
}

/// Combat footwork: W, S, A and D step along and across the cursor
/// direction (the body faces the cursor); held, they walk on.
fn combat_steps(
    keys: &ButtonInput<KeyCode>,
    body: &mut CharacterBody,
    pb: &mut PlayerBody,
    anim: &mut Animator,
    dt: f32,
) {
    let look = forward(pb.yaw);
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
    mut bodies: Query<(&mut PlayerBody, &mut Animator, Has<CarriesTorch>)>,
) {
    if !keys.just_pressed(KeyCode::Tab) {
        return;
    }
    for (mut pb, mut anim, torch) in &mut bodies {
        pb.combat = !pb.combat;
        anim.moveset = match (pb.combat, torch) {
            (true, true) => TORCH_MOVESET.into(),
            (true, false) => COMBAT_MOVESET.into(),
            (false, true) => TORCH_MOVESET.into(),
            (false, false) => "Default".into(),
        };
        anim.play_loop(if pb.combat { "Cmbt - Idle" } else { "Idle" });
    }
}

/// Combat mode (Tab): facing the cursor, attacks on the left button whose
/// direction follows the mouse's vertical motion.
#[allow(clippy::type_complexity)]
fn player_combat(
    mut commands: Commands,
    buttons: Res<ButtonInput<MouseButton>>,
    motion: Res<AccumulatedMouseMotion>,
    cursor: Res<CursorGround>,
    players: Query<&GlobalTransform, With<Player>>,
    mut bodies: Query<(
        Entity,
        &mut PlayerBody,
        &mut Transform,
        &mut Animator,
        Option<&Fighter>,
        Option<&Attacking>,
        Has<Fallen>,
    )>,
) {
    let Some(player) = players.iter().next() else {
        return;
    };
    for (entity, mut pb, mut t, mut anim, fighter, attacking, fallen) in &mut bodies {
        if fallen || fighter.is_some_and(|f| f.down) || !pb.combat {
            continue;
        }
        if let Some(c) = cursor.0 {
            let to = c - player.translation();
            if to.xz().length() > 0.2 {
                pb.yaw = yaw_of(to);
                pb.heading = pb.yaw;
                t.rotation = Quat::from_rotation_y(pb.yaw);
            }
        }
        if buttons.just_pressed(MouseButton::Left) && attacking.is_none() && anim.looping {
            let action = match motion.delta.y {
                y if y < -2.0 => "Attack Up",
                y if y > 2.0 => "Attack Down",
                _ => "Attack Mid",
            };
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

#[allow(clippy::type_complexity)]
fn chase_camera(
    time: Res<Time>,
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
    fn zoom_is_field_of_view() {
        assert!((zoom_fov(2000.0, 1.6).to_degrees() - 45.0).abs() < 1e-3);
        assert!((zoom_fov(800.0, 1.6).to_degrees() - 18.0).abs() < 1e-3);
        assert!((min_pitch(2000.0) - 0.925).abs() < 1e-6);
        assert_eq!(min_pitch(900.0), 0.8);
    }
}
