//! Recovery steps, pinned to the original's centimetres.
//! (Included from body_steps.rs.)
use bevy::math::Affine3A;
use bevy::prelude::Vec3;

use super::super::floor_test::standing_at;
use super::super::tests::floor_quad;
use super::super::{FootState, Ragdoll};
use super::*;

fn close(a: f32, b: f32) -> bool {
    (a - b).abs() < 1e-3
}

/// Bonus 0 is the published 22.5 cm product. A human's race bonus is 1, so
/// the same product is 45 cm; 1.25 while already stepping, and 65 cm of
/// base after being down.
#[test]
fn the_trigger_is_one_and_a_half_radii() {
    assert!(close(
        trigger_distance(1.0, 30.0, 1.0, false, 0.0, 1.0),
        22.5
    ));
    assert!(close(
        trigger_distance(1.0, 30.0, 1.0, true, 0.0, 1.0),
        28.125
    ));
    assert!(close(
        trigger_distance(1.0, 65.0, 1.0, false, 0.0, 1.0),
        48.75
    ));
    assert!(close(
        trigger_distance(1.0, 30.0, 1.0, false, 1.0, 1.0),
        45.0
    ));
    // E = 0.5: E⁴ = 0.0625, R = 0.625 · 30 · 0.5 = 9.375, 1.5 · 9.375 · 0.0625.
    assert!(close(
        trigger_distance(0.5, 30.0, 1.0, false, 0.0, 1.0),
        0.87890625
    ));
    assert!(!starts(0.19, true, 100.0));
    assert!(starts(0.2, true, 22.6));
    assert!(!starts(1.0, false, 100.0));
    let mut going = Steps {
        stepping: true,
        placed: true,
        ..Steps::default()
    };
    let mut up = trigger(1.0, true, 0.0);
    up.getting_up = true;
    assert!(matches!(consider(&mut going, &up), Plan::Ended));
    assert!(!going.stepping);
    let mut going = Steps {
        stepping: true,
        placed: true,
        ..Steps::default()
    };
    let mut walk = trigger(1.0, true, 0.0);
    walk.locomoting = true;
    assert!(matches!(consider(&mut going, &walk), Plan::Ended));
    assert!(!going.stepping);
    assert!(close(step_length(1.0, 1.0), 80.0));
}

/// A knockback step keeps the target the blow set, including while the body
/// is walking. Balance under 0.2 ends it.
#[test]
fn a_knockback_step_keeps_the_blow_target() {
    let mut steps = Steps::default();
    begin_knockback(&mut steps, Vec3::new(0.2, 1.0, 0.0));
    let mut walk = trigger(1.0, true, 0.0);
    walk.locomoting = true;
    match consider(&mut steps, &walk) {
        Plan::Stepping(origin) => assert!(close(origin.x, 0.2), "{origin}"),
        other => panic!("expected the knockback step, got {other:?}"),
    }
    assert!(steps.knockback);
    let mut down = trigger(0.1, true, 0.0);
    down.locomoting = true;
    assert!(matches!(consider(&mut steps, &down), Plan::Ended));
    assert!(!steps.knockback);
}

fn trigger(balance: f32, supported: bool, h_cm: f32) -> Trigger {
    Trigger {
        balance,
        supported,
        getting_up: false,
        locomoting: false,
        down: false,
        h_cm,
        chest: Vec3::X * (h_cm / super::CM),
        fast: Vec3::ZERO,
        energy: 1.0,
        support: 1.0,
        scale: 1.0,
        bonus: 0.0,
        extension: 1.0,
        frame: Vec3::ZERO,
        landings: 0,
    }
}

/// The shipped trigger: balance under 0.2, no support, or a chest inside
/// the reach does not start a step.
fn starts(balance: f32, supported: bool, h_cm: f32) -> bool {
    let mut steps = Steps {
        placed: true,
        radius_base: 30.0,
        ..Steps::default()
    };
    matches!(
        consider(&mut steps, &trigger(balance, supported, h_cm)),
        Plan::Stepping(_)
    )
}

