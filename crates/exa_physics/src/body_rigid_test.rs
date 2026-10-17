use super::super::tests::floor_quad;
use super::*;
use crate::contact::Level;

const NONE: &[[Vec3; 3]] = &[];

/// A 40 cm crate's rigid body (a tetrahedron over its box, equal point
/// masses, one 20 cm sphere at its centre and four at its bottom corners),
/// its node at `at`.
fn crate_at(at: Vec3) -> RigidPart {
    RigidPart {
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
        mobility: 0.5,
        spheres: vec![
            (Vec3::new(0.0, 0.2, 0.0), 0.2),
            (Vec3::new(0.15, 0.05, 0.15), 0.05),
            (Vec3::new(-0.15, 0.05, 0.15), 0.05),
            (Vec3::new(0.15, 0.05, -0.15), 0.05),
            (Vec3::new(-0.15, 0.05, -0.15), 0.05),
        ],
        origin: Vec3::ZERO,
        links: Vec::new(),
    }
}

fn lowest(body: &Ragdoll) -> f32 {
    body.parts
        .iter()
        .flat_map(|p| {
            let t = p.transform();
            p.spheres
                .iter()
                .map(move |&(c, r)| t.transform_point3(c).y - r)
        })
        .fold(f32::MAX, f32::min)
}

/// Gravity shares 4/mᵢ scaled to sum to 4: equal masses share equally;
/// point masses 2, 4, 8, 8 (a centre of mass at weights ½, ¼, ⅛, ⅛) give
/// 2, 1, 0.5, 0.5; masses that give no shares fall back to equal ones.
#[test]
fn gravity_acts_at_the_centre_of_mass() {
    assert_eq!(gravity_shares([4.0; 4]), [1.0; 4]);
    assert_eq!(gravity_shares([2.0, 4.0, 8.0, 8.0]), [2.0, 1.0, 0.5, 0.5]);
    assert_eq!(gravity_shares([0.0, 4.0, 4.0, 4.0]), [1.0; 4]);
    // One integration from rest moves each particle by its share of a
    // tick's gravity (0.142352 cm a tick²).
    let mut part = crate_at(Vec3::ZERO);
    part.point_masses = [2.0, 4.0, 8.0, 8.0];
    let mut body = Ragdoll::rigid(&[part]);
    body.sleep = Sleep::Awake;
    let before = body.parts[0].positions;
    body.integrate();
    for (i, w) in [2.0, 1.0, 0.5, 0.5].into_iter().enumerate() {
        let fell = (before[i].y - body.parts[0].positions[i].y) * 100.0;
        assert!(
            (fell - 0.142352 * w).abs() < 1e-4,
            "particle {i}: {fell} cm"
        );
    }
}

/// Registered as the original registers a body: asleep (countdown 0, in
/// deep sleep), its particles at its node's matrix, at rest, with the
/// bodies' friction 0.75 and its mobility.
#[test]
fn a_registered_body_is_asleep_where_its_node_is() {
    let at = Vec3::new(1.0, 0.5, -2.0);
    let body = Ragdoll::rigid(&[crate_at(at)]);
    assert_eq!((body.countdown(), body.sleep), (0, Sleep::Deep));
    let p = &body.parts[0];
    assert_eq!(p.positions[3], at + Vec3::new(0.0, 0.4, 0.0));
    assert_eq!(p.previous, p.positions);
    assert_eq!((p.friction, p.inverse_mass), (0.75, 0.5));
    assert!(p.transform().translation.distance(at.into()) < 1e-5);
    assert!(!body.meets_movable && !body.fell_out());
}

/// A registered body stays exactly where the level put it, even in the
/// air, until something wakes it.
#[test]
fn a_sleeping_prop_stays_put() {
    let floor = floor_quad(0.0, 5.0);
    let mut body = Ragdoll::rigid(&[crate_at(Vec3::new(0.0, 0.3, 0.0))]);
    let placed = body.parts[0].positions;
    for _ in 0..500 {
        body.tick(floor.as_slice());
    }
    assert_eq!(body.parts[0].positions, placed);
    assert_eq!((body.countdown(), body.sleep), (0, Sleep::Deep));
}

