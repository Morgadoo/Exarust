//! The inverted-tetrahedron repair, the soft weld and the world links' laws,
//! pinned to the original's arithmetic (its centimetres worked by hand).

use bevy::math::Affine3A;

use super::*;
use crate::body::{Ragdoll, RigidLink, RigidPart, WorldLink, world_link};

fn rigid_part(world: Affine3A, origin: Vec3, links: Vec<RigidLink>) -> RigidPart {
    RigidPart {
        entity: Entity::PLACEHOLDER,
        parent: None,
        world,
        rest: [Vec3::ZERO, Vec3::X, Vec3::Y, Vec3::Z],
        point_masses: [4.0; 4],
        mobility: 1.0,
        parent_world: Affine3A::IDENTITY,
        spheres: vec![(Vec3::ZERO, 0.1)],
        origin,
        links,
        mesh: None,
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

/// A world link on the single part's first particle, anchored at `anchor`.
fn world_link_to(anchor: Vec3, slack: f32, strength: f32, flags: u32) -> WorldLink {
    WorldLink {
        part: 0,
        on_part: [1.0, 0.0, 0.0, 0.0],
        anchor,
        slack,
        strength,
        smoothing: 0.5,
        flags,
        axis: Vec3::ZERO,
        length: 0.0,
        origin: false,
        last_pass: None,
    }
}

/// The correction (m) a world link gives the single part at rest.
fn pull(mut link: WorldLink, body: &Ragdoll) -> Vec3 {
    link.correction(&body.parts[0]).unwrap_or(Vec3::ZERO)
}

#[test]
fn a_plain_world_link_takes_the_excess_over_its_slack() {
    let body = one_part();
    // 20 cm gap, 10 cm slack: half the gap, 10 cm, and the taut length.
    let mut link = world_link_to(Vec3::X * 0.2, 0.1, 1.0, 0);
    let c = link.correction(&body.parts[0]).unwrap();
    assert!((c - Vec3::X * 0.1).length() < 1e-6, "{c}");
    assert!((link.length - 0.2).abs() < 1e-6);
    // Inside its slack it does nothing and forgets the length.
    let mut link = world_link_to(Vec3::X * 0.05, 0.1, 1.0, 0);
    link.length = 0.3;
    assert_eq!(link.correction(&body.parts[0]), None);
    assert_eq!(link.length, 0.0);
}

#[test]
fn a_zero_slack_world_link_steps_at_most_ten_strength_centimetres() {
    let body = one_part();
    // 30 cm at strength 1: the pull is cut to 10 cm.
    let c = pull(world_link_to(Vec3::X * 0.3, 0.0, 1.0, 0), &body);
    assert!((c - Vec3::X * 0.1).length() < 1e-6, "{c}");
    // Strength 2 reaches 20 cm; beyond it the factor is s²·sqrt(100/d²):
    // 4·(10/30)·30 cm = 40 cm.
    let c = pull(world_link_to(Vec3::X * 0.3, 0.0, 2.0, 0), &body);
    assert!((c - Vec3::X * 0.4).length() < 1e-6, "{c}");
    // Within 10·strength it is the strength times the gap.
    let c = pull(world_link_to(Vec3::X * 0.05, 0.0, 2.0, 0), &body);
    assert!((c - Vec3::X * 0.1).length() < 1e-6, "{c}");
}

#[test]
fn a_plane_world_link_removes_eighty_percent_along_its_axis() {
    let body = one_part();
    let mut link = world_link_to(Vec3::X * 0.1, 0.0, 1.0, world_link::PLANE);
    link.axis = Vec3::X;
    // 10 cm along the axis loses 80 %, leaving 2 cm.
    let c = pull(link, &body);
    assert!((c - Vec3::X * 0.02).length() < 1e-6, "{c}");
}

#[test]
fn a_horizontal_drag_is_twice_the_strength_capped_by_the_root_mobility() {
    // Mobility 1: the step is capped at 3 cm of a 10 cm gap, 0.3 of it
    // twice: 6 cm, level with the ground.
    let body = one_part();
    let link = world_link_to(Vec3::new(0.1, 5.0, 0.0), 0.0, 1.0, world_link::HORIZONTAL);
    let c = pull(link, &body);
    assert!((c - Vec3::X * 0.06).length() < 1e-6, "{c}");
    // Mobility 4 would cap at 6 cm but 5 cm is the most: 0.5·2 of the gap.
    let mut body = one_part();
    body.parts[0].inverse_mass = 4.0;
    let link = world_link_to(Vec3::X * 0.1, 0.0, 1.0, world_link::HORIZONTAL);
    let c = pull(link, &body);
    assert!((c - Vec3::X * 0.1).length() < 1e-6, "{c}");
}

#[test]
fn motion_toward_the_anchor_is_taken_off_a_drag_and_a_spring() {
    // The point moved 1 cm toward the anchor this tick: the drag's 6 cm
    // pull is 5.
    let mut body = one_part();
    body.parts[0].start[0] = Vec3::X * -0.01;
    let link = world_link_to(Vec3::X * 0.1, 0.0, 1.0, world_link::HORIZONTAL);
    let c = pull(link, &body);
    assert!((c - Vec3::X * 0.05).length() < 1e-6, "{c}");
    // The spring pulls a tenth of the strength times the gap, 1 cm of 10.
    let body = one_part();
    let spring = world_link_to(Vec3::X * 0.1, 0.0, 1.0, world_link::SPRING);
    let c = pull(spring.clone(), &body);
    assert!((c - Vec3::X * 0.01).length() < 1e-6, "{c}");
    // With 2 cm of approach: the approach along the scaled pull's own
    // direction (0.1·2 = 0.2 cm) comes off, then 0.4 of the motion:
    // 1 − 0.02 − 0.8 = 0.18 cm.
    let mut body = one_part();
    body.parts[0].start[0] = Vec3::X * -0.02;
    let c = pull(spring, &body);
    assert!((c - Vec3::X * 0.0018).length() < 1e-6, "{c}");
}

#[test]
fn a_slider_projects_the_point_on_its_axis_and_applies_the_pull_twice() {
    let body = one_part();
    let mut link = world_link_to(Vec3::new(-0.002, -0.001, 0.0), 0.1, 1.0, world_link::SLIDER);
    link.axis = Vec3::X * 0.1;
    // The point (the origin) is drawn half the way to the fraction's point
    // (the anchor): q = (−1, −0.5) mm from the anchor's (−2, −1); the
    // fraction is 0.1·1 mm / 0.01 = 0.01, which puts the axis point at
    // (−1, −1) mm: the correction is (slid − q) + the half-step, (−1, −1) mm.
    let c = link.correction(&body.parts[0]).unwrap();
    assert!((c - Vec3::new(-0.001, -0.001, 0.0)).length() < 1e-7, "{c}");
    assert!((link.length - 0.01).abs() < 1e-7);
    // The iteration applies it twice: the point moves (−2, −2) mm.
    let mut body = Ragdoll::rigid(&[RigidPart {
        links: vec![RigidLink {
            on_body: Vec3::ZERO,
            on_parent: Vec3::new(-0.002, -0.001, 0.0),
            slack: 0.1,
            strength: 1.0,
            smoothing: 0.5,
            flags: world_link::SLIDER,
            axis: Vec3::X * 0.1,
        }],
        ..rigid_part(Affine3A::IDENTITY, Vec3::ZERO, Vec::new())
    }]);
    body.world[0].length = 0.0;
    // The link conversion gives a slider 0.01; this checks the pull at the
    // half smoothing worked above.
    body.world[0].smoothing = 0.5;
    let on_part = body.world[0].on_part;
    let before = body.parts[0].point(&on_part);
    body.pull_world_links(0);
    let moved = body.parts[0].point(&on_part) - before;
    assert!(
        (moved - Vec3::new(-0.002, -0.002, 0.0)).length() < 1e-6,
        "{moved}"
    );
}

#[test]
fn a_disabled_or_late_world_link_is_skipped_and_only_its_part_moves() {
    let mut body = Ragdoll::rigid(&[RigidPart {
        links: vec![RigidLink {
            on_body: Vec3::ZERO,
            on_parent: Vec3::X * 0.2,
            slack: 0.0,
            strength: 1.0,
            smoothing: 0.5,
            flags: world_link::ITERATION_LIMIT,
            axis: Vec3::ZERO,
        }],
        ..rigid_part(Affine3A::IDENTITY, Vec3::ZERO, Vec::new())
    }]);
    // Keep only the stored link: it runs while the pass is within its last.
    body.world.retain(|l| !l.origin);
    body.world[0].last_pass = Some(0);
    let before = body.parts[0].positions[0];
    body.pull_world_links(0);
    let first = body.parts[0].positions[0];
    assert!(first.x > before.x, "{first} vs {before}");
    // A later pass is past its last: no further pull.
    body.pass = 1;
    body.pull_world_links(0);
    assert_eq!(body.parts[0].positions[0], first);
    // A disabled link never runs.
    body.pass = 0;
    body.world[0].flags |= crate::body::LINK_DISABLED;
    body.pull_world_links(0);
    assert_eq!(body.parts[0].positions[0], first);
}

/// A limp ball of radius 6 cm at `at`, for the edge arithmetic.
fn ball(at: Vec3) -> Ragdoll {
    Ragdoll::new(&[PartSetup {
        bone: 11,
        entity: Entity::PLACEHOLDER,
        parent: None,
        world: Affine3A::from_translation(at),
        centre: Vec3::ZERO,
        half: Vec3::splat(0.06),
        origin: Vec3::ZERO,
        links: vec![],
        foot: None,
        pins: Vec::new(),
        spheres: None,
    }])
}

/// The edge restore takes its lengths with the squares summed x, then z,
/// then y (the original's own inline sum), which is not the order its other
/// distance helper uses: the vector here sums to 3.6789370 one way and
/// 3.6789367 the other.
#[test]
fn the_edge_restore_sums_its_squares_x_z_y() {
    let v = Vec3::new(0.9857491, 0.7262738, 1.4764017);
    let xz_y = ((v.x * v.x + v.z * v.z) + v.y * v.y).sqrt();
    let xy_z = ((v.x * v.x + v.y * v.y) + v.z * v.z).sqrt();
    assert_ne!(xz_y.to_bits(), xy_z.to_bits());
    assert_eq!(edge_length(v).to_bits(), xz_y.to_bits());
}

/// `keep_rigid` walks the six edges in order, moves both ends by half the
/// correction each (the first edge's correction, with the later edges only
/// adjusting it) and starts the strain afresh.
#[test]
fn the_edge_restore_moves_both_ends_half_way() {
    let mut b = ball(Vec3::ZERO);
    let p = &mut b.parts[0];
    // A rest tetrahedron and a stretched copy of it.
    p.rest = [
        Vec3::ZERO,
        Vec3::new(0.4, 0.0, 0.0),
        Vec3::new(0.0, 0.5, 0.0),
        Vec3::new(0.0, 0.0, 0.6),
    ];
    p.positions = p.rest;
    p.positions[1] = Vec3::new(0.8, 0.0, 0.0);
    p.strain = 0.7;
    let before: Vec<f32> = [(0, 1), (0, 2), (0, 3), (1, 2), (1, 3), (2, 3)]
        .iter()
        .map(|&(a, c)| edge_length(p.positions[c] - p.positions[a]))
        .collect();
    keep_rigid(p);
    assert_eq!(p.strain, 0.0);
    // The stretched edge is pulled to within a fifth of its rest length, and
    // the six edges' total error falls (one pass leaves some edges short).
    const EDGES: [(usize, usize); 6] = [(0, 1), (0, 2), (0, 3), (1, 2), (1, 3), (2, 3)];
    let len = edge_length(p.positions[1] - p.positions[0]);
    assert!((len - 0.4).abs() < 0.08, "{len}");
    let error = |p: &Part| {
        EDGES
            .iter()
            .map(|&(a, c)| {
                edge_length(p.positions[c] - p.positions[a]) - edge_length(p.rest[c] - p.rest[a])
            })
            .map(|e| e * e)
            .sum::<f32>()
    };
    let after = error(p);
    let mut was = 0.0;
    for (n, &(a, c)) in EDGES.iter().enumerate() {
        let rest = edge_length(p.rest[c] - p.rest[a]);
        was += (before[n] - rest) * (before[n] - rest);
    }
    assert!(after < was, "{after} vs {was}");
}

#[test]
fn a_clock_at_or_past_its_stamp_counts_down_from_one() {
    // 5 ms past the stamp with 2·0.5 ms of offset over a 12 ms tick:
    // 1 − (5 − 1)/12.
    let f = crate::body::clock_fraction(105, 100, 12, 2.0, 0.5);
    assert_eq!(f, 1.0 - (5.0_f32 - 1.0) / 12.0);
    assert_eq!(crate::body::clock_fraction(100, 100, 12, 0.0, 1.0), 1.0);
    // Far behind it is held at −2; a NaN offset passes the ordered clamp.
    assert_eq!(crate::body::clock_fraction(200, 100, 12, 0.0, 1.0), -2.0);
    assert!(crate::body::clock_fraction(200, 100, 12, f32::NAN, 1.0).is_nan());
}

#[test]
fn a_clock_before_its_stamp_counts_up_from_one_then_two() {
    // 3 ms ahead, within a 12 ms tick: (3 + 1)/12 + 1.
    let f = crate::body::clock_fraction(97, 100, 12, 2.0, 0.5);
    assert_eq!(f, (3.0_f32 + 1.0) / 12.0 + 1.0);
    // Exactly a tick ahead still counts the difference.
    assert_eq!(
        crate::body::clock_fraction(88, 100, 12, 0.0, 1.0),
        12.0 / 12.0 + 1.0
    );
    // Further ahead only the offset counts, past 2.
    assert_eq!(
        crate::body::clock_fraction(10, 100, 12, 3.0, 1.0),
        3.0_f32 / 12.0 + 2.0
    );
    // A clock that wrapped back under its stamp: the unsigned compare
    // sees the stamp far ahead.
    assert_eq!(crate::body::clock_fraction(1, u32::MAX, 12, 0.0, 1.0), 2.0);
}

#[test]
fn a_bodys_draw_fraction_adds_its_lag_in_ticks() {
    assert_eq!(crate::body::body_draw_fraction(10, 8, 0.25, false), 2.25);
    assert_eq!(crate::body::body_draw_fraction(8, 10, 0.25, false), -1.75);
    assert_eq!(crate::body::body_draw_fraction(10, 8, 0.25, true), 0.0);
}
