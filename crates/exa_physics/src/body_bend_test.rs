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
    let anchor = Vec3::new(0.2, 0.4, 0.0);
    let mid = bend_target(foot, anchor, true, 0.05);
    assert!(close((mid - foot).length(), 0.1));
    assert!(close((mid - anchor).length(), 0.1));
    assert!(close(mid.y, anchor.y));
    let raw = super::super::lift::lift_unclamped(0.0, &Bearing::default());
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
