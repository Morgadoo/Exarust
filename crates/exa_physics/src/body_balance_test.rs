//! Balance and support: the original's rules, each pinned to its numbers
//! (`docs/formats/RSQ.md`, Physical body).
//! (Included from body_balance.rs: `#[path = "body_balance_test.rs"] mod tests;`)
use bevy::math::Affine3A;

use super::super::{Bearing, BodyStats, Drive, PartSetup};
use super::*;

const NONE: &[[Vec3; 3]] = &[];

fn cm(x: f32, y: f32, z: f32) -> Vec3 {
    Vec3::new(x, y, z) * 0.01
}

/// A square of upward faces at height `y`, `half` metres either side of
/// `centre` (x, z).
fn square(centre: Vec2, y: f32, half: f32) -> Vec<[Vec3; 3]> {
    let at = |x: f32, z: f32| Vec3::new(centre.x + x * half, y, centre.y + z * half);
    vec![
        [at(-1.0, -1.0), at(-1.0, 1.0), at(1.0, 1.0)],
        [at(-1.0, -1.0), at(1.0, 1.0), at(1.0, -1.0)],
    ]
}

fn floor() -> Vec<[Vec3; 3]> {
    square(Vec2::ZERO, 0.0, 10.0)
}

/// One part of the tests' biped (cm, in the part's space).
struct Bone {
    bone: usize,
    parent: Option<usize>,
    /// Origin in the parent (the pelvis's in the world).
    origin: [f32; 3],
    centre: [f32; 3],
    half: [f32; 3],
}

const fn bone(
    bone: usize,
    parent: Option<usize>,
    origin: [f32; 3],
    centre: [f32; 3],
    half: [f32; 3],
) -> Bone {
    Bone {
        bone,
        parent,
        origin,
        centre,
        half,
    }
}

/// The biped's parts: pelvis, the right leg (thigh, lower leg, foot), the
/// left, lumbar and thorax.
const BIPED: [Bone; 9] = [
    bone(1, None, [0.0, 103.0, 0.0], [0.0; 3], [15.0, 10.0, 10.0]),
    bone(
        2,
        Some(0),
        [-9.0, -8.5, 3.5],
        [0.0, -22.0, 0.0],
        [7.0, 22.0, 7.0],
    ),
    bone(
        3,
        Some(1),
        [0.0, -43.2, 0.0],
        [0.0, -22.0, 0.0],
        [5.0, 22.0, 5.0],
    ),
    bone(
        4,
        Some(2),
        [1.2, -41.9, -2.3],
        [0.0, -4.0, 6.0],
        [4.5, 4.5, 11.0],
    ),
    bone(
        5,
        Some(0),
        [9.0, -8.5, 3.5],
        [0.0, -22.0, 0.0],
        [7.0, 22.0, 7.0],
    ),
    bone(
        6,
        Some(4),
        [0.0, -43.2, 0.0],
        [0.0, -22.0, 0.0],
        [5.0, 22.0, 5.0],
    ),
    bone(
        7,
        Some(5),
        [-1.2, -41.9, -2.3],
        [0.0, -4.0, 6.0],
        [4.5, 4.5, 11.0],
    ),
    bone(
        8,
        Some(0),
        [0.0, 8.7, 1.3],
        [0.0, 6.0, 0.0],
        [13.0, 8.0, 9.0],
    ),
    bone(
        9,
        Some(7),
        [0.0, 11.6, -3.0],
        [0.0, 13.0, 3.0],
        [17.0, 15.0, 10.0],
    ),
];

/// A living biped standing upright with its feet on the ground at the
/// origin: legs whose lower legs carry the stored foot contact and a foot
/// part, a thorax with the two shoulder pins.
fn biped() -> Ragdoll {
    biped_at(Affine3A::IDENTITY)
}

/// The biped placed by `place` (applied about the world origin).
fn biped_at(place: Affine3A) -> Ragdoll {
    let mut world = Vec::new();
    for b in &BIPED {
        let local = Vec3::from_array(b.origin) * 0.01;
        let at = b.parent.map_or(local, |j| world[j] + local);
        world.push(at);
    }
    let setup: Vec<PartSetup> = BIPED
        .iter()
        .enumerate()
        .map(|(i, b)| PartSetup {
            bone: b.bone,
            entity: Entity::PLACEHOLDER,
            parent: b.parent,
            world: place * Affine3A::from_translation(world[i]),
            centre: Vec3::from_array(b.centre) * 0.01,
            half: Vec3::from_array(b.half) * 0.01,
            origin: b
                .parent
                .map_or(Vec3::ZERO, |_| Vec3::from_array(b.origin) * 0.01),
            links: vec![],
            foot: matches!(b.bone, 3 | 6).then(|| cm(0.16, -48.97, 2.4)),
            pins: if b.bone == 9 {
                vec![cm(10.4, 9.65, 1.25), cm(-10.4, 9.65, 1.25)]
            } else {
                vec![]
            },
            spheres: None,
        })
        .collect();
    Ragdoll::new(&setup).with_drive()
}

fn drive(body: &Ragdoll) -> &Drive {
    body.drive.as_ref().unwrap()
}

fn bearing(body: &mut Ragdoll, bearing: Bearing) {
    body.set_bearing(bearing);
}

/// Moves every particle of the body by `by`, as if it had moved there.
fn shift(body: &mut Ragdoll, by: Vec3) {
    for p in &mut body.parts {
        for x in &mut p.positions {
            *x += by;
        }
    }
}

/// The chest is the mean of the thorax pins (±10.4, 9.65, 1.25 cm in the
/// thorax) on the body's particles.
#[test]
fn the_chest_is_the_mean_of_the_thorax_pins() {
    let body = biped();
    let thorax = Vec3::from(body.part_of(9).unwrap().transform().translation);
    let chest = body.chest_point().unwrap();
    assert!(
        chest.abs_diff_eq(thorax + cm(0.0, 9.65, 1.25), 1e-5),
        "{chest}"
    );
}

