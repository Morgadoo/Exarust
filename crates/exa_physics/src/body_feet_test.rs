//! The feet: the original's rules, each pinned to its numbers
//! (`docs/formats/RSQ.md`, Physical body, Feet).
//! (Included from body_feet.rs: `#[path = "body_feet_test.rs"] mod tests;`)
use bevy::math::Affine3A;

use super::super::floor_test::humanoid;
use super::super::tests::floor_quad;
use super::super::{Drive, PartSetup, Ragdoll};
use super::*;

fn close(a: f32, b: f32, tolerance: f32) -> bool {
    (a - b).abs() <= tolerance
}

fn drive(body: &Ragdoll) -> &Drive {
    body.drive.as_ref().unwrap()
}

/// The feet's sole height (m): their boxes reach 10 cm under the ankles,
/// less 0.5 cm.
const SOLE: f32 = 0.095;

/// The standing humanoid on feet: each lower leg ends at an ankle 10 cm
/// over `at`, where a foot hangs whose box (24 cm long, 10 cm deep, its
/// toe ahead along +z) stands on `at`. Its lift reads its legs 3.8 cm
/// longer than hip to ankle, so that its balance point puts its pelvis
/// 1.2 m over `at` with its legs straight and its ankles 0.5 cm over their
/// anchors.
fn footed(at: Vec3) -> (Ragdoll, Vec<Affine3A>) {
    let raised = at + Vec3::Y * 0.1;
    let (mut setup, mut pose) = humanoid(raised);
    for (shin, x) in [(2, 0.1), (4, -0.1)] {
        let world = Affine3A::from_translation(raised + Vec3::new(x, 0.0, 0.0));
        let along = Vec3::new(0.0, -0.05, 0.05).normalize() * 0.12;
        setup.push(PartSetup {
            bone: if shin == 2 { 4 } else { 7 },
            entity: Entity::PLACEHOLDER,
            parent: Some(shin),
            world,
            centre: Vec3::new(0.0, -0.05, 0.05),
            half: Vec3::new(0.05, 0.05, 0.12),
            origin: Vec3::new(0.0, -0.5, 0.0),
            links: vec![(Vec3::X * 0.04, Vec3::X * 0.04, 0.0), (along, along, 0.12)],
            foot: None,
            pins: Vec::new(),
            spheres: None,
        });
        pose.push(world);
        setup[shin].foot = Some(Vec3::new(0.0, -0.5, 0.0));
    }
    for s in &mut setup {
        if s.bone == 8 {
            s.pins = vec![Vec3::new(0.15, 0.4, 0.0), Vec3::new(-0.15, 0.4, 0.0)];
        }
    }
    let mut body = Ragdoll::new(&setup).with_drive();
    for leg in &mut body.legs {
        leg.length = 1.0 + 0.0383;
    }
    (body, pose)
}

/// One character tick and one solver tick of a body in `pose`, its root at
/// `root`, on `level`.
fn stand(body: &mut Ragdoll, root: Affine3A, pose: &[Affine3A], level: &[[Vec3; 3]]) {
    body.measure_balance(level);
    body.aim(root, pose.to_vec());
    body.tick(level);
}

/// What the feet take from a standing human at full strength.
fn full() -> FootDrive {
    FootDrive {
        driven: true,
        support: 1.0,
        off_feet: false,
        has_support: true,
        walking: false,
        moving: false,
        run: 0.0,
        scale: 1.0,
        gain: 1.25,
        sole_gain: 1.25,
        pelvis: 1.2,
    }
}

/// The contact's point now, and Σw² of its weights (a push moves the point
/// by that share of itself).
fn ankle(body: &Ragdoll, leg: usize) -> (Vec3, f32) {
    let l = &body.legs[leg];
    let w2 = l.contact.iter().map(|w| w * w).sum();
    (body.parts[l.shin].point(&l.contact), w2)
}

/// A leg's friction (its lower leg's and its foot's) is 0 while its foot
/// does not touch, 2.0 touching, 0 anchored; lifted, it is still 2.0.
#[test]
fn legs_take_their_friction_from_their_feet() {
    let f = |bits: u32| leg_friction(FootState(bits));
    assert_eq!(f(0), 0.0);
    assert_eq!(f(FootState::TOUCHING), 2.0);
    assert_eq!(f(FootState::TOUCHING | FootState::ANCHORED), 0.0);
    assert_eq!(f(FootState::ANCHORED), 0.0);
    assert_eq!(f(FootState::TOUCHING | FootState::LIFTED), 2.0);
    // Standing on a floor: both feet touch, the lower legs and feet take
    // 2.0. Walking at progress 0, the near foot (phase 0.2) stays and the
    // far foot (phase 0.7) lifts.
    let floor = floor_quad(0.0, 5.0);
    let (mut body, pose) = footed(Vec3::ZERO);
    // Planted once their feet have met the floor.
    for _ in 0..2 {
        stand(&mut body, Affine3A::IDENTITY, &pose, &floor);
    }
    let friction = |body: &Ragdoll| {
        body.legs
            .iter()
            .flat_map(|l| {
                [
                    body.parts[l.shin].friction,
                    body.parts[l.foot.unwrap()].friction,
                ]
            })
            .collect::<Vec<f32>>()
    };
    assert_eq!(friction(&body), [2.0; 4]);
    body.drive.as_mut().unwrap().bearing.walking = true;
    body.drive.as_mut().unwrap().bearing.progress = 0.0;
    stand(&mut body, Affine3A::IDENTITY, &pose, &floor);
    assert_eq!(friction(&body), [2.0, 2.0, 0.0, 0.0]);
    assert_eq!(body.legs[0].state, FootState(FootState::TOUCHING));
    assert_eq!(body.legs[1].state, FootState(0));
}

/// Walk phase 0 plants the free near foot and lifts the touching far foot.
#[test]
fn a_walk_plants_and_lifts_by_its_phase() {
    assert!(close(gait_phase(0.0, 0), 0.2, 1e-6));
    assert!(close(gait_phase(0.0, 1), 0.7, 1e-6));
    assert_eq!(
        gait_plant(gait_phase(0.0, 0), false, 0.5, false),
        GaitPlant::Plant
    );
    assert_eq!(
        gait_plant(gait_phase(0.0, 0), true, 0.5, false),
        GaitPlant::Hold
    );
    assert_eq!(
        gait_plant(gait_phase(0.0, 1), true, 0.5, false),
        GaitPlant::Lift
    );
    assert_eq!(
        gait_plant(gait_phase(0.0, 1), false, 0.5, false),
        GaitPlant::Hold
    );
    assert_eq!(gait_plant(0.0, true, 0.5, false), GaitPlant::Lift);
    assert_eq!(gait_plant(0.41, false, 0.5, false), GaitPlant::Hold);
}

/// The servo pushes a still point d·rate·gain (10 cm off at rate 0.2 and
/// gain 0.05: 0.1 cm); one already closing 4 % of its gap a tick is pushed
/// d·(0.2 − 0.04)·gain, one closing 5 % or more not at all.
#[test]
fn the_servo_closes_its_rate_of_the_gap() {
    let d = Vec3::new(10.0, 0.0, 0.0);
    let push = |moved: f32| servo(d, Vec3::X * moved, 0.05, 0.2);
    assert!(push(0.0).is_some_and(|p| close(p.x, 0.1, 1e-6) && p.y == 0.0));
    assert!(push(0.4).is_some_and(|p| close(p.x, 0.08, 1e-6)));
    assert_eq!(push(0.5), None);
    assert_eq!(servo(Vec3::ZERO, Vec3::ZERO, 1.0, 0.2), None);
}