/// Slow chest velocity leads by 15 ticks; faster than 2 cm a tick, by 30 cm
/// along it. k = (2 − S)(2 − E⁴S) is 1 for an alert supported body.
#[test]
fn the_target_leads_the_chest() {
    let slow = lead(Vec3::X * 0.01, 1.0, 1.0, 1.0);
    assert!(close(cm(slow).x, 15.0), "{}", cm(slow).x);
    let fast = lead(Vec3::X * 0.05, 1.0, 1.0, 1.0);
    assert!(close(cm(fast).x, 30.0), "{}", cm(fast).x);
    assert_eq!(next_base(30.0, false), 30.0);
    assert_eq!(next_base(30.0, true), 65.0);
    assert!(close(next_base(65.0, false), 64.5));
}

/// The score's reference leads the fast velocity by 10 ticks.
#[test]
fn the_reference_leads_by_ten_ticks() {
    let at = score_reference(Vec3::ZERO, None, false, Vec3::Z * 0.01, Vec3::Z, 0.0);
    assert!(close(cm(at).z, 10.0), "{}", cm(at).z);
}

/// No penalty inside 0.6 of the leg; past it, ((|g| − 0.65 · step) · 0.1)².
#[test]
fn the_reach_penalty_starts_past_the_leg() {
    assert_eq!(reach_penalty(50.0, 100.0, 80.0), 0.0);
    assert!(close(reach_penalty(70.0, 100.0, 80.0), 3.24));
}

/// Feet 60 cm apart, the chest on the line: the natural order scores about
/// 2.524 and a crossed order about −4.744.
#[test]
fn a_crossed_stance_scores_lower() {
    let side = Vec3::X;
    let natural = stance_score(Vec3::ZERO, Vec3::X * 60.0, Vec3::ZERO, side, 0.0, 1.0);
    let crossed = stance_score(Vec3::X * 60.0, Vec3::ZERO, Vec3::ZERO, side, 0.0, 1.0);
    assert!(close(natural, 2.524), "{natural}");
    assert!(close(crossed, -4.744), "{crossed}");
}

/// A foot under 5 cm scores −10 and the body stays; 6 cm off, that foot steps.
#[test]
fn a_foot_under_5_cm_stays() {
    let feet = [foot(Vec3::ZERO), foot(Vec3::X * 60.0)];
    let ctx = ctx();
    assert_eq!(pick(&feet, &[Vec3::ZERO, Vec3::X * 60.0], &ctx), None);
    let spots = [Vec3::X * 6.0, Vec3::X * 60.0];
    assert_eq!(pick(&feet, &spots, &ctx), Some(0));
}

fn foot(at: Vec3) -> FootView {
    FootView {
        anchored: false,
        stalled: false,
        anchor: at,
        point: at,
        leg: 100.0,
        changed_ms: f32::NAN,
        landed_ms: f32::NAN,
        started_ms: f32::NAN,
    }
}

fn ctx() -> ScoreCtx {
    ScoreCtx {
        now_ms: 0.0,
        scale: 1.0,
        crouch: 0.0,
        reference: Vec3::X * 30.0,
        side: Vec3::X,
        root: Vec3::X * 30.0,
        origin: Vec3::X * 30.0,
        step: 80.0,
        episode: false,
        size: 1.0,
        still_at: f32::NAN,
        still_plus: f32::NAN,
    }
}

