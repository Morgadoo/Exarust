//! A synthetic sliding leaf (not game data): a 2 m panel held by three
//! slider links of 1.7 m travel pointing down, at the bottom of its rails
//! (travel fraction 1, shut) at its file position.
use super::*;
use crate::body::{RigidLink, RigidPart, world_link};
use crate::door::{LOCKED, MOTORISED};
use bevy::math::Affine3A;

const NONE: &[[Vec3; 3]] = &[];
const TRAVEL: f32 = 1.7;

fn panel(at: Vec3) -> RigidPart {
    // Each anchor sits a full travel above its body point.
    let slider = |x: f32, y: f32| RigidLink {
        on_body: Vec3::new(x, y, 0.0),
        on_parent: Vec3::new(x, y + TRAVEL, 0.0),
        slack: TRAVEL,
        strength: 0.0,
        smoothing: 0.01,
        flags: world_link::SLIDER,
        axis: Vec3::new(0.0, -TRAVEL, 0.0),
    };
    let mut spheres = Vec::new();
    for i in 0..6 {
        for j in 0..8 {
            spheres.push((
                Vec3::new(-0.75 + 0.3 * i as f32, 0.1 + 0.25 * j as f32, 0.0),
                0.06,
            ));
        }
    }
    RigidPart {
        entity: Entity::PLACEHOLDER,
        parent: None,
        world: Affine3A::from_translation(at),
        rest: [
            Vec3::new(0.0, 0.0, 0.1),
            Vec3::new(0.0, 0.0, -0.1),
            Vec3::new(1.0, 2.0, 0.0),
            Vec3::new(-1.0, 2.0, 0.0),
        ],
        point_masses: [4.0; 4],
        parent_world: Affine3A::IDENTITY,
        mobility: 0.05,
        spheres,
        origin: at,
        links: vec![slider(0.7, 0.2), slider(0.7, 1.9), slider(-0.7, 1.0)],
        mesh: None,
        triangles: None,
    }
}

fn slide_at(at: Vec3, flags: u32) -> (Ragdoll, SlideBody) {
    let mut body = Ragdoll::rigid(&[panel(at)]);
    let pending = PendingSlide {
        record: Record {
            flags,
            ..Record::default()
        },
        saved: 0,
        handle_height: 1.0,
    };
    let glue = SlideBody::new(&mut body, &pending).expect("a sliding leaf");
    (body, glue)
}

fn tick(body: &mut Ragdoll, glue: &mut SlideBody, n: i32) {
    body.tick(NONE);
    if !body.asleep() {
        glue.tick(body, n * 12);
    }
}

fn height(body: &Ragdoll) -> f32 {
    body.parts[0].transform().translation.y
}

/// The leaf's travel link is the one 30–160 cm up; at the bottom of its
/// rails it reads shut, and a locked leaf is not passable.
#[test]
fn a_sliding_leaf_at_the_end_of_its_rails_reads_shut() {
    let (body, glue) = slide_at(Vec3::ZERO, LOCKED);
    assert!(glue.door.shape.sliding);
    assert!((glue.fraction(&body) - 1.0).abs() < 1e-4);
    assert!(glue.door.closed && !glue.door.passable);
    // The travel link is the third (its body point is 100 cm up).
    assert_eq!(glue.travel, 2);
}

/// A shut leaf at rest stays where it is and goes to sleep.
#[test]
fn a_shut_leaf_stays_shut() {
    let (mut body, mut glue) = slide_at(Vec3::new(0.0, 0.0, 4.0), LOCKED);
    body.wake_to(WAKE_COUNT);
    let start = height(&body);
    for n in 0..600 {
        tick(&mut body, &mut glue, n);
    }
    assert!(
        (height(&body) - start).abs() < 2e-3,
        "{}",
        height(&body) - start
    );
    assert!(glue.door.closed);
    assert!(body.asleep());
}

/// A plain leaf's open jumps its fraction to 0.6, which is written into
/// the travel link before the tick reads it: the slider law's next pass
/// draws the leaf 0.01 of the way there, and then the link's fraction is
/// where the leaf lies (the law rewrites it each pass).
#[test]
fn an_open_writes_its_jump_into_the_travel_link() {
    let (mut body, mut glue) = slide_at(Vec3::ZERO, 0);
    let start = height(&body);
    glue.door.open();
    assert_eq!(glue.door.slide.fraction, door::SLIDE_JUMP);
    body.wake_to(WAKE_COUNT);
    // The jump is written at the first door tick, after that tick's
    // solver: the leaf moves from the second.
    let mut risen: f32 = 0.0;
    for n in 0..20 {
        tick(&mut body, &mut glue, n);
        risen = risen.max(height(&body) - start);
    }
    assert!(risen > 0.002, "{risen} m");
    assert!(!glue.door.closed);
}

/// A motorised leaf's open crawls its fraction 0.01 a tick toward 0, but
/// against the default environment's gravity the slider's pull (0.1 of a
/// 1.7 cm gap a pass) holds the leaf only a millimetre up vertical rails:
/// with the plain open's single shove, what keeps the sliding doors behind
/// the loader's flag.
#[test]
fn a_motorised_leaf_barely_lifts_against_gravity() {
    let (mut body, mut glue) = slide_at(Vec3::ZERO, MOTORISED);
    let start = height(&body);
    glue.door.open();
    body.wake_to(WAKE_COUNT);
    for n in 0..300 {
        tick(&mut body, &mut glue, n);
    }
    let risen = height(&body) - start;
    assert!(risen < 0.01, "{risen} m");
}
