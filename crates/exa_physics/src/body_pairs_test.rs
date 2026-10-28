//! Contacts between bodies, pinned to the original's rules
//! (`context/re/body-pairs.md` §3–§11): group and layer masks, the order
//! key, the immediate push with its mobility shares and φ, the pair pass's
//! slop and reversed friction, the pair friction, dropping a satisfied
//! pair, and the wake of a sleeper.
use bevy::math::Affine3A;

use super::super::tests::floor_quad;
use super::super::{BodyPlugin, PartSetup, Ragdoll, Sleep, SolverStage, SolverStages, WAKE_COUNT};
use super::*;
use crate::sector::Bounds;

const NONE: &[[Vec3; 3]] = &[];

fn entity(i: u32) -> Entity {
    Entity::from_raw_u32(i).unwrap()
}

/// A part of `bone` at `at`, its box `half` about its origin, under
/// `parent`.
fn setup(id: u32, bone: usize, parent: Option<usize>, at: Vec3, half: Vec3) -> PartSetup {
    PartSetup {
        bone,
        entity: entity(id),
        parent,
        world: Affine3A::from_translation(at),
        centre: Vec3::ZERO,
        half,
        origin: Vec3::ZERO,
        links: vec![],
        foot: None,
        pins: Vec::new(),
        spheres: None,
    }
}

/// A limp body of one ball-shaped part of `bone` (one sphere of radius
/// `r`, at least 6 cm) centred at `at`.
fn ball(id: u32, bone: usize, at: Vec3, r: f32) -> Ragdoll {
    Ragdoll::new(&[setup(id, bone, None, at, Vec3::splat(r))])
}

/// The centre of a ball's sphere.
fn centre(b: &Ragdoll) -> Vec3 {
    b.parts[0]
        .transform()
        .transform_point3(b.parts[0].spheres[0].0)
}

/// One tick of `bodies` as the game's solver runs it: the awake ones
/// solved, the sleeping ones (a first sleeping tick taken) seen by the
/// stages.
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
    solve_among(&mut awake, &mut sleeping, level, stages);
}

fn pairs() -> Vec<Box<dyn SolverStage>> {
    vec![Box::new(Pairs::default())]
}

fn the_pairs(stages: &[Box<dyn SolverStage>]) -> &Pairs {
    (&*stages[0] as &dyn std::any::Any)
        .downcast_ref::<Pairs>()
        .unwrap()
}

use super::super::solve_among;

/// Two limbs thrown at each other at 2 m/s each stop at their spheres
/// (10 cm each) and part again; without the pairs they pass through each
/// other.
#[test]
fn two_limbs_stop_at_their_spheres() {
    let run = |with_pairs: bool| {
        let mut a = ball(1, 11, Vec3::new(-0.3, 2.0, 0.0), 0.1);
        let mut b = ball(2, 11, Vec3::new(0.3, 2.0, 0.0), 0.1);
        a.set_velocity(Vec3::X * 2.0);
        b.set_velocity(Vec3::NEG_X * 2.0);
        let mut stages = if with_pairs { pairs() } else { Vec::new() };
        let (mut gaps, mut met) = (Vec::new(), None);
        for tick in 0..30 {
            step(&mut [&mut a, &mut b], NONE, &mut stages);
            gaps.push(centre(&a).distance(centre(&b)));
            if with_pairs && met.is_none() && !the_pairs(&stages).touches().is_empty() {
                met = Some(tick);
            }
        }
        (gaps, met)
    };
    let (free, _) = run(false);
    assert!(free.iter().any(|&d| d < 0.05), "{free:?}");
    let (gaps, met) = run(true);
    // They meet in the ninth tick, 4.6 cm closer each tick: the push
    // leaves them 0.6 cm in (it moves their contact points by their
    // weights, not their centres), and they part from the next.
    let first = met.unwrap();
    assert_eq!(first, 8);
    assert!(gaps[first] > 0.19 && gaps[first] < 0.2, "{gaps:?}");
    for w in gaps[first..].windows(2) {
        assert!(w[1] > w[0] && w[1] > 0.199, "{gaps:?}");
    }
}

/// Two limbs meeting head-on at 2 m/s each strike each other: their
/// relative smoothed velocity at the hit (2.8 cm a tick against the 4.8 they
/// were thrown at, as the solver damps it and the impact reads a tenth of
/// the way to each tick's motion) squared, times the cosine against the
/// push, over 2·m + 1 of the other's contact mobility m. At 0.2 m/s each
/// the squared travel is under 1 cm² and neither is struck.
#[test]
fn limbs_meeting_fast_strike_each_other() {
    let first_touch = |speed: f32| {
        let mut a = ball(1, 11, Vec3::new(-0.3, 2.0, 0.0), 0.1);
        let mut b = ball(2, 11, Vec3::new(0.3, 2.0, 0.0), 0.1);
        a.set_velocity(Vec3::X * speed);
        b.set_velocity(Vec3::NEG_X * speed);
        let mobility = a.parts[0].pairing.mobility;
        let mut stages = pairs();
        for _ in 0..400 {
            step(&mut [&mut a, &mut b], NONE, &mut stages);
            if let Some(t) = the_pairs(&stages).touches().first() {
                return (*t, mobility);
            }
        }
        panic!("the two balls never touched");
    };
    let (fast, m) = first_touch(2.0);
    assert!(fast.travel.x > 2.5 && fast.travel.x < 3.2, "{fast:?}");
    let impact = fast.impact().unwrap();
    assert!((impact - fast.travel2).abs() < 0.01 * impact, "{fast:?}");
    let each = impact / (m * 2.0 + 1.0);
    assert_eq!(fast.struck, [Some(each), Some(each)]);
    let (slow, _) = first_touch(0.2);
    assert!(slow.travel2 < STRIKE_TRAVEL2, "{slow:?}");
    assert_eq!(slow.impact(), None);
    assert_eq!(slow.struck, [None, None]);
}

/// A limb thrown at a resting one strikes it, but not one that is placed
/// (settling): the original's pair with exactly one placed body goes
/// through its placed callback, which counts no hits, so neither side takes
/// an impact.
#[test]
fn a_placed_body_takes_no_impact() {
    let first_touch = |placed: bool| {
        let mut a = ball(1, 11, Vec3::new(-0.3, 2.0, 0.0), 0.1);
        let mut b = ball(2, 11, Vec3::new(0.3, 2.0, 0.0), 0.1);
        if placed {
            a.settle_for(100);
        }
        b.set_velocity(Vec3::NEG_X * 4.0);
        let mut stages = pairs();
        for _ in 0..400 {
            step(&mut [&mut a, &mut b], NONE, &mut stages);
            if let Some(t) = the_pairs(&stages).touches().first() {
                return *t;
            }
        }
        panic!("the two balls never touched");
    };
    let free = first_touch(false);
    assert!(free.struck.iter().all(Option::is_some), "{free:?}");
    let placed = first_touch(true);
    // The placed callback records the contact without counting impact hits.
    assert!(placed.met, "{placed:?}");
    assert_eq!(placed.hits, 0);
    assert_eq!(placed.struck, [None, None]);
}

