//! A hinged door as a body in the solver: the leaf is a hung rigid body
//! (its hinge pin and swing stop are its stored world links, with the
//! origin pivot registration adds), and [`DoorBody`] runs the door's own
//! rules ([`crate::door::Door`]) on it each tick it is awake.
//!
//! **Set-up** (the original's door init on a registered body). The leaf's
//! mobility becomes 0.05, which only the pair stage and the hand read; the
//! body meets no level (a hung root). Its links give the door's shape
//! ([`crate::door::shape`], in centimetres). The door frame is built from
//! the hinge axis and the stop's tied point in the parent node's frame
//! (that point less the node's own position): its up row is the negated
//! axis, its forward row that point with its part along up taken off,
//! normalised, its side row up × forward, normalised. The handle is the
//! body point (−150·sign cm, the node box's centre height, 0). The stored
//! word is the placement's word. A sibling node's door (the other leaf of
//! a double door) is the partner, and both take pair layer bits `0x3000`.
//!
//! **Each awake tick**, after the solver: the handle's motion since the
//! last tick (cm) is smoothed by half and its squared length is the creak
//! energy; the stop's measured length (cm, 0 inside its slack) is the
//! knock travel; the angle is the heading, in the door frame, of the stop
//! point less the body's origin, turned into the parent node's frame. The
//! door's step ([`crate::door::Door::step`]) then answers a drive angle,
//! which moves the stop point to the point at that angle on the stop's
//! radius about the body origin, spread over the particles by its weights
//! (positions only, so the move carries into the next tick's velocity).
//! A signalled open's 3 cm shove is applied at the handle along the leaf's
//! z. The original runs this in the body's own step, between its
//! integration and the link passes; Exarust runs it after the solver's
//! tick, so the links meet the drive one tick later.
//!
//! **The hand** ([`DoorBody::take_hold`]): the press adds a world link from
//! the leaf point 100 cm from the hinge-side origin toward the hit, slack
//! 1 cm, strength 0.003, run in the tick's first three passes, and marks the
//! leaf held; each frame moves its anchor ([`DoorBody::pull_to`]) or sets
//! its disabled bit while the request is suppressed; the release removes
//! it. The plain slack law moves the leaf: with three passes a tick the
//! point closes about 0.9% of its gap a tick, a soft spring. A shut leaf is
//! held at its closed angle by the drive until the held pull count runs
//! out (about seven ticks of pull below closed), then it swings free.
//!
//! Not reproduced: the door's sounds (the step reports them), the
//! navigation passage report, the last-pusher stamp, sliding doors (they
//! stay static level geometry).
//!
//! Evidence: the door's init and hinged step, the hand's door press and
//! drag handler and the engine's link-adding routine, read in the
//! instructions; `context/re/master-physics.md` §2–§3.

use bevy::math::{Affine3A, Mat3};
use bevy::prelude::*;

use super::{BodySystems, BodyTicks, CM, LINK_DISABLED, Ragdoll, WAKE_COUNT, WorldLink, weights};
use crate::door::{self, Door, Record, Tick};
use crate::engine_math::{direction_from_yaw, heading, normalize_approx};

/// A hinged leaf's mobility, whatever its file stores.
pub const HINGED_MOBILITY: f32 = 0.05;
/// The hand's door link: its slack (m; 1 cm), its strength, its flags (the
/// iteration limit alone) and the last pass it runs in.
pub const HAND_SLACK: f32 = 0.01;
pub const HAND_STRENGTH: f32 = 0.003;
pub const HAND_FLAGS: u32 = super::world_link::ITERATION_LIMIT;
pub const HAND_LAST_PASS: u32 = 2;
/// How far from the hinge-side origin the hand's link takes the leaf (m).
pub const HAND_REACH: f32 = 1.0;
/// The handle's distance from the hinge (cm), on the side away from the
/// stop's sign.
const HANDLE_CM: f32 = 150.0;

/// On a hinged door's root node until its body is registered: the door
/// record, the placement's stored word and the node box's centre height
/// (m, the node's frame).
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct PendingDoor {
    pub record: Record,
    pub saved: i32,
    pub handle_height: f32,
}

