//! The AI's locomotion driver: how a brain walks a route on its body.
//!
//! The original's AI does not move its character. Every state that walks a
//! route (the roam's and the patrol's legs, the flight, the combat chase,
//! the search's follow) calls one driver each tick, which writes the body's
//! locomotion request: the movement goal, the heading, the look, the flags
//! and the moving byte, as the player's exploration steering does. The
//! body's movement decision and locomotion routine then turn it and walk it
//! on its feet (`exa_game`'s movement stage).
//!
//! In the scene, with the path follower's goal (its carrot) set:
//! - a body that steps is busy, and nothing is written;
//! - `d` is the goal less the standing point across the ground; its length
//!   (cm, 1 when its square is at most 1, the direction then the frame's
//!   forward and the alignment 0) gives the direction and the alignment
//!   `d·pelvis forward / |d|`;
//! - the heading eases a tenth of the way toward the direction and is made
//!   a unit again (the engine's approximate reciprocal square root);
//! - with `m = 0.007·|d|` (in double; 0.7 beyond 100 cm), an alignment not
//!   over `m` (a NaN one included),
//!   or a route no longer followed, asks only to move (flags 1) and moves
//!   while the alignment passes `m/2`; otherwise the goal goes 150 cm out
//!   from the standing point along `d`, the body moves (flags 1), and it
//!   runs (4) when the urgency is at least 1, the grid's cost at the goal
//!   (clearance plus penalty) is at most 0.5 and the alignment passes 0.9,
//!   with the sprint latch (8) at urgency 2;
//! - the look eases a tenth of the way toward the heading (a unit again);
//!   the raw cursor and the desired direction take it; the head's drift
//!   eases 0.02 of the way toward 0, and the smoothed cursor is the
//!   heading's yaw plus the drift.
//!
//! Out of the scene the driver only sets the moving byte (the path
//! follower slides the body; [`crate::path::slide`]).
//!
//! When a route ends, the soft stop ([`soft_stop`]) clears the run bit and
//! sets the stopping bit, and pulls a goal farther than `8·speed + 20` cm
//! back along the frame's forward: the decision then brakes the body to a
//! stop.
//!
//! Exarust's remainder: the driver's obstacle check (another character
//! ahead on the line stops the walk) is not ported.

use bevy::prelude::*;
use exa_anim::{Animator, Ease};
use exa_combat::stance::Stance;
use exa_physics::body::Ragdoll;
use exa_physics::body::locomotion::{Locomotion, Locomotor, flags, forward};
use exa_physics::engine_math::{heading, reciprocal_sqrt};

/// Centimetres per metre.
const CM: f32 = 100.0;
/// The heading's and the look's ease a tick.
const EASE: f32 = 0.1;
/// The head drift's ease toward 0 a tick.
const HEAD_EASE: f64 = 0.02;
/// The far goal's distance (cm) out from the standing point.
const REACH_CM: f32 = 150.0;
/// The moving gate's slope and cap: `min(0.007·|d|, 0.7)`.
const GATE_SLOPE: f64 = 0.007;
const GATE_CAP: f32 = 0.7;
const GATE_FROM_CM: f32 = 100.0;
/// A run needs this alignment, and the grid's cost at the goal at most this.
const RUN_ALIGN: f64 = 0.9;
const RUN_COST: f32 = 0.5;

/// The driver's own state, kept by the state that walks: the heading and
/// look vectors it eases, and the head's drift. A drive starts from the
/// request's heading and look.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Driver {
    /// A route is being driven: the vectors below are live.
    pub driving: bool,
    pub heading: Vec3,
    pub look: Vec3,
    /// The head's yaw drift (rad), eased toward 0.
    pub head_drift: f32,
    /// The goal's direction across the ground (unit) the last drive
    /// measured: the obstacle check walks along it.
    pub dir: Vec3,
    /// The character the obstacle check last stopped the walk for (its
    /// entity bits): it is skipped until the brain plans again.
    pub in_way: Option<u64>,
}

