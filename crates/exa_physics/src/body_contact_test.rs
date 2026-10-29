//! Level contacts of the physical body: static and swept detection,
//! one-sided faces, stored contacts inside the iterations, friction, rest.
use super::floor_test::{app, floor, hinged, humanoid, lowest};
use super::*;

/// A limp ball of radius 6 cm with its centre at `at`.
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

/// A floor quad at height `y`, facing up.
fn quad_floor(y: f32) -> Vec<[Vec3; 3]> {
    super::tests::floor_quad(y, 50.0)
}

/// A falling ball crosses many radii per tick; the sweep stops it on the
/// floor's front every tick, and it comes to rest touching it.
#[test]
fn a_fast_ball_does_not_tunnel_through_a_floor() {
    let mut app = app(floor(Vec3::ZERO), 1.0 / BODY_HZ);
    let mut b = ball(Vec3::new(0.0, 1.5, 0.0));
    // 24 m/s: 29 cm, nearly five radii, per tick.
    b.set_velocity(Vec3::NEG_Y * 24.0);
    let body = app.world_mut().spawn(b).id();
    for f in 0..200 {
        app.update();
        let (y, _) = lowest(app.world().get::<Ragdoll>(body).unwrap());
        assert!(y > 0.0, "tick {f}: centre {y:.3} m");
    }
    let (y, _) = lowest(app.world().get::<Ragdoll>(body).unwrap());
    assert!((y - 0.06).abs() < 0.002, "rests at {y:.4} m");
}

/// A zero-thickness wall stops a fast ball coming at its front (15 m/s, 18
/// cm or three radii a tick), which ends no tick behind it, and lets one
/// coming from behind through.
#[test]
fn a_thin_wall_stops_balls_from_its_front_only() {
    // A wall in the plane z = 0, facing +z.
    let at = |x: f32, y: f32| Vec3::new(x, y, 0.0);
    let wall = vec![
        [at(-5.0, -5.0), at(5.0, -5.0), at(5.0, 5.0)],
        [at(-5.0, -5.0), at(5.0, 5.0), at(-5.0, 5.0)],
    ];
    for (from, speed, stops) in [(1.0, -15.0, true), (-1.0, 20.0, false)] {
        let mut b = ball(Vec3::new(0.0, 1.0, from));
        b.set_velocity(Vec3::Z * speed);
        for tick in 0..30 {
            b.tick(wall.as_slice());
            let z = b.parts[0].transform().translation.z;
            assert!(!stops || z > 0.0, "tick {tick}: behind the front, z {z:.3}");
        }
        let z = b.parts[0].transform().translation.z;
        if !stops {
            assert!(z > 0.5, "stopped from behind: z {z:.3}");
        }
    }
}

/// A limp body dropped from 10 m (14 m/s at impact) lands with no sphere
/// centre under the floor.
#[test]
fn a_body_dropped_from_ten_metres_lands_on_the_floor() {
    let at = Vec3::new(60.0, 0.0, 40.0);
    let mut app = app(floor(at), 4.0 / BODY_HZ);
    let (setup, _) = humanoid(at + Vec3::Y * 10.0);
    let body = app.world_mut().spawn(Ragdoll::new(&setup)).id();
    for f in 0..300 {
        app.update();
        let (y, bone) = lowest(app.world().get::<Ragdoll>(body).unwrap());
        assert!(
            y > at.y,
            "frame {f}: bone {bone} centre {:.3} m under",
            at.y - y
        );
    }
}

