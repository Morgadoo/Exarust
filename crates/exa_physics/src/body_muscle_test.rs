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
            spheres: None,
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
            spheres: None,
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

/// The root is taken as the energy is: a NaN stays a NaN strength, and a
/// negative root is a NaN. Damping squares E − 0.5 unless that difference
/// compares below 0, so a NaN damping stays and a negative energy damps
/// nothing. −0 is not below 0.5, but −0 − 0.5 is, and √(−0) stays −0.
#[test]
fn a_nan_energy_keeps_a_nan_muscle_strength() {
    let gains = |energy| {
        Drive {
            energy,
            ..Drive::default()
        }
        .muscle_gains()
    };
    let (strength, damping) = gains(f32::NAN);
    assert!(strength.is_nan());
    assert!(damping.is_nan());
    let (strength, damping) = gains(-1.0);
    assert!(strength.is_nan());
    assert_eq!(damping, 0.0);
    let (strength, damping) = gains(-0.0);
    assert_eq!(strength.to_bits(), (-0.0f32).to_bits());
    assert_eq!(damping, 0.0);
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
        legs: LegMode::Walk,
        ..Bearing::default()
    };
    let mut s = human.stance;
    for _ in 0..19 {
        s = stance_factor(s, &walk, &human);
    }
    assert!(s < 0.7 - 1e-3, "{s}");
    s = stance_factor(s, &walk, &human);
    assert!((s - 0.7).abs() < 1e-5, "{s}");
    let run = Bearing {
        stance_run: 1.0,
        legs: LegMode::Run,
        ..walk
    };
    for _ in 0..30 {
        s = stance_factor(s, &run, &human);
    }
    assert!((s - 1.0).abs() < 1e-5, "{s}");
    assert_eq!(stance_factor(s, &walk, &human), s);
    assert_eq!(stance_factor(s, &Bearing::default(), &human), 0.5);
    // The rise is in double precision and stops on the target: 0.695 +
    // 0.01 at a half run blend's target of 0.85 is 0.705; 0.845 is 0.85.
    let half = Bearing {
        stance_run: 0.5,
        ..walk
    };
    assert_eq!(stance_factor(0.695, &half, &human), 0.705);
    assert_eq!(stance_factor(0.845, &half, &human), 0.85);
    // Standing dormant, or in the shock's stagger, it is 1.
    let full = Bearing {
        full_stance: true,
        ..Bearing::default()
    };
    assert_eq!(stance_factor(0.7, &full, &human), 1.0);
    assert_eq!(
        stance_factor(
            0.7,
            &Bearing {
                full_stance: true,
                ..half
            },
            &human
        ),
        0.71
    );
}

/// One arm tick through the muscle tune: both stored stiffnesses set to
/// `x`, the tone and bearing given, then `aim`; returns the right arm's
/// stiffness, which a branch that writes both arms applies to both.
fn one_arm_tick(x: f32, tone: f32, bearing: Bearing, race: RaceMuscles, stats: BodyStats) -> f32 {
    let mut body = both_arms();
    {
        let d = body.drive.as_mut().unwrap();
        d.race = race;
        d.stats = stats;
        d.tone = tone;
    }
    body.set_arm_stiffness(x);
    body.set_bearing(bearing);
    body.aim(Affine3A::IDENTITY, vec![Affine3A::IDENTITY; 3]);
    body.arm_stiffness().unwrap()
}

