//! The carry and the lift: the original's rules, each pinned to its numbers
//! (`docs/formats/RSQ.md`, Physical body).
//! (Included from body_lift.rs: `#[path = "body_lift_test.rs"] mod tests;`)
use super::super::Drive;
use super::super::floor_test::{STANDING_LEG, STANDING_SOLE, standing_at};
use super::super::tests::floor_quad;
use super::*;

const NONE: &[[Vec3; 3]] = &[];

fn close(a: f32, b: f32, tolerance: f32) -> bool {
    (a - b).abs() <= tolerance
}

fn drive(body: &Ragdoll) -> &Drive {
    body.drive.as_ref().unwrap()
}

/// The pelvis's height (m): its origin's.
fn pelvis(body: &Ragdoll) -> f32 {
    body.part_of(1).unwrap().transform().translation.y
}

/// One character tick and one solver tick of a body standing in its pose
/// on `level`.
fn stand(body: &mut Ragdoll, pose: &[Affine3A], level: &[[Vec3; 3]]) {
    body.measure_balance(level);
    body.aim(Affine3A::IDENTITY, pose.to_vec());
    body.tick(level);
}

/// Standing, f = scale·0.075(h + 15), h the target less the pelvis height
/// (cm): 1 when the pelvis is 1.7 cm over its target at scale 1, 1.5 at
/// most (5 cm under it), none once the pelvis is 15 cm over it, but down
/// to −10 crouched deeper than 0.9. Moving, f = scale·(0.4·run + 0.2)(h + 5):
/// 1 at the target walking, 3.3 cm over it running, 2 at most, none 5 cm
/// over it. The scale product is a double narrowed once, before the clamp.
/// Half scale gives 0.5625 standing at h = 0 and 0.5 walking there; scale 2
/// clamps the standing term to 1.5. The standing lead is 15, 10 while
/// stepping and 20 when dormant. Falling a cm a tick at f = 1 adds
/// (1 − 1/3)·E², rising takes it off.
#[test]
fn the_lift_is_the_originals() {
    let standing = Bearing::default();
    let lift = |h: f32, rise: f32, energy: f32, b: &Bearing| lift_factor(h, rise, energy, b, 1.0);
    let near = |f: Option<f32>, want: f32| f.is_some_and(|f| close(f, want, 1e-5));
    assert!(near(lift(-15.0, 0.0, 1.0, &standing), 0.0));
    assert!(near(lift(-5.0 / 3.0, 0.0, 1.0, &standing), 1.0));
    assert!(near(lift(5.0, 0.0, 1.0, &standing), 1.5));
    assert!(near(lift(30.0, 0.0, 1.0, &standing), 1.5));
    assert_eq!(lift(-15.5, 0.0, 1.0, &standing), None);
    let crouched = Bearing {
        crouch: 0.95,
        ..standing
    };
    assert!(near(lift(-15.5, 0.0, 1.0, &crouched), -0.0375));
    assert!(near(lift(-200.0, 0.0, 1.0, &crouched), -10.0));
    let walking = Bearing {
        moving: true,
        ..standing
    };
    let running = Bearing {
        run: 1.0,
        ..walking
    };
    assert!(near(lift(0.0, 0.0, 1.0, &walking), 1.0));
    assert!(near(lift(-10.0 / 3.0, 0.0, 1.0, &running), 1.0));
    assert!(near(lift(10.0, 0.0, 1.0, &walking), 2.0));
    assert_eq!(lift(-5.5, 0.0, 1.0, &walking), None);
    let at_one = -5.0 / 3.0;
    assert!(near(lift(at_one, -1.0, 1.0, &standing), 1.0 + 2.0 / 3.0));
    assert!(near(lift(at_one, -1.0, 0.5, &standing), 1.0 + 2.0 / 12.0));
    assert!(near(lift(at_one, 1.0, 1.0, &standing), 1.0 - 2.0 / 3.0));
    // At the clamp the damping is (1 − 1.5/3.5) of the speed.
    assert!(near(lift(30.0, 2.0, 1.0, &standing), 1.5 - 2.0 * 4.0 / 7.0));
    assert_eq!(lift_factor(0.0, 0.0, 1.0, &standing, 0.5), Some(0.5625));
    assert_eq!(lift_factor(0.0, 0.0, 1.0, &walking, 0.5), Some(0.5));
    assert_eq!(lift_unclamped(0.0, &standing, 2.0), 2.25);
    assert_eq!(lift_factor(0.0, 0.0, 1.0, &standing, 2.0), Some(1.5));
    // The standing lead is 10 cm while stepping and 20 when dormant; the
    // stepping test comes first.
    let stepping = Bearing {
        stepping: true,
        ..standing
    };
    let dormant = Bearing {
        dormant: true,
        ..standing
    };
    assert_eq!(lift_unclamped(0.0, &stepping, 1.0), 0.75);
    assert_eq!(lift_unclamped(0.0, &dormant, 1.0), 1.5);
    let both = Bearing {
        stepping: true,
        dormant: true,
        ..standing
    };
    assert_eq!(lift_unclamped(0.0, &both, 1.0), 0.75);
}