/// A pair whose push or travel has almost no length takes the original's
/// 0 cosine when the product of the squared lengths is at most 1e-8, which
/// leaves no impact; a longer push gives the true cosine.
#[test]
fn a_degenerate_pair_cosine_is_zero() {
    let mut t = Touch {
        a: (entity(1), 0),
        b: (entity(2), 0),
        met: true,
        hits: 1,
        point: Vec3::ZERO,
        push: Vec3::ZERO,
        deepest: 0.0,
        travel: Vec3::new(-3.0, 0.0, 0.0),
        travel2: 9.0,
        struck: [None; 2],
        hit_event: false,
    };
    assert_eq!(t.impact(), Some(0.0));
    // The push in cm: 9 × 1e-10 is under 1e-8, still 0.
    t.push = Vec3::X * 1e-7;
    assert_eq!(t.impact(), Some(0.0));
    // The push in cm: 9 × 1e-4 is over it, so the cosine is −1 and the
    // impact is the travel².
    t.push = Vec3::X * 0.01;
    assert_eq!(t.impact(), Some(9.0));
    t.b.0 = t.a.0;
    assert_eq!(t.impact(), None);
    // A hit-event pair takes no impact.
    t.b.0 = entity(2);
    t.hit_event = true;
    assert_eq!(t.impact(), None);
}

/// A striker (bit 0x10000) against a hittable limb takes the hit event;
/// two hittable limbs, or a striker against a non-hittable body, take the
/// impact path.
#[test]
fn a_striker_on_a_limb_takes_the_hit_event() {
    use crate::part::PartState;
    let limb = PartState(PartState::LIMB | PartState::DYNAMIC);
    let striker = PartState(PartState::HELD | PartState::STRIKE_CHAIN);
    let held = PartState(PartState::HELD);
    let striker_limb = PartState(PartState::LIMB | PartState::STRIKE_CHAIN);
    assert!(hittable(limb) && hittable(held) && hittable(striker));
    assert!(hit_event(striker, limb));
    assert!(hit_event(limb, striker_limb));
    assert!(!hit_event(limb, held));
    assert!(!hit_event(
        striker,
        PartState(PartState::DYNAMIC | PartState::HAS_CONTACTS)
    ));
}

/// Bodies in pairs keep the original's interleaving with the pair passes
/// while the others run their iterations and friction together in
/// parallel: with two balls colliding and three falling on a floor, every
/// particle ends each tick exactly where it ends when a stage forces every
/// body through the interleaved path.
#[test]
fn free_bodies_in_parallel_leave_the_pairs_unchanged() {
    struct Every;
    impl SolverStage for Every {}
    let floor = floor_quad(0.0, 5.0);
    let scene = || {
        let mut a = ball(1, 11, Vec3::new(-0.3, 0.6, 0.0), 0.1);
        let mut b = ball(2, 11, Vec3::new(0.3, 0.6, 0.0), 0.1);
        a.set_velocity(Vec3::X * 2.0);
        b.set_velocity(Vec3::NEG_X * 2.0);
        let mut bodies = vec![a, b];
        for i in 0..3 {
            bodies.push(ball(10 + i, 11, Vec3::new(1.0 + i as f32, 0.5, 1.0), 0.1));
        }
        bodies
    };
    let (mut split, mut joined) = (scene(), scene());
    let mut alone = pairs();
    let mut together: Vec<Box<dyn SolverStage>> = vec![Box::new(Pairs::default()), Box::new(Every)];
    let mut met = false;
    for _ in 0..60 {
        let mut x: Vec<&mut Ragdoll> = split.iter_mut().collect();
        step(&mut x, floor.as_slice(), &mut alone);
        let mut y: Vec<&mut Ragdoll> = joined.iter_mut().collect();
        step(&mut y, floor.as_slice(), &mut together);
        met |= !the_pairs(&alone).touches().is_empty();
        for (p, q) in split.iter().zip(&joined) {
            for (u, v) in p.parts.iter().zip(&q.parts) {
                assert_eq!(u.positions, v.positions);
                assert_eq!(u.previous, v.previous);
            }
        }
    }
    assert!(met, "the two balls never touched");
}

/// The group masks of a human's 18 parts give the original's
/// self-collision matrix: legs, pelvis, spine and head never meet each
/// other; the lower body meets the arms but not the shoulders; the lumbar
/// meets the forearms and hands, the thorax and head the hands; the two
/// humeri and the two forearms skip each other; a shoulder, humerus or
/// forearm meets only the other hand; the hands meet each other. 13 parts
/// have children, so 13 bits are given.
#[test]
fn the_self_collision_matrix() {
    // Bone: parent bone. 1 pelvis; 2–4 right femur, shin, foot; 5–7 left;
    // 8 lumbar, 9 thorax, 10 head; 11–13 right shoulder, humerus, forearm;
    // 14–16 left; 17, 18 the hands.
    let parent = [0, 1, 2, 3, 1, 5, 6, 1, 8, 9, 9, 11, 12, 9, 14, 15, 13, 16];
    let setups: Vec<PartSetup> = (1..=18)
        .map(|bone| {
            let mut s = setup(bone as u32, bone, None, Vec3::ZERO, Vec3::splat(0.1));
            s.parent = (parent[bone - 1] > 0).then(|| parent[bone - 1] - 1);
            if matches!(bone, 3 | 6) {
                s.foot = Some(Vec3::NEG_Y * 0.4);
            }
            s
        })
        .collect();
    let body = Ragdoll::new(&setups);
    let group = |bone: usize| body.parts[bone - 1].pairing.group;
    let bits = (1..=18).fold(0u32, |m, b| m | group(b));
    assert_eq!(bits.count_ones(), 13);
    let lower = [1, 2, 3, 4, 5, 6, 7];
    let meets = |a: usize, b: usize| -> bool {
        let arm = |x: usize, y: usize| matches!((x, y), (12 | 13 | 17 | 15 | 16 | 18, _));
        let (lo, hi) = (a.min(b), a.max(b));
        let pair = |x: usize, y: usize| (lo, hi) == (x.min(y), x.max(y));
        if lower.contains(&a) || lower.contains(&b) {
            let other = if lower.contains(&a) { b } else { a };
            return !lower.contains(&other) && arm(other, 0);
        }
        // The lumbar: forearms and hands.
        if a == 8 || b == 8 {
            let other = a + b - 8;
            return matches!(other, 13 | 16 | 17 | 18);
        }
        // The thorax and the head: the hands.
        if matches!(a, 9 | 10) || matches!(b, 9 | 10) {
            let other = if matches!(a, 9 | 10) { b } else { a };
            return matches!(other, 17 | 18);
        }
        // Each arm meets only the other hand; the hands meet.
        let right = [11, 12, 13, 17];
        let left = [14, 15, 16, 18];
        pair(17, 18)
            || (right.contains(&a) && b == 18)
            || (right.contains(&b) && a == 18)
            || (left.contains(&a) && b == 17)
            || (left.contains(&b) && a == 17)
    };
    for a in 1..=18 {
        for b in a + 1..=18 {
            assert_eq!(group(a) & group(b) == 0, meets(a, b), "bones {a} and {b}");
        }
    }
    // The feet share their masks.
    assert_eq!(group(4), group(7));
}

