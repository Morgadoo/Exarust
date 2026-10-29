//! A synthetic lever (not game data): an axle of two pins 15 cm either side
//! of its origin along x, an arm point 40 cm out, and an arm link whose
//! slack allows a half swing of 0.6 rad either side of its tied point,
//! which lies 40 cm along z. At its file position the arm sits on the
//! first detent.
use super::*;
use crate::body::{RigidLink, RigidPart};

const NONE: &[[Vec3; 3]] = &[];
const ALPHA: f32 = 0.6;
const RADIUS: f32 = 0.4;
const TARGET: u32 = 0x8000_0071;

fn link(on_body: Vec3, on_parent: Vec3, slack: f32) -> RigidLink {
    RigidLink {
        on_body,
        on_parent,
        slack,
        strength: 1.0,
        smoothing: 0.2,
        flags: 0,
        axis: Vec3::ZERO,
    }
}

/// The arm's rigid body at `at`, turned so its arm point (0, 0.4, 0) lies
/// at angle −α from the tied point about the axle.
fn arm(at: Vec3) -> RigidPart {
    let turn = Quat::from_rotation_x(std::f32::consts::FRAC_PI_2 - ALPHA);
    let mut spheres = Vec::new();
    for j in 0..8 {
        spheres.push((Vec3::new(0.0, 0.05 * j as f32, 0.0), 0.03));
    }
    let slack = 2.0 * RADIUS * (ALPHA * 0.5).sin();
    RigidPart {
        entity: Entity::PLACEHOLDER,
        parent: None,
        world: Affine3A::from_rotation_translation(turn, at),
        rest: [
            Vec3::new(0.1, -0.1, 0.0),
            Vec3::new(-0.1, -0.1, 0.0),
            Vec3::new(0.0, 0.5, -0.2),
            Vec3::new(0.0, 0.5, 0.2),
        ],
        point_masses: [4.0; 4],
        parent_world: Affine3A::IDENTITY,
        mobility: 0.16,
        spheres,
        origin: at,
        links: vec![
            link(Vec3::new(0.15, 0.0, 0.0), Vec3::new(0.15, 0.0, 0.0), 0.0),
            link(
                Vec3::new(0.0, RADIUS, 0.0),
                Vec3::new(0.0, 0.0, RADIUS),
                slack,
            ),
            link(Vec3::new(-0.15, 0.0, 0.0), Vec3::new(-0.15, 0.0, 0.0), 0.0),
        ],
        mesh: None,
    }
}

fn record(time: u16) -> Record {
    Record {
        kind: 1,
        flags: 0,
        target: TARGET,
        second: 0,
        time,
    }
}

/// A lever and its glue at `at`, set up as registration does.
fn lever_at(at: Vec3, time: u16, saved: i32) -> (Ragdoll, LeverBody) {
    let mut body = Ragdoll::rigid(&[arm(at)]);
    let turn = Quat::from_rotation_x(std::f32::consts::FRAC_PI_2 - ALPHA);
    let pending = PendingLever {
        record: record(time),
        saved,
        id: 0x8000_004a,
    };
    let glue =
        LeverBody::new(&body, &pending, Affine3A::IDENTITY, turn, at, 0).expect("a lever arm");
    body.parts[0].inverse_mass = LEVER_MOBILITY;
    body.parts[0].pairing.mobility = LEVER_MOBILITY;
    body.enter_mechanism_environment();
    (body, glue)
}

/// A tick of the solver and then of the lever, as the game runs them; the
/// signals the tick sent.
fn tick(body: &mut Ragdoll, glue: &mut LeverBody, n: i32) -> Vec<(u32, i32)> {
    let now = n * 12;
    if glue.count_down(now) {
        body.wake_to(WAKE_COUNT);
    }
    body.tick(NONE);
    if body.asleep() {
        return Vec::new();
    }
    let out = glue.tick(body, now);
    glue.signals(&out)
}

/// The lever reads its swing from its links and its angle from its arm,
/// and settles on the detent it sits at.
#[test]
fn a_lever_reads_its_swing_and_settles_on_its_detent() {
    let (body, glue) = lever_at(Vec3::new(2.0, 1.0, -3.0), 0, 0);
    let lever = glue.lever;
    // The engine's fast arcsine is a little off the exact half swing.
    let alpha = lever::half_swing(2.0 * 40.0 * (ALPHA * 0.5).sin(), 40.0);
    assert!((alpha - ALPHA).abs() < 2e-3, "{alpha}");
    assert!((lever.detent0 + alpha).abs() < 1e-4, "{}", lever.detent0);
    assert!((lever.span - 2.0 * alpha).abs() < 1e-5);
    assert!((lever.angle + ALPHA).abs() < 1e-3, "{}", lever.angle);
    assert_eq!(lever.detent_index(), 0);
    assert_eq!(lever.last, lever.detent0);
    assert!(lever.has_target && !lever.has_second);
    assert!(body.in_mechanism_environment());
    assert!(body.parts[0].gravity.iter().all(|&g| g == 0.0));
    // The drive's point at the arm's angle is the arm point.
    let point = glue.arm_point(&body);
    assert!(glue.arm_point_at(lever.angle).distance(point) < 1e-4);
    assert!((glue.angle_of(point) - lever.angle).abs() < 1e-5);
}

