//! The AI's route walk: one route a character, followed by one walk.
//!
//! The original keeps a single route on the AI: the planner's points, the
//! follower's segment and fraction, the "following" byte the route start
//! sets and every stop clears, the "plan wanted" byte the follower sets
//! when its line is blocked, the walk's urgency (0 walk, 1 hurry, 2 flight)
//! and a count of ticks after a route ended. Every state that plans (the
//! roam and patrol legs, the flight, the combat chase, the follow and its
//! search) writes its own goal and plans into that one route; the AI tick
//! then calls one walk after the state call while the route is followed
//! ([`walk`], run by [`walk_routes`] after every state of the brains'
//! tick). Exarust keeps the route as [`AiRoute`] on the character's model,
//! next to its [`crate::Duellist`].
//!
//! The walk ([`walk`]):
//! - a body that steps is busy: nothing more is done;
//! - the path follower ([`follow`]) sets the movement goal (its carrot);
//!   a route it ends leaves the after-route count at 80;
//! - out of the scene only the moving byte is set;
//! - in the scene, `d` is the goal less the standing point across the
//!   ground; with `|d|` (1 when its square is at most 1, the direction then
//!   the frame's forward and the alignment 0) the direction is `d/|d|` and
//!   the alignment `d·pelvis forward / |d|`; the heading eases a tenth of
//!   the way toward the direction and is made a unit again;
//! - with `m = 0.007·|d|` (in double; 0.7 beyond 100 cm), an alignment not
//!   over `m` (a NaN one included) or a route the follower let go asks only
//!   to move, moving while the alignment passes `m/2`; otherwise the goal
//!   goes 150 cm out from the standing point along `d`, the body moves, and
//!   it runs when the urgency is above 0, the grid's cost at the goal is
//!   not over 0.5 (a NaN cost passes) and the alignment passes 0.9 (in
//!   double), with the sprint latch at urgency 2;
//! - the AI's look yaw is the heading's; the body's look eases a tenth of
//!   the way toward the heading (a unit again);
//! - the head looks at the AI's look target ([`crate::motion::look_at`])
//!   when it has one in front; otherwise the head's offset target is 0 and
//!   the offset eases 0.02 of the way toward it (in double); the cursor is
//!   the look yaw plus the offset, wrapped once;
//! - a nearby body in the way ([`crate::motion::body_in_way`]) soft-stops
//!   the walk ([`soft_stop`]).
//!
//! The follower ([`follow`]):
//! - a route of fewer than two points stops ([`route_stop`]);
//! - in the scene the ground point is projected onto every segment with a
//!   horizontal length (the parameter clamped to [0, 1], a NaN kept), the
//!   nearest in 3-D taking the segment and its unclamped parameter; out of
//!   the scene the standing point slides 2 cm along the route from where
//!   it is (no physics) and the ground and standing points take it;
//! - the carrot lies 100 cm on from there along the route, spilling onto
//!   the later segments (a segment without horizontal length is skipped);
//!   a step of no length stops the route; the goal is the carrot;
//! - at urgency 0 the run bit is set until the carrot lies past 0.99 of
//!   the last segment (in double), where it clears and a ground point
//!   within 30 cm of the carrot (3-D) soft-stops; above 0 the run bit
//!   clears and a ground point within 100 cm of the last point soft-stops;
//! - in the scene the line from the ground point to the carrot is walked
//!   through the grid: a blocked line clears the run bit and asks for a
//!   plan.
//!
//! The soft stop ([`soft_stop`]) ends the route and its mode word, clears
//! the run bit, sets the stopping bit, pulls a goal farther than
//! `8·speed + 20` cm back along the frame's forward and walks the line
//! from the ground point to it, which clamps it where the grid blocks. The
//! route stop ([`route_stop`]) ends the route and its mode word, drops the
//! plan request and clears the run bit. The walk writes its flags after
//! the follower's, so a soft stop the follower made keeps only its goal:
//! the walk asks the body to move toward it. The in-the-way test skips the
//! body the walk lets pass (the leader a follow walks to).
//!
//! The planner's end test (the goal record's flat or half-height radius
//! test) belongs to the plan, not to the follower: the follower makes no
//! call through the goal record ([`crate::route::NavRouter`] asks it).
//!
//! Exarust's remainder:
//! - the ground and standing points are both the character root's place
//!   (the running game's root is not on the grid's floor);
//! - a state that loses the character (combat, a flight, the scene) ends
//!   its own route, and a route carries the state that planned it
//!   ([`Walker`]) so that only that state ends it [Exarust's guard: the
//!   original's states stop the route at their own points];
//! - a route can carry a deadline (path length × 20 ms + 2 s) [Exarust's
//!   guard, as the idle walk had: the original's route has none];
//! - a flight without a grid route is a straight route to its way out
//!   [Exarust: the original fights that tick];
//! - a body without muscles (no locomotion request) is steered by the NPC
//!   driver's stand-in toward the follower's goal, with its walk clip;
//! - the stop bytes beside the following byte are not modelled; the
//!   walk's clear of the body it lets pass and of the mode word at the end
//!   of its after-route count runs only when the walk is called without a
//!   route, which the AI tick never does (it walks only a followed route).
//!
//! Evidence: the original's AI walk, its path follower, its route stop and
//! soft stop, its head look and its body-in-the-way test, read from the
//! instructions (sums in their order, the double compares, the NaN
//! branches, the constants from the image).

use bevy::prelude::*;
use exa_anim::Animator;
use exa_combat::Condition;
use exa_physics::CharacterBody;
use exa_physics::body::BodyTicks;
use exa_physics::body::locomotion::{flags, forward};
use exa_world::nav::NavGrid;

use crate::drive::{DriveBody, Driver, Walking};
use crate::motion::{self, BodyAt, Gaze};

