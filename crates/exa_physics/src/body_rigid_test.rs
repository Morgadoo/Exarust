use super::super::tests::floor_quad;
use super::super::{CM, PartSetup, SolverStage, limp_retention};
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
        parent_world: Affine3A::IDENTITY,
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
        mesh: None,
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
    let want = [2.0, 1.0, 0.5, 0.5];
    assert_eq!(gravity_shares([2.0, 4.0, 8.0, 8.0]), want);
    assert_eq!(gravity_shares([0.0, 4.0, 4.0, 4.0]), [1.0; 4]);
    // One integration from rest moves each particle by its share of a
    // tick's gravity (0.142352 cm a tick²).
    let mut part = crate_at(Vec3::ZERO);
    part.point_masses = [2.0, 4.0, 8.0, 8.0];
    let mut body = Ragdoll::rigid(&[part]);
    body.sleep = Sleep::Awake;
    let before = body.parts[0].positions;
    body.integrate();
    for (i, w) in want.into_iter().enumerate() {
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
    assert!(!body.fell_out());
    // It meets other bodies by its mobility (order key 200), in no layer.
    let pairing = p.pairing;
    assert_eq!((pairing.mobility, pairing.key()), (0.5, 200));
    assert_eq!((pairing.group, pairing.layer, pairing.out), (0, 0, false));
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
            flags: 0,
            axis: Vec3::ZERO,
        }],
        ..crate_at(Vec3::ZERO)
    };
    let mut body = Ragdoll::rigid(&[parent, child]);
    // The child never meets its parent: their group masks share a bit.
    assert_ne!(body.parts[0].pairing.group & body.parts[1].pairing.group, 0);
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

/// A character's limp forearm (bone 11, a 24 cm limb) centred at `at`.
fn limb_at(at: Vec3) -> PartSetup {
    PartSetup {
        bone: 11,
        entity: Entity::PLACEHOLDER,
        parent: None,
        world: Affine3A::from_translation(at),
        centre: Vec3::ZERO,
        half: Vec3::new(0.05, 0.12, 0.05),
        origin: Vec3::ZERO,
        links: vec![],
        foot: None,
        pins: Vec::new(),
    }
}

/// Bodies skip a rigid body's stand-ins (they are the character capsules'):
/// woken over one, a prop and a character's limb both fall through it to
/// the ground; they meet the rigid body itself through their pairs.
#[test]
fn bodies_skip_the_stand_ins() {
    let level = TwoFloors {
        ground: floor_quad(0.0, 5.0),
        stand_in: floor_quad(1.0, 5.0),
    };
    let mut prop = Ragdoll::rigid(&[crate_at(Vec3::new(0.0, 1.2, 0.0))]);
    prop.wake();
    let mut limb = Ragdoll::new(&[limb_at(Vec3::new(1.0, 1.3, 0.0))]);
    for _ in 0..300 {
        prop.tick(&level);
        limb.tick(&level);
    }
    assert!(lowest(&prop).abs() < 0.01, "prop at {}", lowest(&prop));
    assert!(lowest(&limb).abs() < 0.01, "limb at {}", lowest(&limb));
}

/// One tick of `bodies` as the game's solver runs it with the pairs: the
/// awake ones solved, the sleepers (a first sleeping tick taken) seen by
/// the pairs.
fn step(bodies: &mut [&mut Ragdoll], level: &[[Vec3; 3]], stages: &mut [Box<dyn SolverStage>]) {
    let (mut awake, mut sleeping): (Vec<&mut Ragdoll>, Vec<&mut Ragdoll>) =
        (Vec::new(), Vec::new());
    for b in bodies.iter_mut() {
        match b.sleep {
            Sleep::Awake => awake.push(&mut **b),
            Sleep::Asleep => {
                b.sleep_tick();
                sleeping.push(&mut **b);
            }
            Sleep::Deep => sleeping.push(&mut **b),
        }
    }
    super::super::solve_among(&mut awake, &mut sleeping, level, stages);
}

fn pairs() -> Vec<Box<dyn SolverStage>> {
    vec![Box::new(super::super::Pairs::default())]
}

