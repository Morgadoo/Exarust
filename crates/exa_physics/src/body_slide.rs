//! A sliding door (exanima01's 37 portcullises) as a body in the solver:
//! the leaf is a hung rigid body held by its slider links (registration
//! adds no origin pivot when the first link slides), and [`SlideBody`]
//! runs the door's sliding rules ([`crate::door::Door`]) on it each tick it
//! is awake, as the original builds a door body for every door record.
//!
//! **Set-up** (the original's door init on a registered sliding leaf). The
//! leaf keeps its registered mobility and the default environment. Its
//! links give the door's shape ([`crate::door::shape`], in centimetres):
//! any slider link makes it slide, and its travel link is the first whose
//! body point is 30–160 cm up. The leaf reads shut when that link's travel
//! fraction is at least 0.995 (1 is shut, 0 open); a frozen leaf parks at
//! its fraction with smoothing 1. The handle is the body point (−150 cm,
//! the node box's centre height, 0).
//!
//! **Each awake tick, right after the leaf's integration** (the solver's
//! [`SolverStage::integrated`], [`SlideStage`]), before the link passes:
//! the original's door step integrates a sliding leaf first and then runs
//! its sliding step, so the step measures the integrated leaf and its
//! writes meet this tick's passes. A held leaf's hand link is anchored at
//! the drag target, kept within 60 cm of the link's point. The handle's
//! motion since the last tick (cm) is smoothed halfway (`s + (v − s)·½`)
//! and its squared length is the creak energy; the travel link's fraction
//! is the door's fraction; the door's sliding step ([`crate::door::Door::step`])
//! then writes its fraction and smoothing back into that link, and the
//! slider law pulls the leaf there (the point drawn `smoothing` of the way
//! to the fraction's place, its correction applied twice) in each pass,
//! rewriting the fraction from where the leaf lies. A signalled open jumps
//! a plain leaf's fraction to 0.6 for 80 ticks, a shove of 1% of the gap in
//! the first pass that the leaf's inertia carries; a motorised one crawls
//! 0.01 a tick toward 0 for 300, a close 0.075 a tick toward 1, which then
//! latches. An open command wakes the body each tick it runs.
//!
//! The shipped leaves' rails lie level: each portcullis is placed slid
//! into its wall (its slider points on their anchors, fraction 0, open),
//! and shuts by travelling 170 cm along its rail into its frame, so the
//! slider law, not a lift, moves it.
//!
//! **The hand** ([`SlideBody::take_hold`]): the press adds the door's drag
//! link at the hit (slack 1 cm, strength 0.003, the first three passes);
//! each frame the drag target is the link's point moved by the cursor's
//! offset along the leaf's own x axis ([`SlideBody::pull_to`]); out of
//! reach the link's disabled bit is set; the release removes it.
//!
//! Its sounds go out as the hinged leaf's do (`crate::mechanism_sound`),
//! the knock on the slam set.
//!
//! Its passage flag follows the travel as the hinged leaf's follows the
//! angle; the original's report of it to the navigation grid changes
//! nothing there (`crate::body_door`), so it is not made. A sliding leaf
//! takes no partner (no shipped sliding door has a sibling leaf).
//!
//! Evidence: the door's init (its sliding branch), hinged step (its
//! sliding entry) and sliding step, the hand's door drag handler and the
//! world-link slider law, read in the instructions.

use bevy::prelude::*;

use super::{
    BodySystems, BodyTicks, CM, LINK_DISABLED, Ragdoll, SolverStage, SolverStages, WAKE_COUNT,
    WorldLink, weights,
};
use crate::door::{self, Door, DoorTick, Record, Tick};
use crate::mechanism_sound::{self, MechanismSounded};

/// The handle's distance from the origin (cm), as the hinged leaf's.
const HANDLE_CM: f32 = 150.0;

