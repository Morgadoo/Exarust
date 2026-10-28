//! The standing leg bend, pinned to the law of cosines and the one-tick lag.
//! (Included from body_bend.rs.)
use bevy::math::{Affine3A, Quat, Vec3};

use super::super::floor_test::standing_at;
use super::super::tests::floor_quad;
use super::super::{Bearing, Ragdoll};
use super::*;

fn close(a: f32, b: f32) -> bool {
    (a - b).abs() < 1e-3
}

/// Both bones 40 cm, hip-to-target 40 cm: cosine 0.5, knee 60°. At 20 cm
/// the cosine is 0.875, above 0.866, so the knee is held at 30°.
#[test]
fn the_knee_is_the_law_of_cosines() {
    assert!(close(knee_angle(0.4, 0.4, 0.4), 1.0471976));
    assert!(close(knee_angle(0.4, 0.4, 0.2), 0.5235988));
}

/// From a zero correction and a zero rate, one tick toward a larger
/// correction stores the rate (0.02, or 0.2 while stepping) and leaves the
/// rate at half of that. It never passes the target.
#[test]
fn the_correction_advances_by_the_rate() {
    let (angle, rate) = ease(0.0, 0.0, 1.0, 0.02);
    assert!(close(angle, 0.02));
    assert!(close(rate, 0.01));
    let (stepping, stepping_rate) = ease(0.0, 0.0, 1.0, 0.2);
    assert!(close(stepping, 0.2));
    assert!(close(stepping_rate, 0.1));
    let (held, _) = ease(0.0, 0.0, 0.01, 0.02);
    assert!(close(held, 0.01));
}

/// An anchored target is the midpoint, at the anchor's height. An
/// unanchored one, standing at the target height, is the anchor lowered
/// by twice 1.125 cm.
#[test]
fn the_bend_target_drops_an_unanchored_foot() {
    let foot = Vec3::new(0.0, 0.4, 0.0);
    let anchor = Vec3::new(0.2, 0.5, 0.0);
    let mid = bend_target(foot, anchor, true, 0.05);
    assert!(close((mid.x - foot.x).abs(), 0.1));
    assert!(close((mid.z - foot.z).abs(), 0.0));
    assert!(close(mid.y, anchor.y));
    let raw = super::super::lift::lift_unclamped(0.0, &Bearing::default(), 1.0);
    assert!(close(raw, 1.125));
    let dropped = bend_target(foot, anchor, false, 2.0 * raw / super::super::CM);
    assert!(close(dropped.y, anchor.y - 0.0225));
    assert!(close(dropped.x, anchor.x) && close(dropped.z, anchor.z));
}

/// 75% of 40° is 30°. Not stepping, 30° loses a tenth and is 27°.
#[test]
fn the_thigh_turns_three_quarters_and_then_decays() {
    let rest = Vec3::new(0.0, 0.0, 1.0);
    let toward = Vec3::new(0.6427876, 0.0, 0.76604444);
    let (_, angle) = extra_turn(rest, toward).to_axis_angle();
    assert!(close(angle, 0.5235988), "turned {angle}");
    let held = Quat::from_axis_angle(Vec3::X, 0.5235988);
    let (_, left) = decay_turn(held).to_axis_angle();
    assert!(close(left, 0.4712389), "decayed to {left}");
}

/// A human step of 40 cm, crouch 0, stance 0.5, lifts −11.5185 cm. A free
/// foot's 10 cm lift is 8 cm the next tick.
#[test]
fn the_step_lifts_and_a_free_foot_keeps_eight_tenths() {
    assert!(close(step_lift(40.0, 40.0, 0.0, 0.5), -11.518518));
    assert!(close(next_lift(10.0, false, false, 0.0), 8.0));
}

/// A floor, and a 15 cm step under the right foot only (x ≥ 0), wide in z
/// so a bending ankle stays on it. The left ankle (x = −0.1, probe 5 cm)
/// does not reach it.
fn step_under_the_right_foot() -> Vec<[Vec3; 3]> {
    let mut tris = floor_quad(0.0, 4.0);
    let y = 0.15;
    let at = |x: f32, z: f32| Vec3::new(x, y, z);
    tris.push([at(0.0, -1.0), at(0.0, 1.0), at(0.4, 1.0)]);
    tris.push([at(0.0, -1.0), at(0.4, 1.0), at(0.4, -1.0)]);
    tris
}

