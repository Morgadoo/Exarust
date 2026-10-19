//! How far a drag pulls, and when the grabbed box is still allowed.
//!
//! The pull starts 1000 cm along the cursor direction. Each accepted hit
//! closer than that replaces it, so a hit at 1000 cm or beyond leaves the
//! reach where it is. The box is refused when any corner, after it is
//! placed, is higher than 280 cm or lower than 1000 cm. A corner exactly
//! on either limit is kept.
//!
//! Evidence: the drag reach and the box gate. The level polygons that
//! produce a hit are not in this slice.

use bevy::math::{Quat, Vec3};

/// How far a drag looks, in centimetres.
pub const DRAG_REACH_CM: f32 = 1000.0;
/// A corner higher than this refuses the drag, in centimetres.
pub const DRAG_ABOVE_CM: f32 = 280.0;
/// A corner lower than this refuses the drag, in centimetres.
pub const DRAG_BELOW_CM: f32 = -1000.0;

/// The point the hand pulls toward. `direction` is multiplied by the
/// nearest accepted hit, which starts at [`DRAG_REACH_CM`].
pub fn drag_point(direction: Vec3, hits_cm: &[f32]) -> Vec3 {
    let mut reach = DRAG_REACH_CM;
    for &hit in hits_cm {
        if hit.is_finite() && hit < reach {
            reach = hit;
        }
    }
    direction * reach
}

/// Whether every corner of the box stays between the drag's height limits.
pub fn drag_box_holds(center: Vec3, half: Vec3, rotation: Quat, translation: Vec3) -> bool {
    for sx in [-1.0_f32, 1.0] {
        for sy in [-1.0_f32, 1.0] {
            for sz in [-1.0_f32, 1.0] {
                let corner = center + Vec3::new(sx * half.x, sy * half.y, sz * half.z);
                let y = (rotation * corner + translation).y;
                if y > DRAG_ABOVE_CM || y < DRAG_BELOW_CM {
                    return false;
                }
            }
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_near_hit_shortens_the_pull_and_a_high_box_is_refused() {
        let pull = drag_point(Vec3::X, &[1200.0, 400.0, f32::NAN]);
        assert!(
            pull.abs_diff_eq(Vec3::new(400.0, 0.0, 0.0), 1e-3),
            "{pull:?}"
        );
        let full = drag_point(Vec3::X, &[1500.0]);
        assert!(full.abs_diff_eq(Vec3::new(1000.0, 0.0, 0.0), 1e-3));

        let held = drag_box_holds(
            Vec3::new(0.0, 270.0, 0.0),
            Vec3::splat(10.0),
            Quat::IDENTITY,
            Vec3::ZERO,
        );
        let high = drag_box_holds(
            Vec3::new(0.0, 270.0, 0.0),
            Vec3::new(10.0, 20.0, 10.0),
            Quat::IDENTITY,
            Vec3::ZERO,
        );
        assert!(held);
        assert!(!high);

        let low_held = drag_box_holds(
            Vec3::new(0.0, -990.0, 0.0),
            Vec3::splat(10.0),
            Quat::IDENTITY,
            Vec3::ZERO,
        );
        let low = drag_box_holds(
            Vec3::new(0.0, -990.0, 0.0),
            Vec3::new(10.0, 20.0, 10.0),
            Quat::IDENTITY,
            Vec3::ZERO,
        );
        assert!(low_held);
        assert!(!low);

        let lifted = drag_box_holds(
            Vec3::ZERO,
            Vec3::splat(10.0),
            Quat::IDENTITY,
            Vec3::new(0.0, 300.0, 0.0),
        );
        assert!(!lifted);
    }
}
