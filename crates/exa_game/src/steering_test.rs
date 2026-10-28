//! The player's exploration behaviour: the original's rules, each pinned to
//! its numbers (`docs/PLAYER.md`, Movement).
//! (Included from steering.rs: `#[path = "steering_test.rs"] mod tests;`)
use std::f32::consts::{FRAC_PI_2, PI};

use exa_physics::body::locomotion::Locomotion;

use super::*;

/// A line walk with no obstacle.
fn open(_: Vec3, to: Vec3) -> (Vec3, bool) {
    (to, false)
}

/// The keys `w`, `a`, `s`, `d`, Shift and the mouse as a string of those
/// letters ("ws" holds W and S, "m" the mouse, "S" Shift).
fn keys(held: &str) -> Keys {
    Keys {
        w: held.contains('w'),
        a: held.contains('a'),
        s: held.contains('s'),
        d: held.contains('d'),
        shift: held.contains('S'),
        mouse: held.contains('m'),
    }
}

/// A player standing at the origin facing +Z.
fn standing() -> (Steering, Locomotion, Locomotor) {
    let (req, body) = Locomotion::facing(0.0);
    (Steering::default(), req, body)
}

/// W asks to move by the keys; S standing (0.5 cm a tick) backpedals, S
/// faster than 1 cm a tick brakes and turns about; S with the mouse is the
/// mouse's (not heading-driven); down or dead nothing changes.
#[test]
fn the_request_follows_the_keys_and_the_mouse() {
    let (mut s, mut req, mut body) = standing();
    s.request(&mut req, &body, &keys("w"), None, Condition::NORMAL);
    assert!(s.requested && req.has(flags::KEYS) && !req.has(flags::BACK));
    body.speed = 0.5;
    s.request(&mut req, &body, &keys("s"), None, Condition::NORMAL);
    assert!(req.has(flags::BACK) && !req.has(flags::BRAKE));
    let (mut s, mut req, mut body) = standing();
    body.speed = 1.5;
    s.request(&mut req, &body, &keys("s"), None, Condition::NORMAL);
    assert!(req.has(flags::BRAKE | flags::TURN_ABOUT) && !req.has(flags::BACK));
    s.request(&mut req, &body, &keys("sm"), None, Condition::NORMAL);
    assert!(!req.has(flags::KEYS));
    // W again clears backwards.
    s.request(&mut req, &body, &keys("w"), None, Condition::NORMAL);
    assert!(!req.has(flags::TURN_ABOUT));
    let (mut s, mut req, body) = standing();
    s.request(&mut req, &body, &keys("w"), None, Condition::DOWN);
    assert!(!s.requested && req.flags == 0);
}

/// Released while the body walks, the request holds (heading-driven) until
/// the walk's key segment is 0 or 4, which also clears backwards; released
/// standing it ends at once.
#[test]
fn a_released_walk_finishes_its_stride() {
    let (mut s, mut req, body) = standing();
    s.request(&mut req, &body, &keys("w"), Some(2), Condition::NORMAL);
    req.moving = true;
    req.flags |= flags::BACK;
    for segment in [2, 3] {
        s.request(&mut req, &body, &keys(""), Some(segment), Condition::NORMAL);
        assert!(s.requested && req.has(flags::KEYS), "{segment}");
    }
    s.request(&mut req, &body, &keys(""), Some(4), Condition::NORMAL);
    assert!(!s.requested && !req.has(flags::BACK));
    s.requested = true;
    s.request(&mut req, &body, &keys(""), Some(0), Condition::NORMAL);
    assert!(!s.requested);
    s.requested = true;
    req.moving = false;
    s.request(&mut req, &body, &keys(""), Some(2), Condition::NORMAL);
    assert!(!s.requested);
}

/// The mouse's request walks to a cursor within 3 m (the walk chevron) and
/// runs beyond (the run chevron); the keys' is neither.
#[test]
fn the_cursor_mode_runs_beyond_3_m() {
    let mode = |held: &str, distance: f32| {
        let (mut s, mut req, body) = standing();
        s.raw = Some((0.0, distance));
        s.request(&mut req, &body, &keys(held), None, Condition::NORMAL);
        s.mode
    };
    assert_eq!(mode("m", 2.9), CursorMode::Walk);
    assert_eq!(mode("m", 3.1), CursorMode::Run);
    assert_eq!(mode("w", 3.1), CursorMode::Other);
    assert_eq!(mode("", 3.1), CursorMode::Other);
}

