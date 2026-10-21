//! The locomotion routine and the standing run blend: the original's rules,
//! each pinned to its numbers (`docs/formats/RSQ.md`, Locomotion).
//! (Included from body_gait.rs: `#[path = "body_gait_test.rs"] mod tests;`)
use super::super::{Stop, flags};
use super::*;

fn close(a: f32, b: f32) -> bool {
    (a - b).abs() < 1e-5
}

/// A human walking along +Z (moving, leg mode walk, aligned, amount 1) on
/// level ground with support, its walk playing.
fn walker() -> (Locomotion, Locomotor, Stride) {
    let req = Locomotion {
        flags: flags::MOVE,
        moving: true,
        free_walk: true,
        ..default()
    };
    let body = Locomotor {
        stop: Stop::Moving,
        leg: LegMode::Walk,
        amount: 1.0,
        ..default()
    };
    let at = Stride {
        supported: true,
        level: 1.0,
        encumbrance: 0.0,
        crouch: 0.0,
        tone: 1.0,
        progress: 0.0,
        moved: Vec3::ZERO,
        heading: Vec3::Z,
        walking: true,
        in_combat: false,
        now: 10_000,
    };
    (req, body, at)
}

/// The loader's per-tick rows of a human: thrust 88.9/83 = 1.07108 and the
/// backpedal's −0.93735; push 550/83² = 0.079837; clock 27/83 = 0.32530,
/// 0.2, 0.16265 and the backpedal's −0.33735; the built-in stops 50, 200,
/// 250 and 50 cm. A half-feral character's sprint row is halfway to the
/// jog's (clock 0.18133); a character that is not feral keeps the race's.
#[test]
fn the_gait_table_is_scaled_per_tick() {
    let [walk, jog, sprint, back] = GaitTable::HUMAN_ROWS;
    assert!(close(walk.thrust, 1.071_084_3) && close(back.thrust, -0.937_349_4));
    assert!(close(walk.push, 0.079_837_41) && walk.push == back.push);
    assert!(close(walk.clock, 0.325_301_2) && close(jog.clock, 0.2));
    assert!(close(sprint.clock, 0.162_650_6) && close(back.clock, -0.337_349_4));
    let stops: Vec<f32> = GaitTable::HUMAN_ROWS.iter().map(GaitRow::stop_cm).collect();
    assert_eq!(stops, [50.0, 200.0, 250.0, 50.0]);
    let half = GaitTable::new(GaitTable::HUMAN_ROWS, 0.5, 1.0);
    assert!(close(
        half.sprint.clock,
        0.5 * sprint.clock + 0.5 * jog.clock
    ));
    assert!(close(half.sprint.stop_cm(), 225.0));
    assert_eq!(GaitTable::default().sprint, sprint);
}

/// Running, the run blend rises 0.05 a tick to its cap: 1 in 20 ticks, 0.85
/// at encumbrance 0.5, 0.8 with the sprint latch. With support it first
/// bleeds by √align·level: aligned at 0.81 a blend of 0.5 holds (0.9 × 0.5
/// + 0.05); without support it rises on.
#[test]
fn running_raises_the_run_blend_to_its_cap() {
    let (mut req, mut body, mut at) = walker();
    body.leg = LegMode::Run;
    for _ in 0..20 {
        locomote(&req, &mut body, &GaitTable::default(), &at);
    }
    assert!((body.run - 1.0).abs() < 1e-6, "{}", body.run);
    at.encumbrance = 0.5;
    locomote(&req, &mut body, &GaitTable::default(), &at);
    assert_eq!(body.run, 0.85);
    req.flags |= flags::SPRINT;
    locomote(&req, &mut body, &GaitTable::default(), &at);
    assert_eq!(body.run, 0.8);
    let (req, mut body, mut at) = walker();
    body.leg = LegMode::Run;
    body.align = 0.81;
    body.run = 0.5;
    locomote(&req, &mut body, &GaitTable::default(), &at);
    assert!(close(body.run, 0.5), "{}", body.run);
    at.supported = false;
    locomote(&req, &mut body, &GaitTable::default(), &at);
    assert!(close(body.run, 0.55), "{}", body.run);
}

