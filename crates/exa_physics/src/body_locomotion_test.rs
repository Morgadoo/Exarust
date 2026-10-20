//! The movement decision, the body turn and the alignment: the original's
//! rules, each pinned to its numbers (`docs/formats/RSQ.md`, Physical body,
//! Movement decision).
//! (Included from body_locomotion.rs: `#[path = "body_locomotion_test.rs"] mod tests;`)
use std::f32::consts::{FRAC_PI_2, PI};

use super::super::PartSetup;
use super::*;

/// Degrees a second of a turn of `rate` rad a tick.
fn per_second(rate: f32) -> f32 {
    rate.to_degrees() * 83.0
}

/// A body standing at the origin facing +Z, its request moving.
fn walker(free_walk: bool, goal: Vec3) -> (Locomotion, Locomotor) {
    let (mut req, body) = Locomotion::facing(0.0);
    req.moving = true;
    req.free_walk = free_walk;
    req.goal = goal;
    req.flags = flags::MOVE;
    (req, body)
}

/// The base rate is 240°/s: 4π/3 over 83 ticks, 0.0504674 rad a tick.
#[test]
fn the_base_turn_is_240_degrees_a_second() {
    assert!((BASE_TURN - 0.050_467_35).abs() < 1e-7, "{BASE_TURN}");
    assert!((per_second(BASE_TURN) - 240.0).abs() < 1e-3);
}

/// The table: free walk (1 − 0.1·speed)·B, 240°/s standing still and
/// 201.6°/s at 1.6 cm a tick (no floor: none at 10 cm a tick); the slow
/// turn 120°/s; otherwise 300°/s, 210°/s at encumbrance 0.5, and ÷√scale
/// for a larger body (scale 1.21: 272.7°/s before the step's own ÷1.1);
/// attacking, its law's rate as it is; crouched beyond 0.5, ×0.6.
#[test]
fn the_body_turns_by_the_originals_table() {
    let by = |f: &dyn Fn(&mut TurnBy)| {
        let mut b = TurnBy::BRISK;
        f(&mut b);
        b
    };
    let free = by(&|b| b.free_walk = true);
    assert!((per_second(turn_rate(&free, 0.0)) - 240.0).abs() < 1e-3);
    assert!((per_second(turn_rate(&free, 1.6)) - 201.6).abs() < 1e-3);
    assert!(turn_rate(&free, 10.0).abs() < 1e-9);
    let slow = by(&|b| b.slow = true);
    assert!((per_second(turn_rate(&slow, 3.0)) - 120.0).abs() < 1e-3);
    assert!((per_second(turn_rate(&TurnBy::BRISK, 3.0)) - 300.0).abs() < 1e-3);
    let worn = by(&|b| b.encumbrance = 0.5);
    assert!((per_second(turn_rate(&worn, 0.0)) - 210.0).abs() < 1e-3);
    let big = by(&|b| b.scale = 1.21);
    assert!((per_second(turn_rate(&big, 0.0)) - 300.0 / 1.1).abs() < 1e-3);
    let attacking = by(&|b| b.attack = Some(0.04));
    assert_eq!(turn_rate(&attacking, 0.0), 0.04);
    // Free walk and the slow turn come before the attack law.
    let walking_attack = by(&|b| {
        b.attack = Some(0.04);
        b.free_walk = true;
    });
    assert_eq!(turn_rate(&walking_attack, 0.0), BASE_TURN);
    let crouched = by(&|b| b.crouch = 0.6);
    assert!((per_second(turn_rate(&crouched, 0.0)) - 180.0).abs() < 1e-3);
    let half = by(&|b| b.crouch = 0.5);
    assert!((per_second(turn_rate(&half, 0.0)) - 300.0).abs() < 1e-3);
}