/// The driver's state on a character whose body walks by its locomotion
/// (one with muscles): [`add_locomotion`] gives it, with the request.
#[derive(Component, Debug, Clone, Copy, PartialEq, Default)]
pub struct Driven(pub Driver);

/// What a walking state reads and writes of a character on its
/// locomotion: the request, the body's state, the driver and its body.
pub type Walking = (
    &'static mut Locomotion,
    &'static Locomotor,
    &'static mut Driven,
    Option<&'static Ragdoll>,
);

/// The grid's cost at a point (m) the driver's run test reads: the
/// clearance plus the penalty, each 0 to 1 (0 without a grid).
pub fn grid_cost(grid: Option<&exa_world::nav::NavGrid>, at: Vec3) -> f32 {
    grid.map_or(0.0, |g| {
        let (clearance, penalty) = exa_world::nav::cost_at(g, at);
        clearance + penalty
    })
}

/// What the driver reads of the body.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DriveBody {
    /// The standing point (m).
    pub standing: Vec3,
    /// The pelvis's forward and the character frame's forward.
    pub pelvis_forward: Vec3,
    pub frame_forward: Vec3,
    /// A step runs.
    pub stepping: bool,
    /// The speed along the heading (cm a tick), for the soft stop.
    pub speed: f32,
}

impl DriveBody {
    /// The body as the driver reads it: the standing point `standing` (m),
    /// the frame's forward from the body's yaw, the pelvis's forward, its
    /// step and its speed.
    pub fn of(standing: Vec3, body: &Locomotor, ragdoll: Option<&Ragdoll>) -> Self {
        let facing = forward(body.yaw);
        Self {
            standing,
            pelvis_forward: ragdoll.map_or(facing, |r| r.pelvis_forward(facing)),
            frame_forward: facing,
            stepping: ragdoll.is_some_and(Ragdoll::stepping),
            speed: body.speed,
        }
    }
}

/// The engine's in-place unit: the squares summed `(y² + 0) + (x² + z²)`,
/// times their approximate reciprocal square root (0 for a zero vector).
fn unit(v: Vec3) -> Vec3 {
    let s = (v.y * v.y + 0.0) + (v.x * v.x + v.z * v.z);
    let r = if s == 0.0 { 0.0 } else { reciprocal_sqrt(s) };
    v * r
}

/// The engine's ease of `a` toward `b` by `t`, each axis `a + (b − a)·t`.
fn ease(a: Vec3, b: Vec3, t: f32) -> Vec3 {
    Vec3::new(
        a.x + (b.x - a.x) * t,
        a.y + (b.y - a.y) * t,
        a.z + (b.z - a.z) * t,
    )
}

