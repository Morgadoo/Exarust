//! One planner a body tick, and the path it asks the navigation grid for.
//!
//! Only the brain whose turn it is plans; every other caller records how
//! long it has waited, and the longest waiter is remembered for the next
//! turn: a wait counts only when it is longer than the longest so far
//! (unsigned, on the wrapping tick clock: the brains' tick runs once per
//! body tick, [`crate::BrainTick`]). The planner searches the
//! navigation grid from the body's feet to its goal, and starts following
//! when the path has at least two points and either the plan is forced or
//! the path's ends are more than 150 cm apart horizontally (squared,
//! `z² + x²` against 22 500); otherwise it clears the route. A downed body
//! does not plan and records no wait (its caller's test).
//!
//! The navigation grid itself is [`exa_world::nav`]'s; this module only
//! owns the turn and the start test, so there is one pathfinder.
//!
//! Evidence: the original's plan turn and its route start, read from the
//! instructions (the waiting count on the step counter and its strict
//! compare, the hand-over in the game loop's pre-tick hook list, the horizontal span in the start test).

use bevy::math::Vec3;

/// A route is followed when its ends lie farther apart than this (cm).
pub const FOLLOW_SPAN_CM: f32 = 150.0;

/// The goal a plan walks to.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Goal {
    pub point: Vec3,
    /// How close counts as arrived (cm).
    pub radius_cm: f32,
    /// The motion the follower walks in.
    pub motion: i32,
}

/// The planner's route source: the navigation grid.
pub trait Router {
    /// The path from `from` to `to` (cm): points at cell centres, empty
    /// when no path starts.
    fn route(&mut self, from: Vec3, to: Vec3) -> Vec<Vec3>;
}

/// A search result kept for following.
#[derive(Debug, Clone, PartialEq)]
pub struct Route {
    pub points: Vec<Vec3>,
    pub cursor: i32,
    pub fraction: f32,
}

/// The turn: who may plan this tick, and who has waited longest.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PlanTurn {
    /// The tick counter the waits are measured on.
    pub clock: u32,
    /// The brain whose turn it is.
    pub current: Option<u64>,
    /// The longest waiter and how long it has waited (ticks).
    pub longest: Option<u64>,
    pub longest_wait: u32,
    /// When each brain last had its turn.
    pub last: Vec<(u64, u32)>,
}

impl PlanTurn {
    /// Whether `who` plans this tick; another brain's try records its
    /// wait.
    pub fn turn(&mut self, who: u64, clock: u32) -> bool {
        if self.current == Some(who) {
            self.stamp(who, clock);
            return true;
        }
        let last = self
            .last
            .iter()
            .find(|(id, _)| *id == who)
            .map_or(0, |(_, at)| *at);
        let waited = clock.wrapping_sub(last);
        if waited > self.longest_wait {
            self.longest = Some(who);
            self.longest_wait = waited;
        }
        false
    }

    /// Gives the turn to the longest waiter (the manager's next pick).
    pub fn next_turn(&mut self) -> Option<u64> {
        self.current = self.longest;
        self.longest = None;
        self.longest_wait = 0;
        self.current
    }

    fn stamp(&mut self, who: u64, clock: u32) {
        if let Some(entry) = self.last.iter_mut().find(|(id, _)| *id == who) {
            entry.1 = clock;
        } else {
            self.last.push((who, clock));
        }
    }

    /// The last tick `who` planned.
    pub fn last_plan(&self, who: u64) -> Option<u32> {
        self.last
            .iter()
            .find(|(id, _)| *id == who)
            .map(|(_, at)| *at)
    }
}

/// What a plan asks the follower to do.
#[derive(Debug, Clone, PartialEq)]
pub struct Planned {
    pub goal: Goal,
    pub route: Route,
    /// The route is followed: at least two points, and forced or more than
    /// 150 cm end to end.
    pub following: bool,
}

/// The squared horizontal span between a path's ends, as the start test
/// sums it (`z² + x²`).
pub fn route_span2(points: &[Vec3]) -> f32 {
    match (points.first(), points.last()) {
        (Some(first), Some(last)) => {
            let dx = first.x - last.x;
            let dz = first.z - last.z;
            dz * dz + dx * dx
        }
        _ => 0.0,
    }
}

/// The path's ground length (cm): the sum of its segments' flat lengths
/// (`√(z² + x²)`).
pub fn path_length(points: &[Vec3]) -> f32 {
    let mut length = 0.0f32;
    for pair in points.windows(2) {
        let dx = pair[0].x - pair[1].x;
        let dz = pair[0].z - pair[1].z;
        length += (dz * dz + dx * dx).sqrt();
    }
    length
}

/// Asks `router` for the route and starts following it when the original
/// does. `None` when it is not this brain's turn.
pub fn plan(
    turn: &mut PlanTurn,
    who: u64,
    clock: u32,
    from: Vec3,
    goal: Goal,
    forced: bool,
    router: &mut dyn Router,
) -> Option<Planned> {
    if !turn.turn(who, clock) {
        return None;
    }
    let points = router.route(from, goal.point);
    let following =
        points.len() >= 2 && (forced || route_span2(&points) > FOLLOW_SPAN_CM * FOLLOW_SPAN_CM);
    Some(Planned {
        goal,
        route: Route {
            points,
            cursor: if following { 1 } else { 0 },
            fraction: 0.0,
        },
        following,
    })
}