/// Walking, it falls 0.015 a tick; braking at 3 cm a tick it is at most
/// 0.36 (f = 0.2), at 2 none; 2 cm a tick of vertical travel takes 0.01
/// more; it never goes under 0.
#[test]
fn walking_braking_and_climbing_bleed_the_run_blend() {
    let (mut req, mut body, mut at) = walker();
    body.run = 0.85;
    locomote(&req, &mut body, &GaitTable::default(), &at);
    assert!(close(body.run, 0.835));
    req.flags |= flags::BRAKE;
    body.speed = 3.0;
    locomote(&req, &mut body, &GaitTable::default(), &at);
    assert!(close(body.run, 0.36), "{}", body.run);
    body.speed = 2.0;
    locomote(&req, &mut body, &GaitTable::default(), &at);
    assert_eq!(body.run, 0.0);
    req.flags = flags::MOVE;
    body.run = 0.5;
    at.moved = Vec3::new(0.0, -2.0, 0.0);
    locomote(&req, &mut body, &GaitTable::default(), &at);
    assert!(close(body.run, 0.475), "{}", body.run);
    body.run = 0.005;
    locomote(&req, &mut body, &GaitTable::default(), &at);
    assert_eq!(body.run, 0.0);
}

/// Standing, the run blend falls 0.02 a tick (0.5 → 0.48), never under 0;
/// getting up it is 0 at once. Either way the clock's step is not the
/// clip's.
#[test]
fn a_standing_body_lets_its_run_blend_fall() {
    let mut body = Locomotor {
        run: 0.5,
        locomoted: true,
        ..default()
    };
    stand(&mut body, false);
    assert!(close(body.run, 0.48) && !body.locomoted);
    body.run = 0.01;
    stand(&mut body, false);
    assert_eq!(body.run, 0.0);
    body.run = 0.7;
    stand(&mut body, true);
    assert_eq!(body.run, 0.0);
}

/// Standing or getting up, the exertion value keeps 95% a tick.
#[test]
fn a_standing_body_lets_its_exertion_fall() {
    let mut body = Locomotor {
        exertion: 0.5,
        ..default()
    };
    stand(&mut body, false);
    assert_eq!(body.exertion, (0.5f64 * 0.95) as f32);
    stand(&mut body, true);
    assert_eq!(body.exertion, ((0.5f64 * 0.95) as f32 as f64 * 0.95) as f32);
}

/// The columns at a half run blend: walking the clock by √run (0.23670),
/// running by the blend (0.26265); the stop 125 cm on the jog row, 150 cm
/// on the sprint row (the latch, a level of 1 and 3.6 cm a tick), 125 cm
/// again at a level of 0.9 or 3.5 cm a tick; thrust × the level;
/// backwards the backpedal row as it is.
#[test]
fn the_gait_columns_blend_by_the_run() {
    let (mut req, mut body, mut at) = walker();
    // The bleeds left out: no travel, a walk falling 0.015.
    let half = |body: &mut Locomotor| body.run = 0.515;
    half(&mut body);
    locomote(&req, &mut body, &GaitTable::default(), &at);
    assert!(close(body.run, 0.5));
    assert!(close(body.gait.clock, 0.236_699_88), "{}", body.gait.clock);
    assert!(close(body.gait.stop_cm(), 125.0));
    body.leg = LegMode::Run;
    body.run = 0.45;
    locomote(&req, &mut body, &GaitTable::default(), &at);
    assert!(close(body.run, 0.5));
    assert!(close(body.gait.clock, 0.262_650_6), "{}", body.gait.clock);
    body.leg = LegMode::Walk;
    req.flags |= flags::SPRINT;
    body.speed = 3.6;
    half(&mut body);
    locomote(&req, &mut body, &GaitTable::default(), &at);
    assert!(close(body.gait.stop_cm(), 150.0), "{}", body.gait.stop_cm());
    at.level = 0.9;
    half(&mut body);
    body.speed = 3.6;
    locomote(&req, &mut body, &GaitTable::default(), &at);
    assert!(close(body.gait.stop_cm(), 125.0));
    let [walk, jog, ..] = GaitTable::HUMAN_ROWS;
    assert!(close(
        body.gait.thrust,
        (jog.thrust * 0.5 + walk.thrust * 0.5) * 0.9
    ));
    at.level = 1.0;
    half(&mut body);
    body.speed = 3.5;
    locomote(&req, &mut body, &GaitTable::default(), &at);
    assert!(close(body.gait.stop_cm(), 125.0));
    req.flags = flags::MOVE | flags::BACK;
    locomote(&req, &mut body, &GaitTable::default(), &at);
    assert_eq!(body.gait, GaitTable::HUMAN_ROWS[3]);
}