/// One tick of the driver for a body in the scene following its route,
/// whose path follower set the goal `carrot` (m); `following` is false once
/// the follower let the route go. `urgency` is the walk's (0 walk, 1 hurry,
/// 2 flee), `slow_turn` the idle states' slow turn, `cost` the grid's cost
/// at a point (m). Returns false while the body steps (busy, nothing
/// written).
#[allow(clippy::too_many_arguments)]
pub fn drive(
    req: &mut Locomotion,
    driver: &mut Driver,
    body: &DriveBody,
    carrot: Vec3,
    following: bool,
    urgency: i32,
    slow_turn: bool,
    cost: &dyn Fn(Vec3) -> f32,
) -> bool {
    if body.stepping {
        return false;
    }
    if !driver.driving {
        driver.driving = true;
        driver.heading = forward(req.heading);
        driver.look = forward(req.look);
    }
    req.goal = carrot;
    req.free_walk = false;
    req.slow_turn = slow_turn;
    let standing = body.standing * CM;
    let mut d = carrot * CM - standing;
    d.y = 0.0;
    let squared = d.z * d.z + d.x * d.x + 0.0;
    // Within 1 cm (or a NaN square) the frame's forward is the direction.
    #[allow(clippy::neg_cmp_op_on_partial_ord)]
    let (length, dir, align) = if !(squared > 1.0) {
        (1.0, body.frame_forward, 0.0)
    } else {
        let length = squared.sqrt();
        let dir = d * (1.0 / length);
        let p = body.pelvis_forward;
        let align = (d.z * p.z + d.y * p.y + d.x * p.x) / length;
        (length, dir, align)
    };
    driver.dir = dir;
    driver.heading = unit(ease(driver.heading, dir, EASE));
    // The short gate is formed in double from the double 0.007.
    let gate = if length <= GATE_FROM_CM {
        (f64::from(length) * GATE_SLOPE) as f32
    } else {
        GATE_CAP
    };
    // An alignment not over the gate (a NaN one included) only asks to move.
    #[allow(clippy::neg_cmp_op_on_partial_ord)]
    if !(gate < align) || !following {
        req.flags = flags::MOVE;
        req.moving = gate * 0.5 < align;
    } else {
        let goal = standing + d * (REACH_CM / length);
        req.goal = goal / CM;
        let run = urgency >= 1 && cost(req.goal) <= RUN_COST;
        req.moving = true;
        req.flags = flags::MOVE;
        if run && f64::from(align) > RUN_ALIGN {
            req.flags |= flags::RUN;
            if urgency == 2 {
                req.flags |= flags::SPRINT;
            }
        }
    }
    let yaw = heading(driver.heading);
    req.heading = yaw;
    driver.look = unit(ease(driver.look, driver.heading, EASE));
    let look = heading(driver.look);
    req.look = look;
    req.desired = look;
    // The drift eases toward its zeroed target in double.
    driver.head_drift =
        (f64::from(-driver.head_drift) * HEAD_EASE + f64::from(driver.head_drift)) as f32;
    req.cursor = exa_physics::body::locomotion::wrap(yaw + driver.head_drift);
    true
}

/// The driver's obstacle check, after its drive: the first live nearby
/// character (`bodies`, their entity bits and standing points in cm; a
/// dead one skipped) whose point, moved 20 cm along the goal's direction,
/// the walk from `from_cm` along that direction reaches within its 120 cm
/// steer line ([`crate::motion::body_in_way`]), skipping the one it last
/// stopped for. Such a character is remembered and the caller stops the
/// walk: the soft stop, and the route is no longer followed.
pub fn obstacle(driver: &mut Driver, from_cm: Vec3, bodies: &[(u64, Vec3, bool)]) -> bool {
    let at: Vec<crate::motion::BodyAt> = bodies
        .iter()
        .enumerate()
        .map(|(i, (_, p, dead))| crate::motion::BodyAt {
            id: i,
            position_cm: *p,
            dead: *dead,
        })
        .collect();
    let skip: Vec<usize> = bodies
        .iter()
        .position(|(id, ..)| Some(*id) == driver.in_way)
        .into_iter()
        .collect();
    match crate::motion::body_in_way(from_cm, driver.dir, &at, &skip) {
        Some(i) => {
            driver.in_way = Some(bodies[i].0);
            true
        }
        None => false,
    }
}

/// The soft stop the driver ends a route with: the run bit off, the
/// stopping bit on, no free walk, and a goal farther than `8·speed + 20`
/// cm from the standing point pulled back to that along the frame's
/// forward. The driver stops driving.
pub fn soft_stop(req: &mut Locomotion, driver: &mut Driver, body: &DriveBody) {
    driver.driving = false;
    req.free_walk = false;
    req.flags = (req.flags & !flags::RUN) | flags::STOPPING;
    let limit = body.speed * 8.0 + 20.0;
    let standing = body.standing * CM;
    if limit * limit < standing.distance_squared(req.goal * CM) {
        req.goal = (standing + body.frame_forward * limit) / CM;
    }
}

/// Faces a character without walking: the request's heading, look and
/// cursors take `yaw` (the yaw the AI turned its model to), and the body's
/// movement decision turns it there at its rates, as the original's AI
/// writes the heading and lets the body tick turn the body (the idle
/// states at the slow 0.5 rate).
pub fn face(req: &mut Locomotion, yaw: f32) {
    req.heading = yaw;
    req.look = yaw;
    req.cursor = yaw;
    req.desired = yaw;
}

