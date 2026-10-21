//! The character-view behaviour: the figure a GUI preview shows turns its
//! body and head toward the mouse.
//!
//! Each tick the horizontal direction from the body's ground point to the
//! pointer's world point gives a yaw target at 0.7 of its angle (the
//! engine's arc-cosine yaw, the product taken in double), clamped to
//! ±1.5 rad. The gap to the body's yaw wraps by 2π (compared in double); a
//! gap of at most 1 rad moves the yaw by 5 % of itself, a larger one by
//! exactly 0.05 rad. The cursor direction is then built from the new yaw
//! by the engine's fast sine and cosine, with the stored pitch as its
//! height, and normalised. The head's target is the entity's stored aim
//! vector blended 40 % of the way toward that direction with its height
//! zeroed; the head's look eases 5 % toward the target a tick and is
//! normalised. The pointer's world point is the original's own cursor
//! point; Exarust intersects the mouse ray with the vertical plane through
//! the body (a guess at the preview camera's geometry, which the original
//! does not share). Where the stored aim vector comes from is not known:
//! it is an input of [`PreviewTurn`], the forward direction by default.
//!
//! Evidence: the original's one override, read from the instructions.

use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use exa_ai::combat::yaw;
use exa_assets::polyhedra::fast_sin_cos;

/// The share of the pointer direction's angle the body turns toward, a
/// double constant in the original.
pub const YAW_SHARE: f64 = 0.7;
/// The target yaw's clamp, radians.
pub const YAW_LIMIT: f32 = 1.5;
/// The clamped step of a large gap, radians a tick (a double constant).
pub const YAW_STEP: f64 = 0.05;
/// How much of a gap up to 1 rad a tick closes (a double constant).
pub const YAW_RATE: f64 = 0.05;
/// The share of the cursor direction the head's target takes.
pub const HEAD_BLEND: f32 = 0.4;
/// The head look's easing rate a tick.
pub const HEAD_RATE: f32 = 0.05;
/// 2π, single precision, as the original stores it.
const TWO_PI: f32 = f32::from_bits(0x40c9_0fdb);

/// A preview figure's turn: its body yaw, its pitch, the stored aim vector
/// the head blends from and the head's look direction.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct PreviewTurn {
    pub yaw: f32,
    pub pitch: f32,
    pub aim: Vec3,
    pub look: Vec3,
}

impl Default for PreviewTurn {
    fn default() -> Self {
        Self {
            yaw: 0.0,
            pitch: 0.0,
            aim: Vec3::Z,
            look: Vec3::Z,
        }
    }
}

/// Where the preview pointer is in the world, if anywhere.
#[derive(Resource, Debug, Clone, Copy, Default, PartialEq)]
pub struct PreviewPointer(pub Option<Vec3>);

/// The original's signum, as the engine's helper returns it: ±1 (with the
/// value's sign, a NaN's too) for a non-zero value, 0 for zero.
fn signum(value: f32) -> f32 {
    if value != 0.0 {
        1.0f32.copysign(value)
    } else {
        0.0
    }
}

/// `v` scaled to unit length, summed as the engine does (height first);
/// zero stays zero. The original uses the processor's approximate
/// reciprocal square root; this is exact.
fn normalised(v: Vec3) -> Vec3 {
    let len2 = v.y * v.y + (v.x * v.x + v.z * v.z);
    if len2 == 0.0 {
        return Vec3::ZERO;
    }
    v * (1.0 / len2.sqrt())
}

/// The direction the cursor points at, from a yaw and the stored pitch
/// (whose value is the direction's height before normalising).
pub fn look_direction(yaw: f32, pitch: f32) -> Vec3 {
    let (sin, cos) = fast_sin_cos(yaw);
    normalised(Vec3::new(sin, pitch, cos))
}