/// A shove wakes a prop to 256: it falls the 10 cm it hung over the
/// floor, comes to rest on it, and sleeps again 128 ticks after it stops
/// (its countdown falls two a tick at rest).
#[test]
fn a_shoved_prop_falls_settles_and_sleeps_again() {
    let floor = floor_quad(0.0, 5.0);
    let mut body = Ragdoll::rigid(&[crate_at(Vec3::new(0.0, 0.1, 0.0))]);
    body.shove(0, Vec3::new(0.5, 0.0, 0.0), 0.0);
    assert_eq!((body.countdown(), body.sleep), (256, Sleep::Awake));
    let mut slept = None;
    for tick in 0..600 {
        body.tick(floor.as_slice());
        if body.asleep() && slept.is_none() {
            slept = Some(tick);
        }
    }
    let slept = slept.expect("the prop never slept again");
    // 140 ticks: about 12 to land and stop, then 128 at rest.
    assert!((128..160).contains(&slept), "slept after {slept} ticks");
    let bottom = lowest(&body);
    assert!(
        bottom.abs() < 0.01,
        "rests {:.1} cm off the floor",
        bottom * 100.0
    );
    // Slid about 6 cm on the floor's friction (0.5 m/s).
    let moved = body.parts[0].transform().translation.x;
    assert!(
        (0.04..0.09).contains(&moved),
        "the shove moved it {moved} m"
    );
    // Asleep again, it no longer moves.
    let rested = body.parts[0].positions;
    for _ in 0..100 {
        body.tick(floor.as_slice());
    }
    assert_eq!(body.parts[0].positions, rested);
}

/// An articulated child: its stored links, then a pivot from its origin to
/// where its node sits in the parent, each correction shared by the
/// mobilities (a sack's 0.4676 child on its 0.1370 body: 0.7734 and
/// 0.2266).
#[test]
fn articulated_children_pivot_on_their_parent() {
    let parent = RigidPart {
        mobility: 0.136_975_75,
        ..crate_at(Vec3::new(0.0, 0.5, 0.0))
    };
    let origin = Vec3::new(0.3, 0.2, 0.0);
    let child = RigidPart {
        parent: Some(0),
        mobility: 0.467_577_55,
        world: Affine3A::from_translation(Vec3::new(0.3, 0.7, 0.0)),
        origin,
        links: vec![RigidLink {
            on_body: Vec3::new(0.13, 0.0, 0.0),
            on_parent: Vec3::new(0.13, 0.0, 0.0),
            slack: 0.0451,
            strength: 1.0,
            smoothing: 0.2,
        }],
        ..crate_at(Vec3::ZERO)
    };
    let mut body = Ragdoll::rigid(&[parent, child]);
    let links = body.links_of(1);
    assert_eq!(links.len(), 2);
    let (stored, pivot) = (&links[0], &links[1]);
    assert!(!stored.origin && pivot.origin);
    assert_eq!((pivot.slack, pivot.strength), (0.0, 1.0));
    assert!((stored.shares.0 - 0.7734).abs() < 1e-4 && (stored.shares.1 - 0.2266).abs() < 1e-4);
    assert_eq!(stored.smoothing, 0.2);
    // The pivot's points meet where the child was placed.
    let (on_child, on_parent) = body.link_points(pivot);
    assert!(on_child.length() < 1e-5 && on_parent.distance(origin) < 1e-5);
    // Woken, the pair falls onto the floor without coming apart.
    let floor = floor_quad(0.0, 5.0);
    body.wake();
    for _ in 0..300 {
        body.tick(floor.as_slice());
    }
    let pivot = &body.links_of(1)[1];
    let gap = body.parts[0]
        .point(&pivot.on_parent)
        .distance(body.parts[1].point(&pivot.on_part));
    assert!(gap < 0.01, "the pivot opened {:.1} cm", gap * 100.0);
    assert!(lowest(&body) > -0.01);
}

/// A level whose second floor, 1 m up, is a rigid body's stand-in.
struct TwoFloors {
    ground: Vec<[Vec3; 3]>,
    stand_in: Vec<[Vec3; 3]>,
}

impl Level for TwoFloors {
    fn pieces(&self, _: Vec3, _: Vec3, out: &mut Vec<u32>) {
        out.extend([0, 1]);
    }

