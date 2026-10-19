//! A path walker at a door.
//!
//! With a live waypoint that is a door, openness at most 0.7 holds the
//! walker. If the door is not already toggling, the walker toggles it.
//! No path, a cursor at the end, a missing door, or a door more than 0.7
//! open lets the walker continue.
//!
//! Evidence: the path step that meets a door. A wait in that step gives up
//! strictly after ten seconds. An approach within 250 cm of a span's
//! midpoint starts that wait. The wander look eases toward the heading
//! plus an offset. The aim snap and the other phases are not in this slice.

use bevy::math::Vec2;
use exa_physics::door::{self, SignalDrive, SignalLeaf};

/// Where the walker is on its path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Walk {
    /// Negative when the walker has no path.
    pub path: i32,
    pub cursor: i32,
    pub waypoints: i32,
}

/// The walker is closing on a waypoint span.
pub const APPROACH: i32 = 3;
/// The walker is waiting at a door.
pub const AT_DOOR: i32 = 4;
/// Horizontal reach of a waypoint span, in centimetres.
pub const SPAN_REACH_CM: f32 = 250.0;
const SPAN_REACH_CM2: f32 = 62_500.0;
/// The look sway runs in this mode.
pub const LOOK_MODE: i32 = 1;
const SWAY_PI: f32 = 3.1415927;
const SWAY_TURN: f32 = SWAY_PI * 2.0;

/// A walker's wandering look.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LookSway {
    pub mode: i32,
    pub heading: f32,
    pub stored_heading: f32,
    pub offset: f32,
    pub look: f32,
    pub timer_ms: u32,
    pub roll: u32,
}

/// Ease the look toward the heading plus a wandering offset.
///
/// Only the wander mode runs. The stored heading copies the heading, and
/// the look starts from the extracted angle. While the timer is still at
/// the clock, the offset and the roll stay. Once the timer is behind, the
/// offset is `unit * 3.6 - 1.8`, the roll steps by `roll * 0x8088405 + 1`,
/// and the next timer is the high half of that step times 4000, plus the
/// clock, plus 4000. The aim wraps once when it passes half a turn. The
/// look then moves by `(0.09 - posture * 0.06)` of the remaining way.
/// The flag call and the angle extract are not in this slice.
pub fn sway_look(sway: &mut LookSway, extracted: f32, posture: f32, now_ms: u32, unit: f32) {
    if sway.mode != LOOK_MODE {
        return;
    }
    sway.stored_heading = sway.heading;
    sway.look = extracted;
    if sway.timer_ms < now_ms {
        sway.offset = unit * 3.6 - 1.8;
        sway.roll = sway.roll.wrapping_mul(0x808_8405).wrapping_add(1);
        let gap = ((u64::from(sway.roll).wrapping_mul(4_000)) >> 32) as u32;
        sway.timer_ms = gap.wrapping_add(now_ms).wrapping_add(4_000);
    }
    let mut aim = sway.heading + sway.offset;
    if -SWAY_PI <= aim {
        if SWAY_PI < aim {
            aim -= SWAY_TURN;
        }
    } else {
        aim += SWAY_TURN;
    }
    let gain = 0.09 - posture * 0.06;
    sway.look = gain * (aim - sway.look) + sway.look;
}
/// The walker is seeking a waypoint again.
pub const SEEKING: i32 = 2;
/// The door let the walker through.
pub const PAST_DOOR: i32 = 5;
/// A door wait gives up strictly after this many milliseconds.
pub const DOOR_WAIT_MS: u32 = 10_000;

/// A walker waiting on the door at its cursor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DoorWait {
    pub phase: i32,
    pub since_ms: u32,
    /// Waypoints already skipped. Bit `n` is cursor `n` modulo 32.
    pub skipped: u32,
    /// Node the search avoids. `-1` avoids none.
    pub avoid: i32,
    /// Non-zero while the search must not run.
    pub hold_search: u8,
}

/// True when `at` is within 250 cm, horizontally, of the span's midpoint.
///
/// A missing span is not near. The compare is the squared distance, so
/// exactly 250 cm is near and anything farther is not.
pub fn near_span(ends: Option<[Vec2; 2]>, at: Vec2) -> bool {
    let Some([a, b]) = ends else {
        return false;
    };
    let mid = a + (b - a) * 0.5;
    let d = at - mid;
    d.length_squared() <= SPAN_REACH_CM2
}