fn rotation(t: Affine3A) -> Quat {
    Quat::from_mat3a(&t.matrix3)
}

/// The shin's rotation relative to its thigh in the pose the muscles sample.
fn shin_turn(body: &Ragdoll, leg: usize) -> Quat {
    let shin = body.legs[leg].shin;
    let thigh = body.parts[shin].parent.unwrap();
    let targets = body.drive.as_ref().unwrap().targets();
    rotation(targets[thigh]).inverse() * rotation(targets[shin])
}

/// The right leg: its contact is the one at positive x.
fn right_leg(body: &Ragdoll) -> usize {
    body.legs
        .iter()
        .enumerate()
        .find(|(_, l)| body.parts[l.shin].point(&l.contact).x > 0.0)
        .map(|(i, _)| i)
        .unwrap()
}

/// Stored correction and rate are 0, and the step asks for more than 0.02
/// rad. The pose the muscles follow on that tick is unchanged. The next
/// aim's shin turn is 0.02 rad about the leg's x axis.
#[test]
fn the_muscles_follow_the_bend_on_the_next_tick() {
    let level = step_under_the_right_foot();
    let (mut body, pose) = standing_at(Vec3::ZERO);
    let root = Affine3A::IDENTITY;
    let aimed: Vec<Affine3A> = pose.iter().map(|p| root * *p).collect();
    body.set_bearing(Bearing::default());
    body.measure_balance(level.as_slice());
    let mut chain = aimed.clone();
    body.fold_leg_ik(&mut chain);
    body.aim(root, chain);
    let leg = right_leg(&body);
    let followed = shin_turn(&body, leg);
    let pose_now = body.drive.as_ref().unwrap().targets().to_vec();
    body.tick(level.as_slice());
    let pose_after = body.drive.as_ref().unwrap().targets();
    for (before, after) in pose_now.iter().zip(pose_after) {
        assert!(
            (before.translation - after.translation).length() < 1e-5,
            "the computing tick moved the pose"
        );
        let drift = rotation(*before).inverse() * rotation(*after);
        assert!(
            drift.to_axis_angle().1 < 1e-4,
            "the computing tick turned the pose"
        );
    }
    let knee = body.legs[leg].knee;
    let rate = body.legs[leg].knee_rate;
    assert!(close(knee, 0.02), "knee correction {knee}");
    assert!(close(rate, 0.01), "knee rate {rate}");
    let mut chain = aimed;
    body.fold_leg_ik(&mut chain);
    body.aim(root, chain);
    let next = shin_turn(&body, leg);
    let extra = followed.inverse() * next;
    let (axis, angle) = extra.to_axis_angle();
    assert!(close(angle, 0.02), "next pose turned {angle}");
    assert!(axis.x > 0.99, "axis {axis}, not the leg's x");
}