/// A step: the full rate beyond 0.6 rad, ×|err|/0.6 within, ×0.9 moving by
/// the keys, ÷√scale; a turn ends on the heading only within 1e-4 rad (the
/// eased step never covers what is left), the yaw kept within (−π, π].
#[test]
fn a_turn_steps_eases_and_ends_on_the_heading() {
    let rate = turn_rate(&TurnBy::BRISK, 0.0);
    let mut body = Locomotor::default();
    turn(&mut body, FRAC_PI_2, rate, 1.0, false);
    assert!((body.yaw - rate).abs() < 1e-7 && body.turning);
    assert!((body.error - FRAC_PI_2).abs() < 1e-6);
    assert!((per_second(body.turn_rate) - 300.0).abs() < 1e-3);
    // Moving by the keys: ×0.9 (270°/s); a body of scale 1.21: ÷1.1.
    let mut keys = Locomotor::default();
    turn(&mut keys, -FRAC_PI_2, rate, 1.0, true);
    assert!((per_second(keys.turn_rate) - 270.0).abs() < 1e-3);
    assert!((keys.yaw + keys.turn_rate).abs() < 1e-7);
    let mut big = Locomotor::default();
    turn(&mut big, FRAC_PI_2, rate, 1.21, false);
    assert!((per_second(big.turn_rate) - 300.0 / 1.1).abs() < 1e-3);
    // 0.3 rad left: half the rate.
    let mut near = Locomotor::default();
    turn(&mut near, 0.3, rate, 1.0, false);
    assert!((near.turn_rate - rate * 0.5).abs() < 1e-6);
    // The eased turn shrinks the error by 1 − rate/0.6 a tick: from 0.3 rad
    // it is still turning after 70 ticks, and ends within 1e-4 rad.
    let mut ticks = 1;
    while near.turning {
        turn(&mut near, 0.3, rate, 1.0, false);
        ticks += 1;
    }
    assert!((71..80).contains(&ticks), "{ticks}");
    assert_eq!(near.yaw, 0.3);
    // Across ±π the shortest way (0.683 rad), and the yaw wraps.
    let mut back = Locomotor {
        yaw: 3.1,
        ..default()
    };
    turn(&mut back, -2.5, rate, 1.0, false);
    assert!(
        back.error > 0.0 && back.yaw < -3.0 && back.yaw > -PI,
        "{back:?}"
    );
}

/// A standing exploration quarter turn at the slow rate (120°/s): 0.0252
/// rad a tick for its first 38 ticks, past 90 % of the turn after about 70
/// ticks (0.84 s); the same turn at 300°/s is past 90 % after about 28.
#[test]
fn a_standing_quarter_turn_takes_the_slow_rate() {
    let ninety = |rate: f32| {
        let mut body = Locomotor::default();
        let mut ticks = 0;
        while body.yaw < 0.9 * FRAC_PI_2 {
            turn(&mut body, FRAC_PI_2, rate, 1.0, false);
            ticks += 1;
        }
        ticks
    };
    let slow = BASE_TURN * 0.5;
    let mut body = Locomotor::default();
    for _ in 0..38 {
        turn(&mut body, FRAC_PI_2, slow, 1.0, false);
    }
    assert!((body.yaw - 38.0 * slow).abs() < 1e-5, "{}", body.yaw);
    assert_eq!(ninety(slow), 70);
    assert_eq!(ninety(BASE_TURN * 1.25), 28);
}

/// Free walk never brakes: the move vector is the facing × 200 cm, the
/// body moves (amount 1) at any speed and stays in the walk's leg mode.
#[test]
fn free_walk_never_brakes() {
    let (mut req, mut body) = walker(true, Vec3::new(0.0, 0.0, 0.1));
    for speed in [0.0, 0.3, 1.7] {
        decide(&mut req, &mut body, &Footing::steady(Vec3::ZERO, speed));
        assert_eq!(
            (body.stop, body.leg, body.amount),
            (Stop::Moving, LegMode::Walk, 1.0)
        );
        assert!(!req.has(flags::BRAKE) && req.moving);
    }
}

/// Toward a goal a walking body brakes within the walk's 50 cm: 40 cm off
/// at 1.7 cm a tick it brakes and moves on; under 1 cm a tick its amount is
/// 0, yet it moves while faster than 0.5 cm a tick; at 0.4 it stops
/// (braked), the request no longer moving. 60 cm off it does not brake.
#[test]
fn a_walking_body_brakes_within_its_stop_distance() {
    let (mut req, mut body) = walker(false, Vec3::new(0.0, 0.0, 0.6));
    decide(&mut req, &mut body, &Footing::steady(Vec3::ZERO, 1.7));
    assert!(!req.has(flags::BRAKE) && body.moving());
    req.goal = Vec3::new(0.0, 0.0, 0.4);
    decide(&mut req, &mut body, &Footing::steady(Vec3::ZERO, 1.7));
    assert!(req.has(flags::BRAKE) && body.moving());
    decide(&mut req, &mut body, &Footing::steady(Vec3::ZERO, 0.8));
    assert!(body.moving());
    decide(&mut req, &mut body, &Footing::steady(Vec3::ZERO, 0.4));
    assert_eq!(
        (body.stop, body.amount, body.leg),
        (Stop::Braked, 0.0, LegMode::Standing)
    );
    assert!(!req.moving);
}