/// Each lower leg is a leg, in part order. Its contact is its foot part's
/// origin, the ankle (1.2, −41.9, −2.3 cm), not the stored point, as the
/// original's muscle setup moves it; the leg's length is knee (43.2 cm) +
/// ankle (41.98 cm); the sole height is the foot box's depth under its
/// origin (4.5 + 4 cm) less 0.5 cm.
#[test]
fn legs_stand_on_their_ankles() {
    let body = biped();
    let shins: Vec<usize> = body
        .legs()
        .iter()
        .map(|l| body.parts[l.shin].bone)
        .collect();
    assert_eq!(shins, [3, 6]);
    let leg = &body.legs()[0];
    assert!(leg.point.abs_diff_eq(cm(1.2, -41.9, -2.3), 1e-6));
    assert_eq!(body.parts[leg.shin].foot, Some(leg.point));
    let ankle = cm(1.2, -41.9, -2.3).length();
    assert!(
        (leg.length - (0.432 + ankle)).abs() < 1e-6,
        "{}",
        leg.length
    );
    assert!((ankle - 0.4198).abs() < 1e-4);
    assert!((body.sole() - 0.08).abs() < 1e-6, "{}", body.sole());
}

/// An alert body standing upright on the ground never loses its balance:
/// its lean is vertical, so q = 1 (u_y⁸ + 0.05, capped), the counter goes
/// to the standing floor of 80 on the first tick and stays, balance stays
/// 1, both feet are grounded and S = 1.
#[test]
fn an_upright_body_never_falls() {
    let mut body = biped();
    let ground = floor();
    for tick in 0..1000 {
        body.measure_balance(ground.as_slice());
        let b = drive(&body).balance();
        assert_eq!((b.value, b.counter()), (1.0, 80), "tick {tick}");
        assert_eq!(drive(&body).support(), 1.0);
    }
    let b = drive(&body).balance();
    // The chest 123.55 cm over the ankles' midpoint, 1.65 cm behind it.
    assert!(b.lean().abs_diff_eq(Vec3::Y, 0.02), "{}", b.lean());
    assert!(b.has_support() && b.airborne() == 0);
    for leg in body.legs() {
        assert!(leg.grounded);
        assert!(leg.ground.unwrap().abs() < 1e-5);
    }
}

/// With no ground under its feet a standing body has no balance and no
/// support, and counts the ticks it is airborne; a moving one keeps its
/// balance but has no support either, and so does one its character
/// controller carries (Exarust's own, while capsules carry bodies).
#[test]
fn balance_is_0_without_ground() {
    let mut body = biped();
    bearing(
        &mut body,
        Bearing {
            carried: false,
            ..Bearing::default()
        },
    );
    body.measure_balance(floor().as_slice());
    for tick in 1..=5 {
        body.measure_balance(NONE);
        let b = drive(&body).balance();
        assert_eq!(b.value, 0.0);
        assert_eq!(b.airborne(), tick);
        assert!(!b.has_support());
        assert_eq!(drive(&body).support(), 0.0);
        // A query that meets nothing finds ground 100 m down, under the
        // feet and under the pelvis.
        assert_eq!(body.legs()[0].ground, Some(-100.0));
        assert_eq!(b.ground(), Some(-100.0));
    }
    let moving = Bearing {
        moving: true,
        carried: false,
        ..Bearing::default()
    };
    let carried = Bearing::default();
    for kept in [moving, carried] {
        let mut body = biped();
        bearing(&mut body, kept);
        body.measure_balance(NONE);
        assert_eq!(drive(&body).balance().value, 1.0, "{kept:?}");
        assert_eq!(drive(&body).support(), 0.0);
    }
    // A fallen body is not carried by its controller.
    let mut body = biped();
    body.fall();
    body.measure_balance(NONE);
    assert_eq!(drive(&body).balance().value, 0.0);
}

/// Without support the body asks for the ground under its pelvis: from
/// 20 cm above it (1.23 m), a 20 cm sphere swept 20 m down, to 18.77 m
/// under the floor it stood on (100 m down when it meets nothing); with
/// support it does not.
#[test]
fn without_support_the_ground_under_the_pelvis() {
    let unsupported = Bearing {
        carried: false,
        ..Bearing::default()
    };
    let mut standing = biped();
    standing.measure_balance(floor().as_slice());
    assert!(drive(&standing).balance().has_support());
    assert_eq!(drive(&standing).balance().ground(), None);
    for (depth, found) in [(15.0, Some(-15.0)), (18.5, Some(-18.5)), (19.0, None)] {
        let mut body = biped();
        bearing(&mut body, unsupported);
        let level = square(Vec2::ZERO, -depth, 10.0);
        body.measure_balance(level.as_slice());
        let b = drive(&body).balance();
        assert!(!b.has_support(), "{depth} m");
        let ground = b.ground().unwrap();
        match found {
            Some(g) => assert!((ground - g).abs() < 1e-4, "{depth} m: {ground}"),
            None => assert_eq!(ground, -100.0, "{depth} m"),
        }
    }
}

/// Landing again after a time in the air gives the chest a landing
/// momentum weight of 5, which fades 0.05 a tick.
#[test]
fn landing_brings_momentum() {
    let mut body = biped();
    bearing(
        &mut body,
        Bearing {
            moving: true,
            ..Bearing::default()
        },
    );
    body.measure_balance(NONE);
    assert!(!drive(&body).balance().has_support());
    body.measure_balance(floor().as_slice());
    assert_eq!(drive(&body).balance().landing, 5.0);
    body.measure_balance(floor().as_slice());
    assert!((drive(&body).balance().landing - 4.95).abs() < 1e-6);
}

