//! A synthetic pressure plate (not game data): a slab held by three slider
//! links of 4 cm travel straight down, at the top of their rails at its
//! file position.
use super::*;
use crate::body::{RigidLink, RigidPart, world_link};
use bevy::math::Affine3A;

const NONE: &[[Vec3; 3]] = &[];
const TARGET: u32 = 0x8000_00e4;
const SECOND: u32 = 0x8000_013b;

fn slab(at: Vec3) -> RigidPart {
    let slider = |x: f32, z: f32| RigidLink {
        on_body: Vec3::new(x, 0.05, z),
        on_parent: Vec3::new(x, 0.05, z),
        slack: 0.04,
        strength: 0.0,
        smoothing: 0.01,
        flags: world_link::SLIDER,
        axis: Vec3::new(0.0, -0.04, 0.0),
    };
    let mut spheres = Vec::new();
    for i in 0..4 {
        for j in 0..4 {
            spheres.push((
                Vec3::new(-0.3 + 0.2 * i as f32, 0.05, -0.3 + 0.2 * j as f32),
                0.04,
            ));
        }
    }
    RigidPart {
        entity: Entity::PLACEHOLDER,
        parent: None,
        world: Affine3A::from_translation(at),
        rest: [
            Vec3::new(-0.4, 0.1, -0.4),
            Vec3::new(-0.4, -0.1, -0.4),
            Vec3::new(0.4, 0.0, -0.4),
            Vec3::new(0.0, 0.0, 0.4),
        ],
        point_masses: [4.0; 4],
        parent_world: Affine3A::IDENTITY,
        mobility: 0.45,
        spheres,
        origin: at,
        links: vec![slider(-0.2, -0.2), slider(0.2, -0.2), slider(0.0, 0.3)],
        mesh: None,
    }
}

fn plate_at(at: Vec3, kind: u8, flags: u16, saved: i32) -> (Ragdoll, PlateBody) {
    let mut body = Ragdoll::rigid(&[slab(at)]);
    let pending = PendingPlate {
        record: Record {
            kind,
            flags,
            target: TARGET,
            second: SECOND,
        },
        saved,
        id: 0x8000_0139,
    };
    let glue = PlateBody::new(&body, &pending).expect("a plate");
    let mobility = plate::mobility(body.parts[0].inverse_mass);
    body.parts[0].inverse_mass = mobility;
    body.parts[0].pairing.mobility = mobility;
    body.enter_mechanism_environment();
    (body, glue)
}

fn tick(body: &mut Ragdoll, glue: &mut PlateBody) -> Vec<(u32, i32)> {
    body.tick(NONE);
    if body.asleep() {
        return Vec::new();
    }
    let out = glue.tick(body);
    glue.signals(&out)
}

fn height(body: &Ragdoll) -> f32 {
    body.parts[0].transform().translation.y
}

#[test]
fn a_plate_reads_its_record_and_its_mobility() {
    let (body, glue) = plate_at(Vec3::ZERO, 1, plate::ONE_SHOT | plate::SECOND | 4, 0);
    assert_eq!(body.parts[0].inverse_mass, 0.03);
    assert!(body.in_mechanism_environment());
    assert!(glue.plate.one_shot && glue.plate.second && !glue.plate.invert);
    assert!(!glue.clickable());
    let (_, button) = plate_at(Vec3::ZERO, 2, 0, 1);
    assert!(button.clickable() && button.plate.invert && button.plate.down());
}

/// A plate nobody pushes does not creep and goes to sleep.
#[test]
fn a_plate_at_rest_stays_and_sleeps() {
    let (mut body, mut glue) = plate_at(Vec3::new(1.0, 0.0, 2.0), 1, 0, 0);
    body.wake_to(WAKE_COUNT);
    let start = height(&body);
    for _ in 0..600 {
        assert!(tick(&mut body, &mut glue).is_empty());
    }
    assert!((height(&body) - start).abs() < 1e-3);
    assert!(body.asleep());
}

/// A pushed plate goes down its rails, signals 1 to both its targets once
/// past 0.7, and 0 once let go and back under 0.5.
#[test]
fn a_pushed_plate_presses_and_releases() {
    let (mut body, mut glue) = plate_at(Vec3::ZERO, 1, plate::SECOND, 0);
    let start = height(&body);
    glue.push(&mut body, true);
    let mut sent = Vec::new();
    for _ in 0..120 {
        sent.extend(tick(&mut body, &mut glue));
    }
    assert_eq!(sent, vec![(TARGET, 1), (SECOND, 1)]);
    assert!(glue.plate.down());
    let sunk = start - height(&body);
    assert!(sunk > 0.025 && sunk < 0.045, "{sunk} m");
    glue.push(&mut body, false);
    sent.clear();
    for _ in 0..200 {
        sent.extend(tick(&mut body, &mut glue));
    }
    assert_eq!(sent, vec![(TARGET, 0), (SECOND, 0)]);
    assert!(!glue.plate.down());
    assert!((start - height(&body)).abs() < 0.01);
}

/// A weight holding the slab at the bottom of its rails presses it, as a
/// body standing on a pressure plate does.
#[test]
fn a_weight_presses_a_plate() {
    let (mut body, mut glue) = plate_at(Vec3::ZERO, 1, 0, 0);
    let rest = body.parts[0].positions;
    let mut sent = Vec::new();
    for _ in 0..80 {
        for i in 0..4 {
            body.parts[0].positions[i] = rest[i] - Vec3::Y * 0.04;
            body.parts[0].previous[i] = rest[i] - Vec3::Y * 0.04;
        }
        body.wake_to(WAKE_COUNT);
        sent.extend(tick(&mut body, &mut glue));
    }
    assert_eq!(sent, vec![(TARGET, 1)]);
}

/// An inverted plate signals −1 going down and nothing coming up.
#[test]
fn an_inverted_plate_signals_minus_one() {
    let (mut body, mut glue) = plate_at(Vec3::ZERO, plate::INVERTED_KIND, 0, 0);
    glue.push(&mut body, true);
    let mut sent = Vec::new();
    for _ in 0..120 {
        sent.extend(tick(&mut body, &mut glue));
    }
    glue.push(&mut body, false);
    for _ in 0..200 {
        sent.extend(tick(&mut body, &mut glue));
    }
    assert_eq!(sent, vec![(TARGET, -1)]);
}
