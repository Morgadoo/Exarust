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
//! plus an offset. An aim snap copies the body's place, takes its heading's
//! yaw, and asks for the idle stance. A searched path is followed when it
//! has two points and is forced, or when its ends are more than 150 cm
//! apart. In a scene, the goal is 100 cm past the ground point's
//! projection on that path. Out of the scene, the body slides 2 cm along
//! the path. Near the end, the run bit clears and a close body soft-stops.
//! In the scene, a blocked line to the carrot clears the run bit and asks
//! for a re-plan. The other phases are not in this slice.

use bevy::math::Vec2;
use bevy::math::Vec3;
use exa_anim::Animator;
use exa_combat::stance::{self, Stance};
use exa_physics::door::{self, SignalDrive, SignalLeaf};
use exa_world::nav::{self, NavGrid};

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

/// Where an aim snap writes the body's place and a cleared pace.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Aim {
    pub place: Vec3,
    /// The heading's yaw, also written beside the stored heading.
    pub base_heading: f32,
    /// Cleared to 0. What else reads this word was not established.
    pub pace: i32,
}

/// The body an aim snap reads: its frame origin and its heading vector.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BodyFrame {
    pub place: Vec3,
    /// Heading vector. Its height is not part of the yaw.
    pub heading: Vec3,
    /// Cleared to 0. What the byte means was not established.
    pub marker: u8,
}

/// Copy the body's place, take the yaw of its heading, and ask for idle.
///
/// The heading, the stored heading and the base heading all become that
/// yaw. The roll steps by `roll * 0x8088405 + 1`. The timer, the same word
/// the wander look reads, becomes the high half of that step times 3000,
/// plus the clock, plus 1000. The pace word and the body's marker are
/// cleared. The stance becomes idle through the stance helper: the old
/// stance's clip starts Idle over 30 frames, and any other clip stays.
pub fn snap_aim(
    aim: &mut Aim,
    sway: &mut LookSway,
    body: &mut BodyFrame,
    stance: &mut Stance,
    anim: &mut Animator,
    now_ms: u32,
) {
    aim.place = body.place;
    let yaw = body.heading.x.atan2(body.heading.z);
    sway.heading = yaw;
    sway.stored_heading = yaw;
    aim.base_heading = yaw;
    sway.roll = sway.roll.wrapping_mul(0x808_8405).wrapping_add(1);
    let gap = ((u64::from(sway.roll).wrapping_mul(3_000)) >> 32) as u32;
    sway.timer_ms = gap.wrapping_add(now_ms).wrapping_add(1_000);
    aim.pace = 0;
    body.marker = 0;
    stance::set_stance(stance, anim, Stance::Idle);
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

/// A searched path is followed when it is long enough, or forced.
///
/// Fewer than two points is never followed. Otherwise a forced path is
/// followed, and an unforced path is followed only when its first and last
/// points are more than 150 cm apart.
pub fn start_follow(points: &[Vec3], forced: bool) -> bool {
    if points.len() < 2 {
        return false;
    }
    forced || (points[0] - points[points.len() - 1]).length() > 150.0
}

/// A point 100 cm past where the ground point projects onto a path.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PathCarrot {
    /// End index of the nearest segment. Segments start at 1.
    pub segment: u32,
    /// Unclamped horizontal parameter of that segment.
    pub fraction: f32,
    /// The point 100 cm further along, or none when that step has no length.
    pub goal: Option<Vec3>,
}

/// Horizontal length below this is not a segment.
const SEGMENT_EPS: f32 = f32::from_bits(0x33ff_ffd7);
const CARROT_CM: f32 = 100.0;

/// Project `ground` onto `points` and step 100 cm ahead.
///
/// Fewer than two points has no goal. A segment with no horizontal length
/// is skipped. The stored fraction is unclamped. The 100 cm is measured
/// from that foot, and spills onto later segments. A step that lands on a
/// joint has no goal. The cursor stays the projection.
pub fn carrot(points: &[Vec3], ground: Vec3) -> PathCarrot {
    if points.len() < 2 {
        return PathCarrot {
            segment: 0,
            fraction: 0.0,
            goal: None,
        };
    }
    let mut best = f32::MAX;
    let mut segment = 1u32;
    let mut fraction = 0.0;
    let mut chosen = 0.0;
    let mut found = false;
    for k in 1..points.len() {
        let start = points[k - 1];
        let v = points[k] - start;
        let horizontal = v.x * v.x + v.z * v.z;
        if horizontal < SEGMENT_EPS {
            continue;
        }
        let t = (v.x * (ground.x - start.x) + v.z * (ground.z - start.z)) / horizontal;
        let clamped = t.clamp(0.0, 1.0);
        let projected = start + v * clamped;
        let distance = (projected - ground).length_squared();
        if distance < best {
            best = distance;
            segment = k as u32;
            fraction = t;
            chosen = horizontal;
            found = true;
        }
    }
    if !found || chosen < SEGMENT_EPS {
        return PathCarrot {
            segment,
            fraction,
            goal: None,
        };
    }
    let length = chosen.sqrt();
    let (end, step) = if length > fraction * length + CARROT_CM {
        (segment as usize, (fraction * length + CARROT_CM) / length)
    } else {
        let mut left = CARROT_CM - (1.0 - fraction) * length;
        let mut end = points.len() - 1;
        let mut step = 1.0;
        for k in (segment as usize + 1)..points.len() {
            let v = points[k] - points[k - 1];
            let horizontal = v.x * v.x + v.z * v.z;
            if horizontal < SEGMENT_EPS {
                continue;
            }
            let span = horizontal.sqrt();
            if left < span {
                step = left / span;
                end = k;
                break;
            }
            left -= span;
        }
        (end, step)
    };
    if step.abs() < SEGMENT_EPS {
        return PathCarrot {
            segment,
            fraction,
            goal: None,
        };
    }
    let start = points[end - 1];
    PathCarrot {
        segment,
        fraction,
        goal: Some(start + (points[end] - start) * step),
    }
}