/// The follower: toward a cursor a quarter turn away its first tick turns
/// 0.0172718 rad ((π/2 × 0.015708) × 0.7); half a turn away the error is
/// clamped to π/2 and the first tick is the same; it settles with no
/// overshoot, within 5 % after 83 ticks (its slow pole keeps 0.959 a
/// tick). A cursor closer than 50 cm gives no direction.
#[test]
fn the_cursor_follower_settles_in_about_a_second() {
    let mut f = CursorFollower::at(0.0);
    f.follow(FRAC_PI_2);
    f.tick();
    assert!((f.yaw() - 0.017_271_8).abs() < 1e-6, "{}", f.yaw());
    let mut half = CursorFollower::at(0.0);
    half.follow(PI - 0.001);
    half.tick();
    assert!((half.yaw() - 0.017_271_8).abs() < 1e-6);
    let mut peak = 0.0f32;
    for _ in 1..83 {
        f.tick();
        peak = peak.max(f.yaw());
    }
    assert!(
        peak <= FRAC_PI_2 && FRAC_PI_2 - f.yaw() < 0.05 * FRAC_PI_2,
        "{}",
        f.yaw()
    );
    let (mut s, mut req, _) = standing();
    s.aim(&mut req, Some(Vec3::new(0.3, 0.0, 0.3)), false);
    assert_eq!(s.raw, None);
    s.aim(&mut req, Some(Vec3::new(2.0, 0.0, 0.0)), false);
    assert_eq!(s.raw, Some((FRAC_PI_2, 2.0)));
    assert_eq!(req.desired, FRAC_PI_2);
}

/// Standing, A turns the heading a quarter left and D right, once more than
/// 1000 ms of the game clock have passed since the last turn key (not at
/// exactly 1000); both cancel but still count as a turn key.
#[test]
fn a_standing_quarter_turn_waits_for_a_second() {
    let explore = |s: &mut Steering, req: &mut Locomotion, held: &str, now: u64| {
        let body = Locomotor::default();
        s.explore(
            req,
            &body,
            &keys(held),
            now,
            Vec3::ZERO,
            Condition::NORMAL,
            &open,
        );
    };
    let (mut s, mut req, _) = standing();
    explore(&mut s, &mut req, "a", 1000);
    assert_eq!(req.heading, 0.0);
    explore(&mut s, &mut req, "a", 1001);
    assert!((req.heading - FRAC_PI_2).abs() < 1e-6);
    explore(&mut s, &mut req, "a", 1500);
    assert!((req.heading - FRAC_PI_2).abs() < 1e-6);
    explore(&mut s, &mut req, "d", 2002);
    assert!(req.heading.abs() < 1e-6);
    explore(&mut s, &mut req, "ad", 3003);
    assert!(req.heading.abs() < 1e-6 && s.turn_key_ms == 3003);
    explore(&mut s, &mut req, "d", 3500);
    assert!(req.heading.abs() < 1e-6);
    // An object held in the hand blocks it.
    s.holds_object = true;
    explore(&mut s, &mut req, "d", 9000);
    assert!(req.heading.abs() < 1e-6);
}

/// Walking by the keys A and D steer 0.025 rad a tick and count as turn
/// keys: standing again, a quarter turn waits more than a second after the
/// last steer; the look turns 30 % toward the smoothed cursor at once.
#[test]
fn steering_a_walk_counts_as_a_turn_key() {
    let (mut s, mut req, body) = standing();
    req.cursor = FRAC_PI_2;
    for now in [100, 112, 124] {
        s.request(&mut req, &body, &keys("wa"), Some(1), Condition::NORMAL);
        s.explore(
            &mut req,
            &body,
            &keys("wa"),
            now,
            Vec3::ZERO,
            Condition::NORMAL,
            &open,
        );
    }
    assert!((req.heading - 3.0 * STEER).abs() < 1e-6);
    let heading = forward(req.heading);
    let look = (heading + (forward(FRAC_PI_2) - heading) * 0.3).normalize();
    assert!((req.look - yaw_of(look)).abs() < 1e-6);
    assert!(req.moving && req.free_walk && req.has(flags::KEYS));
    let stood = Locomotor::default();
    req.moving = false;
    s.request(&mut req, &stood, &keys("a"), None, Condition::NORMAL);
    s.explore(
        &mut req,
        &stood,
        &keys("a"),
        1124,
        Vec3::ZERO,
        Condition::NORMAL,
        &open,
    );
    assert!((req.heading - 3.0 * STEER).abs() < 1e-6);
    s.explore(
        &mut req,
        &stood,
        &keys("a"),
        1125,
        Vec3::ZERO,
        Condition::NORMAL,
        &open,
    );
    assert!((req.heading - (3.0 * STEER + FRAC_PI_2)).abs() < 1e-6);
}

/// Standing, the smoothed cursor direction is mixed 0.4 with the facing
/// each tick (a cursor 90° left reads 56.3°), and the head eases 5 % a tick
/// toward the heading moved 40 % toward it.
#[test]
fn standing_the_head_eases_toward_the_cursor() {
    let (mut s, mut req, body) = standing();
    req.cursor = FRAC_PI_2;
    s.explore(
        &mut req,
        &body,
        &keys(""),
        0,
        Vec3::ZERO,
        Condition::NORMAL,
        &open,
    );
    assert!(
        (req.cursor.to_degrees() - 56.3099).abs() < 1e-3,
        "{}",
        req.cursor.to_degrees()
    );
    // The eased-toward direction is the heading–cursor lerp as it is (its
    // length 0.887 here), not normalised first: 0.962° after one tick,
    // not 1.079°.
    let mixed = forward(req.cursor);
    let wanted = Vec3::Z + (mixed - Vec3::Z) * 0.4;
    let look = (Vec3::Z + (wanted - Vec3::Z) * 0.05).normalize();
    assert!((req.look - yaw_of(look)).abs() < 1e-6);
    assert!(
        (req.look.to_degrees() - 0.9619).abs() < 1e-3,
        "{}",
        req.look.to_degrees()
    );
    assert!(req.slow_turn && !req.moving);
}