/// The navigation grid as a [`Router`]: centimetre points in, centimetre
/// points out, the search itself [`exa_world::nav::find_path`]'s.
pub struct NavRouter<'a> {
    pub grid: &'a exa_world::nav::NavGrid,
    /// The walker's size class.
    pub size: u8,
}

impl Router for NavRouter<'_> {
    fn route(&mut self, from: Vec3, to: Vec3) -> Vec<Vec3> {
        exa_world::nav::find_path(self.grid, from / 100.0, to / 100.0, self.size)
            .points
            .into_iter()
            .map(|p| p * 100.0)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The path length is the sum of the flat segments; a height change
    /// adds nothing.
    #[test]
    fn the_path_length_sums_the_flat_segments() {
        assert_eq!(path_length(&[]), 0.0);
        assert_eq!(path_length(&[Vec3::ZERO]), 0.0);
        assert_eq!(
            path_length(&[
                Vec3::ZERO,
                Vec3::new(300.0, 0.0, 400.0),
                Vec3::new(300.0, 100.0, 400.0)
            ]),
            500.0
        );
    }

    use exa_world::nav::{Layer, NavCell, NavGrid};

    /// A flat grid whose floors stand at voxel 100 (10 m over base 0): a
    /// start much lower would take the empty upper layer, as the planner's
    /// start rule does.
    fn yard(width: usize, depth: usize) -> NavGrid {
        let mut grid = NavGrid::empty(width, depth, 0.0);
        for z in 0..depth {
            for x in 0..width {
                grid.set_cell(
                    x,
                    z,
                    Layer::Lower,
                    NavCell {
                        floor: 100,
                        ..NavCell::default()
                    },
                );
            }
        }
        grid
    }

    /// A grid of 40 × 40 cells (planner cells −10 to 10 in reach), from
    /// its cell (11, 20) to its cell (28, 20), 5.1 m apart.
    fn route() -> (NavGrid, Vec3, Vec3) {
        let grid = yard(40, 40);
        let from = grid.cell_centre(11, 20);
        let to = grid.cell_centre(28, 20);
        (
            grid,
            Vec3::new(from.x, 10.0, from.y) * 100.0,
            Vec3::new(to.x, 10.0, to.y) * 100.0,
        )
    }

    #[test]
    fn only_the_turn_plans_and_the_longest_waiter_is_remembered() {
        let mut turn = PlanTurn::default();
        assert!(!turn.turn(7, 1));
        assert!(!turn.turn(9, 2));
        assert_eq!(turn.longest, Some(9));
        assert_eq!(turn.longest_wait, 2);
        // An equal wait does not take the place.
        assert!(!turn.turn(5, 2));
        assert_eq!(turn.longest, Some(9));
        turn.current = Some(7);
        assert!(turn.turn(7, 3));
        assert_eq!(turn.last_plan(7), Some(3));
        assert_eq!(turn.next_turn(), Some(9));
    }

    #[test]
    fn a_route_is_followed_past_the_span_and_cleared_below_it() {
        let (grid, from, to) = route();
        let mut turn = PlanTurn {
            current: Some(1),
            ..PlanTurn::default()
        };
        let mut router = NavRouter {
            grid: &grid,
            size: 0,
        };
        let goal = Goal {
            point: to,
            radius_cm: 40.0,
            motion: 1,
        };
        let planned = plan(&mut turn, 1, 0, from, goal, false, &mut router).unwrap();
        assert!(planned.following);
        assert!(planned.route.points.len() >= 2);
        assert_eq!(planned.route.cursor, 1);
        assert_eq!(planned.route.points.first().copied(), Some(from));
        assert_eq!(planned.route.points.last().copied(), Some(to));

        // The next cell is 30 cm away: under the span, not followed,
        // unless forced.
        let near = grid.cell_centre(12, 20);
        let near = Vec3::new(near.x, 10.0, near.y) * 100.0;
        let mut turn = PlanTurn {
            current: Some(1),
            ..PlanTurn::default()
        };
        let planned = plan(
            &mut turn,
            1,
            0,
            from,
            Goal {
                point: near,
                radius_cm: 40.0,
                motion: 1,
            },
            false,
            &mut router,
        )
        .unwrap();
        assert!(!planned.following);
        assert_eq!(planned.route.cursor, 0);
        let mut turn = PlanTurn {
            current: Some(1),
            ..PlanTurn::default()
        };
        let planned = plan(
            &mut turn,
            1,
            0,
            from,
            Goal {
                point: near,
                radius_cm: 40.0,
                motion: 1,
            },
            true,
            &mut router,
        )
        .unwrap();
        assert!(planned.following);
    }

    #[test]
    fn the_span_is_horizontal() {
        assert_eq!(
            route_span2(&[Vec3::new(0.0, 0.0, 0.0), Vec3::new(300.0, 900.0, 0.0)]),
            90_000.0
        );
        assert_eq!(route_span2(&[]), 0.0);
    }
}
