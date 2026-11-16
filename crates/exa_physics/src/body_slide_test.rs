//! Synthetic sliding leaves (not game data): a 2 m panel held by three
//! slider links of 1.7 m travel, either down vertical rails (at their
//! bottom, travel fraction 1, shut) or along level rails like the shipped
//! portcullises' (placed at the open end, fraction 0). Each tick runs the
//! solver with the leaves' stage, so the door's step runs right after the
//! integration, as the original's does.
use super::*;
use crate::body::{RigidLink, RigidPart, solve_with, world_link};
use crate::door::{LOCKED, MOTORISED};
use bevy::math::Affine3A;

const NONE: &[[Vec3; 3]] = &[];
const TRAVEL: f32 = 1.7;

fn panel(at: Vec3, rail: Vec3) -> RigidPart {
    // Each anchor sits so that the body point is at fraction 1 along a
    // downward rail, or at fraction 0 along a level one.
    let slider = |x: f32, y: f32| {
        let on_body = Vec3::new(x, y, 0.0);
        let on_parent = if rail.y < 0.0 {
            on_body - rail
        } else {
            on_body
        };
        RigidLink {
            on_body,
            on_parent,
            slack: TRAVEL,
            strength: 0.0,
            smoothing: 0.01,
            flags: world_link::SLIDER,
            axis: rail,
        }
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

/// Down vertical rails.
const DOWN: Vec3 = Vec3::new(0.0, -TRAVEL, 0.0);
/// Along level rails, as the shipped portcullis's (its model's −x).
const LEVEL: Vec3 = Vec3::new(-TRAVEL, 0.0, 0.0);

fn slide_on(at: Vec3, rail: Vec3, flags: u32, saved: i32) -> (Ragdoll, SlideBody) {
    let mut body = Ragdoll::rigid(&[panel(at, rail)]);
    let pending = PendingSlide {
        record: Record {
            flags,
            ..Record::default()
        },
        saved,
        handle_height: 1.0,
    };
    let glue = SlideBody::new(&mut body, &pending).expect("a sliding leaf");
    (body, glue)
}

fn slide_at(at: Vec3, flags: u32) -> (Ragdoll, SlideBody) {
    slide_on(at, DOWN, flags, 0)
}

/// One tick of the solver with the leaves' stage: the door's step runs
/// right after the integration when the body is awake.
fn tick(body: &mut Ragdoll, glue: &mut SlideBody, n: i32) -> Vec<MechanismSounded> {
    if body.asleep() {
        body.sleep_tick();
        return Vec::new();
    }
    let key = body.parts[0].entity;
    let mut stages = SolverStages::default();
    stages.add(SlideStage::with(n * 12, vec![(key, glue.clone())]));
    solve_with(&mut [body], NONE, stages.all());
    let mut stepped = stages.get_mut::<SlideStage>().unwrap().take_stepped();
    let (_, stepped_glue, sounds) = stepped.pop().expect("the leaf stepped");
    *glue = stepped_glue;
    sounds
}

fn height(body: &Ragdoll) -> f32 {
    body.parts[0].transform().translation.y
}

fn along_x(body: &Ragdoll) -> f32 {
    body.parts[0].transform().translation.x
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

/// The step runs after the integration and before the passes: the jump an
/// open writes reaches the travel link before this tick's first pass, so
/// the leaf moves in the very tick that writes it, 1% of the gap (twice,
/// spread by the weights) and the inertia carries it on.
#[test]
fn an_open_writes_its_jump_into_the_travel_link_before_the_passes() {
    let (mut body, mut glue) = slide_at(Vec3::ZERO, 0);
    let start = height(&body);
    glue.door.open();
    assert_eq!(glue.door.slide.fraction, door::SLIDE_JUMP);
    body.wake_to(WAKE_COUNT);
    tick(&mut body, &mut glue, 0);
    assert!(height(&body) - start > 1e-4, "{} m", height(&body) - start);
    let mut risen: f32 = 0.0;
    for n in 1..20 {
        tick(&mut body, &mut glue, n);
        risen = risen.max(height(&body) - start);
    }
    assert!(risen > 0.002, "{risen} m");
    assert!(!glue.door.closed);
}

/// Down a vertical rail (no shipped rail is vertical) a motorised leaf's
/// crawl, 0.1 of a 1.7 cm gap in a tick's first pass, holds against the
/// default environment's gravity only a centimetre up.
#[test]
fn a_motorised_leaf_barely_lifts_up_a_vertical_rail() {
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

/// Along level rails, as the shipped portcullises lie, the leaf placed at
/// the open end (fraction 0) is open and stays there; a close command
/// drives it 0.075 a tick toward 1 at smoothing 0.05 until it reads shut,
/// slams once and latches, held at fraction 1 with smoothing 0.5.
#[test]
fn a_close_slides_a_level_leaf_shut_and_latches_it() {
    use crate::door::{LATCHES, MODE_TOGGLE};
    let saved = door::stored_word(MODE_TOGGLE, 200, false);
    let (mut body, mut glue) = slide_on(Vec3::ZERO, LEVEL, LATCHES, saved);
    assert!(glue.fraction(&body) < 1e-4 && !glue.door.closed);
    assert_eq!(glue.door.slide.mode, door::SlideMode::Close);
    body.wake_to(WAKE_COUNT);
    let start = along_x(&body);
    let mut slams = 0;
    let mut shut_at = None;
    for n in 0..1500 {
        let sounds = tick(&mut body, &mut glue, n);
        slams += sounds
            .iter()
            .filter(|s| matches!(s, MechanismSounded::Shot { set, .. } if set.ends_with("slm")))
            .count();
        if glue.door.closed && shut_at.is_none() {
            shut_at = Some(n);
        }
    }
    let travelled = start - along_x(&body);
    assert!(
        shut_at.is_some(),
        "travelled {travelled} m, fraction {}",
        glue.fraction(&body)
    );
    assert!((travelled - TRAVEL).abs() < 0.02, "{travelled} m");
    assert!(glue.door.closed && glue.door.latched);
    assert_eq!(slams, 1);
    // The step holds the shut leaf at fraction 1 with smoothing 0.5; the
    // passes then rewrite the fraction from where the leaf lies.
    let link = &body.world[glue.travel_link()];
    assert_eq!(link.smoothing, door::SLIDE_HELD_SMOOTH);
    assert!(link.length > 0.999, "{}", link.length);
}

/// A motorised open on a shut level leaf crawls the fraction down 0.01 a
/// tick; each tick of the open wakes the body (to 256), so the leaf keeps
/// moving toward its open end while the command runs.
#[test]
fn a_motorised_open_slides_a_level_leaf_open() {
    // Shut it first: a close saved in its word runs from the first tick.
    let saved = door::stored_word(door::MODE_TOGGLE, 200, false);
    let (mut body, mut glue) = slide_on(Vec3::ZERO, LEVEL, MOTORISED, saved);
    body.wake_to(WAKE_COUNT);
    for n in 0..1500 {
        tick(&mut body, &mut glue, n);
    }
    assert!(glue.door.closed, "fraction {}", glue.fraction(&body));
    let shut_x = along_x(&body);
    glue.door.open();
    assert_eq!(glue.door.slide.mode, door::SlideMode::Open);
    body.wake_to(WAKE_COUNT);
    let mut least_count = i32::MAX;
    for n in 1500..1800 {
        tick(&mut body, &mut glue, n);
        if glue.door.slide.mode == door::SlideMode::Open {
            least_count = least_count.min(body.countdown);
        }
    }
    assert!(!glue.door.closed);
    assert!(
        along_x(&body) - shut_x > 0.3,
        "{} m",
        along_x(&body) - shut_x
    );
    assert!(least_count >= WAKE_COUNT - 1, "{least_count}");
}

/// The hand's drag moves a level leaf along its rail: the press adds the
/// drag link at the hit, each frame's target is the link's point moved by
/// the cursor's offset along the leaf's x axis (its rail), kept within
/// 60 cm, and the release removes the link.
#[test]
fn the_hand_drags_a_level_leaf_along_its_rail() {
    let (mut body, mut glue) = slide_on(Vec3::ZERO, LEVEL, 0, 0);
    let links = body.world.len();
    let hit = body.parts[0].point(&weights(&body.parts[0], Vec3::new(0.0, 1.0, 0.0)));
    glue.take_hold(&mut body, hit);
    assert_eq!(body.world.len(), links + 1);
    assert!(glue.door.held);
    let start = along_x(&body);
    for n in 0..300 {
        // The cursor 2 m along the rail's closing side and 1 m across.
        let cursor = hit + Vec3::new(-2.0, 0.0, 1.0);
        glue.pull_to(&mut body, Some(cursor));
        tick(&mut body, &mut glue, n);
        let link = glue.hand_link(&body).unwrap();
        let point = body.parts[0].point(&link.on_part);
        // The anchor stays on the rail's line through the point, within
        // 60 cm of it.
        assert!((link.anchor - point).length() <= 0.6 + 1e-4);
        assert!((link.anchor.z - point.z).abs() < 1e-3);
    }
    let moved = start - along_x(&body);
    assert!(moved > 0.05, "{moved} m");
    glue.let_go(&mut body);
    assert_eq!(body.world.len(), links);
    assert!(!glue.door.held);
}