/// A full grip pulls a tenth of the gap and takes back the point's whole
/// motion across the ground; with none, gain 1.25, a spring of 0.005 and a
/// damping of 0.0375. Over its anchor the hold is kept within 1 cm, halved
/// at 5 cm (0.5·(9 − 5)·0.125), gone past 9 cm, and the anchor is not
/// pulled down to.
#[test]
fn the_grip_holds_and_lets_go_over_the_anchor() {
    let moved = Vec3::new(0.5, 0.3, 0.0);
    let p = grip(Vec3::new(2.0, 1.0, 0.0), moved, 1.25, 1.0).unwrap();
    assert!(close(p.x, 0.2 - 0.5, 1e-6) && close(p.y, 0.1, 1e-6) && p.z == 0.0);
    let p = grip(Vec3::new(2.0, 0.0, 0.0), moved, 1.25, 0.0).unwrap();
    assert!(close(p.x, 2.0 * 0.005 - 0.5 * 0.0375, 1e-6) && p.y == 0.0);
    // Over the anchor: the height gap is dropped and the hold fades.
    let held = |above: f32| grip(Vec3::new(2.0, -above, 0.0), Vec3::ZERO, 0.0, 1.0).unwrap();
    assert!(close(held(1.0).x, 0.2, 1e-6) && held(1.0).y == 0.0);
    assert!(close(held(5.0).x, 0.2 * 0.5, 1e-6));
    assert_eq!(held(9.5).x, 0.0);
}

/// Standing, a foot's grip rises 0.001 an iteration to 1 (1000 iterations,
/// 50 ticks), its leg's ramp 0.001 to 0.25 (250 iterations); held, the ramp
/// is 0.25 at once; rising slowly, 0.0001 an iteration until it passes
/// 0.25, which ends the slow rise.
#[test]
fn the_grip_rises_over_1000_iterations() {
    let (mut body, _) = footed(Vec3::ZERO);
    let fd = full();
    let at = ankle(&body, 0).0;
    let run = |body: &mut Ragdoll, n: usize| {
        let Ragdoll { parts, legs, .. } = body;
        for _ in 0..n {
            drive_contact(&mut parts[legs[0].shin], &mut legs[0], &fd);
        }
    };
    {
        let leg = &mut body.legs[0];
        leg.state = FootState(FootState::TOUCHING);
        leg.anchor = at;
    }
    run(&mut body, 250);
    assert!(close(body.legs[0].ramp, 0.25, 1e-6) && close(body.legs[0].grip, 0.25, 1e-4));
    run(&mut body, 750);
    assert!(close(body.legs[0].grip, 1.0, 1e-4), "{}", body.legs[0].grip);
    assert_eq!(body.legs[0].approach, 1.0);
    run(&mut body, 10);
    assert_eq!(body.legs[0].grip, 1.0);
    // Held.
    body.legs[0].ramp = 0.0;
    body.legs[0].state = FootState(FootState::TOUCHING | FootState::RAMP_HELD);
    run(&mut body, 1);
    assert_eq!(body.legs[0].ramp, 0.25);
    // Rising slowly.
    body.legs[0].ramp = 0.2;
    body.legs[0].state = FootState(FootState::TOUCHING | FootState::RAMP_SLOW);
    run(&mut body, 499);
    assert!(body.legs[0].state.has(FootState::RAMP_SLOW));
    assert!(close(body.legs[0].ramp, 0.2499, 1e-4));
    run(&mut body, 3);
    assert!(!body.legs[0].state.has(FootState::RAMP_SLOW));
    assert_eq!(body.legs[0].ramp, 0.25);
}

/// A step's foot reaches its anchor within 5 cm: 4.9 cm off it is reached
/// (flag 8, a landing), its anchor moves halfway to it across the ground,
/// its approach is 0.95 of what it was plus 0.05; 5.1 cm off its approach
/// follows how fast it closes. Either way it has no grip and its leg's
/// ramp rises 0.01.
#[test]
fn a_step_reaches_its_anchor_within_5_cm() {
    let (mut body, _) = footed(Vec3::ZERO);
    let fd = full();
    let at = ankle(&body, 0).0;
    let Ragdoll { parts, legs, .. } = &mut body;
    let leg = &mut legs[0];
    leg.state = FootState(FootState::TOUCHING | FootState::ANCHORED);
    leg.approach = 1.0;
    leg.grip = 0.7;
    leg.anchor = at + Vec3::new(0.051, 0.0, 0.0);
    assert!(!drive_contact(&mut parts[leg.shin], leg, &fd));
    assert!(!leg.state.has(FootState::REACHED));
    // Still, it closes nothing: 0.95 of its approach.
    assert!(close(leg.approach, 0.95, 1e-6), "{}", leg.approach);
    assert_eq!((leg.grip, leg.ramp), (0.0, 0.01));
    let now = leg.at;
    leg.anchor = now + Vec3::new(0.049, 0.0, 0.0);
    let anchor = leg.anchor;
    assert!(drive_contact(&mut parts[leg.shin], leg, &fd));
    assert!(leg.state.has(FootState::REACHED));
    assert!(close(leg.approach, 0.95 * 0.95 + 0.05, 1e-6));
    let halfway = (leg.at + anchor) * 0.5;
    assert!(close(leg.anchor.x, halfway.x, 1e-6) && close(leg.anchor.z, halfway.z, 1e-6));
}

/// A lifted foot, or one that does not touch, is only pushed up toward its
/// anchor: never across the ground and never down.
#[test]
fn a_lifted_foot_moves_only_up() {
    for bits in [FootState::TOUCHING | FootState::LIFTED, 0] {
        for walking in [false, true] {
            let (mut body, _) = footed(Vec3::ZERO);
            let fd = FootDrive { walking, ..full() };
            let (start, _) = ankle(&body, 0);
            let Ragdoll { parts, legs, .. } = &mut body;
            let leg = &mut legs[0];
            leg.state = FootState(bits);
            leg.grip = 1.0;
            leg.anchor = start + Vec3::new(0.05, 0.03, 0.04);
            for _ in 0..20 {
                drive_contact(&mut parts[leg.shin], leg, &fd);
            }
            let now = parts[leg.shin].point(&leg.contact);
            assert!(close(now.x, start.x, 1e-6) && close(now.z, start.z, 1e-6));
            assert!(
                now.y > start.y + 0.001,
                "{bits:#x} walking {walking}: {now}"
            );
            assert_eq!(leg.ramp, 0.0);
            // An anchor under it pushes nothing.
            leg.anchor = now - Vec3::new(0.05, 0.03, 0.04);
            for _ in 0..20 {
                drive_contact(&mut parts[leg.shin], leg, &fd);
            }
            assert_eq!(parts[leg.shin].point(&leg.contact), now);
        }
    }
}

