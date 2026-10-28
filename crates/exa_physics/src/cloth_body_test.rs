//! The cloth's body stage: triangle bounds, the closest point, the push
//! buffer, the push pass and the countdown trade.
//! (Included from cloth_body.rs: `#[cfg(test)] #[path = "cloth_body_test.rs"] mod tests;`)
use super::*;
use crate::cloth::{Particle, gravity_cm, step};

fn still(at: Vec3) -> Particle {
    Particle {
        position: at,
        previous: at,
    }
}

/// The right-angled triangle's sphere sits on its hypotenuse; an acute
/// one's is the circumscribed sphere.
#[test]
fn a_triangles_sphere_is_its_smallest() {
    let (centre, r) = triangle_sphere(
        Vec3::ZERO,
        Vec3::new(6.0, 0.0, 0.0),
        Vec3::new(0.0, 8.0, 0.0),
    );
    assert_eq!((centre, r), (Vec3::new(3.0, 4.0, 0.0), 5.0));
    // Longest edge 1-2 (29 against 4 + 25): on that edge.
    let (centre, r) = triangle_sphere(
        Vec3::ZERO,
        Vec3::new(2.0, 0.0, 0.0),
        Vec3::new(0.0, 5.0, 0.0),
    );
    assert_eq!(centre, Vec3::new(1.0, 2.5, 0.0));
    assert!((r - (29.0f32).sqrt() * 0.5).abs() < 1e-6);
    // Longest edge 0-2 (16 against 2 + 10): on that edge.
    let (centre, r) = triangle_sphere(
        Vec3::ZERO,
        Vec3::new(1.0, 1.0, 0.0),
        Vec3::new(4.0, 0.0, 0.0),
    );
    assert_eq!((centre, r), (Vec3::new(2.0, 0.0, 0.0), 2.0));
    // Acute: the circumcentre (1, 0.75), radius 1.25, through the
    // approximate reciprocal.
    let (centre, r) = triangle_sphere(
        Vec3::ZERO,
        Vec3::new(2.0, 0.0, 0.0),
        Vec3::new(1.0, 2.0, 0.0),
    );
    assert!(
        centre.abs_diff_eq(Vec3::new(1.0, 0.75, 0.0), 2e-3),
        "{centre}"
    );
    assert!((r - 1.25).abs() < 2e-3, "{r}");
    let (normal, offset) = face_plane(
        Vec3::new(0.0, 0.0, 3.0),
        Vec3::new(1.0, 0.0, 3.0),
        Vec3::new(0.0, 1.0, 3.0),
    );
    assert!(normal.abs_diff_eq(Vec3::Z, 1e-3), "{normal}");
    assert!((offset - 3.0).abs() < 3e-3);
}

/// The closest point and its weights in each of the original's branches.
#[test]
fn the_closest_point_takes_the_originals_branches() {
    let (v0, v1, v2) = (
        Vec3::ZERO,
        Vec3::new(10.0, 0.0, 0.0),
        Vec3::new(0.0, 10.0, 0.0),
    );
    let at = |x: f32, y: f32| closest_point(v0, v1, v2, Vec3::new(x, y, 0.0));
    let (p, w) = at(2.0, 3.0);
    assert_eq!(p, Vec3::new(2.0, 3.0, 0.0));
    assert!((w[0] - 0.5).abs() < 1e-6 && (w[1] - 0.2).abs() < 1e-6 && (w[2] - 0.3).abs() < 1e-6);
    // Past the far edge: onto it, halfway.
    assert_eq!(at(8.0, 8.0), (Vec3::new(5.0, 5.0, 0.0), [0.0, 0.5, 0.5]));
    // Left of the 0-2 edge, below the 0-1 edge, beyond each corner.
    let (p, w) = at(-3.0, 4.0);
    assert!(p.abs_diff_eq(Vec3::new(0.0, 4.0, 0.0), 1e-6));
    assert!((w[0] - 0.6).abs() < 1e-6 && w[1] == 0.0 && (w[2] - 0.4).abs() < 1e-6);
    let (p, w) = at(4.0, -2.0);
    assert!(p.abs_diff_eq(Vec3::new(4.0, 0.0, 0.0), 1e-6));
    assert!((w[0] - 0.6).abs() < 1e-6 && (w[1] - 0.4).abs() < 1e-6 && w[2] == 0.0);
    assert_eq!(at(-1.0, -1.0), (v0, [1.0, 0.0, 0.0]));
    assert_eq!(at(12.0, -1.0), (v1, [0.0, 1.0, 0.0]));
}