/// A 60 cm table of mobility 0.05 on the floor: four leg spheres and a
/// top of 5 × 5 spheres 12 cm across, 10 cm apart, its node at the origin.
fn table() -> RigidPart {
    let mut spheres: Vec<(Vec3, f32)> =
        [(-0.25, -0.25), (0.25, -0.25), (-0.25, 0.25), (0.25, 0.25)]
            .map(|(x, z)| (Vec3::new(x, 0.05, z), 0.05))
            .to_vec();
    spheres.extend((-2..=2).flat_map(|i| {
        (-2..=2).map(move |k| (Vec3::new(0.1 * i as f32, 0.54, 0.1 * k as f32), 0.06))
    }));
    RigidPart {
        rest: [
            Vec3::new(-0.3, 0.0, -0.3),
            Vec3::new(0.3, 0.0, -0.3),
            Vec3::new(0.0, 0.0, 0.3),
            Vec3::new(0.0, 0.6, 0.0),
        ],
        mobility: 0.05,
        spheres,
        ..crate_at(Vec3::ZERO)
    }
}

/// A 10 cm cup of mobility 0.5 (one 5 cm sphere), its node at `at`.
fn cup_at(at: Vec3) -> RigidPart {
    let mut cup = crate_at(at);
    cup.rest = cup.rest.map(|r| r * 0.25);
    cup.spheres = vec![(Vec3::new(0.0, 0.05, 0.0), 0.05)];
    cup
}

/// A cup dropped on a table that is a prop body, both registered asleep:
/// the cup (the more mobile, key 200 to the table's 2 000) wakes the table
/// at its first contact, comes to rest on its top through their pairs, and
/// both sleep again with the table where it stood.
#[test]
fn props_rest_on_prop_bodies() {
    let floor = floor_quad(0.0, 5.0);
    let mut table = Ragdoll::rigid(&[table()]);
    let stood = table.parts[0].transform().translation;
    let mut cup = Ragdoll::rigid(&[cup_at(Vec3::new(0.03, 0.75, 0.02))]);
    cup.wake();
    let mut stages = pairs();
    let mut woke = false;
    for _ in 0..1000 {
        step(&mut [&mut cup, &mut table], floor.as_slice(), &mut stages);
        woke |= table.sleep == Sleep::Awake;
        if cup.asleep() && table.asleep() {
            break;
        }
    }
    assert!(woke, "the table never woke");
    assert!(cup.asleep() && table.asleep());
    assert!(lowest(&cup) > 0.55, "cup at {}", lowest(&cup));
    let moved = table.parts[0].transform().translation.distance(stood);
    assert!(moved < 0.01, "the table moved {moved} m");
}