/// Centimetres per metre.
const CM: f32 = 100.0;
/// Below this a squared horizontal length is no segment.
const EPS: f32 = f32::from_bits(0x33ff_ffd7);
/// The follower's start for its nearest projection.
const FAR: f32 = f32::from_bits(0x7f7f_c99e);
/// The out-of-scene slide (cm a tick), and the length a segment without a
/// horizontal span counts as.
const SLIDE_CM: f32 = 2.0;
const SHORT_SPAN: f32 = f32::from_bits(0x3dcc_cccd);
/// The carrot's lead along the route (cm).
const CARROT_CM: f32 = 100.0;
/// At urgency 0 the run bit holds until the carrot passes this share of
/// the last segment (a double compare).
const RUN_UNTIL: f64 = 0.99;
/// The soft stops' reaches, squared: the carrot at urgency 0, the last
/// point above.
const STOP_AT_CARROT: f32 = 900.0;
const STOP_AT_END: f32 = 10_000.0;
/// The heading's and the look's ease a tick.
const EASE: f32 = f32::from_bits(0x3dcc_cccd);
/// The moving gate: `0.007·|d|` in double, 0.7 beyond 100 cm.
const GATE_FROM_CM: f32 = 100.0;
const GATE_SLOPE: f64 = 0.007;
const GATE_CAP: f32 = f32::from_bits(0x3f33_3333);
/// The far goal's reach out from the standing point (cm).
const REACH_CM: f32 = 150.0;
/// A run needs the grid's cost at the goal not over this, and the alignment
/// over the double 0.9.
const RUN_COST: f32 = 0.5;
const RUN_ALIGN: f64 = 0.9;
/// The head offset's ease toward its target a tick (double).
const HEAD_EASE: f64 = 0.02;
/// The soft stop's pull-back: `8·speed + 20` cm.
const STOP_SPEED: f32 = 8.0;
const STOP_BASE_CM: f32 = 20.0;
/// The ticks the walk counts down after a route ends.
pub const AFTER_TICKS: u32 = 80;

/// The state that planned the route the character follows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Walker {
    /// The roam's or patrol's leg.
    Roam,
    /// The flight to its way out.
    Flight,
    /// The combat chase.
    Chase,
    /// The follow state's walk and its search's.
    Follow,
}

/// The character's route (cm) and its walk's state.
#[derive(Component, Debug, Clone, PartialEq)]
pub struct AiRoute {
    /// The planned points (cm).
    pub points: Vec<Vec3>,
    /// The follower's segment (the end point's index, from 1).
    pub key: u32,
    /// Where along that segment the follower is (unclamped).
    pub fraction: f32,
    /// The route is followed: the AI tick walks it.
    pub following: bool,
    /// The follower's line was blocked: the state should plan again.
    pub wants_plan: bool,
    /// The walk's urgency: 0 walk, 1 hurry, 2 flight.
    pub urgency: i32,
    /// The ticks the walk still counts after its route ended.
    pub after_ticks: u32,
    /// The state that planned it.
    pub owner: Option<Walker>,
    /// [Exarust] The route is a straight line, not a grid plan.
    pub straight: bool,
    /// [Exarust's guard] When the route is given up (ms).
    pub give_up_ms: Option<u32>,
    /// [Exarust] The stand-in's velocity for a body without muscles (m/s).
    pub velocity: Vec3,
    /// The walk's mode word: the follow's 4 while it follows its leader, 6
    /// while it closes on it; every stop (the route stop and the soft stop)
    /// clears it.
    pub mode: i32,
    /// Where the leader stood when the follow last planned to it (cm).
    pub leader_at: Vec3,
    /// The body the walk lets pass: its in-the-way test skips it (the
    /// leader a follow walks to), as entity bits.
    pub passes: Option<u64>,
}

impl Default for AiRoute {
    fn default() -> Self {
        Self {
            points: Vec::new(),
            key: 0,
            fraction: 0.0,
            following: false,
            wants_plan: false,
            urgency: 0,
            after_ticks: 0,
            owner: None,
            straight: false,
            give_up_ms: None,
            velocity: Vec3::ZERO,
            mode: 0,
            leader_at: Vec3::ZERO,
            passes: None,
        }
    }
}

impl AiRoute {
    /// The route start: the planner's points are followed from their first
    /// segment, no plan is wanted, at the state's urgency.
    pub fn start(&mut self, points: Vec<Vec3>, urgency: i32, owner: Walker) {
        self.points = points;
        self.key = 1;
        self.fraction = 0.0;
        self.following = true;
        self.wants_plan = false;
        self.urgency = urgency;
        self.owner = Some(owner);
        self.straight = false;
        self.give_up_ms = None;
    }

    /// [`AiRoute::start`] with Exarust's deadline: the route's ground length
    /// × 20 ms + 2 s after `now_ms`.
    pub fn start_until(&mut self, points: Vec<Vec3>, urgency: i32, owner: Walker, now_ms: u32) {
        let length = crate::route::path_length(&points);
        self.start(points, urgency, owner);
        self.give_up_ms = Some(
            now_ms
                .wrapping_add((length * 20.0).floor() as i32 as u32)
                .wrapping_add(2000),
        );
    }

    /// [Exarust] A straight route from `from` to `to` (cm), followed.
    pub fn straight(&mut self, from: Vec3, to: Vec3, urgency: i32, owner: Walker) {
        self.start(vec![from, to], urgency, owner);
        self.straight = true;
    }

    /// The route stop's AI side: the route is no longer followed, no plan
    /// is wanted and the walk's mode word clears.
    pub fn stop(&mut self) {
        self.following = false;
        self.wants_plan = false;
        self.mode = 0;
    }

    /// Stops the route when `owner` planned it.
    pub fn stop_if(&mut self, owner: Walker) {
        if self.owner == Some(owner) {
            self.stop();
        }
    }

    /// `owner`'s route is followed.
    pub fn follows(&self, owner: Walker) -> bool {
        self.following && self.owner == Some(owner)
    }

    /// The followed route's last point (cm).
    pub fn goal(&self) -> Option<Vec3> {
        self.following
            .then(|| self.points.last().copied())
            .flatten()
    }
}

/// What the walk reads and writes of the body (cm; directions unit).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WalkBody {
    /// The ground point and the standing point.
    pub ground_cm: Vec3,
    pub standing_cm: Vec3,
    /// The standing point's height over the ground point.
    pub pelvis_cm: f32,
    /// The movement goal.
    pub goal_cm: Vec3,
    /// The locomotion flags ([`flags`]).
    pub flags: u32,
    /// The moving byte.
    pub moving: bool,
    /// A step runs: the walk is busy.
    pub stepping: bool,
    /// The speed along the heading (cm a tick).
    pub speed: f32,
    /// The character frame's forward and the pelvis's forward.
    pub frame_forward: Vec3,
    pub pelvis_forward: Vec3,
    /// The heading and look directions the walk eases.
    pub heading: Vec3,
    pub look: Vec3,
    /// The cursor's yaw the head look writes.
    pub cursor: f32,
}

impl WalkBody {
    /// A body standing at `at_cm` facing `facing` (unit), its heading and
    /// look along it.
    pub fn standing(at_cm: Vec3, facing: Vec3) -> Self {
        Self {
            ground_cm: at_cm,
            standing_cm: at_cm,
            pelvis_cm: 0.0,
            goal_cm: at_cm,
            flags: 0,
            moving: false,
            stepping: false,
            speed: 0.0,
            frame_forward: facing,
            pelvis_forward: facing,
            heading: facing,
            look: facing,
            cursor: motion::yaw_of(facing),
        }
    }
}