/// Layer masks: in a channel both members and either exclusive skip each
/// other; bits 0–3 make no channel.
#[test]
fn layer_masks_keep_channels_apart() {
    for (a, b, apart) in [
        (0x300, 0x300, true),
        (0x300, 0, false),
        (0x300, 0x100, true),
        (0x100, 0x100, false),
        (0x30, 0x10, true),
        (0x10, 0x10, false),
        (0x40, 0xC0, true),
        (0x40, 0x40, false),
        (0x4000, 0xC000, true),
        (0xF, 0xF, false),
        (0x330, 0x300, true),
        (0x30, 0x300, false),
    ] {
        assert_eq!(layers_apart(a, b), apart, "{a:#x} {b:#x}");
        assert_eq!(layers_apart(b, a), apart, "{b:#x} {a:#x}");
    }
}

/// The order key is 100 over the mobility, rounded half to even: a
/// human pelvis (0.45 × 2/3 = 0.3) 333, a hand (2.5 × 2/3) 60; a frozen
/// body's 0.
#[test]
fn the_order_key_is_a_hundred_over_the_mobility() {
    let pelvis = ball(1, 1, Vec3::ZERO, 0.1);
    let hand = ball(2, 17, Vec3::ZERO, 0.1);
    assert!((pelvis.parts[0].pairing.mobility - 0.3).abs() < 1e-6);
    assert_eq!(pelvis.parts[0].pairing.key(), 333);
    assert_eq!(hand.parts[0].pairing.key(), 60);
    let mut p = pelvis.parts[0].pairing;
    p.mobility = 40.0;
    assert_eq!(p.key(), 2);
    p.mobility = 0.0;
    assert_eq!(p.key(), 0);
}

/// A hit on one particle of A against B's centroid: φ_A = 1, φ_B = 4, so
/// f = 0.4 and A takes its share × 1.6 of the push on that particle while
/// each of B's particles takes its share × 0.4 back; the two parts'
/// momentum (mass 1/mobility, spread evenly) is kept.
#[test]
fn the_immediate_push_keeps_momentum() {
    assert_eq!(
        spread([1.0, 0.0, 0.0, 0.0]),
        Some(([1.0, 0.0, 0.0, 0.0], 1.0))
    );
    assert_eq!(spread([0.25; 4]), Some(([1.0; 4], 4.0)));
    assert_eq!(spread([-1.0, 0.0, 0.0, 0.0]), None);
    let mut a = ball(1, 1, Vec3::ZERO, 0.1);
    let mut b = ball(2, 17, Vec3::X, 0.1);
    let (a0, b0) = (a.parts[0].positions, b.parts[0].positions);
    let push = Vec3::new(0.0005, 0.0, 0.0);
    push_apart(
        &mut a.parts[0],
        &mut b.parts[0],
        [1.0, 0.0, 0.0, 0.0],
        [0.25; 4],
        push,
    );
    let (ma, mb) = (0.3_f32, 2.5_f32 * 2.0 / 3.0);
    let (sa, sb) = (ma / (ma + mb), mb / (ma + mb));
    let moved =
        |p: &[Vec3; 4], q: &[Vec3; 4]| -> [Vec3; 4] { std::array::from_fn(|i| p[i] - q[i]) };
    let da = moved(&a.parts[0].positions, &a0);
    let db = moved(&b.parts[0].positions, &b0);
    assert!((da[0].x - 0.0005 * sa * 1.6).abs() < 1e-7, "{da:?}");
    assert!(da[1..].iter().all(|d| *d == Vec3::ZERO));
    for d in db {
        assert!((d.x + 0.0005 * sb * 0.4).abs() < 1e-7, "{db:?}");
    }
    let momentum = |d: &[Vec3; 4], m: f32| d.iter().sum::<Vec3>() / 4.0 / m;
    let total = momentum(&da, ma) + momentum(&db, mb);
    assert!(total.length() < 1e-7, "{total}");
}

/// Two balls at rest, A's sphere side, `depth` (m) into each other along x,
/// with a pair kept for the centroid of each: the point's depth along +x
/// is `depth`, and A slipped `slip` (m) this tick.
fn kept_pair(depth: f32, slip: Vec3, up: bool) -> (Ragdoll, Ragdoll, Pair) {
    let normal = if up { Vec3::Y } else { Vec3::X };
    let mut a = ball(1, 11, Vec3::ZERO, 0.1);
    let b = ball(2, 11, normal * depth, 0.1);
    for p in &mut a.parts[0].start {
        *p -= slip;
    }
    let point = Point {
        a: [0.25; 4],
        b: [0.25; 4],
        normal,
    };
    let pair = Pair {
        a: Side {
            key: entity(1),
            body: 0,
            part: 0,
        },
        b: Side {
            key: entity(2),
            body: 1,
            part: 0,
        },
        points: [point; POINTS],
        count: 1,
        live: true,
    };
    (a, b, pair)
}

/// How far each of A's particles moved across the normal, in cm.
fn across_cm(before: &Ragdoll, after: &Ragdoll) -> f32 {
    (after.parts[0].positions[0] - before.parts[0].positions[0]).z * 100.0
}