/// Shift's sprint latch: W + Shift spends it on the run bit (a jog); the
/// mouse beyond 3 m runs and keeps it; within 3 m it is spent; W alone
/// walks.
#[test]
fn w_and_shift_jog_while_the_far_mouse_keeps_the_latch() {
    let run = |held: &str, distance: f32| {
        let (mut s, mut req, body) = standing();
        s.raw = Some((0.0, distance));
        req.desired = 0.0;
        Steering::latch(&mut req, &keys(held));
        s.request(&mut req, &body, &keys(held), Some(1), Condition::NORMAL);
        s.explore(
            &mut req,
            &body,
            &keys(held),
            0,
            Vec3::ZERO,
            Condition::NORMAL,
            &open,
        );
        (req.has(flags::RUN), req.has(flags::SPRINT), req.slow_turn)
    };
    assert_eq!(run("wS", 1.0), (true, false, true));
    assert_eq!(run("mS", 5.0), (true, true, false));
    assert_eq!(run("mS", 2.0), (true, false, true));
    assert_eq!(run("m", 5.0), (true, false, false));
    assert_eq!(run("w", 5.0), (false, false, true));
}

/// The look-ahead: walking at 1.69 cm a tick its goal lies 92.1 cm ahead
/// (75 + 6·v²) and the walk is free; a wall clamping it 60 cm ahead ends
/// the free walk, and within 100 cm stops the run and keeps the brake;
/// backwards it lies 70 cm behind.
#[test]
fn the_look_ahead_clamps_the_goal_at_a_wall() {
    let (mut s, mut req, mut body) = standing();
    body.speed = 1.69;
    s.request(&mut req, &body, &keys("w"), Some(1), Condition::NORMAL);
    s.explore(
        &mut req,
        &body,
        &keys("w"),
        0,
        Vec3::ZERO,
        Condition::NORMAL,
        &open,
    );
    assert!(
        (req.goal.z - (0.75 + 0.06 * 1.69 * 1.69)).abs() < 1e-5,
        "{}",
        req.goal
    );
    assert!(req.free_walk);
    let wall = |from: Vec3, to: Vec3| {
        let d = to - from;
        if d.length() > 0.6 {
            (from + d.normalize() * 0.6, true)
        } else {
            (to, false)
        }
    };
    req.flags |= flags::RUN | flags::SPRINT | flags::BRAKE;
    s.mode = CursorMode::Other;
    s.explore(
        &mut req,
        &body,
        &keys("w"),
        0,
        Vec3::ZERO,
        Condition::NORMAL,
        &wall,
    );
    assert!(!req.free_walk && (req.goal.z - 0.6).abs() < 1e-6);
    assert!(req.has(flags::BRAKE) && !req.has(flags::RUN) && !req.has(flags::SPRINT));
    let (mut s, mut req, body) = standing();
    s.request(&mut req, &body, &keys("s"), Some(1), Condition::NORMAL);
    s.explore(
        &mut req,
        &body,
        &keys("s"),
        0,
        Vec3::ZERO,
        Condition::NORMAL,
        &open,
    );
    assert!((req.goal.z + 0.7).abs() < 1e-6 && req.moving);
}

/// By the mouse with the cursor behind (beyond 120° of the facing) the
/// heading turns to it and nothing moves; ahead, the heading is the
/// cursor's and the look the heading.
#[test]
fn a_reversal_turns_before_walking() {
    let (mut s, mut req, body) = standing();
    req.cursor = PI;
    req.desired = PI;
    s.raw = Some((PI, 2.0));
    s.request(&mut req, &body, &keys("m"), Some(1), Condition::NORMAL);
    s.explore(
        &mut req,
        &body,
        &keys("m"),
        0,
        Vec3::ZERO,
        Condition::NORMAL,
        &open,
    );
    assert!((req.heading.abs() - PI).abs() < 1e-5 && !req.moving);
    let (mut s, mut req, body) = standing();
    req.cursor = 0.5;
    req.desired = 0.5;
    s.raw = Some((0.5, 2.0));
    s.request(&mut req, &body, &keys("m"), Some(1), Condition::NORMAL);
    s.explore(
        &mut req,
        &body,
        &keys("m"),
        0,
        Vec3::ZERO,
        Condition::NORMAL,
        &open,
    );
    assert!((req.heading - 0.5).abs() < 1e-6 && req.look == req.heading && req.moving);
}