/// Pushes that point the same way are not added twice: only what goes
/// beyond the longer one's projection counts.
#[test]
fn a_vertex_push_is_not_counted_twice() {
    let mut buffered = Vec3::ZERO;
    add_push(&mut buffered, Vec3::X);
    assert_eq!(buffered, Vec3::X);
    add_push(&mut buffered, Vec3::X * 2.0);
    assert_eq!(buffered, Vec3::X * 2.0);
    add_push(&mut buffered, Vec3::Y * 3.0);
    assert_eq!(buffered, Vec3::new(2.0, 3.0, 0.0));
    // Along (2, 3, 0), the shorter (1, 0, 0) loses (2/13)·(2, 3, 0).
    add_push(&mut buffered, Vec3::X);
    assert!(
        buffered.abs_diff_eq(Vec3::new(3.0 - 4.0 / 13.0, 3.0 - 6.0 / 13.0, 0.0), 1e-5),
        "{buffered}"
    );
}

/// The lower countdown takes the higher: a moving body wakes the cloth, a
/// sleeping one is owed the cloth's.
#[test]
fn a_cloth_and_a_body_trade_countdowns() {
    let mut cloth = 5;
    assert_eq!(trade_countdowns(&mut cloth, 256), (true, None));
    assert_eq!(cloth, 256);
    let mut cloth = 20;
    assert_eq!(trade_countdowns(&mut cloth, 0), (false, Some(20)));
    assert_eq!(cloth, 20);
    let mut cloth = 0;
    assert_eq!(trade_countdowns(&mut cloth, 0), (false, None));
    assert_eq!(cloth, 0);
}

fn floor_triangle(at: f32) -> (Vec<Particle>, Vec<ClothTri>) {
    (
        vec![
            still(Vec3::new(0.0, 0.0, at)),
            still(Vec3::new(20.0, 0.0, at)),
            still(Vec3::new(0.0, 20.0, at)),
        ],
        vec![ClothTri { a: 0, b: 1, c: 2 }],
    )
}

fn sphere(from: Vec3, to: Vec3, radius: f32, moving: bool) -> ClothBody {
    ClothBody {
        spheres: vec![BodySweep {
            from,
            to,
            radius,
            moving,
        }],
        countdown: 0,
    }
}

/// A sphere 3 cm over the cloth's face with a padded radius of 5 cm pushes
/// the closest point's triangle 2 cm away from itself, shared by weight
/// over the largest weight; from behind it pushes the other way; one out of
/// reach pushes nothing; a sweep through the face drives it ahead.
#[test]
fn a_body_sphere_pushes_the_triangle_it_reaches() {
    let (particles, triangles) = floor_triangle(0.0);
    let mut cloth = Cloth::new(particles.clone());
    cloth.triangles = triangles.clone();
    let bounds = triangle_bounds(&cloth);
    // The bound: the hypotenuse's sphere, centre (10, 10), radius √800 / 2.
    assert!(
        bounds[0]
            .probe
            .position
            .abs_diff_eq(Vec3::new(10.0, 10.0, 0.0), 1e-5)
    );
    assert!((bounds[0].probe.radius + 800.0f32.sqrt() * 0.5).abs() < 1e-4);
    assert_eq!(bounds[0].probe.radius, bounds[0].probe.radius_rate);
    let mut pushes = vec![Vec3::ZERO; 3];
    let over = sphere(
        Vec3::new(5.0, 5.0, 3.0),
        Vec3::new(5.0, 5.0, 3.0),
        5.0,
        false,
    );
    let touch = touch_body(&particles, &triangles, &bounds, &over, &mut pushes);
    assert!(touch.pushed);
    assert_eq!(touch.tests, 1);
    // Weights (0.5, 0.25, 0.25), depth 2, largest 0.5: 2, 1 and 1 cm down.
    assert!(
        pushes[0].abs_diff_eq(Vec3::new(0.0, 0.0, -2.0), 2e-3),
        "{:?}",
        pushes[0]
    );
    assert!(pushes[1].abs_diff_eq(Vec3::new(0.0, 0.0, -1.0), 2e-3));
    assert!(pushes[2].abs_diff_eq(Vec3::new(0.0, 0.0, -1.0), 2e-3));
    assert!(touch.reaction.abs_diff_eq(Vec3::new(0.0, 0.0, 2.0), 2e-3));

    let mut pushes = vec![Vec3::ZERO; 3];
    let under = sphere(
        Vec3::new(5.0, 5.0, -3.0),
        Vec3::new(5.0, 5.0, -3.0),
        5.0,
        false,
    );
    assert!(touch_body(&particles, &triangles, &bounds, &under, &mut pushes).pushed);
    assert!(pushes[0].abs_diff_eq(Vec3::new(0.0, 0.0, 2.0), 2e-3));

    let mut pushes = vec![Vec3::ZERO; 3];
    let clear = sphere(
        Vec3::new(5.0, 5.0, 10.0),
        Vec3::new(5.0, 5.0, 10.0),
        5.0,
        false,
    );
    let touch = touch_body(&particles, &triangles, &bounds, &clear, &mut pushes);
    assert!(!touch.pushed);
    assert_eq!(pushes, vec![Vec3::ZERO; 3]);

    // Through the face from 4 cm over to 4 cm under: the end is 4 cm past
    // the face and the radius 5 cm, so the triangle goes 9 cm ahead of it.
    let mut pushes = vec![Vec3::ZERO; 3];
    let through = sphere(
        Vec3::new(5.0, 5.0, 4.0),
        Vec3::new(5.0, 5.0, -4.0),
        5.0,
        true,
    );
    assert!(touch_body(&particles, &triangles, &bounds, &through, &mut pushes).pushed);
    assert!(
        pushes[0].abs_diff_eq(Vec3::new(0.0, 0.0, -9.0), 1e-2),
        "{:?}",
        pushes[0]
    );
}