/// The pair pass takes a point over 0.01 cm deep back to 0.01 cm, and its
/// friction is reversed: limbs side by side (n_y = 0, μ = 2.5 × 2.5) have
/// a limit of 0.0625 cm², so a slip of 0.2 cm loses only 0.000625 of
/// itself while one of 0.3 cm is cancelled; one on top of the other
/// (n_y = 1) has 6.25 cm², 2.4 cm kept and 2.6 cm cancelled. A point
/// within the slop needs nothing.
#[test]
fn the_pair_pass_keeps_a_slop_and_reverses_friction() {
    // Equal mobilities: each side takes half, spread over four particles
    // at a quarter each.
    let share = 0.5 * 0.25;
    let solve = |depth_cm: f32, slip_cm: f32, up: bool| {
        let (mut a, mut b, pair) = kept_pair(depth_cm / 100.0, Vec3::Z * slip_cm / 100.0, up);
        let before = a.clone();
        let work = solve_pair(&mut a.parts[0], &mut b.parts[0], &pair.points[..1]);
        (work, before, a)
    };
    let (work, before, after) = solve(0.51, 0.2, false);
    assert!(work);
    let normal_cm = (after.parts[0].positions[0] - before.parts[0].positions[0]).x * 100.0;
    assert!((normal_cm - share * 0.5).abs() < 1e-5, "{normal_cm}");
    let kept = across_cm(&before, &after);
    assert!((kept + share * 0.000625 * 0.2).abs() < 1e-7, "{kept}");
    let (_, before, after) = solve(0.51, 0.3, false);
    assert!((across_cm(&before, &after) + share * 0.3).abs() < 1e-5);
    let (_, before, after) = solve(0.51, 2.4, true);
    assert!((across_cm(&before, &after) + share * 0.01 * 6.25 * 2.4).abs() < 1e-5);
    let (_, before, after) = solve(0.51, 2.6, true);
    assert!((across_cm(&before, &after) + share * 2.6).abs() < 1e-5);
    let (work, before, after) = solve(0.009, 0.3, false);
    assert!(!work);
    assert_eq!(before.parts[0].positions, after.parts[0].positions);
}

/// The pair friction in the 8 passes is the plain rule: a slip under the
/// limit (0.2 cm, under √0.0625) is cancelled, one over it (0.3 cm) loses
/// only 0.000625 of itself, each side moving toward the other's slip by
/// its share over its raw weights.
#[test]
fn the_pair_friction_sticks_below_its_limit() {
    let share = 0.5 * 0.25;
    for (slip_cm, want) in [(0.2, -share * 0.2), (0.3, -share * 0.3 * 0.000625)] {
        let (mut a, mut b, pair) = kept_pair(0.0, Vec3::Z * slip_cm / 100.0, false);
        let before = a.clone();
        rub_pair(&mut a.parts[0], &mut b.parts[0], &pair);
        let moved = across_cm(&before, &a);
        assert!((moved - want).abs() < 1e-7, "{slip_cm}: {moved}");
    }
}

/// Pairs whose friction product is under 1.19e-7 do not rub: one limb on
/// another (n_y = 1, so the limit is the product itself) keeps a slip of
/// 3e-4 cm (9e-8 cm², under either limit) with a product of 1.1e-7, and
/// has it cancelled with 1.2e-7. A product exactly at the epsilon rubs,
/// and so does a NaN one (the original's compare takes the parity branch).
#[test]
fn pairs_with_almost_no_friction_do_not_rub() {
    for (mu, rubs) in [(1.1e-7_f32, false), (1.2e-7, true), (MIN_FRICTION, true)] {
        let (mut a, mut b, pair) = kept_pair(0.0, Vec3::Z * 3e-6, true);
        a.parts[0].friction = 1.0;
        b.parts[0].friction = mu;
        let before = a.clone();
        rub_pair(&mut a.parts[0], &mut b.parts[0], &pair);
        let moved = across_cm(&before, &a);
        let want = if rubs { -0.5 * 0.25 * 3e-4 } else { 0.0 };
        assert!((moved - want).abs() < 1e-7, "{mu}: {moved}");
    }
}

/// A pair whose points need nothing in an iteration (its parts moved
/// apart) is dropped for the rest of the tick: pushed back into each
/// other, they are not corrected until the next detection.
#[test]
fn a_pair_is_dropped_once_satisfied() {
    let mut a = ball(1, 11, Vec3::new(0.0, 2.0, 0.0), 0.1);
    let mut b = ball(2, 11, Vec3::new(0.19, 2.0, 0.0), 0.1);
    let mut stages = pairs();
    stages[0].detect(&mut [&mut a, &mut b], &mut []);
    assert_eq!(the_pairs(&stages).active(), 1);
    assert_eq!(the_pairs(&stages).touches().len(), 1);
    // Still in by more than the slop: corrected, kept.
    stages[0].pair_pass(&mut [&mut a, &mut b], 1);
    assert_eq!(the_pairs(&stages).active(), 1);
    let shift = |b: &mut Ragdoll, dx: f32| {
        for p in &mut b.parts[0].positions {
            p.x += dx;
        }
    };
    shift(&mut b, 0.05);
    stages[0].pair_pass(&mut [&mut a, &mut b], 2);
    assert_eq!(the_pairs(&stages).active(), 0);
    shift(&mut b, -0.1);
    let before = (a.parts[0].positions, b.parts[0].positions);
    for i in 3..=20 {
        stages[0].pair_pass(&mut [&mut a, &mut b], i);
    }
    assert_eq!(before, (a.parts[0].positions, b.parts[0].positions));
    assert!(centre(&a).distance(centre(&b)) < 0.15);
    stages[0].iterated(&mut [&mut a, &mut b]);
    assert_eq!(the_pairs(&stages).points(), 0);
}

/// A body that touches a sleeper wakes it: the sleeper takes the awake
/// body's countdown and drops its level contacts; a contact between two
/// sleepers is never looked for.
#[test]
fn sleepers_are_woken_by_contact() {
    let ground = floor_quad(0.0, 5.0);
    let mut sleeper = ball(1, 11, Vec3::new(0.0, 0.1, 0.0), 0.1);
    let mut ticks = 0;
    while sleeper.sleep != Sleep::Deep && ticks < 400 {
        sleeper.tick(ground.as_slice());
        ticks += 1;
    }
    assert_eq!(sleeper.sleep, Sleep::Deep);
    assert!(!sleeper.parts[0].contacts.is_empty());
    let mut stages = pairs();
    // Two sleepers side by side: nothing.
    let mut near = sleeper.clone();
    near.parts[0].entity = entity(3);
    for p in &mut near.parts[0].positions {
        p.x += 0.15;
    }
    step(
        &mut [&mut sleeper, &mut near],
        ground.as_slice(),
        &mut stages,
    );
    assert!(the_pairs(&stages).touches().is_empty());
    assert_eq!(sleeper.sleep, Sleep::Deep);
    // A ball dropped onto it.
    let mut faller = ball(2, 11, Vec3::new(0.05, 0.28, 0.0), 0.1);
    step(
        &mut [&mut faller, &mut sleeper],
        ground.as_slice(),
        &mut stages,
    );
    assert_eq!(the_pairs(&stages).touches().len(), 1);
    assert_eq!(sleeper.sleep, Sleep::Awake);
    assert_eq!(sleeper.countdown(), faller.countdown());
    assert_eq!(faller.countdown(), WAKE_COUNT - 2);
    assert!(sleeper.parts[0].contacts.is_empty());
}