/// The chest's velocity is smoothed twice a tick, fast (0.4) and slow
/// (0.1), from where it was the last two ticks.
#[test]
fn the_chests_velocity_is_smoothed_twice() {
    let mut body = biped();
    let ground = floor();
    body.measure_balance(ground.as_slice());
    body.measure_balance(ground.as_slice());
    shift(&mut body, cm(1.0, 0.0, 0.0));
    // The move shows the tick after it is seen.
    body.measure_balance(ground.as_slice());
    assert_eq!(drive(&body).balance().fast, Vec3::ZERO);
    body.measure_balance(ground.as_slice());
    let b = drive(&body).balance();
    assert!(b.fast.abs_diff_eq(cm(0.4, 0.0, 0.0), 1e-7), "{}", b.fast);
    assert!(b.slow.abs_diff_eq(cm(0.1, 0.0, 0.0), 1e-7), "{}", b.slow);
}

/// Score q = u_y⁸ standing (the original's lean table: chest 125 cm above
/// the contacts, 6 to 60 cm off them), u_y⁴ moving, plus 0.05 × the race's
/// balance bonus, capped at 1. A human (bonus 1) leaning 44 cm with the
/// chest 130 cm up scores 0.698, not under 0.6: its score drops under 0.6
/// only past 52.2 cm (48 cm without the bonus).
#[test]
fn the_score_is_the_leans_height_to_the_eighth() {
    let uy = |off: f32, up: f32| up / off.hypot(up);
    for (off, want) in [
        (6.0, 0.9908),
        (14.0, 0.9514),
        (20.0, 0.9038),
        (30.0, 0.7993),
        (40.0, 0.6771),
        (50.0, 0.5523),
        (60.0, 0.4363),
    ] {
        let q = score(uy(off, 125.0), false, 0.0);
        assert!((q - want).abs() < 1e-3, "{off} cm: {q}");
    }
    assert!((score(uy(40.0, 125.0), true, 0.0) - 0.8229).abs() < 1e-3);
    assert!((score(uy(44.0, 130.0), false, 1.0) - 0.698).abs() < 1e-3);
    assert!(score(uy(52.0, 130.0), false, 1.0) > 0.6);
    assert!(score(uy(53.0, 130.0), false, 1.0) < 0.6);
    assert!(score(uy(47.0, 130.0), false, 0.0) > 0.6);
    assert!(score(uy(49.0, 130.0), false, 0.0) < 0.6);
    assert_eq!(score(1.0, false, 1.0), 1.0);
}

/// The counter grows by 10 − 10q rounded half to even, as the processor
/// rounds: nothing at q = 0.95, 2 at 0.85 and at 0.75, 4 at 0.65 and at
/// 0.55, 6 at 0.45, 8 at 0.25.
#[test]
fn the_counter_rounds_half_to_even() {
    for (q, gain) in [
        (1.0, 0),
        (0.951, 0),
        (0.95, 0),
        (0.94, 1),
        (0.85, 2),
        (0.75, 2),
        (0.65, 4),
        (0.55, 4),
        (0.5, 5),
        (0.45, 6),
        (0.25, 8),
        (0.0, 10),
    ] {
        assert_eq!(counter_gain(q), gain, "q = {q}");
    }
}

/// Steady (q > 0.99), standing still the counter falls by 5 but never under
/// 80, even from 0; moving it falls by 10 to 0. At 100 balance is the
/// score and the counter stays at 100 while q ≤ 0.99. Below 100 balance
/// recovers by (5 − v)·0.01 a tick at a slow chest speed v under 5 cm a
/// tick, 0.05 at rest.
#[test]
fn the_counter_and_recovery() {
    let mut b = Balance::default();
    b.settle(1.0, 0.0, true);
    assert_eq!(b.counter, 80);
    b.counter = 100;
    b.settle(1.0, 0.0, true);
    assert_eq!(b.counter, 95);
    b.settle(1.0, 0.0, false);
    assert_eq!(b.counter, 85);
    b.counter = 3;
    b.settle(1.0, 0.0, false);
    assert_eq!(b.counter, 0);
    b.counter = 99;
    b.settle(0.7, 0.0, true);
    assert_eq!((b.counter, b.value), (100, 0.7));
    b.settle(0.98, 0.0, true);
    assert_eq!((b.counter, b.value), (100, 0.98));
    assert_eq!(recovery(0.0), Some(0.05));
    assert!((recovery(9.0).unwrap() - 0.02).abs() < 1e-6);
    assert!((recovery(4.9 * 4.9).unwrap() - 0.001).abs() < 1e-6);
    assert_eq!(recovery(25.0), None);
    let mut b = Balance {
        value: 0.5,
        counter: 80,
        ..Balance::default()
    };
    b.settle(0.9, 9.0, true);
    assert_eq!(b.counter, 81);
    assert!((b.value - 0.52).abs() < 1e-6);
}

/// The floor of 80 makes a second wobble topple a body fast: a lean that
/// scores 0.55 (+4 a tick) takes a fresh body 25 ticks to lose its balance,
/// one that has stood since 5.
#[test]
fn the_floor_of_80_makes_a_second_wobble_topple_fast() {
    let topple = |mut b: Balance| {
        (1..100)
            .find(|_| {
                b.settle(0.55, 0.0, true);
                b.value < 0.6
            })
            .unwrap()
    };
    assert_eq!(topple(Balance::default()), 25);
    let mut stood = Balance::default();
    stood.settle(1.0, 0.0, true);
    assert_eq!(topple(stood), 5);
}

/// A slow vertical chest speed over 3 cm a tick cuts a grounded body's
/// score by (1 − 0.333(v − 3))² (to nothing above 6) and sets the counter
/// to 100, so balance follows the cut score.
#[test]
fn vertical_speed_cuts_the_score() {
    assert_eq!(vertical_cut(0.8, 3.0, true), 0.8);
    assert!((vertical_cut(0.8, 4.5, true) - 0.8 * 0.2505).abs() < 1e-4);
    assert_eq!(vertical_cut(0.8, 7.0, true), 0.0);
    assert_eq!(vertical_cut(0.8, 7.0, false), 0.8);
    // Sinking 5 cm a tick: the slow speed passes 3 cm a tick on the tenth
    // tick of sinking (it shows a tick late).
    let mut body = biped();
    let ground = floor();
    body.measure_balance(ground.as_slice());
    let mut cut = None;
    for tick in 1..=12 {
        shift(&mut body, cm(0.0, -5.0, 0.0));
        body.measure_balance(ground.as_slice());
        let b = drive(&body).balance();
        if b.counter() == 100 && cut.is_none() {
            cut = Some(tick);
            let v = b.slow.y.abs() * 100.0;
            assert!(v > 3.0);
            assert!((b.value - vertical_cut(1.0, v, true)).abs() < 1e-5);
        }
    }
    assert_eq!(cut, Some(10));
}

