//! Link muscles: the original's laws and gains, each pinned to its numbers
//! (`docs/formats/RSQ.md`, Physical body).
//! (Included from body_muscle.rs: `#[path = "body_muscle_test.rs"] mod tests;`)
use super::super::{PartSetup, drive_parts};
use super::*;

const NONE: &[[Vec3; 3]] = &[];

fn cm(x: f32, y: f32, z: f32) -> Vec3 {
    Vec3::new(x, y, z) * 0.01
}

/// Where the right forearm's origin sits in the upper arm.
fn elbow() -> Vec3 {
    cm(-26.06, -0.49, -2.0)
}

/// The rest angle of the elbow about its axis (y): the forearm's stored
/// parent point (10.39, 0, 6) cm is its point (0, 0, 12) cm turned by it.
const REST: f32 = std::f32::consts::FRAC_PI_3;

fn forearm_turn(angle: f32) -> Affine3A {
    Affine3A::from_translation(elbow()) * Affine3A::from_rotation_y(angle)
}

/// A living right upper arm with its forearm, as the original's links join
/// them (a 2.61 cm point on the elbow's axis, a 12 cm range point and the
/// origin pivot), the elbow at its rest angle.
fn arm() -> Ragdoll {
    let upper = Affine3A::from_translation(Vec3::new(2.0, 1.4, -1.0));
    let setup = [
        PartSetup {
            bone: 12,
            entity: Entity::PLACEHOLDER,
            parent: None,
            world: upper,
            centre: cm(-13.0, 0.0, 0.0),
            half: cm(13.0, 4.5, 4.5),
            origin: Vec3::ZERO,
            links: vec![],
            foot: None,
            pins: Vec::new(),
        },
        PartSetup {
            bone: 13,
            entity: Entity::PLACEHOLDER,
            parent: Some(0),
            world: upper * forearm_turn(REST),
            centre: cm(-12.0, 0.0, 0.0),
            half: cm(12.0, 4.0, 4.0),
            origin: elbow(),
            links: vec![
                (cm(0.0, 10.0, 0.0), cm(0.0, 10.0, 0.0), 0.02611),
                (cm(0.0, 0.0, 12.0), cm(10.39, 0.0, 6.0), 0.12),
            ],
            foot: None,
            pins: Vec::new(),
        },
    ];
    Ragdoll::new(&setup).with_drive()
}

/// The arm's pose with the elbow at `angle`, the upper arm where it is.
fn arm_pose(body: &Ragdoll, angle: f32) -> Vec<Affine3A> {
    let upper = body.parts[0].transform();
    vec![upper, upper * forearm_turn(angle)]
}

/// The elbow's angle about its axis.
fn elbow_angle(body: &Ragdoll) -> f32 {
    let upper = body.parts[0].transform();
    let fore = body.parts[1].transform();
    let z = upper.matrix3.inverse() * (fore.matrix3 * Vec3::Z);
    z.x.atan2(z.z)
}

/// Idle: D = (1 − 0.5)² = 0.25 and k = 40 / 0.5 = 80, so the damping gain
/// at a small error is 0.25·80/1.4 = 14.3, and 8.6 on the pelvis and legs;
/// walking, k = 50 gives 8.9. The strength A = √E·T is 0.5 idle (T = 0.5).
#[test]
fn damping_gains_are_the_originals() {
    let mut drive = Drive::default();
    let (strength, damping) = drive.muscle_gains();
    assert!((strength - 0.5).abs() < 1e-6);
    assert!((damping / 1.4 - 14.286).abs() < 1e-3, "{}", damping / 1.4);
    assert!((damping * LEG_DAMPING / 1.4 - 8.571).abs() < 1e-3);
    drive.bearing.moving = true;
    assert!((drive.muscle_gains().1 / 1.4 - 8.929).abs() < 1e-3);
    // A skeleton stands with stance factor 0.75: k = 53.3.
    let mut skeleton = Drive {
        race: RaceMuscles {
            stance: 0.75,
            arm_relaxation: 0.5,
            ..RaceMuscles::HUMAN
        },
        ..Drive::default()
    };
    skeleton.stance = 0.75;
    assert!((skeleton.muscle_gains().1 - 0.25 * 40.0 / 0.75).abs() < 1e-4);
}

