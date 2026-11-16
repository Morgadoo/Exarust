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
//!
//! The body's three directions the AI turns it by are the original's body
//! fields: its look direction (the player's cursor direction, the
//! request's cursor), its look smoothing (the head's look, the request's
//! look, which the torso twist turns toward) and its heading (the request's
//! heading, which the movement decision turns the body to at its rates).
//! The AI keeps them as vectors in its gaze ([`crate::motion::Gaze`]: the
//! brain's, or the driver's for a fighter without a brain). After the
//! state, a present body whose route is not followed runs the AI tick's
//! heading block ([`after_state`]) and the vectors are written into the
//! request as yaws ([`publish`]); a direction the body or the route walk
//! wrote in the request since then is taken back into the gaze at the start
//! of the next AI tick ([`adopt_body_directions`]), as the original keeps
//! one copy of each.

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
    /// The heading, look and cursor yaws [`publish`] last wrote into the
    /// request: a request yaw that differs was written by someone else (the
    /// body, the route walk), and [`adopt_body_directions`] takes it back.
    /// None until the AI first writes the body's directions: until then the
    /// standing character faces as its model does ([`face_standing`]).
    pub published: Option<[f32; 3]>,
}

/// The gaze that keeps a character's body directions: its brain's, or the
/// driver's for a fighter without a brain.
pub fn body_gaze<'a>(
    npc: Option<&'a mut crate::brain::Npc>,
    driver: &'a mut Driver,
) -> &'a mut crate::motion::Gaze {
    match npc {
        Some(npc) => &mut npc.gaze,
        None => &mut driver.gaze,
    }
}

/// What the AI tick's heading block after the state did ([`after_state`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AfterState {
    /// Stepping or attacking: the look smoothing and the heading took the
    /// look direction.
    Copied,
    /// In combat: the look snap ran; true when it moved the heading.
    Snapped(bool),
    /// Out of combat: the brain's look chain is the block
    /// ([`crate::motion::look_chain`], run by its tick).
    Chain,
}

/// The AI tick's heading block after the state, for a present body whose
/// route is not followed: a body that is stepping, or else in the attacking
/// condition, takes its look direction as its look smoothing and its
/// heading; otherwise a body with its in-combat mark runs the look snap
/// ([`crate::motion::look_snap`]); otherwise the out-of-combat look chain
/// runs (the brain's tick runs it, with the same first branch).
pub fn after_state(
    gaze: &mut crate::motion::Gaze,
    stepping: bool,
    attacking: bool,
    in_combat: bool,
) -> AfterState {
    if stepping || attacking {
        gaze.body_smooth = gaze.body_look;
        gaze.body_heading = gaze.body_look;
        AfterState::Copied
    } else if in_combat {
        AfterState::Snapped(crate::motion::look_snap(gaze))
    } else {
        AfterState::Chain
    }
}

/// Writes the body's directions the gaze keeps into the locomotion
/// request: the heading the movement decision turns the body to, the look
/// the torso twist reads and the cursor (with the wanted direction), each
/// the yaw of its vector by the engine's two-argument arctangent, as the
/// body's turn reads its heading. A gaze that never took the body's
/// directions (zero) writes nothing.
pub fn publish(req: &mut Locomotion, driver: &mut Driver, gaze: &crate::motion::Gaze) {
    if gaze.body_heading == Vec3::ZERO {
        return;
    }
    let heading = crate::motion::yaw_atan2(gaze.body_heading);
    let look = crate::motion::yaw_atan2(gaze.body_smooth);
    let cursor = crate::motion::yaw_atan2(gaze.body_look);
    req.heading = heading;
    req.look = look;
    req.cursor = cursor;
    req.desired = cursor;
    driver.published = Some([heading, look, cursor]);
}

/// Takes back into the gaze each body direction whose request yaw differs
/// from the one [`publish`] last wrote (the body's turn toward a fall, the
/// route walk, a flight's cursor): the vector is the engine's direction of
/// that yaw. Nothing before the first publish.
pub fn adopt(req: &Locomotion, driver: &mut Driver, gaze: &mut crate::motion::Gaze) {
    let Some([heading, look, cursor]) = driver.published else {
        return;
    };
    if req.heading != heading {
        gaze.body_heading = crate::motion::dir_of(req.heading);
    }
    if req.look != look {
        gaze.body_smooth = crate::motion::dir_of(req.look);
    }
    if req.cursor != cursor {
        gaze.body_look = crate::motion::dir_of(req.cursor);
    }
    driver.published = Some([req.heading, req.look, req.cursor]);
}