/// A hinged door's body glue: the door's rules and what they read from the
/// leaf.
#[derive(Component, Debug, Clone, PartialEq)]
pub struct DoorBody {
    pub door: Door,
    /// The door frame's rows, in the parent node's frame.
    side: Vec3,
    up: Vec3,
    forward: Vec3,
    /// The parent node's rotation in the world.
    parent_turn: Mat3,
    /// The stop's index in the body's world links.
    stop: usize,
    /// The handle point's weights on the leaf's particles.
    handle: [f32; 4],
    /// Where the handle was at the last tick (m).
    handle_at: Vec3,
    /// The handle's smoothed motion (cm a tick).
    smoothed: Vec3,
    /// The other leaf of a double door.
    pub partner: Option<Entity>,
    /// The hand's link, by its index in the body's world links.
    hand: Option<usize>,
    /// The cooldowns' random draws.
    seed: u32,
}

impl DoorBody {
    /// The door of a registered hung leaf (part 0): `parent_world` is its
    /// parent node's world transform and `origin` the node's position in
    /// it. `None` for a sliding leaf or a body with no stop.
    pub fn new(
        body: &Ragdoll,
        pending: &PendingDoor,
        parent_world: Affine3A,
        origin: Vec3,
    ) -> Option<Self> {
        let leaf = body.parts.first()?;
        let rest_cm = leaf.rest.map(|r| r * CM);
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
        let shape = door::shape(&rest_cm, &links);
        if shape.sliding || shape.radius <= 0.0 {
            return None;
        }
        let stop = *stored.get(shape.travel_link)?;
        let up = -shape.axis;
        let tied = parent_world
            .inverse()
            .transform_point3(body.world[stop].anchor)
            - origin;
        let forward = normalize_approx(tied - up * tied.dot(up));
        let side = normalize_approx(up.cross(forward));
        let handle_local = Vec3::new(-HANDLE_CM * shape.sign / CM, pending.handle_height, 0.0);
        let handle = weights(leaf, handle_local);
        let mut glue = Self {
            door: Door::new(pending.record, shape, pending.saved, 0.0, 0.0),
            side,
            up,
            forward,
            parent_turn: Mat3::from(parent_world.matrix3),
            stop,
            handle,
            handle_at: leaf.point(&handle),
            smoothed: Vec3::ZERO,
            partner: None,
            hand: None,
            seed: 1,
        };
        let heading = glue.heading(body);
        glue.door = Door::new(pending.record, shape, pending.saved, heading, 0.0);
        Some(glue)
    }

    /// The stop's heading in the door frame, before the swing sign.
    pub fn heading(&self, body: &Ragdoll) -> f32 {
        let leaf = &body.parts[0];
        let stop = leaf.point(&body.world[self.stop].on_part);
        let from = stop - Vec3::from(leaf.transform().translation);
        let local = self.parent_turn.transpose() * from;
        heading(Vec3::new(
            self.side.dot(local),
            self.up.dot(local),
            self.forward.dot(local),
        ))
    }

    /// The world point at `angle` (after the swing sign) on the stop's
    /// radius about the leaf's origin.
    fn stop_at(&self, body: &Ragdoll, angle: f32) -> Vec3 {
        let leaf = &body.parts[0];
        let d = direction_from_yaw(angle * self.door.shape.sign) * (self.door.shape.radius / CM);
        let local = self.side * d.x + self.up * d.y + self.forward * d.z;
        Vec3::from(leaf.transform().translation) + self.parent_turn * local
    }

    /// Moves the stop point to the point at `angle`, by its weights.
    fn drive(&self, body: &mut Ragdoll, angle: f32) {
        let target = self.stop_at(body, angle);
        let w = body.world[self.stop].on_part;
        let leaf = &mut body.parts[0];
        let delta = target - leaf.point(&w);
        if delta.is_finite() {
            leaf.push(&w, delta);
        }
    }

    /// The next of the original's unit draws, as a value in 0..100.
    fn roll(&mut self) -> u32 {
        self.seed = self.seed.wrapping_mul(0x0808_8405).wrapping_add(1);
        ((u64::from(self.seed) * 100) >> 32) as u32
    }