/// The clock's rate: with the latch × (1 − 0.1·enc) on any forward row
/// (the jog's 0.2 → 0.19 at encumbrance 0.5 below the sprint's level),
/// × the clock factor (1/0.9 for a short character), crouched beyond 0.1
/// × (1 − 0.1·crouch) (0.95 at 0.5; none at 0.09).
#[test]
fn the_clock_rate_takes_the_latch_height_and_crouch() {
    let (mut req, mut body, mut at) = walker();
    body.leg = LegMode::Run;
    body.run = 0.95;
    req.flags |= flags::SPRINT;
    at.level = 0.9;
    at.encumbrance = 0.5;
    locomote(&req, &mut body, &GaitTable::default(), &at);
    assert!((body.run - 0.8).abs() < 1e-6);
    let walk = GaitTable::HUMAN_ROWS[0].clock;
    let jog = 0.2 * 0.8 + walk * 0.2;
    assert!(close(body.gait.clock, jog * 0.95), "{}", body.gait.clock);
    let (req, mut body, mut at) = walker();
    let short = GaitTable::new(GaitTable::HUMAN_ROWS, 0.0, 1.0 / 0.9);
    body.run = 0.015;
    locomote(&req, &mut body, &short, &at);
    assert!(close(body.gait.clock, walk / 0.9));
    at.crouch = 0.5;
    body.run = 0.015;
    locomote(&req, &mut body, &GaitTable::default(), &at);
    assert!(close(body.gait.clock, walk * 0.95));
    at.crouch = 0.09;
    body.run = 0.015;
    locomote(&req, &mut body, &GaitTable::default(), &at);
    assert_eq!(body.gait.clock, walk);
}

/// Another clip playing, the walk starts: more than 600 ms after the last
/// locomotion at key 2 ahead, key 4 backwards, with the heading more than
/// 1 rad on the yaw's negative side key 5 at a run blend over 0.5 and key
/// 8 under it, without a key more than 1 rad on its positive side; within
/// 600 ms it resumes its clock. A start resets the boost ramp; once the
/// walk plays it rises 0.5/83 a tick, × the alignment.
#[test]
fn the_walk_starts_at_its_key_or_resumes() {
    let start = |error: f32, run: f32, back: bool, last: u64| {
        let (mut req, mut body, mut at) = walker();
        if back {
            req.flags |= flags::BACK;
        }
        at.walking = false;
        body.error = error;
        body.run = run;
        body.last_ran = last;
        body.last_clock = 37.5;
        body.boost = 0.7;
        let s = locomote(&req, &mut body, &GaitTable::default(), &at);
        assert_eq!(body.boost, 0.0);
        assert_eq!(body.last_ran, 10_000);
        s
    };
    assert_eq!(start(0.0, 0.0, false, 0), Some(Start::Key(2)));
    assert_eq!(start(0.0, 0.0, true, 0), Some(Start::Key(4)));
    assert_eq!(start(-1.5, 0.6, false, 0), Some(Start::Key(5)));
    assert_eq!(start(-1.5, 0.51, false, 0), Some(Start::Key(8)));
    assert_eq!(start(1.5, 0.0, false, 0), Some(Start::Unkeyed));
    assert_eq!(start(1.0, 0.0, false, 0), Some(Start::Key(2)));
    assert_eq!(start(0.0, 0.0, false, 9_399), Some(Start::Key(2)));
    assert_eq!(start(0.0, 0.0, false, 9_400), Some(Start::Resume(37.5)));
    let (req, mut body, at) = walker();
    body.align = 0.5;
    assert_eq!(locomote(&req, &mut body, &GaitTable::default(), &at), None);
    assert!(close(body.boost, 0.006_024_096 * 0.5));
}