/// Where a body stands after one out-of-scene slide.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PathSlide {
    /// End index of the segment the body is on. Segments start at 1.
    pub segment: u32,
    /// Unclamped parameter on that segment. It can be negative.
    pub fraction: f32,
    /// The path point. Its height is the path height.
    pub ground: Vec3,
    /// The standing point. Its height is the path height plus the pelvis base.
    pub standing: Vec3,
}

/// One step of path following.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PathPace {
    /// Fewer than two points, or a segment index with no span.
    Stopped,
    /// A low scene bit is set, so the slide does not run.
    InScene,
    /// The body moved 2 cm along the path.
    Slide(PathSlide),
}

const SLIDE_CM: f32 = 2.0;
/// Stand-in length when a segment has no horizontal span.
const SHORT_SPAN: f32 = 0.1;

fn place(points: &[Vec3], segment: u32, fraction: f32, pelvis: f32) -> PathSlide {
    let start = points[segment as usize - 1];
    let end = points[segment as usize];
    let ground = start + (end - start) * fraction;
    let mut standing = ground;
    standing.y += pelvis;
    PathSlide {
        segment,
        fraction,
        ground,
        standing,
    }
}

/// Slide 2 cm along `points` from `segment` and `fraction`.
///
/// The low two scene bits are not read here. A horizontal length at or
/// below the tiny epsilon counts as 0.1. The step stays on this segment
/// only when that length is strictly past the fraction times the length
/// plus 2 cm. Otherwise it crosses one segment, and the new fraction is
/// `(1 − (1 − fraction) · length) · (1 − fraction) − 2`, divided by the
/// next length. On the last segment that step stops at the end.
pub fn slide(points: &[Vec3], segment: u32, fraction: f32, pelvis: f32) -> Option<PathSlide> {
    if points.len() < 2 || segment < 1 || segment as usize >= points.len() {
        return None;
    }
    let here = points[segment as usize] - points[segment as usize - 1];
    let horizontal = here.x * here.x + here.z * here.z;
    let length = if horizontal <= SEGMENT_EPS {
        SHORT_SPAN
    } else {
        horizontal.sqrt()
    };
    if length <= fraction * length + SLIDE_CM {
        let next = segment as usize + 1;
        if next >= points.len() {
            return Some(place(points, segment, 1.0, pelvis));
        }
        let ahead = 1.0 - (1.0 - fraction) * length;
        let onward = points[next] - points[segment as usize];
        let span = (onward.x * onward.x + onward.z * onward.z).sqrt();
        if span == 0.0 {
            return Some(place(points, segment, 1.0, pelvis));
        }
        let fraction = (ahead * (1.0 - fraction) - SLIDE_CM) / span;
        Some(place(points, segment + 1, fraction, pelvis))
    } else {
        let fraction = (fraction * length + SLIDE_CM) / length;
        Some(place(points, segment, fraction, pelvis))
    }
}

/// Follow one step. The slide runs only when the low two scene bits are clear.
pub fn follow_path(
    points: &[Vec3],
    segment: u32,
    fraction: f32,
    pelvis: f32,
    scene: u32,
) -> PathPace {
    if points.len() < 2 {
        return PathPace::Stopped;
    }
    if scene & 3 != 0 {
        return PathPace::InScene;
    }
    match slide(points, segment, fraction, pelvis) {
        Some(moved) => PathPace::Slide(moved),
        None => PathPace::Stopped,
    }
}

/// Run is bit 2 of the locomotion flags. Stopping is bit 5.
const LOCO_RUN: u32 = 4;
const LOCO_STOP: u32 = 0x20;
/// The carrot fraction is promoted before it is compared with this.
const RUN_UNTIL: f64 = 0.99;
const STOP_AT_CARROT: f32 = 900.0;
const STOP_AT_END: f32 = 10_000.0;

/// What a body is doing as it nears the end of a path.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Approach {
    pub flags: u32,
    pub urgency: i32,
    /// The carrot lies on the last segment.
    pub on_last: bool,
    pub carrot_fraction: f32,
    /// The ground point.
    pub ground: Vec3,
    /// The point 100 cm ahead.
    pub carrot: Vec3,
    /// The last point of the path.
    pub last: Vec3,
    /// The standing point. The soft stop measures the carrot from here.
    pub standing: Vec3,
    /// Frame forward. A far carrot is pulled back along it.
    pub forward: Vec3,
    /// Speed along the heading, cm per tick.
    pub speed: f32,
    pub free_walk: bool,
    pub following: bool,
}

/// Flags, the carrot, and whether a soft stop fired.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Arrival {
    pub flags: u32,
    pub free_walk: bool,
    pub following: bool,
    pub carrot: Vec3,
    pub stopped: bool,
}

fn soft_stop(approach: &Approach, flags: u32) -> Arrival {
    let limit = approach.speed * 8.0 + 20.0;
    let carrot = if limit * limit < approach.standing.distance_squared(approach.carrot) {
        approach.standing + approach.forward * limit
    } else {
        approach.carrot
    };
    Arrival {
        flags: flags & !LOCO_RUN | LOCO_STOP,
        free_walk: false,
        following: false,
        carrot,
        stopped: true,
    }
}

/// Set or clear the run bit, and soft-stop when the body is close enough.
///
/// Urgency of 0 or less keeps the run bit until the carrot fraction, promoted
/// to a double, is past 0.99 on the last segment. Past that, the run bit
/// clears, and a 3-D squared distance under 900 from the ground to the carrot
/// soft-stops. Urgency of 1 or more clears the run bit and soft-stops when
/// the ground is under 100 cm of the last point, also in 3-D. A soft stop
/// sets the stopping bit, clears free walk and following, and pulls a carrot
/// farther than 8·speed + 20 back along the facing.
pub fn arrive(approach: Approach) -> Arrival {
    let keep = Arrival {
        flags: approach.flags,
        free_walk: approach.free_walk,
        following: approach.following,
        carrot: approach.carrot,
        stopped: false,
    };
    if approach.urgency <= 0 {
        let past =
            approach.carrot_fraction.is_finite() && f64::from(approach.carrot_fraction) > RUN_UNTIL;
        if approach.on_last && past {
            let flags = approach.flags & !LOCO_RUN;
            if approach.ground.distance_squared(approach.carrot) < STOP_AT_CARROT {
                soft_stop(&approach, flags)
            } else {
                Arrival { flags, ..keep }
            }
        } else {
            Arrival {
                flags: approach.flags | LOCO_RUN,
                ..keep
            }
        }
    } else {
        let flags = approach.flags & !LOCO_RUN;
        if approach.ground.distance_squared(approach.last) < STOP_AT_END {
            soft_stop(&approach, flags)
        } else {
            Arrival { flags, ..keep }
        }
    }
}