/// On a sliding door's root node until its body is registered: the door
/// record, the placement's stored word and the node box's centre height
/// (m, the node's frame).
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct PendingSlide {
    pub record: Record,
    pub saved: i32,
    pub handle_height: f32,
}

/// A sliding leaf's body glue: the door's rules and its travel link.
#[derive(Component, Debug, Clone, PartialEq)]
pub struct SlideBody {
    pub door: Door,
    /// The travel link's index in the body's world links.
    travel: usize,
    /// The handle point's weights on the leaf's particles.
    handle: [f32; 4],
    /// Where the handle was at the last tick (m).
    handle_at: Vec3,
    /// Where it was the tick before (m), where a wait's open plays.
    handle_before: Vec3,
    /// The handle's smoothed motion (cm a tick).
    smoothed: Vec3,
    /// The fraction last written into the travel link: a command that
    /// changed the door's fraction since (an open's jump) is written into
    /// the link before the tick reads it, as the original's commands write
    /// the link itself.
    written: f32,
    /// The hand's link, by its index in the body's world links.
    hand: Option<usize>,
    /// Where the hand drags the link's point (m).
    drag: Vec3,
}

impl SlideBody {
    /// The door of a registered sliding leaf (part 0); `None` when the
    /// leaf does not slide or has no travel link.
    pub fn new(body: &mut Ragdoll, pending: &PendingSlide) -> Option<Self> {
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
        if !shape.sliding {
            return None;
        }
        // Without a link 30–160 cm up the original keeps index 0.
        let travel = *stored.get(shape.travel_link)?;
        let handle_local = Vec3::new(-HANDLE_CM / CM, pending.handle_height, 0.0);
        let handle = weights(leaf, handle_local);
        let handle_at = leaf.point(&handle);
        let fraction = body.world[travel].length;
        let door = Door::new(pending.record, shape, pending.saved, 0.0, fraction);
        if door.frozen {
            body.world[travel].smoothing = door.slide.smoothing;
        }
        Some(Self {
            door,
            travel,
            handle,
            handle_at,
            handle_before: handle_at,
            smoothed: Vec3::ZERO,
            written: fraction,
            hand: None,
            drag: handle_at,
        })
    }

    /// The travel link's index in the body's world links.
    pub fn travel_link(&self) -> usize {
        self.travel
    }

    /// The travel link's fraction (1 shut, 0 open).
    pub fn fraction(&self, body: &Ragdoll) -> f32 {
        body.world[self.travel].length
    }

    /// One tick of an awake leaf, right after its integration
    /// ([`SlideStage`]): the hand link's anchor, the handle's motion, the
    /// door's sliding step and its writes into the travel link; an open
    /// command wakes the body.
    pub fn tick(&mut self, body: &mut Ragdoll, now_ms: i32) -> DoorTick {
        // A held leaf's drag link is anchored at the drag target, kept
        // within 60 cm of the link's point.
        if self.door.held
            && let Some(i) = self.hand.filter(|&i| i < body.world.len())
        {
            let point = body.parts[0].point(&body.world[i].on_part);
            let offset = door::slide_hand((self.drag - point) * CM);
            body.world[i].anchor = point + offset / CM;
        }
        let leaf = &body.parts[0];
        let handle = leaf.point(&self.handle);
        let moved = (handle - self.handle_at) * CM;
        self.handle_before = self.handle_at;
        self.handle_at = handle;
        // Halfway to this tick's motion, in the original's form.
        self.smoothed += (moved - self.smoothed) * 0.5;
        let energy = self.smoothed.length_squared();
        let link = &mut body.world[self.travel];
        if self.door.slide.fraction != self.written {
            link.length = self.door.slide.fraction;
        }
        self.door.slide.fraction = link.length;
        let tick = Tick::new(now_ms, 0.0, energy, link.length);
        let out = self.door.step(tick, None);
        link.length = self.door.slide.fraction;
        link.smoothing = self.door.slide.smoothing;
        self.written = link.length;
        if out.wake {
            body.wake_to(WAKE_COUNT);
        }
        out
    }