/// The strength follows √E: a body dazed to a quarter of its energy pulls
/// at half its strength. The damping is (E − 0.5)², nothing at E = 0.5 or
/// below.
#[test]
fn strength_follows_the_root_of_energy_and_damping_stops_at_half() {
    let mut drive = Drive::default();
    let (full, _) = drive.muscle_gains();
    drive.energy = 0.25;
    let (dazed, damping) = drive.muscle_gains();
    assert!((dazed - 0.5 * full).abs() < 1e-6, "{dazed} against {full}");
    assert_eq!(damping, 0.0);
    for e in [0.5, 0.4, 0.05] {
        drive.energy = e;
        assert_eq!(drive.muscle_gains().1, 0.0, "E = {e}");
    }
    drive.energy = 0.75;
    assert!((drive.muscle_gains().1 - 0.0625 * 80.0).abs() < 1e-4);
}

/// The pull: the error plus its change times gain/(0.5|e|² + 1.4), then
/// e/(1 + 0.1|e|): 10 cm pulls 5 cm; 1 cm moving 0.1 cm with gain 20 pulls
/// (1 + 0.1·20/1.9)/(1 + 0.1·2.053) = 1.703 cm. Under 1.19e-7 cm², nothing.
#[test]
fn pulls_are_damped_and_saturated() {
    let pull = muscle_pull(Vec3::X * 10.0, Vec3::ZERO, 20.0, SATURATION).unwrap();
    assert!((pull.x - 5.0).abs() < 1e-5, "{pull}");
    let pull = muscle_pull(Vec3::X, Vec3::X * 0.1, 20.0, SATURATION).unwrap();
    assert!((pull.x - 1.703).abs() < 1e-3, "{pull}");
    assert_eq!(muscle_pull(Vec3::splat(1e-4), Vec3::ZERO, 20.0, 0.1), None);
    assert_eq!(muscle_pull(Vec3::NAN, Vec3::ZERO, 20.0, 0.1), None);
}

/// A moving body's stance factor rises 0.01 a tick from the race's 0.5 to
/// 0.7 walking (20 ticks) and 1 running, keeps 1 when the run blend falls
/// while it still moves, and is the race's again once it stands.
#[test]
fn the_stance_factor_rises_with_the_gait() {
    let human = RaceMuscles::HUMAN;
    let walk = Bearing {
        moving: true,
        ..Bearing::default()
    };
    let mut s = human.stance;
    for _ in 0..19 {
        s = stance_factor(s, &walk, &human);
    }
    assert!(s < 0.7 - 1e-3, "{s}");
    s = stance_factor(s, &walk, &human);
    assert!((s - 0.7).abs() < 1e-5, "{s}");
    let run = Bearing { run: 1.0, ..walk };
    for _ in 0..30 {
        s = stance_factor(s, &run, &human);
    }
    assert!((s - 1.0).abs() < 1e-5, "{s}");
    assert_eq!(stance_factor(s, &walk, &human), s);
    assert_eq!(stance_factor(s, &Bearing::default(), &human), 0.5);
}