/// A lever at rest does not creep and goes to sleep; it sends nothing.
#[test]
fn a_lever_at_rest_stays_and_sleeps() {
    let (mut body, mut glue) = lever_at(Vec3::new(0.0, 1.0, 0.0), 0, 0);
    body.wake_to(WAKE_COUNT);
    let start = glue.arm_point(&body);
    for n in 0..800 {
        assert!(tick(&mut body, &mut glue, n).is_empty(), "tick {n}");
    }
    let moved = glue.arm_point(&body).distance(start);
    assert!(moved < 2e-3, "{moved} m");
    assert!(body.asleep());
}

/// The wheel turns the held lever toward the cursor 0.02 rad a tick; on the
/// second detent it signals 1 to its target once, and stays there after the
/// release.
#[test]
fn the_wheel_turns_a_lever_onto_its_other_detent() {
    let at = Vec3::new(0.0, 1.0, 0.0);
    let (mut body, mut glue) = lever_at(at, 0, 0);
    let (across, _) = glue.grip_normals();
    assert!(across.abs_diff_eq(Vec3::X, 1e-5), "{across}");
    let point = glue.arm_point(&body);
    let grabbed = glue.take_hold(&mut body, point);
    assert!(grabbed.abs() < 1e-4, "{grabbed}");
    assert!(glue.lever.held && !body.asleep());
    // The cursor past the far end of the swing: the hand stops at it.
    let far = at + Vec3::new(0.0, -(1.0f32).sin(), (1.0f32).cos()) * 0.5;
    let mut sent = Vec::new();
    for n in 0..200 {
        glue.turn_to(&mut body, far, far, true, grabbed);
        sent.extend(tick(&mut body, &mut glue, n));
    }
    let second = glue.lever.detent0 + glue.lever.span;
    assert!(
        (glue.lever.hand - second).abs() < 1e-5,
        "{}",
        glue.lever.hand
    );
    assert!(
        (glue.lever.angle - second).abs() < 0.03,
        "{} {second}",
        glue.lever.angle
    );
    assert_eq!(sent, vec![(TARGET, 1)]);
    glue.let_go(&mut body);
    assert!(!glue.lever.held);
    for n in 200..500 {
        assert!(tick(&mut body, &mut glue, n).is_empty());
    }
    assert!((glue.lever.angle - second).abs() < 0.03);
}

/// A lever with a return time goes back to its first detent once the time
/// has run, and signals 0 then.
#[test]
fn a_timed_lever_returns_and_signals_off() {
    let at = Vec3::new(0.0, 1.0, 0.0);
    // 5 × 100 ms.
    let (mut body, mut glue) = lever_at(at, 5, 0);
    let point = glue.arm_point(&body);
    let grabbed = glue.take_hold(&mut body, point);
    let far = at + Vec3::new(0.0, -(1.0f32).sin(), (1.0f32).cos()) * 0.5;
    let mut sent = Vec::new();
    let mut n = 0;
    while sent.is_empty() && n < 200 {
        glue.turn_to(&mut body, far, far, true, grabbed);
        sent.extend(tick(&mut body, &mut glue, n));
        n += 1;
    }
    assert_eq!(sent, vec![(TARGET, 1)]);
    assert!(glue.armed);
    glue.let_go(&mut body);
    sent.clear();
    for k in n..n + 300 {
        sent.extend(tick(&mut body, &mut glue, k));
    }
    assert_eq!(sent, vec![(TARGET, 0)]);
    assert!(!glue.armed && !glue.lever.returning);
    assert!((glue.lever.angle - glue.lever.detent0).abs() < 0.03);
}

/// The second grip takes the heading of the cursor's sideways offset at
/// 30 cm along forward; the clamp keeps the hand on the swing.
#[test]
fn the_wheel_clamps_its_angle_to_the_swing() {
    assert_eq!(swing_clamp(-0.5, -0.9, 1.0), -0.5);
    assert_eq!(swing_clamp(-0.5, 0.9, 1.0), 0.5);
    assert!((swing_clamp(-0.5, 0.1, 1.0) - 0.1).abs() < 1e-6);
    assert!(swing_clamp(-0.5, f32::NAN, 1.0).is_nan());
    // The gap is not wrapped before the clamp: −6 is no turn. The float π
    // is over the double π, so it wraps to −π.
    assert_eq!(swing_clamp(3.0, -3.0, 1.0), 3.0);
    assert_eq!(
        swing_clamp(3.0, std::f32::consts::PI, 1.0),
        std::f32::consts::PI - std::f32::consts::TAU
    );
    let at = Vec3::ZERO;
    let (mut body, mut glue) = lever_at(at, 0, 0);
    // 30 cm sideways along the frame's side row (−y here) at any height:
    // 45 degrees, clamped to the second detent's 0.6.
    let side = at + Vec3::new(0.3, -0.3, 0.7);
    glue.turn_to(&mut body, side, side, false, 0.0);
    let second = glue.lever.detent0 + glue.lever.span;
    assert!(
        (glue.lever.hand - second).abs() < 1e-5,
        "{}",
        glue.lever.hand
    );
    let little = at + Vec3::new(0.0, -0.05, 0.7);
    glue.turn_to(&mut body, little, little, false, 0.0);
    let want = (5.0f32).atan2(30.0);
    assert!((glue.lever.hand - want).abs() < 2e-3, "{}", glue.lever.hand);
}

/// A return armed in the saved word (bits 16 and up, in units of 100 ms)
/// is armed again at set-up.
#[test]
fn a_saved_return_is_armed_again() {
    let (_, glue) = lever_at(Vec3::ZERO, 5, 3 << 16);
    assert!(glue.armed);
    assert_eq!(glue.lever.deadline_ms, 300);
    let (_, plain) = lever_at(Vec3::ZERO, 0, 3 << 16);
    assert!(!plain.armed);
}