/// The arms' stiffness: easing 0.8x + 0.08 to 0.4 standing (a skeleton's
/// to 0.3), 0.2 striding or dazed, 0.5 off balance, and at least 0.4 × the
/// turn left (at most 0.4). A kick keeps it. A power is 0.3 and a weapon
/// switch 0.4. Its arm parts' anchors relax at most
/// 0.6 − (x − 0.1)·0.8: 0.36 standing, 0.52 striding, 0.28 off balance.
#[test]
fn arm_stiffness_follows_the_bodys_state() {
    let human = RaceMuscles::HUMAN;
    let alive = BodyStats::HUMAN;
    let tick = |x, b: Bearing, tone, race: RaceMuscles| one_arm_tick(x, tone, b, race, alive);
    let still = Bearing::default();
    let mut x = 0.0;
    for _ in 0..60 {
        x = tick(x, still, 1.0, human);
    }
    assert!((x - 0.4).abs() < 1e-4, "{x}");
    assert!((tick(0.0, still, 1.0, human) - 0.08).abs() < 1e-6);
    let skeleton = RaceMuscles {
        stance: 0.75,
        arm_relaxation: 0.5,
        ..RaceMuscles::HUMAN
    };
    let mut y = 0.0;
    for _ in 0..60 {
        y = tick(y, still, 1.0, skeleton);
    }
    assert!((y - 0.3).abs() < 1e-4, "{y}");
    let striding = Bearing {
        striding: true,
        ..still
    };
    assert_eq!(tick(x, striding, 1.0, human), 0.2);
    assert_eq!(tick(x, still, 0.8, human), 0.2);
    let fallen = Bearing {
        off_balance: true,
        ..striding
    };
    assert_eq!(tick(x, fallen, 0.5, human), 0.5);
    let turning = Bearing {
        turn: -0.75,
        ..striding
    };
    assert!((tick(x, turning, 1.0, human) - 0.3).abs() < 1e-6);
    let about = Bearing { turn: 3.0, ..still };
    assert_eq!(tick(0.0, about, 1.0, human), 0.4);
    for (x, cap) in [(0.4, 0.36), (0.2, 0.52), (0.5, 0.28)] {
        assert!((arm_relax_cap(x) - cap).abs() < 1e-6, "{x}");
    }
    // A kick keeps the stiffness, turn and all. A power is 0.3 and a
    // weapon switch 0.4, and the setter's floor raises either.
    let kicking = Bearing {
        kicking: true,
        turn: 2.0,
        ..still
    };
    assert_eq!(tick(0.55, kicking, 1.0, human), 0.55);
    assert_eq!(
        tick(
            0.55,
            Bearing {
                kicking: true,
                ..fallen
            },
            1.0,
            human
        ),
        0.5
    );
    let casting = Bearing {
        casting: true,
        ..still
    };
    assert_eq!(tick(0.8, casting, 1.0, human), 0.3);
    assert_eq!(
        tick(
            0.8,
            Bearing {
                casting: true,
                turn: 2.0,
                ..still
            },
            1.0,
            human
        ),
        0.4
    );
    let switching = Bearing {
        switching: true,
        ..still
    };
    assert_eq!(tick(0.2, switching, 1.0, human), 0.4);
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
    assert_eq!(one_arm_tick(0.4, 1.0, striding, human, zombie), 0.05);
    let half = BodyStats {
        walking_arms: 0.125,
        ..BodyStats::HUMAN
    };
    assert_eq!(one_arm_tick(0.4, 1.0, striding, human, half), 0.125);
    assert_eq!(one_arm_tick(0.4, 0.8, striding, human, zombie), 0.2);
    let fallen = Bearing {
        off_balance: true,
        ..striding
    };
    assert_eq!(one_arm_tick(0.4, 1.0, fallen, human, zombie), 0.5);
}