    /// The sound requests of a tick's report (`crate::mechanism_sound`):
    /// the creak voice at the body's centre; the knock and the slam (both
    /// the slam set) and the unlatch at the handle point.
    pub fn sounds(&self, source: Entity, body: &Ragdoll, out: &DoorTick) -> Vec<MechanismSounded> {
        mechanism_sound::door_cues(
            source,
            &self.door.record,
            true,
            out,
            Vec3::from(body.parts[0].transform().translation),
            self.handle_at,
            self.handle_before,
        )
    }

    /// The handle point (m) as the last tick measured it.
    pub fn handle_point(&self) -> Vec3 {
        self.handle_at
    }

    /// The hand takes hold of the leaf at the world point `at`: the door's
    /// drag link anchored there (slack 1 cm, strength 0.003, the first
    /// three passes), the leaf marked held and woken.
    pub fn take_hold(&mut self, body: &mut Ragdoll, at: Vec3) {
        self.let_go(body);
        let leaf = &body.parts[0];
        let local = leaf.transform().inverse().transform_point3(at);
        body.world.push(WorldLink {
            part: 0,
            on_part: weights(leaf, local),
            anchor: at,
            slack: super::door_body::HAND_SLACK,
            strength: super::door_body::HAND_STRENGTH,
            smoothing: 0.0,
            flags: super::door_body::HAND_FLAGS,
            axis: Vec3::ZERO,
            length: 0.0,
            origin: false,
            last_pass: Some(super::door_body::HAND_LAST_PASS),
        });
        self.hand = Some(body.world.len() - 1);
        self.drag = at;
        self.door.held = true;
        body.wake_to(WAKE_COUNT);
    }

