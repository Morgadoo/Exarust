//! The inverted-tetrahedron repair and the other plain-link kinds, pinned to
//! the original's arithmetic.

use bevy::math::Affine3A;

use super::*;
use crate::body::{LINK_BREAKABLE, Ragdoll, RigidLink, RigidPart};

fn rigid_part(world: Affine3A, origin: Vec3, links: Vec<RigidLink>) -> RigidPart {
    RigidPart {
        entity: Entity::PLACEHOLDER,
        parent: None,
        world,
        rest: [Vec3::ZERO, Vec3::X, Vec3::Y, Vec3::Z],
        point_masses: [4.0; 4],
        mobility: 1.0,
        spheres: vec![(Vec3::ZERO, 0.1)],
        origin,
        links,
    }
}

fn one_part() -> Ragdoll {
    Ragdoll::rigid(&[rigid_part(Affine3A::IDENTITY, Vec3::ZERO, Vec::new())])
}

fn orientation(p: &[Vec3; 4]) -> f32 {
    (p[1] - p[0]).cross(p[2] - p[0]).dot(p[3] - p[0])
}

#[test]
fn an_inverted_tetrahedron_is_rebuilt_on_its_rest_tetrahedron() {
    let mut body = one_part();
    let rest = body.parts[0].rest;
    // Mirror the fourth particle through the origin: the tetrahedron is
    // inside out, the edge midpoints' vector leans along the cross of the
    // opposite edges.
    body.parts[0].positions = [rest[0], rest[1], rest[2], -rest[3]];
    body.parts[0].previous = body.parts[0].positions;
    body.parts[0].start = body.parts[0].positions;
    assert!(orientation(&body.parts[0].positions) < 0.0);
    body.repair_inverted();
    let p = &body.parts[0];
    assert!(orientation(&p.positions) > 0.0, "{:?}", p.positions);
    // The six edges keep the rest lengths.
    for (a, b) in [(0, 1), (0, 2), (0, 3), (1, 2), (1, 3), (2, 3)] {
        let want = (p.rest[b] - p.rest[a]).length();
        let now = (p.positions[b] - p.positions[a]).length();
        assert!((now - want).abs() < 1e-4, "edge {a}{b}: {now} vs {want}");
    }
    // The original rebuilds on the rest tetrahedron: particle 0 stays at
    // its centre's displacement from the tick's start (none here).
    assert!(p.positions[0].length() < 1e-5, "{:?}", p.positions[0]);
}

#[test]
fn drawing_interpolates_the_particles_between_the_last_two_ticks() {
    let mut body = one_part();
    let from = body.parts[0].positions;
    let mut to = from;
    to[0] += Vec3::new(0.4, 0.0, 0.0);
    to[1] += Vec3::new(0.0, 0.2, 0.0);
    body.parts[0].drawn_particles = (from, to);
    let drawn = body.parts[0].drawn(0.25);
    for i in 0..4 {
        let want = from[i].lerp(to[i], 0.25);
        let got = drawn.transform_point3(body.parts[0].rest[i]);
        assert!(got.abs_diff_eq(want, 1e-5), "{got} vs {want}");
    }
}

#[test]
fn a_right_way_tetrahedron_is_left_alone() {
    let mut body = one_part();
    let before = body.parts[0].positions;
    body.parts[0].start = before;
    body.repair_inverted();
    assert_eq!(body.parts[0].positions, before);
}

#[test]
fn a_soft_weld_is_a_quarter_of_its_strength_less_four_tenths_of_the_motion() {
    let link = RigidLink {
        on_body: Vec3::ZERO,
        on_parent: Vec3::ZERO,
        slack: 0.2,
        strength: 2.0,
        smoothing: 0.5,
        flags: crate::body::LINK_SOFT_WELD,
        axis: Vec3::ZERO,
    };
    let body = Ragdoll::rigid(&[
        rigid_part(Affine3A::IDENTITY, Vec3::ZERO, Vec::new()),
        RigidPart {
            parent: Some(0),
            ..rigid_part(
                Affine3A::from_translation(Vec3::X * 0.6),
                Vec3::X * 0.5,
                vec![link],
            )
        },
    ]);
    let l = &body.links[0];
    // 10 cm gap, 2 cm of motion: 2·0.25·10 = 5 cm, less 0.4·2 = 0.8 cm.
    let correction = l
        .welded(Vec3::new(0.1, 0.0, 0.0), Vec3::new(0.02, 0.0, 0.0))
        .unwrap();
    assert!(
        (correction - Vec3::new(0.042, 0.0, 0.0)).length() < 1e-6,
        "{correction}"
    );
    // Under the general epsilon it does nothing.
    assert_eq!(l.welded(Vec3::new(1e-7, 0.0, 0.0), Vec3::ZERO), None);
}