/// A shield in the off hand doubles that arm while the body strides at
/// full tone and on balance. The weapon arm keeps the walking arms. Off
/// balance, a daze and a parry do not take the double.
#[test]
fn a_shield_stride_doubles_the_off_arm() {
    let bare = |bone: usize| PartSetup {
        bone,
        entity: Entity::PLACEHOLDER,
        parent: None,
        world: Affine3A::IDENTITY,
        centre: Vec3::ZERO,
        half: Vec3::splat(0.05),
        origin: Vec3::ZERO,
        links: vec![],
        foot: None,
        pins: Vec::new(),
        spheres: None,
    };
    let mut body = Ragdoll::new(&[bare(12), bare(15)]).with_drive();
    let pose = vec![Affine3A::IDENTITY; 2];
    let factor =
        |body: &Ragdoll, bone: usize| body.parts.iter().find(|p| p.bone == bone).unwrap().factor;
    let stride = Bearing {
        striding: true,
        shield: true,
        ..Bearing::default()
    };
    body.set_bearing(stride);
    body.aim(Affine3A::IDENTITY, pose.clone());
    // Walking arms 0.2, times 1.25. The off arm (bone 15) is twice that.
    assert_eq!(factor(&body, 12), 0.2 * 1.25);
    assert_eq!(factor(&body, 15), 0.4 * 1.25);
    body.set_bearing(Bearing {
        striding: true,
        ..Bearing::default()
    });
    body.aim(Affine3A::IDENTITY, pose.clone());
    assert_eq!(factor(&body, 12), 0.2 * 1.25);
    assert_eq!(factor(&body, 15), 0.2 * 1.25);
    body.set_bearing(Bearing {
        striding: true,
        shield: true,
        off_balance: true,
        ..Bearing::default()
    });
    body.aim(Affine3A::IDENTITY, pose.clone());
    assert_eq!(factor(&body, 12), 0.5 * 1.25);
    assert_eq!(factor(&body, 15), 0.5 * 1.25);
    body.set_bearing(stride);
    body.drive.as_mut().unwrap().tone = 0.8;
    body.aim(Affine3A::IDENTITY, pose.clone());
    assert_eq!(factor(&body, 12), 0.2 * 1.25);
    assert_eq!(factor(&body, 15), 0.2 * 1.25);
    body.drive.as_mut().unwrap().tone = 1.0;
    body.set_arm_stiffness(0.5);
    body.set_bearing(Bearing {
        striding: true,
        shield: true,
        parrying: true,
        ..Bearing::default()
    });
    body.aim(Affine3A::IDENTITY, pose);
    let eased = crate::motile::parry_blend(0.5) * 1.25;
    assert_eq!(factor(&body, 12), eased);
    assert_eq!(factor(&body, 15), eased);
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
        legs: LegMode::Walk,
        ..Bearing::default()
    });
    for _ in 0..200 {
        body.aim(Affine3A::IDENTITY, pose.clone());
    }
    assert!((body.links[limit].strength - 0.4).abs() < 1e-4);
    assert!((body.parts[thigh].factor - 0.2).abs() < 1e-6);
    // Running the thigh pulls at 0.6; backpedalling, and stepping (which
    // the decision does not count as locomoting), as standing.
    body.set_bearing(Bearing {
        moving: true,
        legs: LegMode::Run,
        ..Bearing::default()
    });
    body.aim(Affine3A::IDENTITY, pose.clone());
    assert_eq!(body.parts[thigh].factor, 0.6);
    assert!((body.links[limit].strength - 0.4).abs() < 1e-4);
    for legs in [LegMode::Backpedal, LegMode::Standing] {
        body.set_bearing(Bearing {
            moving: true,
            legs,
            ..Bearing::default()
        });
        body.aim(Affine3A::IDENTITY, pose.clone());
        assert!((body.parts[thigh].factor - 0.25).abs() < 1e-6);
    }
    // A collapsing body's legs keep their gains.
    body.set_bearing(Bearing {
        legs: LegMode::Run,
        collapsing: true,
        ..Bearing::default()
    });
    let kept = body.links[limit].strength;
    body.aim(Affine3A::IDENTITY, pose.clone());
    assert!((body.parts[thigh].factor - 0.25).abs() < 1e-6);
    assert_eq!(body.links[limit].strength, kept);
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
            spheres: None,
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
        legs: LegMode::Walk,
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
            one_arm_tick(x, tone, collapsing, human, BodyStats::HUMAN),
            0.2
        );
    }
}