/// The drive of a human walking aligned at full tone: the heading ×
/// 1.07108 × the phase push (0.8 at the cycle's start, 1.1 a quarter in),
/// less 0.4 × its displacement (1.687 cm a tick along +Z: 0.18207);
/// misaligned the push grows by 2 − align² (×1.36 at 0.8); under the
/// row's boost speed it is boosted by the ramp; braking it is −0.2 × the
/// displacement alone.
#[test]
fn the_drive_pushes_along_the_heading() {
    let (mut req, mut body, mut at) = walker();
    locomote(&req, &mut body, &GaitTable::default(), &at);
    assert!(close(body.drive.z, 0.856_867_4), "{}", body.drive);
    assert_eq!((body.drive.x, body.drive.y), (0.0, 0.0));
    at.moved = Vec3::Z * 1.687;
    locomote(&req, &mut body, &GaitTable::default(), &at);
    assert!(close(body.drive.z, 0.182_067_4), "{}", body.drive);
    at.moved = Vec3::ZERO;
    at.progress = 0.25;
    locomote(&req, &mut body, &GaitTable::default(), &at);
    assert!(close(body.drive.z, 1.178_192_7), "{}", body.drive);
    at.progress = 0.0;
    body.align = 0.8;
    body.amount = 0.8;
    locomote(&req, &mut body, &GaitTable::default(), &at);
    let thrust = GaitTable::HUMAN_ROWS[0].thrust;
    assert!(
        close(body.drive.z, 0.8 * thrust * 0.8 * 1.36),
        "{}",
        body.drive
    );
    // A row with a boost speed of 4: standing still the push gains 4 ×
    // the ramp.
    let mut rows = GaitTable::HUMAN_ROWS;
    rows[0].boost = 4.0;
    let boosted = GaitTable::new(rows, 0.0, 1.0);
    let (req2, mut body, at) = walker();
    body.boost = 0.5 - 0.006_024_096;
    locomote(&req2, &mut body, &boosted, &at);
    assert!(close(body.boost, 0.5));
    assert!(
        close(body.drive.z, thrust * (0.8 + 4.0 * 0.5)),
        "{}",
        body.drive
    );
    req.flags |= flags::BRAKE;
    let (_, mut body, mut at) = walker();
    at.moved = Vec3::new(0.5, 0.0, 2.0);
    locomote(&req, &mut body, &GaitTable::default(), &at);
    assert!((body.drive - Vec3::new(-0.1, 0.0, -0.4)).length() < 1e-6);
}

/// The speed is the displacement along the heading and its direction:
/// (1, 0.5, 2) along +Z is 2 cm a tick going +Z; backwards along +Z,
/// 2 going −Z; none when it stands.
#[test]
fn the_speed_is_the_displacement_along_the_heading() {
    let (req, mut body, mut at) = walker();
    at.moved = Vec3::new(1.0, 0.5, 2.0);
    locomote(&req, &mut body, &GaitTable::default(), &at);
    assert_eq!((body.speed, body.going), (2.0, Vec3::Z));
    at.moved = Vec3::new(0.3, 0.0, -2.0);
    locomote(&req, &mut body, &GaitTable::default(), &at);
    assert_eq!((body.speed, body.going), (2.0, -Vec3::Z));
    at.moved = Vec3::X;
    locomote(&req, &mut body, &GaitTable::default(), &at);
    assert_eq!(body.speed, 0.0);
}

