//! The world links' laws, pinned to the original's centimetres and
//! constants.

use bevy::math::Affine3A;

use super::*;
use crate::body::{LINK_ITERATION_LIMIT, Ragdoll, RigidPart};

/// A 1 m rest tetrahedron at the origin, as a rigid body with mobility 1.
fn tetra() -> Ragdoll {
    Ragdoll::rigid(&[RigidPart {
        entity: Entity::PLACEHOLDER,
        parent: None,
        world: Affine3A::IDENTITY,
        rest: [Vec3::ZERO, Vec3::X, Vec3::Y, Vec3::Z],
        point_masses: [4.0; 4],
        mobility: 1.0,
        spheres: vec![(Vec3::ZERO, 0.1)],
        origin: Vec3::ZERO,
        links: Vec::new(),
    }])
}

/// The correction a plain link applies, in metres.
fn plain_pull(anchor: Vec3, slack: f32, strength: f32, flags: u32) -> Vec3 {
    let mut body = tetra();
    let mut link = WorldLink::plain(0, &body.parts[0], Vec3::ZERO, anchor, Affine3A::IDENTITY);
    link.slack = slack;
    link.strength = strength;
    link.flags = flags;
    body.add_world_link(link).unwrap();
    let before = body.parts[0].point(&body.world_links[0].on_part);
    body.pull_world_links(0);
    body.parts[0].point(&body.world_links[0].on_part) - before
}

#[test]
fn a_plain_link_takes_the_excess_over_its_slack() {
    // 20 cm gap, 10 cm slack: half the gap, 10 cm.
    let pull = plain_pull(Vec3::X * 0.2, 0.1, 1.0, 0);
    assert!((pull - Vec3::X * 0.1).length() < 1e-5, "{pull}");
    // Inside the slack it does nothing and clears its state.
    let mut body = tetra();
    let mut link = WorldLink::plain(
        0,
        &body.parts[0],
        Vec3::ZERO,
        Vec3::X * 0.05,
        Affine3A::IDENTITY,
    );
    link.slack = 0.1;
    link.state = 7.0;
    body.add_world_link(link).unwrap();
    body.pull_world_links(0);
    assert_eq!(body.world_links[0].state, 0.0);
    assert_eq!(body.parts[0].positions[0], Vec3::ZERO);
}

#[test]
fn a_zero_slack_link_steps_at_most_ten_strength_centimetres() {
    // 30 cm away at strength 1: the 10 cm cap.
    let pull = plain_pull(Vec3::X * 0.3, 0.0, 1.0, 0);
    assert!((pull - Vec3::X * 0.1).length() < 1e-5, "{pull}");
    // Strength 2: beyond 20 cm the pull is the strength squared times
    // sqrt(100/d²): sqrt(100/900)·4·30 cm = 40 cm.
    let pull = plain_pull(Vec3::X * 0.3, 0.0, 2.0, 0);
    assert!((pull - Vec3::X * 0.4).length() < 1e-5, "{pull}");
    // Within 10·strength it is the gap times the strength.
    let pull = plain_pull(Vec3::X * 0.05, 0.0, 2.0, 0);
    assert!((pull - Vec3::X * 0.1).length() < 1e-5, "{pull}");
}

#[test]
fn a_plane_link_removes_eighty_percent_along_its_axis() {
    // The gap along x loses 80 %, leaving 2 cm of 10; a tiny y remains.
    let mut body = tetra();
    let mut link = WorldLink::plain(
        0,
        &body.parts[0],
        Vec3::ZERO,
        Vec3::new(0.1, 0.0, 0.0),
        Affine3A::IDENTITY,
    );
    link.flags = WORLD_PLANE;
    link.axis = Vec3::X;
    body.add_world_link(link).unwrap();
    let before = body.parts[0].point(&body.world_links[0].on_part);
    body.pull_world_links(0);
    let pull = body.parts[0].point(&body.world_links[0].on_part) - before;
    assert!((pull - Vec3::X * 0.02).length() < 1e-5, "{pull}");
}

#[test]
fn a_horizontal_drag_is_twice_the_strength_capped_at_three_root_mobility() {
    // Mobility 1: a 3 cm cap; a 10 cm gap pulls 6 cm across the ground.
    let mut body = tetra();
    let link = {
        let mut l = WorldLink::plain(
            0,
            &body.parts[0],
            Vec3::ZERO,
            Vec3::new(0.1, 5.0, 0.0),
            Affine3A::IDENTITY,
        );
        l.flags = WORLD_HORIZONTAL;
        l
    };
    body.add_world_link(link).unwrap();
    let before = body.parts[0].point(&body.world_links[0].on_part);
    body.pull_world_links(0);
    let pull = body.parts[0].point(&body.world_links[0].on_part) - before;
    assert!((pull - Vec3::X * 0.06).length() < 1e-5, "{pull}");
    assert!(pull.y.abs() < 1e-6, "{pull}");
    // Mobility 4 would cap at 6 cm, but 5 is the most: the factor is
    // 5/10·2, so a 10 cm gap pulls 10 cm.
    let mut body = tetra();
    body.parts[0].inverse_mass = 4.0;
    let mut l = WorldLink::plain(
        0,
        &body.parts[0],
        Vec3::ZERO,
        Vec3::X * 0.1,
        Affine3A::IDENTITY,
    );
    l.flags = WORLD_HORIZONTAL;
    body.add_world_link(l).unwrap();
    let before = body.parts[0].point(&body.world_links[0].on_part);
    body.pull_world_links(0);
    let pull = body.parts[0].point(&body.world_links[0].on_part) - before;
    assert!((pull - Vec3::X * 0.1).length() < 1e-5, "{pull}");
}