    /// One tick of an awake leaf, after the solver's tick.
    pub fn tick(&mut self, body: &mut Ragdoll, now_ms: i32, partner: Option<&mut Door>) {
        let leaf = &body.parts[0];
        let handle = leaf.point(&self.handle);
        let moved = (handle - self.handle_at) * CM;
        self.handle_at = handle;
        self.smoothed = self.smoothed * 0.5 + moved * 0.5;
        let s = self.smoothed;
        let energy = s.z * s.z + s.y * s.y + s.x * s.x;
        let travel = body.world[self.stop].length * CM;
        let speed = (handle - leaf.point_at_start(&self.handle)) * CM;
        let heading = self.heading(body);
        let roll = self.roll();
        let tick = Tick {
            now_ms,
            heading,
            energy,
            travel,
            handle_speed_sq: speed.length_squared(),
            roll,
        };
        let out = self.door.step(tick, partner);
        if out.shove != Vec3::ZERO {
            let leaf = &mut body.parts[0];
            let push = leaf.transform().transform_vector3(out.shove / CM);
            let w = self.handle;
            leaf.push(&w, push);
        }
        if let Some(angle) = out.drive {
            self.drive(body, angle);
        }
    }

    /// Whether the leaf is latched and not being unlocked: a press rattles
    /// it instead of dragging.
    pub fn latched(&self) -> bool {
        self.door.latched
    }

    /// Whether an unlock is running (the latch does not rattle then).
    pub fn forcing(&self) -> bool {
        self.door.unlock_pending
    }

    /// The hinge-side origin of the leaf (m): the body's origin.
    pub fn origin(&self, body: &Ragdoll) -> Vec3 {
        Vec3::from(body.parts[0].transform().translation)
    }

    /// The hand takes hold of the leaf at the world point `at`: a world
    /// link anchored there (slack 1 cm, strength 0.003, the first three
    /// passes), the leaf marked held and woken.
    pub fn take_hold(&mut self, body: &mut Ragdoll, at: Vec3) {
        self.let_go(body);
        let leaf = &body.parts[0];
        let local = Vec3::from(leaf.transform().inverse().transform_point3(at));
        body.world.push(WorldLink {
            part: 0,
            on_part: weights(leaf, local),
            anchor: at,
            slack: HAND_SLACK,
            strength: HAND_STRENGTH,
            smoothing: 0.0,
            flags: HAND_FLAGS,
            axis: Vec3::ZERO,
            length: 0.0,
            origin: false,
            last_pass: Some(HAND_LAST_PASS),
        });
        self.hand = Some(body.world.len() - 1);
        self.door.held = true;
        body.wake_to(WAKE_COUNT);
    }

    /// One frame of the hand's pull: the link's anchor moves to `anchor`
    /// and the leaf wakes, or, with `None` (the request suppressed), the
    /// link's disabled bit is set and the anchor stays.
    pub fn pull_to(&mut self, body: &mut Ragdoll, anchor: Option<Vec3>) {
        let Some(link) = self.hand.and_then(|i| body.world.get_mut(i)) else {
            return;
        };
        match anchor {
            Some(at) => {
                link.flags &= !LINK_DISABLED;
                link.anchor = at;
                body.wake_to(WAKE_COUNT);
            }
            None => link.flags |= LINK_DISABLED,
        }
    }

    /// The hand lets go: its link is removed and the leaf is no longer
    /// held.
    pub fn let_go(&mut self, body: &mut Ragdoll) {
        if let Some(i) = self.hand.take()
            && i < body.world.len()
        {
            body.world.remove(i);
        }
        self.door.held = false;
    }

    /// The hand's link, while held.
    pub fn hand_link<'a>(&self, body: &'a Ragdoll) -> Option<&'a WorldLink> {
        self.hand.and_then(|i| body.world.get(i))
    }
}