/// A limb resting on the floor and dragged down by its links within a tick
/// is held by last tick's contacts during the iterations, before any new
/// detection.
#[test]
fn stored_contacts_hold_a_limb_dragged_by_its_links() {
    let level = quad_floor(0.0);
    // Pelvis above, a shin lying on the floor below it.
    let e = Entity::PLACEHOLDER;
    let mut body = Ragdoll::new(&[
        PartSetup {
            bone: 1,
            entity: e,
            parent: None,
            world: Affine3A::from_translation(Vec3::new(0.0, 0.45, 0.0)),
            centre: Vec3::ZERO,
            half: Vec3::splat(0.1),
            origin: Vec3::ZERO,
            links: vec![],
            foot: None,
            pins: Vec::new(),
            spheres: None,
        },
        PartSetup {
            bone: 3,
            entity: e,
            parent: Some(0),
            // Its origin, the knee, at its end; pinned there and above its
            // middle.
            world: Affine3A::from_translation(Vec3::new(-0.2, 0.06, 0.0)),
            centre: Vec3::new(0.2, 0.0, 0.0),
            half: Vec3::new(0.2, 0.05, 0.05),
            origin: Vec3::new(-0.2, -0.39, 0.0),
            links: vec![(Vec3::new(0.2, 0.05, 0.0), Vec3::new(0.2, 0.05, 0.0), 0.0)],
            foot: None,
            pins: Vec::new(),
            spheres: None,
        },
    ]);
    for _ in 0..200 {
        body.tick(level.as_slice());
    }
    assert!(
        !body.parts[1].contacts.is_empty(),
        "the shin found no contact"
    );
    // Yank the pelvis down at 20 m/s and run one tick by hand.
    for x in &mut body.parts[0].previous {
        *x += Vec3::Y * (20.0 / BODY_HZ);
    }
    body.integrate();
    for _ in 0..LINK_ITERATIONS {
        body.iterate();
    }
    let shin = &body.parts[1];
    let t = shin.transform();
    for &(c, r) in &shin.spheres {
        let y = t.transform_point3(c).y;
        assert!(y > r - 0.01, "the links pulled the shin's centre to {y:.3}");
    }
}

/// A limp body comes to rest touching the floor, does not creep, and falls
/// asleep; contacts never wake it, and waking starts it afresh.
#[test]
fn a_limp_body_rests_on_the_floor_and_sleeps() {
    let at = Vec3::new(81.0, 0.0, 39.2);
    let mut app = app(floor(at), 1.0 / BODY_HZ);
    let (setup, _) = hinged(at + Vec3::Y * 0.3);
    let body = app.world_mut().spawn(Ragdoll::new(&setup)).id();
    for _ in 0..(3 * 83) {
        app.update();
    }
    let pelvis = |app: &App| {
        Vec3::from(
            app.world()
                .get::<Ragdoll>(body)
                .unwrap()
                .part_of(1)
                .unwrap()
                .transform()
                .translation,
        )
    };
    let settled = pelvis(&app);
    for _ in 0..(5 * 83) {
        app.update();
    }
    let ragdoll = app.world().get::<Ragdoll>(body).unwrap();
    assert!(ragdoll.asleep(), "still awake after 8 s");
    assert!(
        pelvis(&app).distance(settled) < 0.01,
        "crept {:.3} m",
        pelvis(&app).distance(settled)
    );
    for p in &ragdoll.parts {
        let t = p.transform();
        for &(c, r) in &p.spheres {
            let gap = t.transform_point3(c).y - at.y - r;
            assert!(gap > -0.003, "bone {} sphere sunk {:.4} m", p.bone, -gap);
        }
    }
    // Every part rests within a centimetre of its socket, and every link
    // within its slack.
    for (i, p) in ragdoll.parts.iter().enumerate() {
        let Some(j) = p.parent else { continue };
        let (own, parent) = (p.transform(), ragdoll.parts[j].transform());
        for l in ragdoll.links_of(i) {
            let (a, b) = ragdoll.link_points(l);
            let beyond = own.transform_point3(a).distance(parent.transform_point3(b)) - l.slack;
            assert!(
                beyond < 0.01,
                "bone {} rests {:.1} mm beyond its link (origin pivot {})",
                p.bone,
                beyond * 1000.0,
                l.origin
            );
        }
    }
    // Asleep, it is not tested against the level and stays as it is.
    let still = ragdoll.parts.clone();
    for _ in 0..83 {
        app.update();
    }
    let ragdoll = app.world().get::<Ragdoll>(body).unwrap();
    assert!(ragdoll.asleep());
    assert_eq!(ragdoll.parts[0].positions, still[0].positions);
    // A shove wakes it with no contacts carried over from before the sleep.
    let mut ragdoll = app.world_mut().get_mut::<Ragdoll>(body).unwrap();
    ragdoll.shove(8, Vec3::X, 0.0);
    assert!(ragdoll.parts.iter().all(|p| p.contacts.is_empty()));
}