/// The drive leaves a foot alone in a body at half its tone or less or
/// without support, and a foot less than 10 cm under the pelvis (a body
/// lying down); its point is still refreshed.
#[test]
fn the_drive_needs_tone_support_and_a_foot_under_the_pelvis() {
    for fd in [
        FootDrive {
            driven: false,
            ..full()
        },
        FootDrive {
            pelvis: 0.19,
            ..full()
        },
    ] {
        let (mut body, _) = footed(Vec3::ZERO);
        let (start, _) = ankle(&body, 0);
        let Ragdoll { parts, legs, .. } = &mut body;
        let leg = &mut legs[0];
        leg.state = FootState(FootState::TOUCHING);
        leg.grip = 1.0;
        leg.anchor = start + Vec3::new(0.05, 0.0, 0.0);
        leg.at = Vec3::ZERO;
        drive_contact(&mut parts[leg.shin], leg, &fd);
        assert_eq!(parts[leg.shin].point(&leg.contact), start);
        assert_eq!((leg.at, leg.grip), (start, 1.0));
    }
    // An unsupported body's FootDrive is not driven, nor one at half tone.
    let (mut body, _) = footed(Vec3::ZERO);
    body.drive.as_mut().unwrap().tone = 0.5;
    assert!(!FootDrive::of(drive(&body)).driven);
    body.drive.as_mut().unwrap().tone = 0.51;
    assert!(FootDrive::of(drive(&body)).driven);
    // No ground under its feet: no support.
    let nothing: Vec<[Vec3; 3]> = Vec::new();
    body.measure_balance(nothing.as_slice());
    assert!(!drive(&body).balance().has_support());
    assert!(!FootDrive::of(drive(&body)).driven);
}

/// The toe (the sole's point 20 cm ahead of the ankle) is pushed straight
/// up to its floor by the gap × S, at most 0.5 cm an iteration while its
/// foot stands, 0.1 cm otherwise; not at all off its feet or without
/// support.
#[test]
fn the_toe_is_held_over_its_floor() {
    let rise = |bits: u32, support: f32, gap: f32, fd: FootDrive| {
        let (mut body, _) = footed(Vec3::ZERO);
        let Ragdoll { parts, legs, .. } = &mut body;
        let leg = &mut legs[0];
        let foot = &mut parts[leg.foot.unwrap()];
        let toe = foot.point(&leg.toe);
        assert!(close(toe.y, 0.005, 1e-5), "the toe stands at {}", toe.y);
        leg.state = FootState(bits);
        leg.floor = toe.y + gap;
        let w2: f32 = leg.toe.iter().map(|w| w * w).sum();
        push_sole(foot, leg, &FootDrive { support, ..fd });
        let now = foot.point(&leg.toe);
        assert!(close(now.x, toe.x, 1e-6) && close(now.z, toe.z, 1e-6));
        (now.y - toe.y) / w2 * CM
    };
    let standing = FootState::TOUCHING;
    assert!(close(rise(standing, 1.0, 0.03, full()), 0.5, 1e-3));
    assert!(close(rise(standing, 1.0, 0.003, full()), 0.3, 1e-3));
    assert!(close(rise(standing, 0.5, 0.004, full()), 0.2, 1e-3));
    assert!(close(rise(0, 1.0, 0.03, full()), 0.1, 1e-3));
    assert!(close(
        rise(standing | FootState::LIFTED, 1.0, 0.03, full()),
        0.1,
        1e-3
    ));
    assert!(close(
        rise(standing | FootState::ANCHORED, 1.0, 0.03, full()),
        0.1,
        1e-3
    ));
    assert!(close(rise(standing, 1.0, -0.01, full()), 0.0, 1e-6));
    let off_feet = FootDrive {
        off_feet: true,
        ..full()
    };
    assert_eq!(rise(standing, 1.0, 0.03, off_feet), 0.0);
    let unsupported = FootDrive {
        has_support: false,
        ..full()
    };
    assert_eq!(rise(standing, 1.0, 0.03, unsupported), 0.0);
}

/// After the sole push, a foot whose forward lies within a right angle of
/// its animated forward is servoed so its toe meets that orientation: the
/// horizontal gap, projected on the orientation's side axis, closes at
/// rate 0.4×scale with gain P×pin factor×S. Two centimetres along that
/// axis, unmoved, S = 1, P = 1 and pin factor 1.25, is a 1 cm push. A foot
/// turned past a right angle, one already closing 0.05 of the gap, and one
/// off its feet or without support, are left.
#[test]
fn the_sole_yaws_toward_its_animated_side() {
    let (body, _) = footed(Vec3::ZERO);
    let leg = &body.legs[0];
    let foot = &body.parts[leg.foot.unwrap()];
    let local = foot.rest_point(&leg.toe);
    let w2: f32 = leg.toe.iter().map(|w| w * w).sum();
    let rotation = foot.transform().to_scale_rotation_translation().1;
    // A local yaw that swings the toe 2 cm along the animated side axis.
    let theta = (0.02 / local.z).asin();
    let turn = rotation * Quat::from_rotation_y(theta);
    let toe = foot.point(&leg.toe);
    let origin = foot.point(&foot.own_origin);
    let flat = (origin + turn * local - toe).with_y(0.0);
    let side = turn * Vec3::X;
    let along = side * flat.dot(side);
    assert!(
        close(along.length() * CM, 2.0, 1e-3),
        "the gap is {:.4} cm",
        along.length() * CM
    );
    let moved = |foot: &super::super::Part| {
        let now = foot.point(&leg.toe);
        (now - toe) / w2 * CM
    };
    let mut turned = foot.clone();
    let mut standing_leg = leg.clone();
    standing_leg.state = FootState(FootState::TOUCHING);
    turn_sole(&mut turned, &standing_leg, &full(), turn);
    let push = moved(&turned);
    let expected = along.normalize() * 1.0;
    assert!(
        close(push.x, expected.x, 1e-3)
            && close(push.y, expected.y, 1e-3)
            && close(push.z, expected.z, 1e-3),
        "pushed {push}, expected {expected}"
    );
    // Already closing the whole gap: the servo leaves it.
    let mut closing = foot.clone();
    closing.push(&[1.0; 4], along);
    let held = closing.point(&leg.toe);
    turn_sole(&mut closing, &standing_leg, &full(), turn);
    assert_eq!(closing.point(&leg.toe), held);
    // Past a right angle the foot keeps its pose.
    let mut flipped = foot.clone();
    turn_sole(
        &mut flipped,
        &standing_leg,
        &full(),
        rotation * Quat::from_rotation_y(std::f32::consts::PI),
    );
    assert_eq!(flipped.point(&leg.toe), toe);
    let off = FootDrive {
        off_feet: true,
        ..full()
    };
    let mut untouched = foot.clone();
    turn_sole(&mut untouched, &standing_leg, &off, turn);
    assert_eq!(untouched.point(&leg.toe), toe);
    let unsupported = FootDrive {
        has_support: false,
        ..full()
    };
    turn_sole(&mut untouched, &standing_leg, &unsupported, turn);
    assert_eq!(untouched.point(&leg.toe), toe);
    // No contact state yet: the settling foot is left where it is.
    let mut unclassified = foot.clone();
    turn_sole(&mut unclassified, leg, &full(), turn);
    assert_eq!(unclassified.point(&leg.toe), toe);
}