/// A body that does not brake (running) stops once its move vector is
/// within 25 cm (625 cm²) at no more than 0.5 cm a tick: at 24.9 cm and
/// 0.5 it arrives; at 25.1 cm or 0.51 cm a tick it moves on. Running it
/// never brakes within its stop distance; a brake set before keeps it
/// walking (leg mode walk), braking.
#[test]
fn a_body_arrives_within_25_cm() {
    let run = |goal: f32, speed: f32| {
        let (mut req, mut body) = walker(false, Vec3::new(0.0, 0.0, goal));
        req.flags |= flags::RUN;
        decide(&mut req, &mut body, &Footing::steady(Vec3::ZERO, speed));
        assert!(!req.has(flags::BRAKE));
        body.stop
    };
    assert_eq!(run(0.249, 0.5), Stop::Arrived);
    assert_eq!(run(0.251, 0.5), Stop::Moving);
    assert_eq!(run(0.249, 0.51), Stop::Moving);
    let (mut req, mut body) = walker(false, Vec3::new(0.0, 0.0, 1.0));
    req.flags |= flags::RUN | flags::BRAKE;
    decide(&mut req, &mut body, &Footing::steady(Vec3::ZERO, 3.0));
    assert_eq!(body.leg, LegMode::Walk);
    assert!(req.has(flags::BRAKE));
}

/// The stop distance blends with the run: after a run (blend 0.6, the jog
/// row) a walking body brakes within 0.5 + 1.5·0.6 = 1.4 m; standing again
/// it is the walk's 0.5 m.
#[test]
fn the_stop_distance_follows_the_run_blend() {
    let (mut req, mut body) = walker(false, Vec3::new(0.0, 0.0, 1.3));
    body.stop_distance = 1.4;
    decide(&mut req, &mut body, &Footing::steady(Vec3::ZERO, 2.0));
    assert!(req.has(flags::BRAKE));
    req.moving = false;
    decide(&mut req, &mut body, &Footing::steady(Vec3::ZERO, 0.0));
    assert_eq!(body.stop, Stop::Standing);
    req.moving = true;
    decide(&mut req, &mut body, &Footing::steady(Vec3::ZERO, 0.0));
    assert_eq!(body.stop_distance, 0.5);
}

/// Backwards a body backpedals and brakes within the backpedal's 50 cm
/// (the turn-about bit cleared); the S-brake (brake and turn-about) turns
/// into the backpedal once the body is under 1 cm a tick.
#[test]
fn backwards_and_the_s_brake() {
    let (mut req, mut body) = walker(false, Vec3::new(0.0, 0.0, -0.4));
    req.flags |= flags::BACK | flags::TURN_ABOUT;
    decide(&mut req, &mut body, &Footing::steady(Vec3::ZERO, 0.9));
    assert_eq!(body.leg, LegMode::Backpedal);
    assert!(req.has(flags::BRAKE) && !req.has(flags::TURN_ABOUT));
    let (mut req, mut body) = walker(true, Vec3::ZERO);
    req.flags |= flags::BRAKE | flags::TURN_ABOUT;
    decide(&mut req, &mut body, &Footing::steady(Vec3::ZERO, 1.2));
    assert_eq!(body.leg, LegMode::Walk);
    assert!(req.has(flags::BRAKE) && !req.has(flags::BACK));
    decide(&mut req, &mut body, &Footing::steady(Vec3::ZERO, 0.9));
    assert!(req.has(flags::BACK) && !req.has(flags::BRAKE) && body.moving());
}

