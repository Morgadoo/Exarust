//! The character-view behaviour: the figure a GUI preview shows turns its
//! body and head toward the mouse.
//!
//! Each tick the horizontal direction from the body's ground point to the
//! pointer's world point gives a yaw target at 0.7 of its angle, clamped to
//! ±1.5 rad. The body's yaw eases toward it: a gap of at most 1 rad moves
//! by 5 % of itself, a larger one by exactly 0.05 rad (the step is
//! clamped). The cursor direction is then built from the new yaw and the
//! stored pitch; the head's look blends 40 % of the way toward it and then
//! eases a further 5 % a tick, normalised. The pointer's world point is the
//! original's own cursor point; Exarust intersects the mouse ray with the
//! vertical plane through the body (a guess at the preview camera's
//! geometry, which the original does not share).
//!
//! Evidence: the original's one override, read from the instructions.

use bevy::prelude::*;
use bevy::window::PrimaryWindow;

/// The share of the pointer direction's angle the body turns toward.
pub const YAW_SHARE: f32 = 0.7;
/// The target yaw's clamp, radians.
pub const YAW_LIMIT: f32 = 1.5;
/// The clamped step of a large gap, radians a tick.
pub const YAW_STEP: f32 = 0.05;
/// How much of a gap up to 1 rad a tick closes.
pub const YAW_RATE: f32 = 0.05;
/// The share of the cursor direction the head's look takes a tick.
pub const HEAD_BLEND: f32 = 0.4;
/// The head look's easing rate a tick.
pub const HEAD_RATE: f32 = 0.05;
/// π, single precision, as the original stores it.
const PI: f32 = 3.141_592_7;

/// A preview figure's turn: its body yaw, its pitch and the head's look
/// direction.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct PreviewTurn {
    pub yaw: f32,
    pub pitch: f32,
    pub look: Vec3,
}

impl Default for PreviewTurn {
    fn default() -> Self {
        Self {
            yaw: 0.0,
            pitch: 0.0,
            look: Vec3::Z,
        }
    }
}

/// Where the preview pointer is in the world, if anywhere.
#[derive(Resource, Debug, Clone, Copy, Default, PartialEq)]
pub struct PreviewPointer(pub Option<Vec3>);

/// The yaw of a direction in the horizontal plane: 0 along +Z, rising
/// toward +X, the convention of `Quat::from_rotation_y`.
pub fn yaw_of(direction: Vec3) -> f32 {
    direction.x.atan2(direction.z)
}

/// The original's signum, as the engine's helper returns it: ±1 for a
/// non-zero value, 0 for zero.
fn signum(value: f32) -> f32 {
    if value > 0.0 {
        1.0
    } else if value < 0.0 {
        -1.0
    } else {
        0.0
    }
}

/// The direction the cursor points at, from a yaw and the stored pitch
/// (whose value is the direction's height before normalising).
pub fn look_direction(yaw: f32, pitch: f32) -> Vec3 {
    Vec3::new(yaw.sin(), pitch, yaw.cos()).normalize_or_zero()
}

/// One tick of the behaviour: the yaw turns toward 0.7 of the pointer's
/// angle (clamped to ±1.5) by 5 % of the gap up to 1 rad, else by 0.05;
/// the head's look then blends 40 % toward the cursor direction and eases
/// 5 % toward that, normalised.
pub fn tick(turn: &mut PreviewTurn, body: Vec3, pointer: Vec3) {
    let mut target = yaw_of(pointer - body) * YAW_SHARE;
    if YAW_LIMIT < target.abs() {
        target = signum(target) * YAW_LIMIT;
    }
    let mut gap = target - turn.yaw;
    if gap < -PI {
        gap += 2.0 * PI;
    } else if PI < gap {
        gap -= 2.0 * PI;
    }
    if gap.abs() <= 1.0 {
        turn.yaw += gap * YAW_RATE;
    } else {
        turn.yaw += signum(gap) * YAW_STEP;
    }
    let cursor = look_direction(turn.yaw, turn.pitch);
    let toward = turn.look + (cursor - turn.look) * HEAD_BLEND;
    turn.look = (turn.look + (toward - turn.look) * HEAD_RATE).normalize_or_zero();
}

/// The mouse-ray point on the vertical plane through `body`, facing the
/// camera's horizontal forward direction, from the camera's vertical field
/// of view. A guess at how the preview turns the original's cursor point
/// into a world point.
pub fn pointer_point(
    camera: &Transform,
    fov: f32,
    body: Vec3,
    cursor: Vec2,
    width: f32,
    height: f32,
) -> Vec3 {
    let forward = camera.forward().as_vec3();
    let normal = Vec3::new(forward.x, 0.0, forward.z).normalize_or_zero();
    if normal == Vec3::ZERO {
        return body;
    }
    let half_height = (fov * 0.5).tan();
    let aspect = width / height.max(1.0);
    let offset_x = (cursor.x / width.max(1.0)) * 2.0 - 1.0;
    let offset_y = 1.0 - (cursor.y / height.max(1.0)) * 2.0;
    let right = camera.right().as_vec3();
    let up = camera.up().as_vec3();
    let aim = forward + right * (offset_x * aspect * half_height) + up * (offset_y * half_height);
    let denominator = normal.dot(aim);
    if denominator.abs() < 1e-6 {
        return body;
    }
    let t = normal.dot(body - camera.translation) / denominator;
    camera.translation + aim * t
}