/// Leaving the walk stamps T and T+100. An anchored foot's base is then
/// 11.5 under T+200, 3.5 under T+300 and 2 under T+700. A boundary is the
/// next band. NaN stamps, and a clock at T+700, leave the base to the
/// foot's own landing.
#[test]
fn a_stand_still_window_sets_the_anchored_base() {
    let t = 1_000.0;
    let plus = t + 100.0;
    assert_eq!(still_base(plus, t, t), Some(11.5));
    assert_eq!(still_base(plus, t, t + 100.0), Some(11.5));
    assert_eq!(still_base(plus, t, t + 200.0), Some(3.5));
    assert_eq!(still_base(plus, t, t + 250.0), Some(3.5));
    assert_eq!(still_base(plus, t, t + 300.0), Some(2.0));
    assert_eq!(still_base(plus, t, t + 400.0), Some(2.0));
    assert_eq!(still_base(plus, t, t + 700.0), None);
    assert_eq!(still_base(f32::NAN, t, t), None);
    assert_eq!(still_base(plus, f32::NAN, t), None);

    let mut feet = [foot(Vec3::ZERO), foot(Vec3::X * 60.0)];
    feet[0].anchored = true;
    feet[0].landed_ms = t + 250.0;
    let spots = [Vec3::ZERO, Vec3::X * 60.0];
    let mut c = ctx();
    c.now_ms = t + 250.0;
    c.still_at = t;
    c.still_plus = plus;
    let windowed = foot_score(0, &feet, &spots, &c);
    c.still_at = f32::NAN;
    c.still_plus = f32::NAN;
    let per_leg = foot_score(0, &feet, &spots, &c);
    assert!(close(per_leg - windowed, 8.0), "{per_leg} {windowed}");
}

/// The episode ends on its four rules and not while it is young and closing.
#[test]
fn the_episode_ends_on_its_rules() {
    assert!(episode_over(101.0, false, 50.0, 1.0, 0, 0, 1.0));
    assert!(episode_over(0.0, true, 1.0, 1.0, 33, 0, 1.0));
    assert!(episode_over(201.0, true, 21.0, 0.04, 0, 0, 1.0));
    assert!(episode_over(0.0, true, 50.0, 1.0, 0, 2, 1.0));
    assert!(!episode_over(50.0, true, 50.0, 1.0, 0, 0, 1.0));
    assert_eq!(note_near(0, 20.0, 1.0), 1);
    assert_eq!(note_near(4, 21.0, 1.0), 0);
}

fn still_lead(idle: bool, side: Vec3) -> ChestLead {
    ChestLead {
        idle,
        level: 1.0,
        scale: 1.0,
        fast: Vec3::ZERO,
        side,
        kind: LEAD_KIND,
    }
}

/// A 40 cm error with the pin on its posed target, a still chest, scale 1
/// and stance level 1. Sideways: 33.6 cm, or 19.2 cm while Idle. The same
/// error straight ahead: 684.768 cm. One centimetre a tick along the
/// sideways error: 23.1 cm. A pin already 10 cm along that error leaves
/// 30 cm, which moves the target 25.2 cm.
#[test]
fn the_thorax_leads_by_the_gain() {
    let posed = Vec3::Y;
    let standing = Vec3::ZERO;
    let side = Vec3::X;
    let target = Vec3::X * 0.4;
    let led = lead_pose(posed, target, standing, posed, &still_lead(false, side));
    assert!(close(led.x, 0.336), "{}", led.x);
    assert!(close(led.y, 1.0));
    let idle = lead_pose(posed, target, standing, posed, &still_lead(true, side));
    assert!(close(idle.x, 0.192), "{}", idle.x);
    let ahead = lead_pose(
        posed,
        Vec3::Z * 0.4,
        standing,
        posed,
        &still_lead(false, side),
    );
    assert!(close(ahead.z, 6.84768), "{}", ahead.z);
    assert!(close(ahead.y, 1.0));
    let mut closing = still_lead(false, side);
    closing.fast = Vec3::X * 0.01;
    let moved = lead_pose(posed, target, standing, posed, &closing);
    assert!(close(moved.x, 0.231), "{}", moved.x);
    let off = lead_pose(
        posed,
        target,
        standing,
        posed + Vec3::X * 0.1,
        &still_lead(false, side),
    );
    assert!(close(off.x, 0.252), "{}", off.x);
}

fn stand(body: &mut Ragdoll, pose: &[Affine3A], level: &[[Vec3; 3]]) {
    body.measure_balance(level);
    body.aim(Affine3A::IDENTITY, pose.to_vec());
    body.tick(level);
}

fn shift_chest(body: &mut Ragdoll, by: Vec3) {
    let i = body.parts.iter().position(|p| p.bone == 8).unwrap();
    for p in &mut body.parts[i].positions {
        *p += by;
    }
}