/// A looking head's link anchors are placed relative to the thorax where
/// it physically is: with the whole pose turned 0.5 rad about the thorax,
/// a head that does not look keeps the anchors of the unturned pose (its
/// turn relative to the thorax's target is the same), one that looks takes
/// the turn into its anchors.
#[test]
fn a_looking_heads_anchors_follow_the_physical_thorax() {
    let thorax = Affine3A::from_translation(Vec3::new(0.0, 1.4, 0.0));
    let neck = cm(0.0, 20.0, 0.0);
    let setup = [
        PartSetup {
            bone: 9,
            entity: Entity::PLACEHOLDER,
            parent: None,
            world: thorax,
            centre: Vec3::ZERO,
            half: cm(15.0, 15.0, 10.0),
            origin: Vec3::ZERO,
            links: vec![],
            foot: None,
            pins: Vec::new(),
            spheres: None,
        },
        PartSetup {
            bone: 10,
            entity: Entity::PLACEHOLDER,
            parent: Some(0),
            world: thorax * Affine3A::from_translation(neck),
            centre: cm(0.0, 10.0, 0.0),
            half: cm(8.0, 10.0, 8.0),
            origin: neck,
            links: vec![(cm(0.0, 0.0, 10.0), cm(0.0, 20.0, 10.0), 0.05)],
            foot: None,
            pins: Vec::new(),
            spheres: None,
        },
    ];
    let still = vec![thorax, thorax * Affine3A::from_translation(neck)];
    let turn = Affine3A::from_rotation_y(0.5);
    let turned: Vec<Affine3A> = still
        .iter()
        .map(|t| thorax * turn * thorax.inverse() * *t)
        .collect();
    let anchors = |looks: bool, targets: &[Affine3A]| {
        let mut body = Ragdoll::new(&setup).with_drive();
        body.set_bearing(Bearing {
            head_looks: looks,
            ..Bearing::default()
        });
        body.aim(Affine3A::IDENTITY, targets.to_vec());
        body.links
            .iter()
            .filter_map(|l| l.anchor)
            .collect::<Vec<[f32; 4]>>()
    };
    let rest = anchors(false, &still);
    assert!(!rest.is_empty());
    assert_eq!(anchors(false, &turned), rest);
    let looked = anchors(true, &turned);
    let moved = looked
        .iter()
        .zip(&rest)
        .any(|(a, b)| a.iter().zip(b).any(|(x, y)| (x - y).abs() > 1e-3));
    assert!(moved, "{looked:?} {rest:?}");
    assert_eq!(anchors(true, &still).len(), rest.len());
}

/// A head that does not look places its anchors from its pose relative to
/// its parent's pose before the torso twist. Twisting only the thorax
/// 0.5 rad about the vertical, and naming the untwisted thorax, leaves
/// the anchors of the untwisted pose; the same targets without that parent
/// take the inverse twist.
#[test]
fn a_head_that_does_not_look_anchors_on_the_untwisted_thorax() {
    let thorax = Affine3A::from_translation(Vec3::new(0.0, 1.4, 0.0));
    let neck = cm(0.0, 20.0, 0.0);
    let setup = [
        PartSetup {
            bone: 9,
            entity: Entity::PLACEHOLDER,
            parent: None,
            world: thorax,
            centre: Vec3::ZERO,
            half: cm(15.0, 15.0, 10.0),
            origin: Vec3::ZERO,
            links: vec![],
            foot: None,
            pins: Vec::new(),
            spheres: None,
        },
        PartSetup {
            bone: 10,
            entity: Entity::PLACEHOLDER,
            parent: Some(0),
            world: thorax * Affine3A::from_translation(neck),
            centre: cm(0.0, 10.0, 0.0),
            half: cm(8.0, 10.0, 8.0),
            origin: neck,
            links: vec![(cm(0.0, 0.0, 10.0), cm(0.0, 20.0, 10.0), 0.05)],
            foot: None,
            pins: Vec::new(),
            spheres: None,
        },
    ];
    let head = thorax * Affine3A::from_translation(neck);
    let still = vec![thorax, head];
    let mut twisted = thorax;
    twisted.matrix3 = Affine3A::from_rotation_y(0.5).matrix3 * thorax.matrix3;
    let targets = vec![twisted, head];
    let anchors = |note: bool| {
        let mut body = Ragdoll::new(&setup).with_drive();
        body.set_bearing(Bearing {
            head_looks: false,
            ..Bearing::default()
        });
        if note {
            body.drive
                .as_mut()
                .unwrap()
                .note_head_pose_parent(Some(thorax));
        }
        body.aim(Affine3A::IDENTITY, targets.clone());
        body.links
            .iter()
            .filter_map(|l| l.anchor)
            .collect::<Vec<[f32; 4]>>()
    };
    let mut rest_body = Ragdoll::new(&setup).with_drive();
    rest_body.aim(Affine3A::IDENTITY, still);
    let rest: Vec<[f32; 4]> = rest_body.links.iter().filter_map(|l| l.anchor).collect();
    assert!(!rest.is_empty());
    assert_eq!(anchors(true), rest);
    let twisted_anchors = anchors(false);
    let moved = twisted_anchors
        .iter()
        .zip(&rest)
        .any(|(a, b)| a.iter().zip(b).any(|(x, y)| (x - y).abs() > 1e-3));
    assert!(moved, "{twisted_anchors:?} {rest:?}");
}

