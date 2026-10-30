//! A lever as a body in the solver: the arm is a hung rigid body (its axle
//! pins and its arm link are its stored world links, with the origin pivot
//! registration adds), and [`LeverBody`] runs the lever's own rules
//! ([`crate::lever::Lever`]) on it each tick it is awake.
//!
//! **Set-up** (the original's lever init on a registered body). The arm's
//! mobility becomes 1 and the body moves into the mechanism environment (no
//! gravity, 0.001 of its velocity kept a second,
//! [`Ragdoll::enter_mechanism_environment`]). Its links give the lever's
//! swing ([`crate::lever::shape`], in centimetres): the last pin farther
//! than 1 cm from the origin gives the axle, the last longer link is the
//! arm, of radius r, with half swing α = 2·asin(slack / 2r). The lever frame
//! is built in the parent node's frame: its up row is the axle turned by the
//! node's own rotation and negated, its forward row the arm link's tied
//! point (less the node's own position) with its part along up taken off,
//! normalised, its side row up × forward, normalised; the frame's origin is
//! the node's own position. The first detent is the tied point's heading in
//! that frame less α (so 0 − α: the tied point lies on the forward row), the
//! second one step of 2α further. The arm's angle is the heading, in the
//! frame, of the arm point less the node's world position at set-up, turned
//! into the parent node's frame; the lever settles on its nearest detent
//! (ties to even), and an armed return saved in the object word (bits 16
//! and up, in units of 100 ms) is armed again.
//!
//! **Each awake tick**, after the solver: the arm point's motion since the
//! last tick (cm) is the creak's velocity; the angle is read as at set-up;
//! the lever's step ([`crate::lever::Lever::step`]) answers a drive angle,
//! and the arm point is moved onto the point at that angle on the arm's
//! radius about the frame's origin, spread over the particles by its weights
//! (positions only). The step's report of a detent is a signal to the
//! lever's targets ([`crate::mechanism::MechanismSignal`]). An armed return
//! counts down every tick, asleep or not (the original keeps it in a global
//! callback list), and wakes the lever when it fires.
//!
//! **The hand** takes no link: the wheel press marks the lever held and
//! records the angle between the arm and the cursor's point on the plane
//! through the node across the axle ([`LeverBody::take_hold`]); each frame
//! sets the lever's hand angle from the cursor ([`LeverBody::turn_to`]), and
//! the step's drive turns the arm toward it 0.02 rad a tick. The release
//! clears the hold and wakes the lever.
//!
//! **Sounds** (`crate::mechanism_sound`): each awake tick sends the creak
//! voice (`leverb_mov` at the arm's centre) and the return's tick loop
//! (`leverb_tck01`): started when a return is armed, at load too, faded
//! while it returns and freed at its end or at the report of 0.
//!
//! Not reproduced: the kind 6 lever's light fade is reported as a
//! [`LeverTick::light`] direction but nothing applies it to a spawned
//! light. The light node under the lever is found at registration (a light
//! node at or under the arm node's parent, the operative's node) and handed
//! to [`Lever::configure`]. The original runs the lever's rules in the
//! body's own step
//! between its integration and the link passes; Exarust runs them after the
//! solver's tick, as it does the doors'.
//!
//! Evidence: the lever init, step, record read, timed return and its tick
//! loop, the frame helpers they call (the axle, the plane projection, the
//! cross product, the heading) and the hand's wheel press and drag
//! handler, read in the instructions.

use bevy::math::{Affine3A, Mat3, Quat};
use bevy::prelude::*;

use super::{BodySystems, BodyTicks, CM, Ragdoll, WAKE_COUNT};
use crate::door;
use crate::engine_math::{direction_from_yaw, heading};
use crate::lever::{self, Lever, LeverTick, Record, Signal};
use crate::mechanism::MechanismSignal;
use crate::mechanism_sound::{self, MechanismSounded, TickCue};

/// A lever's mobility, whatever its file stores.
pub const LEVER_MOBILITY: f32 = 1.0;
/// The second wheel grip's point is taken this far (cm) along the frame's
/// forward row.
pub const SECOND_GRIP_CM: f32 = 30.0;

/// On a lever's root node until its body is registered: the operative
/// record, the placement's stored word and the placed id the lever signals
/// from.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct PendingLever {
    pub record: Record,
    pub saved: i32,
    pub id: u32,
    /// A light node was found under the lever's node: a kind 6 lever fades
    /// it.
    pub light: bool,
}

