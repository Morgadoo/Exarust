//! The idle states' shared head and the roam and patrol walk.
//!
//! Every idle state of the original (the wait, the roam, the patrol and the
//! idle-activity driver) opens the same way: the investigation of a
//! disturbance, then the alert look-around ([`crate::motion::alert_look`]),
//! then the attention to the contact the target pass attends ([`attend`]);
//! only when neither takes the tick does the state clear the body's
//! in-combat mark, stand the body in its idle motion and run its own phase.
//!
//! The roam's phases are [`crate::idle::tick`]'s: it stands until its
//! deadline, plans a path to a wander point a quarter of the way through a
//! drawn door ([`crate::idle::pick_wander`]), walks it, settles its look and
//! pauses 0.5–2.5 s, plans a forced path through the door into the room it
//! aimed at ([`crate::idle::pick_home`]), walks it, settles and pauses
//! 1–5 s. A patrol walks the wander leg only and pauses 0.5–2.5 s. The plan
//! is the one-planner-a-frame turn ([`crate::route::PlanTurn`]): the goal is
//! picked only on the brain's turn, and a turn always ends the planning
//! phase, whether or not the route is long enough to follow.
//!
//! [`roam_step`] runs the phases on plain values; [`follow_walk`] walks the
//! followed route; [`crate::brain::idle_walk`] is the running game's caller.

use bevy::math::{Vec2, Vec3};

use crate::idle::{self, Exit, Kind, Moment, Phase, Routine};
use crate::motion::{self, Gaze, Turning};
use crate::route::{self, PlanTurn, Router};

/// The notice score the idle states attend a contact from.
pub const ATTEND_LEVEL: f32 = 1.0;

/// The look window while the scripted focus holds (50°).
const FOCUS_WINDOW: f32 = 50.0;
/// The engine's degree in radians, in single precision.
const DEGREE: f32 = f32::from_bits(0x3c8e_fa35);
/// The window's upper share, in single precision.
const WIDEST_SHARE: f32 = f32::from_bits(0x3f33_3333);

/// The half-angle (rad) within which an attended contact does not pull the
/// look: `1 − 0.1·score` (in double precision, rounded to single), held to
/// 0.25 from below and to 0.7 from above (the upper test in double
/// precision), then times the double π. A NaN score keeps a NaN window, so
/// the look never moves.
pub fn attend_window(score: f32) -> f32 {
    let mut share = (1.0 - f64::from(score) * 0.1) as f32;
    if share < 0.25 {
        share = 0.25;
    } else if f64::from(share) > 0.7 {
        share = WIDEST_SHARE;
    }
    (f64::from(share) * std::f64::consts::PI) as f32
}

/// The contact the target pass attends, as an idle state sees it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Attended {
    /// Its notice score, `(concern + hostility)·6 + (noticed − attention)`,
    /// or `None` when it is not among the brain's contacts.
    pub score: Option<f32>,
    /// The yaw from the brain's body to the attended's point.
    pub yaw: f32,
    /// It is the scripted focus and the focus look holds.
    pub focus_held: bool,
}

/// The score [`attend`] reads for a contact: the same notice score the
/// classifier forms, summed in its order.
pub fn notice_score(concern: f32, hostility: f32, noticed: f32, attention: f32) -> f32 {
    (concern + hostility) * 6.0 + (noticed - attention)
}

/// The idle states' attention: with no attended contact, or one whose score
/// lies under `level` (a NaN score attends), it does nothing and answers
/// false. Otherwise the AI's look takes the yaw to the contact when the
/// wrapped gap passes the window ([`attend_window`]; 50° for a held focus)
/// and it answers true: the caller then turns the body
/// ([`motion::turn_towards`]), stands it in its idle motion and puts a roam
/// or patrol back in its wait.
///
/// [guess: a contact outside the list leaves the original's window
/// register unset; Exarust takes a zero window, so the look follows it.]
pub fn attend(turn: &mut Turning, attended: Option<Attended>, level: f32) -> bool {
    let Some(attended) = attended else {
        return false;
    };
    let mut window = 0.0;
    if let Some(score) = attended.score {
        if level > score {
            return false;
        }
        window = attend_window(score);
    }
    if attended.focus_held {
        window = FOCUS_WINDOW * DEGREE;
    }
    if window < motion::gap(attended.yaw, turn.look).abs() {
        turn.look = attended.yaw;
    }
    true
}