/// The run bit and re-plan flag after walking from the ground to the carrot.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LineReplan {
    pub flags: u32,
    pub replan: bool,
    /// The carrot. A blocked walk does not move it.
    pub carrot: Vec3,
}

/// In the scene, walk the line from `ground` to `carrot`. A blocked walk
/// clears the run bit and asks for a re-plan. Out of the scene the line is
/// not walked. The carrot is left where it was.
pub fn line_replan(
    grid: &NavGrid,
    ground: Vec3,
    carrot: Vec3,
    flags: u32,
    scene: u32,
    replan: bool,
) -> LineReplan {
    if scene & 3 == 0 {
        return LineReplan {
            flags,
            replan,
            carrot,
        };
    }
    let hit = nav::line_walk(grid, ground, carrot);
    if hit.blocked {
        LineReplan {
            flags: flags & !LOCO_RUN,
            replan: true,
            carrot,
        }
    } else {
        LineReplan {
            flags,
            replan,
            carrot,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::math::{Vec2, Vec3};
    use exa_anim::{Animator, Ease};
    use exa_combat::stance::Stance;
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

    fn frame(heading: Vec3) -> BodyFrame {
        BodyFrame {
            place: Vec3::new(10.0, 20.0, 30.0),
            heading,
            marker: 1,
        }
    }

    fn fresh_aim() -> Aim {
        Aim {
            place: Vec3::ZERO,
            base_heading: 4.0,
            pace: 7,
        }
    }

    #[test]
    fn an_aim_snap_takes_the_bodys_yaw_and_asks_for_idle() {
        let mut aim = fresh_aim();
        let mut look = sway();
        look.offset = 0.4;
        let mut body = frame(Vec3::new(0.0, 9.0, 1.0));
        let mut stance = Stance::Defence;
        let mut anim = Animator::new("Default", "Defence stance");
        snap_aim(
            &mut aim,
            &mut look,
            &mut body,
            &mut stance,
            &mut anim,
            2_000,
        );
        assert_eq!(aim.place, Vec3::new(10.0, 20.0, 30.0));
        assert!(look.heading.abs() < 1e-6);
        assert!(look.stored_heading.abs() < 1e-6);
        assert!(aim.base_heading.abs() < 1e-6);
        assert_eq!(look.roll, 134_775_814);
        assert_eq!(look.timer_ms, 3_094);
        assert_eq!(aim.pace, 0);
        assert_eq!(body.marker, 0);
        assert_eq!(stance, Stance::Idle);
        assert_eq!(anim.action, "Idle");
        assert_eq!((anim.blend, anim.ease), (30.0, Ease::Smooth));

        sway_look(&mut look, 0.0, 0.0, 2_000, 1.0);
        assert!((look.offset - 0.4).abs() < 1e-6);
        assert_eq!(look.roll, 134_775_814);
        assert_eq!(look.timer_ms, 3_094);

        let mut aim = fresh_aim();
        let mut look = sway();
        let mut body = frame(Vec3::X);
        let mut stance = Stance::Defence;
        let mut anim = Animator::new("Default", "Walk");
        snap_aim(
            &mut aim,
            &mut look,
            &mut body,
            &mut stance,
            &mut anim,
            2_000,
        );
        assert!((look.heading - 1.5707964).abs() < 1e-5);
        assert!((aim.base_heading - look.heading).abs() < 1e-6);
        assert_eq!(stance, Stance::Idle);
        assert_eq!(anim.action, "Walk");
        assert_eq!(anim.blend, 0.0);
        assert_eq!(body.marker, 0);
        assert_eq!(aim.pace, 0);

        let mut aim = fresh_aim();
        let mut look = sway();
        let mut body = frame(Vec3::new(0.0, 0.0, -1.0));
        let mut stance = Stance::Idle;
        let mut anim = Animator::new("Default", "Idle");
        snap_aim(
            &mut aim,
            &mut look,
            &mut body,
            &mut stance,
            &mut anim,
            2_000,
        );
        assert!((look.heading - 3.1415927).abs() < 1e-5);
        assert_eq!(anim.action, "Idle");
        assert_eq!(anim.blend, 0.0);
        assert_eq!(anim.ease, Ease::Linear);
        assert_eq!(stance, Stance::Idle);
    }

    #[test]
    fn a_path_is_followed_when_its_ends_are_past_one_hundred_fifty_centimetres() {
        assert!(!start_follow(&[], true));
        assert!(!start_follow(&[Vec3::ZERO], true));
        assert!(!start_follow(
            &[Vec3::ZERO, Vec3::new(150.0, 0.0, 0.0)],
            false
        ));
        assert!(start_follow(
            &[Vec3::ZERO, Vec3::new(0.0, 151.0, 0.0)],
            false
        ));
        assert!(start_follow(&[Vec3::ZERO, Vec3::ZERO], true));
    }

    #[test]
    fn a_carrot_is_one_hundred_centimetres_past_the_projection() {
        let behind = carrot(
            &[Vec3::ZERO, Vec3::new(0.0, 0.0, 200.0)],
            Vec3::new(0.0, 0.0, -50.0),
        );
        assert_eq!(behind.segment, 1);
        assert_eq!(behind.fraction, -0.25);
        assert_eq!(behind.goal, Some(Vec3::new(0.0, 0.0, 50.0)));

        let corner = carrot(
            &[
                Vec3::ZERO,
                Vec3::new(0.0, 0.0, 80.0),
                Vec3::new(0.0, 40.0, 280.0),
            ],
            Vec3::new(0.0, 0.0, 80.0),
        );
        assert_eq!(corner.segment, 1);
        assert_eq!(corner.fraction, 1.0);
        assert_eq!(corner.goal, Some(Vec3::new(0.0, 20.0, 180.0)));

        let raised = carrot(
            &[
                Vec3::ZERO,
                Vec3::new(10.0, 0.0, 0.0),
                Vec3::new(10.0, 100.0, 0.0),
                Vec3::new(10.0, 100.0, 10.0),
            ],
            Vec3::new(10.0, 100.0, 0.0),
        );
        assert_eq!(raised.segment, 3);
        assert_eq!(raised.fraction, 0.0);
        assert_eq!(raised.goal, Some(Vec3::new(10.0, 100.0, 10.0)));

        let joint = carrot(
            &[
                Vec3::ZERO,
                Vec3::new(0.0, 0.0, 100.0),
                Vec3::new(0.0, 0.0, 200.0),
            ],
            Vec3::ZERO,
        );
        assert_eq!(joint.segment, 1);
        assert_eq!(joint.fraction, 0.0);
        assert_eq!(joint.goal, None);

        assert_eq!(carrot(&[Vec3::ZERO], Vec3::ZERO).goal, None);
    }

    #[test]
    fn an_out_of_scene_body_slides_two_centimetres() {
        let line = [Vec3::ZERO, Vec3::new(0.0, 0.0, 100.0)];
        let moved = follow_path(&line, 1, 0.0, 90.0, 0);
        assert_eq!(
            moved,
            PathPace::Slide(PathSlide {
                segment: 1,
                fraction: 0.02,
                ground: Vec3::new(0.0, 0.0, 2.0),
                standing: Vec3::new(0.0, 90.0, 2.0),
            })
        );
        assert!(matches!(
            follow_path(&line, 1, 0.0, 90.0, 1),
            PathPace::InScene
        ));
        assert!(matches!(
            follow_path(&line, 1, 0.0, 90.0, 3),
            PathPace::InScene
        ));
        assert!(matches!(
            follow_path(&line, 1, 0.0, 90.0, 4),
            PathPace::Slide(_)
        ));

        let bend = [
            Vec3::ZERO,
            Vec3::new(0.0, 0.0, 8.0),
            Vec3::new(0.0, 0.0, 16.0),
        ];
        let crossed = follow_path(&bend, 1, 0.75, 0.0, 0);
        assert_eq!(
            crossed,
            PathPace::Slide(PathSlide {
                segment: 2,
                fraction: -0.28125,
                ground: Vec3::new(0.0, 0.0, 5.75),
                standing: Vec3::new(0.0, 0.0, 5.75),
            })
        );

        let risen = [
            Vec3::ZERO,
            Vec3::new(0.0, 10.0, 0.0),
            Vec3::new(0.0, 10.0, 20.0),
        ];
        let vertical = follow_path(&risen, 1, 0.0, 4.0, 0);
        assert_eq!(
            vertical,
            PathPace::Slide(PathSlide {
                segment: 2,
                fraction: f32::from_bits(0xbd61_47ae),
                ground: Vec3::new(0.0, 10.0, f32::from_bits(0xbf8c_cccd)),
                standing: Vec3::new(0.0, 14.0, f32::from_bits(0xbf8c_cccd)),
            })
        );

        let end = follow_path(&line, 1, 0.99, 0.0, 0);
        assert_eq!(
            end,
            PathPace::Slide(PathSlide {
                segment: 1,
                fraction: 1.0,
                ground: Vec3::new(0.0, 0.0, 100.0),
                standing: Vec3::new(0.0, 0.0, 100.0),
            })
        );
        assert!(matches!(
            follow_path(&[], 1, 0.0, 0.0, 0),
            PathPace::Stopped
        ));
    }

    fn approach(urgency: i32, on_last: bool, fraction: f32) -> Approach {
        Approach {
            flags: 1,
            urgency,
            on_last,
            carrot_fraction: fraction,
            ground: Vec3::ZERO,
            carrot: Vec3::ZERO,
            last: Vec3::new(0.0, 0.0, 200.0),
            standing: Vec3::ZERO,
            forward: Vec3::Z,
            speed: 0.0,
            free_walk: true,
            following: true,
        }
    }

    #[test]
    fn a_run_clears_past_the_end_and_a_close_body_soft_stops() {
        let early = arrive(approach(0, false, 0.0));
        assert_eq!(early.flags, 1 | LOCO_RUN);
        assert!(!early.stopped);
        assert!(early.following);

        let under = arrive(approach(0, true, f32::from_bits(0x3f7d_70a3)));
        assert_eq!(under.flags, 1 | LOCO_RUN);
        assert!(!under.stopped);

        let mut at = approach(0, true, 0.99);
        at.carrot = Vec3::new(0.0, 0.0, 10.0);
        let close = arrive(at);
        assert!(close.stopped);
        assert_eq!(close.flags & LOCO_RUN, 0);
        assert_eq!(close.flags & LOCO_STOP, LOCO_STOP);
        assert!(!close.free_walk);
        assert!(!close.following);
        assert_eq!(close.carrot, Vec3::new(0.0, 0.0, 10.0));

        let mut exact = approach(0, true, 1.0);
        exact.carrot = Vec3::new(30.0, 0.0, 0.0);
        let held = arrive(exact);
        assert!(!held.stopped);
        assert_eq!(held.flags, 1);
        assert!(held.following);

        let mut high = approach(0, true, 1.0);
        high.carrot = Vec3::new(0.0, 40.0, 0.0);
        assert!(!arrive(high).stopped);

        let mut raised = approach(0, true, 1.0);
        raised.standing = Vec3::new(0.0, 100.0, 0.0);
        let pulled = arrive(raised);
        assert!(pulled.stopped);
        assert_eq!(pulled.carrot, Vec3::new(0.0, 100.0, 20.0));

        let mut near_end = approach(1, false, 0.0);
        near_end.last = Vec3::new(99.0, 0.0, 0.0);
        let sprint = arrive(near_end);
        assert!(sprint.stopped);
        assert_eq!(sprint.flags & LOCO_RUN, 0);

        let mut far_end = approach(1, false, 0.0);
        far_end.last = Vec3::new(100.0, 0.0, 0.0);
        let jogging = arrive(far_end);
        assert!(!jogging.stopped);
        assert_eq!(jogging.flags, 1);

        let back = arrive(approach(-1, false, 0.0));
        assert_eq!(back.flags, 1 | LOCO_RUN);
        assert!(!back.stopped);
    }

    fn yard(wall: bool) -> (NavGrid, Vec3, Vec3) {
        let mut grid = NavGrid::empty(8, 8, 0.0);
        for z in 0..8 {
            for x in 0..8 {
                grid.set_cell(
                    x,
                    z,
                    nav::Layer::Lower,
                    nav::NavCell {
                        floor: 10,
                        ..nav::NavCell::default()
                    },
                );
            }
        }
        if wall {
            grid.set_cell(
                4,
                3,
                nav::Layer::Lower,
                nav::NavCell {
                    floor: 40,
                    underside: 0,
                    ..nav::NavCell::default()
                },
            );
        }
        let from = grid.cell_centre(1, 3);
        let to = grid.cell_centre(6, 3);
        (
            grid,
            Vec3::new(from.x, 1.0, from.y),
            Vec3::new(to.x, 1.0, to.y),
        )
    }

    #[test]
    fn a_blocked_line_to_the_carrot_asks_for_a_replan() {
        let (open, ground, carrot) = yard(false);
        let clear = line_replan(&open, ground, carrot, 1 | LOCO_RUN, 1, false);
        assert_eq!(
            clear,
            LineReplan {
                flags: 1 | LOCO_RUN,
                replan: false,
                carrot,
            }
        );

        let (walled, ground, carrot) = yard(true);
        let blocked = line_replan(&walled, ground, carrot, 1 | LOCO_RUN, 1, false);
        assert_eq!(
            blocked,
            LineReplan {
                flags: 1,
                replan: true,
                carrot,
            }
        );

        let outside = line_replan(&walled, ground, carrot, 1 | LOCO_RUN, 0, false);
        assert_eq!(
            outside,
            LineReplan {
                flags: 1 | LOCO_RUN,
                replan: false,
                carrot,
            }
        );
        let higher = line_replan(&walled, ground, carrot, 1 | LOCO_RUN, 4, true);
        assert_eq!(higher.flags, 1 | LOCO_RUN);
        assert!(higher.replan);
    }
}

/// The eight neighbour offsets in the original's order: north-west,
/// north, north-east, west, east, south-west, south, south-east.
pub const PATH_NEIGHBOURS: [(i32, i32); 8] = [
    (-1, -1),
    (-1, 0),
    (-1, 1),
    (0, -1),
    (0, 1),
    (1, -1),
    (1, 0),
    (1, 1),
];

/// The move cost of each neighbour: diagonals 7, orthogonals 5.
pub const PATH_MOVE_COST: [i32; 8] = [7, 5, 7, 5, 5, 7, 5, 7];

/// A world coordinate to a grid cell: `trunc(world / unit + 0.5)`.
pub fn path_cell_coord(world: f32, inverse_unit: f32) -> i32 {
    (world * inverse_unit + 0.5) as i32
}

/// A grid cell to a world coordinate: `(cell − 0.5) · unit`.
pub fn path_world_coord(cell: i32, unit: f32) -> f32 {
    (cell as f32 - 0.5) * unit
}

/// Whether a step may enter a cell: the next floor is set, its rise over
/// the current floor is −20 through +4, the A byte is at most
/// `180/(quality+1)` and the shifted B byte at most 600.
pub fn path_step_ok(
    current_floor: i32,
    next_floor: u8,
    a: u8,
    b: u8,
    quality: i32,
    shift: u32,
) -> bool {
    let rise = i32::from(next_floor) - current_floor;
    let quality = quality.max(0) + 1;
    next_floor != 0
        && (-20..=4).contains(&rise)
        && i32::from(a) <= 180 / quality
        && ((i32::from(b)) << shift) <= 600
}

/// The step cost: `g + 10·dz² + direction + 2·A + (B << shift)`.
pub fn path_step_cost(current_g: i32, dz: i32, direction: usize, a: u8, b: u8, shift: u32) -> i32 {
    current_g
        + 10 * dz * dz
        + PATH_MOVE_COST[direction]
        + 2 * i32::from(a)
        + ((i32::from(b)) << shift)
}

/// The heuristic: ten times the 3D distance, truncated.
pub fn path_heuristic(dx: i32, dy: i32, dz: i32) -> i32 {
    let squared = f64::from(dx * dx + dy * dy + dz * dz);
    (10.0 * squared.sqrt()) as i32
}

#[cfg(test)]
mod grid_cost_tests {
    use super::*;

    #[test]
    fn cells_and_worlds_round_with_a_half_cell_shift() {
        assert_eq!(path_cell_coord(0.5, 1.0), 1);
        assert_eq!(path_cell_coord(1.4, 1.0), 1);
        assert_eq!(path_cell_coord(-0.5, 1.0), 0);
        assert_eq!(path_world_coord(1, 1.0), 0.5);
        assert_eq!(path_world_coord(0, 1.0), -0.5);
    }

    #[test]
    fn a_step_checks_the_floor_rise_and_the_two_penalties() {
        assert!(path_step_ok(10, 12, 50, 0, 0, 0));
        assert!(!path_step_ok(10, 0, 50, 0, 0, 0));
        assert!(!path_step_ok(10, 32, 50, 0, 0, 0));
        assert!(!path_step_ok(10, 15, 50, 0, 0, 0));
        assert!(!path_step_ok(10, 12, 181, 0, 0, 0));
        assert!(!path_step_ok(10, 12, 50, 100, 0, 3));
    }

    #[test]
    fn the_cost_and_heuristic_follow_the_equation() {
        assert_eq!(path_step_cost(100, 2, 3, 10, 4, 1), 173);
        assert_eq!(path_heuristic(3, 4, 0), 50);
        assert_eq!(PATH_MOVE_COST, [7, 5, 7, 5, 5, 7, 5, 7]);
    }
}

/// The pathfinder's grid for one floor layer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathGrid {
    pub width: i32,
    pub height: i32,
    pub floor: Vec<u8>,
    pub a: Vec<u8>,
    pub b: Vec<u8>,
}

impl PathGrid {
    fn index(&self, x: i32, y: i32) -> Option<usize> {
        if x < 0 || y < 0 || x >= self.width || y >= self.height {
            return None;
        }
        Some((y * self.width + x) as usize)
    }
}

/// The search's query.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PathRequest {
    pub start: (i32, i32),
    pub goal: (i32, i32),
    pub goal_floor: i32,
    pub quality: i32,
    pub shift: u32,
    /// A best node nearer than this ends the search early; 0 never does.
    pub accept: i32,
}

