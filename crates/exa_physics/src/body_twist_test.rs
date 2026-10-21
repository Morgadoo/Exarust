//! The torso twist: the original's rules, each pinned to its numbers
//! (`docs/formats/RSQ.md`, Torso twist).
//! (Included from body_twist.rs: `#[path = "body_twist_test.rs"] mod tests;`)
use std::f32::consts::{FRAC_PI_2, PI};

use super::*;

fn close(a: f32, b: f32) -> bool {
    (a - b).abs() < 1e-5
}

/// A standing human at full tone, its frame facing +Z, looking `look`.
fn looking(look: f32) -> TwistBy {
    TwistBy {
        helpless: false,
        tone: 1.0,
        look,
        frame: 0.0,
        moving: false,
        attack: None,
        casting: false,
        crouch: 0.0,
        tilt: 0.0,
    }
}

/// The original's arc cosine: 1.046816 for 0.5 (the exact 1.047198),
/// 2.0947766 for −0.5, π/2, 0 and π at 0, 1 and −1. Its yaw of a
/// direction: 0.785179 for (1, 0, 1), negative for a negative x, 0 for a
/// vertical one.
#[test]
fn the_yaw_is_the_originals_arc_cosine() {
    assert!(close(acos_approx(0.5), 1.046_816));
    assert!(close(acos_approx(-0.5), 2.094_776_6));
    assert_eq!(acos_approx(0.0), FRAC_PI_2);
    assert_eq!(acos_approx(1.0), 0.0);
    assert!(close(acos_approx(-1.0), PI));
    assert!(close(heading_of(Vec3::new(1.0, 0.0, 1.0)), 0.785_179));
    assert!(close(heading_of(Vec3::new(-1.0, 0.0, 1.0)), -0.785_179));
    assert_eq!(heading_of(Vec3::Y), 0.0);
}

/// Standing, the twist's yaw follows the look 0.05 rad a tick (0.05 after
/// a tick toward a look 0.5 rad off, there after 11 at 0.500297, the
/// original's arc cosine's), 0.02 locomoting, the weapon's handling ×
/// 0.12 × the swing strength attacking (0.072 at 1.2 and 0.5); a look 2
/// rad off is held at 1 rad. Helpless, or at a tone of 0.85, the twist is
/// the identity and its yaw is kept.
#[test]
fn the_twist_follows_the_look() {
    let mut body = Locomotor::default();
    twist(&mut body, &looking(0.5));
    assert!(close(body.twist_yaw, 0.05));
    for _ in 0..10 {
        twist(&mut body, &looking(0.5));
    }
    assert!(close(body.twist_yaw, 0.500_296_7), "{}", body.twist_yaw);
    let step = |by: TwistBy| {
        let mut body = Locomotor::default();
        twist(&mut body, &by);
        body.twist_yaw
    };
    assert!(close(
        step(TwistBy {
            moving: true,
            ..looking(0.5)
        }),
        0.02
    ));
    assert!(close(
        step(TwistBy {
            attack: Some((1.2, 0.5)),
            ..looking(0.5)
        }),
        0.072
    ));
    let mut far = Locomotor::default();
    for _ in 0..40 {
        twist(&mut far, &looking(2.0));
    }
    assert_eq!(far.twist_yaw, 1.0);
    // The frame turns the look into its own: a frame at 0.5 looking at 0.5
    // has next to nothing to follow (the arc cosine's error near 1).
    let ahead = step(TwistBy {
        frame: 0.5,
        ..looking(0.5)
    });
    assert!(ahead.abs() < 1e-3, "{ahead}");
    for held in [
        TwistBy {
            helpless: true,
            ..looking(0.5)
        },
        TwistBy {
            tone: 0.85,
            ..looking(0.5)
        },
    ] {
        let mut body = far;
        twist(&mut body, &held);
        assert_eq!((body.twist, body.twist_yaw), (Quat::IDENTITY, 1.0));
    }
}

/// The rotation: forward along the twist's direction, its up square to
/// it. Level, a turn about the vertical by the twist's yaw; crouched fully
/// the direction dips to (0, −0.6, 0.8) (−1 normalised, × 0.75,
/// normalised), casting to (0, −0.57735, 0.8165) (−y² instead);
/// attacking crouched to (0, −0.21951, 0.97561) (0.3 of the crouch).
#[test]
fn the_twist_looks_along_its_direction() {
    let mut body = Locomotor::default();
    for _ in 0..10 {
        twist(&mut body, &looking(1.0));
    }
    let level = body.twist * Vec3::Z;
    assert!((level - Vec3::new(0.5f32.sin(), 0.0, 0.5f32.cos())).length() < 1e-5);
    assert!((body.twist * Vec3::Y - Vec3::Y).length() < 1e-5);
    let dipped = |by: TwistBy| {
        let mut body = Locomotor::default();
        twist(&mut body, &by);
        (body.twist * Vec3::Z, body.twist * Vec3::Y)
    };
    let (z, y) = dipped(TwistBy {
        crouch: 1.0,
        ..looking(0.0)
    });
    assert!((z - Vec3::new(0.0, -0.6, 0.8)).length() < 1e-5, "{z}");
    assert!((y - Vec3::new(0.0, 0.8, 0.6)).length() < 1e-5, "{y}");
    let (z, _) = dipped(TwistBy {
        crouch: 1.0,
        casting: true,
        ..looking(0.0)
    });
    assert!(
        (z - Vec3::new(0.0, -0.577_350_3, 0.816_496_6)).length() < 1e-5,
        "{z}"
    );
    let (z, _) = dipped(TwistBy {
        crouch: 1.0,
        attack: Some((1.0, 1.0)),
        ..looking(0.0)
    });
    assert!(
        (z - Vec3::new(0.0, -0.219_512_2, 0.975_609_8)).length() < 1e-5,
        "{z}"
    );
}

/// The spine: the thorax turns by the whole twist and the lumbar (under
/// the root) by half of a turn about the vertical; their positions keep
/// the pose; the identity changes nothing.
#[test]
fn the_spine_takes_the_twist_the_lumbar_half() {
    let at = |x: f32| Affine3A::from_translation(Vec3::new(x, 1.0, 0.0));
    let mut parts = vec![at(0.0), at(0.1), at(0.2)];
    let twist = Quat::from_rotation_y(0.6);
    twist_spine(&mut parts, &[(1, true), (2, false)], twist);
    let turn = |p: &Affine3A| Quat::from_mat3a(&p.matrix3);
    assert_eq!(parts[0], at(0.0));
    assert!(turn(&parts[1]).angle_between(Quat::from_rotation_y(0.3)) < 1e-5);
    assert!(turn(&parts[2]).angle_between(twist) < 1e-5);
    assert_eq!(parts[1].translation, at(0.1).translation);
    let mut kept = vec![at(0.0), at(0.1)];
    twist_spine(&mut kept, &[(1, true)], Quat::IDENTITY);
    assert_eq!(kept, vec![at(0.0), at(0.1)]);
}