/// A foot is grounded with its contact within 20 cm of its ground plus the
/// sole height, or within 30 cm times the body scale while moving. A NaN
/// stays grounded.
#[test]
fn feet_are_grounded_within_20_cm() {
    assert!(foot_grounded(20.0, false, 1.0) && !foot_grounded(20.1, false, 0.5));
    assert!(foot_grounded(30.0, true, 1.0) && !foot_grounded(30.1, true, 1.0));
    // Moving, the window scales. Standing does not, and a NaN stays grounded.
    assert!(foot_grounded(15.0, true, 0.5) && !foot_grounded(15.1, true, 0.5));
    assert!(foot_grounded(60.0, true, 2.0) && !foot_grounded(60.1, true, 2.0));
    assert!(foot_grounded(20.0, false, 2.0));
    assert!(!foot_grounded(20.1, false, f32::NAN));
    assert!(foot_grounded(f32::NAN, true, 1.0));
    assert!(foot_grounded(100.0, true, f32::NAN));
    // The biped's ankles stand 9.4 cm up with an 8 cm sole: lifted 18 cm
    // they are 19.4 cm over, 23 cm they are 24.4 cm over (grounded only
    // moving).
    for (lift, moving, grounded) in [
        (18.0, false, true),
        (23.0, false, false),
        (23.0, true, true),
    ] {
        let mut body = biped_at(Affine3A::from_translation(cm(0.0, lift, 0.0)));
        bearing(
            &mut body,
            Bearing {
                moving,
                ..Bearing::default()
            },
        );
        body.measure_balance(floor().as_slice());
        assert_eq!(
            body.legs().iter().all(|l| l.grounded),
            grounded,
            "{lift} cm, moving {moving}"
        );
    }
    // Half scale narrows the moving window to 15 cm, so a contact 24.4 cm
    // over its ground is not grounded.
    let mut half = biped_at(Affine3A::from_translation(cm(0.0, 23.0, 0.0))).with_stats(BodyStats {
        scale: 0.5,
        ..BodyStats::HUMAN
    });
    bearing(
        &mut half,
        Bearing {
            moving: true,
            ..Bearing::default()
        },
    );
    half.measure_balance(floor().as_slice());
    assert!(half.legs().iter().all(|l| !l.grounded));
}

/// Ground the query finds higher than half a leg under the pelvis is not
/// within reach (the foot is lifted over it), except while getting up.
#[test]
fn ground_above_half_a_leg_is_out_of_reach() {
    // Pelvis 103 cm up, leg 85.2 cm: reach ends 60.4 cm up.
    assert!(within_reach(60.0, 103.0, 85.2, false));
    assert!(!within_reach(61.0, 103.0, 85.2, false));
    assert!(within_reach(61.0, 103.0, 85.2, true));
    // A 65 cm high block under the right foot's probe only (5 cm ahead of
    // its ankle at (−7.8, 1.2) cm).
    let mut ground = floor();
    ground.extend(square(Vec2::new(-0.078, 0.062), 0.65, 0.03));
    let mut body = biped();
    body.measure_balance(ground.as_slice());
    let legs = body.legs();
    assert_eq!(legs[0].ground, None);
    assert!(!legs[0].grounded && legs[1].grounded);
    assert_eq!(drive(&body).support(), 1.0);
}

/// Down, lying or getting up, each leg's query starts at its ankle instead
/// of 70 cm above it, with a 5 cm sphere as a standing body's: under a shelf
/// 40 cm up it finds the floor where a standing body finds the shelf. A
/// walking body's sphere is 10 cm: beside a 5 cm step whose edge runs 7 cm
/// from the probe it meets the edge at 5 + √(10² − 7²) − 10 = 2.14 cm,
/// where the 5 cm sphere of a standing or a down body finds the floor.
#[test]
fn down_the_query_starts_at_the_ankle_5_cm_wide() {
    // The right leg's probe: 5 cm ahead of its ankle at (−7.8, 9.4, 1.2) cm.
    let probe = Vec2::new(-0.078, 0.062);
    let mut shelf = floor();
    shelf.extend(square(probe, 0.4, 0.05));
    let mut step = floor();
    step.extend(square(Vec2::new(probe.x + 0.07 + 0.5, probe.y), 0.05, 0.5));
    let ground = |level: &[[Vec3; 3]], how: &dyn Fn(&mut Ragdoll)| {
        let mut body = biped();
        how(&mut body);
        body.measure_balance(level);
        body.legs()[0].ground.unwrap()
    };
    let standing = |_: &mut Ragdoll| {};
    let lying = |body: &mut Ragdoll| body.fall();
    let getting_up = |body: &mut Ragdoll| {
        bearing(
            body,
            Bearing {
                off_balance: true,
                getting_up: true,
                ..Bearing::default()
            },
        )
    };
    let walking = |body: &mut Ragdoll| {
        bearing(
            body,
            Bearing {
                walking: true,
                ..Bearing::default()
            },
        )
    };
    assert!((ground(&shelf, &standing) - 0.4).abs() < 1e-5);
    assert!((ground(&shelf, &walking) - 0.4).abs() < 1e-5);
    assert!((ground(&step, &walking) - 0.0214).abs() < 1e-4);
    assert!(ground(&step, &standing).abs() < 1e-5);
    for down in [&lying as &dyn Fn(&mut Ragdoll), &getting_up] {
        assert!(ground(&shelf, down).abs() < 1e-5);
        assert!(ground(&step, down).abs() < 1e-5);
    }
}

