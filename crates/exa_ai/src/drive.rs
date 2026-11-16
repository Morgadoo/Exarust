//! The AI's side of a character's locomotion: the request its walk writes.
//!
//! The original's AI does not move its character. Its one route walk
//! ([`crate::route_walk::walk`], called after the state while a route is
//! followed) writes the body's locomotion request: the movement goal, the
//! heading, the look, the flags and the moving byte, as the player's
//! exploration steering does. The body's movement decision and locomotion
//! routine then turn it and walk it on its feet (`exa_game`'s movement
//! stage). This module keeps what that walk reads of the body
//! ([`DriveBody`]), the vectors it eases between ticks ([`Driver`]), the
//! grid's cost its run test reads, the request's start on every fighter with
//! muscles ([`add_locomotion`]) and the standing character's facing
//! ([`face_standing`]).

use bevy::prelude::*;
use exa_anim::{Animator, Ease};
use exa_combat::stance::Stance;
use exa_physics::body::Ragdoll;
use exa_physics::body::locomotion::{Locomotion, Locomotor, forward};

/// The walk's own state between ticks: the heading and look vectors it
/// eases, the head's look (for a fighter without a brain, whose AI look
/// fields live here), the goal's direction and the body it last stopped
/// for. A walk starts from the request's heading and look.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Driver {
    /// A route is being walked: the vectors below are live.
    pub driving: bool,
    pub heading: Vec3,
    pub look: Vec3,
    /// The AI's look yaw and head offset for a fighter without a brain (a
    /// brain keeps them in its own gaze).
    pub gaze: crate::motion::Gaze,
    /// The goal's direction across the ground (unit) the last walk
    /// measured: the way test walks along it.
    pub dir: Vec3,
    /// The character the way test last stopped the walk for (its entity
    /// bits): it is skipped until the brain plans again.
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
