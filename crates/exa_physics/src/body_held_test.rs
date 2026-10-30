//! A body held in the force environment (synthetic crate, not game data).
use super::*;
use crate::dynamics::Environment;
use bevy::math::Affine3A;

const NONE: &[[Vec3; 3]] = &[];

fn crate_at(at: Vec3) -> Ragdoll {
    Ragdoll::rigid(&[RigidPart {
        entity: Entity::PLACEHOLDER,
        parent: None,
        world: Affine3A::from_translation(at),
        rest: [
            Vec3::new(-0.2, 0.0, -0.2),
            Vec3::new(0.2, 0.0, -0.2),
            Vec3::new(0.0, 0.0, 0.2),
            Vec3::new(0.0, 0.4, 0.0),
        ],
        point_masses: [4.0; 4],
        parent_world: Affine3A::IDENTITY,
        mobility: 1.0,
        spheres: vec![(Vec3::new(0.0, 0.2, 0.0), 0.2)],
        origin: at,
        links: Vec::new(),
        mesh: None,
        triangles: None,
    }])
}

fn height(body: &Ragdoll) -> f32 {
    body.parts[0].transform().translation.y
}

/// Held, a body takes no gravity: it stays where it is; let go, it falls.
#[test]
fn a_held_body_floats_and_falls_once_released() {
    let mut body = crate_at(Vec3::new(0.0, 5.0, 0.0));
    body.hold_in(Environment::HELD);
    assert_eq!(body.held_environment(), Some(Environment::HELD));
    for _ in 0..40 {
        body.wake_to(WAKE_COUNT);
        body.tick(NONE);
    }
    assert!((height(&body) - 5.0).abs() < 1e-4, "{}", height(&body));
    body.release_held();
    for _ in 0..40 {
        body.tick(NONE);
    }
    assert!(height(&body) < 4.9, "{}", height(&body));
}

/// Held, a moving body is damped by the held bands: a fast part keeps
/// 0.125 of its speed a second, far less than the default's 0.426.
#[test]
fn a_held_body_loses_its_speed_by_the_held_bands() {
    let mut held = crate_at(Vec3::new(0.0, 5.0, 0.0));
    held.hold_in(Environment::HELD);
    // 2 cm a tick (166 cm/s) along x, every particle alike.
    for x in &mut held.parts[0].positions {
        *x += Vec3::new(0.02, 0.0, 0.0);
    }
    held.wake_to(WAKE_COUNT);
    let mut last = held.origin_at(0).unwrap();
    held.tick(NONE);
    let mut moved = held.origin_at(0).unwrap() - last;
    for _ in 0..3 {
        last = held.origin_at(0).unwrap();
        held.wake_to(WAKE_COUNT);
        held.tick(NONE);
        let next = held.origin_at(0).unwrap() - last;
        let keep = 0.125f32.powf(1.0 / 83.0);
        assert!((next.x - moved.x * keep).abs() < 1e-5, "{next} vs {moved}");
        assert!(next.y.abs() < 1e-6, "no gravity: {next}");
        moved = next;
    }
}

#[test]
fn the_held_environment_converts_for_the_simulation() {
    let step = Environment::HELD.configure_step(83);
    assert_eq!(step.gravity, Vec3::ZERO);
    assert!((step.retention[0] - 0.125f32.powf(1.0 / 83.0)).abs() < 1e-6);
    assert!((step.retention[1] - 0.0125f32.powf(1.0 / 83.0)).abs() < 1e-6);
    assert!((step.retention[2] - 0.025f32.powf(1.0 / 83.0)).abs() < 1e-6);
}