/// A powered golem suit halves the standing thigh factor and quarters the
/// leg limit's target; a stepping foot that is anchored eases the limit to
/// 0.2 (0.8s + 0.04) whatever the leg mode.
#[test]
fn the_golem_suit_and_a_stepping_foot_change_the_leg_gains() {
    let (mut setup, pose) = super::super::floor_test::humanoid(Vec3::ZERO);
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
    let golem = Bearing {
        golem_powered: true,
        ..Bearing::default()
    };
    let (factor, target) = leg_gains(LegMode::Standing, 1.25, 0.5, &golem);
    assert_eq!(factor, 1.25 * 0.2 * 0.5);
    assert_eq!(target, 0.5 * 0.5 * 0.5 * 0.25);
    body.set_bearing(golem);
    for _ in 0..300 {
        body.aim(Affine3A::IDENTITY, pose.clone());
    }
    assert!((body.parts[thigh].factor - 0.125).abs() < 1e-6);
    assert!((body.links[limit].strength - 0.125).abs() < 1e-4);
    // The stepping foot: anchored, the limit leaves the leg mode's target
    // for 0.8s + 0.04.
    let index = body.legs.iter().position(|l| l.shin == shin).unwrap();
    body.legs[index].state = super::super::FootState(super::super::FootState::ANCHORED);
    body.set_bearing(Bearing {
        moving: true,
        legs: LegMode::Run,
        stepping: true,
        ..Bearing::default()
    });
    let before = body.links[limit].strength;
    body.aim(Affine3A::IDENTITY, pose.clone());
    assert!((body.links[limit].strength - (before * 0.8 + 0.04)).abs() < 1e-6);
}

/// A right upper arm, its forearm, and a left forearm.
fn both_arms() -> Ragdoll {
    let bones = [12usize, 13, 16];
    let parents = [None, Some(0usize), None];
    let setup: Vec<PartSetup> = bones
        .into_iter()
        .zip(parents)
        .map(|(bone, parent)| PartSetup {
            bone,
            entity: Entity::PLACEHOLDER,
            parent,
            world: Affine3A::IDENTITY,
            centre: Vec3::ZERO,
            half: Vec3::splat(0.05),
            origin: if parent.is_some() {
                elbow()
            } else {
                Vec3::ZERO
            },
            links: vec![],
            foot: None,
            pins: Vec::new(),
            spheres: None,
        })
        .collect();
    Ragdoll::new(&setup).with_drive()
}