/// A part takes the carry by its tone times its support share: the pelvis
/// and thighs 1, the lower legs 0.5, the feet 0.25; the upper body by its
/// tone times its energy; nothing at a tone of a half or less.
#[test]
fn carry_shares_are_the_originals() {
    for (bone, want) in [
        (1, 1.0),
        (2, 1.0),
        (5, 1.0),
        (3, 0.5),
        (6, 0.5),
        (4, 0.25),
        (7, 0.25),
    ] {
        assert_eq!(carry_share(bone, 1.0, 0.6, 1.0), Some(want), "bone {bone}");
        assert!(close(
            carry_share(bone, 0.9, 0.6, 1.0).unwrap(),
            0.9 * want,
            1e-6
        ));
        // Scale 1.3 stays; the next step up zeroes the legs and pelvis.
        assert_eq!(carry_share(bone, 1.0, 0.6, 1.3), Some(want), "bone {bone}");
        assert_eq!(carry_share(bone, 1.0, 0.6, 1.31), Some(0.0), "bone {bone}");
    }
    for bone in [8, 9, 10, 11, 13, 17] {
        assert!(close(carry_share(bone, 0.9, 0.6, 1.0).unwrap(), 0.54, 1e-6));
        assert!(close(
            carry_share(bone, 0.9, 0.6, 1.31).unwrap(),
            0.54,
            1e-6
        ));
    }
    assert_eq!(carry_share(1, 0.5, 1.0, 1.31), None);
    assert_eq!(carry_share(9, 0.3, 1.0, 1.0), None);
}

/// The leg of a standing human as the arena's Idle gives it (cm): the clip
/// root 91.4 over its ankle, the foot's sole 9.68, the leg 85.15.
fn idle_leg(ground: f32) -> LegHeight {
    LegHeight {
        ground: Some(ground),
        contact: ground + 10.0,
        stance: 91.4 + SOLE,
        length: 85.15,
        lifted: false,
    }
}

const SOLE: f32 = 9.68;

/// The target pelvis height: a standing human's (the clip root 91.4 cm
/// over its ankle, its sole 9.68 cm) is capped at its ground + sole + leg
/// length (85.15 cm) and raised 5 cm: 99.83 cm. It follows a 30 cm step
/// under both feet by 30 cm; with one foot on the step it stays capped by
/// the lower. Moving, it is neither capped nor raised; without ground there
/// is none.
#[test]
fn the_target_height_is_the_originals() {
    let flat = [idle_leg(0.0), idle_leg(0.0)];
    assert!(close(
        target_height(&flat, SOLE, 0.0, 101.5, false, 1.0, 0.0, 0.0),
        99.83,
        1e-3
    ));
    let step = [idle_leg(30.0), idle_leg(30.0)];
    assert!(close(
        target_height(&step, SOLE, 0.0, 131.5, false, 1.0, 0.0, 0.0),
        129.83,
        1e-3
    ));
    let one = [idle_leg(0.0), idle_leg(30.0)];
    assert!(close(
        target_height(&one, SOLE, 0.0, 101.5, false, 1.0, 0.0, 0.0),
        99.83,
        1e-3
    ));
    // Moving: the mean of the two feet's, uncapped.
    assert!(close(
        target_height(&one, SOLE, 0.0, 101.5, true, 1.0, 4.0, 0.0),
        116.08,
        1e-3
    ));
    assert!(close(
        target_height(&flat, SOLE, 0.0, 101.5, true, 1.0, 4.0, 0.0),
        101.08,
        1e-3
    ));
    let none = [LegHeight {
        ground: None,
        ..idle_leg(0.0)
    }; 2];
    assert_eq!(
        target_height(&none, SOLE, 0.0, 101.5, false, 1.0, 0.0, 0.0),
        NO_TARGET
    );
}