/// A footwork landing writes the body's later stand-still stamp (the
/// landing time plus 100 ms, as the trailing plant does) and leaves the
/// earlier one, which the moving foot's start writes; the landed foot's
/// change stamp is the landing time.
#[test]
fn a_footwork_landing_stamps_the_stand_still_clocks() {
    let (mut body, _) = footed(Vec3::ZERO);
    let ankles = [Vec3::new(0.1, 0.1, 0.0), Vec3::new(-0.1, 0.1, 0.0)];
    let spots = ankles.map(|a| Some(a + Vec3::new(0.0, 0.0, 0.3)));
    for (leg, ankle) in body.legs.iter_mut().zip(ankles) {
        leg.state = FootState(FootState::ANCHORED | FootState::TOUCHING);
        leg.anchor = ankle;
        leg.approach = 1.0;
    }
    // Foot 1's anchor is 10 cm off, so the score moves foot 0 and foot 1
    // (anchored) is the one that lands.
    body.legs[1].anchor = ankles[1] + Vec3::new(0.0, 0.0, -0.1);
    let mut work = super::super::steps::Work::plain(1.0);
    work.now_ms = 500.0;
    work.still_at = 120.0;
    plant(
        &mut body.legs,
        &ankles,
        &spots,
        &[Vec3::ZERO; 2],
        &Bearing::default(),
        true,
        &[true; 2],
        &mut work,
    );
    assert_eq!(work.still_plus, 600.0);
    // Foot 0 was already anchored, so no foot started: the earlier stamp
    // stays.
    assert_eq!(work.still_at, 120.0);
    // Ties keep the earlier foot, so foot 1 is the one that landed.
    assert_eq!(body.legs[1].changed_ms, 500.0);
}

/// Before the feet are measured: a body that is not walking plants a foot
/// that does not touch where it is, once it meets the level. Walking, each
/// foot plants or lifts by its phase. Standing, lying down no foot touches; getting up its feet
/// stand on their stance spots (not touching in the first fifth of the
/// clip) until nine tenths, and so do they without support; a body
/// standing with support keeps its feet as they are, but lets go of one
/// whose stance spot is 5 cm or more from its anchor (4.9 cm: kept).
#[test]
fn feet_are_planted_lifted_or_stood_on_their_stance_spots() {
    let (mut body, _) = footed(Vec3::ZERO);
    let ankles = [Vec3::new(0.1, 0.1, 0.0), Vec3::new(-0.1, 0.1, 0.0)];
    // Stance spots near the ankles, and 30 cm ahead of them.
    let near = ankles.map(|a| Some(a + Vec3::new(0.0, 0.0, 0.049)));
    let far = ankles.map(|a| Some(a + Vec3::new(0.0, 0.0, 0.3)));
    let touching = FootState(FootState::TOUCHING);
    let legs = &mut body.legs;
    let plant_in = |legs: &mut [Leg],
                    spots: &[Option<Vec3>; 2],
                    bearing: Bearing,
                    had_support: bool,
                    down: [bool; 2]| {
        let mut work = super::super::steps::Work::plain(1.0);
        plant(
            legs,
            &ankles,
            spots,
            &[Vec3::ZERO; 2],
            &bearing,
            had_support,
            &down,
            &mut work,
        );
        legs.iter().map(|l| (l.state, l.anchor)).collect::<Vec<_>>()
    };
    let standing = Bearing::default();
    let reset = |legs: &mut [Leg]| {
        for leg in legs.iter_mut() {
            leg.state = FootState(FootState::ANCHORED);
            leg.anchor = Vec3::splat(9.0);
            leg.approach = 1.0;
        }
    };
    // Anchored is not touching. On the level it is planted where it is.
    // Off the level it keeps its flags, and the footwork re-targets that
    // anchored foot onto its stance spot.
    reset(legs);
    let planted = plant_in(legs, &near, standing, true, [true, false]);
    assert_eq!(planted[0], (touching, ankles[0]));
    assert_eq!(
        planted[1],
        (FootState(FootState::ANCHORED), near[1].unwrap())
    );
    let planted = plant_in(legs, &near, standing, true, [true; 2]);
    assert_eq!(planted, [(touching, ankles[0]), (touching, ankles[1])]);
    // Touching, they keep their anchors; a foot whose spot is 5 cm off its
    // anchor is let go.
    legs[0].anchor = ankles[0] + Vec3::new(0.0, 0.0, -0.002);
    let kept = plant_in(legs, &near, standing, true, [true; 2]);
    let anchored = FootState(FootState::TOUCHING | FootState::ANCHORED);
    assert_eq!(kept[0], (anchored, near[0].unwrap()));
    assert_eq!(kept[1], (touching, ankles[1]));
    // One foot at a time: the stepping one keeps stepping, re-targeted.
    legs[1].anchor = ankles[1] + Vec3::new(0.0, 0.0, -0.1);
    let kept = plant_in(legs, &far, standing, true, [true; 2]);
    assert_eq!(kept[0], (anchored, far[0].unwrap()));
    assert_eq!(kept[1].0, touching);
    // Reached, it lands; then the other steps.
    legs[0].state.0 |= FootState::REACHED;
    let kept = plant_in(legs, &far, standing, true, [true; 2]);
    assert_eq!(kept[0].0, touching);
    assert_eq!(kept[1], (anchored, far[1].unwrap()));
    // Stalled, it lands where it is, and steps again only 10 cm off.
    legs[1].approach = 0.0;
    let kept = plant_in(legs, &near, standing, true, [true; 2]);
    let stalled = FootState(FootState::TOUCHING | FootState::STALLED);
    assert_eq!(kept[1], (stalled, ankles[1]));
    legs[0].anchor = near[0].unwrap();
    let walking = Bearing {
        walking: true,
        moving: true,
        progress: 0.0,
        ..standing
    };
    reset(legs);
    let planted = plant_in(legs, &far, walking, true, [true; 2]);
    // Phase 0.2 plants the free near foot. Phase 0.7 leaves the free far foot.
    assert_eq!(planted[0].0, touching);
    assert!(close(planted[0].1.x, ankles[0].x, 1e-6));
    assert!(close(planted[0].1.z, ankles[0].z, 1e-6));
    assert_eq!(planted[1].0, FootState(FootState::ANCHORED));
    // Both touching: the far foot's phase lifts it, the near foot stays.
    for leg in legs.iter_mut() {
        leg.state = touching;
    }
    let mixed = plant_in(legs, &far, walking, true, [true; 2]);
    assert_eq!(mixed[0].0, touching);
    assert_eq!(mixed[1].0, FootState(0));
    let lying = Bearing {
        off_feet: true,
        ..standing
    };
    let lying = plant_in(legs, &far, lying, true, [true; 2]);
    assert!(lying.iter().all(|f| f.0 == FootState(0)));
    let rising = |progress: f32| Bearing {
        off_feet: true,
        getting_up: true,
        progress,
        ..standing
    };
    let on_spots = plant_in(legs, &far, rising(0.1), true, [true; 2]);
    assert_eq!(on_spots[1], (FootState(0), far[1].unwrap()));
    let on_spots = plant_in(legs, &far, rising(0.5), true, [true; 2]);
    assert_eq!(on_spots[0], (touching, far[0].unwrap()));
    // Past nine tenths the get-up's feet are a standing body's.
    let kept = plant_in(legs, &far, rising(0.95), true, [true; 2]);
    assert_eq!(kept[0], (touching, far[0].unwrap()));
    reset(legs);
    let on_spots = plant_in(legs, &far, standing, false, [true; 2]);
    assert_eq!(on_spots[0], (touching, far[0].unwrap()));
    // Moving (not walking), the footwork leaves them be.
    legs[0].anchor = Vec3::splat(5.0);
    let moving = Bearing {
        moving: true,
        ..standing
    };
    let kept = plant_in(legs, &far, moving, false, [true; 2]);
    assert_eq!(kept[0], (touching, Vec3::splat(5.0)));
}