    /// One frame of the hand's drag toward the cursor's point `cursor`: the
    /// target is the link's point moved by the cursor's offset along the
    /// leaf's own x axis (its rail), the link enabled and the leaf woken;
    /// with `None` (the request suppressed) the link's disabled bit is set.
    pub fn pull_to(&mut self, body: &mut Ragdoll, cursor: Option<Vec3>) {
        let Some(i) = self.hand.filter(|&i| i < body.world.len()) else {
            return;
        };
        match cursor {
            Some(at) => {
                let leaf = &body.parts[0];
                let point = leaf.point(&body.world[i].on_part);
                let axis = Vec3::from(leaf.transform().matrix3.x_axis);
                let offset = (at - point) * CM;
                // The offset's part along the axis: the dot summed
                // (x + z) + y, as the original's projection sums it.
                let along = (offset.x * axis.x + offset.z * axis.z) + offset.y * axis.y;
                self.drag = point + axis * along / CM;
                body.world[i].flags &= !LINK_DISABLED;
                body.wake_to(WAKE_COUNT);
            }
            None => body.world[i].flags |= LINK_DISABLED,
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

    /// Whether the leaf is latched and not being unlocked: a press rattles
    /// it (and still drags).
    pub fn latched(&self) -> bool {
        self.door.latched
    }

    /// Whether an unlock is running (the latch does not rattle then).
    pub fn forcing(&self) -> bool {
        self.door.unlock_pending
    }
}

/// The sliding leaves' solver stage: each tick it holds every leaf's glue
/// ([`load_slides`]), runs the step of each leaf the solver integrated
/// right after the integration ([`SolverStage::integrated`]), and hands
/// the stepped glue and its sounds back after the solver
/// ([`unload_slides`]). It touches no pair, link or friction stage.
#[derive(Debug, Default)]
pub struct SlideStage {
    now_ms: i32,
    /// The leaves' glue by their root part's entity, until stepped.
    leaves: Vec<(Entity, SlideBody)>,
    /// The leaves stepped this tick, with their sounds.
    stepped: Vec<(Entity, SlideBody, Vec<MechanismSounded>)>,
}

impl SlideStage {
    /// The stage with `leaves` to step at `now_ms`.
    pub fn with(now_ms: i32, leaves: Vec<(Entity, SlideBody)>) -> Self {
        Self {
            now_ms,
            leaves,
            stepped: Vec::new(),
        }
    }

    /// The leaves stepped since the last call, with their sounds; the
    /// others are dropped.
    pub fn take_stepped(&mut self) -> Vec<(Entity, SlideBody, Vec<MechanismSounded>)> {
        self.leaves.clear();
        std::mem::take(&mut self.stepped)
    }
}

impl SolverStage for SlideStage {
    fn integrated(&mut self, bodies: &mut [&mut Ragdoll]) {
        if self.leaves.is_empty() {
            return;
        }
        for body in bodies.iter_mut() {
            let Some(key) = body.parts.first().map(|p| p.entity) else {
                continue;
            };
            let Some(i) = self.leaves.iter().position(|(e, _)| *e == key) else {
                continue;
            };
            let (e, mut glue) = self.leaves.swap_remove(i);
            let out = glue.tick(body, self.now_ms);
            let sounds = glue.sounds(e, body, &out);
            self.stepped.push((e, glue, sounds));
        }
    }

    fn coupled(&self, _bodies: &[&mut Ragdoll], _coupled: &mut [bool]) {}
}

/// Builds the glue of sliding leaves registered this frame, and steps every
/// awake one right after its integration ([`SlideStage`]). Added by
/// [`crate::prop::PropPlugin`].
pub(crate) fn plugin(app: &mut App) {
    use crate::sim_bridge::SimBridgeAppExt;
    crate::mechanism_sound::register(app);
    app.bridge_snapshot::<SlideBody>();
    app.world_mut()
        .get_resource_or_init::<SolverStages>()
        .add(SlideStage::default());
    app.add_systems(
        FixedUpdate,
        (
            register_slides
                .after(crate::body::TickBehaviour)
                .before(BodySystems),
            load_slides
                .in_set(BodySystems)
                .before(crate::body::step_bodies),
            unload_slides
                .in_set(BodySystems)
                .after(crate::body::step_bodies),
        ),
    );
}

/// Turns each registered leaf with a [`PendingSlide`] into a [`SlideBody`].
fn register_slides(
    mut commands: Commands,
    mut pending: Query<(Entity, &PendingSlide, &mut Ragdoll)>,
) {
    for (e, slide, mut body) in &mut pending {
        commands.entity(e).remove::<PendingSlide>();
        if let Some(glue) = SlideBody::new(&mut body, slide) {
            // A command saved in the word runs from the first tick.
            if glue.door.slide.mode != door::SlideMode::Idle {
                body.wake_to(WAKE_COUNT);
            }
            commands.entity(e).insert(glue);
        }
    }
}

/// Hands every leaf's glue to the [`SlideStage`] before the solver.
fn load_slides(
    ticks: Option<Res<BodyTicks>>,
    slides: Query<(Entity, &SlideBody)>,
    mut stages: ResMut<SolverStages>,
) {
    let Some(stage) = stages.get_mut::<SlideStage>() else {
        return;
    };
    let now = ticks.map_or(0, |t| t.millis() as i32);
    *stage = SlideStage::with(now, slides.iter().map(|(e, g)| (e, g.clone())).collect());
}

/// Puts the glue of the leaves the solver stepped back and sends the
/// sounds each step asked for, in the solver's order.
fn unload_slides(
    mut stages: ResMut<SolverStages>,
    mut slides: Query<&mut SlideBody>,
    mut sounds: MessageWriter<MechanismSounded>,
) {
    let Some(stage) = stages.get_mut::<SlideStage>() else {
        return;
    };
    for (e, glue, cues) in stage.take_stepped() {
        if let Ok(mut slot) = slides.get_mut(e) {
            *slot = glue;
        }
        sounds.write_batch(cues);
    }
}

#[cfg(test)]
#[path = "body_slide_test.rs"]
mod tests;