/// A full crouch is 60 cm times the body scale over the ground. Half scale
/// stands at 35 cm (30 plus the 5 cm raise); scale 1.25 stands at 80 cm.
/// Moving, neither the cap nor the raise applies, so half scale is 30 cm.
/// A NaN scale, moving, leaves the target a NaN.
#[test]
fn a_crouch_stands_at_60_cm_times_the_body_scale() {
    let flat = [idle_leg(0.0), idle_leg(0.0)];
    assert_eq!(
        target_height(&flat, SOLE, 1.0, 100.0, false, 0.5, 0.0, 0.0),
        35.0
    );
    assert_eq!(
        target_height(&flat, SOLE, 1.0, 100.0, false, 1.25, 0.0, 0.0),
        80.0
    );
    assert_eq!(
        target_height(&flat, SOLE, 1.0, 100.0, true, 0.5, 4.0, 0.0),
        30.0
    );
    assert!(target_height(&flat, SOLE, 0.0, 100.0, true, f32::NAN, 4.0, 0.0).is_nan());
}

/// The other foot is blended in by half while its contact's height over
/// its ground plus the lower foot's contact height is under 40 cm, by
/// 0.05 a cm less up to 60, not at all from 60 (the original's test, as
/// read: on a floor 50 cm up nothing is blended), and not when it has no
/// ground.
#[test]
fn the_other_foot_is_blended_in() {
    let crouched = |ground: f32, contact: f32| LegHeight {
        ground: Some(ground),
        contact,
        stance: 40.0,
        length: 85.15,
        lifted: false,
    };
    // Uncapped, 40 cm over each foot's ground, the other foot's ground
    // 10 cm higher, its contact 10 cm over it: the spread is the lower
    // contact's height plus 10. Weights 0.5, 0.5, 0.125 and 0, plus 5 cm.
    for (contact, want) in [(25.0, 50.0), (30.0, 50.0), (45.0, 46.25), (50.0, 45.0)] {
        let legs = [crouched(0.0, contact), crouched(10.0, 20.0)];
        let got = target_height(&legs, SOLE, 0.0, 100.0, false, 1.0, 0.0, 0.0);
        assert!(
            close(got, want, 1e-3),
            "contact {contact}: {got}, not {want}"
        );
    }
    let high = [crouched(50.0, 60.0), crouched(50.0, 60.0)];
    assert!(close(
        target_height(&high, SOLE, 0.0, 150.0, false, 1.0, 0.0, 0.0),
        95.0,
        1e-3
    ));
    let lifted = [
        crouched(0.0, 10.0),
        LegHeight {
            ground: None,
            ..crouched(10.0, 20.0)
        },
    ];
    assert!(close(
        target_height(&lifted, SOLE, 0.0, 100.0, false, 1.0, 0.0, 0.0),
        45.0,
        1e-3
    ));
    // A foot whose ground is over a metre above the pelvis is not the
    // lowest: the other is, and caps the target at its reach.
    let above = [crouched(160.0, 170.0), crouched(0.0, 10.0)];
    assert!(close(
        target_height(&above, SOLE, 0.0, 50.0, false, 1.0, 0.0, 0.0),
        99.83,
        1e-3
    ));
}