/// The search's result: the parent chain from the start, whether the goal
/// cell was reached, and how many nodes were made.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathSearch {
    pub path: Vec<(i32, i32)>,
    pub reached: bool,
    pub nodes: usize,
}

struct SearchNode {
    x: i32,
    y: i32,
    g: i32,
    f: i32,
    parent: usize,
}

/// Searches the grid: a LIFO open list popped by the smallest f (the
/// newest wins a tie), at most 4095 nodes, the acceptance distance, and a
/// parent chain collapsed through collinear runs. The two-layer rule and
/// the line-walk shortcut are not modelled.
pub fn search_path(grid: &PathGrid, request: &PathRequest) -> PathSearch {
    let Some(start) = grid.index(request.start.0, request.start.1) else {
        return PathSearch {
            path: Vec::new(),
            reached: false,
            nodes: 0,
        };
    };
    if grid.index(request.goal.0, request.goal.1).is_none() {
        return PathSearch {
            path: Vec::new(),
            reached: false,
            nodes: 0,
        };
    }
    let dx = request.goal.0 - request.start.0;
    let dy = request.goal.1 - request.start.1;
    if dx.abs() > 253 || dy.abs() > 253 {
        return PathSearch {
            path: Vec::new(),
            reached: false,
            nodes: 0,
        };
    }

    let start_floor = i32::from(grid.floor[start]);
    let pseudo = f64::from(grid.a[start]) / 3.0;
    let start_f = path_heuristic(dx, dy, 0).max(0) + (10.0 * pseudo) as i32;
    let mut nodes = vec![
        SearchNode {
            x: 0,
            y: 0,
            g: 0,
            f: 0,
            parent: 0,
        },
        SearchNode {
            x: request.start.0,
            y: request.start.1,
            g: 0,
            f: start_f,
            parent: 0,
        },
    ];
    let mut stamps = vec![0u16; (grid.width * grid.height) as usize];
    stamps[start] = 2;
    let mut open = vec![1usize];
    let mut best = 1usize;
    let mut best_h = start_h(request, start_floor);

    while !open.is_empty() && nodes.len() < 0xfff {
        let mut chosen = 0;
        for index in (0..open.len()).rev() {
            if nodes[open[index]].f < nodes[open[chosen]].f {
                chosen = index;
            }
        }
        let current = open.swap_remove(chosen);
        let current_floor =
            i32::from(grid.floor[grid.index(nodes[current].x, nodes[current].y).unwrap()]);
        if request.accept > best_h {
            break;
        }
        for (direction, &(ny, nx)) in PATH_NEIGHBOURS.iter().enumerate() {
            let x = nodes[current].x + nx;
            let y = nodes[current].y + ny;
            let Some(index) = grid.index(x, y) else {
                continue;
            };
            let next_floor = grid.floor[index];
            if !path_step_ok(
                current_floor,
                next_floor,
                grid.a[index],
                grid.b[index],
                request.quality,
                request.shift,
            ) {
                continue;
            }
            let dz = i32::from(next_floor) - current_floor;
            let g = path_step_cost(
                nodes[current].g,
                dz,
                direction,
                grid.a[index],
                grid.b[index],
                request.shift,
            );
            let h = path_heuristic(
                request.goal.0 - x,
                request.goal.1 - y,
                i32::from(next_floor) - request.goal_floor,
            );
            let f = g + h;
            let state = stamps[index];
            if state == 0 {
                let node = nodes.len();
                nodes.push(SearchNode {
                    x,
                    y,
                    g,
                    f,
                    parent: current,
                });
                stamps[index] = node as u16 + 1;
                open.push(node);
            } else if state != 0xffff {
                let node = state as usize - 1;
                if f < nodes[node].f {
                    nodes[node].f = f;
                    nodes[node].g = g;
                    nodes[node].parent = current;
                }
            } else {
                continue;
            }
            if h < best_h {
                best_h = h;
                best = stamps[index].saturating_sub(1) as usize;
            }
        }
    }

    let mut points = Vec::new();
    let mut cursor = nodes[best].parent;
    points.push((nodes[best].x, nodes[best].y));
    while cursor != 0 {
        points.push((nodes[cursor].x, nodes[cursor].y));
        cursor = nodes[cursor].parent;
    }
    points.reverse();
    let mut collapsed: Vec<(i32, i32)> = Vec::with_capacity(points.len());
    for point in points {
        if collapsed.len() >= 2 {
            let previous = collapsed[collapsed.len() - 1];
            let before = collapsed[collapsed.len() - 2];
            let same = (point.0 - previous.0, point.1 - previous.1);
            let prior = (previous.0 - before.0, previous.1 - before.1);
            if same.0.signum() == prior.0.signum() && same.1.signum() == prior.1.signum() {
                collapsed.pop();
            }
        }
        collapsed.push(point);
    }
    PathSearch {
        path: collapsed,
        reached: nodes[best].x == request.goal.0 && nodes[best].y == request.goal.1,
        nodes: nodes.len(),
    }
}