/// An attack tick writes each side's stiffness, in double for the cap and
/// the saturation, and the turn floor raises a stiffness only when it
/// compares above it. A tick with no attack arms keeps the eased stiffness.
#[test]
fn an_attack_writes_each_arms_stiffness() {
    let mut body = both_arms();
    body.set_attack_arms(Some(AttackArmTune {
        stiffness: (0.8, 0.2),
        drive: (1.0, 0.0),
    }));
    let pose = vec![Affine3A::IDENTITY; 3];
    body.aim(Affine3A::IDENTITY, pose.clone());
    let limbs = 1.25;
    // Right upper arm and forearm share the first stiffness.
    assert_eq!(body.parts[0].factor, 0.8 * limbs);
    assert_eq!(body.parts[1].factor, 0.8 * limbs);
    assert_eq!(body.parts[0].relax_cap, arm_relax_cap(0.8));
    assert_eq!(body.parts[0].saturation, arm_saturation(1.0));
    // The left forearm takes the second.
    assert_eq!(body.parts[2].factor, 0.2 * limbs);
    assert_eq!(body.parts[2].saturation, arm_saturation(0.0));
    assert_eq!(body.parts[2].relax_cap, arm_relax_cap(0.2));
    // Every branch takes the setter's cap, computed in double: for 0.8 it
    // is 0.6 - 0.7·0.8 = 0.03999999, not the single-precision 0.04000002.
    assert_eq!(arm_relax_cap(0.8), 0.039_999_99);

    // Two radians of turn floors a 0.1 stiffness to 0.4: 0.4 × 1.25 = 0.5.
    body.set_bearing(Bearing {
        turn: 2.0,
        ..Bearing::default()
    });
    body.set_attack_arms(Some(AttackArmTune {
        stiffness: (0.1, 0.1),
        drive: (0.0, 0.0),
    }));
    body.aim(Affine3A::IDENTITY, pose.clone());
    assert_eq!(body.parts[0].factor, 0.4 * limbs);
    assert_eq!(body.parts[2].factor, 0.4 * limbs);
    // A NaN stiffness is left, and a NaN turn does not raise one.
    assert!(arm_stiffness_floor(f32::NAN, 2.0).is_nan());
    assert_eq!(arm_stiffness_floor(0.8, f32::NAN), 0.8);

    body.set_bearing(Bearing::default());
    body.set_attack_arms(None);
    body.aim(Affine3A::IDENTITY, pose.clone());
    let eased = body.arm_stiffness().unwrap();
    assert_eq!(body.parts[0].factor, eased * limbs);
    assert_eq!(body.parts[0].saturation, SATURATION);

    // A power sets both arms to 0.3 through the arm setter. The drive
    // slots are zeroed every tick before the state machine, so both sides
    // saturate at 0.1.
    body.set_attack_arms(Some(AttackArmTune {
        stiffness: (0.8, 0.2),
        drive: (1.0, 0.0),
    }));
    body.aim(Affine3A::IDENTITY, pose.clone());
    body.set_attack_arms(None);
    body.set_bearing(Bearing {
        casting: true,
        ..Bearing::default()
    });
    body.aim(Affine3A::IDENTITY, pose.clone());
    assert_eq!(body.parts[0].factor, 0.3 * limbs);
    assert_eq!(body.parts[2].factor, 0.3 * limbs);
    assert_eq!(body.parts[0].relax_cap, arm_relax_cap(0.3));
    assert_eq!(body.parts[0].saturation, arm_saturation(0.0));
    assert_eq!(body.parts[2].saturation, arm_saturation(0.0));

    // A weapon switch sets both to 0.4. The drives are zero again.
    body.set_bearing(Bearing {
        switching: true,
        ..Bearing::default()
    });
    body.aim(Affine3A::IDENTITY, pose.clone());
    assert_eq!(body.parts[0].factor, 0.4 * limbs);
    assert_eq!(body.parts[2].relax_cap, arm_relax_cap(0.4));
    assert_eq!(body.parts[2].saturation, arm_saturation(0.0));

    // A kick writes nothing: the switch's arms stay, turn or not.
    let kept = (
        body.parts[0].factor,
        body.parts[2].factor,
        body.parts[0].saturation,
    );
    body.set_bearing(Bearing {
        kicking: true,
        turn: 2.0,
        ..Bearing::default()
    });
    body.aim(Affine3A::IDENTITY, pose);
    assert_eq!(
        (
            body.parts[0].factor,
            body.parts[2].factor,
            body.parts[0].saturation
        ),
        kept
    );
    assert_eq!(body.arm_stiffness(), Some(0.4));
}