/// A roam's or patrol's walk: the goal its last pick wrote, the route it
/// follows and the room the last wander aimed at.
#[derive(Debug, Clone, PartialEq)]
pub struct IdleWalk {
    /// The followed route (cm); empty while no route is followed.
    pub route: Vec<Vec3>,
    /// The goal the last pick wrote (cm, height 0); a pick that finds no
    /// exit keeps it, as the original keeps its goal.
    pub goal: Option<Vec3>,
    /// The room index the last wander pick aimed at, −1 none: the home-side
    /// pick walks through the door toward it.
    pub toward: i32,
    /// The drawn exit's place in the room's list.
    pub cursor: i32,
    /// When a followed route is given up (ms) [Exarust's guard: the
    /// original's roam has no deadline; the idle driver's, path length·20 +
    /// 2000 ms, keeps a walker stuck on the running game's colliders from
    /// pressing on for ever].
    pub give_up_ms: u32,
    /// The velocity the walk last steered (m/s).
    pub velocity: Vec3,
}

impl Default for IdleWalk {
    fn default() -> Self {
        Self {
            route: Vec::new(),
            goal: None,
            toward: -1,
            cursor: -1,
            give_up_ms: 0,
            velocity: Vec3::ZERO,
        }
    }
}

/// Where a roaming brain stands and the planner it asks.
pub struct RoamPlace<'a> {
    /// Its body's ground point (cm).
    pub at_cm: Vec3,
    /// Its level room, −1 none.
    pub room: i32,
    /// Its room's exits ([`crate::rooms::wander_exits`]); read only on its
    /// planning turn, so the caller may build them lazily.
    pub exits: &'a mut dyn FnMut() -> Vec<Exit>,
    /// The room a wander avoids [guess: the original's avoided room is a
    /// word the idle driver clears; Exarust passes the room before].
    pub avoid: i32,
    /// The planner's turn, the brain's id and the frame it is measured on.
    pub turn: &'a mut PlanTurn,
    pub who: u64,
    pub frame: u32,
    /// The navigation grid's router; none when the level has no grid.
    pub router: Option<&'a mut dyn Router>,
}

/// What one roam step did.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RoamStep {
    /// It had its planning turn this step.
    pub planned: bool,
    /// A walk ended and the look settled.
    pub settled: bool,
}

/// One roam or patrol step after the idle head let it through: on its
/// planning turn the wander (or, for a roam's second leg, the forced
/// home-side) goal is picked and routed; a walk whose route is no longer
/// followed settles the look ([`motion::settle_look`], its draw before the
/// pause's) and the phase goes on ([`idle::tick`]). Both draw from `roll`,
/// the brain's generator (the original's shared one). Any other routine
/// does nothing.
#[allow(clippy::too_many_arguments)]
pub fn roam_step(
    routine: &mut Routine,
    walk: &mut IdleWalk,
    place: RoamPlace<'_>,
    gaze: &mut Gaze,
    turn: &mut Turning,
    now_ms: u32,
    roll: &mut u32,
) -> RoamStep {
    let mut step = RoamStep::default();
    if !matches!(routine.kind, Kind::Roam | Kind::Patrol) {
        return step;
    }
    let home_leg = routine.kind == Kind::Roam && routine.phase == Phase::PlanHome;
    if (routine.phase == Phase::PlanWander || home_leg) && place.turn.turn(place.who, place.frame) {
        step.planned = true;
        let exits = (place.exits)();
        let picked = if home_leg {
            idle::pick_home(place.room, &exits, walk.toward)
        } else {
            idle::pick_wander(place.room, &exits, place.avoid, roll)
        };
        if let Some(picked) = picked {
            walk.goal = Some(picked.goal);
            if !home_leg {
                walk.toward = picked.node;
                walk.cursor = picked.cursor;
            }
        }
        // [Exarust: the router chooses the grid layer by height; the goal
        // is at height 0 and its test ignores height, so it is routed at
        // the walker's own height.]
        let points = match (walk.goal, place.router) {
            (Some(goal), Some(router)) => {
                router.route(place.at_cm, Vec3::new(goal.x, place.at_cm.y, goal.z))
            }
            _ => Vec::new(),
        };
        let followed = points.len() >= 2
            && (home_leg
                || route::FOLLOW_SPAN_CM * route::FOLLOW_SPAN_CM < route::route_span2(&points));
        walk.route.clear();
        if followed {
            let length = route::path_length(&points);
            walk.give_up_ms = now_ms
                .wrapping_add((length * 20.0).floor() as i32 as u32)
                .wrapping_add(2000);
            walk.route = points;
        }
    }
    let walking = matches!(routine.phase, Phase::WalkWander | Phase::WalkHome);
    if walking && walk.route.is_empty() {
        motion::settle_look(gaze, turn, motion::yaw_of(gaze.body_look), now_ms, roll);
        step.settled = true;
    }
    routine.roll = *roll;
    idle::tick(
        routine,
        &Moment {
            now_ms,
            path_accepted: step.planned,
            arrived: walk.route.is_empty(),
            ..Moment::default()
        },
    );
    *roll = routine.roll;
    step
}