/// A lifted lowest foot blends by the whole weight, a lifted other foot by
/// none, and the other's flag wins when both are lifted. The 40–60 cm
/// slope scales that weight. Past 60 cm, and when the other foot has no
/// ground, nothing is blended. Moving skips the cap and the raise.
#[test]
fn a_lifted_foot_sets_the_blend_weight() {
    let foot = |ground: f32, contact: f32, stance: f32, lifted: bool| LegHeight {
        ground: Some(ground),
        contact,
        stance,
        length: 85.15,
        lifted,
    };
    // Spread 20 cm: weights 1, 0 and 0.5. Heights 40 and 80, plus the 5 cm
    // raise, under the cap.
    let pair = |low: bool, other: bool| [foot(0.0, 10.0, 40.0, low), foot(0.0, 10.0, 80.0, other)];
    assert_eq!(
        target_height(&pair(true, false), SOLE, 0.0, 100.0, false, 1.0, 0.0, 0.0),
        85.0
    );
    assert_eq!(
        target_height(&pair(false, true), SOLE, 0.0, 100.0, false, 1.0, 0.0, 0.0),
        45.0
    );
    assert_eq!(
        target_height(&pair(true, true), SOLE, 0.0, 100.0, false, 1.0, 0.0, 0.0),
        45.0
    );
    assert_eq!(
        target_height(&pair(false, false), SOLE, 0.0, 100.0, false, 1.0, 0.0, 0.0),
        65.0
    );
    assert_eq!(
        target_height(&pair(true, false), SOLE, 0.0, 100.0, true, 1.0, 4.0, 0.0),
        80.0
    );
    assert_eq!(
        target_height(&pair(false, true), SOLE, 0.0, 100.0, true, 1.0, 4.0, 0.0),
        40.0
    );
    // Spread 50 cm: the slope takes 0.05 a cm of the weight, so a lifted
    // lowest foot blends by 0.5 (65 cm) and a planted pair by 0.25 (55).
    let sloped = |low: bool| [foot(0.0, 30.0, 40.0, low), foot(0.0, 20.0, 80.0, false)];
    assert_eq!(
        target_height(&sloped(true), SOLE, 0.0, 100.0, false, 1.0, 0.0, 0.0),
        65.0
    );
    assert_eq!(
        target_height(&sloped(false), SOLE, 0.0, 100.0, false, 1.0, 0.0, 0.0),
        55.0
    );
    // 5 cm a tick saturates the spread's mix, so the 50 cm slope still
    // applies and a lifted lowest foot blends by half.
    assert_eq!(
        target_height(&sloped(true), SOLE, 0.0, 100.0, true, 1.0, 5.0, 0.0),
        60.0
    );
    // Spread 70 cm: the flag does not blend past 60.
    let wide = [foot(0.0, 40.0, 40.0, true), foot(0.0, 30.0, 80.0, false)];
    assert_eq!(
        target_height(&wide, SOLE, 0.0, 100.0, false, 1.0, 0.0, 0.0),
        45.0
    );
    // The higher foot is the other one even when it is listed first. No
    // ground still leaves the lowest foot's height, lifted or not.
    let swapped = [foot(10.0, 20.0, 80.0, false), foot(0.0, 10.0, 40.0, true)];
    assert_eq!(
        target_height(&swapped, SOLE, 0.0, 100.0, false, 1.0, 0.0, 0.0),
        95.0
    );
    let unreachable = [
        foot(0.0, 10.0, 40.0, true),
        LegHeight {
            ground: None,
            stance: 80.0,
            ..foot(10.0, 20.0, 80.0, false)
        },
    ];
    assert_eq!(
        target_height(&unreachable, SOLE, 0.0, 100.0, false, 1.0, 0.0, 0.0),
        45.0
    );
}

/// Moving, the blend's half is min(0.5, 0.2·speed) and the weight is then
/// multiplied by min(1, (0.3·speed)²), speed the cm a tick along the
/// heading. At rest that product is 0, so the pelvis stays on the lowest
/// foot, lifted or not. At 4 cm a tick both factors saturate, so a planted
/// pair blends by half and a lifted lowest foot by all of it. A NaN speed
/// saturates the same way. Standing ignores the speed.
#[test]
fn the_moving_blend_follows_the_speed() {
    let foot = |stance: f32, lifted: bool| LegHeight {
        ground: Some(0.0),
        contact: 10.0,
        stance,
        length: 85.15,
        lifted,
    };
    let pair = |low: bool| [foot(40.0, low), foot(80.0, false)];
    assert_eq!(
        target_height(&pair(false), SOLE, 0.0, 100.0, true, 1.0, 0.0, 0.0),
        40.0
    );
    assert_eq!(
        target_height(&pair(true), SOLE, 0.0, 100.0, true, 1.0, 0.0, 0.0),
        40.0
    );
    assert_eq!(
        target_height(&pair(false), SOLE, 0.0, 100.0, true, 1.0, 4.0, 0.0),
        60.0
    );
    assert_eq!(
        target_height(&pair(true), SOLE, 0.0, 100.0, true, 1.0, 4.0, 0.0),
        80.0
    );
    assert_eq!(
        target_height(&pair(false), SOLE, 0.0, 100.0, true, 1.0, f32::NAN, 0.0),
        60.0
    );
    assert_eq!(
        target_height(&pair(false), SOLE, 0.0, 100.0, false, 1.0, 0.0, 0.0),
        65.0
    );
}