/// The squared distance as the engine's helper sums it: `(x² + y²) + z²`.
fn distance2(a: Vec3, b: Vec3) -> f32 {
    let d = a - b;
    (d.x * d.x + d.y * d.y) + d.z * d.z
}

/// A segment's squared horizontal length, `z² + x²`.
fn flat2(d: Vec3) -> f32 {
    d.z * d.z + d.x * d.x
}

/// The engine's lerp, `a + (b − a)·t` per axis.
fn lerp(a: Vec3, b: Vec3, t: f32) -> Vec3 {
    a + (b - a) * t
}

/// The engine's ease of `a` toward `b`, `a + (b − a)·t` per axis.
fn ease(a: Vec3, b: Vec3, t: f32) -> Vec3 {
    Vec3::new(
        a.x + (b.x - a.x) * t,
        a.y + (b.y - a.y) * t,
        a.z + (b.z - a.z) * t,
    )
}

/// The route stop: the route ends, no plan is wanted and the run bit
/// clears.
pub fn route_stop(route: &mut AiRoute, body: &mut WalkBody) {
    route.stop();
    body.flags &= !flags::RUN;
}

/// The soft stop: the route ends (a plan request stays, the mode word
/// clears), the run bit
/// clears and the stopping bit is set; a goal farther than `8·speed + 20`
/// cm from the standing point (3-D, a NaN distance not) is pulled back to
/// that along the frame's forward, and the line from the ground point to
/// the goal is walked through the grid, which clamps the goal where it
/// blocks.
pub fn soft_stop(route: &mut AiRoute, body: &mut WalkBody, grid: Option<&NavGrid>) {
    route.following = false;
    route.mode = 0;
    body.flags = (body.flags & !flags::RUN) | flags::STOPPING;
    let limit = STOP_SPEED * body.speed + STOP_BASE_CM;
    if distance2(body.standing_cm, body.goal_cm) > limit * limit {
        body.goal_cm = body.standing_cm + body.frame_forward * limit;
    }
    crate::ground::walk_line(grid, body.ground_cm, &mut body.goal_cm);
}

/// One tick of the path follower (see the module): sets the body's goal to
/// the carrot, the run bit, and stops the route at its end.
#[allow(clippy::neg_cmp_op_on_partial_ord)]
pub fn follow(route: &mut AiRoute, body: &mut WalkBody, in_scene: bool, grid: Option<&NavGrid>) {
    let count = route.points.len();
    if count < 2 {
        route_stop(route, body);
        route.key = 0;
        return;
    }
    let p = route.points.clone();
    let mut key: usize = 1;
    if !in_scene {
        // [Exarust: a key outside the route is held to its segments; the
        // original reads past the points.]
        key = (route.key as usize).clamp(1, count - 1);
        let d = p[key] - p[key - 1];
        let h = flat2(d);
        let length = if h > EPS { h.sqrt() } else { SHORT_SPAN };
        let f = route.fraction;
        if length > f * length + SLIDE_CM {
            route.fraction = (f * length + SLIDE_CM) / length;
        } else if key + 1 < count {
            let ahead = 1.0 - (1.0 - f) * length;
            let left = ahead - (f * ahead + SLIDE_CM);
            let next = p[key + 1] - p[key];
            let span = (next.x * next.x + next.z * next.z).sqrt();
            key += 1;
            route.fraction = left / span;
        } else {
            // [Exarust: past the last segment the slide stays at its end.]
            route.fraction = 1.0;
        }
        route.key = key as u32;
        let slid = lerp(p[key - 1], p[key], route.fraction);
        body.standing_cm = slid;
        body.standing_cm.y = slid.y + body.pelvis_cm;
        body.ground_cm = slid;
    } else {
        let ground = body.ground_cm;
        let mut best = FAR;
        let mut t_best = 0.0;
        for k in 1..count {
            let d = p[k] - p[k - 1];
            let g = ground - p[k - 1];
            let h = d.z * d.z + d.x * d.x;
            // A NaN length is measured.
            if h < EPS {
                continue;
            }
            let t = (d.z * g.z + d.x * g.x) / h;
            let clamped = if t > 1.0 {
                1.0
            } else if !(t < 0.0) {
                t
            } else {
                0.0
            };
            let q = p[k - 1] + d * clamped;
            let e = q - ground;
            let dist = e.z * e.z + (e.y * e.y + e.x * e.x);
            if best > dist {
                best = dist;
                t_best = t;
                key = k;
            }
        }
        route.key = key as u32;
        route.fraction = t_best;
    }
    let fraction = route.fraction;
    let d = p[key] - p[key - 1];
    let h = flat2(d);
    if h < EPS {
        route_stop(route, body);
        return;
    }
    let length = h.sqrt();
    let (end, step) = if length > fraction * length + CARROT_CM {
        (key, (fraction * length + CARROT_CM) / length)
    } else {
        let mut left = CARROT_CM - (1.0 - fraction) * length;
        let mut end = count - 1;
        let mut step = 1.0;
        for k in key + 1..count {
            let h = flat2(p[k] - p[k - 1]);
            if h < EPS {
                continue;
            }
            let span = h.sqrt();
            if left < span {
                step = left / span;
                end = k;
                break;
            }
            left -= span;
        }
        (end, step)
    };
    // A step of no length ends the route (a NaN one goes on).
    if step.abs() < EPS {
        route_stop(route, body);
        return;
    }
    let carrot = p[end - 1] + (p[end] - p[end - 1]) * step;
    body.goal_cm = carrot;
    if route.urgency > 0 {
        body.flags &= !flags::RUN;
        if distance2(body.ground_cm, p[count - 1]) < STOP_AT_END {
            soft_stop(route, body, grid);
        }
    } else if end == count - 1 && f64::from(step) > RUN_UNTIL {
        body.flags &= !flags::RUN;
        if distance2(body.ground_cm, carrot) < STOP_AT_CARROT {
            soft_stop(route, body, grid);
        }
    } else {
        body.flags |= flags::RUN;
    }
    if in_scene {
        let mut to = carrot;
        if crate::ground::walk_line(grid, body.ground_cm, &mut to) {
            body.flags &= !flags::RUN;
            route.wants_plan = true;
        }
    }
}

/// What the walk is asked besides the route and the body.
#[derive(Clone, Copy)]
pub struct WalkAsk<'a> {
    /// The character is in the scene.
    pub in_scene: bool,
    /// The AI's look target's point (cm), when it has one.
    pub look_at_cm: Option<Vec3>,
    /// The nearby bodies the way test reads, and those it skips.
    pub bodies: &'a [BodyAt],
    pub skip: &'a [usize],
    /// The level's navigation grid.
    pub grid: Option<&'a NavGrid>,
}