/// Lying down (off its feet, not getting up) eases a stored knee of 0.2
/// and a hip of −0.2 toward 0 by 0.02, and leaves the bend target, the
/// step lift and the thigh's extra turn. A get-up still writes a bend
/// target.
#[test]
fn lying_down_releases_the_stored_bend() {
    let level = step_under_the_right_foot();
    let root = Affine3A::IDENTITY;
    let prepare = |bearing: Bearing| {
        let (mut body, pose) = standing_at(Vec3::ZERO);
        let aimed: Vec<Affine3A> = pose.iter().map(|p| root * *p).collect();
        body.set_bearing(bearing);
        body.measure_balance(level.as_slice());
        let mut chain = aimed;
        body.fold_leg_ik(&mut chain);
        body.aim(root, chain);
        let leg = right_leg(&body);
        body.legs[leg].knee = 0.2;
        body.legs[leg].knee_rate = 0.0;
        body.legs[leg].hip = -0.2;
        body.legs[leg].hip_rate = 0.0;
        let kept = (
            body.legs[leg].goal,
            body.legs[leg].lift,
            body.legs[leg].turn,
        );
        (body, leg, kept)
    };
    let lying = Bearing {
        off_feet: true,
        ..Bearing::default()
    };
    let (mut body, leg, kept) = prepare(lying);
    body.bend_legs();
    assert!(
        close(body.legs[leg].knee, 0.18),
        "knee {}",
        body.legs[leg].knee
    );
    assert!(close(body.legs[leg].knee_rate, -0.01));
    assert!(
        close(body.legs[leg].hip, -0.18),
        "hip {}",
        body.legs[leg].hip
    );
    assert!(close(body.legs[leg].hip_rate, 0.01));
    assert_eq!(body.legs[leg].goal, kept.0);
    assert_eq!(body.legs[leg].lift, kept.1);
    assert_eq!(body.legs[leg].turn, kept.2);

    // A get-up still folds, so it writes a bend target. The release leaves
    // the one it had.
    let rising = Bearing {
        off_feet: true,
        getting_up: true,
        ..Bearing::default()
    };
    let (mut body, leg, kept) = prepare(rising);
    body.bend_legs();
    assert_ne!(body.legs[leg].goal, kept.0);
}

/// Lying down, the original eases every stored bend correction toward zero
/// by the standing 0.02 rad a tick, the foot's pitch among them: 0.2 at
/// rest eases to 0.18.
#[test]
fn lying_down_releases_the_stored_foot_pitch() {
    let level = step_under_the_right_foot();
    let root = Affine3A::IDENTITY;
    let (mut body, pose) = standing_at(Vec3::ZERO);
    let aimed: Vec<Affine3A> = pose.iter().map(|p| root * *p).collect();
    body.set_bearing(Bearing {
        off_feet: true,
        ..Bearing::default()
    });
    body.measure_balance(level.as_slice());
    body.aim(root, aimed);
    let leg = right_leg(&body);
    body.legs[leg].pitch = 0.2;
    body.legs[leg].pitch_rate = 0.0;
    body.bend_legs();
    assert!(
        close(body.legs[leg].pitch, 0.18),
        "pitch {}",
        body.legs[leg].pitch
    );
}

/// A shin pointing straight up pitches the foot by the engine's arc cosine
/// of 0.5. A level shin with the contact on its ground pitches none, and
/// a correction under 0.1 cm is none. Half scale with the contact 10 cm
/// high uses the shorter toe. A NaN scale pitches none.
#[test]
fn the_foot_pitch_levels_a_20_cm_toe() {
    let up = foot_pitch_target(0.0, 1.0, 0.0, 0.0, 1.0);
    let cos = 0.5_f32;
    let expect = -crate::engine_math::acos_approx(cos).copysign(-20.0);
    assert_eq!(up.to_bits(), expect.to_bits());
    assert_eq!(foot_pitch_target(1.0, 0.0, 0.0, 0.0, 1.0), 0.0);
    assert_eq!(foot_pitch_target(1.0, 0.0, 0.002, 0.0, 1.0), 0.0);
    let high = foot_pitch_target(1.0, 0.0, 0.0, 10.0, 0.5);
    let half = -crate::engine_math::acos_approx(0.5).copysign(-10.0);
    assert_eq!(high.to_bits(), half.to_bits());
    assert_eq!(foot_pitch_target(1.0, 0.0, 0.0, 0.0, f32::NAN), 0.0);
    let (angle, rate) = ease(0.0, 0.0, up, PITCH_STEP);
    assert_eq!(angle.to_bits(), 0.01_f32.to_bits());
    assert_eq!(rate.to_bits(), (0.01_f32 * 0.5).to_bits());
}