/// The arms' stiffness: easing 0.8x + 0.08 to 0.4 standing (a skeleton's
/// to 0.3), 0.2 striding or dazed, 0.5 off balance, and at least 0.4 × the
/// turn left (at most 0.4). Its arm parts' anchors relax at most
/// 0.6 − (x − 0.1)·0.8: 0.36 standing, 0.52 striding, 0.28 off balance.
#[test]
fn arm_stiffness_follows_the_bodys_state() {
    let human = RaceMuscles::HUMAN;
    let alive = BodyStats::HUMAN;
    let arm_stiffness =
        |x, b: &Bearing, tone, race: &RaceMuscles| arm_stiffness(x, b, tone, race, &alive);
    let still = Bearing::default();
    let mut x = 0.0;
    for _ in 0..60 {
        x = arm_stiffness(x, &still, 1.0, &human);
    }
    assert!((x - 0.4).abs() < 1e-4, "{x}");
    assert!((arm_stiffness(0.0, &still, 1.0, &human) - 0.08).abs() < 1e-6);
    let skeleton = RaceMuscles {
        stance: 0.75,
        arm_relaxation: 0.5,
        ..RaceMuscles::HUMAN
    };
    let mut y = 0.0;
    for _ in 0..60 {
        y = arm_stiffness(y, &still, 1.0, &skeleton);
    }
    assert!((y - 0.3).abs() < 1e-4, "{y}");
    let striding = Bearing {
        striding: true,
        ..still
    };
    assert_eq!(arm_stiffness(x, &striding, 1.0, &human), 0.2);
    assert_eq!(arm_stiffness(x, &still, 0.8, &human), 0.2);
    let fallen = Bearing {
        off_balance: true,
        ..striding
    };
    assert_eq!(arm_stiffness(x, &fallen, 0.5, &human), 0.5);
    let turning = Bearing {
        turn: -0.75,
        ..striding
    };
    assert!((arm_stiffness(x, &turning, 1.0, &human) - 0.3).abs() < 1e-6);
    let about = Bearing { turn: 3.0, ..still };
    assert_eq!(arm_stiffness(0.0, &about, 1.0, &human), 0.4);
    for (x, cap) in [(0.4, 0.36), (0.2, 0.52), (0.5, 0.28)] {
        assert!((arm_relax_cap(x) - cap).abs() < 1e-6, "{x}");
    }
}

/// A feral character's arms hang looser in the walk and run cycles, at its
/// walking arms (a zombie's 0.05, a half-feral one's 0.125), but dazed
/// they are 0.2 and off balance 0.5 as anyone's.
#[test]
fn feral_arms_hang_loose_striding() {
    let human = RaceMuscles::HUMAN;
    let zombie = BodyStats {
        walking_arms: 0.05,
        ..BodyStats::HUMAN
    };
    let striding = Bearing {
        striding: true,
        ..Bearing::default()
    };
    assert_eq!(arm_stiffness(0.4, &striding, 1.0, &human, &zombie), 0.05);
    let half = BodyStats {
        walking_arms: 0.125,
        ..BodyStats::HUMAN
    };
    assert_eq!(arm_stiffness(0.4, &striding, 1.0, &human, &half), 0.125);
    assert_eq!(arm_stiffness(0.4, &striding, 0.8, &human, &zombie), 0.2);
    let fallen = Bearing {
        off_balance: true,
        ..striding
    };
    assert_eq!(arm_stiffness(0.4, &fallen, 1.0, &human, &zombie), 0.5);
}

/// Relaxation rises by its step and falls to its target at once; a
/// negative target (an anchor moving faster than 0.137 cm a tick) leaves
/// it where it was, as the original's does.
#[test]
fn relaxation_rises_slowly_falls_at_once_and_ignores_fast_motion() {
    assert!((relax(0.1, 0.6, 0.015) - 0.115).abs() < 1e-6);
    assert_eq!(relax(0.3, 0.2, 0.015), 0.2);
    assert_eq!(relax(0.3, -2.0, 0.015), 0.3);
    // A supported leg: its hip 92.17 cm from its foot contact against twice
    // the contact's 49.03 cm leaves no relaxation; at that reach, 0.2.
    assert_eq!(leg_relax_target(92.17f32.powi(2), 49.03), 0.0);
    assert!((leg_relax_target(98.06f32.powi(2), 49.03) - 0.2).abs() < 1e-6);
    assert!((leg_relax_target(98.06f32.powi(2) + 450.0, 49.03) - 0.15).abs() < 1e-3);
    // The contact is the ankle, 41.95 cm from the knee: a straight human
    // leg, its hip 85.15 cm from the ankle, relaxes at the cap of 0.2.
    assert!((leg_relax_target(85.15f32.powi(2), 41.95) - 0.2).abs() < 1e-6);
}