/// Moving, the spread is (1 − k)·lowest ground + k·(lowest contact + other
/// contact − other ground), k = min(1, 0.2·speed). Contacts 30 and 20 cm
/// over grounds at 0 are 50 cm apart, which the slope would cut; at 2.5 cm
/// a tick k is 0.5 and the spread is 25, so the weight is not cut, and at
/// 4 cm a tick the spread is 40, so a lifted lowest foot keeps its full
/// weight. At 5 cm a tick k is 1 and the 50 cm slope returns. A NaN speed
/// saturates k the same way. Standing keeps k at 1.
#[test]
fn the_moving_spread_follows_the_speed() {
    let foot = |contact: f32, stance: f32, lifted: bool| LegHeight {
        ground: Some(0.0),
        contact,
        stance,
        length: 85.15,
        lifted,
    };
    let pair = |lifted: bool| [foot(30.0, 40.0, lifted), foot(20.0, 80.0, false)];
    assert_eq!(
        target_height(&pair(false), SOLE, 0.0, 100.0, true, 1.0, 0.0, 0.0),
        40.0
    );
    assert_eq!(
        target_height(&pair(false), SOLE, 0.0, 100.0, true, 1.0, 2.5, 0.0),
        51.25
    );
    assert_eq!(
        target_height(&pair(true), SOLE, 0.0, 100.0, true, 1.0, 4.0, 0.0),
        80.0
    );
    assert_eq!(
        target_height(&pair(true), SOLE, 0.0, 100.0, true, 1.0, 5.0, 0.0),
        60.0
    );
    assert_eq!(
        target_height(&pair(true), SOLE, 0.0, 100.0, true, 1.0, f32::NAN, 0.0),
        60.0
    );
    assert_eq!(
        target_height(&pair(false), SOLE, 0.0, 100.0, false, 1.0, 0.0, 0.0),
        55.0
    );
}

/// Moving, a negative slow vertical chest speed (cm a tick) multiplies the
/// blend weight by max(0, 1 + 0.5·v). At 4 cm a tick a planted pair blends
/// by half, so −1 cm a tick leaves a quarter and −2 cm a tick leaves none.
/// The cut follows the 40–60 cm slope. A rise, a NaN and standing leave
/// the blend as it was.
#[test]
fn a_falling_chest_cuts_the_moving_blend() {
    let foot = |stance: f32, lifted: bool| LegHeight {
        ground: Some(0.0),
        contact: 10.0,
        stance,
        length: 85.15,
        lifted,
    };
    let pair = |low: bool| [foot(40.0, low), foot(80.0, false)];
    let height = |moving, speed, drop, low| {
        target_height(&pair(low), SOLE, 0.0, 100.0, moving, 1.0, speed, drop)
    };
    assert_eq!(height(true, 4.0, -1.0, false), 50.0);
    assert_eq!(height(true, 4.0, -2.0, false), 40.0);
    assert_eq!(height(true, 4.0, -4.0, false), 40.0);
    assert_eq!(height(true, 4.0, 1.0, false), 60.0);
    assert_eq!(height(true, 4.0, 0.0, false), 60.0);
    assert_eq!(height(true, 4.0, f32::NAN, false), 60.0);
    // A lifted lowest foot starts at weight 1, then the drop halves it.
    assert_eq!(height(true, 4.0, -1.0, true), 60.0);
    assert_eq!(height(false, 0.0, -4.0, false), 65.0);
    // Spread 50 cm at 5 cm a tick: the slope leaves half, and −2 cm a tick
    // takes the rest, so the target stays on the lowest foot.
    let sloped = |lifted: bool| {
        [
            LegHeight {
                contact: 30.0,
                ..foot(40.0, lifted)
            },
            LegHeight {
                contact: 20.0,
                ..foot(80.0, false)
            },
        ]
    };
    assert_eq!(
        target_height(&sloped(true), SOLE, 0.0, 100.0, true, 1.0, 5.0, -2.0),
        40.0
    );
}