/// Step a walker that is approaching a waypoint span.
///
/// While the clock is still at the stamp and the walker is busy, nothing
/// changes. Otherwise a span that is not within 250 cm sends the walker
/// back to seeking: the first miss sets only the high skipped bit, and a
/// later miss also skips the cursor. A span within reach starts the door
/// wait at the current clock. Any other phase is left as it is. The aim
/// snap that runs after the choice is not in this slice.
pub fn approach_span(
    wait: &mut DoorWait,
    walk: &Walk,
    ends: Option<[Vec2; 2]>,
    at: Vec2,
    now_ms: u32,
    still_busy: bool,
) {
    if wait.phase != APPROACH {
        return;
    }
    if now_ms <= wait.since_ms && still_busy {
        return;
    }
    if !near_span(ends, at) {
        wait.phase = SEEKING;
        if (wait.skipped & 0x8000_0000) == 0 {
            wait.skipped |= 0x8000_0000;
        } else {
            wait.skipped |= 1u32 << (walk.cursor as u32 & 31);
        }
    } else {
        wait.phase = AT_DOOR;
        wait.since_ms = now_ms;
    }
}

/// Step a walker that is waiting at a door.
///
/// A door that lets the walker through moves it past the door and lets the
/// search run. Strictly more than ten seconds after the wait started, the
/// walker goes back to seeking, clears the avoided node, and marks the
/// cursor skipped. That later rule wins the phase when both happen. Any
/// other phase is left as it is. The heading sway that runs first is not
/// in this slice.
pub fn give_up_door(wait: &mut DoorWait, walk: &Walk, door: Option<&mut SignalLeaf>, now_ms: u32) {
    if wait.phase != AT_DOOR {
        return;
    }
    if pass_door(walk, door) {
        wait.phase = PAST_DOOR;
        wait.hold_search = 0;
    }
    if u64::from(wait.since_ms) + u64::from(DOOR_WAIT_MS) < u64::from(now_ms) {
        wait.avoid = -1;
        wait.phase = SEEKING;
        wait.skipped |= 1u32 << (walk.cursor as u32 & 31);
    }
}