/// Released, a free walk stops softly: run off, stopping on, no longer
/// free, its goal 8·1.69 + 20 = 33.52 cm ahead; a walk toward a goal only
/// drops its run.
#[test]
fn a_released_free_walk_stops_softly() {
    let (mut s, mut req, mut body) = standing();
    body.speed = 1.69;
    s.request(&mut req, &body, &keys("w"), Some(1), Condition::NORMAL);
    s.explore(
        &mut req,
        &body,
        &keys("w"),
        0,
        Vec3::ZERO,
        Condition::NORMAL,
        &open,
    );
    req.flags |= flags::RUN;
    s.request(&mut req, &body, &keys(""), Some(4), Condition::NORMAL);
    s.explore(
        &mut req,
        &body,
        &keys(""),
        12,
        Vec3::ZERO,
        Condition::NORMAL,
        &open,
    );
    assert!(!req.free_walk && req.has(flags::STOPPING) && !req.has(flags::RUN));
    assert!((req.goal.z - 0.3352).abs() < 1e-5, "{}", req.goal);
    req.flags |= flags::RUN;
    s.explore(
        &mut req,
        &body,
        &keys(""),
        24,
        Vec3::ZERO,
        Condition::NORMAL,
        &open,
    );
    assert!(!req.has(flags::RUN) && (req.goal.z - 0.3352).abs() < 1e-5);
}

/// Down, getting up or dead the exploration steers nothing; in combat mode
/// there is no slow turn and a moving body stops softly.
#[test]
fn no_steering_while_down_and_combat_stops_softly() {
    let (mut s, mut req, body) = standing();
    s.requested = true;
    s.explore(
        &mut req,
        &body,
        &keys("w"),
        0,
        Vec3::ZERO,
        Condition::GETTING_UP,
        &open,
    );
    assert!(!req.moving);
    let (mut s, mut req, body) = standing();
    req.moving = true;
    req.free_walk = true;
    s.fight(&mut req, &body, Vec3::ZERO, &open);
    assert!(!req.slow_turn && !req.free_walk && req.has(flags::STOPPING));
}

/// The key segment of a walk keyed every 10 frames from 10 to 80 (the
/// original's Walk): 0 before frame 10 and from 80 on (the wrap), 3 at
/// frame 40, 4 just after it, 7 before 80; less the clip's blend.
#[test]
fn the_walks_key_segments() {
    use exa_formats::rsq::{Action, Clip, Key, MotionSet, Moveset};
    let key = |time| Key {
        time,
        root: [0.0; 3],
        rotations: Vec::new(),
        flags: Vec::new(),
    };
    let set = MotionSet {
        bone_count: 0,
        actions: vec![Action {
            name: "Walk".into(),
            params: [0; 2],
        }],
        movesets: vec![Moveset {
            name: "Default".into(),
            parent: String::new(),
            clips: vec![Some(Clip {
                mode: 0,
                keys: (1..=8).map(|k| key(10.0 * k as f32)).collect(),
            })],
        }],
    };
    let motions = Motions::new(std::sync::Arc::new(set), std::sync::Arc::default(), 0.0);
    let at = |time: f32, blend: f32| {
        let mut anim = Animator::new("Default", "Walk");
        anim.time = time;
        anim.blend = blend;
        key_segment(&anim, &motions).unwrap()
    };
    assert_eq!(at(5.0, 0.0), 0);
    assert_eq!(at(40.0, 0.0), 3);
    assert_eq!(at(45.0, 0.0), 4);
    assert_eq!(at(79.0, 0.0), 7);
    assert_eq!(at(80.0, 0.0), 0);
    assert_eq!(at(45.0, 10.0), 3);
}

/// The installed locomotion clips the release rule reads (skipped without
/// the game): Walk, Run and Backpedal of the "Default" and "Torch"
/// movesets are keyed every 10 frames from 10 to 80, so their segments 0
/// and 4 are frames 0–10 and 40–50, half a cycle apart.
#[test]
fn the_installed_walks_are_keyed_every_10_frames() {
    let Some(dir) = exa_assets::locate_install(None) else {
        return;
    };
    let data = std::sync::Arc::new(exa_assets::GameData::open(&dir).unwrap());
    let motions = Motions::load(&exa_assets::Game(data));
    let walk: Vec<f32> = (1..=8).map(|k| 10.0 * k as f32).collect();
    for moveset in ["Default", "Torch"] {
        for action in ["Walk", "Run", "Backpedal"] {
            let anim = Animator::new(moveset, action);
            let clip = motions.clip(&anim, action).expect(action);
            let times: Vec<f32> = clip.keys.iter().map(|k| k.time).collect();
            assert_eq!(times, walk, "{moveset} {action}");
        }
    }
    let mut anim = Animator::new("Default", "Walk");
    for (time, segment) in [(5.0, 0), (39.0, 3), (45.0, 4), (50.0, 4), (51.0, 5)] {
        anim.time = time;
        assert_eq!(key_segment(&anim, &motions), Some(segment), "{time}");
    }
}