/// After an attack the two arms stay apart. A standing tick eases each
/// stored stiffness; the drive slots are zeroed first, so both saturate
/// at 0.1. A kick leaves that pair. A power writes both the same, so the
/// next standing tick eases from there. A recoil's rest does too.
#[test]
fn a_standing_tick_eases_the_two_stored_stiffnesses() {
    let mut body = both_arms();
    let pose = vec![Affine3A::IDENTITY; 3];
    let limbs = 1.25;
    body.set_attack_arms(Some(AttackArmTune {
        stiffness: (0.8, 0.2),
        drive: (1.0, 0.0),
    }));
    body.aim(Affine3A::IDENTITY, pose.clone());
    body.set_attack_arms(None);
    body.aim(Affine3A::IDENTITY, pose.clone());
    let (right, left) = attack_arms((0.8, 0.2), false, false, 1.0, 0.0);
    assert_ne!(right, left);
    assert_eq!(body.arm_stiffness(), Some(right));
    assert_eq!(body.parts[0].factor, right * limbs);
    assert_eq!(body.parts[1].factor, right * limbs);
    assert_eq!(body.parts[2].factor, left * limbs);
    assert_eq!(body.parts[0].relax_cap, arm_relax_cap(right));
    assert_eq!(body.parts[2].relax_cap, arm_relax_cap(left));
    assert_eq!(body.parts[0].saturation, arm_saturation(0.0));
    assert_eq!(body.parts[2].saturation, arm_saturation(0.0));
    // The next standing tick eases on from that pair.
    body.aim(Affine3A::IDENTITY, pose.clone());
    let (right, left) = attack_arms((right, left), false, false, 1.0, 0.0);
    assert_eq!(body.parts[0].factor, right * limbs);
    assert_eq!(body.parts[2].factor, left * limbs);
    // A kick leaves the pair, turn and all.
    body.set_bearing(Bearing {
        kicking: true,
        turn: 2.0,
        ..Bearing::default()
    });
    body.aim(Affine3A::IDENTITY, pose.clone());
    assert_eq!(body.parts[0].factor, right * limbs);
    assert_eq!(body.parts[2].factor, left * limbs);
    assert_eq!(body.arm_stiffness(), Some(right));
    // A power stores one stiffness on both sides; the drives are zero.
    body.set_bearing(Bearing {
        casting: true,
        ..Bearing::default()
    });
    body.aim(Affine3A::IDENTITY, pose.clone());
    body.set_bearing(Bearing::default());
    body.aim(Affine3A::IDENTITY, pose.clone());
    let (both, other) = attack_arms((0.3, 0.3), false, false, 1.0, 0.0);
    assert_eq!(both, other);
    assert_eq!(body.parts[0].factor, both * limbs);
    assert_eq!(body.parts[2].factor, both * limbs);
    assert_eq!(body.parts[0].saturation, arm_saturation(0.0));
    assert_eq!(body.parts[2].saturation, arm_saturation(0.0));
    // A recoil's rest writes both, and the next standing tick eases from it.
    body.set_arm_stiffness(0.2);
    body.aim(Affine3A::IDENTITY, pose);
    let (rest, other) = attack_arms((0.2, 0.2), false, false, 1.0, 0.0);
    assert_eq!(rest, other);
    assert_eq!(body.arm_stiffness(), Some(rest));
    assert_eq!(body.parts[0].factor, rest * limbs);

    // A parry eases each stored arm by 0.8x + 0.04, then the floor.
    let pose = vec![Affine3A::IDENTITY; 3];
    let mut body = both_arms();
    body.drive.as_mut().unwrap().arms = (0.0, 0.5);
    body.set_bearing(Bearing {
        parrying: true,
        ..Bearing::default()
    });
    body.aim(Affine3A::IDENTITY, pose.clone());
    let right = crate::motile::parry_blend(0.0);
    let left = crate::motile::parry_blend(0.5);
    assert_eq!(right, 0.04);
    assert_eq!(body.arm_stiffness(), Some(right));
    assert_eq!(body.parts[0].factor, right * limbs);
    assert_eq!(body.parts[2].factor, left * limbs);
    assert_eq!(body.parts[0].relax_cap, arm_relax_cap(right));
    assert_eq!(body.parts[2].relax_cap, arm_relax_cap(left));
    assert_eq!(body.parts[0].saturation, arm_saturation(0.0));
    // Two radians of turn floors the eased 0.04 up to 0.4.
    body.set_bearing(Bearing {
        parrying: true,
        turn: 2.0,
        ..Bearing::default()
    });
    body.aim(Affine3A::IDENTITY, pose.clone());
    assert_eq!(body.arm_stiffness(), Some(0.4));
    assert_eq!(body.parts[2].factor, 0.4 * limbs);
    // A daze comes before the parry, so it wins over the blend.
    body.set_arm_stiffness(0.5);
    body.drive.as_mut().unwrap().tone = 0.8;
    body.set_bearing(Bearing {
        parrying: true,
        striding: true,
        casting: true,
        ..Bearing::default()
    });
    body.aim(Affine3A::IDENTITY, pose.clone());
    assert_eq!(body.arm_stiffness(), Some(0.2));
    // Off balance, a collapse, a kick and an attack do.
    body.set_arm_stiffness(0.5);
    body.set_bearing(Bearing {
        parrying: true,
        off_balance: true,
        ..Bearing::default()
    });
    body.aim(Affine3A::IDENTITY, pose.clone());
    assert_eq!(body.arm_stiffness(), Some(0.5));
    body.set_arm_stiffness(0.8);
    body.set_bearing(Bearing {
        parrying: true,
        collapsing: true,
        ..Bearing::default()
    });
    body.aim(Affine3A::IDENTITY, pose.clone());
    assert_eq!(body.arm_stiffness(), Some(0.2));
    body.set_arm_stiffness(0.55);
    body.drive.as_mut().unwrap().tone = 1.0;
    body.set_bearing(Bearing {
        parrying: true,
        kicking: true,
        ..Bearing::default()
    });
    body.aim(Affine3A::IDENTITY, pose.clone());
    assert_eq!(body.arm_stiffness(), Some(0.55));
    body.set_attack_arms(Some(AttackArmTune {
        stiffness: (0.8, 0.2),
        drive: (1.0, 0.0),
    }));
    body.set_bearing(Bearing {
        parrying: true,
        ..Bearing::default()
    });
    body.aim(Affine3A::IDENTITY, pose);
    assert_eq!(body.parts[0].factor, 0.8 * limbs);
    assert_eq!(body.parts[2].factor, 0.2 * limbs);
}