/// The gates: under its most energy (0.999 of 1) a body neither starts nor
/// walks on, its flags and speed cleared; standing it starts with balance
/// 0.99 (a float 0.99 is over the double) and no more than 80 ticks without
/// support, not with 0.985 or 81 ticks; walking a float 0.9 is under the
/// double 0.9: it stumbles, the 40-tick cooldown starts and it lands, and
/// 39 ticks later it is still held, 40 ticks later free.
#[test]
fn the_gates_hold_a_tired_or_unsteady_body() {
    let try_at = |moving: bool, f: &dyn Fn(&mut Footing)| {
        let (mut req, mut body) = walker(true, Vec3::ZERO);
        req.moving = moving;
        req.flags |= flags::RUN;
        let mut at = Footing::steady(Vec3::ZERO, 1.0);
        f(&mut at);
        let d = decide(&mut req, &mut body, &at);
        (body.stop, req.flags, d.land, body.cooldown)
    };
    let tired = try_at(true, &|a| a.energy = 0.999);
    assert_eq!(tired, (Stop::Tired, 0, false, 0));
    let stumbled = try_at(true, &|a| a.balance = 0.9);
    assert_eq!(
        (stumbled.0, stumbled.2, stumbled.3),
        (Stop::Unsteady, true, 40)
    );
    assert_eq!(try_at(true, &|a| a.balance = 0.901).0, Stop::Moving);
    assert_eq!(try_at(false, &|a| a.balance = 0.99).0, Stop::Standing);
    assert_eq!(try_at(false, &|a| a.balance = 0.985).0, Stop::Unbalanced);
    assert_eq!(try_at(false, &|a| a.airborne = 80).0, Stop::Standing);
    assert_eq!(try_at(false, &|a| a.airborne = 81).0, Stop::Airborne);
    assert_eq!(try_at(false, &|a| a.rebalance = 21).0, Stop::Rebalancing);
    assert_eq!(try_at(false, &|a| a.rebalance = 20).0, Stop::Standing);

    let (mut req, mut body) = walker(true, Vec3::ZERO);
    let mut at = Footing::steady(Vec3::ZERO, 1.0);
    at.balance = 0.8;
    decide(&mut req, &mut body, &at);
    count_down(&mut body);
    at.balance = 1.0;
    for tick in 1..=40 {
        req.moving = true;
        decide(&mut req, &mut body, &at);
        let held = body.stop == Stop::Cooldown;
        assert_eq!(held, tick < 40, "tick {tick}");
        count_down(&mut body);
    }
}

/// Stepping refuses but lets the balance test run: a walking body stepping
/// with balance 0.8 still gets the cooldown and lands, its reason stepping.
#[test]
fn stepping_refuses_and_still_stumbles() {
    let (mut req, mut body) = walker(true, Vec3::ZERO);
    req.stepping = Some(Vec3::X);
    let d = decide(&mut req, &mut body, &Footing::steady(Vec3::ZERO, 0.0));
    assert_eq!(
        (body.stop, d.land, body.cooldown),
        (Stop::Stepping, false, 0)
    );
    req.moving = true;
    let mut at = Footing::steady(Vec3::ZERO, 0.0);
    at.balance = 0.8;
    let d = decide(&mut req, &mut body, &at);
    assert_eq!(
        (body.stop, d.land, body.cooldown),
        (Stop::Stepping, true, 40)
    );
}

/// The alignment's target: standing 1 − enc (0.75 at 0.25); by the keys
/// 0.4 + 0.6·(heading·pelvis)²: 1 aligned, 0.4 across, 0.7 at 45°; by the
/// mouse 0.4 + 0.6·(wanted·facing), 0.4 when it points behind; the
/// alignment keeps 0.97 a tick: from 1 toward 0.4 it is 0.982 after a tick and 0.6977
/// after 23, and scales the amount.
#[test]
fn the_alignment_follows_the_pelvis() {
    let z = Vec3::Z;
    let x = Vec3::X;
    let diagonal = Vec3::new(1.0, 0.0, 1.0).normalize();
    assert_eq!(align_target(false, true, z, x, z, z, 0.25), 0.75);
    assert!((align_target(true, true, z, z, x, x, 0.0) - 1.0).abs() < 1e-6);
    assert!((align_target(true, true, z, x, z, z, 0.0) - 0.4).abs() < 1e-6);
    assert!((align_target(true, true, z, diagonal, x, x, 0.0) - 0.7).abs() < 1e-6);
    assert!((align_target(true, false, z, x, -z, z, 0.0) - 0.4).abs() < 1e-6);
    assert!(
        (align_target(true, false, x, x, diagonal, z, 0.0) - (0.4 + 0.6 * diagonal.z)).abs() < 1e-6
    );
    let mut body = Locomotor {
        amount: 1.0,
        ..default()
    };
    align(&mut body, 0.4);
    assert!((body.align - 0.982).abs() < 1e-6 && body.amount == body.align);
    for _ in 1..23 {
        align(&mut body, 0.4);
    }
    assert!(
        (body.align - (0.4 + 0.6 * 0.97f32.powi(23))).abs() < 1e-5,
        "{}",
        body.align
    );
}