/// The smoothed ground rises at once and falls by its share (at once
/// outside the walk); an anchor sits on it plus the sole height, at most
/// 1 m over the ankle; an anchored foot's probe moves 0.05 a tick toward
/// its anchor up to 0.5, its sphere 5 cm plus 10 times that.
#[test]
fn anchors_sit_on_the_smoothed_ground() {
    assert_eq!(smooth_ground(0.0, 5.0, 0.05), 5.0);
    assert_eq!(smooth_ground(10.0, 5.0, 1.0), 5.0);
    assert!(close(smooth_ground(10.0, 5.0, 0.05), 9.75, 1e-5));
    assert!(close(anchor_height(0.0, 9.68, 10.0), 9.68, 1e-6));
    assert!(close(anchor_height(0.0, 9.68, -95.0), 5.0, 1e-6));
    let (mut body, _) = footed(Vec3::ZERO);
    let leg = &mut body.legs[0];
    assert_eq!(probe_blend(leg), 0.0);
    leg.state = FootState(FootState::TOUCHING | FootState::ANCHORED);
    leg.blend = 0.48;
    assert_eq!(probe_blend(leg), 0.5);
    leg.blend = 0.1;
    assert!(close(probe_blend(leg), 0.15, 1e-6));
    assert_eq!(probe_radius(0.0), 5.0);
    assert_eq!(probe_radius(0.5), 10.0);
}

/// A walking foot's phase is the fraction of 2 − 0.5·leg − progress. At
/// the bump's peak the sphere is 20 cm and the sway is
/// (36·scale + 8·speed) cm (the other way backwards). The fall share is
/// one double product narrowed once: at phase 0.5 it is 0.30688005685806274.
#[test]
fn a_walking_probe_widens_and_sways_with_the_gait() {
    let peak = 0.125f32.sqrt();
    let wide = gait_probe(0, 2.0 - peak, 1.0, 1.5, false);
    assert_eq!(wide.radius, 20.0);
    assert_eq!(wide.sway, 48.0);
    assert_eq!(wide.fall.to_bits(), 0x3d53_9678);
    let back = gait_probe(0, 2.0 - peak, 1.0, 1.5, true);
    assert_eq!(back.sway, -48.0);
    // Phase 0.5 (the second leg at progress 0): the sphere stays 10 cm
    // and the sway is 0, but the ground no longer falls at once.
    let half = gait_probe(1, 0.0, 1.0, 0.0, false);
    assert_eq!((half.radius, half.sway), (10.0, 0.0));
    assert_eq!(half.fall.to_bits(), 0x3e9d_1f62);
    // A phase of 0, and a NaN progress, keep the plain 10 cm sphere.
    let still = gait_probe(0, 0.0, 1.0, 4.0, false);
    assert_eq!((still.radius, still.sway, still.fall), (10.0, 0.0, 1.0));
    let nan = gait_probe(0, f32::NAN, 1.0, 1.0, false);
    assert_eq!((nan.radius, nan.sway, nan.fall), (10.0, 0.0, 1.0));
}

/// The stance's level keeps 0.9 of itself and takes 0.1 of 1 − v², v the
/// share of the feet's height difference in their squared distance: 1 on
/// level ground, 0.975 a tick after feet 30 cm apart stand 30 cm apart in
/// height (v = 0.5). The legs' extension is the level times (pelvis over
/// the highest anchor over the pose's pelvis over its mean ankle)², at
/// most 1: a quarter with the pelvis half as high.
#[test]
fn the_stance_levels_and_the_legs_extend() {
    assert_eq!(level(1.0, 0.0, 30.0, 0.0), 1.0);
    assert!(close(level(1.0, 30.0, 30.0, 0.0), 0.975, 1e-6));
    assert!(close(level(0.5, 0.0, 0.0, 30.0), 0.55, 1e-6));
    // Two feet on one spot give 0/0: the original propagates the NaN.
    assert!(level(1.0, 0.0, 0.0, 0.0).is_nan());
    assert_eq!(extension(1.0, 85.0, 85.0), 1.0);
    assert_eq!(extension(1.0, 90.0, 85.0), 1.0);
    assert!(close(extension(1.0, 42.5, 85.0), 0.25, 1e-6));
    assert!(close(extension(0.9, 42.5, 85.0), 0.225, 1e-6));
    // A standing body on level ground: its level 1, its legs extended.
    let floor = floor_quad(0.0, 5.0);
    let (mut body, pose) = footed(Vec3::ZERO);
    for _ in 0..100 {
        stand(&mut body, Affine3A::IDENTITY, &pose, &floor);
    }
    let b = drive(&body).balance();
    assert_eq!(b.level(), 1.0);
    assert!(b.extension() > 0.95, "{}", b.extension());
}

/// A body standing on a floor plants its feet where they stand: both
/// touch, anchored at the floor plus the sole height, gripped in full after
/// 50 ticks, their ankles within 1 cm of their anchors and their toes on
/// the floor. Walking at progress 0, the near foot stays and the far foot
/// lifts; stopping plants the lifted foot where it is and leaves the
/// touching foot's anchor.
#[test]
fn a_standing_body_plants_its_feet() {
    let floor = floor_quad(0.0, 5.0);
    let (mut body, pose) = footed(Vec3::ZERO);
    for _ in 0..120 {
        stand(&mut body, Affine3A::IDENTITY, &pose, &floor);
    }
    for (k, leg) in body.legs.iter().enumerate() {
        assert_eq!(leg.state, FootState(FootState::TOUCHING));
        assert!(close(leg.anchor.y, SOLE, 1e-5), "{}", leg.anchor.y);
        assert_eq!(leg.grip, 1.0);
        let (at, _) = ankle(&body, k);
        assert!(
            (at - leg.anchor).with_y(0.0).length() < 0.01,
            "leg {k}: {at} from {}",
            leg.anchor
        );
        assert!(close(leg.floor, 0.0, 1e-6));
    }
    body.drive.as_mut().unwrap().bearing.walking = true;
    body.drive.as_mut().unwrap().bearing.progress = 0.0;
    stand(&mut body, Affine3A::IDENTITY, &pose, &floor);
    assert_eq!(body.legs[0].state, FootState(FootState::TOUCHING));
    assert_eq!(body.legs[1].state, FootState(0));
    let kept = body.legs[0].anchor;
    body.drive.as_mut().unwrap().bearing.walking = false;
    let far = ankle(&body, 1).0;
    body.measure_balance(floor.as_slice());
    assert_eq!(body.legs[0].state, FootState(FootState::TOUCHING));
    assert_eq!(body.legs[0].anchor, kept);
    assert_eq!(body.legs[1].state, FootState(FootState::TOUCHING));
    assert!(close(body.legs[1].anchor.x, far.x, 1e-6));
    assert!(close(body.legs[1].anchor.z, far.z, 1e-6));
    assert!(close(body.legs[1].anchor.y, SOLE, 1e-5));
}