/// At the gait bump's peak the first foot's sphere is 20 cm and its probe
/// sits 36 cm further along the facing than the plain 5 cm offset, so a
/// platform there is the ground. At a phase with no bump the same platform
/// is missed and the foot finds the floor.
#[test]
fn a_walking_probe_sways_forward_and_widens() {
    let peak = 0.125f32.sqrt();
    let swayed = Vec2::new(-0.078, 0.062 + 0.36);
    let mut level = floor();
    level.extend(square(swayed, 0.2, 0.04));
    let ground = |progress: f32| {
        let mut body = biped();
        bearing(
            &mut body,
            Bearing {
                walking: true,
                ..Bearing::default()
            },
        );
        body.drive.as_mut().unwrap().note_gait(progress, 0.0, false);
        body.measure_balance(level.as_slice());
        body.legs()[0].ground.unwrap()
    };
    assert!(
        (ground(2.0 - peak) - 0.2).abs() < 1e-4,
        "{}",
        ground(2.0 - peak)
    );
    assert!(ground(0.0).abs() < 1e-4);
}

/// Getting up, balance rises 0.05 a tick and the counter falls by 5 while
/// above 2; a body with no foot grounded and its pelvis within 30 cm of the
/// ground under it has support at half its balance.
#[test]
fn getting_up_finds_support_near_the_ground() {
    // Upside down, the pelvis 20 cm off the floor and the feet in the air.
    let flip = Affine3A::from_translation(cm(0.0, 20.0, 0.0))
        * Affine3A::from_rotation_x(std::f32::consts::PI)
        * Affine3A::from_translation(cm(0.0, -103.0, 0.0));
    let mut body = biped_at(flip);
    body.drive.as_mut().unwrap().balance = Balance {
        value: 0.0,
        counter: 100,
        ..Balance::default()
    };
    bearing(
        &mut body,
        Bearing {
            off_balance: true,
            getting_up: true,
            ..Bearing::default()
        },
    );
    body.measure_balance(floor().as_slice());
    let b = drive(&body).balance();
    assert!(body.legs().iter().all(|l| !l.grounded));
    assert_eq!((b.value, b.counter()), (0.05, 95));
    assert!(b.has_support());
    assert!((drive(&body).support() - 0.025).abs() < 1e-6);
    for _ in 0..30 {
        body.measure_balance(floor().as_slice());
    }
    let b = drive(&body).balance();
    assert_eq!((b.value, b.counter()), (1.0, 0));
    assert_eq!(drive(&body).support(), 0.5);
}

/// The Height Query sweeps a sphere down: it finds the floor under it, the
/// edge of a step 5 cm beside its centre line where its lowest point is
/// 30 + √(10² − 5²) − 10 = 28.66 cm up, passes faces turned down, finds a
/// face it starts against where it starts, ignores one it starts behind,
/// and nothing past its depth.
#[test]
fn the_height_query_sweeps_a_sphere_down() {
    let ground = floor();
    let at = Vec3::new(0.0, 0.7, 0.0);
    let found = |level: &[[Vec3; 3]], at: Vec3| level.height_below(at, 0.1, 20.0);
    assert!(found(&ground, at).unwrap().abs() < 1e-5);
    let mut step = floor();
    step.extend(square(Vec2::new(0.55, 0.0), 0.3, 0.5));
    let h = found(&step, at).unwrap();
    assert!((h - 0.2866).abs() < 1e-4, "{h}");
    let mut under = floor();
    under.extend(
        square(Vec2::ZERO, 0.4, 1.0)
            .iter()
            .map(|t| [t[0], t[2], t[1]]),
    );
    assert!(found(&under, at).unwrap().abs() < 1e-5);
    assert_eq!(found(&ground, Vec3::new(0.0, -0.05, 0.0)), Some(-0.05));
    assert_eq!(found(&ground, Vec3::new(0.0, -0.5, 0.0)), None);
    assert_eq!(found(&ground, Vec3::new(0.0, 25.0, 0.0)), None);
}

/// The Height Query's drop of a sphere onto one triangle (face, edges and
/// corners, 0 when it starts against it) is where the dropping centre first
/// comes within its radius of the triangle, for upward faces with the
/// sphere in front of them.
#[test]
fn a_sphere_drops_onto_a_triangle_where_it_first_touches() {
    use rapier3d::parry::query::PointQuery;
    use rapier3d::parry::shape::Triangle;
    let mut seed = 0x2545_f491_u32;
    let mut next = move || {
        seed ^= seed << 13;
        seed ^= seed >> 17;
        seed ^= seed << 5;
        (seed as f32 / u32::MAX as f32) * 2.0 - 1.0
    };
    let (mut hits, mut misses) = (0, 0);
    for _ in 0..4000 {
        let t = [0; 3].map(|_| Vec3::new(next() * 0.3, next() * 0.2, next() * 0.3));
        let c = Vec3::new(next() * 0.3, 0.15 + next() * 0.3, next() * 0.3);
        let r = 0.05 + 0.1 * next().abs();
        let n = (t[1] - t[0]).cross(t[2] - t[0]);
        if n.y <= 1e-3 || n.dot(c - t[0]) < 0.0 {
            continue;
        }
        let triangle = Triangle::new(t[0], t[1], t[2]);
        let gap = |d: f32| {
            let x = c - Vec3::Y * d;
            x.distance(triangle.project_local_point(x, true).point)
        };
        let reach = 0.6;
        let drop = crate::contact::drop_onto(&t, c, r, reach);
        let end = drop.unwrap_or(reach);
        for k in (0..200).filter(|_| end > 0.0) {
            let d = end * k as f32 / 200.0;
            assert!(
                gap(d) > r - 1e-4,
                "{t:?} {c} {r}: touches at {d}, not {drop:?}"
            );
        }
        match drop {
            Some(0.0) => assert!(gap(0.0) <= r + 1e-4),
            Some(d) => assert!((gap(d) - r).abs() < 1e-4, "{t:?} {c} {r}: {d}"),
            None => {}
        }
        if drop.is_some() {
            hits += 1;
        } else {
            misses += 1;
        }
    }
    assert!(hits > 300 && misses > 300, "{hits} hits, {misses} misses");
}

