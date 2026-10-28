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

/// An anchored target is the midpoint of the contact and its anchor, at
/// the contact's height. An unanchored one, standing at the target height,
/// is the contact lowered by twice 1.125 cm: the original works from the
/// contact the foot is at, not from the anchor.
#[test]
fn the_bend_target_drops_an_unanchored_foot() {
    let foot = Vec3::new(0.0, 0.4, 0.0);
    let anchor = Vec3::new(0.2, 0.5, 0.0);
    let mid = bend_target(foot, anchor, true, 0.05);
    assert!(close((mid.x - foot.x).abs(), 0.1));
    assert!(close((mid.z - foot.z).abs(), 0.0));
    assert!(close(mid.y, foot.y));
    let raw = super::super::lift::lift_unclamped(0.0, &Bearing::default(), 1.0);
    assert!(close(raw, 1.125));
    let dropped = bend_target(foot, anchor, false, 2.0 * raw / super::super::CM);
    assert!(close(dropped.y, foot.y - 0.0225));
    assert!(close(dropped.x, foot.x) && close(dropped.z, foot.z));
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
    body.aim(root, aimed.clone());
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
    body.aim(root, aimed);
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
        body.aim(root, aimed);
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
        body.aim(root, aimed.clone());
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