fn start_h(request: &PathRequest, floor: i32) -> i32 {
    let dx = request.goal.0 - request.start.0;
    let dy = request.goal.1 - request.start.1;
    let dz = floor - request.goal_floor;
    path_heuristic(dx, dy, dz)
}

#[cfg(test)]
mod search_tests {
    use super::*;

    fn open_grid(width: i32, height: i32) -> PathGrid {
        PathGrid {
            width,
            height,
            floor: vec![1; (width * height) as usize],
            a: vec![0; (width * height) as usize],
            b: vec![0; (width * height) as usize],
        }
    }

    fn request(start: (i32, i32), goal: (i32, i32)) -> PathRequest {
        PathRequest {
            start,
            goal,
            goal_floor: 1,
            quality: 0,
            shift: 0,
            accept: 0,
        }
    }

    #[test]
    fn a_clear_grid_is_crossed_straight() {
        let grid = open_grid(5, 5);
        let search = search_path(&grid, &request((0, 2), (4, 2)));
        assert!(search.reached);
        assert_eq!(search.path.first(), Some(&(0, 2)));
        assert_eq!(search.path.last(), Some(&(4, 2)));
        assert!(search.path.iter().all(|&(_, y)| y == 2));
        assert!(search.path.windows(2).all(|pair| pair[0].0 < pair[1].0));
        assert!(search.nodes < 0xfff);
    }