/// A stance facing a quarter turn from the frame swings an ankle that
/// stood 1 m ahead around to 1 m toward +X, and keeps its height. The
/// same yaw, and a yaw that is not finite, leave the clip's ankle.
#[test]
fn the_stance_facing_turns_the_spot() {
    use std::f32::consts::FRAC_PI_2;
    let spot = Vec3::new(0.0, 1.2, 1.0);
    let turned = stance_spot(spot, Vec3::ZERO, 0.0, FRAC_PI_2);
    assert!((turned.x - 1.0).abs() < 1e-5, "{}", turned.x);
    assert!((turned.z).abs() < 1e-5, "{}", turned.z);
    assert_eq!(turned.y, 1.2);
    let same = stance_spot(spot, Vec3::ZERO, 0.4, 0.4);
    assert!((same - spot).length() < 1e-5, "{same}");
    assert_eq!(stance_spot(spot, Vec3::ZERO, 0.0, f32::NAN), spot);
}

/// A reset facing is the chest over the pelvis, flattened: a sloped
/// direction keeps only its horizontal part; a vertical one is zero.
#[test]
fn the_reset_heading() {
    let h = super::reset_heading(Vec3::new(3.0, 2.0, 4.0), Vec3::new(0.0, 1.0, 0.0));
    assert!((h - Vec2::new(0.6, 0.8)).length() < 1e-6);
    let h = super::reset_heading(Vec3::new(0.0, 2.0, 0.0), Vec3::new(0.0, 1.0, 0.0));
    assert_eq!(h, Vec2::ZERO);
    assert_eq!(super::reset_heading(Vec3::ZERO, Vec3::ZERO), Vec2::ZERO);
}

/// A walking foot that trails its animated spot cuts the push by
/// (1 − (lead/60)²) on each leg, and a touching foot whose lead (times the
/// alignment, 1 here) is over its swing threshold takes the swing flag
/// (0x10). At progress 0 the thresholds are 10·(0.25 − frac(0.5·leg + 1)):
/// 2.5 cm for leg 0 and −2.5 cm for leg 1, each capped at 25 cm with no
/// ground difference. A 2 cm lead swings leg 1 only; 30 cm swings both.
#[test]
fn the_late_walking_foot_swings_and_cuts_the_push() {
    let floor = floor();
    let mut body = biped();
    body.measure_balance(&floor[..]);
    body.set_bearing(Bearing {
        moving: true,
        walking: true,
        facing: Vec3::Z,
        align: 1.0,
        ..Bearing::default()
    });
    body.measure_balance(&floor[..]);
    assert_eq!(drive(&body).bearing.lag, 1.0);
    for leg in body.legs.iter() {
        assert!(!leg.state.has(FootState::LIFTED), "planted, not swinging");
    }
    let shins: Vec<usize> = body.legs.iter().map(|l| l.shin).collect();
    for &shin in &shins {
        for x in &mut body.parts[shin].positions {
            *x -= Vec3::Z * 0.02;
        }
    }
    body.measure_balance(&floor[..]);
    let touching = |l: &Leg| l.state.has(FootState::TOUCHING);
    if touching(&body.legs[0]) {
        assert!(
            !body.legs[0].state.has(FootState::LIFTED),
            "2 cm is under 2.5"
        );
    }
    if touching(&body.legs[1]) {
        assert!(
            body.legs[1].state.has(FootState::LIFTED),
            "2 cm is over −2.5"
        );
    }
    let one = 1.0 - (2.0f32 / 60.0).powi(2);
    assert!((drive(&body).bearing.lag - one * one).abs() < 1e-5);
    for &shin in &shins {
        for x in &mut body.parts[shin].positions {
            *x -= Vec3::Z * 0.28;
        }
    }
    body.measure_balance(&floor[..]);
    // Only a foot that touches gets the swing flag: a foot in the air is
    // already the gait's swing, and its flag is cleared.
    assert!(
        body.legs
            .iter()
            .any(|l| touching(l) && l.state.has(FootState::LIFTED)),
        "the late touching foot swings"
    );
    for leg in body.legs.iter().filter(|l| touching(l)) {
        assert!(leg.state.has(FootState::LIFTED), "30 cm is over both");
    }
    let half = 1.0 - (30.0f32 / 60.0).powi(2);
    assert!((drive(&body).bearing.lag - half * half).abs() < 1e-5);
}

/// The swing threshold's ground-difference rise is capped at 25 cm by the
/// processor's minimum (a NaN sum gives 25).
#[test]
fn the_swing_threshold_is_capped_at_25() {
    use crate::engine_math::min;
    assert_eq!(min(2.5 + 40.0, 25.0), 25.0);
    assert_eq!(min(2.5 + 10.0, 25.0), 12.5);
    assert_eq!(min(f32::NAN, 25.0), 25.0);
}

/// Walking backwards, the bearing's facing is already negated: a foot that
/// trails its animated spot toward the walk's direction (behind it along
/// the negated facing) cuts the push once, not twice negated into none.
#[test]
fn the_backward_lag_reads_the_negated_facing() {
    let floor = floor();
    let mut body = biped();
    body.measure_balance(&floor[..]);
    body.set_bearing(Bearing {
        moving: true,
        walking: true,
        facing: -Vec3::Z,
        backpedal: true,
        align: 1.0,
        ..Bearing::default()
    });
    if let Some(d) = body.drive.as_mut() {
        d.note_gait(0.0, 0.0, true);
    }
    body.measure_balance(&floor[..]);
    let shins: Vec<usize> = body.legs.iter().map(|l| l.shin).collect();
    for &shin in &shins {
        for x in &mut body.parts[shin].positions {
            *x += Vec3::Z * 0.1;
        }
    }
    body.measure_balance(&floor[..]);
    let one = 1.0 - (10.0f32 / 60.0).powi(2);
    assert!((drive(&body).bearing.lag - one * one).abs() < 1e-5);
}