/// The run blend of a moving body: running it rises 0.05 a tick to 1 in 20
/// ticks, to 0.85 at a cap of 0.85; walking it falls 0.015; braking at 3 cm
/// a tick it is at most 0.36 (f = 0.2), at 2 none. The stop distance:
/// 1.25 m at a half blend, 1.5 with the latch past 3.5 cm a tick (the
/// sprint row), the backpedal's 0.5 backwards. A standing body keeps both.
#[test]
fn the_run_blend_and_the_stop_distance() {
    let stops = StopDistances::HUMAN;
    let (mut req, mut body) = walker(true, Vec3::ZERO);
    body.stop = Stop::Moving;
    body.leg = LegMode::Run;
    for _ in 0..20 {
        locomote(&req, &mut body, 1.0, &stops);
    }
    assert!((body.run - 1.0).abs() < 1e-6, "{}", body.run);
    locomote(&req, &mut body, 0.85, &stops);
    assert_eq!(body.run, 0.85);
    body.leg = LegMode::Walk;
    locomote(&req, &mut body, 1.0, &stops);
    assert!((body.run - 0.835).abs() < 1e-6);
    req.flags |= flags::BRAKE;
    body.speed = 3.0;
    locomote(&req, &mut body, 1.0, &stops);
    assert!((body.run - 0.36).abs() < 1e-6, "{}", body.run);
    body.speed = 2.0;
    locomote(&req, &mut body, 1.0, &stops);
    assert_eq!(body.run, 0.0);
    req.flags = flags::MOVE;
    body.run = 0.515;
    locomote(&req, &mut body, 1.0, &stops);
    assert!(
        (body.stop_distance - 1.25).abs() < 1e-6,
        "{}",
        body.stop_distance
    );
    req.flags |= flags::SPRINT;
    body.run = 0.515;
    body.speed = 3.6;
    locomote(&req, &mut body, 1.0, &stops);
    assert!((body.stop_distance - 1.5).abs() < 1e-6);
    req.flags |= flags::BACK;
    locomote(&req, &mut body, 1.0, &stops);
    assert_eq!(body.stop_distance, 0.5);
    body.stop = Stop::Standing;
    body.run = 0.4;
    locomote(&req, &mut body, 1.0, &stops);
    assert_eq!((body.run, body.stop_distance), (0.4, 0.5));
}

/// The capsule: along the heading at 1.4 m/s walking and 3.6 running,
/// −1.2245 m/s backwards, none standing, the interim footwork's velocity
/// while it steps, and braking in free walk 0.97397 of last tick's a tick
/// (the brake drive's 0.2 × 0.07984 and the limbs' 0.98977).
#[test]
fn the_capsule_executes_the_decision() {
    let gaits = CapsuleGaits::default();
    let o = Vec3::ZERO;
    let (mut req, mut body) = walker(true, Vec3::ZERO);
    req.heading = FRAC_PI_2;
    body.stop = Stop::Moving;
    let v = capsule_velocity(&req, &body, &gaits, Vec3::ZERO, o);
    assert!((v - Vec3::X * 1.4).length() < 1e-6, "{v}");
    body.run = 1.0;
    let v = capsule_velocity(&req, &body, &gaits, Vec3::ZERO, o);
    assert!((v - Vec3::X * 3.6).length() < 1e-5, "{v}");
    req.flags |= flags::BACK;
    let v = capsule_velocity(&req, &body, &gaits, Vec3::ZERO, o);
    assert!((v + Vec3::X * 1.2252).length() < 1e-3, "{v}");
    req.flags |= flags::BRAKE;
    let v = capsule_velocity(&req, &body, &gaits, Vec3::Z, o);
    assert!((v.z - 0.973_97).abs() < 1e-5, "{v}");
    req.stepping = Some(Vec3::new(0.3, 0.0, 0.0));
    let step = capsule_velocity(&req, &body, &gaits, Vec3::Z, o);
    assert_eq!(step, Vec3::new(0.3, 0.0, 0.0));
    req.stepping = None;
    body.stop = Stop::Braked;
    assert_eq!(
        capsule_velocity(&req, &body, &gaits, Vec3::Z, o),
        Vec3::ZERO
    );
}