/// A character's limb thrown at 2 m/s against a prop resting on the floor
/// wakes it and pushes it away through their pairs.
#[test]
fn a_limb_pushes_a_prop() {
    let floor = floor_quad(0.0, 5.0);
    let mut prop = Ragdoll::rigid(&[crate_at(Vec3::ZERO)]);
    let mut limb = Ragdoll::new(&[limb_at(Vec3::new(-0.45, 0.2, 0.0))]);
    limb.set_velocity(Vec3::new(2.0, 0.0, 0.0));
    let mut stages = pairs();
    let mut woke = false;
    for _ in 0..60 {
        step(&mut [&mut limb, &mut prop], floor.as_slice(), &mut stages);
        woke |= prop.sleep == Sleep::Awake;
    }
    assert!(woke, "the prop never woke");
    let pushed = prop.parts[0].transform().translation.x;
    assert!(pushed > 0.02, "pushed {pushed} m");
    assert!(limb.parts[0].transform().translation.x < pushed);
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

/// The settling state: for its ticks the body takes a tick's gravity with
/// no inertia (the same 0.142352 cm each tick); the tick after, its
/// velocity is the last tick's move alone (one tick of gravity, kept by
/// the retention of that speed) and gravity adds to it.
#[test]
fn a_settling_body_falls_without_inertia() {
    let mut body = Ragdoll::rigid(&[crate_at(Vec3::ZERO)]);
    body.wake();
    body.settle_for(3);
    assert_eq!(body.settling(), 3);
    let mut last = body.parts[0].positions[0];
    let mut step = 0.0;
    for tick in 0..3 {
        body.tick(NONE);
        let drop = last.y - body.parts[0].positions[0].y;
        assert!(
            (drop - 0.001_423_5).abs() < 2e-7,
            "tick {tick} dropped {:.8} m",
            drop
        );
        assert_eq!(body.settling(), 2 - tick);
        last = body.parts[0].positions[0];
        step = drop;
    }
    body.tick(NONE);
    let drop = last.y - body.parts[0].positions[0].y;
    let keep = limp_retention(step * CM);
    assert!(
        (drop - (step * keep + step)).abs() < 2e-7,
        "dropped {drop:.8} m, kept {keep}"
    );
}

/// A weighted push moves each particle by the offset times its weight and,
/// with the restore flag, adds the offset's length to the strain: past
/// 0.1 cm the six edge lengths are restored. Without the flag the strain
/// is left alone.
#[test]
fn a_weighted_push_restores_the_edges_past_a_tenth_of_a_centimetre() {
    let mut body = Ragdoll::rigid(&[crate_at(Vec3::ZERO)]);
    body.wake();
    let before = body.parts[0].positions;
    body.push_weighted(Vec3::new(0.0, 0.0004, 0.0), [1.0, 0.5, 0.0, 2.0], false);
    let after = body.parts[0].positions;
    for (i, w) in [1.0f32, 0.5, 0.0, 2.0].into_iter().enumerate() {
        assert_eq!(after[i].y, before[i].y + w * 0.0004);
    }
    assert_eq!(body.parts[0].strain, 0.0);
    // A whole-body move keeps the edges, so their restore moves nothing.
    let mut body = Ragdoll::rigid(&[crate_at(Vec3::ZERO)]);
    body.wake();
    let x = body.parts[0].positions[0].x;
    body.push_weighted(Vec3::new(0.01, 0.0, 0.0), [1.0; 4], true);
    assert_eq!(body.parts[0].positions[0].x, x + 0.01);
    // 1 cm is past the 0.1 cm budget: the edges were restored and the
    // strain reset.
    assert_eq!(body.parts[0].strain, 0.0);
    body.push_weighted(Vec3::new(0.0005, 0.0, 0.0), [1.0; 4], true);
    assert!((body.parts[0].strain - 0.0005).abs() < 1e-9);
    body.push_weighted(Vec3::new(0.0005, 0.0, 0.0), [1.0; 4], true);
    // Exactly the 0.1 cm budget is not past it.
    assert!((body.parts[0].strain - 0.001).abs() < 1e-9);
    body.push_weighted(Vec3::new(0.0001, 0.0, 0.0), [1.0; 4], true);
    assert_eq!(body.parts[0].strain, 0.0);
}

/// A translate moves every particle set with the body, so it adds no
/// velocity and no strain.
#[test]
fn a_translate_moves_the_body_without_velocity() {
    let mut body = Ragdoll::rigid(&[crate_at(Vec3::ZERO)]);
    body.wake();
    let before = body.parts[0].positions;
    let offset = Vec3::new(0.3, 0.1, -0.2);
    body.translate(offset);
    for (i, &b) in before.iter().enumerate() {
        assert_eq!(body.parts[0].positions[i], b + offset);
        assert_eq!(body.parts[0].previous[i], b + offset);
        assert_eq!(body.parts[0].start[i], b + offset);
    }
    assert_eq!(body.parts[0].strain, 0.0);
}

/// Placing a body at a matrix puts its particles at the matrix times its
/// rest points, at rest; its contacts are left for the next tick.
#[test]
fn placing_a_body_sets_it_at_rest() {
    let mut body = Ragdoll::rigid(&[crate_at(Vec3::ZERO)]);
    body.wake();
    body.shove(0, Vec3::new(1.0, 0.0, 0.0), 0.0);
    let at = Affine3A::from_translation(Vec3::new(2.0, 3.0, 4.0));
    body.place_at(&[at]);
    let p = &body.parts[0];
    assert_eq!(p.positions, p.rest.map(|r| at.transform_point3(r)));
    assert_eq!(p.previous, p.positions);
    assert_eq!(p.start, p.positions);
    let placed = p.positions;
    body.tick(NONE);
    // No velocity is left over: the next tick only adds gravity.
    assert!(body.parts[0].positions[0].y < placed[0].y + 1e-9);
    assert!((placed[0].y - body.parts[0].positions[0].y - 0.001_423_5).abs() < 2e-6);
}

/// The hand's drag of a resting crate (mobility 0.5, strength 1, the
/// tick's first two passes) toward an anchor 1 m away across the floor,
/// held by its middle: the floor holds its bottom back, so it tips onto its
/// side as it goes, never more than 10 cm a tick (two passes of at most
/// twice the 2.12 cm cap), and stops with the held point by the anchor,
/// still on the floor; let go, it settles and sleeps.
#[test]
fn a_grabbed_crate_slides_to_its_anchor() {
    let floor = floor_quad(0.0, 5.0);
    let mut body = Ragdoll::rigid(&[crate_at(Vec3::new(0.0, 0.0, 0.0))]);
    body.wake();
    while body.sleep != Sleep::Deep {
        body.tick(floor.as_slice());
    }
    let at = body.parts[0]
        .transform()
        .transform_point3(Vec3::new(0.0, 0.2, 0.0));
    assert!(body.grab(0, at, 1.0, Some(1)));
    let anchor = at + Vec3::new(1.0, 0.0, 0.0);
    body.pull_grab_to(anchor);
    let mut last = body.grab_point().unwrap();
    let mut fastest: f32 = 0.0;
    for _ in 0..300 {
        body.wake_to(256);
        body.tick(floor.as_slice());
        let now = body.grab_point().unwrap();
        fastest = fastest.max((now - last).xz().length());
        last = now;
    }
    assert!(fastest > 0.005 && fastest < 0.1, "{fastest} m a tick");
    assert!((last - anchor).xz().length() < 0.05, "stopped at {last}");
    assert!(lowest(&body).abs() < 0.02, "lifted {}", lowest(&body));
    assert!(body.release_grab());
    assert_eq!(body.grab_point(), None);
    for _ in 0..600 {
        body.tick(floor.as_slice());
    }
    assert!(body.asleep());
    assert!(lowest(&body).abs() < 0.02, "rests at {}", lowest(&body));
}

/// A crate hung from the world by one pin at its centre point, anchored at
/// the node's origin: the pin holds the point near the anchor, the body
/// never meets the level, and the origin pivot is added unless the first
/// link is a slider.
fn hung_crate(at: Vec3, anchor: Vec3, link: RigidLink) -> RigidPart {
    let mut part = crate_at(at);
    part.parent_world = Affine3A::from_translation(anchor);
    part.links = vec![link];
    part
}

fn pin(on_body: Vec3) -> RigidLink {
    RigidLink {
        on_body,
        on_parent: Vec3::ZERO,
        slack: 0.0,
        strength: 1.0,
        smoothing: 0.0,
        flags: 0,
        axis: Vec3::ZERO,
    }
}

/// A hung body's root carries a world link and an origin pivot, and no
/// level detection; a slider's first link suppresses the pivot.
#[test]
fn a_hung_body_anchors_to_its_node() {
    let at = Vec3::new(1.0, 2.8, 3.0);
    let anchor = Vec3::new(1.0, 2.5, 3.0);
    let part = hung_crate(at, anchor, pin(Vec3::new(0.0, 0.2, 0.0)));
    let body = Ragdoll::rigid(&[part]);
    assert!(body.parts[0].no_level);
    assert_eq!(body.world.len(), 2);
    let link = &body.world[0];
    assert_eq!(link.anchor, anchor);
    assert!(!link.origin);
    let pivot = &body.world[1];
    assert!(pivot.origin);
    assert_eq!((pivot.slack, pivot.strength), (0.0, 1.0));
    assert_eq!(pivot.anchor, anchor);
    // The pin's point is the crate's own centre point.
    assert_eq!(
        link.on_part,
        super::super::weights(&body.parts[0], Vec3::new(0.0, 0.2, 0.0))
    );

    let slider = RigidLink {
        flags: world_link::SLIDER,
        slack: 1.0,
        axis: Vec3::Y,
        on_body: Vec3::new(0.0, 0.2, 0.0),
        on_parent: Vec3::ZERO,
        strength: 1.0,
        smoothing: 0.01,
    };
    let part = hung_crate(at, anchor, slider);
    let body = Ragdoll::rigid(&[part]);
    assert_eq!(body.world.len(), 1);
    assert_eq!(body.world[0].flags & world_link::SLIDER, world_link::SLIDER);
    assert!(
        (body.world[0].length - 0.5).abs() < 1e-5,
        "{}",
        body.world[0].length
    );
}

/// A pinned crate swings about its anchor and stays there: the world link
/// removes gravity's fall every iteration.
#[test]
fn a_hung_body_hangs_from_its_pin() {
    // The node's origin is the crate's origin; the stored link's parent
    // point puts its anchor at the crate's centre point, so both links are
    // satisfied where the body is placed.
    let at = Vec3::new(0.0, 1.8, 0.0);
    let anchor = at;
    let mut link = pin(Vec3::new(0.0, 0.2, 0.0));
    link.on_parent = Vec3::new(0.0, 0.2, 0.0);
    let mut body = Ragdoll::rigid(&[hung_crate(at, anchor, link)]);
    body.wake();
    let point = |body: &Ragdoll| {
        let p = &body.parts[0];
        (0..4)
            .map(|i| p.positions[i] * body.world[0].on_part[i])
            .sum::<Vec3>()
    };
    let held = Vec3::new(0.0, 2.0, 0.0);
    assert!(point(&body).distance(held) < 1e-3);
    for _ in 0..200 {
        body.tick(NONE);
    }
    let gap = point(&body).distance(held);
    assert!(gap < 0.01, "the pin opened {gap} m");
    assert!(!body.fell_out());
}

/// A hung body passes through the level: its root skips detection while
/// its pinned point holds it below the floor.
#[test]
fn a_hung_body_skips_the_level() {
    let floor = floor_quad(0.0, 5.0);
    // The anchor is below the floor; the crate hangs at it.
    let at = Vec3::new(0.0, -0.7, 0.0);
    let anchor = Vec3::new(0.0, -0.5, 0.0);
    let mut body = Ragdoll::rigid(&[hung_crate(at, anchor, pin(Vec3::new(0.0, 0.2, 0.0)))]);
    body.wake();
    for _ in 0..50 {
        body.tick(&floor[..]);
    }
    assert!(body.parts[0].contacts.is_empty());
    assert!(
        lowest(&body) < -0.1,
        "the floor stopped it at {}",
        lowest(&body)
    );
}

/// A slider holds its point on the axis and lets it slide to the end of its
/// travel: under gravity down a downward axis the fraction reaches 1.
#[test]
fn a_world_slider_slides_to_its_travel() {
    let anchor = Vec3::ZERO;
    let slider = RigidLink {
        on_body: Vec3::new(0.0, 0.2, 0.0),
        on_parent: Vec3::ZERO,
        slack: 1.0,
        strength: 1.0,
        smoothing: 0.01,
        flags: world_link::SLIDER,
        axis: Vec3::NEG_Y,
    };
    let at = Vec3::new(0.0, -0.2, 0.0);
    let mut body = Ragdoll::rigid(&[hung_crate(at, anchor, slider)]);
    body.wake();
    let point = |body: &Ragdoll| {
        let p = &body.parts[0];
        (0..4)
            .map(|i| p.positions[i] * body.world[0].on_part[i])
            .sum::<Vec3>()
    };
    for _ in 0..300 {
        body.tick(NONE);
    }
    let t = body.world[0].length;
    assert!((t - 1.0).abs() < 1e-4, "slider at {t}");
    let there = point(&body);
    assert!(
        (there - Vec3::new(0.0, -1.0, 0.0)).length() < 0.05,
        "point at {there}"
    );
}