/// A standing body with one foot's anchor on a 15 cm step meets that step
/// with the knee bent, through the shipped character tick and solver.
#[test]
fn a_foot_on_a_15_cm_step_bends_its_knee() {
    let level = step_under_the_right_foot();
    let (mut body, pose) = standing_at(Vec3::ZERO);
    let root = Affine3A::IDENTITY;
    let aimed: Vec<Affine3A> = pose.iter().map(|p| root * *p).collect();
    let bearing = Bearing::default();
    for _ in 0..250 {
        body.set_bearing(bearing);
        body.measure_balance(level.as_slice());
        let mut chain = aimed.clone();
        body.fold_leg_ik(&mut chain);
        body.aim(root, chain);
        body.tick(level.as_slice());
    }
    let leg = right_leg(&body);
    let contact = body.parts[body.legs[leg].shin].point(&body.legs[leg].contact);
    let knee = body.legs[leg].knee;
    assert!(
        (contact.y - 0.15).abs() < 0.04,
        "contact at {}, not on the step",
        contact.y
    );
    assert!(knee > 0.4, "knee correction {knee}");
}

/// The locomotion IK's first tick from a planted straight leg: the clip's
/// ankle 30 cm under its hip (a 30 cm leg where 1 m is expected) asks the
/// knee to bend −2.533 rad; the hip's angle between the clip's leg and the
/// target is 0, the target triangle's hip angle 0 and the clip's 1.266, so
/// the hip takes +1.266. The smoother runs at 0.01 rad a tick here.
#[test]
fn the_locomotion_ik_bends_a_short_clip_leg_over_the_contact() {
    use super::super::floor_test::standing_at;
    use super::super::locomotion::LegMode;
    let (mut body, _) = standing_at(Vec3::ZERO);
    body.set_bearing(Bearing {
        moving: true,
        legs: LegMode::Walk,
        speed: 0.0,
        ..Bearing::default()
    });
    let tick = IkTick {
        step: EASE_STANDING,
        moving_step: 0.01,
        drop: 0.0,
        lift_cm: 0.0,
        stepping: false,
        snap: false,
        running: false,
        stance: 0.5,
        crouch: 0.0,
        frame: Affine3A::IDENTITY,
        sole: 0.0,
    };
    let leg = right_leg(&body);
    {
        let l = &mut body.legs[leg];
        l.anim_contact = Vec3::new(0.1, 0.7, 0.0);
        l.anim_hip = Vec3::new(0.1, 1.0, 0.0);
        l.state = super::super::feet::FootState(super::super::feet::FootState::TOUCHING);
    }
    let thigh = body.parts[body.legs[leg].shin].parent.unwrap();
    assert!((body.parts[thigh].transform().translation.y - 1.0).abs() < 1e-5);
    locomoting_leg(&body.parts, &mut body.legs[leg], &tick);
    let knee = body.legs[leg].knee;
    let hip = body.legs[leg].hip;
    // The first tick moves 0.01 rad toward each correction (−2.533 and
    // +1.266).
    assert!((knee + 0.01).abs() < 1e-4, "knee {knee}");
    assert!((hip - 0.01).abs() < 1e-4, "hip {hip}");
    assert!(body.legs[leg].knee_shown && body.legs[leg].hip_shown);
    // A clip leg as long as the physical one asks for no correction and
    // the smoother takes it back.
    body.legs[leg].anim_contact = Vec3::new(0.1, 0.0, 0.0);
    let tick = IkTick {
        moving_step: 20.0,
        ..tick
    };
    locomoting_leg(&body.parts, &mut body.legs[leg], &tick);
    // The stored rates are negative and positive; one tick cannot reverse
    // them, so each angle moves half its correction back.
    assert!((body.legs[leg].knee + 0.005).abs() < 1e-5);
    assert!((body.legs[leg].hip - 0.005).abs() < 1e-5);
}