/// Braking toward a goal the capsule slows at least enough to stop on it:
/// a soft stop's goal 33.52 cm ahead of a walk at 1.687 cm a tick is
/// reached without passing it, under 0.5 cm a tick within 3 cm of it, in
/// 29 ticks (0.35 s); a run at 4.337 cm a tick braking 100 cm before a wall
/// stops before it. On its goal it halves its speed a tick; with its goal
/// behind (a soft stop's after a backpedal) it slows by the drive alone.
#[test]
fn a_braking_capsule_stops_on_its_goal() {
    let gaits = CapsuleGaits::default();
    let brake = |speed: f32, goal: f32| {
        let (mut req, mut body) = walker(false, Vec3::new(0.0, 0.0, goal / 100.0));
        req.flags |= flags::BRAKE;
        body.stop = Stop::Moving;
        let (mut at, mut v, mut ticks) = (0.0f32, Vec3::Z * speed * 0.83, 0);
        while v.length() / 0.83 > 0.5 {
            v = capsule_velocity(&req, &body, &gaits, v, Vec3::Z * at / 100.0);
            at += v.length() / 0.83;
            ticks += 1;
            assert!(at <= goal, "passed its goal: {at} of {goal}");
        }
        (goal - at, ticks)
    };
    let (left, ticks) = brake(1.687, 33.52);
    assert!(left < 3.0 && ticks == 29, "{left} cm, {ticks} ticks");
    let (left, _) = brake(4.337, 100.0);
    assert!((0.0..3.0).contains(&left), "{left}");
    let (mut req, mut body) = walker(false, Vec3::ZERO);
    req.flags |= flags::BRAKE;
    body.stop = Stop::Moving;
    let v = capsule_velocity(&req, &body, &gaits, Vec3::Z * 0.83, Vec3::Z * -0.001);
    assert!((v.z - 0.415).abs() < 1e-5, "{v}");
    let v = capsule_velocity(&req, &body, &gaits, Vec3::Z * 0.83, Vec3::Z * 0.1);
    assert!((v.z - 0.83 * 0.973_97).abs() < 1e-5, "{v}");
}

/// The pelvis's forward is the facing turned as the pelvis is turned from
/// its target: 30° about the vertical.
#[test]
fn the_pelvis_forward_follows_the_pelvis_turn() {
    let turn = Quat::from_rotation_y(30f32.to_radians());
    let body = Ragdoll::new(&[PartSetup {
        bone: 1,
        entity: Entity::PLACEHOLDER,
        parent: None,
        world: Affine3A::from_rotation_translation(turn, Vec3::Y),
        centre: Vec3::ZERO,
        half: Vec3::splat(0.1),
        origin: Vec3::ZERO,
        links: vec![],
        foot: None,
        pins: Vec::new(),
    }])
    .with_drive();
    // No target yet: the facing.
    assert_eq!(body.pelvis_forward(Vec3::Z), Vec3::Z);
    let mut body = body;
    body.drive.as_mut().unwrap().targets = vec![Affine3A::from_translation(Vec3::Y)];
    let f = body.pelvis_forward(Vec3::Z);
    assert!((f - turn * Vec3::Z).length() < 1e-5, "{f}");
}

/// A body's footing: its energy and most from its drive, its balance steady
/// unless read from the body (0.5), with its ticks without support.
#[test]
fn the_footing_reads_the_body() {
    let mut body = Ragdoll::new(&[PartSetup {
        bone: 1,
        entity: Entity::PLACEHOLDER,
        parent: None,
        world: Affine3A::from_translation(Vec3::Y),
        centre: Vec3::ZERO,
        half: Vec3::splat(0.1),
        origin: Vec3::ZERO,
        links: vec![],
        foot: None,
        pins: Vec::new(),
    }])
    .with_drive();
    let d = body.drive.as_mut().unwrap();
    d.energy = 0.7;
    d.balance.value = 0.5;
    let at = Footing::of(Some(&body), Vec3::ZERO, 1.0);
    assert_eq!((at.energy, at.max_energy, at.balance), (0.7, 1.0, 1.0));
    let at = at.balanced_by(&body);
    assert_eq!((at.balance, at.airborne), (0.5, 0));
    assert_eq!(
        Footing::of(None, Vec3::ZERO, 1.0),
        Footing::steady(Vec3::ZERO, 1.0)
    );
}

#[test]
fn yaws_wrap_within_half_a_turn() {
    assert!((wrap(3.0 * PI / 2.0) + FRAC_PI_2).abs() < 1e-6);
    assert!((wrap(-PI) - PI).abs() < 1e-6);
    assert!((turn_between(3.1, -3.1) - (2.0 * PI - 6.2)).abs() < 1e-5);
    assert!((yaw_of(forward(1.0)) - 1.0).abs() < 1e-6);
}