/// Each slack link's anchor is its point placed by the part's animated turn
/// relative to its parent, at the part's origin; while the pose holds still
/// the part's λ rises 0.015 a tick to its cap (0.36 for an idle arm), a
/// fast swing leaves it, and a slow drift sets it to 0.6 − 32|Δa|².
#[test]
fn anchors_follow_the_animated_joint_and_relax_while_it_holds() {
    let mut body = arm();
    let angle = 1.2;
    let pose = arm_pose(&body, angle);
    body.aim(Affine3A::IDENTITY, pose.clone());
    let range = &body.links[1];
    let want = forearm_turn(angle).transform_point3(cm(0.0, 0.0, 12.0));
    assert!(
        range.kept.unwrap().abs_diff_eq(want, 1e-5),
        "{:?}",
        range.kept
    );
    let anchor = range.anchor.unwrap();
    let on_parent = body.parts[0].rest_point(&anchor);
    assert!(on_parent.abs_diff_eq(want, 1e-5));
    // The pivot has no muscle.
    assert!(body.links[2].origin && body.links[2].anchor.is_none());
    assert!((body.parts[1].relax - 0.015).abs() < 1e-6);
    for _ in 0..40 {
        body.aim(Affine3A::IDENTITY, pose.clone());
    }
    assert!(
        (body.parts[1].relax - 0.36).abs() < 1e-4,
        "{}",
        body.parts[1].relax
    );
    // 0.84 cm a tick at the 12 cm point.
    let held = body.parts[1].relax;
    body.aim(Affine3A::IDENTITY, arm_pose(&body, angle + 0.07));
    assert_eq!(body.parts[1].relax, held);
    // 0.1 cm: 0.6 − 32·0.01 = 0.28.
    let step = 0.1 / 12.0;
    body.aim(Affine3A::IDENTITY, arm_pose(&body, angle + 0.07 + step));
    assert!(
        (body.parts[1].relax - (0.6 - 32.0 * 0.01)).abs() < 1e-3,
        "{}",
        body.parts[1].relax
    );
}

/// The character tick sets the legs: a thigh's factor 0.2 × 1.25 standing
/// and 0.2 moving; a lower leg's limit link's strength moves a tenth of the
/// way each tick from 0.2 to 0.5·S² standing (0.23 after one tick) or 0.4
/// moving, and back to 0 without support.
#[test]
fn the_legs_strength_follows_the_gait_and_support() {
    let (setup, pose) = super::super::floor_test::humanoid(Vec3::ZERO);
    let mut setup = setup;
    // The lower legs carry their foot contacts and a 12.66 cm limit.
    for s in &mut setup {
        if matches!(s.bone, 3 | 6) {
            s.foot = Some(cm(0.16, -48.97, 2.4));
            s.links
                .push((cm(0.0, -12.49, -2.08), cm(0.44, -2.31, -12.44), 0.12657));
        }
    }
    let mut body = Ragdoll::new(&setup).with_drive();
    let shin = body.parts.iter().position(|p| p.bone == 3).unwrap();
    let thigh = body.parts[shin].parent.unwrap();
    let limit = body.parts[shin].leg_limit.unwrap();
    assert_eq!(body.links[limit].strength, 0.2);
    body.aim(Affine3A::IDENTITY, pose.clone());
    assert!((body.links[limit].strength - 0.23).abs() < 1e-6);
    assert!((body.parts[thigh].factor - 0.25).abs() < 1e-6);
    for _ in 0..200 {
        body.aim(Affine3A::IDENTITY, pose.clone());
    }
    assert!((body.links[limit].strength - 0.5).abs() < 1e-4);
    body.set_bearing(Bearing {
        moving: true,
        ..Bearing::default()
    });
    for _ in 0..200 {
        body.aim(Affine3A::IDENTITY, pose.clone());
    }
    assert!((body.links[limit].strength - 0.4).abs() < 1e-4);
    assert!((body.parts[thigh].factor - 0.2).abs() < 1e-6);
    // Standing with no ground under its feet.
    body.set_bearing(Bearing {
        carried: false,
        ..Bearing::default()
    });
    for _ in 0..200 {
        body.measure_balance(NONE);
        body.aim(Affine3A::IDENTITY, pose.clone());
    }
    assert_eq!(body.drive.as_ref().unwrap().support(), 0.0);
    assert!(body.links[limit].strength < 1e-4);
}