/// A sleeper touched by a body registered at rest (countdown 1 in its
/// first tick) takes that countdown and this tick's post-step: it is
/// asleep again at once, the next tick takes the push's motion away, and
/// from then on neither moves.
#[test]
fn a_sleeper_woken_to_one_sleeps_in_the_same_tick() {
    let ground = floor_quad(0.0, 5.0);
    let mut sleeper = ball(1, 11, Vec3::new(0.0, 0.1, 0.0), 0.1);
    let mut ticks = 0;
    while sleeper.sleep != Sleep::Deep && ticks < 400 {
        sleeper.tick(ground.as_slice());
        ticks += 1;
    }
    assert_eq!(sleeper.sleep, Sleep::Deep);
    // A placed body overlapping it by 2 cm.
    let mut placed = ball(2, 11, Vec3::new(0.18, 0.1, 0.0), 0.1);
    placed.register_placed();
    let mut stages = pairs();
    step(
        &mut [&mut placed, &mut sleeper],
        ground.as_slice(),
        &mut stages,
    );
    assert_eq!(the_pairs(&stages).touches().len(), 1);
    assert_eq!(placed.sleep, Sleep::Asleep);
    assert_eq!(sleeper.sleep, Sleep::Asleep);
    assert_eq!(sleeper.countdown(), 0);
    step(
        &mut [&mut placed, &mut sleeper],
        ground.as_slice(),
        &mut stages,
    );
    assert_eq!(placed.sleep, Sleep::Deep);
    assert_eq!(sleeper.sleep, Sleep::Deep);
    let (a, b) = (placed.parts[0].positions, sleeper.parts[0].positions);
    for _ in 0..5 {
        step(
            &mut [&mut placed, &mut sleeper],
            ground.as_slice(),
            &mut stages,
        );
    }
    assert_eq!(placed.parts[0].positions, a);
    assert_eq!(sleeper.parts[0].positions, b);
}

/// A character's lower legs carry the living lower legs' layer and its
/// feet are out of every contact with other parts: two characters' shins
/// pass through each other while they live; once one is dead its shins
/// meet the other's.
#[test]
fn living_shins_pass_through_and_feet_never_meet() {
    let legs = |first: u32, x: f32| {
        let mut pelvis = setup(
            first,
            1,
            None,
            Vec3::new(x, 1.0, x * 60.0),
            Vec3::splat(0.1),
        );
        pelvis.links.clear();
        let mut shin = setup(
            first + 1,
            3,
            Some(0),
            Vec3::new(x, 0.5, 0.0),
            Vec3::new(0.06, 0.2, 0.06),
        );
        shin.foot = Some(Vec3::new(0.0, -0.3, 0.0));
        let foot = setup(
            first + 2,
            4,
            Some(1),
            Vec3::new(x, 0.05, 0.0),
            Vec3::new(0.06, 0.05, 0.12),
        );
        Ragdoll::new(&[pelvis, shin, foot]).with_drive()
    };
    let mut a = legs(10, 0.0);
    let mut b = legs(20, 0.05);
    assert_eq!(a.parts[1].pairing.layer, LIVING_SHIN_LAYER);
    assert_eq!(a.parts[2].pairing.layer, FOOT_LAYER);
    assert!(a.parts[2].pairing.out && !a.parts[1].pairing.out);
    let touches = |a: &mut Ragdoll, b: &mut Ragdoll| {
        let mut stages = pairs();
        stages[0].detect(&mut [a, b], &mut []);
        the_pairs(&stages)
            .touches()
            .iter()
            .map(|t| (t.a.1, t.b.1))
            .collect::<Vec<_>>()
    };
    assert!(touches(&mut a, &mut b).is_empty());
    b.go_limp();
    assert_eq!(b.parts[1].pairing.layer, 0);
    assert_eq!(b.parts[2].pairing.layer, FOOT_LAYER);
    assert_eq!(touches(&mut a, &mut b), [(1, 1)]);
}

/// A light prop dropped on a heavy sleeping one (a cup on a table) wakes
/// it and comes to rest on it, on the table's spheres: the cup (10, the
/// more mobile) brings its sphere; both sleep again afterwards.
#[test]
fn props_rest_on_each_other() {
    let ground = floor_quad(0.0, 5.0);
    let mut table = ball(1, 0, Vec3::new(0.0, 0.06, 0.0), 0.1);
    {
        let p = &mut table.parts[0];
        p.spheres = (-2..=2)
            .flat_map(|i| {
                (-1..=1).map(move |k| (Vec3::new(0.1 * i as f32, 0.0, 0.1 * k as f32), 0.06))
            })
            .collect();
        p.pairing = Pairing::new(&p.rest, &p.rest_inverse, &p.spheres, 0.05);
    }
    let mut ticks = 0;
    while table.sleep != Sleep::Deep && ticks < 400 {
        table.tick(ground.as_slice());
        ticks += 1;
    }
    assert_eq!(table.sleep, Sleep::Deep);
    let top = table.parts[0].transform().translation.y + 0.06;
    let mut cup = ball(2, 0, Vec3::new(0.0, 0.6, 0.0), 0.08);
    cup.parts[0].pairing.mobility = 10.0;
    assert!(cup.parts[0].pairing.key() < table.parts[0].pairing.key());
    let mut stages = pairs();
    let mut woke = false;
    for _ in 0..600 {
        step(&mut [&mut cup, &mut table], ground.as_slice(), &mut stages);
        woke |= table.sleep == Sleep::Awake;
        if cup.asleep() && table.asleep() {
            break;
        }
    }
    assert!(woke);
    assert!(cup.asleep() && table.asleep());
    // On the table's spheres (6 cm), not on the floor: as near to the
    // nearest as the two radii.
    let at = centre(&cup);
    let held = table.parts[0].transform();
    let nearest = table.parts[0]
        .spheres
        .iter()
        .map(|&(c, _)| held.transform_point3(c).distance(at))
        .fold(f32::MAX, f32::min);
    assert!((nearest - 0.14).abs() < 0.002, "{nearest} at {at}");
    assert!(at.y > top, "cup at {at}, table top {top}");
}

/// The body plugin solves with the pair stage.
#[test]
fn the_body_plugin_adds_the_pairs() {
    let mut app = App::new();
    app.add_plugins(BodyPlugin);
    assert!(
        app.world()
            .resource::<SolverStages>()
            .get::<Pairs>()
            .is_some()
    );
}