/// The sampler's IK step: each leg's clip hip-to-ankle offset is recorded
/// from the chain before the bend, and the stored thigh turn, hip and knee
/// are folded into the thigh and shin, the shin keeping its clip offset
/// under its turned parent.
#[test]
fn the_sampler_folds_the_leg_bend_into_the_chain() {
    let part = |bone: usize, parent: Option<usize>, foot: Option<Vec3>| super::super::PartSetup {
        bone,
        entity: bevy::prelude::Entity::PLACEHOLDER,
        parent,
        world: Affine3A::IDENTITY,
        centre: Vec3::ZERO,
        half: Vec3::splat(0.05),
        origin: Vec3::ZERO,
        links: vec![],
        foot,
        pins: Vec::new(),
        spheres: None,
    };
    let shin = Vec3::new(0.0, -0.4, 0.0);
    let mut body = Ragdoll::new(&[
        part(1, None, None),
        part(3, Some(0), Some(shin)),
        part(4, Some(1), None),
    ])
    .with_drive();
    assert_eq!(body.legs()[0].foot, Some(2));
    {
        let leg = &mut body.legs[0];
        leg.knee = 0.2;
        leg.hip = -0.1;
        leg.turn = Quat::from_rotation_y(0.15);
    }
    let thigh = Affine3A::IDENTITY;
    let below = Affine3A::from_translation(shin);
    let foot = Affine3A::from_translation(shin * 2.0);
    let mut chain = [thigh, below, foot];
    body.fold_leg_ik(&mut chain);
    let leg = &body.legs[0];
    // The leg's contact is its foot part's origin, here the shin's own
    // origin: the clip's ankle 0.4 under the hip, kept in the chain's frame
    // for the next aim.
    let (hip, ankle) = leg.sampled_clip.expect("recorded");
    assert!(hip.abs_diff_eq(Vec3::ZERO, 1e-6));
    assert!(
        ankle.abs_diff_eq(Vec3::new(0.0, -0.4, 0.0), 1e-6),
        "{ankle:?}"
    );
    // The thigh takes the extra turn and the hip hinge; the shin keeps its
    // clip offset under the turned thigh and takes the knee hinge.
    let want_thigh = Affine3A::from_quat(Quat::from_rotation_y(0.15) * Quat::from_rotation_x(-0.1));
    assert!(chain[0].abs_diff_eq(want_thigh, 1e-6), "{:?}", chain[0]);
    let local = want_thigh.inverse() * chain[1];
    let (scale, turn, at) = local.to_scale_rotation_translation();
    assert!(at.abs_diff_eq(shin, 1e-6));
    assert!(turn.abs_diff_eq(Quat::from_rotation_x(0.2), 1e-6), "{turn}");
    assert!(scale.abs_diff_eq(Vec3::ONE, 1e-6));
    // The foot keeps its clip placement under the bent shin.
    let foot_local = chain[1].inverse() * chain[2];
    assert!(
        foot_local.abs_diff_eq(Affine3A::from_translation(shin), 1e-6),
        "{foot_local:?}"
    );
}

/// The aim after the sampler's IK step puts its clip record in the world by
/// the body root: a root 3 m along x and turned a quarter about the
/// vertical moves the hip and ankle with it.
#[test]
fn the_aim_puts_the_sampled_record_in_the_world() {
    let (mut body, pose) = standing_at(Vec3::ZERO);
    let root = Affine3A::from_rotation_translation(
        Quat::from_rotation_y(std::f32::consts::FRAC_PI_2),
        Vec3::new(3.0, 0.0, 0.0),
    );
    let mut chain = pose.clone();
    body.fold_leg_ik(&mut chain);
    let leg = right_leg(&body);
    let (hip, ankle) = body.legs[leg].sampled_clip.expect("recorded");
    body.set_bearing(Bearing::default());
    body.aim(root, chain.iter().map(|p| root * *p).collect());
    let leg = &body.legs[leg];
    assert!(leg.sampled_clip.is_none());
    assert!(leg.anim_hip.abs_diff_eq(root.transform_point3(hip), 1e-5));
    assert!(
        leg.anim_contact
            .abs_diff_eq(root.transform_point3(ankle), 1e-5)
    );
    assert!(
        leg.posed_offset
            .abs_diff_eq(root.transform_vector3(ankle - hip), 1e-5)
    );
}

/// A body without muscles keeps its chain: there is no bend to fold.
#[test]
fn a_limp_body_keeps_the_chain() {
    let setup = super::super::PartSetup {
        bone: 1,
        entity: bevy::prelude::Entity::PLACEHOLDER,
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
    let mut body = Ragdoll::new(&[setup]);
    let mut chain = [Affine3A::from_translation(Vec3::Y * 0.5)];
    body.fold_leg_ik(&mut chain);
    assert_eq!(chain[0], Affine3A::from_translation(Vec3::Y * 0.5));
}