/// The clock's step with support: aligned at 1.687 cm a tick 0.32530 ×
/// 2.3496 = 0.76433; at an alignment of 0.8, × 0.976 at the cycle's start
/// and × 1.12 a quarter in; stopping at 3 cm a tick 0.32530 × 3; stopping
/// at 6 cm a tick paced as ever. The phase is then unset.
#[test]
fn the_clock_steps_with_the_speed_and_alignment() {
    let (mut req, mut body, mut at) = walker();
    let step = |req: &Locomotion, body: &mut Locomotor, at: &Stride, speed: f32| {
        at_speed(req, body, at, speed);
        body.clock_step
    };
    assert!(close(step(&req, &mut body, &at, 1.687), 0.764_327_7));
    assert_eq!((body.locomoted, body.phase), (true, PHASE_UNSET));
    body.align = 0.8;
    let paced = 0.764_327_7;
    assert!(close(step(&req, &mut body, &at, 1.687), paced * 0.976));
    at.progress = 0.25;
    assert!(close(step(&req, &mut body, &at, 1.687), paced * 1.12));
    body.align = 1.0;
    req.flags |= flags::STOPPING;
    let walk = GaitTable::HUMAN_ROWS[0].clock;
    assert!(close(step(&req, &mut body, &at, 3.0), walk * 3.0));
    assert!(close(
        step(&req, &mut body, &at, 6.0),
        walk * (0.8 * 6.0 + 1.0)
    ));
}

/// Moves `body` `speed` cm a tick along its heading for one tick of
/// locomotion (the run blend kept at 0).
fn at_speed(req: &Locomotion, body: &mut Locomotor, at: &Stride, speed: f32) {
    let at = Stride {
        moved: Vec3::Z * speed,
        ..*at
    };
    body.run = 0.0;
    locomote(req, body, &GaitTable::default(), &at);
}

/// Without support the clock keeps the phase it had when the body lost
/// it: at progress 0.3 the phase is 0.85 (0.85 − 0.5·round(0.3)), and the
/// step (2 − u)·u·(0.8·speed + 0.9)·rate, u = frac(1.85 − 0.3) = 0.55:
/// 0.58361 at 1.687 cm a tick; at progress 0.7 the phase is 0.35. Back on
/// its feet the phase is unset.
#[test]
fn without_support_the_clock_keeps_its_phase() {
    let (req, mut body, mut at) = walker();
    at.supported = false;
    at.progress = 0.3;
    at_speed(&req, &mut body, &at, 1.687);
    assert!(close(body.phase, 0.85), "{}", body.phase);
    assert!(close(body.clock_step, 0.583_608_6), "{}", body.clock_step);
    at.progress = 0.7;
    at_speed(&req, &mut body, &at, 1.687);
    assert!(close(body.phase, 0.85));
    body.phase = PHASE_UNSET;
    at_speed(&req, &mut body, &at, 1.687);
    assert!(close(body.phase, 0.35), "{}", body.phase);
    at.supported = true;
    at_speed(&req, &mut body, &at, 1.687);
    assert_eq!(body.phase, PHASE_UNSET);
}

/// The clip clock after a tick: 0.3 − 0.8 with the last key at 80 wraps
/// to 79.5, which a resumed walk starts from; the original's fraction
/// keeps the sign (−0.25 of −0.25, 0.75 of 2.75).
#[test]
fn the_clock_wraps_once_and_is_kept() {
    let mut body = Locomotor {
        clock_step: -0.8,
        ..default()
    };
    assert_eq!(advance_clock(&mut body, 0.3, 80.0), 79.5);
    assert_eq!(body.last_clock, 79.5);
    body.clock_step = 0.75;
    assert_eq!(advance_clock(&mut body, 10.0, 80.0), 10.75);
    assert_eq!(fraction(-0.25), -0.25);
    assert_eq!(fraction(2.75), 0.75);
    assert!(fraction(f32::NAN).is_nan());
}

/// The exertion value falls ×0.98 while the run blend is capped and ×0.99
/// otherwise (double precision).
#[test]
fn exertion_bleeds() {
    let (req, mut body, at) = walker();
    body.leg = LegMode::Run;
    body.exertion = 0.5;
    body.run = 1.0;
    locomote(&req, &mut body, &GaitTable::default(), &at);
    assert_eq!(body.exertion, 0.49);
    body.leg = LegMode::Walk;
    locomote(&req, &mut body, &GaitTable::default(), &at);
    assert_eq!(body.exertion, 0.49 * 0.99);
}