/// The all-pairs broad phase (the original's small-sector step) finds the
/// same meeting as the cell grid when the awake body is listed first, and
/// also stops the two limbs.
#[test]
fn the_all_pairs_sector_finds_the_same_meeting() {
    let scene = || {
        let mut a = ball(1, 11, Vec3::new(-0.3, 2.0, 0.0), 0.1);
        let mut b = ball(2, 11, Vec3::new(0.3, 2.0, 0.0), 0.1);
        a.set_velocity(Vec3::X * 2.0);
        b.set_velocity(Vec3::NEG_X * 2.0);
        (a, b)
    };
    for phase in [BroadPhase::Cells, BroadPhase::AllPairs] {
        let (mut a, mut b) = scene();
        let mut stages: Vec<Box<dyn SolverStage>> = vec![Box::new(match phase {
            BroadPhase::Cells => Pairs::default(),
            BroadPhase::AllPairs => Pairs::all_pairs(),
        })];
        let mut met = false;
        for _ in 0..30 {
            step(&mut [&mut a, &mut b], NONE, &mut stages);
            met |= !the_pairs(&stages).touches().is_empty();
        }
        assert!(met, "{phase:?}");
        assert!(centre(&a).distance(centre(&b)) > 0.19, "{phase:?}");
    }
}

/// The meet test is the original's two-way box pre-test, not only the
/// bounding spheres: two overlapping spheres whose mesh boxes are tiny
/// never meet, so no contact is generated.
#[test]
fn the_box_meet_test_gates_the_pair() {
    let mut a = ball(1, 11, Vec3::new(0.0, 2.0, 0.0), 0.1);
    let mut b = ball(2, 11, Vec3::new(0.19, 2.0, 0.0), 0.1);
    a.parts[0].pairing.mesh = Bounds::of_box(Vec3::ZERO, Vec3::splat(0.01));
    b.parts[0].pairing.mesh = Bounds {
        centre: Vec3::ZERO,
        half: Vec3::splat(0.01),
        radius: 0.1,
    };
    let mut stages = pairs();
    stages[0].detect(&mut [&mut a, &mut b], &mut []);
    assert!(the_pairs(&stages).touches().is_empty());
    // With their real mesh boxes the same two parts touch.
    let mut a = ball(1, 11, Vec3::new(0.0, 2.0, 0.0), 0.1);
    let mut b = ball(2, 11, Vec3::new(0.19, 2.0, 0.0), 0.1);
    stages[0].detect(&mut [&mut a, &mut b], &mut []);
    assert_eq!(the_pairs(&stages).touches().len(), 1);
}

/// A pair keeps four points: a fifth replaces one of the closest two in
/// weight space, or is dropped when it is that one.
#[test]
fn a_pair_keeps_four_points_spread_widest() {
    let (_, _, mut pair) = kept_pair(0.0, Vec3::ZERO, false);
    pair.count = 0;
    let at = |w: [f32; 4]| Point {
        a: w,
        b: w,
        normal: Vec3::X,
    };
    for w in [
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [0.0, 0.0, 0.9, 0.1],
    ] {
        pair.offer(at(w));
    }
    assert_eq!(pair.count, 4);
    pair.offer(at([0.0, 0.0, 0.0, 1.0]));
    let kept: Vec<[f32; 4]> = pair.points.iter().map(|p| p.a).collect();
    assert!(kept.contains(&[0.0, 0.0, 0.0, 1.0]), "{kept:?}");
    assert!(kept.contains(&[1.0, 0.0, 0.0, 0.0]) && kept.contains(&[0.0, 1.0, 0.0, 0.0]));
    // A newcomer next to a kept point is dropped.
    pair.offer(at([0.0, 0.0, 0.0, 0.99]));
    assert_eq!(pair.points.iter().map(|p| p.a).collect::<Vec<_>>(), kept);
}

/// A timing probe for the 83 Hz pair stage (`--ignored --nocapture`): 400
/// awake balls 25 cm apart over a 5 m square, half of them touching a
/// neighbour, detected 500 times. It prints the mean time of one
/// detection, to compare a change against the tree before it.
#[test]
#[ignore = "timing probe"]
fn pair_detection_timing() {
    let mut bodies: Vec<Ragdoll> = (0..400u32)
        .map(|i| {
            let (x, z) = ((i % 20) as f32 * 0.25, (i / 20) as f32 * 0.25);
            let lift = if i % 2 == 0 { 0.0 } else { 0.05 };
            ball(i + 1, 11, Vec3::new(x + lift, 2.0, z), 0.12)
        })
        .collect();
    let mut stages = pairs();
    let mut all: Vec<&mut Ragdoll> = bodies.iter_mut().collect();
    for _ in 0..50 {
        stages[0].detect(&mut all, &mut []);
    }
    let runs = 500;
    let start = std::time::Instant::now();
    for _ in 0..runs {
        stages[0].detect(&mut all, &mut []);
    }
    let each = start.elapsed().as_secs_f64() * 1e6 / runs as f64;
    println!(
        "pair detection: {each:.1} us per tick, {} touches",
        the_pairs(&stages).touches().len()
    );
}

/// A placed pair's pass moves only the placed side, by the whole
/// correction (no mobility share), and records the opposite push in its
/// offset array; the other side is untouched.
#[test]
fn the_placed_pass_moves_only_the_placed_side() {
    let (mut a, mut b, pair) = kept_pair(0.0051, Vec3::ZERO, false);
    let (a0, b0) = (a.parts[0].positions, b.parts[0].positions);
    assert!(solve_placed_pair(
        &mut a.parts[0],
        &mut b.parts[0],
        true,
        &pair.points[..1]
    ));
    for (i, before) in a0.iter().enumerate() {
        // depth 0.51 cm less the 0.01 slop is 0.5 cm; each particle takes
        // its quarter of it.
        assert!((a.parts[0].positions[i].x - before.x - 0.00125).abs() < 1e-8);
        assert!((a.parts[0].placed_offset[i].x + 0.00125).abs() < 1e-8);
    }
    assert_eq!(b.parts[0].positions, b0);

    // With B placed the correction goes the other way and A stays.
    let (mut a, mut b, pair) = kept_pair(0.0051, Vec3::ZERO, false);
    let (a0, b0) = (a.parts[0].positions, b.parts[0].positions);
    assert!(solve_placed_pair(
        &mut a.parts[0],
        &mut b.parts[0],
        false,
        &pair.points[..1]
    ));
    for (i, before) in b0.iter().enumerate() {
        assert!((b.parts[0].positions[i].x - before.x + 0.00125).abs() < 1e-8);
        assert!((b.parts[0].placed_offset[i].x - 0.00125).abs() < 1e-8);
    }
    assert_eq!(a.parts[0].positions, a0);
}