/// True when the walker may continue. False when a door holds it.
pub fn pass_door(walk: &Walk, door: Option<&mut SignalLeaf>) -> bool {
    let Some(door) = door else {
        return true;
    };
    if walk.path < 0 || walk.waypoints <= 0 || walk.cursor >= walk.waypoints {
        return true;
    }
    let openness = door::open_span(door);
    let holds = matches!(
        openness.partial_cmp(&door::TOGGLE_OPEN),
        Some(std::cmp::Ordering::Less | std::cmp::Ordering::Equal)
    );
    if !holds {
        return true;
    }
    if door.drive != SignalDrive::Toggle {
        door::toggle(door);
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::math::Vec2;
    use exa_physics::door::{SignalDrive, SignalLeaf, TOGGLE_FAST, TOGGLE_SLOW};

    fn walk() -> Walk {
        Walk {
            path: 0,
            cursor: 0,
            waypoints: 2,
        }
    }

    fn hinge() -> SignalLeaf {
        SignalLeaf {
            sliding: false,
            latched: false,
            motorised: false,
            closed: true,
            count: 20,
            fraction: 1.0,
            angle: 0.0,
            closed_angle: 0.0,
            fast: false,
            drive: SignalDrive::Idle,
            countdown: 0,
        }
    }

    #[test]
    fn a_walker_toggles_a_door_at_most_seven_tenths_open() {
        let mut shut = hinge();
        assert!(pass_door(&Walk { path: -1, ..walk() }, Some(&mut shut)));
        assert!((shut.fraction - 1.0).abs() < 1e-6);
        assert_eq!(shut.drive, SignalDrive::Idle);

        assert!(pass_door(
            &Walk {
                cursor: 2,
                ..walk()
            },
            Some(&mut shut)
        ));
        assert!(pass_door(&walk(), None));

        let mut wide = hinge();
        wide.sliding = true;
        wide.fraction = 0.29;
        assert!(pass_door(&walk(), Some(&mut wide)));
        assert!((wide.fraction - 0.29).abs() < 1e-6);
        assert_eq!(wide.drive, SignalDrive::Idle);

        let mut door = hinge();
        assert!(!pass_door(&walk(), Some(&mut door)));
        assert!((door.fraction - 0.0).abs() < 1e-6);
        assert_eq!(door.drive, SignalDrive::Toggle);
        assert_eq!(door.countdown, TOGGLE_SLOW);

        let mut line = hinge();
        line.sliding = true;
        line.fraction = 0.3;
        line.closed = true;
        assert!(!pass_door(&walk(), Some(&mut line)));
        assert_eq!(line.drive, SignalDrive::Open);
        assert_eq!(line.countdown, TOGGLE_FAST);
        assert!(!line.closed);

        let mut busy = hinge();
        busy.drive = SignalDrive::Toggle;
        busy.countdown = 40;
        busy.fraction = 1.0;
        assert!(!pass_door(&walk(), Some(&mut busy)));
        assert!((busy.fraction - 1.0).abs() < 1e-6);
        assert_eq!(busy.countdown, 40);
        assert_eq!(busy.drive, SignalDrive::Toggle);
    }

    fn waiting(since_ms: u32) -> DoorWait {
        DoorWait {
            phase: AT_DOOR,
            since_ms,
            skipped: 0,
            avoid: 7,
            hold_search: 1,
        }
    }

    #[test]
    fn a_door_wait_gives_up_after_ten_seconds() {
        let mut early = DoorWait {
            phase: SEEKING,
            ..waiting(1_000)
        };
        let mut shut = hinge();
        give_up_door(
            &mut early,
            &walk(),
            Some(&mut shut),
            1_000 + DOOR_WAIT_MS + 1,
        );
        assert_eq!(early.phase, SEEKING);
        assert_eq!(early.avoid, 7);
        assert_eq!(early.skipped, 0);
        assert_eq!(early.hold_search, 1);
        assert_eq!(shut.drive, SignalDrive::Idle);
        assert!((shut.fraction - 1.0).abs() < 1e-6);

        let mut wait = waiting(1_000);
        let mut door = hinge();
        let at = Walk {
            cursor: 3,
            waypoints: 8,
            ..walk()
        };
        give_up_door(&mut wait, &at, Some(&mut door), 1_000 + DOOR_WAIT_MS);
        assert_eq!(wait.phase, AT_DOOR);
        assert_eq!(wait.avoid, 7);
        assert_eq!(wait.skipped, 0);
        assert_eq!(wait.hold_search, 1);
        assert_eq!(door.drive, SignalDrive::Toggle);
        assert_eq!(door.countdown, TOGGLE_SLOW);
        assert!((door.fraction - 0.0).abs() < 1e-6);

        give_up_door(&mut wait, &at, Some(&mut door), 1_000 + DOOR_WAIT_MS + 1);
        assert_eq!(wait.phase, SEEKING);
        assert_eq!(wait.avoid, -1);
        assert_eq!(wait.skipped, 1 << 3);
        assert_eq!(wait.hold_search, 1);
        assert_eq!(door.drive, SignalDrive::Toggle);
        assert_eq!(door.countdown, TOGGLE_SLOW);

        let mut open = waiting(1_000);
        let mut wide = hinge();
        wide.sliding = true;
        wide.fraction = 0.29;
        give_up_door(&mut open, &walk(), Some(&mut wide), 1_000);
        assert_eq!(open.phase, PAST_DOOR);
        assert_eq!(open.hold_search, 0);
        assert_eq!(open.avoid, 7);
        assert_eq!(open.skipped, 0);
        assert_eq!(wide.drive, SignalDrive::Idle);
        assert!((wide.fraction - 0.29).abs() < 1e-6);

        let mut late = waiting(1_000);
        let mut still = hinge();
        still.sliding = true;
        still.fraction = 0.29;
        let far = Walk {
            cursor: 32,
            waypoints: 40,
            ..walk()
        };
        give_up_door(&mut late, &far, Some(&mut still), 1_000 + DOOR_WAIT_MS + 1);
        assert_eq!(late.phase, SEEKING);
        assert_eq!(late.hold_search, 0);
        assert_eq!(late.avoid, -1);
        assert_eq!(late.skipped, 1);
        assert_eq!(still.drive, SignalDrive::Idle);

        let mut wrapped = waiting(u32::MAX - 50);
        let mut across = hinge();
        across.sliding = true;
        across.fraction = 0.29;
        give_up_door(&mut wrapped, &walk(), Some(&mut across), 100);
        assert_eq!(wrapped.phase, PAST_DOOR);
        assert_eq!(wrapped.avoid, 7);
        assert_eq!(wrapped.skipped, 0);

        let mut negative = waiting(1_000);
        let mut back_door = hinge();
        let back = Walk {
            cursor: -1,
            waypoints: 8,
            ..walk()
        };
        give_up_door(
            &mut negative,
            &back,
            Some(&mut back_door),
            1_000 + DOOR_WAIT_MS + 1,
        );
        assert_eq!(negative.phase, SEEKING);
        assert_eq!(negative.skipped, 1 << 31);
    }

    fn approaching(since_ms: u32) -> DoorWait {
        DoorWait {
            phase: APPROACH,
            since_ms,
            skipped: 0,
            avoid: 7,
            hold_search: 1,
        }
    }

    #[test]
    fn an_approach_within_two_hundred_fifty_centimetres_starts_the_door_wait() {
        let ends = Some([Vec2::new(0.0, 0.0), Vec2::new(200.0, 0.0)]);
        let mid = Vec2::new(100.0, 0.0);
        let on_reach = mid + Vec2::new(SPAN_REACH_CM, 0.0);
        let past = mid + Vec2::new(SPAN_REACH_CM + 1.0, 0.0);

        let mut other = approaching(5_000);
        other.phase = AT_DOOR;
        approach_span(&mut other, &walk(), ends, past, 9_000, false);
        assert_eq!(other.phase, AT_DOOR);
        assert_eq!(other.since_ms, 5_000);
        assert_eq!(other.skipped, 0);

        let mut held = approaching(5_000);
        approach_span(&mut held, &walk(), ends, past, 5_000, true);
        assert_eq!(held.phase, APPROACH);
        assert_eq!(held.since_ms, 5_000);
        assert_eq!(held.skipped, 0);
        assert_eq!(held.avoid, 7);

        let mut exact = approaching(5_000);
        approach_span(&mut exact, &walk(), ends, on_reach, 5_000, false);
        assert_eq!(exact.phase, AT_DOOR);
        assert_eq!(exact.since_ms, 5_000);
        assert_eq!(exact.skipped, 0);

        let mut first = approaching(5_000);
        let at = Walk {
            cursor: 3,
            waypoints: 8,
            ..walk()
        };
        approach_span(&mut first, &at, None, past, 5_001, true);
        assert_eq!(first.phase, SEEKING);
        assert_eq!(first.skipped, 0x8000_0000);
        assert_eq!(first.since_ms, 5_000);
        assert_eq!(first.avoid, 7);

        let mut again = approaching(5_000);
        again.skipped = 0x8000_0000;
        approach_span(&mut again, &at, ends, past, 5_001, false);
        assert_eq!(again.phase, SEEKING);
        assert_eq!(again.skipped, 0x8000_0000 | (1 << 3));

        let mut wide = approaching(5_000);
        wide.skipped = 0x8000_0000;
        let far = Walk {
            cursor: 32,
            waypoints: 40,
            ..walk()
        };
        approach_span(&mut wide, &far, ends, past, 5_001, false);
        assert_eq!(wide.skipped, 0x8000_0000 | 1);

        let mut arrived = approaching(5_000);
        approach_span(&mut arrived, &walk(), ends, mid, 5_001, true);
        assert_eq!(arrived.phase, AT_DOOR);
        assert_eq!(arrived.since_ms, 5_001);
        assert_eq!(arrived.skipped, 0);
    }

    fn sway() -> LookSway {
        LookSway {
            mode: LOOK_MODE,
            heading: 0.0,
            stored_heading: 4.0,
            offset: 0.0,
            look: 9.0,
            timer_ms: 1_000,
            roll: 1,
        }
    }

    #[test]
    fn a_wander_look_eases_toward_the_heading_and_draws_an_offset() {
        let mut still = sway();
        still.mode = 0;
        still.timer_ms = 0;
        sway_look(&mut still, 1.0, 0.0, 5_000, 1.0);
        assert_eq!(still.stored_heading, 4.0);
        assert_eq!(still.look, 9.0);
        assert_eq!(still.offset, 0.0);
        assert_eq!(still.roll, 1);

        let mut held = sway();
        sway_look(&mut held, 1.0, 0.0, 1_000, 1.0);
        assert_eq!(held.stored_heading, 0.0);
        assert_eq!(held.offset, 0.0);
        assert_eq!(held.roll, 1);
        assert_eq!(held.timer_ms, 1_000);
        assert!((held.look - 0.91).abs() < 1e-5);

        let mut drawn = sway();
        sway_look(&mut drawn, 0.0, 0.0, 1_001, 1.0);
        assert!((drawn.offset - 1.8).abs() < 1e-6);
        assert_eq!(drawn.roll, 134_775_814);
        assert_eq!(drawn.timer_ms, 5_126);
        assert!((drawn.look - 0.162).abs() < 1e-5);

        let mut wide = sway();
        wide.heading = 4.0;
        wide.timer_ms = 2_000;
        sway_look(&mut wide, 0.0, 1.0, 2_000, 0.0);
        assert!((wide.look - -0.06849556).abs() < 1e-5);

        let mut edge = sway();
        edge.heading = 3.1415927;
        edge.timer_ms = 2_000;
        sway_look(&mut edge, 0.0, 0.0, 2_000, 0.0);
        assert!((edge.look - 0.09 * 3.1415927).abs() < 1e-5);
    }
}