/// Gives every fighter whose body has muscles the original's locomotion
/// request, facing as its model faces, and the driver's state: from then on
/// its body's movement decision turns it and walks it. The facing is the
/// spawn placement's (`Locomotor::placed`): the model's forward flattened,
/// normalised approximately, its yaw by the original's arc cosine.
#[allow(clippy::type_complexity)]
pub fn add_locomotion(
    mut commands: Commands,
    fighters: Query<(Entity, &Transform, &Ragdoll), (With<crate::Duellist>, Without<Locomotion>)>,
) {
    // Debug aid: `EXA_NPC_NO_LOCOMOTION` leaves the fighters without the
    // request (their routes slide as before).
    if std::env::var_os("EXA_NPC_NO_LOCOMOTION").is_some() {
        return;
    }
    for (entity, t, body) in &fighters {
        if body.drive.is_none() {
            continue;
        }
        // The frame stays the model's own turn until the body tick
        // rebuilds it.
        let forward = t.rotation * Vec3::Z;
        let (mut req, mut motor) =
            Locomotion::facing(exa_physics::body::locomotion::yaw_of(forward));
        motor.placed(&mut req, forward);
        commands
            .entity(entity)
            .insert((req, motor, Driven::default()));
    }
}

/// The standing character's facing, after the brains' passes: a character
/// whose route is not driven and whose body does not walk turns toward the
/// yaw the AI turned its model to ([`face`]; the movement stage turns the
/// body at its rates and puts the model back on the body's yaw), and a walk clip its body no longer walks
/// gives way to its stance over 20 frames with the fast ease, as the
/// original's body tick starts the stored stance when it leaves the
/// walking condition.
#[allow(clippy::type_complexity)]
pub fn face_standing(
    mut fighters: Query<(
        &Transform,
        &mut Locomotion,
        &Locomotor,
        &Driven,
        Option<&mut Animator>,
        Option<&Stance>,
    )>,
) {
    for (t, mut req, body, driven, anim, stance) in &mut fighters {
        if driven.0.driving || body.moving() {
            continue;
        }
        let yaw = exa_physics::body::locomotion::yaw_of(t.rotation * Vec3::Z);
        if req.heading != yaw {
            face(&mut req, yaw);
        }
        if let Some(mut anim) = anim
            && anim.looping
            && anim.action == WALK
        {
            let action = stance.map_or(Stance::Idle, |s| *s).action();
            anim.start(action, true, None, WALK_END_BLEND, Ease::Fast);
        }
    }
}

/// The walk clip the locomotion routine starts, and the frames it blends
/// into the stance when the body stops.
const WALK: &str = "Walk";
const WALK_END_BLEND: u32 = 20;

#[cfg(test)]
mod tests {
    use super::*;

    fn body() -> DriveBody {
        DriveBody {
            standing: Vec3::ZERO,
            pelvis_forward: Vec3::Z,
            frame_forward: Vec3::Z,
            stepping: false,
            speed: 0.0,
        }
    }

    /// A goal ahead, well aligned: the goal goes 150 cm out, the body moves
    /// and walks (urgency 0), the heading and look ease toward it.
    #[test]
    fn an_aligned_goal_walks_out_to_150_cm() {
        let mut req = Locomotion::default();
        let mut driver = Driver::default();
        let walked = drive(
            &mut req,
            &mut driver,
            &body(),
            Vec3::new(0.0, 0.0, 1.0),
            true,
            0,
            true,
            &|_| 0.0,
        );
        assert!(walked && req.moving && req.slow_turn && !req.free_walk);
        assert_eq!(req.flags, flags::MOVE);
        assert!((req.goal - Vec3::new(0.0, 0.0, 1.5)).length() < 1e-6);
        assert!(req.heading.abs() < 1e-6);
    }