/// A placed hit records the contact (so the pair is kept and the two sleep
/// countdowns equalise) but no impact hit, and the placed side takes the
/// whole push with its weights normalised to sum one, recording the
/// opposite in its offset array.
#[test]
fn a_placed_hit_takes_the_push_and_records_its_opposite() {
    let mut a = ball(1, 11, Vec3::ZERO, 0.1);
    let mut b = ball(2, 11, Vec3::X * 0.25, 0.1);
    let before_a = a.parts[0].positions;
    let before_b = b.parts[0].positions;
    let mut pair = Pair {
        a: Side {
            key: entity(1),
            body: 0,
            part: 0,
        },
        b: Side {
            key: entity(2),
            body: 1,
            part: 0,
        },
        points: [Point::default(); POINTS],
        count: 0,
        live: true,
    };
    let mut touch = Touch {
        a: (entity(1), 0),
        b: (entity(2), 0),
        met: false,
        hits: 0,
        point: Vec3::ZERO,
        push: Vec3::ZERO,
        deepest: 0.0,
        travel: Vec3::ZERO,
        travel2: 0.0,
        struck: [None; 2],
        hit_event: false,
    };
    let b_now = b.parts[0].transform();
    // B's centre at x = 0.25 with radius 0.1; A's sphere at x = 0 with
    // radius 0.1; q is the touched point on B toward A, and the push takes
    // A off by 0.05 m.
    let q = Vec3::new(-0.1, 0.0, 0.0);
    let delta = Vec3::new(0.05, 0.0, 0.0);
    assert!(on_placed_hit(
        &mut a.parts[0],
        &mut b.parts[0],
        true,
        &b_now,
        Vec3::ZERO,
        q,
        delta,
        &mut pair,
        &mut touch,
    ));
    assert!(touch.met);
    assert_eq!(touch.hits, 0);
    assert_eq!(pair.count, 1);
    assert_eq!(b.parts[0].positions, before_b);
    let push = b_now.transform_vector3(delta);
    let moved = a.parts[0].positions[0] - before_a[0];
    assert!(moved.x > 0.0, "{moved:?}");
    // The offset array keeps the whole opposite push: its weights sum to 1,
    // so its particles together record −push (the edge restore then spreads
    // the push over them).
    let offset: Vec3 = a.parts[0].placed_offset.iter().sum();
    assert!((offset.x + push.x).abs() < 1e-6, "{offset:?}");
}

/// The sector's constructor tests every pair unless the scene's flag word
/// asks for the grid (either of bits 0x2000000 and 0x4000000).
#[test]
fn the_scene_flags_choose_the_grid_or_all_pairs() {
    assert_eq!(BroadPhase::for_scene(0), BroadPhase::AllPairs);
    assert_eq!(BroadPhase::for_scene(0x100_0000), BroadPhase::AllPairs);
    assert_eq!(BroadPhase::for_scene(0x200_0000), BroadPhase::Cells);
    assert_eq!(BroadPhase::for_scene(0x400_0000), BroadPhase::Cells);
    assert_eq!(BroadPhase::for_scene(0x600_0000 | 0x89), BroadPhase::Cells);
    assert_eq!(Pairs::for_scene(0).phase(), BroadPhase::AllPairs);
    assert_eq!(Pairs::for_scene(0x400_0000).phase(), BroadPhase::Cells);
}

/// A striker meeting a living limb takes the hit event: the pair is kept,
/// its touch carries the mean point, the summed push and the smoothed
/// travel, and neither side is struck.
#[test]
fn a_striker_pair_takes_the_hit_event_and_no_impact() {
    use crate::part::PartState;
    let mut a = ball(1, 11, Vec3::ZERO, 0.1);
    let mut b = ball(2, 11, Vec3::X * 0.25, 0.1);
    a.parts[0].state = PartState(PartState::HELD | PartState::STRIKE_CHAIN);
    b.parts[0].state = PartState(PartState::LIMB | PartState::DYNAMIC);
    a.set_velocity(Vec3::X * 2.0);
    let mut stages = pairs();
    for _ in 0..40 {
        step(&mut [&mut a, &mut b], NONE, &mut stages);
        if let Some(t) = the_pairs(&stages).touches().first() {
            assert!(t.met && t.hits > 0, "{t:?}");
            assert!(t.hit_event, "{t:?}");
            let event = the_pairs(&stages).hit_events().next().unwrap();
            assert_eq!(event.a, t.a);
            assert_eq!(event.b, t.b);
            assert_eq!(event.point, t.point);
            assert_eq!(event.travel, t.travel);
            assert_eq!(event.push, t.push);
            assert_eq!(the_pairs(&stages).hit_events().count(), 1);
            assert_eq!(t.struck, [None, None]);
            assert_eq!(t.impact(), None);
            // The mean point is between the two balls' centres.
            assert!((t.point.x - 0.15).abs() < 0.05, "{t:?}");
            return;
        }
    }
    panic!("the two balls never touched");
}

/// A wall triangle in the y-z plane at x = 0, its front toward +x, wide
/// enough that any test sphere projects inside it.
fn wall() -> Vec<[Vec3; 3]> {
    vec![[
        Vec3::new(0.0, -2.0, -2.0),
        Vec3::new(0.0, 2.0, 2.0),
        Vec3::new(0.0, -2.0, 2.0),
    ]]
}

/// A body with one ball part used as the mesh side: a wide mesh bound so
/// the broad phase sees the pair, no stored spheres, and the given mesh.
fn mesh_side(mesh: Vec<[Vec3; 3]>) -> Ragdoll {
    let mut b = ball(2, 11, Vec3::ZERO, 0.01);
    // Its stored spheres sit 10 m away, so only the mesh can answer and the
    // fallback is never used; a body with no spheres at all would be left
    // out of the pair stage (the original's bodies always carry some).
    let far = [(Vec3::new(0.0, 0.0, 10.0), 0.01)];
    b.parts[0].spheres = far.to_vec();
    b.parts[0].pairing.sphere = Bounds::of_spheres(&far);
    b.parts[0].pairing.mesh = Bounds {
        centre: Vec3::ZERO,
        half: Vec3::new(0.1, 2.0, 2.0),
        radius: 2.0,
    };
    b.set_part_triangles(0, mesh);
    b
}