/// The Ground Query is a point ray under the toe: it finds the floor and a
/// step's top under it, stops at its depth, ignores faces turned down and
/// skips a movable piece (a prop's stand-in), as the original's class-word
/// filter does.
#[test]
fn the_ground_query_is_a_point_ray_over_floor_pieces() {
    struct Pieces {
        tris: Vec<[Vec3; 3]>,
        movable: bool,
    }
    impl Level for Pieces {
        fn pieces(&self, _: Vec3, _: Vec3, out: &mut Vec<u32>) {
            out.push(0);
        }
        fn triangles(&self, _: u32, _: Vec3, _: Vec3, out: &mut Vec<[Vec3; 3]>) {
            out.extend_from_slice(&self.tris);
        }
        fn movable(&self, _: u32) -> bool {
            self.movable
        }
        // A floor-flagged mesh, unless it stands in for a movable prop.
        fn floor_piece(&self, _: u32) -> bool {
            !self.movable
        }
    }
    let mut tris = floor();
    tris.extend(square(Vec2::new(0.55, 0.0), 0.3, 0.5));
    let pieces = Pieces {
        tris: tris.clone(),
        movable: false,
    };
    let over = |x: f32, y: f32| Vec3::new(x, y, 0.0);
    assert_eq!(pieces.floor_below(over(0.0, 0.4), 0.6), Some(0.0));
    assert_eq!(pieces.floor_below(over(0.55, 0.4), 0.6), Some(0.3));
    // A point that starts below the step top is under it.
    assert_eq!(pieces.floor_below(over(0.55, 0.2), 0.6), Some(0.0));
    // 0.4 m to the floor is deeper than 0.3 m.
    assert_eq!(pieces.floor_below(over(0.0, 0.4), 0.3), None);
    // A ray inside the face's depth but past it: none.
    assert_eq!(pieces.floor_below(over(0.0, 0.4), 0.05), None);
    // Faces turned down are not floor.
    let down: Vec<[Vec3; 3]> = floor().iter().map(|t| [t[0], t[2], t[1]]).collect();
    let flipped = Pieces {
        tris: down,
        movable: false,
    };
    assert_eq!(flipped.floor_below(over(0.0, 0.4), 0.6), None);
    // A movable piece is some body's stand-in, not the level's floor.
    let stand_in = Pieces {
        tris,
        movable: true,
    };
    assert_eq!(stand_in.floor_below(over(0.0, 0.4), 0.6), None);
    // A NaN or negative depth asks nothing.
    assert_eq!(pieces.floor_below(over(0.0, 0.4), f32::NAN), None);
    assert_eq!(pieces.floor_below(over(0.0, 0.4), -1.0), None);
}

/// The Ground Query's hit raises a moving body's foot ground: the biped
/// stands on the low floor with its ankles at z ≈ 1 cm, and a step top
/// 15 cm up starts under its toes (20 cm ahead, at z ≈ 21 cm). The query
/// finds the step and the foot's ground and floor rise to it; a still body
/// keeps the ground its Height Query found.
#[test]
fn the_ground_query_raises_a_moving_feet_ground() {
    let mut level = floor();
    level.extend(square(Vec2::new(0.0, 0.30), 0.15, 0.15));
    let mut body = biped();
    body.measure_balance(&level[..]);
    body.set_bearing(Bearing {
        moving: true,
        walking: true,
        facing: Vec3::Z,
        align: 1.0,
        ..Bearing::default()
    });
    body.measure_balance(&level[..]);
    let grounds: Vec<f32> = body.legs.iter().map(|l| l.ground.unwrap_or(-1.0)).collect();
    assert!(
        grounds.iter().all(|g| (g - 0.15).abs() < 1e-4),
        "{grounds:?}"
    );
    for leg in body.legs.iter() {
        assert!((leg.floor - 0.15).abs() < 1e-4, "{}", leg.floor);
    }
    let mut still = biped();
    still.measure_balance(&level[..]);
    still.measure_balance(&level[..]);
    for leg in still.legs.iter() {
        assert!(
            leg.ground.is_none_or(|g| g.abs() < 1e-4),
            "{:?}",
            leg.ground
        );
    }
}

/// The placement's leg snap: a biped put on a 15 cm platform finds that
/// ground under each stance spot with the 10 cm sphere dropped from the
/// frame's height plus its leg, sets each anchor to the ground plus the
/// sole, its ground and floor to the hit, the character's ground to the
/// average, and writes the standing IK's corrections at once (their rates
/// zero, no thigh turn).
#[test]
fn the_placement_snaps_the_legs_onto_the_ground_it_finds() {
    let platform = square(Vec2::ZERO, 0.15, 10.0);
    let mut body = biped();
    let pose: Vec<Affine3A> = body.parts.iter().map(|p| p.transform()).collect();
    body.aim(Affine3A::IDENTITY, pose);
    let height = body
        .snap_placement(&platform[..], Affine3A::IDENTITY)
        .expect("the platform is under both feet");
    assert!(height.is_finite());
    for leg in body.legs.iter() {
        assert!(
            (leg.ground.unwrap() - 0.15).abs() < 1e-4,
            "{:?}",
            leg.ground
        );
        assert!((leg.floor - 0.15).abs() < 1e-4);
        assert!((leg.anchor.y - (0.15 + body.sole)).abs() < 1e-4);
        assert_eq!(leg.knee_rate, 0.0);
        assert_eq!(leg.hip_rate, 0.0);
        assert_eq!(leg.turn, Quat::IDENTITY);
    }
    assert!((drive(&body).balance().under - 0.15).abs() < 1e-4);
    // No ground under the feet leaves the body alone.
    let mut empty = biped();
    let pose: Vec<Affine3A> = empty.parts.iter().map(|p| p.transform()).collect();
    empty.aim(Affine3A::IDENTITY, pose);
    assert_eq!(empty.snap_placement(NONE, Affine3A::IDENTITY), None);
}