/// The push pass moves a vertex out along its push after stopping its
/// travel across it (at most 0.2 cm²) or shortening it by √0.1 cm, keeps
/// the push as a plane and clears the buffer; the constraint pass puts a
/// vertex behind its plane back on it.
#[test]
fn the_push_pass_moves_and_holds_a_vertex() {
    let mut cloth = Cloth::new(vec![
        Particle {
            position: Vec3::ZERO,
            previous: Vec3::new(0.3, 0.0, 0.0),
        },
        Particle {
            position: Vec3::ZERO,
            previous: Vec3::new(-1.0, 0.0, 0.0),
        },
        still(Vec3::ZERO),
    ]);
    cloth.pushes = vec![Vec3::Y * 2.0, Vec3::Y * 2.0, Vec3::Y * 1.0e-4];
    let planes = apply_pushes(&mut cloth);
    assert_eq!(cloth.particles[0].position, Vec3::new(0.3, 2.0, 0.0));
    let slid = cloth.particles[1].position;
    assert!(
        slid.abs_diff_eq(Vec3::new(-(0.1f32).sqrt(), 2.0, 0.0), 1e-6),
        "{slid}"
    );
    assert_eq!(cloth.particles[2].position, Vec3::ZERO);
    assert_eq!(planes[0], Some((Vec3::Y, 2.0)));
    assert_eq!(planes[2], None);
    assert_eq!(cloth.pushes, vec![Vec3::ZERO; 3]);
    cloth.particles[0].position.y = 1.0;
    cloth.particles[1].position.y = 3.0;
    hold_planes(&mut cloth, &planes);
    assert_eq!(cloth.particles[0].position.y, 2.0);
    assert_eq!(cloth.particles[1].position.y, 3.0);
}

/// A sleeping cloth over a sleeping body stays asleep and still; a moving
/// body that reaches it wakes it with the body's countdown.
#[test]
fn only_a_moving_body_wakes_a_sleeping_cloth() {
    let (particles, triangles) = floor_triangle(0.0);
    let mut cloth = Cloth::new(particles.clone());
    cloth.triangles = triangles;
    cloth.sleep = 0;
    cloth.bodies = vec![sphere(
        Vec3::new(5.0, 5.0, 3.0),
        Vec3::new(5.0, 5.0, 3.0),
        5.0,
        false,
    )];
    step(&mut cloth, gravity_cm(1.0));
    assert_eq!(cloth.sleep, 0);
    assert_eq!(cloth.particles, particles);
    cloth.bodies[0].countdown = 256;
    step(&mut cloth, gravity_cm(1.0));
    assert_eq!(cloth.sleep, 256);
    assert!(
        cloth.particles[0].position.z < -1.9,
        "{:?}",
        cloth.particles[0]
    );
}

/// A loosely pinned cloth whose rest overlaps a sleeping body's sphere (a
/// banner hanging against a prop) settles against it and falls asleep: the
/// sleeping body never raises its countdown, and the push pass stops its
/// creep across the sphere.
#[test]
fn a_cloth_resting_on_a_sleeping_body_falls_asleep() {
    let rest = [
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(20.0, 0.0, 0.0),
        Vec3::new(0.0, 0.0, 20.0),
        Vec3::new(20.0, 0.0, 20.0),
    ];
    let mut cloth = Cloth::from_mesh(&rest, &[[0, 2, 1], [1, 2, 3]], &[0, 1, 2, 3]);
    for pin in &mut cloth.pins {
        pin.weight = 0.1;
    }
    // A sleeping prop's sphere bulging 1 cm through the cloth's middle.
    let centre = Vec3::new(10.0, -9.0, 10.0);
    cloth.bodies = vec![sphere(centre, centre, 10.0, false)];
    let gravity = gravity_cm(1.0);
    let mut slept = None;
    for tick in 0..400 {
        step(&mut cloth, gravity);
        if cloth.sleep == 0 {
            slept = Some(tick);
            break;
        }
    }
    assert!(slept.is_some(), "still awake at {}", cloth.sleep);
    // The shared edge rests on the sphere's top, 1 cm over the rest plane.
    let lifted = cloth.particles[1].position.y;
    assert!(lifted > 0.5, "{lifted}");
}