#[test]
fn the_iteration_uses_a_soft_weld_link() {
    let link = RigidLink {
        on_body: Vec3::splat(0.25),
        on_parent: Vec3::ZERO,
        slack: 0.2,
        strength: 1.0,
        smoothing: 0.5,
        flags: crate::body::LINK_SOFT_WELD,
        axis: Vec3::ZERO,
    };
    let mut body = Ragdoll::rigid(&[
        rigid_part(Affine3A::IDENTITY, Vec3::ZERO, Vec::new()),
        RigidPart {
            parent: Some(0),
            ..rigid_part(
                Affine3A::from_translation(Vec3::X * 0.6),
                Vec3::X * 0.5,
                vec![link],
            )
        },
    ]);
    // The origin pivot would take most of the gap: disable it, leaving the
    // soft weld's own law.
    body.links[1].flags |= crate::body::LINK_DISABLED;
    let before = body.parts[1].positions[0].x;
    body.iterate();
    // The child's centroid is 35 cm from the parent's point (0.85 − 0.5):
    // 0.25·1·(−35) = −8.75 cm, a tenth −0.875 cm, shared half to the child
    // −0.4375 cm; each particle takes a quarter of it as its weight.
    assert!(
        (body.parts[1].positions[0].x - (before - 0.00109375)).abs() < 1e-6,
        "{} vs {}",
        body.parts[1].positions[0].x,
        before - 0.00109375
    );
}

#[test]
fn resetting_link_lengths_takes_the_current_distance_within_the_slack() {
    let link = RigidLink {
        on_body: Vec3::ZERO,
        on_parent: Vec3::ZERO,
        slack: 0.2,
        strength: 1.0,
        smoothing: 0.5,
        flags: 0,
        axis: Vec3::ZERO,
    };
    let mut body = Ragdoll::rigid(&[
        rigid_part(Affine3A::IDENTITY, Vec3::ZERO, Vec::new()),
        RigidPart {
            parent: Some(0),
            ..rigid_part(
                Affine3A::from_translation(Vec3::X * 0.25),
                Vec3::ZERO,
                vec![link],
            )
        },
    ]);
    body.links[0].length = 0.0;
    body.reset_link_lengths();
    // 25 cm apart, 20 cm of slack: the length is the slack.
    assert!((body.links[0].length - 0.2).abs() < 1e-6);
    // 10 cm apart: the current distance.
    let shift = Vec3::X * -0.15;
    let p = &mut body.parts[1];
    for x in p
        .positions
        .iter_mut()
        .chain(p.previous.iter_mut())
        .chain(p.start.iter_mut())
    {
        *x += shift;
    }
    body.reset_link_lengths();
    assert!((body.links[0].length - 0.1).abs() < 1e-6);
}

#[test]
fn a_breakable_link_spends_its_strain_and_lets_go_below_zero() {
    let link = RigidLink {
        on_body: Vec3::ZERO,
        on_parent: Vec3::ZERO,
        slack: 0.0,
        strength: 1.0,
        smoothing: 0.0,
        flags: LINK_BREAKABLE,
        axis: Vec3::X,
    };
    let mut body = Ragdoll::rigid(&[
        rigid_part(Affine3A::IDENTITY, Vec3::ZERO, Vec::new()),
        RigidPart {
            parent: Some(0),
            ..rigid_part(
                Affine3A::from_translation(Vec3::X * 0.015),
                Vec3::ZERO,
                vec![link],
            )
        },
    ]);
    body.links[0].state = 1.0;
    body.iterate();
    // The child's point is 1.5 cm from the parent's, q = 1 − 0.015 > 0,
    // so the strain falls by 1 + 2q/1.5 > 2 and the link breaks.
    assert!(body.links[0].flags & crate::body::LINK_DISABLED != 0);
    // With the axis opposed the strain falls by one only and stays.
    let link = RigidLink {
        on_body: Vec3::ZERO,
        on_parent: Vec3::ZERO,
        slack: 0.0,
        strength: 1.0,
        smoothing: 0.0,
        flags: LINK_BREAKABLE,
        axis: Vec3::X,
    };
    let mut body = Ragdoll::rigid(&[
        rigid_part(Affine3A::IDENTITY, Vec3::ZERO, Vec::new()),
        RigidPart {
            parent: Some(0),
            ..rigid_part(
                Affine3A::from_translation(Vec3::X * 2.0),
                Vec3::ZERO,
                vec![link],
            )
        },
    ]);
    body.links[0].state = 1.0;
    body.iterate();
    assert_eq!(body.links[0].state, 0.0);
    assert!(body.links[0].flags & crate::body::LINK_DISABLED == 0);
}