/// The part factors: 0.5, a foot half its lower leg's, the head a quarter
/// of its thorax's; an idle arm settles at 0.4 × 1.25, so an idle human's
/// arm muscles pull with σ = 0.5 × 0.5 = 0.25, 0.175 striding (0.2 × 1.25 ×
/// 0.7) and 0.3125 off balance (0.5 × 1.25 × 0.5).
#[test]
fn part_factors_and_arm_strengths() {
    let bones = [
        (1, None),
        (9, Some(0)),
        (10, Some(1)),
        (3, Some(0)),
        (4, Some(3)),
    ];
    let setup: Vec<PartSetup> = bones
        .iter()
        .map(|&(bone, parent)| PartSetup {
            bone,
            entity: Entity::PLACEHOLDER,
            parent,
            world: Affine3A::IDENTITY,
            centre: Vec3::ZERO,
            half: Vec3::splat(0.05),
            origin: Vec3::ZERO,
            links: vec![],
            foot: None,
            pins: Vec::new(),
        })
        .collect();
    let body = Ragdoll::new(&setup);
    let factors: Vec<f32> = body.parts.iter().map(|p| p.factor).collect();
    assert_eq!(factors, [0.5, 0.5, 0.125, 0.5, 0.25]);
    // A zombie's head keeps a fifth of its muscle (1 − 0.8), a half-feral
    // one's 0.6: 0.025 and 0.075.
    for (head_factor, head) in [(0.2, 0.025), (0.6, 0.075)] {
        let stats = BodyStats {
            head_factor,
            ..BodyStats::HUMAN
        };
        let body = Ragdoll::new(&setup).with_drive().with_stats(stats);
        let factors: Vec<f32> = body.parts.iter().map(|p| p.factor).collect();
        assert_eq!(factors.len(), 5);
        assert!((factors[2] - head).abs() < 1e-7, "{factors:?}");
        assert_eq!(
            [factors[0], factors[1], factors[3], factors[4]],
            [0.5, 0.5, 0.5, 0.25]
        );
    }

    let sigma = |body: &Ragdoll| {
        let d = body.drive.as_ref().unwrap();
        body.links[1].strength * body.parts[1].factor * d.muscle_gains().0
    };
    let mut body = arm();
    for _ in 0..60 {
        let pose = arm_pose(&body, REST);
        body.aim(Affine3A::IDENTITY, pose);
    }
    assert!((sigma(&body) - 0.25).abs() < 1e-4, "{}", sigma(&body));
    body.set_bearing(Bearing {
        moving: true,
        striding: true,
        ..Bearing::default()
    });
    for _ in 0..30 {
        let pose = arm_pose(&body, REST);
        body.aim(Affine3A::IDENTITY, pose);
    }
    assert!((sigma(&body) - 0.175).abs() < 1e-4, "{}", sigma(&body));
    body.set_bearing(Bearing {
        off_balance: true,
        ..Bearing::default()
    });
    let pose = arm_pose(&body, REST);
    body.aim(Affine3A::IDENTITY, pose);
    assert!((sigma(&body) - 0.3125).abs() < 1e-4, "{}", sigma(&body));
    // A short character's (scale 0.95) limbs are 1.25 × 0.95 times their
    // stiffness: off balance 0.5 × 1.1875 × 0.5 = 0.2969.
    let mut short = arm().with_stats(BodyStats {
        scale: 0.95,
        ..BodyStats::HUMAN
    });
    short.set_bearing(Bearing {
        off_balance: true,
        ..Bearing::default()
    });
    let pose = arm_pose(&short, REST);
    short.aim(Affine3A::IDENTITY, pose);
    assert!(
        (sigma(&short) - 0.296_875).abs() < 1e-4,
        "{}",
        sigma(&short)
    );
}

/// The elbow follows the clip. Swung 40° over 10 ticks, it lags the clip
/// (the muscles damp the joint's own motion, not the clip's: about 18° when
/// the swing ends), is within 5° of it 20 ticks later, and closes on it
/// without overshooting.
#[test]
fn the_elbow_follows_an_animated_swing() {
    let mut body = arm();
    for _ in 0..30 {
        let pose = arm_pose(&body, REST);
        body.aim(Affine3A::IDENTITY, pose);
        body.tick(NONE);
    }
    assert!((elbow_angle(&body) - REST).abs() < 0.01);
    let swing = 40f32.to_radians();
    let (mut worst, mut beyond) = (0.0f32, 0.0f32);
    for tick in 1..=60 {
        let animated = REST + swing * (tick as f32 / 10.0).min(1.0);
        let pose = arm_pose(&body, animated);
        body.aim(Affine3A::IDENTITY, pose);
        body.tick(NONE);
        let off = (elbow_angle(&body) - animated).to_degrees();
        if tick >= 30 {
            worst = worst.max(off.abs());
        }
        beyond = beyond.max(off);
    }
    assert!(worst < 5.0, "the elbow lags the clip by {worst:.1}°");
    assert!(
        beyond < 0.5,
        "the elbow overshoots the clip by {beyond:.1}°"
    );
}