/// A lever's body glue: the lever's rules and what they read from the arm.
#[derive(Component, Debug, Clone, PartialEq)]
pub struct LeverBody {
    pub lever: Lever,
    pub record: Record,
    /// The placed id the lever's signals come from.
    pub id: u32,
    /// The lever frame's rows, in the parent node's frame, and its origin
    /// (the node's own position).
    side: Vec3,
    up: Vec3,
    forward: Vec3,
    offset: Vec3,
    /// The parent node's world transform.
    parent_world: Affine3A,
    /// The node's world position at set-up (m).
    origin: Vec3,
    /// The arm link's index in the body's world links, and its point's
    /// weights on the arm's particles.
    arm: usize,
    arm_weights: [f32; 4],
    /// The arm point's distance from the origin (m).
    radius: f32,
    /// Where the arm point was at the last tick (m).
    arm_at: Vec3,
    /// A timed return is armed and counting down.
    pub armed: bool,
}

/// The engine's exact normalisation: the reciprocal of the root of
/// `(y² + 0) + x² + z²`.
fn unit(v: Vec3) -> Vec3 {
    let length = ((v.y * v.y + 0.0) + v.x * v.x + v.z * v.z).sqrt();
    v * (1.0 / length)
}

/// Wraps an angle into ±π as the lever's hand angle is: under −π (a double
/// compare) the float 2π is added, over π it is taken off.
fn wrap(angle: f32) -> f32 {
    if f64::from(angle) < -std::f64::consts::PI {
        angle + std::f32::consts::TAU
    } else if f64::from(angle) > std::f64::consts::PI {
        angle - std::f32::consts::TAU
    } else {
        angle
    }
}

impl LeverBody {
    /// The lever of a registered hung arm (part 0): `parent_world` is its
    /// parent node's world transform, `turn` and `offset` the node's own
    /// rotation and position in it. `None` for a body with no arm link.
    pub fn new(
        body: &Ragdoll,
        pending: &PendingLever,
        parent_world: Affine3A,
        turn: Quat,
        offset: Vec3,
        now_ms: i32,
    ) -> Option<Self> {
        let part = body.parts.first()?;
        let rest_cm = part.rest.map(|r| r * CM);
        let stored: Vec<usize> = (0..body.world.len())
            .filter(|&i| body.world[i].part == 0 && !body.world[i].origin)
            .collect();
        let links: Vec<door::Link> = stored
            .iter()
            .map(|&i| {
                let l = &body.world[i];
                door::Link {
                    on_body: l.on_part,
                    on_parent: [0.0; 4],
                    slack: l.slack * CM,
                    flags: l.flags,
                }
            })
            .collect();
        // The arm is the last link longer than an axle pin.
        let arm_at = links
            .iter()
            .rposition(|l| !(l.slack <= lever::AXLE_SLACK || l.slack.is_nan()))?;
        let arm = stored[arm_at];
        let shape = lever::shape(&rest_cm, &links);
        if shape.radius <= 0.0 || !shape.half_swing.is_finite() {
            return None;
        }
        let up = turn * shape.axis;
        let tied = parent_world
            .inverse()
            .transform_point3(body.world[arm].anchor)
            - offset;
        let along = (tied.x * up.x + tied.z * up.z) + tied.y * up.y + 0.0;
        let forward = unit(tied - up * along);
        let side = unit(up.cross(forward));
        if !(side.is_finite() && forward.is_finite() && up.is_finite()) {
            return None;
        }
        let arm_weights = body.world[arm].on_part;
        let mut glue = Self {
            lever: Lever::new(shape, 0.0),
            record: pending.record,
            id: pending.id,
            side,
            up,
            forward,
            offset,
            parent_world,
            origin: parent_world.transform_point3(offset),
            arm,
            arm_weights,
            radius: shape.radius / CM,
            arm_at: part.point(&arm_weights),
            armed: false,
        };
        let detent0 = heading(glue.to_frame(tied)) - shape.half_swing;
        let shape = lever::Shape { detent0, ..shape };
        let angle = glue.angle_of(glue.arm_at);
        glue.lever = Lever::new(shape, angle);
        glue.lever.configure(&pending.record, pending.light);
        if glue.lever.return_ms != 0 && ((pending.saved as u32) >> 16) != 0 {
            glue.lever.restore(pending.saved, now_ms);
            glue.armed = true;
        }
        Some(glue)
    }