/// A quarter turn in place keeps the planted feet within 1 cm of where
/// they were planted: the pose turns 90° about the vertical over 60 ticks
/// and holds; the pelvis turns with it, and its feet step to their new
/// stance spots one at a time (six steps, each reaching its spot), while a
/// foot planted with its grip full stays within 1 cm of its anchor (0.6 cm)
/// and of where its grip became full. (A foot that just landed settles
/// onto its anchor, moved halfway to it, while its grip rises.)
#[test]
fn a_quarter_turn_in_place_keeps_planted_feet_within_a_cm() {
    let floor = floor_quad(0.0, 5.0);
    let (mut body, pose) = footed(Vec3::ZERO);
    for _ in 0..120 {
        stand(&mut body, Affine3A::IDENTITY, &pose, &floor);
    }
    assert_eq!(drive(&body).balance().landings(), 0);
    let mut farthest = 0.0f32;
    let mut stepped = [false; 2];
    // Where each foot's ankle was when it last stood on a new anchor.
    let mut stood_at: [Option<(Vec3, Vec3)>; 2] = [None; 2];
    for tick in 0..300 {
        let angle = std::f32::consts::FRAC_PI_2 * (tick as f32 / 60.0).min(1.0);
        let turn = Affine3A::from_rotation_y(angle);
        let turned: Vec<Affine3A> = pose.iter().map(|t| turn * *t).collect();
        let before: Vec<(FootState, Vec3)> =
            body.legs.iter().map(|l| (l.state, l.anchor)).collect();
        stand(&mut body, Affine3A::IDENTITY, &turned, &floor);
        let feet = body.legs.iter().zip(&before).enumerate();
        for (k, (leg, &(state, anchor))) in feet {
            stepped[k] |= leg.state.has(FootState::ANCHORED);
            // A foot standing on one anchor with its grip full: how far its
            // ankle is from the anchor, and has moved since the grip filled.
            let stood = |s: FootState| s.stands() && !s.has(FootState::ANCHORED);
            let (at, _) = ankle(&body, k);
            let held = leg.grip == 1.0 && leg.anchor.xz() == anchor.xz();
            if stood(state) && stood(leg.state) && held {
                let start = match stood_at[k] {
                    Some((on, start)) if on.xz() == anchor.xz() => start,
                    _ => at,
                };
                stood_at[k] = Some((anchor, start));
                farthest = farthest
                    .max((at - start).with_y(0.0).length())
                    .max((at - anchor).with_y(0.0).length());
            }
        }
        let anchored = body
            .legs
            .iter()
            .filter(|l| l.state.has(FootState::ANCHORED));
        assert!(anchored.count() < 2, "both feet step at tick {tick}");
    }
    assert!(
        farthest < 0.01,
        "a planted foot slid {:.2} cm",
        farthest * 100.0
    );
    assert_eq!(stepped, [true; 2]);
    assert!(drive(&body).balance().landings() >= 2);
    for (k, leg) in body.legs.iter().enumerate() {
        assert!(
            leg.state.stands() && !leg.state.has(FootState::ANCHORED),
            "{k}: {:?}",
            leg.state
        );
    }
    let facing = body.part_of(1).unwrap().transform().matrix3.z_axis;
    let turned = Vec3::from(facing).with_y(0.0).normalize();
    assert!(turned.x > 0.7, "the pelvis faces {turned}");
}

/// Standing feet grip through a light shove: a 2 m/s sideways blow to the
/// torso sways the chest about 2 cm (the support pins hold it), and the
/// feet stay within 1 cm of their anchors (0.1 cm).
#[test]
fn standing_feet_grip_through_a_light_shove() {
    let floor = floor_quad(0.0, 5.0);
    let (mut body, pose) = footed(Vec3::ZERO);
    for _ in 0..120 {
        stand(&mut body, Affine3A::IDENTITY, &pose, &floor);
    }
    let planted: Vec<Vec3> = body.legs.iter().map(|l| l.anchor).collect();
    let chest = body.chest_point().unwrap();
    body.shove(8, Vec3::X * 2.0, 0.0);
    let (mut farthest, mut swayed) = (0.0f32, 0.0f32);
    for _ in 0..200 {
        stand(&mut body, Affine3A::IDENTITY, &pose, &floor);
        for (k, planted) in planted.iter().enumerate() {
            let (at, _) = ankle(&body, k);
            farthest = farthest.max((at - *planted).with_y(0.0).length());
        }
        swayed = swayed.max((body.chest_point().unwrap() - chest).with_y(0.0).length());
    }
    assert!((0.015..0.03).contains(&swayed), "the chest swayed {swayed}");
    assert!(farthest < 0.01, "a foot slid {:.2} cm", farthest * 100.0);
}

/// On a step: a body standing with its right foot on a 15 cm block and its
/// left on the floor anchors each foot on its own ground plus the sole
/// height (24.5 and 9.5 cm), and its stance's level follows 1 − v², v =
/// 15²/(15² + d²) for its feet d cm apart across the ground (0.60 with its
/// feet 11.4 cm apart).
#[test]
fn feet_on_a_step_anchor_on_their_own_ground() {
    let mut level = floor_quad(0.0, 5.0);
    // The block's top over the floor, under the right foot only.
    let at = |x: f32, z: f32| Vec3::new(0.1 + x * 0.08, 0.15 + 1e-4, z * 0.2);
    level.push([at(-1.0, -1.0), at(-1.0, 1.0), at(1.0, 1.0)]);
    level.push([at(-1.0, -1.0), at(1.0, 1.0), at(1.0, -1.0)]);
    let (mut body, pose) = footed(Vec3::ZERO);
    // The right foot set on the block: its lower leg and foot raised 15 cm.
    for leg in body.legs.iter().take(1) {
        for part in [leg.shin, leg.foot.unwrap()] {
            let p = &mut body.parts[part];
            for x in p.positions.iter_mut().chain(p.previous.iter_mut()) {
                x.y += 0.15;
            }
        }
    }
    let mut lifted = pose.clone();
    for (i, p) in body.parts.iter().enumerate() {
        if matches!(p.bone, 3 | 4) {
            lifted[i].translation.y += 0.15;
        }
    }
    for _ in 0..200 {
        stand(&mut body, Affine3A::IDENTITY, &lifted, &level);
    }
    let legs = &body.legs;
    assert!(
        legs.iter().all(|l| l.state.stands()),
        "{:?}",
        legs.iter().map(|l| l.state).collect::<Vec<_>>()
    );
    assert!(
        close(legs[0].anchor.y, 0.15 + SOLE, 2e-3),
        "{}",
        legs[0].anchor.y
    );
    assert!(close(legs[1].anchor.y, SOLE, 2e-3), "{}", legs[1].anchor.y);
    let (a, b) = (ankle(&body, 0).0, ankle(&body, 1).0);
    let d2 = cm(a - b).with_y(0.0).length_squared();
    let v = 225.0 / (225.0 + d2);
    let level = drive(&body).balance().level();
    assert!(close(level, 1.0 - v * v, 0.01), "level {level}, v {v}");
    assert!(level < 0.95);
}