/// The friction law: below the limit a contact point's slip is cancelled,
/// above it damped by a hundredth of the limit; walls hold 100 times less.
#[test]
fn friction_sticks_slow_slips_and_lets_fast_ones_slide() {
    let up = Vec3::Y;
    // Limb on a floor: limit 1.25 cm² per tick², stick below ~1.12 cm/tick.
    let slow = Vec3::X * 0.011;
    assert!((rub(slow, up, LIMB_FRICTION) + slow).length() < 1e-9);
    let fast = Vec3::X * 0.012;
    let c = rub(fast, up, LIMB_FRICTION);
    assert!((c + fast * 0.0125).length() < 1e-7, "{c}");
    // On a wall (n_y = 0) the limit is 0.0125: stick below ~0.11 cm/tick.
    let side = Vec3::X;
    assert!((rub(Vec3::Y * 0.001, side, LIMB_FRICTION) + Vec3::Y * 0.001).length() < 1e-9);
    assert!(rub(Vec3::Y * 0.002, side, LIMB_FRICTION).length() < 0.002 * 0.001);
    // The normal part of the slip is not friction's.
    assert_eq!(rub(up * 0.5, up, LIMB_FRICTION), Vec3::ZERO);
}

/// The swept test, branch by branch, on a floor triangle facing up.
#[test]
fn swept_hits_follow_the_original_rules() {
    let t = [
        Vec3::new(-1.0, 0.0, -1.0),
        Vec3::new(-1.0, 0.0, 1.0),
        Vec3::new(1.0, 0.0, 1.0),
    ];
    let r = 0.1;
    let inside = Vec3::new(-0.5, 0.0, 0.5);
    // The end still in front: the static test at the end.
    let (q, d) = swept_hit(&t, inside + Vec3::Y, inside + Vec3::Y * 0.04, r).unwrap();
    assert!((q - inside).length() < 1e-6 && (d - Vec3::Y * 0.06).length() < 1e-6);
    // The end crossed: back to the first touch, along the reversed motion.
    let start = inside + Vec3::new(-0.3, 1.0, 0.0);
    let end = inside + Vec3::new(0.0, -0.2, 0.0);
    let (_, d) = swept_hit(&t, start, end, r).unwrap();
    let centre = end + d;
    assert!((centre.y - r).abs() < 1e-5, "touches at {centre}");
    assert!(d.normalize().dot((start - end).normalize()) > 0.9999);
    // A start behind the face: nothing.
    assert!(swept_hit(&t, inside - Vec3::Y * 0.05, inside - Vec3::Y * 0.5, r).is_none());
    // The static test is one-sided too.
    assert!(sphere_hit(&t, inside - Vec3::Y * 0.01, r).is_none());
    // Beside the triangle's edge, barely crossing: pushed off the edge
    // point, never further behind the plane.
    let edge = Vec3::new(0.0, 0.0, 0.0);
    let off = Vec3::new(0.05, 0.0, -0.05).normalize() * 0.05;
    let (q, d) = swept_hit(
        &t,
        edge + off + Vec3::Y * 0.5,
        edge + off - Vec3::Y * 1e-4,
        r,
    )
    .unwrap();
    assert!(q.distance(edge) < 1e-5);
    assert!(d.y >= 0.0 && d.length() <= r, "push {d}");
}

/// Four contacts at most per part: a fifth replaces the least isolated.
#[test]
fn a_part_keeps_its_four_most_spread_contacts() {
    let mut b = ball(Vec3::ZERO);
    let p = &mut b.parts[0];
    let contact = |w: [f32; 4]| LevelContact {
        weights: w,
        point: Vec3::ZERO,
        normal: Vec3::Y,
        plane: 0.0,
    };
    for w in [
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [0.9, 0.1, 0.0, 0.0],
    ] {
        p.offer(contact(w));
    }
    p.offer(contact([0.0, 0.0, 0.0, 1.0]));
    let mut kept: Vec<[f32; 4]> = p.contacts.iter().map(|c| c.weights).collect();
    kept.sort_by(|a, b| b.partial_cmp(a).unwrap());
    assert_eq!(kept.len(), 4);
    assert!(kept.contains(&[0.0, 0.0, 0.0, 1.0]));
    assert!(kept.contains(&[0.0, 1.0, 0.0, 0.0]) && kept.contains(&[0.0, 0.0, 1.0, 0.0]));
    // One of the two near the first particle went.
    assert_eq!(kept.iter().filter(|w| w[0] > 0.5).count(), 1);
}