/// The snap's character ground is where the placement puts the feet: a 15 cm
/// platform puts them at 15 cm, not at the pelvis's frame height.
#[test]
fn the_placement_frame_height_maps_to_the_feet() {
    let platform = square(Vec2::ZERO, 0.15, 10.0);
    let mut body = biped();
    let pose: Vec<Affine3A> = body.parts.iter().map(|p| p.transform()).collect();
    body.aim(Affine3A::IDENTITY, pose);
    body.snap_placement(&platform[..], Affine3A::IDENTITY)
        .unwrap();
    let feet = body.placement_feet();
    assert!((feet - 0.15).abs() < 1e-4, "feet {feet}");
}

/// The leg order: the biped's first thigh hangs at −9 cm (its right), so
/// its legs keep their order; mirrored, the first thigh at +9 cm swaps
/// them. A single leg never swaps.
#[test]
fn the_first_leg_is_the_one_on_the_negative_side() {
    let setup = |x: f32| -> Vec<PartSetup> {
        BIPED
            .iter()
            .map(|b| PartSetup {
                bone: b.bone,
                entity: Entity::PLACEHOLDER,
                parent: b.parent,
                world: Affine3A::IDENTITY,
                centre: Vec3::ZERO,
                half: Vec3::splat(0.01),
                origin: if b.bone == 2 {
                    Vec3::new(x, -0.085, 0.035)
                } else {
                    Vec3::from_array(b.origin) * 0.01
                },
                links: vec![],
                foot: None,
                pins: vec![],
                spheres: None,
            })
            .collect()
    };
    // Part 2 is the first lower leg (bone 3), under the thigh (part 1).
    assert!(!legs_swap(&setup(-0.09), 2, 2));
    assert!(legs_swap(&setup(0.09), 2, 2));
    assert!(!legs_swap(&setup(0.09), 2, 1));
    assert!(!legs_swap(&setup(f32::NAN), 2, 2));
}

/// A body off its feet takes its ground from one 10 cm sphere cast down
/// from its pelvis: over a 4 cm pedestal 30 cm up between its feet (which
/// its feet's 5 cm spheres, 7.8 cm to either side, miss) its ground is the
/// pedestal while its feet stand on the floor and give it support. On its
/// feet the same body's ground is its lowest foot's.
#[test]
fn a_down_body_takes_its_ground_from_under_its_pelvis() {
    let mut level = floor();
    level.extend(square(Vec2::ZERO, 0.3, 0.02));
    let mut standing = biped();
    standing.measure_balance(level.as_slice());
    assert!(drive(&standing).balance.under.abs() < 1e-5);
    let mut down = biped();
    bearing(
        &mut down,
        Bearing {
            off_feet: true,
            ..Bearing::default()
        },
    );
    down.measure_balance(level.as_slice());
    let b = &drive(&down).balance;
    assert!(b.has_support());
    assert!((b.under - 0.3).abs() < 1e-5, "{}", b.under);
    // With no support the feet routine's own probe under the pelvis
    // replaces it: over nothing, 100 m down.
    down.measure_balance(NONE);
    assert_eq!(drive(&down).balance.under, -100.0);
}

/// The flat-floor bit puts every foot on a floor at 0 without a probe: its
/// ground 0, its anchor at the sole plus 1 cm, its state touching, reached
/// and ramp-held, its floor 1 cm; the body has full support over nothing
/// at all, and its lift aims 10 cm under the target it has on a real floor.
#[test]
fn a_flat_floor_body_stands_on_0_without_probing() {
    let mut real = biped();
    real.measure_balance(floor().as_slice());
    real.lift();
    let walking = drive(&real).lift_target.unwrap();
    let mut flat = biped();
    bearing(
        &mut flat,
        Bearing {
            splay: true,
            ..Bearing::default()
        },
    );
    flat.measure_balance(NONE);
    let sole = flat.sole;
    for leg in flat.legs() {
        assert_eq!(leg.ground, Some(0.0));
        assert!((leg.anchor.y - (sole + 0.01)).abs() < 1e-6);
        assert_eq!(leg.state, feet::FLAT_FLOOR_FOOT);
        assert_eq!(leg.state.0, 0x4a);
        assert!((leg.floor - 0.01).abs() < 1e-6);
    }
    let d = drive(&flat);
    assert_eq!(d.feet_mode, feet::FeetMode::FlatFloor);
    assert!(d.balance.has_support());
    assert_eq!(d.balance.under, 0.0);
    assert_eq!(d.support, 1.0);
    flat.lift();
    let target = drive(&flat).lift_target.unwrap();
    assert!(
        (target - (walking - 0.1)).abs() < 1e-4,
        "{target} vs {walking}"
    );
}

/// A hovering race probes once under its pelvis with a 20 cm sphere from
/// 20 cm over it: a hit is its ground, a miss raises the next probe 5 cm.
/// It has full support either way and its feet are not probed.
#[test]
fn a_hovering_body_probes_once_under_its_pelvis() {
    let mut body = biped();
    body.drive.as_mut().unwrap().race.hovers = true;
    let before: Vec<Option<f32>> = body.legs().iter().map(|l| l.ground).collect();
    body.measure_balance(NONE);
    let d = drive(&body);
    assert_eq!(d.feet_mode, feet::FeetMode::Hovering);
    assert_eq!(d.hover.raise, 5.0);
    assert!(d.balance.has_support());
    assert_eq!(d.support, 1.0);
    body.measure_balance(square(Vec2::ZERO, -0.5, 10.0).as_slice());
    let d = drive(&body);
    assert_eq!(d.hover.raise, 0.0);
    assert!((d.balance.under + 0.5).abs() < 1e-5, "{}", d.balance.under);
    let after: Vec<Option<f32>> = body.legs().iter().map(|l| l.ground).collect();
    assert_eq!(before, after);
    // The flat-floor bit wins over the hover flag.
    bearing(
        &mut body,
        Bearing {
            splay: true,
            ..Bearing::default()
        },
    );
    body.measure_balance(NONE);
    assert_eq!(drive(&body).feet_mode, feet::FeetMode::FlatFloor);
}