/// Standing on its legs the lift holds a body up where f = 1, its pelvis
/// 1.7 cm over its target; set down 1 cm into the floor, it is pushed up
/// out of it, rises less than 2 cm over where it comes to rest, and rests.
#[test]
fn the_pelvis_settles_1_7_cm_over_its_target() {
    let (mut body, pose) = standing_at(Vec3::new(0.0, -0.01, 0.0));
    let floor = floor_quad(0.0, 5.0);
    let mut highest = 0.0f32;
    for _ in 0..600 {
        stand(&mut body, &pose, &floor);
        highest = highest.max(pelvis(&body));
    }
    let target = drive(&body).lift_target().unwrap();
    assert!(
        close(target, STANDING_LEG + STANDING_SOLE + 0.05, 1e-5),
        "{target}"
    );
    let over = pelvis(&body) - target;
    let (_, velocity) = body.centre_of_mass();
    assert!(
        close(over, 1.0 / 60.0, 0.002),
        "{:.2} cm over",
        over * 100.0
    );
    assert!(
        close(drive(&body).lift(), 1.0, 0.03),
        "f {}",
        drive(&body).lift()
    );
    assert!(highest - pelvis(&body) < 0.02);
    assert!(velocity.length() < 0.005, "still moving at {velocity}");
}

/// Without support the carry is gravity: a body with no ground under its
/// feet, at full tone and energy, falls at g, every part of it.
#[test]
fn no_support_gives_full_gravity() {
    let (mut body, pose) = standing_at(Vec3::ZERO);
    body.measure_balance(NONE);
    body.aim(Affine3A::IDENTITY, pose);
    assert!(!drive(&body).balance().has_support());
    assert_eq!(drive(&body).carry(), gravity_per_tick());
    assert_eq!(drive(&body).lift(), 0.0);
    let before: Vec<[Vec3; 4]> = body.parts.iter().map(|p| p.positions).collect();
    body.integrate();
    for (p, b) in body.parts.iter().zip(&before) {
        for (x, was) in p.positions.iter().zip(b) {
            let fell = *x - *was;
            assert!(
                fell.abs_diff_eq(gravity_per_tick(), 2e-7),
                "bone {}: {fell}",
                p.bone
            );
        }
    }
}

/// A fallen body lying on the floor is not lifted: its feet have no ground
/// within reach, so it has no support and its carry is gravity.
#[test]
fn a_fallen_body_is_not_lifted() {
    let (mut body, pose) = standing_at(Vec3::ZERO);
    let lie = Affine3A::from_translation(Vec3::Y * 0.15)
        * Affine3A::from_rotation_x(-std::f32::consts::FRAC_PI_2)
        * Affine3A::from_translation(Vec3::NEG_Y * 1.1);
    for p in &mut body.parts {
        p.positions = p.positions.map(|x| lie.transform_point3(x));
        p.previous = p.positions;
    }
    body.fall();
    let floor = floor_quad(0.0, 5.0);
    for _ in 0..20 {
        stand(&mut body, &pose, &floor);
        assert!(!drive(&body).balance().has_support());
        assert_eq!(drive(&body).carry(), gravity_per_tick());
        assert_eq!(drive(&body).lift(), 0.0);
    }
}

/// The lift scales by twice the tone over a half: at tone 0.75 the carry
/// takes off half of what it does at full tone, at 0.5 none, and the parts
/// then fall by gravity alone.
#[test]
fn the_lift_scales_with_the_tone() {
    let floor = floor_quad(0.0, 5.0);
    let lifted = |tone: f32| {
        let (mut body, pose) = standing_at(Vec3::new(0.0, -0.05, 0.0));
        body.drive.as_mut().unwrap().tone = tone;
        body.measure_balance(floor.as_slice());
        body.aim(Affine3A::IDENTITY, pose);
        let d = drive(&body);
        (
            d.lift(),
            (d.carry() - gravity_per_tick()).y / gravity_per_tick().y,
        )
    };
    let (full, taken) = lifted(1.0);
    assert!(full > 0.5 && close(taken, -full, 1e-5), "{full} {taken}");
    let (half, taken) = lifted(0.75);
    assert!(close(half, full / 2.0, 1e-5) && close(taken, -half, 1e-5));
    assert_eq!(lifted(0.5), (0.0, 0.0));
}