/// A body carried by its controller is not swept by the carry, and its
/// contacts travel with it, across the ground only: the root's rise is the
/// lift's to follow. Where it is drawn from stays: the carry is drawn as
/// motion between ticks, as the controller is.
#[test]
fn a_carried_body_moves_its_last_pose_and_contacts() {
    // The feet 1 cm into the floor, so that the shins touch it.
    let (setup, pose) = humanoid(Vec3::new(0.0, -0.01, 0.0));
    let mut body = Ragdoll::new(&setup).with_drive();
    let floor = quad_floor(0.0);
    // A few ticks: the floor pushes the body up out of it and it falls
    // back (a legless body has no lift).
    for _ in 0..3 {
        body.aim(Affine3A::IDENTITY, pose.clone());
        body.tick(floor.as_slice());
    }
    let down = Affine3A::from_translation(Vec3::new(0.5, -0.3, 0.0));
    let shift = Vec3::new(0.5, 0.0, 0.0);
    let across = Affine3A::from_translation(shift);
    let before: Vec<(Affine3A, Vec<LevelContact>, (Affine3A, Affine3A))> = body
        .parts
        .iter()
        .map(|p| (p.settled, p.contacts.clone(), p.shown))
        .collect();
    assert!(
        before.iter().any(|(_, c, _)| !c.is_empty()),
        "no contacts to carry"
    );
    body.aim(down, pose.iter().map(|t| down * *t).collect());
    for (p, (settled, contacts, shown)) in body.parts.iter().zip(before) {
        assert!(
            p.settled
                .translation
                .abs_diff_eq((across * settled).translation, 1e-5)
        );
        assert_eq!(p.shown, shown);
        assert_eq!(p.contacts.len(), contacts.len());
        for (c, old) in p.contacts.iter().zip(contacts) {
            assert!(c.point.abs_diff_eq(old.point + shift, 1e-5));
            assert!((c.plane - (old.plane + old.normal.dot(shift))).abs() < 1e-5);
        }
    }
}

/// The sweep starts where the last tick left the sphere: a ball that went
/// through a thin wall from behind is stopped by its front on the way back.
#[test]
fn a_ball_back_through_a_wall_is_stopped_by_its_front() {
    // A wall in the plane z = 0, facing +z.
    let at = |x: f32, y: f32| Vec3::new(x, y, 0.0);
    let wall = vec![
        [at(-5.0, -5.0), at(5.0, -5.0), at(5.0, 5.0)],
        [at(-5.0, -5.0), at(5.0, 5.0), at(-5.0, 5.0)],
    ];
    let mut b = ball(Vec3::new(0.0, 1.0, -1.0));
    b.set_velocity(Vec3::Z * 20.0);
    for _ in 0..10 {
        b.tick(wall.as_slice());
    }
    let z = b.parts[0].transform().translation.z;
    assert!(z > 0.5, "stopped from behind: z {z:.3}");
    b.set_velocity(Vec3::Z * -20.0);
    for _ in 0..30 {
        b.tick(wall.as_slice());
    }
    let z = b.parts[0].transform().translation.z;
    assert!(z > 0.05, "went back through the front: z {z:.3}");
}

/// The stored contacts' friction acts inside the iterations: a ball resting
/// on a floor, set sliding below the stick limit, has its contact points
/// held during the next tick's iterations, before any detection.
#[test]
fn stored_contacts_stick_inside_the_iterations() {
    let floor = quad_floor(0.0);
    let mut b = ball(Vec3::new(0.0, 0.06, 0.0));
    for _ in 0..100 {
        b.tick(floor.as_slice());
    }
    assert!(!b.parts[0].contacts.is_empty(), "the ball found no contact");
    // 0.5 m/s: 0.6 cm per tick, under the 1.12 cm stick limit of a limb.
    b.set_velocity(Vec3::X * 0.5);
    b.integrate();
    for _ in 0..LINK_ITERATIONS {
        b.iterate();
    }
    let p = &b.parts[0];
    for c in &p.contacts {
        let slip = p.slip(&c.weights);
        let slide = (slip - c.normal * slip.dot(c.normal)).length();
        assert!(slide < 1e-4, "a contact point slid {slide:.4} m");
    }
}