fn two_feet() -> (Ragdoll, [Vec3; 2]) {
    let shin = |bone: usize, x: f32| PartSetup {
        bone,
        entity: Entity::from_raw_u32(bone as u32 + 1).unwrap(),
        parent: Some(0),
        world: Affine3A::from_translation(Vec3::new(x, 0.0, 0.0)),
        centre: Vec3::ZERO,
        half: Vec3::new(0.06, 0.2, 0.06),
        origin: Vec3::ZERO,
        links: vec![],
        foot: Some(Vec3::new(0.0, -0.2, 0.0)),
        pins: Vec::new(),
        spheres: None,
    };
    let pelvis = PartSetup {
        bone: 1,
        entity: Entity::from_raw_u32(1).unwrap(),
        parent: None,
        world: Affine3A::from_translation(Vec3::ZERO),
        centre: Vec3::ZERO,
        half: Vec3::splat(0.1),
        origin: Vec3::ZERO,
        links: vec![],
        foot: None,
        pins: Vec::new(),
        spheres: None,
    };
    let body = Ragdoll::new(&[pelvis, shin(3, 0.3), shin(5, -0.3)]).with_drive();
    let ankles = [
        body.parts[body.legs[0].shin].point(&body.legs[0].contact),
        body.parts[body.legs[1].shin].point(&body.legs[1].contact),
    ];
    (body, ankles)
}

#[test]
fn a_stagger_target_walks_one_foot() {
    let (mut body, ankles) = two_feet();
    let spots = [Some(ankles[0]), Some(ankles[1])];
    let walked = body.walk_stagger(Vec3::new(0.0, 1.0, 2.0), Vec3::ZERO, &spots, 1.0);
    assert_eq!(walked.origin.z.to_bits(), 0x3f61_47ae);
    assert_eq!(walked.origin.x.to_bits(), 0);
    assert_eq!(walked.foot, Some(0));
    assert!(!walked.ended);
    assert_eq!(walked.states, vec![FootState::ANCHORED, 0]);
    assert_eq!(walked.anchors[0].x.to_bits(), 0x3e99_9999);
    assert_eq!(walked.anchors[0].y.to_bits(), 0xbe4c_cccc);
    assert_eq!(walked.anchors[0].z.to_bits(), 0x3f61_47ae);
    assert_eq!(walked.anchors[1].x.to_bits(), 0xbe99_9999);
    assert_eq!(walked.anchors[1].z.to_bits(), 0);
    let (mut near, ankles) = two_feet();
    let spots = [Some(ankles[0]), Some(ankles[1])];
    let walked = near.walk_stagger(Vec3::new(0.0, 0.0, 0.5), Vec3::ZERO, &spots, 1.0);
    assert_eq!(walked.origin.z.to_bits(), 0x3f00_0000);
    // The two feet are mirror images here and score alike to the last bit
    // or so; whichever steps lands on the target's line.
    let foot = walked.foot.expect("one foot steps");
    assert_eq!(walked.anchors[foot].z.to_bits(), 0x3f00_0000);
    let (mut still, ankles) = two_feet();
    let spots = [Some(ankles[0]), Some(ankles[1])];
    let walked = still.walk_stagger(Vec3::ZERO, Vec3::ZERO, &spots, 1.0);
    assert_eq!(walked.foot, None);
    assert_eq!(walked.states, vec![0, 0]);
}

/// Leaving the walk, the rearmost foot (by where its contact is, not by
/// its stance spot) is anchored at its stance spot under the frame
/// (keeping its height) and the other lands where it is, anchored or not.
#[test]
fn the_trailing_foot_is_planted_on_leaving_the_walk() {
    let (mut body, pose) = footed(Vec3::ZERO);
    body.aim(Affine3A::IDENTITY, pose.clone());
    for leg in &mut body.legs {
        leg.state = FootState(FootState::ANCHORED | FootState::RAMP_SLOW);
        leg.anchor = Vec3::new(9.0, 9.0, 9.0);
    }
    // The second foot's contact trails the first's.
    let shin = body.legs[1].shin;
    for x in &mut body.parts[shin].positions {
        *x -= Vec3::Z * 0.2;
    }
    body.plant_trailing_foot();
    let tick = 1000.0 / 83.0;
    assert!(close(drive(&body).steps.still_at, tick, 1e-4));
    assert!(close(drive(&body).steps.still_plus, tick + 100.0, 1e-4));
    assert!(body.legs[1].state.has(FootState::ANCHORED));
    assert!(body.legs[1].state.has(FootState::RAMP_SLOW));
    assert!(body.legs[0].state.has(FootState::TOUCHING));
    assert!(!body.legs[0].state.has(FootState::ANCHORED));
    let posed = drive(&body).targets()[body.legs[1].shin].transform_point3(body.legs[1].point);
    assert!(close(body.legs[1].anchor.x, posed.x, 1e-5));
    assert!(close(body.legs[1].anchor.z, posed.z, 1e-5));
    assert_ne!(body.legs[1].anchor, Vec3::new(9.0, 9.0, 9.0));
    // The other foot's anchor is where it is.
    let at = body.parts[body.legs[0].shin].point(&body.legs[0].contact);
    assert_eq!(body.legs[0].anchor, at);
    // A foot that was not anchored still lands, keeping its anchor.
    for leg in &mut body.legs {
        leg.state = FootState(FootState::RAMP_SLOW);
        leg.anchor = Vec3::new(9.0, 9.0, 9.0);
    }
    body.plant_trailing_foot();
    assert_eq!(
        body.legs[0].state.0,
        FootState::RAMP_SLOW | FootState::TOUCHING
    );
    assert_eq!(body.legs[0].anchor, Vec3::new(9.0, 9.0, 9.0));
}

/// Clearing a state mask leaves the other bits; the full tone reset is 1
/// and the get-up motions set the getting-up condition.
#[test]
fn state_masks_and_the_full_tone_are_the_originals() {
    let (mut body, _pose) = footed(Vec3::ZERO);
    for leg in &mut body.legs {
        leg.state = FootState(0xffff);
    }
    body.clear_foot_state(0xffff);
    assert!(body.legs.iter().all(|l| l.state.0 == 0));
    assert!(full_tone(0x0E, 0.75).getting_up);
    assert!(full_tone(0x0F, 0.75).getting_up);
    assert!(!full_tone(0x0D, 0.75).getting_up);
    assert_eq!(full_tone(0, 0.75).stance, 0.75);
    assert_eq!(full_tone(0, 0.75).tone, 1.0);
    assert_eq!(full_tone(0, 0.75).stagger_decay, 0.0);
}