/// A collapsing body's lift is the standing one's times its tone squared as
/// well: at tone 0.9 it lifts 0.8·0.81 = 0.648 of the full lift (0.8 a
/// standing body's), at 0.75 half of 0.5625, and none at 0.5.
#[test]
fn a_collapsing_lift_fades_by_the_tone_squared() {
    let floor = floor_quad(0.0, 5.0);
    let lifted = |tone: f32, collapsing: bool| {
        let (mut body, pose) = standing_at(Vec3::new(0.0, -0.05, 0.0));
        let d = body.drive.as_mut().unwrap();
        d.tone = tone;
        d.bearing.collapsing = collapsing;
        body.measure_balance(floor.as_slice());
        body.aim(Affine3A::IDENTITY, pose);
        drive(&body).lift()
    };
    let full = lifted(1.0, false);
    assert!(close(lifted(1.0, true), full, 1e-6));
    assert!(close(lifted(0.9, false), 0.8 * full, 1e-5));
    assert!(close(lifted(0.9, true), 0.648 * full, 1e-5));
    assert!(close(lifted(0.75, true), 0.5 * 0.5625 * full, 1e-5));
    assert_eq!(lifted(0.5, true), 0.0);
}

/// A dazed body sags: its upper body takes the carry by its energy. Its
/// soles 25 cm deep, the lift's target is over where its legs reach, so it
/// hovers with its feet off the floor where its weight balances the lift,
/// f = Σm/Σam (a its parts' shares): its pelvis 0.3 cm over its target at
/// full energy, 1.2 cm under it at 0.75 and 3.0 cm under at 0.5.
#[test]
fn a_dazed_body_sags_in_proportion_to_its_energy() {
    let floor = floor_quad(0.0, 5.0);
    let hover = |energy: f32| {
        let (mut body, pose) = standing_at(Vec3::new(0.0, 0.2, 0.0));
        body.sole = 0.25;
        // Its feet do not touch (as a walking body's), so that only the
        // lift holds it.
        body.drive.as_mut().unwrap().bearing.walking = true;
        // Outside both plant windows (phases 0.45 and 0.95), so the lift is
        // what holds the feet up.
        body.drive.as_mut().unwrap().bearing.progress = 0.25;
        for _ in 0..800 {
            // The tick recovers the double 0.005 before the parts take the carry.
            body.drive.as_mut().unwrap().energy = energy - 0.005;
            stand(&mut body, &pose, &floor);
        }
        let lowest = body
            .parts
            .iter()
            .flat_map(|p| p.positions)
            .fold(f32::MAX, |low, x| low.min(x.y));
        assert!(
            lowest > 0.05,
            "a foot touches the floor at {energy}: {lowest}"
        );
        let (mass, carried) = body.parts.iter().fold((0.0, 0.0), |(m, am), p| {
            let a = carry_share(p.bone, 1.0, energy, 1.0).unwrap();
            (m + 1.0 / p.inverse_mass, am + a / p.inverse_mass)
        });
        let f: f32 = mass / carried;
        let want = -(f / 0.075 - 15.0);
        let over = (pelvis(&body) - drive(&body).lift_target().unwrap()) * 100.0;
        assert!(
            close(over, want, 0.05),
            "energy {energy}: {over} cm, not {want}"
        );
        over
    };
    let overs = [1.0, 0.75, 0.5].map(hover);
    for (over, want) in overs.iter().zip([0.28, -1.21, -3.05]) {
        assert!(close(*over, want, 0.02), "{overs:?}");
    }
}

/// The target follows the ground: a body standing on a 30 cm platform that
/// is taken away drops onto the floor (carried by its controller, it keeps
/// its balance in the air), and its target and pelvis settle 30 cm lower.
#[test]
fn the_target_follows_a_30_cm_step() {
    let (mut body, pose) = standing_at(Vec3::new(0.0, 0.3, 0.0));
    let (high, low) = (floor_quad(0.3, 5.0), floor_quad(0.0, 5.0));
    for _ in 0..300 {
        stand(&mut body, &pose, &high);
    }
    let (target, standing) = (drive(&body).lift_target().unwrap(), pelvis(&body));
    let mut lowest = f32::MAX;
    for _ in 0..500 {
        stand(&mut body, &pose, &low);
        lowest = lowest.min(pelvis(&body));
        assert!(drive(&body).balance().value > 0.6);
    }
    let dropped = target - drive(&body).lift_target().unwrap();
    assert!(close(dropped, 0.3, 1e-4), "the target dropped {dropped}");
    let settled = pelvis(&body);
    assert!(
        close(standing - settled, 0.3, 0.005),
        "{}",
        standing - settled
    );
    // Landing, it sinks no more than 2 cm under where it settles.
    assert!(settled - lowest < 0.02, "{}", settled - lowest);
}