/// One tick of the behaviour: the yaw turns toward 0.7 of the pointer's
/// angle (clamped to ±1.5) by 5 % of the gap up to 1 rad, else by 0.05;
/// the head's look then eases toward the aim blended 40 % toward the
/// cursor direction (height zero), normalised.
pub fn tick(turn: &mut PreviewTurn, body: Vec3, pointer: Vec3) {
    let toward = normalised(Vec3::new(pointer.x - body.x, 0.0, pointer.z - body.z));
    let mut target = (f64::from(yaw(toward)) * YAW_SHARE) as f32;
    if YAW_LIMIT < target.abs() {
        target = signum(target) * YAW_LIMIT;
    }
    let mut gap = target - turn.yaw;
    if f64::from(gap) < -std::f64::consts::PI {
        gap += TWO_PI;
    } else if f64::from(gap) > std::f64::consts::PI {
        gap -= TWO_PI;
    }
    turn.yaw = if gap.abs() > 1.0 {
        (f64::from(turn.yaw) + f64::from(signum(gap)) * YAW_STEP) as f32
    } else {
        (f64::from(gap) * YAW_RATE + f64::from(turn.yaw)) as f32
    };
    let cursor = look_direction(turn.yaw, turn.pitch);
    let mut target = turn.aim + (cursor - turn.aim) * HEAD_BLEND;
    target.y = 0.0;
    turn.look = normalised(turn.look + (target - turn.look) * HEAD_RATE);
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

    /// A pointer straight ahead leaves the figure still; one to the side
    /// turns the body by the clamped step, and the head eases toward the
    /// aim blended toward the cursor direction with its height zeroed.
    #[test]
    fn the_figure_turns_toward_the_pointer() {
        let body = Vec3::ZERO;
        let mut turn = PreviewTurn::default();
        tick(&mut turn, body, Vec3::new(0.0, 0.0, 10.0));
        assert_eq!(turn.yaw, 0.0);
        assert_eq!(turn.look, Vec3::Z);

        // Straight to the side: the arc-cosine yaw is π/2, the target
        // 0.7·π/2 = 1.0996 rad, a gap above 1: a clamped 0.05 rad step.
        let mut turn = PreviewTurn::default();
        tick(&mut turn, body, Vec3::new(10.0, 0.0, 0.0));
        assert!((turn.yaw - 0.05).abs() < 1e-6, "{}", turn.yaw);
        let (sin, cos) = fast_sin_cos(turn.yaw);
        let cursor = normalised(Vec3::new(sin, 0.0, cos));
        let mut aim = Vec3::Z + (cursor - Vec3::Z) * 0.4;
        aim.y = 0.0;
        let head = normalised(Vec3::Z + (aim - Vec3::Z) * 0.05);
        assert!(turn.look.abs_diff_eq(head, 1e-6), "{turn:?} {head:?}");
    }

    /// The head blends from the stored aim vector, not from its own look;
    /// the pitch is the cursor direction's height, which the blend zeroes.
    #[test]
    fn the_head_blends_from_the_stored_aim() {
        let mut turn = PreviewTurn {
            aim: Vec3::X,
            pitch: 5.0,
            ..PreviewTurn::default()
        };
        tick(&mut turn, Vec3::ZERO, Vec3::new(0.0, 0.0, 10.0));
        // Cursor direction ahead: (0, 5, 1) normalised; the blend is
        // x = 0.6, z = 0.4·(1/√26), height zeroed, then 5 % toward it.
        let cursor_z = 1.0 / 26.0f32.sqrt();
        let target = Vec3::new(0.6, 0.0, 0.4 * cursor_z);
        let want = normalised(Vec3::Z + (target - Vec3::Z) * 0.05);
        assert!(
            turn.look.abs_diff_eq(want, 1e-6),
            "{:?} {want:?}",
            turn.look
        );
        assert_eq!(turn.look.y, 0.0);
    }

    /// A gap under 1 rad closes by 5 % of itself; the yaw target is 0.7 of
    /// the pointer's angle, and a pointer far to the side clamps.
    #[test]
    fn small_gaps_ease_and_large_targets_clamp() {
        let mut turn = PreviewTurn {
            yaw: 0.5,
            ..PreviewTurn::default()
        };
        let pointer = Vec3::new(0.5f32.tan() * 10.0, 0.0, 10.0);
        let target = (f64::from(yaw(normalised(pointer))) * 0.7) as f32;
        assert!((target - 0.35).abs() < 1e-3, "{target}");
        let gap = target - 0.5;
        tick(&mut turn, Vec3::ZERO, pointer);
        assert!((turn.yaw - (0.5 + gap * 0.05)).abs() < 1e-5, "{}", turn.yaw);
        // Behind and to the side: 0.7·(−3π/4) = −1.65 clamps to −1.5, a
        // gap above 1, so a 0.05 step the other way.
        let mut turn = PreviewTurn::default();
        tick(&mut turn, Vec3::ZERO, Vec3::new(-10.0, 0.0, -10.0));
        assert!((turn.yaw + 0.05).abs() < 1e-6, "{}", turn.yaw);
    }

    /// The signum is ±1 by sign bit for any non-zero value (a NaN too) and
    /// 0 for a zero of either sign; the cursor direction comes from the
    /// fast sine and cosine.
    #[test]
    fn the_helpers_follow_the_engines() {
        assert_eq!(signum(0.0), 0.0);
        assert_eq!(signum(-0.0), 0.0);
        assert_eq!(signum(-3.0), -1.0);
        assert_eq!(signum(f32::NAN), 1.0);
        assert_eq!(signum(-f32::NAN), -1.0);
        assert_eq!(look_direction(0.0, 0.0), Vec3::Z);
        let side = look_direction(std::f32::consts::FRAC_PI_2, 0.0);
        assert!(side.abs_diff_eq(Vec3::X, 1e-6), "{side:?}");
        assert_eq!(normalised(Vec3::ZERO), Vec3::ZERO);
    }
}