/// What one walk did: the walk's answer (0 no route and nothing counted, 1
/// walked, 2 counting down after a route), the direction it measured to the
/// goal (the way test's), and the body it found in the way.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Walked {
    pub answer: u32,
    pub direction: Vec3,
    pub in_way: Option<usize>,
}

/// The grid's cost at a point (cm): the clearance plus the penalty, 0
/// without a grid.
fn cost_at(grid: Option<&NavGrid>, at_cm: Vec3) -> f32 {
    crate::drive::grid_cost(grid, at_cm / CM)
}

/// One walk (see the module).
#[allow(clippy::neg_cmp_op_on_partial_ord)]
pub fn walk(route: &mut AiRoute, body: &mut WalkBody, gaze: &mut Gaze, ask: WalkAsk) -> Walked {
    let mut walked = Walked {
        answer: 1,
        direction: body.frame_forward,
        in_way: None,
    };
    if !route.following {
        if route.after_ticks == 0 {
            walked.answer = 0;
        } else {
            route.after_ticks -= 1;
            walked.answer = 2;
        }
        return walked;
    }
    if body.stepping {
        return walked;
    }
    follow(route, body, ask.in_scene, ask.grid);
    if !route.following {
        route.after_ticks = AFTER_TICKS;
    }
    if !ask.in_scene {
        body.moving = true;
        return walked;
    }
    let mut d = body.goal_cm - body.standing_cm;
    d.y = 0.0;
    let squared = d.z * d.z + (d.y * d.y + d.x * d.x);
    // Within 1 cm (or a NaN square) the frame's forward is the direction.
    let (length, direction, align) = if !(squared > 1.0) {
        (1.0, body.frame_forward, 0.0)
    } else {
        let length = squared.sqrt();
        let p = body.pelvis_forward;
        let align = (d.z * p.z + (d.y * p.y + d.x * p.x)) / length;
        (length, d * (1.0 / length), align)
    };
    walked.direction = direction;
    body.heading = motion::unit(ease(body.heading, direction, EASE));
    let gate = if length > GATE_FROM_CM {
        GATE_CAP
    } else {
        (f64::from(length) * GATE_SLOPE) as f32
    };
    if gate < align && route.following {
        body.goal_cm = body.standing_cm + d * (REACH_CM / length);
        let run = route.urgency > 0 && !(cost_at(ask.grid, body.goal_cm) > RUN_COST);
        body.moving = true;
        body.flags = flags::MOVE;
        if run && f64::from(align) > RUN_ALIGN {
            body.flags |= flags::RUN;
            if route.urgency == 2 {
                body.flags |= flags::SPRINT;
            }
        }
    } else {
        body.flags = flags::MOVE;
        body.moving = gate * 0.5 < align;
    }
    gaze.look = motion::yaw_of(body.heading);
    body.look = motion::unit(ease(body.look, body.heading, EASE));
    let looked = ask
        .look_at_cm
        .and_then(|point| motion::look_at(gaze, body.standing_cm, point));
    if looked.is_none() {
        gaze.target_offset = 0.0;
        gaze.offset = (f64::from(gaze.target_offset - gaze.offset) * HEAD_EASE
            + f64::from(gaze.offset)) as f32;
    }
    body.cursor = motion::gap(gaze.look + gaze.offset, 0.0);
    walked.in_way = motion::body_in_way(body.standing_cm, direction, ask.bodies, ask.skip);
    if walked.in_way.is_some() {
        soft_stop(route, body, ask.grid);
    }
    walked
}

/// The body's request side of a stop a state makes when it loses the
/// character: the soft stop on the request and the driver (the route
/// itself is ended by the caller).
pub fn stop_request(
    route: &mut AiRoute,
    req: &mut exa_physics::body::locomotion::Locomotion,
    driver: &mut Driver,
    at: &DriveBody,
    grid: Option<&NavGrid>,
) {
    let mut body = request_body(req, driver, at);
    soft_stop(route, &mut body, grid);
    req.flags = body.flags;
    req.goal = body.goal_cm / CM;
    req.free_walk = false;
    driver.driving = false;
}

/// The walk's body from a character on its locomotion: the standing and
/// ground points are the root's place (Exarust's), the vectors the
/// driver's (taken from the request when it is not driving yet).
fn request_body(
    req: &exa_physics::body::locomotion::Locomotion,
    driver: &mut Driver,
    at: &DriveBody,
) -> WalkBody {
    if !driver.driving {
        driver.heading = forward(req.heading);
        driver.look = forward(req.look);
    }
    let standing = at.standing * CM;
    WalkBody {
        ground_cm: standing,
        standing_cm: standing,
        pelvis_cm: 0.0,
        goal_cm: req.goal * CM,
        flags: req.flags,
        moving: req.moving,
        stepping: at.stepping,
        speed: at.speed,
        frame_forward: at.frame_forward,
        pelvis_forward: at.pelvis_forward,
        heading: driver.heading,
        look: driver.look,
        cursor: req.cursor,
    }
}

/// The idle states turn slowly (the exploration behaviour's slow turn);
/// the flight and the chase do not [Exarust: the request's slow-turn bit is
/// Exarust's].
fn slow_turn(owner: Option<Walker>) -> bool {
    matches!(owner, Some(Walker::Roam | Walker::Follow))
}

/// The stand-in's walk clip and order urgency for a body without muscles.
fn stand_in(owner: Option<Walker>, urgency: i32) -> (&'static str, u8) {
    match owner {
        Some(Walker::Flight) => ("Run", 1),
        Some(Walker::Chase) => (crate::COMBAT_WALK, 0),
        Some(Walker::Follow) => ("Walk", urgency.clamp(0, 2) as u8),
        _ => ("Walk", 0),
    }
}