/// Builds the door glue of leaves registered this frame, pairs the two
/// leaves of a double door, and steps every awake leaf after the solver.
/// Added by [`crate::prop::PropPlugin`].
pub(crate) fn plugin(app: &mut App) {
    app.add_systems(
        FixedUpdate,
        (
            register_doors.before(BodySystems),
            step_doors.after(BodySystems),
        ),
    );
}

/// Turns each registered leaf with a [`PendingDoor`] into a [`DoorBody`]:
/// mobility 0.05, the door frame from its parent node, and a partner in a
/// sibling node's leaf (both then take pair layer bits `0x3000`).
#[allow(clippy::type_complexity)]
fn register_doors(
    mut commands: Commands,
    mut pending: Query<(Entity, &PendingDoor, &mut Ragdoll)>,
    mut doors: Query<(Entity, &mut DoorBody, &mut Ragdoll), Without<PendingDoor>>,
    nodes: Query<(&Transform, Option<&ChildOf>)>,
    globals: Query<&GlobalTransform>,
) {
    let mut fresh = Vec::new();
    for (e, door, mut body) in &mut pending {
        let Ok((node, parent)) = nodes.get(e) else {
            continue;
        };
        let parent = parent.map(ChildOf::parent);
        let parent_world = parent
            .and_then(|p| globals.get(p).ok())
            .map_or(Affine3A::IDENTITY, GlobalTransform::affine);
        commands.entity(e).remove::<PendingDoor>();
        let Some(mut glue) = DoorBody::new(&body, door, parent_world, node.translation) else {
            continue;
        };
        body.parts[0].inverse_mass = HINGED_MOBILITY;
        body.parts[0].pairing.mobility = HINGED_MOBILITY;
        // A sibling leaf already registered, or one registered with this.
        let sibling = |q: Entity| {
            q != e
                && parent.is_some()
                && nodes.get(q).ok().and_then(|n| n.1).map(ChildOf::parent) == parent
        };
        if let Some((other, mut other_glue, mut other_body)) =
            doors.iter_mut().find(|(q, ..)| sibling(*q))
        {
            glue.partner = Some(other);
            other_glue.partner = Some(e);
            other_glue.door.has_partner = true;
            other_body.parts[0].pairing.layer =
                door::pair_layer(other_body.parts[0].pairing.layer, true);
        } else if let Some(&(other, _)) = fresh.iter().find(|(q, _)| sibling(*q)) {
            glue.partner = Some(other);
        }
        glue.door.has_partner = glue.partner.is_some();
        fresh.push((e, glue));
    }
    // Leaves registered together name each other.
    let names: Vec<(Entity, Option<Entity>)> = fresh.iter().map(|(e, g)| (*e, g.partner)).collect();
    for (e, glue) in &mut fresh {
        if glue.partner.is_none() {
            glue.partner = names.iter().find(|(_, p)| *p == Some(*e)).map(|(q, _)| *q);
            glue.door.has_partner = glue.partner.is_some();
        }
    }
    for (e, glue) in fresh {
        if glue.partner.is_some()
            && let Ok((_, _, mut body)) = pending.get_mut(e)
        {
            body.parts[0].pairing.layer = door::pair_layer(body.parts[0].pairing.layer, true);
        }
        commands.entity(e).insert(glue);
    }
}

/// Steps every awake leaf, its partner's door rules taken along.
fn step_doors(ticks: Res<BodyTicks>, mut doors: Query<(Entity, &mut DoorBody, &mut Ragdoll)>) {
    let now = ticks.millis() as i32;
    let awake: Vec<(Entity, Option<Entity>)> = doors
        .iter()
        .filter(|(_, _, body)| !body.asleep())
        .map(|(e, glue, _)| (e, glue.partner))
        .collect();
    for (e, partner) in awake {
        let mut other = partner.and_then(|p| doors.get(p).ok().map(|(_, g, _)| g.door));
        if let Ok((_, mut glue, mut body)) = doors.get_mut(e) {
            glue.tick(&mut body, now, other.as_mut());
        }
        if let (Some(p), Some(door)) = (partner, other)
            && let Ok((_, mut glue, _)) = doors.get_mut(p)
        {
            glue.door = door;
        }
    }
}

#[cfg(test)]
#[path = "body_door_test.rs"]
mod tests;