/// With support the head tilt is the exertion's sign × square × 2, and it
/// keeps 95% of that each tick; airborne it is not set and the locomotion
/// routine ramps it by 0.05 a tick to at most 1.
#[test]
fn head_tilt_persists_and_ramps_in_the_air() {
    let (req, mut body, mut at) = walker();
    body.exertion = 0.5;
    lean(&mut body, true);
    assert_eq!(body.head_tilt, ((0.5f32 * 0.5 * 2.0) as f64 * 0.95) as f32);
    // Standing still without support, the tilt keeps falling.
    let before = body.head_tilt;
    lean(&mut body, false);
    assert_eq!(body.head_tilt, (f64::from(before) * 0.95) as f32);
    // Airborne locomotion ramps it, capped at 1.
    at.supported = false;
    body.head_tilt = 0.5;
    locomote(&req, &mut body, &GaitTable::default(), &at);
    assert_eq!(body.head_tilt, (0.5f64 + 0.05) as f32);
    body.head_tilt = 0.99;
    locomote(&req, &mut body, &GaitTable::default(), &at);
    assert_eq!(body.head_tilt, 1.0);
    // A negative exertion tilts the other way.
    body.exertion = -0.5;
    lean(&mut body, true);
    assert!(body.head_tilt < 0.0);
}

/// The grip mode the routine sets: the general mode running (and walking
/// in combat), the walk's mode otherwise.
#[test]
fn locomotion_sets_the_grip_mode() {
    let (mut req, mut body, mut at) = walker();
    body.leg = LegMode::Run;
    locomote(&req, &mut body, &GaitTable::default(), &at);
    assert_eq!(body.grip_mode, GRIP_MODE);
    body.leg = LegMode::Walk;
    locomote(&req, &mut body, &GaitTable::default(), &at);
    assert_eq!(body.grip_mode, GRIP_MODE_WALKING);
    at.in_combat = true;
    locomote(&req, &mut body, &GaitTable::default(), &at);
    assert_eq!(body.grip_mode, GRIP_MODE);
    req.flags |= flags::BACK;
    at.in_combat = false;
    locomote(&req, &mut body, &GaitTable::default(), &at);
    assert_eq!(body.grip_mode, GRIP_MODE_WALKING);
}

/// Another clip playing clears the boost ramp, notifies the start (the
/// original calls its callback and clears the held items' motion flag) and
/// starts the walk, while a playing walk does neither.
#[test]
fn a_walk_start_notifies_once_another_clip_plays() {
    let (req, mut body, at) = walker();
    body.start_callback = true;
    let mut walking = at;
    walking.walking = true;
    assert!(locomote(&req, &mut body, &GaitTable::default(), &walking).is_none());
    assert!(!body.start_notice);
    let mut other = at;
    other.walking = false;
    assert!(locomote(&req, &mut body, &GaitTable::default(), &other).is_some());
    assert!(body.start_notice);
    body.start_callback = false;
    locomote(&req, &mut body, &GaitTable::default(), &other);
    assert!(!body.start_notice);
}

/// The jump branch is dead (nothing writes its flag or timer), but its
/// rule holds: over a timer of 0.5 it decays the clock step kept from the
/// last tick by 0.013 a tick, floored at 0, and skips the normal clock.
#[test]
fn a_jump_only_decays_the_clock_step() {
    let (req, mut body, at) = walker();
    body.clock_step = 1.0;
    body.jump = true;
    body.jump_progress = 1.0;
    locomote(&req, &mut body, &GaitTable::default(), &at);
    assert!(
        (body.clock_step - 0.987).abs() < 1e-6,
        "{}",
        body.clock_step
    );
    body.clock_step = 0.005;
    locomote(&req, &mut body, &GaitTable::default(), &at);
    assert_eq!(body.clock_step, 0.0);
    body.clock_step = 0.0;
    body.jump_progress = 0.5;
    locomote(&req, &mut body, &GaitTable::default(), &at);
    assert!(body.clock_step > 0.0);
}