/// The feet's lag: a foot 10 cm behind its animated spot along the facing
/// cuts the factor by (10/60)² (size 1); 60 cm behind cuts it to 0. The
/// feet routine measures it from last tick's animated contact, so the body
/// aims once before the feet move.
#[test]
fn the_feet_lag_cuts_the_push() {
    let (mut body, pose) = standing_at(Vec3::ZERO);
    let floor = floor_quad(0.0, 5.0);
    body.measure_balance(&floor[..]);
    body.aim(Affine3A::IDENTITY, pose.clone());
    body.set_bearing(Bearing {
        moving: true,
        walking: true,
        facing: Vec3::Z,
        align: 1.0,
        ..Bearing::default()
    });
    body.measure_balance(&floor[..]);
    assert!(close(drive(&body).bearing.lag, 1.0, 1e-4));
    let shins: Vec<usize> = body.legs.iter().map(|l| l.shin).collect();
    for &shin in &shins {
        for x in &mut body.parts[shin].positions {
            *x -= Vec3::Z * 0.1;
        }
    }
    body.measure_balance(&floor[..]);
    let lag = drive(&body).bearing.lag;
    let one = 1.0 - (10.0f32 / 60.0).powi(2);
    assert!(close(lag, one * one, 1e-5), "{lag}");
    for &shin in &shins {
        for x in &mut body.parts[shin].positions {
            *x -= Vec3::Z * 0.5;
        }
    }
    body.measure_balance(&floor[..]);
    assert_eq!(drive(&body).bearing.lag, 0.0);
}

/// A moving body's carry takes the locomotion drive × S² × the push gain ×
/// the feet's lag, in its cm a tick²; a still one takes none of it.
#[test]
fn the_carry_takes_the_locomotion_push() {
    let (mut body, pose) = standing_at(Vec3::ZERO);
    let floor = floor_quad(0.0, 5.0);
    body.measure_balance(&floor[..]);
    body.set_bearing(Bearing {
        moving: true,
        walking: true,
        push: Vec3::X * 2.0,
        push_gain: 0.5,
        facing: Vec3::Z,
        ..Bearing::default()
    });
    body.aim(Affine3A::IDENTITY, pose.clone());
    let d = drive(&body);
    let expected = 2.0 * d.support() * d.support() * 0.5 * d.bearing.lag / CM;
    assert!(close(d.carry().x, expected, 1e-6), "{}", d.carry().x);
    assert_eq!(d.carry().z, 0.0);
    body.set_bearing(Bearing {
        moving: false,
        push: Vec3::X * 2.0,
        push_gain: 0.5,
        ..Bearing::default()
    });
    body.aim(Affine3A::IDENTITY, pose);
    assert_eq!(drive(&body).carry().x, 0.0);
}

/// Only a walking condition measures the feet's lag: any other condition
/// (an attack, say) pushes with the factor 1 even with a foot far behind.
#[test]
fn only_a_walking_condition_measures_the_lag() {
    let (mut body, pose) = standing_at(Vec3::ZERO);
    let floor = floor_quad(0.0, 5.0);
    body.measure_balance(&floor[..]);
    let shins: Vec<usize> = body.legs.iter().map(|l| l.shin).collect();
    for &shin in &shins {
        for x in &mut body.parts[shin].positions {
            *x -= Vec3::Z * 0.7;
        }
    }
    body.set_bearing(Bearing {
        moving: true,
        walking: false,
        push: Vec3::X * 2.0,
        push_gain: 0.5,
        facing: Vec3::Z,
        ..Bearing::default()
    });
    body.aim(Affine3A::IDENTITY, pose);
    let d = drive(&body);
    assert_eq!(d.bearing.lag, 1.0);
    let expected = 2.0 * d.support() * d.support() * 0.5 / CM;
    assert!(close(d.carry().x, expected, 1e-6), "{}", d.carry().x);
}
