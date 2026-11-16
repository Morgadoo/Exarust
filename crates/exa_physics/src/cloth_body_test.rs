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
        part: 0,
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
    // The reaction is the opposite of the vertices' summed move.
    let moved: Vec3 = cloth
        .particles
        .iter()
        .zip(&particles)
        .map(|(now, was)| now.position - was.position)
        .sum();
    assert!(cloth.reaction.abs_diff_eq(-moved, 1e-4));
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

/// A still body whose tetrahedron did not move, and its one sphere.
fn particle_body(centre: Vec3, radius: f32, padding: f32, shift: Vec3) -> ParticleBody {
    let tet = [
        Vec3::ZERO,
        Vec3::new(10.0, 0.0, 0.0),
        Vec3::new(0.0, 10.0, 0.0),
        Vec3::new(0.0, 0.0, 10.0),
    ];
    let mut points = [[0.0; 3]; 8];
    for (i, p) in tet.iter().enumerate() {
        points[i] = (*p + shift).to_array();
        points[i + 4] = p.to_array();
    }
    ParticleBody {
        spheres: vec![(centre, radius)],
        padding,
        points,
        countdown: 256,
        part: 0,
    }
}

fn moving(from: Vec3, to: Vec3) -> Particle {
    Particle {
        position: to,
        previous: from,
    }
}

/// The particle mode pushes a particle straight out to the sphere's
/// surface: the owner's sphere takes no padding, any other body's 1 cm.
#[test]
fn the_particle_mode_pushes_out_to_the_padded_surface() {
    let cloth_box = (Vec3::new(0.0, 5.0, 0.0), Vec3::splat(1.0));
    let owner = particle_body(Vec3::ZERO, 10.0, 0.0, Vec3::ZERO);
    let mut particles = [still(Vec3::new(0.0, 5.0, 0.0))];
    assert!(touch_particles(&mut particles, cloth_box, &owner));
    assert_eq!(particles[0].position, Vec3::new(0.0, 10.0, 0.0));
    let other = particle_body(Vec3::ZERO, 10.0, PARTICLE_PADDING_CM, Vec3::ZERO);
    let mut particles = [still(Vec3::new(0.0, 5.0, 0.0))];
    assert!(touch_particles(&mut particles, cloth_box, &other));
    assert!((particles[0].position - Vec3::new(0.0, 11.0, 0.0)).length() < 1e-5);
    // Outside the padded sphere nothing moves.
    let mut particles = [still(Vec3::new(0.0, 12.0, 0.0))];
    assert!(!touch_particles(&mut particles, cloth_box, &other));
    assert_eq!(particles[0].position, Vec3::new(0.0, 12.0, 0.0));
}

/// A push whose square is under 1e-4 cm² is no push.
#[test]
fn the_particle_mode_ignores_a_tiny_push() {
    let body = particle_body(Vec3::ZERO, 10.0, 0.0, Vec3::ZERO);
    let at = Vec3::new(0.0, 9.995, 0.0);
    let mut particles = [still(at)];
    assert!(!touch_particles(
        &mut particles,
        (at, Vec3::splat(1.0)),
        &body
    ));
    assert_eq!(particles[0].position, at);
}

/// The travel across the push is cancelled while its square is within a
/// tenth of the push's square, else shortened by `√(0.1·push²/travel²)`.
#[test]
fn the_particle_mode_sticks_or_slides_the_travel_across_the_push() {
    let body = particle_body(Vec3::ZERO, 10.0, 0.0, Vec3::ZERO);
    let cloth_box = (Vec3::new(0.0, 5.0, 0.0), Vec3::splat(4.0));
    // Push (0, 5, 0), push² 25: a 1 cm travel (1 <= 2.5) is cancelled, so
    // the particle goes back to where it was across the push.
    let mut particles = [moving(Vec3::new(-1.0, 5.0, 0.0), Vec3::new(0.0, 5.0, 0.0))];
    assert!(touch_particles(&mut particles, cloth_box, &body));
    assert_eq!(particles[0].position, Vec3::new(-1.0, 10.0, 0.0));
    // A 3 cm travel (9 > 2.5) loses √(2.5/9) of itself.
    let mut particles = [moving(Vec3::new(-3.0, 5.0, 0.0), Vec3::new(0.0, 5.0, 0.0))];
    assert!(touch_particles(&mut particles, cloth_box, &body));
    let lose = -(2.5_f32 / 9.0).sqrt();
    let push = Vec3::new(0.0, 5.0, 0.0) + Vec3::new(3.0, 0.0, 0.0) * lose;
    assert_eq!(particles[0].position, Vec3::new(0.0, 5.0, 0.0) + push);
    // The travel along the push does not count.
    let mut particles = [moving(Vec3::new(0.0, 4.0, 0.0), Vec3::new(0.0, 5.0, 0.0))];
    assert!(touch_particles(&mut particles, cloth_box, &body));
    assert_eq!(particles[0].position, Vec3::new(0.0, 10.0, 0.0));
}