/// Horizontal chest-to-support offset (cm), the distance the trigger reads.
fn support_offset(body: &Ragdoll) -> f32 {
    let balance = &body.drive.as_ref().unwrap().balance;
    cm(balance.offset() * balance.lean().with_y(0.0).length())
}

/// An alert human (bonus 1, support 1, settled radius base) does not step
/// at a 44 cm offset and does at 46, and then exactly one foot is stepping.
#[test]
fn a_chest_past_the_trigger_takes_one_step() {
    let floor = floor_quad(0.0, 8.0);
    let (mut body, pose) = standing_at(Vec3::ZERO);
    for _ in 0..40 {
        stand(&mut body, &pose, &floor);
    }
    {
        let drive = body.drive.as_mut().unwrap();
        assert!(!drive.steps.stepping);
        assert!(
            close(drive.steps.radius_base, 30.0),
            "{}",
            drive.steps.radius_base
        );
    }
    let mut under = None;
    let mut over = None;
    for _ in 0..80 {
        {
            let drive = body.drive.as_mut().unwrap();
            drive.support = 1.0;
            drive.energy = 1.0;
        }
        shift_chest(&mut body, Vec3::X * 0.01);
        body.measure_balance(floor.as_slice());
        let h = support_offset(&body);
        let drive = body.drive.as_ref().unwrap();
        if h < 45.0 {
            assert!(!drive.steps.stepping, "stepped at {h} cm");
            if h >= 44.0 {
                under = Some(h);
            }
        }
        if h >= 46.0 {
            over = Some(h);
            assert!(drive.steps.stepping, "no step at {h} cm");
            break;
        }
    }
    assert!(
        under.is_some(),
        "the chest never passed 44 cm under the trigger"
    );
    assert!(over.is_some(), "the chest never reached 46 cm");
    let stepping = body
        .legs
        .iter()
        .filter(|l| l.state.has(FootState::ANCHORED))
        .count();
    assert_eq!(stepping, 1, "{stepping} feet stepped");
}

/// The real pin pass leads the thorax while a step runs and the chest is
/// not out of balance, and does not while it is.
#[test]
fn the_pin_pass_leads_the_thorax() {
    let (_, pose) = standing_at(Vec3::ZERO);
    let mut still = standing_at(Vec3::ZERO).0;
    let mut led = standing_at(Vec3::ZERO).0;
    let mut out = standing_at(Vec3::ZERO).0;
    for body in [&mut still, &mut led, &mut out] {
        body.aim(Affine3A::IDENTITY, pose.clone());
    }
    let arm = |body: &mut Ragdoll, out_of_balance: bool| {
        let drive = body.drive.as_mut().unwrap();
        drive.steps.stepping = true;
        drive.steps.out_of_balance = out_of_balance;
        drive.steps.placed = true;
        drive.steps.standing = Vec3::ZERO;
        drive.steps.target = Vec3::X * 0.4;
        drive.bearing.idle = false;
    };
    arm(&mut led, false);
    arm(&mut out, true);
    let before = |body: &Ragdoll| body.chest_point().unwrap().x;
    let (still0, led0, out0) = (before(&still), before(&led), before(&out));
    still.pull_support_pins();
    led.pull_support_pins();
    out.pull_support_pins();
    let dx = |body: &Ragdoll, from: f32| body.chest_point().unwrap().x - from;
    let (still_dx, led_dx, out_dx) = (dx(&still, still0), dx(&led, led0), dx(&out, out0));
    assert!(led_dx > still_dx + 0.0005, "led {led_dx}, still {still_dx}");
    assert!(
        (out_dx - still_dx).abs() < 0.0005,
        "out of balance moved {out_dx}, still {still_dx}"
    );
}

/// Crouched deeper than half, the footwork's steps are 0.7 of the step
/// length; at half or less they are whole.
#[test]
fn a_deep_crouch_shortens_the_steps() {
    assert_eq!(crouch_stride(0.0), 1.0);
    assert_eq!(crouch_stride(0.5), 1.0);
    assert!(close(crouch_stride(0.51), 0.7));
    assert!(close(crouch_stride(1.0) * step_length(1.0, 1.0), 56.0));
}