/// Links are inside the body: links that cannot all hold (a skeleton's
/// links laid over box-shaped tetrahedra need not agree) must not move the
/// body as a whole. Moving each link point by exactly its share does not
/// balance the two parts' momentum, and a lasting conflict drove the body at
/// up to 25 m/s (the likely cause of placed corpses flying across exanima01
/// at 20–50 m/s); shares spread over the particles by the points' weights
/// balance it.
#[test]
fn conflicting_links_do_not_propel_a_limp_body() {
    let e = Entity::PLACEHOLDER;
    let at = |x: f32| Affine3A::from_translation(Vec3::new(x, 1.0, 0.0));
    let mut body = Ragdoll::new(&[
        PartSetup {
            bone: 2,
            entity: e,
            parent: None,
            world: at(0.0),
            centre: Vec3::ZERO,
            half: Vec3::splat(0.1),
            origin: Vec3::ZERO,
            links: vec![],
            foot: None,
            pins: Vec::new(),
            spheres: None,
        },
        PartSetup {
            bone: 3,
            entity: e,
            parent: Some(0),
            world: at(0.3),
            centre: Vec3::ZERO,
            half: Vec3::splat(0.1),
            origin: Vec3::new(0.3, 0.0, 0.0),
            // The first holds at rest, as the origin pivot does; the second
            // asks for the part 10 cm nearer at z = 0.1: they cannot all
            // hold on rigid parts.
            links: vec![
                (Vec3::new(-0.15, 0.0, 0.0), Vec3::new(-0.15, 0.0, 0.0), 0.0),
                (Vec3::new(-0.15, 0.0, 0.1), Vec3::new(-0.25, 0.0, 0.1), 0.0),
            ],
            foot: None,
            pins: Vec::new(),
            spheres: None,
        },
    ]);
    for _ in 0..83 {
        body.tick(NONE_LEVEL);
    }
    let (_, v) = body.centre_of_mass();
    let drift = Vec2::new(v.x, v.z).length();
    assert!(drift < 0.1, "the links drove the body at {drift:.1} m/s");
}

const NONE_LEVEL: &[[Vec3; 3]] = &[];

/// A piece's level mask is tested against the part's body mask: a living
/// foot (mask 4) ignores a thin/flagged collider (mask 4), while an
/// ordinary part (mask 0) still meets it. Pieces without a mask are 1.
#[test]
fn a_feet_mask_skips_a_thin_collider() {
    struct Masked(Vec<[Vec3; 3]>, u32);
    impl Level for Masked {
        fn pieces(&self, lo: Vec3, hi: Vec3, out: &mut Vec<u32>) {
            Level::pieces(&self.0[..], lo, hi, out);
        }
        fn triangles(&self, piece: u32, lo: Vec3, hi: Vec3, out: &mut Vec<[Vec3; 3]>) {
            Level::triangles(&self.0[..], piece, lo, hi, out);
        }
        fn mask(&self, _piece: u32) -> u32 {
            self.1
        }
    }
    let level = Masked(quad_floor(0.0), crate::sector::MASK_THIN);
    let run = |foot: bool| {
        let mut b = ball(Vec3::new(0.0, 1.0, 0.0));
        if foot {
            b.parts[0].pairing.out = true;
        }
        for _ in 0..200 {
            b.tick(&level);
        }
        b.parts[0].transform().translation.y
    };
    // A limp body's foot carries no mask (the original clears it on
    // collapse and death), so it rests on the thin collider like any part.
    assert!(run(true) > 0.05, "a limp foot fell through a thin collider");
    assert!(run(false) > 0.05, "the ordinary part fell through");
    // A living body's foot carries mask 4 (the leg routine's write), which
    // shares bit 4 with the thin collider: the detection skips it. Being
    // out of pairs or driven does not give the bit by itself.
    let mut b = ball(Vec3::new(0.0, 1.0, 0.0));
    b.parts[0].pairing.out = true;
    b.drive = Some(Drive::default());
    assert_eq!(b.level_mask(0), 0);
    b.parts[0].skips_thin = true;
    assert_eq!(b.level_mask(0), crate::sector::MASK_THIN);
    assert!(!crate::sector::level_meets(b.level_mask(0), level.1));
    b.parts[0].no_level = true;
    assert_eq!(
        b.level_mask(0),
        crate::sector::MASK_SMALL | crate::sector::MASK_THIN
    );
}