    /// A parent-frame vector in the lever frame.
    fn to_frame(&self, v: Vec3) -> Vec3 {
        Vec3::new(self.side.dot(v), self.up.dot(v), self.forward.dot(v))
    }

    /// A lever-frame point in the parent node's frame.
    fn frame_to_parent(&self, l: Vec3) -> Vec3 {
        self.side * l.x + self.up * l.y + self.forward * l.z + self.offset
    }

    /// A world point less the origin, in the lever frame.
    fn frame_offset(&self, world: Vec3) -> Vec3 {
        let turn = Mat3::from(self.parent_world.matrix3);
        self.to_frame(turn.transpose() * (world - self.origin))
    }

    /// The heading, in the lever frame, of a world point about the origin:
    /// the arm's angle when it is the arm point.
    pub fn angle_of(&self, world: Vec3) -> f32 {
        heading(self.frame_offset(world))
    }

    /// The world point at `angle` on the arm's radius about the origin:
    /// where the drive puts the arm point.
    pub fn arm_point_at(&self, angle: f32) -> Vec3 {
        let local = direction_from_yaw(angle) * self.radius;
        self.parent_world
            .transform_point3(self.frame_to_parent(local))
    }

    /// The arm point now (m).
    pub fn arm_point(&self, body: &Ragdoll) -> Vec3 {
        body.parts[0].point(&self.arm_weights)
    }

    /// The node's world position (m), the planes' and the reach's point.
    pub fn origin(&self) -> Vec3 {
        self.origin
    }

    /// The two wheel planes through the origin: the first across the axle
    /// (its normal the frame's up row in the world), the second across the
    /// forward row.
    pub fn grip_normals(&self) -> (Vec3, Vec3) {
        let turn = Mat3::from(self.parent_world.matrix3);
        (unit(turn * self.up), unit(turn * self.forward))
    }

    /// The wheel press at the cursor's point on the first plane: the lever
    /// is held and woken, and the angle between the arm and that point is
    /// returned (wrapped into ±π), which each frame's turn adds back.
    pub fn take_hold(&mut self, body: &mut Ragdoll, on_first: Vec3) -> f32 {
        self.lever.held = true;
        body.wake_to(WAKE_COUNT);
        wrap(self.lever.angle - self.angle_of(on_first))
    }

    /// One frame of the wheel's turn: the lever's hand angle from the
    /// cursor's points on the two planes. With the first grip (the camera
    /// looks along the axle by at least 0.2) it is the first point's heading
    /// plus `grabbed`, wrapped; with the second, the heading of the second
    /// point's sideways offset at 30 cm along forward. Either is clamped to
    /// the swing from the first detent (never below it, at most one step;
    /// a NaN passes) and wrapped. The lever is held and woken.
    pub fn turn_to(
        &mut self,
        body: &mut Ragdoll,
        on_first: Vec3,
        on_second: Vec3,
        first_grip: bool,
        grabbed: f32,
    ) {
        let wanted = if first_grip {
            wrap(self.angle_of(on_first) + grabbed)
        } else {
            let v = self.frame_offset(on_second) * CM;
            heading(Vec3::new(v.x, 0.0, SECOND_GRIP_CM))
        };
        self.lever.hand = swing_clamp(self.lever.detent0, wanted, self.lever.span);
        self.lever.held = true;
        body.wake_to(WAKE_COUNT);
    }

    /// The hand lets go (the release, or the cursor out of reach while
    /// held): the hold clears and the lever wakes.
    pub fn let_go(&mut self, body: &mut Ragdoll) {
        if self.lever.held {
            self.lever.held = false;
            body.wake_to(WAKE_COUNT);
        }
    }

    /// One tick of an awake arm, after the solver's tick.
    pub fn tick(&mut self, body: &mut Ragdoll, now_ms: i32) -> LeverTick {
        let point = self.arm_point(body);
        let velocity = (point - self.arm_at) * CM;
        self.arm_at = point;
        let angle = self.angle_of(point);
        let out = self.lever.step(angle, velocity, now_ms);
        if out.arm_return {
            self.armed = true;
        }
        let target = self.arm_point_at(out.drive);
        let delta = target - point;
        if delta.is_finite() {
            let w = self.arm_weights;
            body.parts[0].push(&w, delta);
        }
        out
    }

    /// The armed return's own tick: true when it fired (the lever then
    /// aims home and should be woken).
    pub fn count_down(&mut self, now_ms: i32) -> bool {
        if !self.armed {
            return false;
        }
        let fired = self.lever.timed_return(now_ms);
        if fired {
            self.armed = false;
        }
        fired
    }