/// The flat squared reach (cm²) within which a followed route has arrived:
/// the path follower's urgent stop, 100 cm from the last point.
const ARRIVED_CM2: f32 = 10_000.0;

/// One step along the followed route from the ground point `feet_cm`: the
/// carrot 100 cm ahead ([`crate::path::carrot`]) to steer to, or `None`
/// once the walk is over. The route ends within 100 cm of its last point
/// (the follower's urgent stop; measured flat, as the running game's root
/// is not on the grid's floor) or past its give-up time.
pub fn follow_walk(walk: &mut IdleWalk, feet_cm: Vec3, now_ms: u32) -> Option<Vec3> {
    let last = *walk.route.last()?;
    let flat = Vec2::new(last.x - feet_cm.x, last.z - feet_cm.z);
    if flat.length_squared() < ARRIVED_CM2 || walk.give_up_ms < now_ms {
        walk.route.clear();
        return None;
    }
    // [Exarust: a carrot that lands on a joint has no goal; the walk then
    // steers to the route's last point rather than stopping.]
    crate::path::carrot(&walk.route, feet_cm)
        .goal
        .or(Some(last))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_attend_window_is_held_between_a_quarter_and_seven_tenths_of_pi() {
        // Score 0: 1 − 0 = 1, held to 0.7 (single precision) times π.
        let widest = (f64::from(f32::from_bits(0x3f33_3333)) * std::f64::consts::PI) as f32;
        assert_eq!(attend_window(0.0), widest);
        assert_eq!(attend_window(3.0), widest);
        // Score 5: 0.5 of π.
        assert_eq!(attend_window(5.0), (0.5f64 * std::f64::consts::PI) as f32);
        // Score 9 and beyond: a quarter of π.
        assert_eq!(attend_window(9.0), (0.25f64 * std::f64::consts::PI) as f32);
        assert_eq!(attend_window(40.0), attend_window(9.0));
        assert!(attend_window(f32::NAN).is_nan());
    }

    #[test]
    fn attending_turns_the_look_past_the_window_only() {
        let mut turn = Turning::default();
        assert!(!attend(&mut turn, None, ATTEND_LEVEL));
        // Under the level: nothing.
        let low = Attended {
            score: Some(0.5),
            yaw: 3.0,
            focus_held: false,
        };
        assert!(!attend(&mut turn, Some(low), ATTEND_LEVEL));
        assert_eq!(turn.look, 0.0);
        // Score 5: a half-π window; 1.0 rad stays inside, 2.0 rad pulls.
        let inside = Attended {
            score: Some(5.0),
            yaw: 1.0,
            focus_held: false,
        };
        assert!(attend(&mut turn, Some(inside), ATTEND_LEVEL));
        assert_eq!(turn.look, 0.0);
        let outside = Attended { yaw: 2.0, ..inside };
        assert!(attend(&mut turn, Some(outside), ATTEND_LEVEL));
        assert_eq!(turn.look, 2.0);
        // The gap wraps: from 3 rad, −3 rad is 0.28 rad away.
        let mut turn = Turning {
            look: 3.0,
            ..Turning::default()
        };
        let wrapped = Attended {
            score: Some(9.0),
            yaw: -3.0,
            focus_held: false,
        };
        assert!(attend(&mut turn, Some(wrapped), ATTEND_LEVEL));
        assert_eq!(turn.look, 3.0);
        // A held focus narrows the window to 50°: 0.9 rad pulls.
        let focus = Attended {
            score: Some(1.0),
            yaw: 3.9,
            focus_held: true,
        };
        assert!(attend(&mut turn, Some(focus), ATTEND_LEVEL));
        assert_eq!(turn.look, 3.9);
        // A NaN score attends with a NaN window: the look stays.
        let mut turn = Turning::default();
        let nan = Attended {
            score: Some(f32::NAN),
            yaw: 2.0,
            focus_held: false,
        };
        assert!(attend(&mut turn, Some(nan), ATTEND_LEVEL));
        assert_eq!(turn.look, 0.0);
        // Outside the list: a zero window, so any gap pulls.
        let stranger = Attended {
            score: None,
            yaw: 0.01,
            focus_held: false,
        };
        assert!(attend(&mut turn, Some(stranger), ATTEND_LEVEL));
        assert_eq!(turn.look, 0.01);
        assert_eq!(notice_score(0.5, 0.25, 2.0, 0.5), 6.0);
    }

    /// A router that walks straight to the goal in two points.
    struct Straight;

    impl Router for Straight {
        fn route(&mut self, from: Vec3, to: Vec3) -> Vec<Vec3> {
            vec![from, to]
        }
    }

    fn doors() -> Vec<Exit> {
        vec![Exit {
            point: Vec2::new(600.0, 0.0),
            near: Vec2::ZERO,
            node: 4,
        }]
    }

    #[allow(clippy::too_many_arguments)]
    fn step(
        routine: &mut Routine,
        walk: &mut IdleWalk,
        turn: &mut PlanTurn,
        at: Vec3,
        now: u32,
        roll: &mut u32,
        gaze: &mut Gaze,
        look: &mut Turning,
    ) -> RoamStep {
        let mut exits = doors;
        let mut router = Straight;
        roam_step(
            routine,
            walk,
            RoamPlace {
                at_cm: at,
                room: 0,
                exits: &mut exits,
                avoid: -1,
                turn,
                who: 7,
                frame: now,
                router: Some(&mut router),
            },
            gaze,
            look,
            now,
            roll,
        )
    }

    /// The roam stands, waits for its turn, walks a quarter of the way
    /// through the door, settles (its draw first) and pauses, then walks a
    /// forced route into the room it aimed at and pauses again.
    #[test]
    fn a_roam_walks_through_a_door_and_settles_after_each_leg() {
        let mut routine = Routine::roam(100);
        let mut walk = IdleWalk::default();
        let mut turn = PlanTurn::default();
        let mut roll = 1u32;
        let mut gaze = Gaze::default();
        let mut look = Turning::default();
        let at = Vec3::new(-300.0, 50.0, 0.0);
        // Past its deadline it goes to plan.
        step(
            &mut routine,
            &mut walk,
            &mut turn,
            at,
            101,
            &mut roll,
            &mut gaze,
            &mut look,
        );
        assert_eq!(routine.phase, Phase::PlanWander);
        // Not its turn: it waits and is remembered as the longest waiter.
        let s = step(
            &mut routine,
            &mut walk,
            &mut turn,
            at,
            102,
            &mut roll,
            &mut gaze,
            &mut look,
        );
        assert!(!s.planned);
        assert_eq!(routine.phase, Phase::PlanWander);
        assert_eq!(turn.next_turn(), Some(7));
        let s = step(
            &mut routine,
            &mut walk,
            &mut turn,
            at,
            103,
            &mut roll,
            &mut gaze,
            &mut look,
        );
        assert!(s.planned);
        assert_eq!(routine.phase, Phase::WalkWander);
        assert_eq!(walk.goal, Some(Vec3::new(150.0, 0.0, 0.0)));
        assert_eq!((walk.toward, walk.cursor), (4, 0));
        assert_eq!(walk.route, vec![at, Vec3::new(150.0, 50.0, 0.0)]);
        // 450 cm long: given up 9000 + 2000 ms out.
        assert_eq!(walk.give_up_ms, 103 + 9000 + 2000);
        // Arrived (the route cleared): the look settles, then it pauses.
        walk.route.clear();
        let before = roll;
        let s = step(
            &mut routine,
            &mut walk,
            &mut turn,
            at,
            500,
            &mut roll,
            &mut gaze,
            &mut look,
        );
        assert!(s.settled);
        assert_eq!(routine.phase, Phase::WaitHome);
        let mut expect = before;
        expect = expect.wrapping_mul(0x0808_8405).wrapping_add(1);
        let sway = ((u64::from(expect) * 3000) >> 32) as u32;
        assert_eq!(look.sway_at_ms, 500 + 1000 + sway);
        expect = expect.wrapping_mul(0x0808_8405).wrapping_add(1);
        let pause = ((u64::from(expect) * 2000) >> 32) as u32;
        assert_eq!(routine.until_ms, 500 + 500 + pause);
        assert_eq!(roll, expect);
        // The home-side leg: forced, through to the far cell.
        let later = routine.until_ms + 1;
        step(
            &mut routine,
            &mut walk,
            &mut turn,
            at,
            later,
            &mut roll,
            &mut gaze,
            &mut look,
        );
        assert_eq!(routine.phase, Phase::PlanHome);
        turn.current = Some(7);
        let near = Vec3::new(560.0, 50.0, 0.0);
        step(
            &mut routine,
            &mut walk,
            &mut turn,
            near,
            later + 1,
            &mut roll,
            &mut gaze,
            &mut look,
        );
        assert_eq!(routine.phase, Phase::WalkHome);
        assert_eq!(walk.goal, Some(Vec3::new(600.0, 0.0, 0.0)));
        // 40 cm: short, but the home leg is forced.
        assert_eq!(walk.route.len(), 2);
        assert_eq!(walk.toward, 4);
    }

    /// A wander route no longer than 150 cm end to end is not followed: the
    /// turn still ends the planning, and the walk ends on the next step.
    #[test]
    fn a_short_wander_route_is_not_followed() {
        let mut routine = Routine::patrol(0);
        routine.phase = Phase::PlanWander;
        let mut walk = IdleWalk::default();
        let mut turn = PlanTurn {
            current: Some(7),
            ..PlanTurn::default()
        };
        let (mut roll, mut gaze, mut look) = (1u32, Gaze::default(), Turning::default());
        let at = Vec3::new(100.0, 0.0, 0.0);
        let s = step(
            &mut routine,
            &mut walk,
            &mut turn,
            at,
            10,
            &mut roll,
            &mut gaze,
            &mut look,
        );
        assert!(s.planned);
        assert!(walk.route.is_empty());
        assert_eq!(routine.phase, Phase::WalkWander);
        let s = step(
            &mut routine,
            &mut walk,
            &mut turn,
            at,
            11,
            &mut roll,
            &mut gaze,
            &mut look,
        );
        assert!(s.settled);
        // A patrol pauses 0.5–2.5 s in its wait.
        assert_eq!(routine.phase, Phase::Wait);
        assert!((511..=2511).contains(&routine.until_ms));
        // A wait or a sentry does not step.
        let mut wait = Routine::wait();
        let s = step(
            &mut wait, &mut walk, &mut turn, at, 12, &mut roll, &mut gaze, &mut look,
        );
        assert_eq!(s, RoamStep::default());
    }

    #[test]
    fn the_walk_steers_to_the_carrot_and_ends_near_the_last_point() {
        let mut walk = IdleWalk {
            route: vec![Vec3::ZERO, Vec3::new(500.0, 0.0, 0.0)],
            give_up_ms: 1000,
            ..IdleWalk::default()
        };
        assert_eq!(
            follow_walk(&mut walk, Vec3::new(0.0, 80.0, 0.0), 10),
            Some(Vec3::new(100.0, 0.0, 0.0))
        );
        // Within 100 cm on the flat, whatever the height: arrived.
        assert_eq!(
            follow_walk(&mut walk, Vec3::new(401.0, 300.0, 0.0), 10),
            None
        );
        assert!(walk.route.is_empty());
        assert_eq!(follow_walk(&mut walk, Vec3::ZERO, 10), None);
        // Past its give-up time it stops too.
        walk.route = vec![Vec3::ZERO, Vec3::new(500.0, 0.0, 0.0)];
        assert_eq!(follow_walk(&mut walk, Vec3::ZERO, 1001), None);
        assert!(walk.route.is_empty());
    }
}
