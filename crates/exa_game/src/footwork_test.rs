use super::*;
use crate::steering::{MOVE_FORWARD, MOVE_LEFT};

const OPEN: StepGate = StepGate {
    condition: 0,
    energy: 1.0,
    swinging: false,
};

/// A word held at `held` (its last laid word `last`), a tap or not.
fn word(held: u32, last: u32, tap: bool) -> MoveWord {
    let mut w = MoveWord::default();
    w.held = held;
    w.last_word = last;
    w.tap = tap;
    w
}

fn started(at: u64, long: bool) -> StepRecord {
    StepRecord {
        active: true,
        started_ms: at,
        long,
        ..StepRecord::default()
    }
}

/// With no step under way the held word lays the goal; a zero held word
/// or a closed gate asks nothing.
#[test]
fn a_free_body_lays_the_goal_from_the_held_word() {
    let w = word(MOVE_FORWARD, 0, false);
    let step = StepRecord::default();
    assert_eq!(
        dispatch(&w, 0, 1000, &step, false, OPEN),
        StepAsk::Start(MOVE_FORWARD)
    );
    assert_eq!(
        dispatch(&word(0, 0, false), MOVE_FORWARD, 1000, &step, false, OPEN),
        StepAsk::Nothing
    );
    let walking = StepGate {
        condition: 1,
        ..OPEN
    };
    assert_eq!(
        dispatch(&w, 0, 1000, &step, false, walking),
        StepAsk::Nothing
    );
    // The kick's bit (0x8000) closes it too; attacking (2) does not.
    for (condition, open) in [(0x8000, false), (0x200, false), (2, true), (8, true)] {
        let gate = StepGate { condition, ..OPEN };
        assert_eq!(gate.open(), open, "{condition:#x}");
    }
    assert!(
        StepGate {
            energy: 0.7,
            ..OPEN
        }
        .open()
    );
}

/// A young step (under 300 ms) queues a raw word different from its own
/// laid word; the same word queues only as the analog word; a tapped word
/// re-lays a short step and leaves a long one.
#[test]
fn a_young_step_queues_or_relays_by_the_tap() {
    let step = started(1000, false);
    let w = word(MOVE_FORWARD, MOVE_FORWARD, false);
    assert_eq!(
        dispatch(&w, MOVE_FORWARD, 1299, &step, true, OPEN),
        StepAsk::Nothing
    );
    assert_eq!(
        dispatch(&w, MOVE_ANALOG, 1299, &step, true, OPEN),
        StepAsk::Queue(MOVE_ANALOG)
    );
    let other = word(MOVE_LEFT, MOVE_FORWARD, false);
    assert_eq!(
        dispatch(&other, MOVE_LEFT, 1100, &step, true, OPEN),
        StepAsk::Queue(MOVE_LEFT)
    );
    assert_eq!(
        dispatch(&other, 0, 1100, &step, true, OPEN),
        StepAsk::Nothing
    );
    let tapped = word(MOVE_FORWARD, MOVE_FORWARD, true);
    assert_eq!(
        dispatch(&tapped, MOVE_FORWARD, 1100, &step, true, OPEN),
        StepAsk::Relay(MOVE_FORWARD)
    );
    assert_eq!(
        dispatch(
            &tapped,
            MOVE_FORWARD,
            1100,
            &started(1000, true),
            true,
            OPEN
        ),
        StepAsk::Nothing
    );
}

/// From 300 ms on, any nonzero raw word queues; a zero one does not.
#[test]
fn an_old_step_queues_any_raw_word() {
    let step = started(1000, true);
    let w = word(MOVE_FORWARD, MOVE_FORWARD, true);
    assert_eq!(
        dispatch(&w, MOVE_FORWARD, 1300, &step, true, OPEN),
        StepAsk::Queue(MOVE_FORWARD)
    );
    assert_eq!(dispatch(&w, 0, 1300, &step, true, OPEN), StepAsk::Nothing);
}

/// A landed step stays under way while its target is strictly more than
/// 20·scale cm from the chest across the ground; its height does not
/// count, and a NaN distance is not under way.
#[test]
fn a_landed_step_is_under_way_until_near() {
    let mut step = started(0, false);
    assert!(step.under_way(Vec3::ZERO, 1.0));
    step.landed = true;
    step.target = Vec3::new(0.0, 5.0, 0.2);
    assert!(!step.under_way(Vec3::ZERO, 1.0));
    step.target = Vec3::new(0.0, 0.0, 0.201);
    assert!(step.under_way(Vec3::ZERO, 1.0));
    assert!(!step.under_way(Vec3::ZERO, 1.01));
    step.target = Vec3::new(f32::NAN, 0.0, 1.0);
    assert!(!step.under_way(Vec3::ZERO, 1.0));
    assert!(!StepRecord::default().under_way(Vec3::ZERO, 1.0));
}

/// Queueing stores the raw word on the word and its full step vector (m)
/// on the step, with a queue of one.
#[test]
fn a_queued_word_keeps_its_step_vector() {
    let mut keys = MoveWord::default();
    let mut step = StepRecord::default();
    step.queue_word(&mut keys, MOVE_LEFT, Vec3::Z, Vec3::ZERO, 80.0, 2.0);
    assert_eq!((keys.queued_word, step.queue), (MOVE_LEFT, 1));
    // The side vector at full weight, scaled to reach × root: 160 cm.
    assert!((step.queued - Vec3::X * 1.6).length() < 1e-6);
}

/// Once the body took the queued step the word takes the queued word
/// back, the queue and the tap clear and the clocks restart, so the word
/// is a tap again only 200 ms later.
#[test]
fn a_taken_queue_gives_the_word_back() {
    let mut keys = MoveWord::default();
    let fwd = [true, false, false, false];
    keys.tick(1000, fwd, Vec2::ZERO, 0.0);
    keys.tick(1300, fwd, Vec2::ZERO, 0.0);
    assert!(keys.tap);
    keys.queued_word = MOVE_LEFT;
    let mut queue = 1;
    keys.take_queue(1400, &mut queue);
    assert_eq!(
        (keys.held, queue),
        (MOVE_FORWARD, 1),
        "a positive queue waits"
    );
    queue = -1;
    keys.take_queue(1400, &mut queue);
    assert_eq!((keys.held, keys.queued_word, queue), (MOVE_LEFT, 0, 0));
    assert!(!keys.tap);
    let left = [false, false, true, false];
    keys.tick(1600, left, Vec2::ZERO, 0.0);
    assert!(!keys.tap);
    keys.tick(1601, left, Vec2::ZERO, 0.0);
    assert!(keys.tap);
}

/// The goal's direction: the key sum at a 0.8 side, or the analog word's
/// own direction with its height dropped.
#[test]
fn the_goal_direction_reads_the_word_or_the_stick() {
    assert_eq!(
        goal_direction(Vec3::Z, MOVE_FORWARD | MOVE_LEFT, Vec3::ZERO),
        Vec3::new(0.8, 0.0, 1.0)
    );
    assert_eq!(
        goal_direction(Vec3::Z, MOVE_ANALOG, Vec3::new(1.0, 2.0, 3.0)),
        Vec3::new(1.0, 0.0, 3.0)
    );
}