    #[test]
    fn a_wall_is_walked_around() {
        let mut grid = open_grid(5, 5);
        for y in 1..5 {
            grid.floor[(y * 5 + 2) as usize] = 0;
        }
        let search = search_path(&grid, &request((0, 2), (4, 2)));
        assert!(search.reached);
        assert!(search.path.len() > 2);
        assert_eq!(search.path.first(), Some(&(0, 2)));
        assert_eq!(search.path.last(), Some(&(4, 2)));
        assert!(
            search
                .path
                .iter()
                .all(|&(x, y)| grid.floor[(y * 5 + x) as usize] != 0)
        );
    }

    #[test]
    fn a_sealed_goal_is_not_reached() {
        let mut grid = open_grid(5, 5);
        for (x, y) in [(3, 1), (3, 2), (3, 3), (4, 1), (4, 3)] {
            grid.floor[(y * 5 + x) as usize] = 0;
        }
        let search = search_path(&grid, &request((0, 2), (4, 2)));
        assert!(!search.reached);
    }
}

/// A grid for the snap sweep: the floor and the two penalty bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnapGrid {
    pub width: i32,
    pub height: i32,
    pub floor: Vec<u8>,
    pub a: Vec<u8>,
    pub b: Vec<u8>,
}

impl SnapGrid {
    fn index(&self, x: i32, y: i32) -> Option<usize> {
        if x < 0 || y < 0 || x >= self.width || y >= self.height {
            return None;
        }
        Some((y * self.width + x) as usize)
    }

