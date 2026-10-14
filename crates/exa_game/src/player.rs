//! The player character: a kinematic body with the human model, moved
//! relative to a chase camera, with walk/run/idle clips chosen by speed.

use bevy::input::mouse::{AccumulatedMouseMotion, AccumulatedMouseScroll};
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use exa_anim::Animator;
use exa_combat::{Attacking, Fighter};
use exa_physics::CharacterBody;

const WALK_SPEED: f32 = 1.4;
const RUN_SPEED: f32 = 3.6;
const TURN_RATE: f32 = 8.0;

/// The player's physics root (origin at the feet).
#[derive(Component)]
pub struct Player;

/// The visual body under the player root; turns to face the movement (or
/// the mouse cursor in combat mode).
#[derive(Component)]
pub struct PlayerBody {
    pub yaw: f32,
    pub combat: bool,
}

/// Moveset and idle used in combat mode with a one-handed weapon.
pub const COMBAT_MOVESET: &str = "One Handed";

#[derive(Component)]
pub struct ChaseCamera {
    pub yaw: f32,
    pub pitch: f32,
    pub distance: f32,
}

impl Default for ChaseCamera {
    fn default() -> Self {
        Self {
            yaw: 0.0,
            pitch: -0.75,
            distance: 4.5,
        }
    }
}

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (player_input, player_combat, chase_camera).chain());
    }
}

fn player_input(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    cameras: Query<&ChaseCamera>,
    mut players: Query<(&mut CharacterBody, &Children), With<Player>>,
    mut bodies: Query<(
        &mut PlayerBody,
        &mut Transform,
        &mut Animator,
        Option<&Fighter>,
    )>,
) {
    let yaw = cameras.iter().next().map_or(0.0, |c| c.yaw);
    let forward = Vec3::new(-yaw.sin(), 0.0, -yaw.cos());
    let right = Vec3::new(-forward.z, 0.0, forward.x);
    let mut dir = Vec3::ZERO;
    for (key, d) in [
        (KeyCode::KeyW, forward),
        (KeyCode::KeyS, -forward),
        (KeyCode::KeyA, -right),
        (KeyCode::KeyD, right),
    ] {
        if keys.pressed(key) {
            dir += d;
        }
    }
    let dir = dir.normalize_or_zero();
    let running = keys.pressed(KeyCode::ShiftLeft);
    let speed = if running { RUN_SPEED } else { WALK_SPEED };
    for (mut body, children) in &mut players {
        body.desired_velocity = dir * speed;
        for child in children.iter() {
            let Ok((mut pb, mut t, mut anim, fighter)) = bodies.get_mut(child) else {
                continue;
            };
            if fighter.is_some_and(|f| f.down) {
                body.desired_velocity = Vec3::ZERO;
                continue;
            }
            if pb.combat {
                // Facing is handled by the cursor in combat mode.
                let action = if dir == Vec3::ZERO {
                    "Cmbt - Idle"
                } else {
                    "Combat Walk"
                };
                if anim.looping && anim.action != action {
                    anim.play_loop(action);
                }
                body.desired_velocity = dir * WALK_SPEED;
                continue;
            }
            if dir != Vec3::ZERO {
                // The model faces +Z.
                let target = dir.x.atan2(dir.z);
                let delta = (target - pb.yaw + std::f32::consts::PI)
                    .rem_euclid(std::f32::consts::TAU)
                    - std::f32::consts::PI;
                pb.yaw += delta * (TURN_RATE * time.delta_secs()).min(1.0);
                t.rotation = Quat::from_rotation_y(pb.yaw);
            }
            let action = match (dir == Vec3::ZERO, running) {
                (true, _) => "Idle",
                (false, false) => "Walk",
                (false, true) => "Run",
            };
            if anim.looping && anim.action != action {
                anim.play_loop(action);
            }
        }
    }
}

fn chase_camera(
    buttons: Res<ButtonInput<MouseButton>>,
    motion: Res<AccumulatedMouseMotion>,
    scroll: Res<AccumulatedMouseScroll>,
    players: Query<&Transform, (With<Player>, Without<ChaseCamera>)>,
    mut cameras: Query<(&mut ChaseCamera, &mut Transform)>,
) {
    let Some(target) = players.iter().next() else {
        return;
    };
    for (mut cam, mut t) in &mut cameras {
        if buttons.pressed(MouseButton::Right) || buttons.pressed(MouseButton::Middle) {
            cam.yaw -= motion.delta.x * 0.004;
            cam.pitch = (cam.pitch - motion.delta.y * 0.004).clamp(-1.4, -0.1);
        }
        cam.distance = (cam.distance - scroll.delta.y * 0.4).clamp(1.5, 15.0);
        let focus = target.translation + Vec3::Y * 1.3;
        let rot = Quat::from_euler(EulerRot::YXZ, cam.yaw, cam.pitch, 0.0);
        t.translation = focus + rot * Vec3::new(0.0, 0.0, cam.distance);
        t.look_at(focus, Vec3::Y);
    }
}

/// Combat mode (F), facing the cursor, and attacks on the left button whose
/// direction follows the mouse's vertical motion.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn player_combat(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    buttons: Res<ButtonInput<MouseButton>>,
    motion: Res<AccumulatedMouseMotion>,
    windows: Query<&Window, With<PrimaryWindow>>,
    cameras: Query<(&Camera, &GlobalTransform), With<ChaseCamera>>,
    players: Query<&GlobalTransform, With<Player>>,
    mut bodies: Query<(
        Entity,
        &mut PlayerBody,
        &mut Transform,
        &mut Animator,
        Option<&Fighter>,
        Option<&Attacking>,
    )>,
) {
    for (entity, mut pb, mut t, mut anim, fighter, attacking) in &mut bodies {
        if fighter.is_some_and(|f| f.down) {
            continue;
        }
        if keys.just_pressed(KeyCode::KeyF) {
            pb.combat = !pb.combat;
            anim.moveset = if pb.combat {
                COMBAT_MOVESET.into()
            } else {
                "Default".into()
            };
            anim.play_loop(if pb.combat { "Cmbt - Idle" } else { "Idle" });
        }
        if !pb.combat {
            continue;
        }
        // Face the point under the cursor on the player's ground plane.
        if let (Ok(window), Ok((camera, cam_gt)), Some(player)) =
            (windows.single(), cameras.single(), players.iter().next())
            && let Some(cursor) = window.cursor_position()
            && let Ok(ray) = camera.viewport_to_world(cam_gt, cursor)
        {
            let ground = player.translation().y;
            if ray.direction.y.abs() > 1e-4 {
                let d = (ground - ray.origin.y) / ray.direction.y;
                if d > 0.0 {
                    let hit = ray.origin + *ray.direction * d;
                    let to = hit - player.translation();
                    if to.xz().length() > 0.2 {
                        pb.yaw = to.x.atan2(to.z);
                        t.rotation = Quat::from_rotation_y(pb.yaw);
                    }
                }
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
