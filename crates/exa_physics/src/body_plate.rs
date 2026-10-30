//! A push surface (a pressure plate or a push button) as a body in the
//! solver: the plate is a hung rigid body held by its slider links (the
//! stored world links; registration adds no origin pivot when the first
//! link slides), and [`PlateBody`] runs the plate's own rules
//! ([`crate::plate::plate_step`]) on it each tick it is awake.
//!
//! **Set-up** (the original's push-surface init on a registered body). The
//! plate's mobility becomes 0.3 when its registered one is at least 0.5,
//! else 0.03 ([`crate::plate::mobility`]), and the body moves into the
//! mechanism environment (no gravity, 0.001 of its velocity kept a second).
//! It reads its operative record (two targets, kind 2 or flag 8 inverts,
//! flag 2 one-shot, flag 0x10 signals the second target) and starts from
//! the object word's low byte (1 down, 0 up).
//!
//! **Each awake tick**, after the solver: every link's smoothing becomes
//! 0.1 and its travel fraction (the slider's state, which the link law
//! updates from where the plate's point lies on its rail) rises by 0.02
//! while the plate is pushed and falls by 0.01 otherwise; the mean goes back
//! into every link. A mean above 0.7 presses a plate that was up and signals
//! 1 (−1 inverted), below 0.5 releases one that was down and signals 0
//! (none inverted) ([`crate::mechanism::MechanismSignal`]). While it grinds
//! the body is kept awake for 256 ticks.
//!
//! **Pushed** is set by the double-click interaction on a push surface
//! whose operative flag 4 is clear ([`PlateBody::clickable`]) and by a
//! signal of class 5 (any value but 0 pushes, 0 lets go); a one-shot plate
//! lets go once a travel reaches 1. A body standing on a pressure plate
//! moves its slider points down its rails, which raises the travels the
//! step reads: the plate's own ±0.02/0.01 ramp is its spring.
//!
//! **Sounds** (`crate::mechanism_sound`): each awake tick sends the grind
//! voice (`Stnb_mov` at the plate's centre) and, when the plate goes down,
//! the click (`Stnb_fit`).
//!
//! Not reproduced: the class word's `0x1000` bit the original sets
//! (nothing in Exarust reads it).
//!
//! Evidence: the push-surface init and step, the signal dispatch's class 5
//! branch, and the controls' object interaction (class 5, flag 4 clear),
//! read in the instructions.

use bevy::prelude::*;

use super::{BodySystems, Ragdoll, WAKE_COUNT, world_link};
use crate::mechanism::MechanismSignal;
use crate::mechanism_sound::{self, MechanismSounded};
use crate::plate::{self, Plate, PlateTick, Record, Signal};

/// The operative flag that keeps the hand from pushing the surface.
pub const NOT_CLICKABLE: u16 = 0x4;

/// On a push surface's root node until its body is registered: the
/// operative record, the placement's stored word and the placed id the
/// plate signals from.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct PendingPlate {
    pub record: Record,
    pub saved: i32,
    pub id: u32,
}

/// A push surface's body glue: the plate's rules and its slider links.
#[derive(Component, Debug, Clone, PartialEq)]
pub struct PlateBody {
    pub plate: Plate,
    pub record: Record,
    /// The placed id the plate's signals come from.
    pub id: u32,
    /// The slider links' indices in the body's world links.
    links: Vec<usize>,
    /// Scratch travels, kept so a tick does not allocate.
    travels: Vec<f32>,
}

impl PlateBody {
    /// The plate of a registered hung surface (part 0); `None` without a
    /// slider link.
    pub fn new(body: &Ragdoll, pending: &PendingPlate) -> Option<Self> {
        let links: Vec<usize> = (0..body.world.len())
            .filter(|&i| {
                body.world[i].part == 0
                    && !body.world[i].origin
                    && body.world[i].flags & world_link::SLIDER != 0
            })
            .collect();
        if links.is_empty() {
            return None;
        }
        Some(Self {
            plate: Plate::new(&pending.record, (pending.saved & 0xFF) as u8),
            record: pending.record,
            id: pending.id,
            travels: vec![0.0; links.len()],
            links,
        })
    }

    /// Whether the hand's interaction can push this surface: its operative
    /// flag 4 is clear.
    pub fn clickable(&self) -> bool {
        self.record.flags & NOT_CLICKABLE == 0
    }

    /// Pushes the surface (or lets it go) and wakes it, as the interaction
    /// and a class 5 signal do.
    pub fn push(&mut self, body: &mut Ragdoll, pushed: bool) {
        self.plate.pushed = pushed;
        body.wake_to(WAKE_COUNT);
    }

    /// One tick of an awake surface, after the solver's tick.
    pub fn tick(&mut self, body: &mut Ragdoll) -> PlateTick {
        for (k, &i) in self.links.iter().enumerate() {
            let link = &mut body.world[i];
            link.smoothing = plate::LINK_SMOOTHING;
            self.travels[k] = link.length;
        }
        let out = plate::plate_step(&mut self.plate, &mut self.travels);
        for (k, &i) in self.links.iter().enumerate() {
            body.world[i].length = self.travels[k];
        }
        if out.wake {
            body.wake_to(plate::WAKE_TICKS as i32);
        }
        out
    }

    /// The signals a tick's report sends: (target, value), the first
    /// target then the second.
    pub fn signals(&self, out: &PlateTick) -> Vec<(u32, i32)> {
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
}

/// Builds the plate glue of surfaces registered this frame and steps every
/// awake surface after the solver. Added by [`crate::prop::PropPlugin`].
pub(crate) fn plugin(app: &mut App) {
    crate::mechanism_sound::register(app);
    app.add_systems(
        FixedUpdate,
        (
            register_plates.before(BodySystems),
            step_plates.after(BodySystems),
        ),
    );
}

/// Turns each registered surface with a [`PendingPlate`] into a
/// [`PlateBody`]: its mobility and the mechanism environment.
fn register_plates(
    mut commands: Commands,
    mut pending: Query<(Entity, &PendingPlate, &mut Ragdoll)>,
) {
    for (e, surface, mut body) in &mut pending {
        commands.entity(e).remove::<PendingPlate>();
        let Some(glue) = PlateBody::new(&body, surface) else {
            continue;
        };
        let mobility = plate::mobility(body.parts[0].inverse_mass);
        body.parts[0].inverse_mass = mobility;
        body.parts[0].pairing.mobility = mobility;
        body.enter_mechanism_environment();
        commands.entity(e).insert(glue);
    }
}

/// Steps every awake surface, sending the signals and the sounds its
/// reports make.
fn step_plates(
    mut plates: Query<(Entity, &mut PlateBody, &mut Ragdoll)>,
    mut signals: MessageWriter<MechanismSignal>,
    mut sounds: MessageWriter<MechanismSounded>,
) {
    for (e, mut glue, mut body) in &mut plates {
        if body.asleep() {
            continue;
        }
        let out = glue.tick(&mut body);
        let centre = Vec3::from(body.parts[0].transform().translation);
        sounds.write_batch(mechanism_sound::plate_cues(e, centre, &out, &glue.plate));
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
#[path = "body_plate_test.rs"]
mod tests;