    /// A cell is walkable when it has a floor and its two penalties sum
    /// under 80.
    pub fn walkable(&self, x: i32, y: i32) -> bool {
        match self.index(x, y) {
            Some(index) => {
                self.floor[index] != 0 && i32::from(self.a[index]) + i32::from(self.b[index]) < 80
            }
            None => false,
        }
    }
}

/// The snap's result: the nearest walkable cell, its height, and the nodes
/// visited.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SnapResult {
    pub cell: Option<(i32, i32)>,
    pub height: f32,
    pub nodes: usize,
}

/// Snaps a point to a walkable cell: the start cell when it is walkable,
/// otherwise a best-first sweep of the 32×32 window around it, in distance
/// order and up to 220 nodes. The height is the base plus the floor times
/// ten.
pub fn snap_to_walkable(grid: &SnapGrid, start: (i32, i32), base_height: f32) -> SnapResult {
    let height_of = |x: i32, y: i32| -> f32 {
        grid.index(x, y).map_or(base_height, |index| {
            base_height + f32::from(grid.floor[index]) * 10.0
        })
    };
    if grid.walkable(start.0, start.1) {
        return SnapResult {
            cell: Some(start),
            height: height_of(start.0, start.1),
            nodes: 1,
        };
    }
    let mut open = vec![start];
    let mut visited: Vec<(i32, i32)> = vec![start];
    let mut nodes = 0;
    while !open.is_empty() && nodes < 220 {
        let mut chosen = 0;
        for index in (0..open.len()).rev() {
            let a = open[index];
            let b = open[chosen];
            let da = (a.0 - start.0).pow(2) + (a.1 - start.1).pow(2);
            let db = (b.0 - start.0).pow(2) + (b.1 - start.1).pow(2);
            if da < db {
                chosen = index;
            }
        }
        let cell = open.swap_remove(chosen);
        nodes += 1;
        if grid.walkable(cell.0, cell.1) {
            return SnapResult {
                cell: Some(cell),
                height: height_of(cell.0, cell.1),
                nodes,
            };
        }
        for &(ny, nx) in &PATH_NEIGHBOURS {
            let x = cell.0 + nx;
            let y = cell.1 + ny;
            if (x - start.0).abs() > 16 || (y - start.1).abs() > 16 {
                continue;
            }
            if visited.contains(&(x, y)) {
                continue;
            }
            if grid.index(x, y).is_some() {
                visited.push((x, y));
                open.push((x, y));
            }
        }
    }
    SnapResult {
        cell: None,
        height: base_height,
        nodes,
    }
}

#[cfg(test)]
mod snap_tests {
    use super::*;

    fn grid_with_open(cell: (i32, i32)) -> SnapGrid {
        let mut grid = SnapGrid {
            width: 5,
            height: 5,
            floor: vec![1; 25],
            a: vec![90; 25],
            b: vec![0; 25],
        };
        grid.a[(cell.1 * 5 + cell.0) as usize] = 0;
        grid
    }

    #[test]
    fn a_walkable_start_is_returned_at_once() {
        let mut grid = grid_with_open((0, 0));
        grid.a[0] = 10;
        let snap = snap_to_walkable(&grid, (0, 0), 100.0);
        assert_eq!(snap.cell, Some((0, 0)));
        assert_eq!(snap.height, 110.0);
        assert_eq!(snap.nodes, 1);
    }