    /// The signals a tick's report sends: (target, value), the first
    /// target then the second.
    pub fn signals(&self, out: &LeverTick) -> Vec<(u32, i32)> {
        let Some(signal) = out.signal else {
            return Vec::new();
        };
        let value = match signal {
            Signal::Off => 0,
            Signal::On => 1,
            Signal::Inverted => -1,
        };
        let mut sent = Vec::new();
        if out.first {
            sent.push((self.record.target, value));
        }
        if out.second {
            sent.push((self.record.second, value));
        }
        sent
    }

    /// The arm link's index in the body's world links.
    pub fn arm_link(&self) -> usize {
        self.arm
    }
}

/// The wheel's clamp: `wanted − detent0` is taken as 0 when negative and
/// `span` when longer (single-precision compares, so a NaN passes both),
/// added back to `detent0`, and wrapped into ±π.
pub fn swing_clamp(detent0: f32, wanted: f32, span: f32) -> f32 {
    let mut a = wanted - detent0;
    if a < 0.0 {
        a = 0.0;
    } else if a > span {
        a = span;
    }
    wrap(a + detent0)
}

/// Builds the lever glue of arms registered this frame and steps every
/// awake arm after the solver. Added by [`crate::prop::PropPlugin`].
pub(crate) fn plugin(app: &mut App) {
    crate::mechanism_sound::register(app);
    app.add_systems(
        FixedUpdate,
        (
            register_levers.before(BodySystems),
            step_levers.after(BodySystems),
        ),
    );
}

/// Turns each registered arm with a [`PendingLever`] into a [`LeverBody`]:
/// mobility 1, the mechanism environment, and the frame from its node.
fn register_levers(
    mut commands: Commands,
    ticks: Option<Res<BodyTicks>>,
    mut pending: Query<(Entity, &PendingLever, &mut Ragdoll)>,
    nodes: Query<(&Transform, Option<&ChildOf>)>,
    globals: Query<&GlobalTransform>,
    mut sounds: MessageWriter<MechanismSounded>,
) {
    let now = ticks.map_or(0, |t| t.millis() as i32);
    for (e, lever, mut body) in &mut pending {
        let Ok((node, parent)) = nodes.get(e) else {
            continue;
        };
        let parent_world = parent
            .and_then(|p| globals.get(p.parent()).ok())
            .map_or(Affine3A::IDENTITY, GlobalTransform::affine);
        commands.entity(e).remove::<PendingLever>();
        let Some(glue) = LeverBody::new(
            &body,
            lever,
            parent_world,
            node.rotation,
            node.translation,
            now,
        ) else {
            continue;
        };
        body.parts[0].inverse_mass = LEVER_MOBILITY;
        body.parts[0].pairing.mobility = LEVER_MOBILITY;
        body.enter_mechanism_environment();
        // A return saved in the object word is armed again, and arming
        // starts the tick loop.
        if glue.armed {
            sounds.write(MechanismSounded::TickLoop {
                source: e,
                position: Vec3::from(body.parts[0].transform().translation),
                cue: TickCue::Start,
            });
        }
        commands.entity(e).insert(glue);
    }
}

/// Counts down every armed return and steps every awake arm, sending the
/// signals and the sounds its reports make.
fn step_levers(
    ticks: Option<Res<BodyTicks>>,
    mut levers: Query<(Entity, &mut LeverBody, &mut Ragdoll)>,
    mut signals: MessageWriter<MechanismSignal>,
    mut sounds: MessageWriter<MechanismSounded>,
) {
    let now = ticks.map_or(0, |t| t.millis() as i32);
    for (e, mut glue, mut body) in &mut levers {
        if glue.count_down(now) {
            body.wake_to(WAKE_COUNT);
        }
        if body.asleep() {
            continue;
        }
        let was_playing = glue.lever.tick_playing;
        let out = glue.tick(&mut body, now);
        let centre = Vec3::from(body.parts[0].transform().translation);
        sounds.write_batch(mechanism_sound::lever_cues(
            e,
            centre,
            &out,
            was_playing,
            &glue.lever,
        ));
        for (target, value) in glue.signals(&out) {
            signals.write(MechanismSignal {
                target,
                source: glue.id,
                value,
            });
        }
    }
}

#[cfg(test)]
#[path = "body_lever_test.rs"]
mod tests;