/// A shock skips the arm state machine: the arm parts keep last tick's
/// stiffness, cap and saturation, while the drive slots are still zeroed.
#[test]
fn a_shock_leaves_the_arms_alone() {
    let mut body = both_arms();
    let pose = vec![Affine3A::IDENTITY; 3];
    body.set_attack_arms(Some(AttackArmTune {
        stiffness: (0.8, 0.2),
        drive: (1.0, 0.0),
    }));
    body.aim(Affine3A::IDENTITY, pose.clone());
    let kept = [
        (
            body.parts[0].factor,
            body.parts[0].relax_cap,
            body.parts[0].saturation,
        ),
        (
            body.parts[2].factor,
            body.parts[2].relax_cap,
            body.parts[2].saturation,
        ),
    ];
    body.set_attack_arms(None);
    body.set_bearing(Bearing {
        shocked: true,
        ..Bearing::default()
    });
    body.aim(Affine3A::IDENTITY, pose);
    assert_eq!(
        [
            (
                body.parts[0].factor,
                body.parts[0].relax_cap,
                body.parts[0].saturation
            ),
            (
                body.parts[2].factor,
                body.parts[2].relax_cap,
                body.parts[2].saturation
            ),
        ],
        kept
    );
}

/// The relaxed blend eases by 0.8x + 0.08 while the body is stepping,
/// without the race's arm relaxation; otherwise by
/// 0.8x + 0.08·(0.5 + 0.5·relaxation). A half-relaxing race shows the
/// difference: 0.48 stepping against 0.46.
#[test]
fn the_relaxed_blend_reads_the_stepping_flag() {
    let pose = vec![Affine3A::IDENTITY; 3];
    let half = RaceMuscles {
        arm_relaxation: 0.5,
        ..RaceMuscles::HUMAN
    };
    let run = |stepping: bool| {
        let mut body = both_arms();
        body.drive.as_mut().unwrap().race = half;
        body.set_arm_stiffness(0.5);
        body.set_bearing(Bearing {
            stepping,
            ..Bearing::default()
        });
        body.aim(Affine3A::IDENTITY, pose.clone());
        body.arm_stiffness().unwrap()
    };
    assert_eq!(run(true), 0.48);
    assert_eq!(run(false), 0.46);
}

/// A golem's body links all take the smoothing 0.995 (the character
/// init's golem set-up); any other race keeps the smoothing its links'
/// points gave them.
#[test]
fn a_golems_links_are_smoothed_at_0_995() {
    let plain = arm().with_race(RaceMuscles::HUMAN);
    let golem = arm().with_race(RaceMuscles {
        golem: true,
        ..RaceMuscles::HUMAN
    });
    assert!(!golem.links.is_empty());
    for (p, g) in plain.links.iter().zip(&golem.links) {
        if g.flags & crate::body::LINK_BODY != 0 {
            assert_eq!(g.smoothing, GOLEM_LINK_SMOOTHING);
        } else {
            assert_eq!(g.smoothing, p.smoothing);
        }
    }
    assert!(
        plain
            .links
            .iter()
            .any(|l| l.smoothing != GOLEM_LINK_SMOOTHING)
    );
}