/// The start of the brains' tick: every character on its locomotion takes
/// back the body directions written into its request since the AI last
/// wrote them ([`adopt`]).
pub fn adopt_body_directions(
    mut bodies: Query<(&Locomotion, &mut Driven, Option<&mut crate::brain::Npc>)>,
) {
    for (req, mut driven, npc) in &mut bodies {
        let driver = &mut driven.0;
        let mut own = driver.gaze;
        let gaze = match npc {
            Some(npc) => &mut npc.into_inner().gaze,
            None => &mut own,
        };
        adopt(req, driver, gaze);
        driver.gaze = own;
    }
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
/// whose route is not driven, whose body does not walk and whose AI has not
/// written its body directions ([`Driver::published`]) turns toward the yaw
/// its model faces ([`face`]; the movement stage turns the body at its
/// rates and puts the model back on the body's yaw), and a walk clip its
/// body no longer walks gives way to its stance over 20 frames with the
/// fast ease, as the original's body tick starts the stored stance when it
/// leaves the walking condition.
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
        if driven.0.published.is_none() {
            let yaw = exa_physics::body::locomotion::yaw_of(t.rotation * Vec3::Z);
            if req.heading != yaw {
                face(&mut req, yaw);
            }
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
    use crate::motion::{Gaze, dir_of, yaw_atan2};

    fn gaze(look: f32, smooth: f32, heading: f32) -> Gaze {
        Gaze {
            body_look: dir_of(look),
            body_smooth: dir_of(smooth),
            body_heading: dir_of(heading),
            ..Gaze::default()
        }
    }

    /// After the state: stepping or attacking, the look smoothing and the
    /// heading take the look direction; in combat the look snap; out of
    /// combat the brain's chain (nothing here).
    #[test]
    fn the_heading_block_after_the_state() {
        let mut g = gaze(1.0, 0.2, 0.0);
        assert_eq!(after_state(&mut g, true, false, true), AfterState::Copied);
        assert_eq!((g.body_smooth, g.body_heading), (dir_of(1.0), dir_of(1.0)));
        let mut g = gaze(1.0, 0.2, 0.0);
        assert_eq!(after_state(&mut g, false, true, false), AfterState::Copied);
        assert_eq!(g.body_heading, dir_of(1.0));
        // In combat the heading stays within 30° of the look smoothing.
        let mut g = gaze(1.0, 0.2, 0.0);
        assert_eq!(
            after_state(&mut g, false, false, true),
            AfterState::Snapped(false)
        );
        assert_eq!(g.body_heading, dir_of(0.0));
        let mut g = gaze(1.0, 0.6, 0.0);
        assert_eq!(
            after_state(&mut g, false, false, true),
            AfterState::Snapped(true)
        );
        assert_eq!(g.body_heading, dir_of(1.0));
        let mut g = gaze(1.0, 0.6, 0.0);
        assert_eq!(after_state(&mut g, false, false, false), AfterState::Chain);
        assert_eq!(g.body_heading, dir_of(0.0));
    }

    /// The body directions go into the request as yaws (the cursor's also
    /// as the wanted direction); a request yaw the body or the walk wrote
    /// since is taken back as its direction, the others stay.
    #[test]
    fn the_body_directions_go_into_the_request_and_come_back() {
        let (mut req, _) = Locomotion::facing(0.0);
        let mut driver = Driver::default();
        // Never turned: nothing is written.
        publish(&mut req, &mut driver, &Gaze::default());
        assert_eq!((req.heading, driver.published), (0.0, None));
        let mut g = gaze(1.0, 0.5, 0.25);
        publish(&mut req, &mut driver, &g);
        let written = [
            yaw_atan2(dir_of(0.25)),
            yaw_atan2(dir_of(0.5)),
            yaw_atan2(dir_of(1.0)),
        ];
        assert_eq!([req.heading, req.look, req.cursor], written);
        assert_eq!(req.desired, written[2]);
        assert_eq!(driver.published, Some(written));
        // Nothing else wrote: nothing is taken back.
        let before = g;
        adopt(&req, &mut driver, &mut g);
        assert_eq!(g, before);
        // The body turned its heading toward a fall: the heading comes back.
        req.heading = -2.0;
        adopt(&req, &mut driver, &mut g);
        assert_eq!(g.body_heading, dir_of(-2.0));
        assert_eq!(
            (g.body_smooth, g.body_look),
            (before.body_smooth, before.body_look)
        );
        assert_eq!(driver.published.map(|p| p[0]), Some(-2.0));
    }
}
