//! The stages of the 83 Hz tick, named and ordered as the original's
//! simulation runs them, so that each piece of work joins its stage instead
//! of one loop. Evidence: `docs/formats/RSQ.md` (Physical body, how a tick
//! runs).
//!
//! A tick, in `FixedUpdate`:
//!
//! 1. [`BodyPreStep`]: what runs before any character (the original's
//!    cloth wind, then its sim objects against nearby bodies).
//! 2. [`CharacterTicks`]: every living character's tick, the player's last,
//!    each character running every [`CharacterStage`] of the
//!    [`CharacterTick`] schedule, in order, before the next character
//!    starts ([`run_character_ticks`]).
//! 3. [`BodySystems`]: the solver, the queued blows then [`solve_with`]:
//!    integration; 20 iterations, each a pair pass then every body's links;
//!    the contacts dropped; sim objects; body–body detection; sim-object
//!    contacts; body–level detection; 8 friction passes, pairs then the
//!    level; the post-step. Each [`SolverStage`] in [`SolverStages`] is
//!    called at every stage it has work for.
//! 4. [`BodyPostStep`]: what runs per character after the solver (the
//!    original's pending hit and its damage, the footstep phase, sounds).

use std::any::Any;

use bevy::ecs::schedule::{ScheduleLabel, SingleThreadedExecutor};
use bevy::prelude::*;

use super::{BodySystems, CharacterTicks, FRICTION_PASSES, LINK_ITERATIONS, Ragdoll};
use crate::contact::Level;

/// The work before any character's tick. In the fixed step.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct BodyPreStep;

/// The work after the solver, per character. In the fixed step.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct BodyPostStep;

/// The schedule of one character's tick: its [`CharacterStage`]s in order,
/// run once for every living character ([`run_character_ticks`]), the
/// character it ticks in [`TickedCharacter`]. Its systems run one at a time
/// and see each earlier character's whole tick; they must not rely on
/// change detection, which spans the other characters' runs.
#[derive(ScheduleLabel, Debug, Clone, PartialEq, Eq, Hash)]
pub struct CharacterTick;

/// The stages of one character's tick, in the original's order. A
/// character runs every stage before the next character starts.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CharacterStage {
    /// Wound painting and red regeneration (a dead character's tick ends
    /// here), the impact accumulator's decay, burning.
    Upkeep,
    /// Readiness, stepping progress, the queued motion, the pin gain,
    /// energy and the blow sag, the daze block and its pose capture.
    Readiness,
    /// Chest velocities and the standing anchor.
    Anchor,
    /// The player's input or the AI: heading, look, aim, the locomotion
    /// and attack requests, steps; waking.
    Behaviour,
    /// Posture and condition: falls and get-ups, or the collapse.
    Posture,
    /// The attack (the swing routine), focus and status effects, health
    /// bookkeeping.
    Attack,
    /// The movement decision, the body's turn and alignment, the
    /// locomotion drive, the clip clock.
    Movement,
    /// The torso twist, the physical capture, the sampler, gait plants
    /// and footsteps.
    Sampler,
    /// The carry, chest pins, balance, the feet, the lift servo and the
    /// leg IK.
    Carry,
}

impl CharacterStage {
    /// Every stage, in the order a character runs them.
    pub const ORDER: [Self; 9] = [
        Self::Upkeep,
        Self::Readiness,
        Self::Anchor,
        Self::Behaviour,
        Self::Posture,
        Self::Attack,
        Self::Movement,
        Self::Sampler,
        Self::Carry,
    ];
}

/// The character the [`CharacterTick`] schedule is ticking: its body root
/// (the [`Ragdoll`]) and its character, the controller entity above it if
/// it has one. Present only while the character ticks run.
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq)]
pub struct TickedCharacter {
    pub body: Entity,
    pub character: Option<Entity>,
}

/// Runs the [`CharacterTick`] schedule once for each character of `order`
/// (the original's order: the player last), each character's whole tick
/// before the next one's.
pub fn run_character_ticks(world: &mut World, order: impl IntoIterator<Item = TickedCharacter>) {
    for ticked in order {
        world.insert_resource(ticked);
        world.run_schedule(CharacterTick);
    }
    world.remove_resource::<TickedCharacter>();
}

/// Orders the tick's sets in the fixed step and builds the character tick's
/// schedule with its stages in order.
pub(super) fn configure(app: &mut App) {
    use CharacterStage::*;
    app.configure_sets(
        FixedUpdate,
        (BodyPreStep, CharacterTicks, BodySystems, BodyPostStep).chain(),
    )
    .edit_schedule(CharacterTick, |schedule| {
        schedule.set_executor(SingleThreadedExecutor::new());
    })
    .configure_sets(
        CharacterTick,
        (
            Upkeep, Readiness, Anchor, Behaviour, Posture, Attack, Movement, Sampler, Carry,
        )
            .chain(),
    )
    .init_resource::<SolverStages>();
}