/// One pass of a link muscle moves the part and its parent by equal and
/// opposite momenta: the muscles never set a body moving.
#[test]
fn one_muscle_pass_keeps_the_momentum() {
    let mut body = arm();
    for _ in 0..5 {
        let pose = arm_pose(&body, REST);
        body.aim(Affine3A::IDENTITY, pose);
        body.tick(NONE);
    }
    // Aimed 50° away, the forearm moving.
    let pose = arm_pose(&body, REST + 50f32.to_radians());
    body.aim(Affine3A::IDENTITY, pose);
    for (k, p) in body.parts[1].previous.iter_mut().enumerate() {
        *p -= Vec3::new(0.002, -0.001 * k as f32, 0.003);
    }
    body.step();
    let before: Vec<[Vec3; 4]> = body.parts.iter().map(|p| p.positions).collect();
    let muscle = &body.links[1];
    let (part, parent) = (&body.parts[1], &body.parts[0]);
    let pull = muscle.muscle(part, parent, muscle.anchor.unwrap(), (0.5, 20.0));
    assert!(pull.is_some_and(|c| c.length() > 1e-3), "{pull:?}");
    body.iterate();
    let momentum: Vec3 = body
        .parts
        .iter()
        .zip(&before)
        .flat_map(|(p, b)| (0..4).map(move |i| (p.positions[i] - b[i]) / p.inverse_mass))
        .sum();
    let moved: f32 = body
        .parts
        .iter()
        .zip(&before)
        .map(|(p, b)| {
            (0..4)
                .map(|i| (p.positions[i] - b[i]).length())
                .sum::<f32>()
        })
        .sum();
    assert!(moved > 1e-3, "the pass moved nothing");
    assert!(
        momentum.length() < 1e-5,
        "a muscle pass moved the body by {momentum}"
    );
}

/// The pose drive turns only the pelvis, feet, spine and head: an arm or a
/// leg aimed away from how it is turned keeps its particles through the
/// drive.
#[test]
fn the_pose_drive_leaves_the_limbs_to_their_muscles() {
    let (setup, pose) = super::super::floor_test::humanoid(Vec3::ZERO);
    let mut body = Ragdoll::new(&setup).with_drive();
    let away: Vec<Affine3A> = pose
        .iter()
        .map(|t| *t * Affine3A::from_rotation_y(0.3))
        .collect();
    body.aim(Affine3A::IDENTITY, pose);
    body.aim(Affine3A::IDENTITY, away);
    let before: Vec<[Vec3; 4]> = body.parts.iter().map(|p| p.positions).collect();
    let Ragdoll { parts, drive, .. } = &mut body;
    drive_parts(parts, drive.as_ref().unwrap());
    for (p, b) in body.parts.iter().zip(&before) {
        let moved = p.positions != *b;
        let driven = matches!(p.bone, 1 | 4 | 7..=10);
        assert_eq!(moved, driven, "bone {}", p.bone);
    }
}

/// A collapsing body's arms are 0.2, off balance or not, at any tone, and
/// whatever they were (the original's collapse branch sets 0.2 every tick;
/// its turn error is 0 then).
#[test]
fn collapsing_arms_are_0_2() {
    let human = RaceMuscles::HUMAN;
    let collapsing = Bearing {
        collapsing: true,
        off_balance: true,
        ..Bearing::default()
    };
    for (x, tone) in [(0.5, 1.0), (0.4, 0.6), (0.05, 0.26)] {
        assert_eq!(
            arm_stiffness(x, &collapsing, tone, &human, &BodyStats::HUMAN),
            0.2
        );
    }
}