#[test]
fn approaching_motion_is_taken_off_a_drag_and_a_spring() {
    // Drag: 10 cm gap, 3 cm cap, 6 cm pull; 1 cm of approach leaves 5.
    let mut body = tetra();
    let mut link = WorldLink::plain(
        0,
        &body.parts[0],
        Vec3::ZERO,
        Vec3::X * 0.1,
        Affine3A::IDENTITY,
    );
    link.flags = WORLD_HORIZONTAL;
    body.add_world_link(link).unwrap();
    let before = body.parts[0].point(&body.world_links[0].on_part);
    // Pretend the point already moved 1 cm toward the anchor this tick.
    body.parts[0].start[0] = Vec3::X * -0.01;
    body.pull_world_links(0);
    let pull = body.parts[0].point(&body.world_links[0].on_part) - before;
    assert!((pull - Vec3::X * 0.05).length() < 1e-5, "{pull}");
}

#[test]
fn a_world_spring_is_a_tenth_of_the_strength_less_motion() {
    // 10 cm gap, strength 1: 1 cm; a 2 cm approach leaves -1 cm.
    let mut body = tetra();
    let mut link = WorldLink::plain(
        0,
        &body.parts[0],
        Vec3::ZERO,
        Vec3::X * 0.1,
        Affine3A::IDENTITY,
    );
    link.flags = WORLD_SOFT_WELD;
    body.add_world_link(link).unwrap();
    let before = body.parts[0].point(&body.world_links[0].on_part);
    body.pull_world_links(0);
    let pull = body.parts[0].point(&body.world_links[0].on_part) - before;
    assert!((pull - Vec3::X * 0.01).length() < 1e-5, "{pull}");

    let mut body = tetra();
    let mut link = WorldLink::plain(
        0,
        &body.parts[0],
        Vec3::ZERO,
        Vec3::X * 0.1,
        Affine3A::IDENTITY,
    );
    link.flags = WORLD_SOFT_WELD;
    body.add_world_link(link).unwrap();
    body.parts[0].start[0] = Vec3::X * -0.02;
    body.parts[0].positions[0] = Vec3::ZERO;
    body.pull_world_links(0);
    // The original normalises the already-scaled spring (0.1·gap), so the
    // 2 cm approach removes 0.02 cm and 0.4 of the motion removes 0.8 cm:
    // 1 − 0.02 − 0.8 = 0.18 cm.
    let moved = body.parts[0].point(&body.world_links[0].on_part);
    assert!((moved.x - 0.0018).abs() < 1e-6, "{moved}");
}

#[test]
fn a_slider_clamps_its_parameter_and_applies_twice() {
    // Travel 20 cm (2 cm stored slack is its square? no: travel² = 4 cm²
    // is 0.0004 m²). Start at 0.5, point at the anchor, 1 % smoothing.
    let mut body = tetra();
    let mut link = WorldLink::plain(
        0,
        &body.parts[0],
        Vec3::ZERO,
        Vec3::ZERO,
        Affine3A::IDENTITY,
    );
    link.flags = WORLD_SLIDER;
    link.axis = Vec3::X * 0.2;
    link.slack = 0.04;
    link.smoothing = 0.01;
    link.state = 0.5;
    body.add_world_link(link).unwrap();
    body.pull_world_links(0);
    let state = body.world_links[0].state;
    assert!((state - 0.005).abs() < 1e-7, "{state}");
    // q = p + 1 %·(target − p) = 0.1 cm; t = (20 cm·0.1 cm)/4 cm² = 0.005.
    // The correction is 0.1 cm, applied twice: 0.2 cm.
    assert!(
        (body.parts[0].positions[0].x - 0.002).abs() < 1e-6,
        "{}",
        body.parts[0].positions[0]
    );
}

#[test]
fn a_disabled_or_late_link_does_nothing() {
    let mut body = tetra();
    let mut link = WorldLink::plain(
        0,
        &body.parts[0],
        Vec3::ZERO,
        Vec3::X * 0.2,
        Affine3A::IDENTITY,
    );
    link.flags = LINK_DISABLED;
    body.add_world_link(link).unwrap();
    body.pull_world_links(0);
    assert_eq!(body.parts[0].positions[0], Vec3::ZERO);

    let mut body = tetra();
    let mut link = WorldLink::plain(
        0,
        &body.parts[0],
        Vec3::ZERO,
        Vec3::X * 0.2,
        Affine3A::IDENTITY,
    );
    link.flags = LINK_ITERATION_LIMIT;
    link.last_pass = Some(1);
    body.add_world_link(link).unwrap();
    body.pass = 2;
    body.pull_world_links(0);
    assert_eq!(body.parts[0].positions[0], Vec3::ZERO);
    body.pass = 1;
    body.pull_world_links(0);
    assert!(body.parts[0].positions[0].x > 0.0);
}

/// A link's point moves by its weights, unshared (no other body takes the
/// other half).
#[test]
fn a_world_link_moves_its_part_alone() {
    let mut body = tetra();
    let mut link = WorldLink::plain(
        0,
        &body.parts[0],
        Vec3::new(0.5, 0.0, 0.0),
        Vec3::X * 0.22,
        Affine3A::IDENTITY,
    );
    link.slack = 0.02;
    body.add_world_link(link).unwrap();
    let weights = body.world_links[0].on_part;
    let before: [Vec3; 4] = body.parts[0].positions;
    body.pull_world_links(0);
    // The point is 28 cm past the anchor, 2 cm of slack: 26 cm back,
    // spread by the weights.
    let moved = body.parts[0].point(&weights) - {
        let p = &before;
        (0..4).map(|i| p[i] * weights[i]).sum::<Vec3>()
    };
    let want = -0.26 * weights.iter().map(|w| w * w).sum::<f32>();
    assert!((moved.x - want).abs() < 1e-5, "{moved} vs {want}");
}