/// The walking ground weights: the stored height difference smooths
/// 0.9/0.1 toward the phase's difference, the phase's bias comes from it,
/// and the first foot's weight is (1 − r)·0.5 + (1 − |2t − 1|)·r with
/// r = 1 − run/2. Standing, the stored difference loses a tenth and the
/// feet share the ground equally.
#[test]
fn the_walking_ground_weights_follow_the_phase_and_run() {
    // Phase 0.45 picks g0 − g1 = −10; the stored −1 biases the phase by
    // 0.4125, so the first foot takes 1 − |0.825 − 1| = 0.825 at a walk.
    let (last, first, second) = gait_weights(0.0, 0.0, true, 10.0, 20.0, 0.0);
    assert!(close(last, -1.0, 1e-6), "{last}");
    assert!(close(first, 0.825, 1e-6), "{first}");
    assert!(close(second, 0.175, 1e-6), "{second}");
    // A run of 1 halves the phase term: 0.5·0.5 + 0.825·0.5.
    let (_, first, _) = gait_weights(0.0, 1.0, true, 10.0, 20.0, 0.0);
    assert!(close(first, 0.6625, 1e-6), "{first}");
    // Standing keeps half the ground each; the stored difference decays.
    let (last, first, second) = gait_weights(0.5, 1.0, false, 0.0, 0.0, 2.0);
    assert!(close(last, 1.8, 1e-6), "{last}");
    assert_eq!((first, second), (0.5, 0.5));
    // A NaN stored difference takes the middling bias: 0.4 − NaN = NaN.
    let (last, first, _) = gait_weights(0.0, 0.0, true, 1.0, 2.0, f32::NAN);
    assert!(last.is_nan());
    assert!(first.is_nan());
}

/// A queued step is taken when its direction is more than cos 0.7 off the
/// line to the target, or the target is within 40 × scale cm; it aims at
/// the chest plus the queued vector, tells the AI when that points along
/// the facing, and counts down to −2 before the first landing and −1
/// after it.
#[test]
fn a_queued_step_aims_from_the_chest() {
    let chest = Vec3::ZERO;
    let target = Vec3::new(10.0, 0.0, 0.0);
    // Straight at the target, far away: not taken.
    let side = Vec3::Z;
    assert!(queued_step(chest, target, Vec3::X, side, 1.0, 0).is_none());
    // Behind the line: taken, the target the chest plus the queue.
    let step = queued_step(chest, target, Vec3::NEG_X, side, 1.0, 0).unwrap();
    assert_eq!(step.target, Vec3::NEG_X);
    assert!(!step.tell_ai);
    assert_eq!(step.countdown, -2);
    let step = queued_step(chest, target, Vec3::NEG_X, Vec3::NEG_X, 1.0, 1).unwrap();
    assert!(step.tell_ai);
    assert_eq!(step.countdown, -1);
    // Within 40 × scale cm even along the line: taken.
    let near = Vec3::new(0.39, 0.0, 0.0);
    assert!(queued_step(chest, near, Vec3::X, side, 1.0, 0).is_some());
    let far = Vec3::new(0.41, 0.0, 0.0);
    assert!(queued_step(chest, far, Vec3::X, side, 1.0, 0).is_none());
    // A target on the chest is unordered and takes the queue.
    assert!(queued_step(chest, chest, Vec3::X, side, 1.0, 0).is_some());
}

/// A lifted walking foot plants early before phase 0.2 when it moves
/// forward at most 0.3 cm a tick, or, backpedalling, back at most 0.2 cm
/// a tick. The bounds are the double 0.3 and −0.2: the single-precision
/// 0.3 is just over and does not plant, nor does the single −0.2.
#[test]
fn a_slow_foot_plants_early() {
    assert_eq!(gait_plant(0.1, false, 0.29, false), GaitPlant::Plant);
    assert_eq!(gait_plant(0.1, false, 0.3, false), GaitPlant::Hold);
    assert_eq!(gait_plant(0.1, false, 0.31, false), GaitPlant::Hold);
    assert_eq!(gait_plant(0.1, false, -0.19, true), GaitPlant::Plant);
    assert_eq!(gait_plant(0.1, false, -0.2, true), GaitPlant::Hold);
    assert_eq!(gait_plant(0.1, false, f32::NAN, false), GaitPlant::Plant);
    assert_eq!(gait_plant(0.2, false, 9.0, false), GaitPlant::Plant);
    // The single 0.4 is past the double 0.4: no plant.
    assert_eq!(gait_plant(0.4, false, 0.0, false), GaitPlant::Hold);
    // A NaN phase lifts a touching foot and plants a free one.
    assert_eq!(gait_plant(f32::NAN, true, 0.0, false), GaitPlant::Lift);
    assert_eq!(gait_plant(f32::NAN, false, 9.0, false), GaitPlant::Plant);
}

/// A step's loudness is 0.01 + 0.0139 × its length up to 140 cm, and 1
/// beyond: 5 cm gives 0.0795, 140 cm 1.956, 200 cm 1.
#[test]
fn a_steps_loudness_follows_its_length() {
    assert!(close(step_loudness(5.0), 0.0795, 1e-6));
    assert!(close(step_loudness(140.0), 1.956, 1e-6));
    assert_eq!(step_loudness(200.0), 1.0);
    assert_eq!(step_loudness(-10.0), -0.129);
}

/// A lower leg's child takes the original's second flag word bit when the
/// leg joins the world, and loses its dynamic state bit.
#[test]
fn a_foot_child_takes_the_second_flag_words_bit() {
    use crate::part::{FOOT_CHILD_FLAG, PartState};
    let pelvis = PartSetup {
        bone: 1,
        entity: Entity::from_raw_u32(1).unwrap(),
        parent: None,
        world: Affine3A::from_translation(Vec3::ZERO),
        centre: Vec3::ZERO,
        half: Vec3::splat(0.1),
        origin: Vec3::ZERO,
        links: vec![],
        foot: None,
        pins: Vec::new(),
        spheres: None,
    };
    let shin = PartSetup {
        bone: 3,
        entity: Entity::from_raw_u32(2).unwrap(),
        parent: Some(0),
        world: Affine3A::from_translation(Vec3::new(0.3, 0.0, 0.0)),
        centre: Vec3::ZERO,
        half: Vec3::new(0.06, 0.2, 0.06),
        origin: Vec3::ZERO,
        links: vec![],
        foot: Some(Vec3::new(0.0, -0.2, 0.0)),
        pins: Vec::new(),
        spheres: None,
    };
    let foot = PartSetup {
        bone: 4,
        entity: Entity::from_raw_u32(3).unwrap(),
        parent: Some(1),
        world: Affine3A::from_translation(Vec3::new(0.3, -0.1, 0.0)),
        centre: Vec3::ZERO,
        half: Vec3::splat(0.07),
        origin: Vec3::ZERO,
        links: vec![],
        foot: None,
        pins: Vec::new(),
        spheres: None,
    };
    let body = Ragdoll::new(&[pelvis, shin, foot]);
    assert_ne!(body.parts[2].flags2 & FOOT_CHILD_FLAG, 0);
    assert!(!body.parts[2].state.has(PartState::DYNAMIC));
    assert!(body.parts[0].state.has(PartState::DYNAMIC) && body.parts[0].flags2 == 0);
}
