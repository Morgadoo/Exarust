//! A sliding door (a portcullis, a secret door) as a body in the solver:
//! the leaf is a hung rigid body held by its slider links (registration
//! adds no origin pivot when the first link slides), and [`SlideBody`] runs
//! the door's sliding rules ([`crate::door::Door`]) on it each tick it is
//! awake. Off unless the level loader is asked for it (`--sliding-doors`):
//! see the caveat below.
//!
//! **Set-up** (the original's door init on a registered sliding leaf). The
//! leaf keeps its registered mobility and the default environment (gravity
//! acts along and across its rails). Its links give the door's shape
//! ([`crate::door::shape`], in centimetres): any slider link makes it slide,
//! and its travel link is the first whose body point is 30–160 cm up. The
//! leaf reads shut when that link's travel fraction is at least 0.995 (1 is
//! shut, 0 open); a frozen leaf parks at its fraction with smoothing 1. The
//! handle is the body point (−150 cm, the node box's centre height, 0).
//!
//! **Each awake tick**, after the solver: the handle's motion since the
//! last tick (cm) is smoothed by half and its squared length is the creak
//! energy; the travel link's fraction is the door's fraction; the door's
//! sliding step ([`crate::door::slide_tick`] through
//! [`crate::door::Door::step`]) then writes its fraction and smoothing back
//! into that link, and the slider law pulls the leaf there (the point drawn
//! `smoothing` of the way to the fraction's place each pass). A signalled
//! open jumps a plain leaf's fraction to 0.6 for 80 ticks; a motorised one
//! crawls 0.01 a tick toward 0 for 300; a close drives 0.075 a tick toward
//! 1 and latches.
//!
//! **Caveat.** The slider law rewrites the travel link's fraction from
//! where the leaf lies on every pass, so a command's fraction pulls the
//! leaf for one pass only: a plain open's jump to 0.6 shoves it 0.01 of the
//! way, and a motorised open's 0.01 a tick, at smoothing 0.1, balances the
//! default environment's gravity a millimetre up vertical rails. The
//! shipped portcullises are also placed with their slider points on their
//! anchors, fraction 0, the open end of the rule. Whether the original's
//! rails run along gravity, or something else lifts the leaf, was not
//! traced; so the live leaves stay behind the loader's flag, and a static
//! leaf is what a level gets by default.
//!
//! Not reproduced: the hand's drag of a sliding leaf (the press only
//! records it), the door's sounds (reported, not played) and the navigation
//! passage report.
//!
//! Evidence: the door's init (its sliding branch) and sliding step, the
//! world-link slider law, read in the instructions.

use bevy::prelude::*;

use super::{BodySystems, BodyTicks, CM, Ragdoll, WAKE_COUNT, weights};
use crate::door::{self, Door, DoorTick, Record, Tick};

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
    /// The handle's smoothed motion (cm a tick).
    smoothed: Vec3,
    /// The fraction last written into the travel link: a command that
    /// changed the door's fraction since (an open's jump) is written into
    /// the link before the tick reads it, as the original's commands write
    /// the link itself.
    written: f32,
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
            smoothed: Vec3::ZERO,
            written: fraction,
        })
    }

    /// The travel link's fraction (1 shut, 0 open).
    pub fn fraction(&self, body: &Ragdoll) -> f32 {
        body.world[self.travel].length
    }

    /// One tick of an awake leaf, after the solver's tick.
    pub fn tick(&mut self, body: &mut Ragdoll, now_ms: i32) -> DoorTick {
        let leaf = &body.parts[0];
        let handle = leaf.point(&self.handle);
        let moved = (handle - self.handle_at) * CM;
        self.handle_at = handle;
        self.smoothed = self.smoothed * 0.5 + moved * 0.5;
        let s = self.smoothed;
        let energy = s.z * s.z + s.y * s.y + s.x * s.x;
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
        out
    }
}

/// Builds the glue of sliding leaves registered this frame and steps every
/// awake one after the solver. Added by [`crate::prop::PropPlugin`].
pub(crate) fn plugin(app: &mut App) {
    app.add_systems(
        FixedUpdate,
        (
            register_slides.before(BodySystems),
            step_slides.after(BodySystems),
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

/// Steps every awake sliding leaf.
fn step_slides(ticks: Option<Res<BodyTicks>>, mut slides: Query<(&mut SlideBody, &mut Ragdoll)>) {
    let now = ticks.map_or(0, |t| t.millis() as i32);
    for (mut glue, mut body) in &mut slides {
        if !body.asleep() {
            glue.tick(&mut body, now);
        }
    }
}

#[cfg(test)]
#[path = "body_slide_test.rs"]
mod tests;