/// The AI tick's walk for every character: after every state of the
/// brains' tick has run, a character whose route is followed walks it
/// ([`walk`]) once a body tick, on its locomotion request when its body
/// has muscles, else by the stand-in. The scene's node move (leaving or
/// entering) holds the walk for its tick; out of the scene the follower
/// slides the character along its route and the root is placed there.
/// A body without muscles whose route ended stops and leaves its walk
/// clip. A frame between ticks only puts back the stand-in's velocity.
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub fn walk_routes(
    ticks: Option<Res<BodyTicks>>,
    mut clock: Option<ResMut<crate::brain::BrainClock>>,
    nav: Option<Res<NavGrid>>,
    mut walkers: Query<(
        &mut AiRoute,
        Option<&mut crate::brain::Npc>,
        Option<&crate::combatant::Combatant>,
        Option<&crate::presence::Presence>,
        &ChildOf,
        &mut Transform,
        Option<&mut Animator>,
        Option<Walking>,
    )>,
    mut roots: Query<
        (&mut CharacterBody, &GlobalTransform, Option<&mut Transform>),
        Without<AiRoute>,
    >,
    others: Query<(&GlobalTransform, &Condition)>,
    targets: Query<&GlobalTransform>,
) {
    let mut step = true;
    if let (Some(ticks), Some(clock)) = (ticks.as_deref(), clock.as_deref_mut()) {
        step = clock.routed != ticks.0;
        clock.routed = ticks.0;
    }
    let now = ticks.as_ref().map_or(0, |t| t.millis() as u32);
    let grid = nav.as_deref();
    for (mut route, npc, combatant, presence, parent, mut t, anim, walking) in &mut walkers {
        let Ok((mut root, root_gt, root_t)) = roots.get_mut(parent.parent()) else {
            continue;
        };
        if !step {
            if route.following && route.velocity != Vec3::ZERO {
                root.desired_velocity = route.velocity;
            }
            continue;
        }
        let route = &mut *route;
        if !route.following {
            // A stand-in walk that ended stops its body and its clip.
            if route.velocity != Vec3::ZERO {
                route.velocity = Vec3::ZERO;
                root.desired_velocity = Vec3::ZERO;
                let (clip, _) = stand_in(route.owner, route.urgency);
                if let Some(mut anim) = anim
                    && anim.action == clip
                    && clip != crate::COMBAT_WALK
                {
                    anim.play_loop("Idle");
                }
            }
            continue;
        }
        use crate::presence::PresenceState;
        let state = presence.map_or(PresenceState::Present, |p| p.state);
        if matches!(state, PresenceState::Leaving | PresenceState::Entering) {
            continue;
        }
        let in_scene = state == PresenceState::Present;
        let feet = root_gt.translation();
        let feet_cm = feet * CM;
        if route.give_up_ms.is_some_and(|until| until < now) {
            route.stop();
            if let Some((mut req, motor, mut driven, ragdoll)) = walking {
                let at = DriveBody::of(feet, motor, ragdoll);
                stop_request(route, &mut req, &mut driven.0, &at, grid);
            }
            continue;
        }
        let look_at_cm = combatant
            .and_then(|c| c.look_target)
            .and_then(|e| targets.get(e).ok())
            .map(|g| g.translation() * CM);
        let nearby: Vec<(u64, Vec3, bool)> = npc.as_ref().map_or_else(Vec::new, |n| {
            crate::brain::nearby_bodies(&n.nearby, &others)
        });
        let bodies: Vec<BodyAt> = nearby
            .iter()
            .enumerate()
            .map(|(i, (_, p, dead))| BodyAt {
                id: i,
                position_cm: *p,
                dead: *dead,
            })
            .collect();
        let mut npc = npc;
        if let Some((mut req, motor, mut driven, ragdoll)) = walking {
            let at = DriveBody::of(feet, motor, ragdoll);
            let driver = &mut driven.0;
            let skip: Vec<usize> = nearby
                .iter()
                .enumerate()
                .filter(|(_, (id, ..))| Some(*id) == driver.in_way || Some(*id) == route.passes)
                .map(|(i, _)| i)
                .collect();
            let mut body = request_body(&req, driver, &at);
            let mut own = driver.gaze;
            let gaze = match npc.as_deref_mut() {
                Some(n) => &mut n.gaze,
                None => &mut own,
            };
            let walked = walk(
                route,
                &mut body,
                gaze,
                WalkAsk {
                    in_scene,
                    look_at_cm,
                    bodies: &bodies,
                    skip: &skip,
                    grid,
                },
            );
            driver.gaze = own;
            if !in_scene && let Some(mut root_t) = root_t {
                root_t.translation = body.ground_cm / CM;
            }
            req.goal = body.goal_cm / CM;
            req.flags = body.flags;
            req.moving = body.moving;
            req.free_walk = false;
            req.slow_turn = slow_turn(route.owner);
            if in_scene && !body.stepping {
                driver.heading = body.heading;
                driver.look = body.look;
                driver.dir = walked.direction;
                let yaw = motion::yaw_of(body.heading);
                let look = motion::yaw_of(body.look);
                req.heading = yaw;
                req.look = look;
                req.desired = look;
                req.cursor = body.cursor;
            }
            if let Some(i) = walked.in_way {
                driver.in_way = Some(nearby[i].0);
            }
            driver.driving = route.following;
            continue;
        }
        // A body without muscles: the follower's goal through the NPC
        // driver's stand-in.
        let facing = npc.as_ref().map_or_else(
            || motion::yaw_of(t.rotation * Vec3::Z),
            |n| n.facing(t.rotation),
        );
        let mut body = WalkBody::standing(feet_cm, motion::dir_of(facing));
        follow(route, &mut body, in_scene, grid);
        if !in_scene {
            if let Some(mut root_t) = root_t {
                root_t.translation = body.ground_cm / CM;
            }
            continue;
        }
        let (clip, urgency) = stand_in(route.owner, route.urgency);
        if !route.following {
            route.after_ticks = AFTER_TICKS;
            route.velocity = Vec3::ZERO;
            root.desired_velocity = Vec3::ZERO;
            if let Some(mut anim) = anim
                && anim.action == clip
                && clip != crate::COMBAT_WALK
            {
                anim.play_loop("Idle");
            }
            continue;
        }
        let order = crate::order::follow(&[feet, body.goal_cm / CM], feet, facing, urgency);
        root.desired_velocity = order.velocity;
        route.velocity = order.velocity;
        if route.owner != Some(Walker::Chase) {
            match npc.as_deref_mut() {
                Some(n) => n.turn_model(&mut t, order.heading),
                None => t.rotation = Quat::from_rotation_y(order.heading),
            }
        }
        if let Some(mut anim) = anim
            && order.velocity != Vec3::ZERO
            && anim.action != clip
        {
            anim.play_loop(clip);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn route(points: Vec<Vec3>, urgency: i32) -> AiRoute {
        let mut r = AiRoute::default();
        r.start(points, urgency, Walker::Roam);
        r
    }

    fn at(p: Vec3) -> WalkBody {
        WalkBody::standing(p, Vec3::X)
    }

    /// The carrot lies 100 cm past the ground point's projection, spilling
    /// onto the next segment; the run bit is set at urgency 0 away from
    /// the end.
    #[test]
    fn the_carrot_leads_a_metre_along_the_route() {
        let mut r = route(
            vec![
                Vec3::ZERO,
                Vec3::new(150.0, 0.0, 0.0),
                Vec3::new(150.0, 0.0, 300.0),
            ],
            0,
        );
        let mut body = at(Vec3::new(100.0, 0.0, 20.0));
        follow(&mut r, &mut body, true, None);
        assert_eq!(r.key, 1);
        assert!((r.fraction - 100.0 / 150.0).abs() < 1e-6);
        // 50 cm left on the first segment, 50 cm onto the second.
        assert!((body.goal_cm - Vec3::new(150.0, 0.0, 50.0)).length() < 1e-4);
        assert_eq!(body.flags & flags::RUN, flags::RUN);
        assert!(r.following);
    }

    /// At urgency 0 the carrot at the route's end clears the run bit and
    /// a ground point within 30 cm of it soft-stops; above 0 a ground point
    /// within a metre of the end soft-stops.
    #[test]
    fn the_follower_stops_at_its_end() {
        let points = vec![Vec3::ZERO, Vec3::new(500.0, 0.0, 0.0)];
        // 120 cm short: the carrot 0.96 along the segment, the run holds.
        let mut r = route(points.clone(), 0);
        let mut body = at(Vec3::new(380.0, 0.0, 0.0));
        body.flags = flags::MOVE;
        follow(&mut r, &mut body, true, None);
        assert!(r.following);
        assert_eq!(body.flags, flags::MOVE | flags::RUN);
        // 104 cm short: the carrot past 0.99, the run clears; 100 cm from
        // the carrot is no stop.
        let mut body = at(Vec3::new(396.0, 0.0, 0.0));
        body.flags = flags::MOVE | flags::RUN;
        follow(&mut r, &mut body, true, None);
        assert!(r.following);
        assert_eq!(body.flags, flags::MOVE);
        // 20 cm short: soft stop, the goal pulled back to 20 cm (speed 0).
        let mut body = at(Vec3::new(480.0, 0.0, 0.0));
        follow(&mut r, &mut body, true, None);
        assert!(!r.following);
        assert_eq!(body.flags & flags::STOPPING, flags::STOPPING);
        assert_eq!(body.goal_cm, Vec3::new(500.0, 0.0, 0.0));
        // Hurried: 99 cm from the end stops, 101 cm does not.
        let mut r = route(points.clone(), 1);
        let mut body = at(Vec3::new(399.0, 0.0, 0.0));
        follow(&mut r, &mut body, true, None);
        assert!(r.following);
        let mut body = at(Vec3::new(401.0, 0.0, 0.0));
        follow(&mut r, &mut body, true, None);
        assert!(!r.following);
        // The goal is the carrot (the last point), within the 20 cm pull.
        assert!((body.goal_cm - Vec3::new(421.0, 0.0, 0.0)).length() < 1e-4);
    }

    /// Out of the scene the standing point slides 2 cm a tick along the
    /// route and crosses onto the next segment as the original's sums do.
    #[test]
    fn out_of_the_scene_the_follower_slides_two_centimetres() {
        let mut r = route(
            vec![
                Vec3::ZERO,
                Vec3::new(3.0, 0.0, 0.0),
                Vec3::new(3.0, 0.0, 400.0),
            ],
            0,
        );
        let mut body = at(Vec3::ZERO);
        follow(&mut r, &mut body, false, None);
        assert_eq!(r.key, 1);
        assert!((r.fraction - 2.0 / 3.0).abs() < 1e-6);
        assert!((body.ground_cm - Vec3::new(2.0, 0.0, 0.0)).length() < 1e-5);
        // The next slide crosses: 1 − (1 − f)·3, then (a − (f·a + 2)) / 400.
        follow(&mut r, &mut body, false, None);
        let f = 2.0f32 / 3.0;
        let a = 1.0 - (1.0 - f) * 3.0;
        let expected = (a - (f * a + 2.0)) / 400.0;
        assert_eq!(r.key, 2);
        assert_eq!(r.fraction, expected);
    }

    /// A route of one point stops; a carrot step of no length stops.
    #[test]
    fn a_route_without_a_segment_stops() {
        let mut r = route(vec![Vec3::ZERO], 0);
        let mut body = at(Vec3::ZERO);
        body.flags = flags::RUN;
        follow(&mut r, &mut body, true, None);
        assert!(!r.following && r.key == 0 && body.flags == 0);
        let mut r = route(vec![Vec3::ZERO, Vec3::new(0.0, 50.0, 0.0)], 0);
        follow(&mut r, &mut body, true, None);
        assert!(!r.following);
    }

    /// The walk holds a far goal 150 cm out, runs only with urgency on
    /// cheap ground and the alignment past 0.9, sprints at urgency 2, and
    /// counts 80 ticks once its route ends.
    #[test]
    fn the_walk_aims_holds_runs_and_counts_down() {
        let points = vec![Vec3::ZERO, Vec3::new(1000.0, 0.0, 0.0)];
        for (urgency, expect) in [
            (0, flags::MOVE),
            (1, flags::MOVE | flags::RUN),
            (2, flags::MOVE | flags::RUN | flags::SPRINT),
        ] {
            let mut r = route(points.clone(), urgency);
            let mut body = at(Vec3::ZERO);
            let mut gaze = Gaze::at(0.0);
            let walked = walk(
                &mut r,
                &mut body,
                &mut gaze,
                WalkAsk {
                    in_scene: true,
                    look_at_cm: None,
                    bodies: &[],
                    skip: &[],
                    grid: None,
                },
            );
            assert_eq!(walked.answer, 1);
            assert_eq!(body.flags, expect, "urgency {urgency}");
            assert!(body.moving);
            assert!((body.goal_cm - Vec3::new(150.0, 0.0, 0.0)).length() < 1e-4);
            assert!((gaze.look - std::f32::consts::FRAC_PI_2).abs() < 1e-3);
        }
        // At the end: the follower stops, the walk asks only to move and
        // counts down.
        let mut r = route(points, 0);
        let mut body = at(Vec3::new(990.0, 0.0, 0.0));
        let mut gaze = Gaze::at(0.0);
        let ask = WalkAsk {
            in_scene: true,
            look_at_cm: None,
            bodies: &[],
            skip: &[],
            grid: None,
        };
        walk(&mut r, &mut body, &mut gaze, ask);
        assert!(!r.following);
        assert_eq!(body.flags, flags::MOVE);
        assert_eq!(r.after_ticks, AFTER_TICKS);
        assert_eq!(walk(&mut r, &mut body, &mut gaze, ask).answer, 2);
        assert_eq!(r.after_ticks, AFTER_TICKS - 1);
        r.after_ticks = 0;
        assert_eq!(walk(&mut r, &mut body, &mut gaze, ask).answer, 0);
    }

    /// The head looks at a target in front and eases back toward the look
    /// with none; a body in the way soft-stops the walk.
    #[test]
    fn the_head_follows_the_look_target_and_a_body_in_the_way_stops() {
        let points = vec![Vec3::ZERO, Vec3::new(0.0, 0.0, 1000.0)];
        let mut r = route(points.clone(), 0);
        let mut body = WalkBody::standing(Vec3::ZERO, Vec3::Z);
        let mut gaze = Gaze::at(0.0);
        let ask = WalkAsk {
            in_scene: true,
            look_at_cm: Some(Vec3::new(100.0, 0.0, 100.0)),
            bodies: &[],
            skip: &[],
            grid: None,
        };
        walk(&mut r, &mut body, &mut gaze, ask);
        // The target is a quarter turn's half to the right: the offset
        // eases 0.02 of it.
        let target = motion::yaw_of(Vec3::new(100.0, 0.0, 100.0));
        assert_eq!(gaze.target_offset, target);
        assert_eq!(gaze.offset, (f64::from(target) * 0.02) as f32);
        assert_eq!(body.cursor, motion::gap(gaze.look + gaze.offset, 0.0));
        // With no target the offset eases back 0.02 a tick.
        let held = gaze.offset;
        walk(
            &mut r,
            &mut body,
            &mut gaze,
            WalkAsk {
                look_at_cm: None,
                ..ask
            },
        );
        assert_eq!(gaze.target_offset, 0.0);
        assert_eq!(
            gaze.offset,
            (f64::from(0.0 - held) * 0.02 + f64::from(held)) as f32
        );
        // A body 60 cm ahead is in the way: the walk soft-stops.
        let mut r = route(points, 0);
        let mut body = WalkBody::standing(Vec3::ZERO, Vec3::Z);
        let bodies = [BodyAt {
            id: 0,
            position_cm: Vec3::new(0.0, 0.0, 60.0),
            dead: false,
        }];
        let walked = walk(
            &mut r,
            &mut body,
            &mut gaze,
            WalkAsk {
                bodies: &bodies,
                look_at_cm: None,
                ..ask
            },
        );
        assert_eq!(walked.in_way, Some(0));
        assert!(!r.following);
        assert_eq!(body.flags & flags::STOPPING, flags::STOPPING);
    }

    /// The soft stop pulls a far goal to `8·speed + 20` cm along the
    /// frame's forward; a near one stays.
    #[test]
    fn the_soft_stop_pulls_the_goal_in() {
        let mut r = route(vec![Vec3::ZERO, Vec3::new(0.0, 0.0, 500.0)], 0);
        let mut body = WalkBody::standing(Vec3::ZERO, Vec3::Z);
        body.goal_cm = Vec3::new(0.0, 0.0, 100.0);
        body.speed = 0.5;
        body.flags = flags::MOVE | flags::RUN;
        soft_stop(&mut r, &mut body, None);
        assert_eq!(body.flags, flags::MOVE | flags::STOPPING);
        assert_eq!(body.goal_cm, Vec3::new(0.0, 0.0, 24.0));
        assert!(!r.following);
        body.goal_cm = Vec3::new(0.0, 0.0, 10.0);
        soft_stop(&mut r, &mut body, None);
        assert_eq!(body.goal_cm, Vec3::new(0.0, 0.0, 10.0));
    }

    /// A flat grid with its floors at voxel 100 (10 m over base 0) and,
    /// when `wall`, a wall 3 m higher along cell column 24.
    fn ground(wall: bool) -> NavGrid {
        use exa_world::nav::{Layer, NavCell};
        let mut grid = NavGrid::empty(40, 40, 0.0);
        for z in 0..40 {
            for x in 0..40 {
                let floor = if wall && x == 24 { 130 } else { 100 };
                grid.set_cell(
                    x,
                    z,
                    Layer::Lower,
                    NavCell {
                        floor,
                        ..NavCell::default()
                    },
                );
            }
        }
        grid
    }

    fn cell_cm(grid: &NavGrid, x: usize, z: usize) -> Vec3 {
        let c = grid.cell_centre(x, z);
        Vec3::new(c.x * CM, 1000.0, c.y * CM)
    }

    /// In the scene a line to the carrot the grid blocks clears the run bit
    /// and asks for a plan; the carrot itself is the goal still. Out of the
    /// scene the line is not walked.
    #[test]
    fn a_blocked_line_to_the_carrot_asks_for_a_plan() {
        let grid = ground(true);
        // The route runs along the row into the wall, 3 cells from it.
        let from = cell_cm(&grid, 21, 20);
        let to = cell_cm(&grid, 30, 20);
        let mut r = route(vec![from, to], 0);
        let mut body = at(from);
        follow(&mut r, &mut body, true, Some(&grid));
        assert!(r.wants_plan && r.following);
        assert_eq!(body.flags & flags::RUN, 0);
        assert!((body.goal_cm - (from + Vec3::new(100.0, 0.0, 0.0))).length() < 1e-3);
        // On open ground the line is clear: the run holds, no plan.
        let open = ground(false);
        let mut r = route(vec![from, to], 0);
        let mut body = at(from);
        follow(&mut r, &mut body, true, Some(&open));
        assert!(!r.wants_plan);
        assert_eq!(body.flags & flags::RUN, flags::RUN);
        // Out of the scene: no line walk.
        let mut r = route(vec![from, to], 0);
        let mut body = at(from);
        follow(&mut r, &mut body, false, Some(&grid));
        assert!(!r.wants_plan);
    }

    /// The real AI plugin's brains' tick: a roaming character on its
    /// locomotion plans its leg into the character's route, and the AI's
    /// walk (after every state) writes the request as the original's walk
    /// does from the follower's carrot: the goal 150 cm out along the carrot
    /// (aligned past 0.7), the move flag alone at urgency 0, the heading a
    /// tenth of the way toward the carrot and the AI's look yaw on it. At
    /// the leg's end the follower soft-stops, the walk's flags (written
    /// after) keep only the move bit, the after-route count is 80 and the
    /// roam settles on the next tick.
    #[test]
    fn the_ai_tick_walks_the_roam_leg_through_the_one_walk() {
        use exa_formats::level::LevelGrid;
        use exa_physics::body::locomotion::Locomotion;
        use exa_world::room_view::LevelRooms;
        use exa_world::rooms::{CELL_OPEN, Rooms};
        use std::sync::Arc;

        let mut level = LevelGrid {
            wall_sets: vec!["GEWS_UWA01".into()],
            floor_sets: vec!["GEFS_UWA01".into()],
            width: 6,
            height: 3,
            edges: vec![0; 18],
            plane_b: vec![0; 18],
            plane_c: vec![0; 18],
        };
        let open = |edges: &mut Vec<u32>, x: u32, y: u32, sides: [u32; 4]| {
            edges[(y * 6 + x) as usize] =
                CELL_OPEN | sides[0] | sides[1] << 4 | sides[2] << 8 | sides[3] << 12;
        };
        open(&mut level.edges, 1, 1, [7, 7, 7, 0]);
        open(&mut level.edges, 2, 1, [7, 0, 7, 0xB]);
        open(&mut level.edges, 3, 1, [7, 0xB, 7, 0]);
        open(&mut level.edges, 4, 1, [7, 0, 7, 7]);
        let mut nav = NavGrid::empty(80, 80, 0.0);
        for z in 0..80 {
            for x in 0..80 {
                nav.set_cell(
                    x,
                    z,
                    exa_world::nav::Layer::Lower,
                    exa_world::nav::NavCell {
                        floor: 100,
                        ..exa_world::nav::NavCell::default()
                    },
                );
            }
        }
        let mut app = App::new();
        app.add_plugins((bevy::MinimalPlugins, crate::AiPlugin));
        app.insert_resource(LevelRooms::new(Arc::new(Rooms::flood(&level))));
        app.insert_resource(nav);
        app.insert_resource(BodyTicks(1));
        let at = Vec3::new(-9.0, 10.0, 3.0);
        let target = app
            .world_mut()
            .spawn(GlobalTransform::from_translation(Vec3::new(
                50.0, 10.0, 50.0,
            )))
            .id();
        let root = app
            .world_mut()
            .spawn((
                CharacterBody::default(),
                GlobalTransform::from_translation(at),
                Transform::from_translation(at),
            ))
            .id();
        let mut brain = crate::brain::Npc::new([0.0; 4]);
        brain.routine = crate::idle::Routine::roam(0);
        brain.level_room = 0;
        // It faces its door (+x), as its body would once turned.
        let (req, motor) = Locomotion::facing(std::f32::consts::FRAC_PI_2);
        let npc = app
            .world_mut()
            .spawn((
                brain,
                crate::Duellist::new(target),
                Condition::NORMAL,
                exa_combat::Fighter::default(),
                GlobalTransform::from_translation(at),
                Transform::default(),
                ChildOf(root),
                Animator::new("Default", "Idle"),
                (req, motor, crate::drive::Driven::default()),
            ))
            .id();
        let tick = |app: &mut App| {
            app.world_mut().resource_mut::<BodyTicks>().0 += 1;
            app.world_mut().run_schedule(crate::BrainChain);
        };
        // Tick until the leg is planned; the walk runs in the same tick.
        let mut before = None;
        for _ in 0..6 {
            let req = *app.world().get::<Locomotion>(npc).unwrap();
            tick(&mut app);
            if app.world().get::<AiRoute>(npc).unwrap().following {
                before = Some(req);
                break;
            }
        }
        let before = before.expect("the roam planned its leg");
        let route = app.world().get::<AiRoute>(npc).unwrap().clone();
        assert!(route.follows(Walker::Roam));
        assert_eq!(route.urgency, 0);
        // The follower's carrot from where the character stands.
        let feet_cm = at * CM;
        let mut probe = route.clone();
        probe.key = 1;
        probe.fraction = 0.0;
        let mut body = WalkBody::standing(feet_cm, Vec3::X);
        follow(&mut probe, &mut body, true, None);
        let carrot = body.goal_cm;
        // The walk's numbers, by the original's sums.
        let mut d = carrot - feet_cm;
        d.y = 0.0;
        let len = (d.z * d.z + (d.y * d.y + d.x * d.x)).sqrt();
        let pelvis = forward(std::f32::consts::FRAC_PI_2);
        let align = (d.z * pelvis.z + (d.y * pelvis.y + d.x * pelvis.x)) / len;
        assert!(align > 0.7, "{align}");
        let req = app.world().get::<Locomotion>(npc).unwrap();
        let goal_cm = feet_cm + d * (150.0 / len);
        assert!((req.goal * CM - goal_cm).length() < 1e-3, "{:?}", req.goal);
        assert_eq!(req.flags, flags::MOVE);
        assert!(req.moving);
        let heading = motion::unit(ease(forward(before.heading), d * (1.0 / len), 0.1));
        assert_eq!(req.heading, motion::yaw_of(heading));
        let gaze = app.world().get::<crate::brain::Npc>(npc).unwrap().gaze;
        assert_eq!(gaze.look, req.heading);
        assert!(
            app.world()
                .get::<crate::drive::Driven>(npc)
                .unwrap()
                .0
                .driving
        );
        // At the leg's last point the follower soft-stops.
        let last = *route.points.last().unwrap();
        let end = last / CM;
        *app.world_mut().get_mut::<GlobalTransform>(root).unwrap() =
            GlobalTransform::from_translation(end);
        tick(&mut app);
        let walked = app.world().get::<AiRoute>(npc).unwrap();
        assert!(!walked.following);
        assert_eq!(walked.after_ticks, AFTER_TICKS);
        let req = app.world().get::<Locomotion>(npc).unwrap();
        assert_eq!(req.flags, flags::MOVE);
        assert!(!req.moving);
        assert!((req.goal - end).length() < 1e-5);
        assert!(
            !app.world()
                .get::<crate::drive::Driven>(npc)
                .unwrap()
                .0
                .driving
        );
        // The roam settles and pauses before its home-side leg.
        tick(&mut app);
        let brain = app.world().get::<crate::brain::Npc>(npc).unwrap();
        assert_eq!(brain.routine.phase, crate::idle::Phase::WaitHome);
    }

    /// The real AI plugin's brains' tick stops the route of a character
    /// that is down, whichever state planned it.
    #[test]
    fn the_ai_tick_stops_a_downed_characters_route() {
        for (condition, followed) in [(Condition::NORMAL, true), (Condition::DOWN, false)] {
            let mut app = App::new();
            app.add_plugins((bevy::MinimalPlugins, crate::AiPlugin));
            app.insert_resource(BodyTicks(1));
            let target = app.world_mut().spawn(GlobalTransform::IDENTITY).id();
            let root = app
                .world_mut()
                .spawn((CharacterBody::default(), GlobalTransform::IDENTITY))
                .id();
            // A follow's route: no other state of the exa_ai chain ends it.
            let mut route = AiRoute::default();
            route.start(
                vec![Vec3::ZERO, Vec3::new(0.0, 0.0, 800.0)],
                0,
                Walker::Follow,
            );
            let npc = app
                .world_mut()
                .spawn((
                    crate::brain::Npc::new([0.0; 4]),
                    crate::Duellist::new(target),
                    route,
                    condition,
                    exa_combat::Fighter::default(),
                    GlobalTransform::IDENTITY,
                    Transform::default(),
                    ChildOf(root),
                ))
                .id();
            app.world_mut().resource_mut::<BodyTicks>().0 += 1;
            app.world_mut().run_schedule(crate::BrainChain);
            let route = app.world().get::<AiRoute>(npc).unwrap();
            assert_eq!(route.following, followed, "{condition:?}");
        }
    }
}