/// The body's own motion at the particle is taken off its travel: a
/// particle carried with the body does not slide.
#[test]
fn the_particle_mode_measures_the_travel_against_the_body() {
    let shift = Vec3::new(1.0, 0.0, 0.0);
    let body = particle_body(Vec3::new(1.0, 0.0, 0.0), 10.0, 0.0, shift);
    let v = body.velocity_at(Vec3::new(1.0, 5.0, 0.0));
    assert!((v - shift).length() < 1e-3, "{v}");
    let cloth_box = (Vec3::new(1.0, 5.0, 0.0), Vec3::splat(4.0));
    let mut particles = [moving(Vec3::new(0.0, 5.0, 0.0), Vec3::new(1.0, 5.0, 0.0))];
    assert!(touch_particles(&mut particles, cloth_box, &body));
    let at = particles[0].position;
    assert!((at - Vec3::new(1.0, 10.0, 0.0)).length() < 1e-3, "{at}");
}

/// A particle-mode cloth (the hair's record) takes its bodies straight onto
/// its particles, and a pushing body wakes it with its countdown.
#[test]
fn a_particle_mode_cloth_is_pushed_and_woken_by_a_body() {
    let mut cloth = Cloth::new(vec![still(Vec3::new(0.0, 5.0, 0.0))]);
    cloth.record = crate::cloth_record::ClothRecord::HAIR;
    cloth.sleep = 0;
    cloth.particle_bodies = vec![particle_body(Vec3::ZERO, 10.0, 0.0, Vec3::ZERO)];
    step(&mut cloth, gravity_cm(1.0));
    assert_eq!(cloth.sleep, 256);
    assert_eq!(cloth.particles[0].position, Vec3::new(0.0, 10.0, 0.0));
    // The triangle buffer is never filled in the particle mode.
    assert!(cloth.pushes.iter().all(|p| *p == Vec3::ZERO));
}

/// The body's share of a cloth push takes the original's three words:
/// 0.015 for an active striker, 0.01 for a cloth on a held chain, 0.002
/// for every other contact (read from the executable's data).
#[test]
fn the_body_share_takes_the_originals_words() {
    assert_eq!(body_share(true, false).to_bits(), 0x3c75_c28f);
    assert_eq!(body_share(true, true).to_bits(), 0x3c75_c28f);
    assert_eq!(body_share(false, true).to_bits(), 0x3c23_d70a);
    assert_eq!(body_share(false, false).to_bits(), 0x3b03_126f);
}

/// The slide friction a pushing sphere puts into the body's motion: the
/// speed's part across the push is kept at `−√(0.1·push²/v²)` of itself
/// when `0.2·push² < v²`, else turned back whole; each squared speed over
/// the small float adds `v²·2e-5` to the rustle tail (double precision);
/// a body that barely moved there takes nothing.
#[test]
fn a_pushing_sphere_rubs_the_bodys_motion() {
    // v = (3, 0, 4) cm, push (0, 0, −1) cm: v² = 25, push² = 1; across the
    // push (3, 0, 0), kept at −√(0.1/25).
    let mut tail = 0.5f32;
    let slide = body_slide(
        Vec3::new(3.0, 0.0, 4.0),
        Vec3::new(0.0, 0.0, -1.0),
        &mut tail,
    )
    .expect("a moving body rubs");
    let keep = -(0.1f64 / 25.0).sqrt() as f32;
    assert_eq!(slide, Vec3::new(3.0 * keep, 0.0, 0.0));
    assert_eq!(tail, (25.0f64 * 2.0e-5 + 0.5f64) as f32);
    // 0.2 · 4 ≥ 0.25: the slide (0.3, 0, 0) is turned back whole (0 − v,
    // so the zero lanes stay +0).
    let mut tail = 0.0f32;
    let back = body_slide(
        Vec3::new(0.3, 0.0, 0.4),
        Vec3::new(0.0, 0.0, -2.0),
        &mut tail,
    )
    .unwrap();
    assert_eq!(back.x, -0.3);
    assert_eq!(back.y.to_bits(), 0);
    assert_eq!(back.z.to_bits(), 0);
    assert_eq!(tail, (0.25f32 as f64 * 2.0e-5) as f32);
    // At or under the small float, or unordered: nothing, no rustle.
    let mut tail = 1.0f32;
    assert_eq!(body_slide(Vec3::splat(1.0e-4), Vec3::Y, &mut tail), None);
    assert_eq!(
        body_slide(Vec3::new(f32::NAN, 0.0, 0.0), Vec3::Y, &mut tail),
        None
    );
    assert_eq!(tail, 1.0);
}

/// One part's rest spheres over a cloth triangle (cm = world m · 100, the
/// cloth's frame the world's).
fn live_part(centre_m: Vec3, radius_m: f32) -> crate::body::Ragdoll {
    crate::body::Ragdoll::new(&[crate::body::PartSetup {
        bone: 1,
        entity: bevy::prelude::Entity::from_raw_u32(7).unwrap(),
        parent: None,
        world: bevy::math::Affine3A::from_translation(centre_m),
        centre: Vec3::ZERO,
        half: Vec3::splat(0.05),
        origin: Vec3::ZERO,
        links: vec![],
        foot: None,
        pins: Vec::new(),
        spheres: Some(vec![(Vec3::ZERO, radius_m)]),
    }])
}