    #[test]
    fn a_blocked_start_finds_the_nearest_walkable_cell() {
        let grid = grid_with_open((2, 0));
        let snap = snap_to_walkable(&grid, (0, 0), 0.0);
        assert_eq!(snap.cell, Some((2, 0)));
        assert_eq!(snap.height, 10.0);
        assert!(snap.nodes <= 220);
    }

    #[test]
    fn a_fully_blocked_window_finds_nothing() {
        let grid = SnapGrid {
            width: 5,
            height: 5,
            floor: vec![1; 25],
            a: vec![90; 25],
            b: vec![0; 25],
        };
        let snap = snap_to_walkable(&grid, (2, 2), 7.0);
        assert_eq!(snap.cell, None);
        assert_eq!(snap.height, 7.0);
        assert!(snap.nodes <= 220);
    }
}

/// Shortcuts a cell path: from the last kept point, a point is dropped when
/// the direct line to the following point is clear, and kept otherwise.
/// The first and last points are always kept. `clear` answers whether the
/// line between two cells is walkable.
pub fn shortcut_path<F>(points: &[(i32, i32)], mut clear: F) -> Vec<(i32, i32)>
where
    F: FnMut((i32, i32), (i32, i32)) -> bool,
{
    if points.len() <= 2 {
        return points.to_vec();
    }
    let mut out = vec![points[0]];
    let mut index = 1;
    while index + 1 < points.len() {
        let next = index + 1;
        if clear(out[out.len() - 1], points[next]) {
            index = next;
        } else {
            out.push(points[index]);
            index += 1;
        }
    }
    out.push(points[points.len() - 1]);
    out
}

#[cfg(test)]
mod shortcut_tests {
    use super::*;

    #[test]
    fn a_clear_line_drops_the_middle_points() {
        let points = [(0, 0), (1, 0), (2, 0), (3, 0)];
        assert_eq!(shortcut_path(&points, |_, _| true), vec![(0, 0), (3, 0)]);
    }

    #[test]
    fn blocked_lines_keep_the_bends() {
        let points = [(0, 0), (1, 0), (2, 1), (3, 1)];
        let clear = |a: (i32, i32), b: (i32, i32)| {
            (a.0 - b.0).abs() + (a.1 - b.1).abs() <= 1 || a == (0, 0) && b == (2, 1)
        };
        assert_eq!(shortcut_path(&points, clear), vec![(0, 0), (2, 1), (3, 1)]);
    }

    #[test]
    fn short_paths_stay_as_they_are() {
        let points = [(0, 0), (1, 1)];
        assert_eq!(shortcut_path(&points, |_, _| true), points);
    }
}

/// The endpoint fix-up: from the end of the path, drop up to eight points
/// whose penalty byte is above `64/(quality+1)`. The first point is kept.
/// Returns the number of points that remain.
pub fn trim_path_end(points: usize, penalties: &[u8], quality: i32) -> usize {
    let threshold = 64 / (quality.max(0) + 1);
    let mut end = points.min(penalties.len());
    for _ in 0..8 {
        if end <= 1 {
            break;
        }
        if i32::from(penalties[end - 1]) > threshold {
            end -= 1;
        } else {
            break;
        }
    }
    end
}

#[cfg(test)]
mod trim_tests {
    use super::*;

    #[test]
    fn high_penalties_are_dropped_from_the_end() {
        assert_eq!(trim_path_end(5, &[0, 0, 100, 100, 100], 0), 2);
        assert_eq!(trim_path_end(2, &[0, 100], 0), 1);
        assert_eq!(trim_path_end(3, &[0, 100, 0], 0), 3);
    }

    #[test]
    fn the_trim_stops_after_eight_points() {
        assert_eq!(
            trim_path_end(
                12,
                &[0, 100, 100, 100, 100, 100, 100, 100, 100, 100, 100, 100],
                0
            ),
            4
        );
    }

    #[test]
    fn the_quality_lowers_the_threshold() {
        assert_eq!(trim_path_end(2, &[0, 40], 0), 2);
        assert_eq!(trim_path_end(2, &[0, 40], 1), 1);
    }
}

/// The budgeted shortcut: like [`shortcut_path`], but the clear predicate
/// receives the budget `costs[later] − costs[kept] + 10`.
pub fn shortcut_path_with_budget<F>(
    points: &[(i32, i32)],
    costs: &[i32],
    mut clear: F,
) -> Vec<(i32, i32)>
where
    F: FnMut((i32, i32), (i32, i32), i32) -> bool,
{
    if points.len() <= 2 {
        return points.to_vec();
    }
    let cost = |index: usize| costs.get(index).copied().unwrap_or(0);
    let mut out = vec![points[0]];
    let mut kept = 0;
    let mut index = 1;
    while index + 1 < points.len() {
        let next = index + 1;
        let budget = cost(next) - cost(kept) + 10;
        if clear(points[kept], points[next], budget) {
            index = next;
        } else {
            out.push(points[index]);
            kept = index;
            index += 1;
        }
    }
    out.push(points[points.len() - 1]);
    out
}

#[cfg(test)]
mod budget_tests {
    use super::*;

    #[test]
    fn the_budget_is_passed_to_the_line_walk() {
        let points = [(0, 0), (1, 0), (2, 0), (3, 0)];
        let costs = [0, 100, 101, 102];
        let clear = |_: (i32, i32), _: (i32, i32), budget: i32| budget <= 20;
        assert_eq!(
            shortcut_path_with_budget(&points, &costs, clear),
            vec![(0, 0), (1, 0), (3, 0)]
        );
    }

    #[test]
    fn a_clear_line_keeps_only_the_ends() {
        let points = [(0, 0), (1, 0), (2, 0), (3, 0)];
        let costs = [0, 10, 20, 30];
        assert_eq!(
            shortcut_path_with_budget(&points, &costs, |_, _, _| true),
            vec![(0, 0), (3, 0)]
        );
    }
}
