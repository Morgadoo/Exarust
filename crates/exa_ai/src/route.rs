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

/// How a search that stopped short of its goal decides whether its end is
/// near enough (the goal record's end test).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GoalTest {
    /// None installed: the planner's own test, an end farther than 100 cm
    /// from the goal in 3-D is too far.
    Near,
    /// Too far outside the record's radius across the ground (height
    /// ignored): the wander, home-side and flight goals.
    Flat,
    /// Too far outside the record's radius with the height at half weight:
    /// the chase and the follower's walk to its leader.
    HalfHeight,
    /// Never too far: the follower's closing walk and the AI's Long Door.
    Always,
}

/// The goal record a character's planner reads besides the goal point:
/// the walker's size class, the accept distance (heuristic units, ten a
/// cell), the end test and its radius (cm). Each state that plans writes
/// the fields it sets and keeps the others from the last state that did,
/// as the original keeps one record on the character. The penalty shift
/// is the character's group-alert count ([`crate::brain::Npc::alert_count`]).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlanGoal {
    pub size: u8,
    pub accept: i32,
    pub test: GoalTest,
    pub radius_cm: f32,
}

/// The size class, accept distance and test the character's constructor
/// writes (no radius: the record starts zeroed).
impl Default for PlanGoal {
    fn default() -> Self {
        Self {
            size: 3,
            accept: 10,
            test: GoalTest::Near,
            radius_cm: 0.0,
        }
    }
}

/// The goal radius of the wander, home-side and flight goals, and of a
/// chase or follow whose target stands (cm).
pub const GOAL_RADIUS_CM: f32 = 150.0;
/// The chase's and follow's goal radius when the target is down (cm).
pub const DOWN_TARGET_RADIUS_CM: f32 = 300.0;

/// The size class the roam and patrol states write before their wander or
/// home-side pick.
pub const ROAM_SIZE: u8 = 4;
/// The flight's accept distance (three cells), written with its way out.
pub const FLIGHT_ACCEPT: i32 = 30;

/// The size class the chase and the flight write: the phase word applying
/// roles wrote, plus 3 (a marked body's −1 gives 2, a follower's 1 gives 4).
pub fn phase_size(phase_word: i32) -> u8 {
    (phase_word + 3) as u8
}

impl PlanGoal {
    /// The wander or home-side goal: stop at two cells, the flat test at
    /// 150 cm. (The roam and patrol states write [`ROAM_SIZE`] themselves,
    /// before the pick.)
    pub fn roam(&mut self) {
        self.accept = 20;
        self.test = GoalTest::Flat;
        self.radius_cm = GOAL_RADIUS_CM;
    }

    /// The flight's way out: stop at three cells, the flat test at the
    /// way's radius.
    pub fn flight(&mut self, radius_cm: f32) {
        self.accept = FLIGHT_ACCEPT;
        self.test = GoalTest::Flat;
        self.radius_cm = radius_cm;
    }

    /// The chase and follow goal's test: the half-height test at 150 cm,
    /// 300 cm when the target is down.
    pub fn toward_target(&mut self, target_down: bool) {
        self.test = GoalTest::HalfHeight;
        self.radius_cm = if target_down {
            DOWN_TARGET_RADIUS_CM
        } else {
            GOAL_RADIUS_CM
        };
    }

    /// Whether a search's end `end_cm` is too far from `goal_cm`, or `None`
    /// for the planner's own test.
    pub fn too_far(&self, goal_cm: Vec3, end_cm: Vec3) -> Option<bool> {
        let offset = end_cm - goal_cm;
        match self.test {
            GoalTest::Near => None,
            GoalTest::Flat => Some(crate::combatant::outside_radius(
                self.radius_cm,
                offset,
                false,
            )),
            GoalTest::HalfHeight => Some(crate::combatant::outside_radius(
                self.radius_cm,
                offset,
                true,
            )),
            GoalTest::Always => Some(false),
        }
    }
}

/// The planner's route source: the navigation grid.
pub trait Router {
    /// The path from `from` to `to` (cm): points at cell centres, empty
    /// when no path starts.
    fn route(&mut self, from: Vec3, to: Vec3) -> Vec<Vec3>;