/// The live contact pushes the cloth as the fixed contact does and hands
/// the body its share at once: the sphere's summed push times 0.002, in
/// metres, on the part's particles by the sphere centre's raw weights in
/// its rest tetrahedron (the approximate reciprocal); a body with motion
/// at the sphere also takes the slide friction on its motion record and
/// adds its rustle to the cloth's tail; the countdown trade wakes a
/// sleeping body up to the cloth's.
#[test]
fn a_live_body_takes_its_share_of_the_cloths_push() {
    let (particles, triangles) = floor_triangle(0.0);
    let mut cloth = Cloth::new(particles.clone());
    cloth.triangles = triangles.clone();
    let bounds = triangle_bounds(&cloth);
    // A part 3 cm over the face (radius 3 cm, padded 5 cm), still.
    let mut body = live_part(Vec3::new(0.05, 0.05, 0.03), 0.03);
    let before = body.parts[0].positions;
    let contact = LiveContact {
        to_object: bevy::math::Affine3A::IDENTITY,
        cloth_box: crate::cloth::bounds(&particles),
        padding: 2.0,
        held_chain: false,
    };
    // The same sphere through the fixed contact gives the cloth's pushes
    // and the summed push.
    let sweep = sphere_sweep(
        &contact.to_object,
        &body.parts[0].transform(),
        &body.parts[0].settled_transform(),
        (Vec3::ZERO, 0.03),
        2.0,
        contact.cloth_box,
    )
    .expect("the sphere reaches the cloth");
    let mut fixed = vec![Vec3::ZERO; 3];
    let alone = touch_body(
        &particles,
        &triangles,
        &bounds,
        &ClothBody {
            spheres: vec![sweep],
            countdown: 0,
            part: 0,
        },
        &mut fixed,
    );
    assert!(alone.pushed);
    let mut pushes = vec![Vec3::ZERO; 3];
    let mut tail = 0.0;
    let touch = touch_ragdoll(
        &particles,
        &triangles,
        &bounds,
        &mut pushes,
        &mut tail,
        &mut body,
        0,
        &contact,
    );
    assert!(touch.pushed);
    assert_eq!((touch.spheres, touch.swept), (1, 0));
    assert_eq!(pushes, fixed);
    assert_eq!(touch.reaction, alone.reaction);
    // The body: push · 0.002 (cm) to metres, by the raw weights.
    let share = alone.reaction * 0.002;
    let rest = body.parts[0].rest.map(|p| p.to_array());
    let w = engine_math::barycentric_fast(Vec3::ZERO, &rest);
    for k in 0..4 {
        assert_eq!(body.parts[0].positions[k], before[k] + share * 0.01 * w[k]);
    }
    // A still body adds no rustle and keeps no motion.
    assert_eq!(tail, 0.0);
    assert_eq!(body.parts[0].motion_at(&w), Vec3::ZERO);

    // Moving 1 cm a tick along x at the sphere: v² = 1, the push across
    // it, so the slide keeps −√(0.1·push²/1) of the motion, and the tail
    // takes 2e-5.
    let mut body = live_part(Vec3::new(0.05, 0.05, 0.03), 0.03);
    body.parts[0].add_motion(&[1.0; 4], Vec3::new(0.01, 0.0, 0.0));
    let velocity = body.parts[0].motion_at(&w) * 100.0;
    let mut pushes = vec![Vec3::ZERO; 3];
    let mut tail = 0.0;
    touch_ragdoll(
        &particles,
        &triangles,
        &bounds,
        &mut pushes,
        &mut tail,
        &mut body,
        0,
        &contact,
    );
    let mut expected_tail = 0.0;
    let slide = body_slide(velocity, share, &mut expected_tail).unwrap();
    assert_eq!(tail, expected_tail);
    assert_eq!(tail, (f64::from(velocity.length_squared()) * 2.0e-5) as f32);
    let motion = body.parts[0].motion_at(&[1.0, 0.0, 0.0, 0.0]);
    assert_eq!(motion, Vec3::new(0.01, 0.0, 0.0) + slide * 0.01 * w[0]);

    // A sleeping body (countdown 0) under the awake cloth is touched up to
    // the cloth's countdown.
    let mut body = live_part(Vec3::new(0.05, 0.05, 0.03), 0.03);
    body.hold_out();
    body.set_disabled(false);
    assert!(body.asleep());
    let mut countdown = 20;
    assert!(!trade_body_countdowns(&mut countdown, &mut body));
    assert!(!body.asleep());
    assert_eq!(body.countdown(), 20);
    // An awake body with the higher countdown wakes the cloth instead.
    let mut countdown = 0;
    assert!(trade_body_countdowns(&mut countdown, &mut body));
    assert_eq!(countdown, 20);
}