/// The look-ahead's reach (cm): 75 + 6·v² ahead, 92.137 at 1.69 cm a tick;
/// 45 + 6·v² for combat mode's walk, 62.137; 70 behind backwards.
#[test]
fn the_look_ahead_reaches_by_the_speed() {
    assert!((look_ahead_reach(1.69, false, false) - 92.1366).abs() < 1e-3);
    assert!((look_ahead_reach(1.69, false, true) - 62.1366).abs() < 1e-3);
    assert_eq!(look_ahead_reach(0.0, false, true), 45.0);
    assert_eq!(look_ahead_reach(4.0, true, false), -70.0);
}

/// The look-ahead's line walk is recorded for the overlay: from where the
/// character stands toward the reach, where it ended and whether it was
/// blocked; a soft stop records its goal's walk.
#[test]
fn the_look_ahead_is_recorded_for_the_overlay() {
    let (mut s, mut req, mut body) = standing();
    body.speed = 1.69;
    let at = Vec3::new(1.0, 0.0, 2.0);
    let wall = |from: Vec3, to: Vec3| (from + (to - from) * 0.5, true);
    s.request(&mut req, &body, &keys("w"), Some(1), Condition::NORMAL);
    s.explore(&mut req, &body, &keys("w"), 0, at, Condition::NORMAL, &wall);
    let l = s.line.unwrap();
    assert_eq!(l.asked, Asked::LookAhead);
    assert_eq!(l.from, at);
    assert!((l.to.z - at.z - 0.921366).abs() < 1e-5);
    assert!(l.blocked && (l.end - (at + Vec3::Z * 0.460683)).length() < 1e-5);
    assert_eq!(req.goal, l.end);
    req.free_walk = true;
    req.moving = true;
    s.request(&mut req, &body, &keys(""), Some(0), Condition::NORMAL);
    s.explore(&mut req, &body, &keys(""), 0, at, Condition::NORMAL, &open);
    assert_eq!(s.line.unwrap().asked, Asked::SoftStop);
}

/// The spawn snap: nothing on the first two behaviour ticks, on the third
/// the move across the ground to the nearest walkable point, never again;
/// none when the third finds the character down.
#[test]
fn the_spawn_snaps_on_the_third_tick() {
    let nearest = |p: Vec3| p + Vec3::new(0.3, 0.5, -0.15);
    let mut s = Steering::default();
    let at = Vec3::new(2.0, 1.0, 3.0);
    assert_eq!(s.snap(Condition::NORMAL, at, &nearest), None);
    assert_eq!(s.snap(Condition::NORMAL, at, &nearest), None);
    let by = s.snap(Condition::NORMAL, at, &nearest).unwrap();
    assert!((by - Vec3::new(0.3, 0.0, -0.15)).length() < 1e-6, "{by}");
    assert_eq!(s.snap(Condition::NORMAL, at, &nearest), None);
    let mut s = Steering::default();
    s.snap(Condition::NORMAL, at, &nearest);
    s.snap(Condition::NORMAL, at, &nearest);
    assert_eq!(s.snap(Condition::DOWN, at, &nearest), None);
    assert_eq!(s.snap(Condition::NORMAL, at, &nearest), None);
}

/// A soft stop pulls a goal in when its squared distance, in centimetres
/// and in three dimensions, is over (8·v + 20)². At speed 0 that reach is
/// 20 cm: 10 cm ahead and 30 cm up is pulled to 20 cm along the facing at
/// the standing height; 20 cm ahead on the level, and 10 cm ahead on the
/// level, stay; a NaN component stays.
#[test]
fn a_soft_stop_measures_the_goal_in_three_dimensions() {
    let (_s, mut req, body) = standing();
    let at = Vec3::ZERO;
    req.goal = Vec3::new(0.0, 0.30, 0.10);
    soft_stop(&mut req, &body, at, &open);
    assert!(
        (req.goal - Vec3::new(0.0, 0.0, 0.20)).length() < 1e-6,
        "{:?}",
        req.goal
    );
    req.goal = Vec3::new(0.0, 0.0, 0.20);
    soft_stop(&mut req, &body, at, &open);
    assert!(
        (req.goal - Vec3::new(0.0, 0.0, 0.20)).length() < 1e-6,
        "{:?}",
        req.goal
    );
    req.goal = Vec3::new(0.0, 0.0, 0.10);
    soft_stop(&mut req, &body, at, &open);
    assert!(
        (req.goal - Vec3::new(0.0, 0.0, 0.10)).length() < 1e-6,
        "{:?}",
        req.goal
    );
    req.goal = Vec3::new(f32::NAN, 0.0, 1.0);
    soft_stop(&mut req, &body, at, &open);
    assert!(req.goal.x.is_nan());
}