/// A fresh pair and touch for a direct `collide` call.
fn fresh() -> (Pair, Touch) {
    let pair = Pair {
        a: Side {
            key: entity(1),
            body: 0,
            part: 0,
        },
        b: Side {
            key: entity(2),
            body: 1,
            part: 0,
        },
        points: [Point::default(); POINTS],
        count: 0,
        live: true,
    };
    let touch = Touch {
        a: (entity(1), 0),
        b: (entity(2), 0),
        met: false,
        hits: 0,
        point: Vec3::ZERO,
        push: Vec3::ZERO,
        deepest: 0.0,
        travel: Vec3::ZERO,
        travel2: 0.0,
        struck: [None; 2],
        hit_event: false,
    };
    (pair, touch)
}

/// A sphere in front of a part's triangle is pushed off it along the
/// triangle's closest-point direction: the mesh narrow phase, not the
/// sphere fallback (the mesh side has no stored spheres at all). The push
/// is +x because the face's normal points there and the sphere's centre is
/// 5 cm in front of the plane, inside the triangle.
#[test]
fn a_sphere_meets_a_parts_mesh_triangle() {
    let mut a = ball(1, 11, Vec3::new(0.05, 0.0, 0.0), 0.1);
    let mut b = mesh_side(wall());
    let (mut pair, mut touch) = fresh();
    let before = a.parts[0].positions;
    collide(
        &mut a.parts[0],
        &mut b.parts[0],
        &mut pair,
        &mut touch,
        None,
    );
    assert!(touch.met && touch.hits == 1, "{touch:?}");
    assert_eq!(pair.count, 1);
    // The push is 5 cm along +x spread over the part's weights: every
    // particle moves forward on x.
    for (now, was) in a.parts[0].positions.iter().zip(before) {
        assert!(now.x > was.x, "{now:?} {was:?}");
    }
    // The stored normal is the push's direction.
    assert!(
        pair.points[0].normal.x > 0.99,
        "{:?}",
        pair.points[0].normal
    );
}

/// The triangle test is one-sided: a centre behind the face is never
/// pushed back, so a sphere that ended up behind a wall passes through it.
#[test]
fn a_sphere_behind_a_mesh_triangle_is_left_alone() {
    let mut a = ball(1, 11, Vec3::new(-0.05, 0.0, 0.0), 0.1);
    let mut b = mesh_side(wall());
    let (mut pair, mut touch) = fresh();
    let before = a.parts[0].positions;
    collide(
        &mut a.parts[0],
        &mut b.parts[0],
        &mut pair,
        &mut touch,
        None,
    );
    assert!(!touch.met && touch.hits == 0, "{touch:?}");
    assert_eq!(a.parts[0].positions, before);
}

/// A fast sphere crosses a wall in one tick: the swept query takes it back
/// to where it first touched (its centre one radius in front of the plane).
/// The pair stage has no static-only gate (only the level detection has
/// one), so a held item's static-only mode sweeps the same.
#[test]
fn a_fast_sphere_is_caught_by_the_swept_mesh_query() {
    let run = |static_only: bool| {
        // The sphere starts one radius' reach in front of the face and
        // crosses it: the swept test only runs from the front side.
        let mut a = ball(1, 11, Vec3::new(0.3, 0.0, 0.0), 0.1);
        let mut b = mesh_side(wall());
        a.parts[0].static_only = static_only;
        // 60 m/s is 72 cm a tick: far enough that the end centre is more
        // than a radius behind the face.
        a.set_velocity(Vec3::NEG_X * 60.0);
        let mut stages = pairs();
        step(&mut [&mut a, &mut b], NONE, &mut stages);
        (
            a.parts[0].positions[0].x,
            the_pairs(&stages).touches().len(),
        )
    };
    let (swept, touches) = run(false);
    assert_eq!(touches, 1);
    // The push takes the contact point back toward the face; the immediate
    // push spreads over the particle weights, so the centre does not reach
    // the plane in the same tick, but it is nowhere near the -0.42 m it
    // would have travelled freely.
    assert!((-0.15..0.3).contains(&swept), "swept to {swept}");
    assert_eq!(run(true), (swept, touches));
}

/// The contact offset array a placed body's hits record is readable by its
/// owner and cleared by the integration while the body is placed, as the
/// original's contact offset array is.
#[test]
fn a_placed_bodys_contact_offsets_are_read_and_cleared() {
    let mut a = ball(1, 11, Vec3::ZERO, 0.1);
    let mut b = ball(2, 11, Vec3::X * 0.25, 0.1);
    let (mut pair, mut touch) = fresh();
    let b_now = b.parts[0].transform();
    assert!(on_placed_hit(
        &mut a.parts[0],
        &mut b.parts[0],
        true,
        &b_now,
        Vec3::ZERO,
        Vec3::new(-0.1, 0.0, 0.0),
        Vec3::new(0.05, 0.0, 0.0),
        &mut pair,
        &mut touch,
    ));
    let offsets: Vec<Vec3> = a.placed_offsets(0).unwrap().to_vec();
    assert!(offsets.iter().any(|o| o.length() > 0.0), "{offsets:?}");
    assert_eq!(a.placed_offsets(9), None);
    assert_eq!(a.placed_offsets_all().count(), 1);
    // The next integration while placed starts the array again.
    a.settle_for(2);
    a.tick(&[] as &[[Vec3; 3]]);
    assert_eq!(a.placed_offsets(0), Some([Vec3::ZERO; 4]));
    // A direct clear does the same after a hit.
    assert!(on_placed_hit(
        &mut a.parts[0],
        &mut b.parts[0],
        true,
        &b_now,
        Vec3::ZERO,
        Vec3::new(-0.1, 0.0, 0.0),
        Vec3::new(0.05, 0.0, 0.0),
        &mut pair,
        &mut touch,
    ));
    a.clear_placed_offsets();
    assert_eq!(a.placed_offsets(0), Some([Vec3::ZERO; 4]));
}

/// A placed pair's mesh hit also takes the static path and the placed
/// callback: the contact is recorded with no impact hit, the placed side
/// moves and its offset array holds the opposite push.
#[test]
fn a_placed_mesh_hit_records_an_offset_and_no_hit() {
    let mut a = ball(1, 11, Vec3::new(0.05, 0.0, 0.0), 0.1);
    let mut b = mesh_side(wall());
    a.settle_for(3);
    let (mut pair, mut touch) = fresh();
    let before = a.parts[0].positions;
    collide(
        &mut a.parts[0],
        &mut b.parts[0],
        &mut pair,
        &mut touch,
        Some(true),
    );
    assert!(touch.met, "{touch:?}");
    assert_eq!(touch.hits, 0);
    assert_eq!(pair.count, 1);
    assert!(a.parts[0].positions[0].x > before[0].x);
    let offsets: Vec<Vec3> = a.placed_offsets(0).unwrap().to_vec();
    assert!(offsets.iter().any(|o| o.length() > 0.0), "{offsets:?}");
}