    /// Urgency runs on a cheap, aligned goal; urgency 2 adds the sprint
    /// latch; a dear goal walks.
    #[test]
    fn urgency_runs_and_sprints_on_cheap_ground() {
        let run = |urgency, cost: f32| {
            let mut req = Locomotion::default();
            let mut driver = Driver::default();
            drive(
                &mut req,
                &mut driver,
                &body(),
                Vec3::new(0.0, 0.0, 3.0),
                true,
                urgency,
                false,
                &|_| cost,
            );
            req.flags
        };
        assert_eq!(run(1, 0.5), flags::MOVE | flags::RUN);
        assert_eq!(run(2, 0.0), flags::MOVE | flags::RUN | flags::SPRINT);
        assert_eq!(run(1, 0.6), flags::MOVE);
        assert_eq!(run(0, 0.0), flags::MOVE);
    }

    /// A goal behind does not pass the gate: the body is asked to move only
    /// once it is turned (the alignment over half the gate), and the goal
    /// stays the carrot; the heading eases a tenth of the way.
    #[test]
    fn a_goal_behind_turns_first() {
        let mut req = Locomotion::default();
        let mut driver = Driver::default();
        let carrot = Vec3::new(0.0, 0.0, -2.0);
        drive(
            &mut req,
            &mut driver,
            &body(),
            carrot,
            true,
            0,
            true,
            &|_| 0.0,
        );
        assert!(!req.moving);
        assert_eq!((req.goal, req.flags), (carrot, flags::MOVE));
        // (0,0,1) + 0.1·((0,0,−1) − (0,0,1)) = (0,0,0.8), a unit again.
        assert!((driver.heading - Vec3::Z).length() < 1e-3);
    }

    /// A character 1 m ahead on the walk stops it, once: the next check
    /// skips it; one beside the line does not.
    #[test]
    fn a_character_ahead_stops_the_walk_once() {
        let mut driver = Driver {
            dir: Vec3::Z,
            ..Driver::default()
        };
        let ahead = [(7u64, Vec3::new(0.0, 0.0, 100.0), false)];
        assert!(obstacle(&mut driver, Vec3::ZERO, &ahead));
        assert_eq!(driver.in_way, Some(7));
        assert!(!obstacle(&mut driver, Vec3::ZERO, &ahead));
        let beside = [(8u64, Vec3::new(100.0, 0.0, 50.0), false)];
        assert!(!obstacle(&mut driver, Vec3::ZERO, &beside));
        let dead = [(9u64, Vec3::new(0.0, 0.0, 100.0), true)];
        assert!(!obstacle(&mut driver, Vec3::ZERO, &dead));
    }

    /// A stepping body is busy: nothing is written.
    #[test]
    fn a_stepping_body_is_busy() {
        let mut req = Locomotion::default();
        let mut driver = Driver::default();
        let stepping = DriveBody {
            stepping: true,
            ..body()
        };
        assert!(!drive(
            &mut req,
            &mut driver,
            &stepping,
            Vec3::Z,
            true,
            0,
            true,
            &|_| 0.0
        ));
        assert_eq!(req, Locomotion::default());
    }

    /// The soft stop clears the run bit, sets the stopping bit and pulls a
    /// far goal to 8·speed + 20 cm along the frame's forward.
    #[test]
    fn the_soft_stop_pulls_a_far_goal_in() {
        let mut req = Locomotion {
            flags: flags::MOVE | flags::RUN,
            goal: Vec3::new(5.0, 0.0, 0.0),
            ..Locomotion::default()
        };
        let mut driver = Driver {
            driving: true,
            ..Driver::default()
        };
        let moving = DriveBody {
            speed: 2.0,
            ..body()
        };
        soft_stop(&mut req, &mut driver, &moving);
        assert_eq!(req.flags, flags::MOVE | flags::STOPPING);
        assert!((req.goal - Vec3::new(0.0, 0.0, 0.36)).length() < 1e-6);
        assert!(!driver.driving);
    }
}