/// A footwork step's target: a step's length along its keys, clamped by
/// the line walk. On the level, 20 cm is 400 cm² and is not taken; 25 cm
/// is. A point 15 cm ahead and 15 cm up is 450 cm² and is taken; 15 cm
/// ahead on the level is not. An unordered distance is not taken.
#[test]
fn a_step_target_is_clamped_and_taken_past_400_cm2() {
    let mut s = Steering::default();
    let wall =
        |near: f32| move |from: Vec3, to: Vec3| (from + (to - from).normalize() * near, true);
    let target = Vec3::X * 0.8;
    assert_eq!(s.step_target(Vec3::ZERO, 0.0, target, &wall(0.2)), None);
    let t = s.step_target(Vec3::ZERO, 0.0, target, &wall(0.25)).unwrap();
    assert!((t - Vec3::X * 0.25).length() < 1e-6);
    assert_eq!(s.step_target(Vec3::ZERO, 0.0, target, &open), Some(target));
    assert_eq!(s.line.unwrap().asked, Asked::Step);
    let raised = |_: Vec3, _: Vec3| (Vec3::new(0.15, 0.15, 0.0), true);
    assert_eq!(
        s.step_target(Vec3::ZERO, 0.0, target, &raised),
        Some(Vec3::new(0.15, 0.15, 0.0))
    );
    let flat = |_: Vec3, _: Vec3| (Vec3::new(0.15, 0.0, 0.0), true);
    assert_eq!(s.step_target(Vec3::ZERO, 0.0, target, &flat), None);
    let unordered = |_: Vec3, _: Vec3| (Vec3::new(1.0, f32::NAN, 0.0), false);
    assert_eq!(s.step_target(Vec3::ZERO, 0.0, target, &unordered), None);
    // The line walk starts at the chest's x/z on the ground, and the 400
    // cm² gate is measured from the chest.
    let tells = std::cell::RefCell::new(Vec::new());
    let record = |from: Vec3, to: Vec3| {
        tells.borrow_mut().push((from, to));
        (to, false)
    };
    let chest = Vec3::new(0.0, 1.5, 0.0);
    assert_eq!(
        s.step_target(chest, 0.0, chest + Vec3::X * 0.8, &record),
        Some(chest + Vec3::X * 0.8)
    );
    assert_eq!(*tells.borrow(), vec![(Vec3::ZERO, chest + Vec3::X * 0.8)]);
}

/// The combat right-button walk goes toward the cursor with the fast turn,
/// walking within 3 m and running beyond.
#[test]
fn combat_walks_to_the_cursor() {
    // Combat: toward the cursor across the ground, walking within 3 m and
    // running beyond, with the fast turn; the request keeps only the sprint
    // latch.
    let (mut s, mut req, body) = standing();
    req.slow_turn = true;
    req.flags |= flags::BACK | flags::KEYS;
    s.raw = Some((0.0, 2.0));
    s.combat_request(true);
    assert!(s.requested && s.mode == CursorMode::Walk);
    assert!(s.combat_walk(&mut req, &body, Vec3::ZERO, Vec3::new(0.0, 0.0, 2.0), &open));
    assert!(req.moving && req.free_walk && !req.slow_turn);
    assert_eq!(req.flags, flags::MOVE);
    assert_eq!(req.heading, 0.0);
    let (mut s, mut req, body) = standing();
    s.raw = Some((FRAC_PI_2, 4.0));
    s.combat_request(true);
    assert_eq!(s.mode, CursorMode::Run);
    s.combat_walk(&mut req, &body, Vec3::ZERO, Vec3::new(4.0, 0.0, 0.0), &open);
    assert!(req.has(flags::RUN));
    assert!((req.heading - FRAC_PI_2).abs() < 1e-6);
    // Within 3 m the Run key's latch turns into the run.
    let (mut s, mut req, body) = standing();
    s.raw = Some((0.0, 2.0));
    s.combat_request(true);
    req.flags |= flags::SPRINT;
    s.combat_walk(&mut req, &body, Vec3::ZERO, Vec3::new(0.0, 0.0, 2.0), &open);
    assert_eq!(req.flags, flags::MOVE | flags::RUN);
    // A cursor within 50 cm is no walk, but the button still asks it.
    let (mut s, mut req, body) = standing();
    s.combat_request(true);
    assert!(s.requested);
    assert!(!s.combat_walk(&mut req, &body, Vec3::ZERO, Vec3::new(0.0, 0.0, 0.2), &open));
    assert!(!req.moving);
    // Released, the button asks nothing more.
    let mut s = Steering::default();
    s.combat_request(false);
    assert!(!s.requested);
}