    /// The goal record the next search reads, for the state to write its
    /// fields before it plans. None for a router without one.
    fn goal(&mut self) -> Option<&mut PlanGoal> {
        None
    }
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
        // An equal wait keeps the earlier brain. The original's brains
        // claim in its character list's order, the order they joined the
        // scene; Exarust's queries visit them in an order that depends on
        // when each body's components arrived during the load, so "earlier"
        // is the earlier spawned (the lower entity index), whatever order
        // the claims come in. A level then runs alike from run to run.
        let earlier = |a: u64, b: u64| (a & 0xFFFF_FFFF) < (b & 0xFFFF_FFFF);
        if waited > self.longest_wait
            || (waited == self.longest_wait
                && waited > 0
                && self.longest.is_some_and(|other| earlier(who, other)))
        {
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
/// points out, the search itself [`exa_world::nav::find_path_to`]'s with
/// the character's goal record and its alert count as the penalty shift.
pub struct NavRouter<'a> {
    pub grid: &'a exa_world::nav::NavGrid,
    /// The character's goal record.
    pub goal: &'a mut PlanGoal,
    /// The penalty byte's shift: the group-alert count.
    pub shift: i32,
}

impl<'a> NavRouter<'a> {
    pub fn new(grid: &'a exa_world::nav::NavGrid, goal: &'a mut PlanGoal, shift: i32) -> Self {
        Self { grid, goal, shift }
    }
}

impl Router for NavRouter<'_> {
    fn route(&mut self, from: Vec3, to: Vec3) -> Vec<Vec3> {
        let record = *self.goal;
        let ask = exa_world::nav::Search {
            size: record.size,
            shift: self.shift as u32,
            accept: record.accept,
        };
        let test = |end_cm: Vec3| record.too_far(to, end_cm).unwrap_or(false);
        let too_far: Option<&dyn Fn(Vec3) -> bool> = match record.test {
            GoalTest::Near => None,
            _ => Some(&test),
        };
        exa_world::nav::find_path_to(self.grid, from / 100.0, to / 100.0, ask, too_far)
            .points
            .into_iter()
            .map(|p| p * 100.0)
            .collect()
    }

    fn goal(&mut self) -> Option<&mut PlanGoal> {
        Some(self.goal)
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
        // An equal wait takes the place only for an earlier spawned brain
        // (a lower index), whatever the claims' order.
        assert!(!turn.turn(11, 2));
        assert_eq!(turn.longest, Some(9));
        assert!(!turn.turn(5, 2));
        assert_eq!(turn.longest, Some(5));
        assert!(!turn.turn(9, 2));
        assert_eq!(turn.longest, Some(5));
        turn.current = Some(7);
        assert!(turn.turn(7, 3));
        assert_eq!(turn.last_plan(7), Some(3));
        assert_eq!(turn.next_turn(), Some(5));
    }

    #[test]
    fn a_route_is_followed_past_the_span_and_cleared_below_it() {
        let (grid, from, to) = route();
        let mut turn = PlanTurn {
            current: Some(1),
            ..PlanTurn::default()
        };
        let mut goal = PlanGoal::default();
        let mut router = NavRouter::new(&grid, &mut goal, 1);
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

    /// The constructor's record leaves the planner's 100 cm test; the
    /// flat test ignores height, the half-height test halves it, and the
    /// closing walk's never finds an end too far.
    #[test]
    fn the_goal_records_end_tests() {
        let goal = Vec3::new(0.0, 0.0, 0.0);
        let mut record = PlanGoal::default();
        assert_eq!((record.size, record.accept), (3, 10));
        assert_eq!(record.too_far(goal, Vec3::new(500.0, 0.0, 0.0)), None);
        record.roam();
        assert_eq!(record.accept, 20);
        assert_eq!(
            record.too_far(goal, Vec3::new(140.0, 900.0, 0.0)),
            Some(false)
        );
        assert_eq!(record.too_far(goal, Vec3::new(151.0, 0.0, 0.0)), Some(true));
        record.toward_target(false);
        assert_eq!(
            record.too_far(goal, Vec3::new(0.0, 290.0, 0.0)),
            Some(false)
        );
        assert_eq!(record.too_far(goal, Vec3::new(0.0, 310.0, 0.0)), Some(true));
        record.toward_target(true);
        assert_eq!(record.radius_cm, DOWN_TARGET_RADIUS_CM);
        record.flight(150.0);
        assert_eq!((record.accept, record.test), (30, GoalTest::Flat));
        // The phase word plus 3: marked, plain, follower.
        assert_eq!((phase_size(-1), phase_size(0), phase_size(1)), (2, 3, 4));
        record.test = GoalTest::Always;
        assert_eq!(record.too_far(goal, Vec3::splat(1e6)), Some(false));
    }

    /// The router reads the record: the constructor's size class 3 keeps
    /// off a cell of clearance 50 that class 0 walks through.
    #[test]
    fn the_router_plans_with_the_record() {
        let (mut grid, from, to) = route();
        for z in 0..40 {
            for x in 0..40 {
                if z != 20 {
                    grid.set_cell(x, z, Layer::Lower, NavCell::default());
                }
            }
        }
        grid.set_cell(
            20,
            20,
            Layer::Lower,
            NavCell {
                floor: 100,
                clearance: 50,
                ..NavCell::default()
            },
        );
        let mut record = PlanGoal {
            size: 0,
            ..PlanGoal::default()
        };
        let small = NavRouter::new(&grid, &mut record, 1).route(from, to);
        assert_eq!(small.last().copied(), Some(to));
        let mut record = PlanGoal::default();
        let big = NavRouter::new(&grid, &mut record, 1).route(from, to);
        let end = big.last().copied().unwrap();
        assert!(end.x < to.x - 200.0, "{end:?}");
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