/// Work a lane adds to the solver's stages (pairs, props' mechanisms, world
/// links, cloth), each method one stage of the original's sector step.
/// `bodies` are the awake bodies the tick solves, in the solver's order.
/// Every method does nothing unless a stage has work there.
pub trait SolverStage: Any + Send + Sync {
    /// After every body is integrated (where the original's doors, levers
    /// and plates run their own step).
    fn integrated(&mut self, _bodies: &mut [&mut Ragdoll]) {}
    /// At the start of each of the 20 iterations, before any body's links
    /// (the pair pass). `iteration` counts 1 to 20, as the original's.
    fn pair_pass(&mut self, _bodies: &mut [&mut Ragdoll], _iteration: usize) {}
    /// After every body's links in each iteration (links between bodies:
    /// grips, bites, the hand's drag).
    fn links(&mut self, _bodies: &mut [&mut Ragdoll], _iteration: usize) {}
    /// After the iterations, before the level contacts are dropped (the
    /// pair contacts are dropped here).
    fn iterated(&mut self, _bodies: &mut [&mut Ragdoll]) {}
    /// The sim objects' step (cloth), after every contact is dropped.
    fn sim_objects(&mut self, _bodies: &mut [&mut Ragdoll]) {}
    /// Body–body detection, before the level's.
    fn detect(&mut self, _bodies: &mut [&mut Ragdoll]) {}
    /// Sim objects against each other, the bodies and the level.
    fn sim_object_contacts(&mut self, _bodies: &mut [&mut Ragdoll]) {}
    /// Each of the 8 friction passes, before the bodies' level friction
    /// (the pair friction).
    fn friction(&mut self, _bodies: &mut [&mut Ragdoll]) {}
    /// After every body's post-step.
    fn post_step(&mut self, _bodies: &mut [&mut Ragdoll]) {}
}

/// The solver stages the fixed step's solver calls, in the order they were
/// added.
#[derive(Resource, Default)]
pub struct SolverStages(Vec<Box<dyn SolverStage>>);

impl SolverStages {
    /// Adds a stage after the others.
    pub fn add(&mut self, stage: impl SolverStage) {
        self.0.push(Box::new(stage));
    }

    /// The first stage of type `T`.
    pub fn get<T: SolverStage>(&self) -> Option<&T> {
        self.0
            .iter()
            .find_map(|s| (&**s as &dyn Any).downcast_ref())
    }

    /// The first stage of type `T`, to change.
    pub fn get_mut<T: SolverStage>(&mut self) -> Option<&mut T> {
        self.0
            .iter_mut()
            .find_map(|s| (&mut **s as &mut dyn Any).downcast_mut())
    }

    /// Every stage, for [`solve_with`].
    pub fn all(&mut self) -> &mut [Box<dyn SolverStage>] {
        &mut self.0
    }
}

/// One tick of the world's solver over `bodies` against the static `level`
/// with `stages`, in the original's order (the character ticks have set
/// every living body's targets): every body integrated (inertia, gravity
/// and its muscles' pull); 20 iterations, each the stages' pair pass, then
/// every body's links, stored level contacts and edges, then the stages'
/// links between bodies; the stages' and every body's contacts dropped;
/// the sim objects; body–body detection; sim-object contacts; every body's
/// spheres tested against the level (an immediate push, the contacts kept
/// for the next tick); 8 friction passes, each the stages' pair friction
/// then every body's level friction; the post-step.
pub fn solve_with<L: Level + ?Sized>(
    bodies: &mut [&mut Ragdoll],
    level: &L,
    stages: &mut [Box<dyn SolverStage>],
) {
    for b in bodies.iter_mut() {
        b.step();
    }
    for s in stages.iter_mut() {
        s.integrated(bodies);
    }
    for iteration in 1..=LINK_ITERATIONS {
        for s in stages.iter_mut() {
            s.pair_pass(bodies, iteration);
        }
        for b in bodies.iter_mut() {
            b.iterate();
        }
        for s in stages.iter_mut() {
            s.links(bodies, iteration);
        }
    }
    for s in stages.iter_mut() {
        s.iterated(bodies);
    }
    for b in bodies.iter_mut() {
        for p in &mut b.parts {
            p.contacts.clear();
        }
    }
    for s in stages.iter_mut() {
        s.sim_objects(bodies);
    }
    for s in stages.iter_mut() {
        s.detect(bodies);
    }
    for s in stages.iter_mut() {
        s.sim_object_contacts(bodies);
    }
    for b in bodies.iter_mut() {
        b.detect(level);
    }
    for _ in 0..FRICTION_PASSES {
        for s in stages.iter_mut() {
            s.friction(bodies);
        }
        for b in bodies.iter_mut() {
            b.rub();
        }
    }
    for b in bodies.iter_mut() {
        b.post_step();
    }
    for s in stages.iter_mut() {
        s.post_step(bodies);
    }
}

#[cfg(test)]
#[path = "body_stages_test.rs"]
mod tests;