/// A key pressed for under 200 ms keeps the movement word until 200 ms
/// after the press (strictly over), so a tap moves for a step; a key held
/// longer clears the word as soon as it is released. The movement target
/// is let go once the word has been zero for over 200 ms since its last
/// nonzero tick.
#[test]
fn a_tap_keeps_the_movement_word_for_a_step() {
    let mut w = MoveWord::default();
    let fwd = [true, false, false, false];
    let none = [false; 4];
    assert_eq!(w.tick(1000, fwd, Vec2::ZERO, 0.0), MOVE_FORWARD);
    // Released after 10 ms: the word is kept.
    assert_eq!(w.tick(1010, none, Vec2::ZERO, 0.0), MOVE_FORWARD);
    assert_eq!(w.tick(1200, none, Vec2::ZERO, 0.0), MOVE_FORWARD);
    assert_eq!(w.tick(1201, none, Vec2::ZERO, 0.0), 0);
    assert_eq!(
        MoveWord::keys(MOVE_FORWARD | MOVE_LEFT),
        [true, false, true, false]
    );
    // Held 500 ms: released, the word clears at once.
    let mut w = MoveWord::default();
    w.tick(1000, fwd, Vec2::ZERO, 0.0);
    w.tick(1500, fwd, Vec2::ZERO, 0.0);
    assert_eq!(w.tick(1510, none, Vec2::ZERO, 0.0), 0);
    // The target goes 200 ms after the last nonzero tick (the release
    // tick itself changes the word and does not count).
    let mut w = MoveWord::default();
    w.tick(1000, fwd, Vec2::ZERO, 0.0);
    w.tick(1400, fwd, Vec2::ZERO, 0.0);
    w.tick(1500, none, Vec2::ZERO, 0.0);
    assert!(!w.target_expired);
    w.tick(1600, none, Vec2::ZERO, 0.0);
    assert!(!w.target_expired);
    w.tick(1601, none, Vec2::ZERO, 0.0);
    assert!(w.target_expired);
}

/// The key word follows the reader's order: A is left (2), D right (8),
/// W forward (1), S back (4); A with D and W with S cancel.
#[test]
fn opposing_keys_cancel_in_the_movement_word() {
    // [forward, back, left, right]
    assert_eq!(MoveWord::of_keys([false, false, true, false]), MOVE_LEFT);
    assert_eq!(MoveWord::of_keys([false, false, false, true]), MOVE_RIGHT);
    assert_eq!(MoveWord::of_keys([false, false, true, true]), 0);
    assert_eq!(MoveWord::of_keys([true, true, false, false]), 0);
    assert_eq!(MoveWord::of_keys([true, true, true, false]), MOVE_LEFT);
    assert_eq!(
        MoveWord::of_keys([false, true, false, true]),
        MOVE_BACK | MOVE_RIGHT
    );
    assert_eq!(
        (MOVE_FORWARD, MOVE_LEFT, MOVE_BACK, MOVE_RIGHT),
        (1, 2, 4, 8)
    );
}

/// The word is a tap once it has stayed the same over 200 ms with the
/// body's crouch under 0.1; a crouching body is no tap, and the pad's
/// analog word needs the stick past its squared 0.9.
#[test]
fn a_held_word_counts_as_a_tap() {
    let mut w = MoveWord::default();
    let fwd = [true, false, false, false];
    w.tick(1000, fwd, Vec2::ZERO, 0.0);
    w.tick(1200, fwd, Vec2::ZERO, 0.0);
    assert!(!w.tap);
    w.tick(1201, fwd, Vec2::ZERO, 0.0);
    assert!(w.tap);
    let mut w = MoveWord::default();
    w.tick(1000, fwd, Vec2::ZERO, 1.0);
    w.tick(1300, fwd, Vec2::ZERO, 1.0);
    assert!(!w.tap);
    // A stick at 0.5 (squared 0.25) is an analog word but not a tap; a full
    // one is.
    let mut w = MoveWord::default();
    assert_eq!(
        w.tick(1000, [false; 4], Vec2::new(0.5, 0.0), 0.0),
        MOVE_ANALOG
    );
    w.tick(1300, [false; 4], Vec2::new(0.5, 0.0), 0.0);
    assert!(!w.tap);
    let mut w = MoveWord::default();
    w.tick(1000, [false; 4], Vec2::new(1.0, 0.0), 0.0);
    w.tick(1300, [false; 4], Vec2::new(1.0, 0.0), 0.0);
    assert!(w.tap);
    // The stick's dead zone: its single squared length over the double
    // 0.01. The single 0.1 squares to 0.010000001, just over, so a tenth of
    // a stick already reads the analog word; a NaN fails the ordered test
    // and reads the keys.
    let mut w = MoveWord::default();
    assert_eq!(w.tick(0, fwd, Vec2::new(0.09, 0.0), 0.0), MOVE_FORWARD);
    let mut w = MoveWord::default();
    assert_eq!(w.tick(0, fwd, Vec2::new(0.1, 0.0), 0.0), MOVE_ANALOG);
    let mut w = MoveWord::default();
    assert_eq!(w.tick(0, fwd, Vec2::new(f32::NAN, 0.0), 0.0), MOVE_FORWARD);
}