/// The vertical field of view of a camera's projection, radians.
fn vertical_fov(projection: &Projection) -> f32 {
    match projection {
        Projection::Perspective(p) => p.fov,
        Projection::Orthographic(_) => std::f32::consts::FRAC_PI_4,
        _ => std::f32::consts::FRAC_PI_4,
    }
}

fn turn_previews(
    pointer: Res<PreviewPointer>,
    cameras: Query<(&GlobalTransform, &Projection), With<Camera3d>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut turns: Query<(&mut PreviewTurn, &mut Transform)>,
) {
    let window = windows.iter().next();
    let camera = cameras.iter().next();
    for (mut turn, mut transform) in &mut turns {
        let point = match pointer.0 {
            Some(point) => point,
            None => {
                let (Some(window), Some((at, projection))) = (window, camera) else {
                    continue;
                };
                let Some(cursor) = window.cursor_position() else {
                    continue;
                };
                let camera = Transform {
                    translation: at.translation(),
                    rotation: at.rotation(),
                    scale: at.scale(),
                };
                pointer_point(
                    &camera,
                    vertical_fov(projection),
                    transform.translation,
                    cursor,
                    window.width(),
                    window.height(),
                )
            }
        };
        tick(&mut turn, transform.translation, point);
        transform.rotation = Quat::from_rotation_y(turn.yaw);
    }
}

/// Turns the entities marked with [`PreviewTurn`] toward the mouse.
pub struct CharViewPlugin;

impl Plugin for CharViewPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PreviewPointer>()
            .add_systems(Update, turn_previews);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::FRAC_PI_2;

    /// A pointer to the side turns the body by 5 % of the gap, and the head
    /// takes 40 % of the cursor direction then 5 % more.
    #[test]
    fn the_figure_turns_toward_the_pointer() {
        let body = Vec3::ZERO;
        let pointer = Vec3::new(0.0, 0.0, 10.0);
        let mut turn = PreviewTurn::default();
        tick(&mut turn, body, pointer);
        assert_eq!(turn.yaw, 0.0);
        assert_eq!(turn.look, Vec3::Z);

        // Straight to the side: target 0.7·π/2 = 1.0996 rad, gap above 1:
        // a clamped 0.05 rad step.
        let mut turn = PreviewTurn::default();
        tick(&mut turn, body, Vec3::new(10.0, 0.0, 0.0));
        assert!((turn.yaw - 0.05).abs() < 1e-6, "{}", turn.yaw);
        // The cursor direction follows the yaw: (sin, 0, cos), and the head
        // is 0.4 then 0.05 of the way there, normalised.
        let cursor = Vec3::new(turn.yaw.sin(), 0.0, turn.yaw.cos());
        let want = (Vec3::Z + (cursor - Vec3::Z) * 0.4).normalize();
        let head = (Vec3::Z + (want - Vec3::Z) * 0.05).normalize();
        assert!(turn.look.abs_diff_eq(head, 1e-6), "{turn:?} {head:?}");
    }

    /// A gap under 1 rad closes by 5 % of itself; the yaw target is 0.7 of
    /// the pointer's angle, and a pointer behind clamps to ±1.5.
    #[test]
    fn small_gaps_ease_and_large_targets_clamp() {
        let mut turn = PreviewTurn {
            yaw: 0.5,
            ..PreviewTurn::default()
        };
        let pointer = Vec3::new(0.5f32.tan() * 10.0, 0.0, 10.0);
        let target = pointer.x.atan2(pointer.z) * 0.7;
        let gap = target - 0.5;
        tick(&mut turn, Vec3::ZERO, pointer);
        assert!((turn.yaw - (0.5 + gap * 0.05)).abs() < 1e-5, "{}", turn.yaw);
        // Behind and to the side: beyond the clamp in either direction.
        let mut turn = PreviewTurn::default();
        tick(&mut turn, Vec3::ZERO, Vec3::new(-10.0, 0.0, -10.0));
        assert!((turn.yaw + 0.05).abs() < 1e-6, "{}", turn.yaw);
        assert!((YAW_LIMIT - 1.5).abs() < 1e-9);
        assert_eq!(signum(0.0), 0.0);
        assert_eq!(look_direction(0.0, 0.0), Vec3::Z);
        let side = look_direction(FRAC_PI_2, 0.0);
        assert!(side.abs_diff_eq(Vec3::X, 1e-6), "{side:?}");
    }
}
