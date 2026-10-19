//! A path walker at a door.
//!
//! With a live waypoint that is a door, openness at most 0.7 holds the
//! walker. If the door is not already toggling, the walker toggles it.
//! No path, a cursor at the end, a missing door, or a door more than 0.7
//! open lets the walker continue.
//!
//! Evidence: the path step that meets a door. The rest of the path is not
//! in this slice.

use exa_physics::door::{self, SignalDrive, SignalLeaf};

/// Where the walker is on its path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Walk {
    /// Negative when the walker has no path.
    pub path: i32,
    pub cursor: i32,
    pub waypoints: i32,
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
}