/// The movement goal's direction: W is the facing, S back, A and D the side
/// vector at 0.8, and the bits add in the original's order (W then A, D and
/// S); no bit gives zero.
#[test]
fn the_key_goal_takes_the_facing_and_the_scaled_side() {
    let facing = Vec3::Z;
    assert_eq!(
        key_goal_direction(facing, MOVE_FORWARD),
        Vec3::new(0.0, 0.0, 1.0)
    );
    assert_eq!(key_goal_direction(facing, 0), Vec3::ZERO);
    // +Z facing: the side is (1, 0, 0), so A adds 0.8 of it and D takes it.
    assert_eq!(
        key_goal_direction(facing, MOVE_LEFT),
        Vec3::new(0.8, 0.0, 0.0)
    );
    assert_eq!(
        key_goal_direction(facing, MOVE_RIGHT),
        Vec3::new(-0.8, 0.0, 0.0)
    );
    assert_eq!(
        key_goal_direction(facing, MOVE_FORWARD | MOVE_LEFT),
        Vec3::new(0.8, 0.0, 1.0)
    );
    assert_eq!(
        key_goal_direction(facing, MOVE_BACK | MOVE_RIGHT),
        Vec3::new(-0.8, 0.0, -1.0)
    );
    // Facing +X: the side is (0, 0, -1), at 0.8.
    let facing = Vec3::X;
    assert_eq!(
        key_goal_direction(facing, MOVE_LEFT),
        Vec3::new(0.0, 0.0, -0.8)
    );
}

/// A tapped word lengthens the stride to `(1.5 − 0.3·encumbrance)` of the
/// step, in double precision, then multiplies the clock factor; a held word
/// keeps the step.
#[test]
fn a_tap_takes_a_longer_stride() {
    // 80 cm, encumbrance 0, clock 1: 120 cm; encumbrance 0.5: 108 cm.
    assert_eq!(goal_stride(80.0, 1.0, 0.0, true), 120.0);
    assert_eq!(goal_stride(80.0, 1.0, 0.5, true), 108.0);
    assert_eq!(goal_stride(80.0, 0.25, 0.0, true), 30.0);
    assert_eq!(goal_stride(80.0, 1.0, 0.0, false), 80.0);
    // The double product narrows once: (1.5 − 0.3·0.1)·72 = 105.84.
    assert_eq!(goal_stride(72.0, 1.0, 0.1, true), 105.84);
    // A NaN encumbrance makes a tap's stride NaN.
    assert!(goal_stride(80.0, 1.0, f32::NAN, true).is_nan());
}

/// The goal lets go of the movement target past 400 cm² (20 cm): 20 cm does
/// not, 20.1 does, and a NaN height never does.
#[test]
fn the_goal_releases_past_four_hundred_squared_centimetres() {
    assert!(!goal_released(Vec3::new(0.0, 0.0, 20.0)));
    assert!(goal_released(Vec3::new(0.0, 0.0, 20.1)));
    assert!(!goal_released(Vec3::new(20.0, 0.0, 0.0)));
    assert!(goal_released(Vec3::new(20.1, 0.0, 0.0)));
    assert!(!goal_released(Vec3::new(0.0, 20.0, 0.0)));
    assert!(goal_released(Vec3::new(0.0, 20.1, 0.0)));
    assert!(!goal_released(Vec3::new(0.0, f32::NAN, 0.0)));
    // 400 cm² across the ground with 0 height.
    assert!(!goal_released(Vec3::new(0.0, 0.0, 400.0f32.sqrt())));
}

/// The movement word's step vector is its direction scaled to the body's
/// step times its factor, over the direction's length, with the height
/// dropped: 80 cm along +Z, 80 cm of a diagonal kept, the pad's own
/// direction; a zero word divides by zero.
#[test]
fn the_key_step_is_the_word_scaled_to_the_step() {
    let facing = Vec3::Z;
    let step = key_step(facing, MOVE_FORWARD, Vec3::ZERO, 80.0, 1.0);
    assert_eq!(step, Vec3::new(0.0, 0.0, 80.0));
    // W and A: (0.8, 0, 1) over 1.2806248, times 80, is 49.9756 and 62.4695 cm.
    let diagonal = key_step(facing, MOVE_FORWARD | MOVE_LEFT, Vec3::ZERO, 80.0, 1.0);
    assert!((diagonal.x - 49.975_6).abs() < 1e-3, "{diagonal}");
    assert!((diagonal.z - 62.469_5).abs() < 1e-3, "{diagonal}");
    assert!((diagonal.length() - 80.0).abs() < 1e-4);
    // A pure side step keeps the full step.
    let side = key_step(facing, MOVE_LEFT, Vec3::ZERO, 80.0, 1.0);
    assert_eq!(side, Vec3::new(80.0, 0.0, 0.0));
    // The factor scales it (the step's clock share).
    let half = key_step(facing, MOVE_FORWARD, Vec3::ZERO, 80.0, 0.5);
    assert_eq!(half, Vec3::new(0.0, 0.0, 40.0));
    // The pad's direction, its height dropped.
    let analog = key_step(facing, MOVE_ANALOG, Vec3::new(0.0, 3.0, 2.0), 50.0, 1.0);
    assert_eq!(analog, Vec3::new(0.0, 0.0, 50.0));
    // No direction: the division by zero gives a NaN vector.
    assert!(key_step(facing, 0, Vec3::ZERO, 80.0, 1.0).is_nan());
}
