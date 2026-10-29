//! The placement routines' arithmetic, pinned to the original's numbers.
//! (Included from body_placement.rs.)
use bevy::math::Affine3A;

use super::*;

/// A frame turned a quarter about y at (10, 100, -5).
fn frame() -> Affine3A {
    Affine3A::from_rotation_translation(
        Quat::from_rotation_y(std::f32::consts::FRAC_PI_2),
        Vec3::new(10.0, 100.0, -5.0),
    )
}

fn near(a: Vec3, b: Vec3) -> bool {
    (a - b).abs().max_element() < 1e-4
}

/// Every part is lowered by the pelvis's posed height, so the pelvis
/// lands at the frame's height, then put on the frame.
#[test]
fn the_pelvis_lands_at_the_frames_height() {
    let posed = [
        Affine3A::from_translation(Vec3::new(0.0, 98.0, 2.0)),
        Affine3A::from_translation(Vec3::new(10.0, 50.0, 0.0)),
    ];
    let placed = placed_parts(&posed, 0, frame());
    assert!(near(
        placed[0].translation.into(),
        Vec3::new(12.0, 100.0, -5.0)
    ));
    // (10, -48, 0) turned a quarter: x → -z.
    assert!(near(
        placed[1].translation.into(),
        Vec3::new(10.0, 52.0, -15.0)
    ));
    // A missing pelvis lowers nothing.
    let kept = placed_parts(&posed, 9, Affine3A::IDENTITY);
    assert_eq!(kept[1].translation.y, 50.0);
}

/// A foot anchor is the contact on the frame less the clip root, unrotated;
/// on the ground its height is the snap's ground plus the sole, at a
/// matrix the mapped height plus the sole less 8.
#[test]
fn foot_anchors_take_the_routines_heights() {
    let contact = Vec3::new(0.0, 4.0, 10.0);
    let root = Vec3::new(1.0, 95.0, 2.0);
    let on_frame = frame().transform_point3(contact) - root;
    let g = ground_foot_anchor(frame(), contact, root, 3.0, 4.5);
    assert!(near(g, Vec3::new(on_frame.x, 7.5, on_frame.z)));
    let m = matrix_foot_anchor(frame(), contact, root, 4.5);
    assert!(near(
        m,
        Vec3::new(on_frame.x, (on_frame.y + 4.5) - 8.0, on_frame.z)
    ));
    assert!(near(on_frame, Vec3::new(19.0, 9.0, -7.0)));
}

/// A matrix placement's pins sit 5 cm under their place on the frame.
#[test]
fn matrix_pins_sit_five_lower() {
    assert!(near(
        matrix_pin(frame(), Vec3::new(0.0, 40.0, 0.0)),
        Vec3::new(10.0, 135.0, -5.0)
    ));
}

/// The mean scales the sum by the float reciprocal of the count; with no
/// points it is the fallback.
#[test]
fn the_mean_scales_by_the_reciprocal() {
    let points = [
        Vec3::new(1.0, 2.0, 3.0),
        Vec3::new(2.0, 4.0, 6.0),
        Vec3::ONE,
    ];
    let third = 1.0f32 / 3.0;
    assert_eq!(
        mean_point(&points, Vec3::ZERO),
        Vec3::new(4.0 * third, 7.0 * third, 10.0 * third)
    );
    assert_eq!(mean_point(&[], Vec3::splat(7.0)), Vec3::splat(7.0));
}