    fn triangles(&self, piece: u32, lo: Vec3, hi: Vec3, out: &mut Vec<[Vec3; 3]>) {
        let tris = if piece == 0 {
            &self.ground
        } else {
            &self.stand_in
        };
        tris.as_slice().triangles(0, lo, hi, out);
    }

    fn movable(&self, piece: u32) -> bool {
        piece == 1
    }
}

/// A rigid body meets only the level: woken over another body's stand-in
/// it falls through it to the ground, while a character's part lands on
/// the stand-in.
#[test]
fn rigid_bodies_meet_only_the_level() {
    let level = TwoFloors {
        ground: floor_quad(0.0, 5.0),
        stand_in: floor_quad(1.0, 5.0),
    };
    let mut prop = Ragdoll::rigid(&[crate_at(Vec3::new(0.0, 1.2, 0.0))]);
    prop.wake();
    let mut limb = Ragdoll::rigid(&[crate_at(Vec3::new(0.0, 1.2, 0.0))]);
    limb.meets_movable = true;
    limb.wake();
    for _ in 0..300 {
        prop.tick(&level);
        limb.tick(&level);
    }
    assert!(lowest(&prop).abs() < 0.01, "prop at {}", lowest(&prop));
    assert!(
        (lowest(&limb) - 1.0).abs() < 0.01,
        "limb at {}",
        lowest(&limb)
    );
}

/// A body that falls below −50 m has fallen out of the world (the original
/// removes its node there); the fast retention holds it under 11.5 m/s
/// (0.1391 m a tick), so it takes 5.5 s.
#[test]
fn a_body_below_minus_fifty_metres_falls_out() {
    let mut body = Ragdoll::rigid(&[crate_at(Vec3::ZERO)]);
    body.wake();
    let mut out = None;
    for tick in 0..2000 {
        body.tick(NONE);
        if body.fell_out() {
            out = Some(tick);
            break;
        }
    }
    let out = out.expect("never fell out");
    // 455 ticks (5.5 s).
    assert!((440..470).contains(&out), "fell out after {out} ticks");
    assert!(body.parts[0].positions[0].y < FALL_OUT_HEIGHT);
}

/// A flat rest tetrahedron is not solid; the shipped ones are.
#[test]
fn flat_rest_points_are_not_solid() {
    assert!(crate_at(Vec3::ZERO).is_solid());
    let mut flat = crate_at(Vec3::ZERO);
    flat.rest[3] = Vec3::new(0.1, 0.0, 0.0);
    assert!(!flat.is_solid());
}

/// Only awake bodies are drawn between ticks: a body registered asleep
/// keeps its node's transform, one in deep sleep is drawn once where its
/// last tick left it and then left alone until it wakes.
#[test]
fn deep_sleepers_are_not_redrawn() {
    let mut app = App::new();
    app.init_resource::<Time<Fixed>>()
        .add_systems(Update, super::super::draw_bodies);
    let at = Vec3::new(0.0, 0.5, 0.0);
    let node = app.world_mut().spawn(Transform::from_translation(at)).id();
    let part = RigidPart {
        entity: node,
        ..crate_at(at)
    };
    let mut body = Ragdoll::rigid(&[part]);
    app.world_mut().entity_mut(node).insert(body.clone());
    let elsewhere = Transform::from_xyz(5.0, 5.0, 5.0);
    app.world_mut().entity_mut(node).insert(elsewhere);
    app.update();
    // Registered asleep and never simulated: not drawn.
    assert_eq!(*app.world().get::<Transform>(node).unwrap(), elsewhere);
    // Woken, it falls onto the floor and sleeps again.
    let floor = floor_quad(0.0, 5.0);
    body.wake();
    while body.sleep != Sleep::Deep {
        body.tick(floor.as_slice());
    }
    app.world_mut().entity_mut(node).insert(body);
    app.update();
    let drawn = *app.world().get::<Transform>(node).unwrap();
    assert!(drawn.translation.y < 0.1, "drawn at {}", drawn.translation);
    // Unchanged since, it is not drawn again.
    app.world_mut().entity_mut(node).insert(elsewhere);
    app.update();
    assert_eq!(*app.world().get::<Transform>(node).unwrap(), elsewhere);
}
