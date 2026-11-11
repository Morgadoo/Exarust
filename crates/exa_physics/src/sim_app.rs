//! The simulation's own world: the shell the fixed step moves into.
//!
//! The original runs every scene's simulation on a thread of its own. Other
//! threads hand it work through a ring of deferred calls that it runs at the
//! top of each pass, before it looks at its clock; the renderer reads the
//! last stepped tick and time it publishes (`step_driver`). Exarust is moving
//! its fixed step the same way, in stages (OpenSpec
//! `plan-the-simulation-thread` and `shell-the-simulation-world`):
//!
//! - [`SimShell`] owns a Bevy [`SubApp`]: a [`World`] of its own with the
//!   [`SimStep`] schedule, run either inline on the frame's thread or on a
//!   thread named `exa-sim` ([`SimMode`]).
//! - [`SimInbox`] (frame world) queues [`SimCommand`]s in post order; a step
//!   runs every command queued before it, then [`SimStep`] once.
//! - [`SimOutbox`] (simulation world) queues [`FrameEvent`]s in send order;
//!   the frame runs them after the step, so none is lost or doubled.
//! - After each step the shell publishes a [`SimSnapshot`] through a
//!   latest-wins [`SnapshotBuffer`]; the frame keeps the last two
//!   ([`Snapshots`]) to draw between them.
//! - [`SimId`] names an object in both worlds; each world keeps its own
//!   [`SimLinks`] from ids to its entities.
//!
//! The simulation runs in **lockstep** with the frame's fixed step: the
//! [`SimLockstep`] schedule runs before `FixedFirst` in every pass of
//! `FixedMain` and waits for exactly one simulation step. Without the bridge
//! the simulation's world holds only its clock; with it
//! ([`crate::sim_bridge::SimBridgePlugin`]) the fixed step's schedules run
//! there, the data they touch copied in before and out after each step.
//!
//! [`SimMode::Free`] runs it free of the frame instead, as the original's
//! thread: its own 12 ms real-clock step, asleep while the frame holds it
//! ([`SimPause`]), the frame's packets drained before each step and a
//! stamped packet ([`FreeStamp`]) sent after it; the frame exchanges
//! packets once a frame without waiting ([`exchange_free`]) and draws at
//! the original's render fraction.
//!
//! The original creates its simulation thread at the time-critical priority
//! (15) and enters the engine's floating-point mode on it, flushing
//! denormals ([`crate::float_mode`]). Exarust's `exa-sim` thread enters the
//! same mode. It keeps the default priority: a real-time or raised priority
//! needs privileges a desktop user does not have on Linux, and the step is
//! paced by its own clock, so the priority changes when a step runs, not
//! what it computes.

use std::collections::HashMap;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

use bevy::app::{FixedMainScheduleOrder, RunFixedMainLoop, RunFixedMainLoopSystems, SubApp};
use bevy::ecs::schedule::ScheduleLabel;
use bevy::prelude::*;

use crate::STEP_HZ;
use crate::sim_bridge::{
    BridgeCoverage, BridgeIn, BridgeOut, FrameBridge, POOL_BATCH, frame_apply, frame_gather,
    reserve_ids, run_fixed_order, sim_apply, sim_apply_nothing, sim_bridge_started, sim_gather,
};
use crate::step_driver::{PAUSED_SLEEP_MS, Poll, StepDriver};

/// Work the frame hands the simulation, run on the simulation's world at
/// the start of the next step, in post order.
pub type SimCommand = Box<dyn FnOnce(&mut World) + Send>;
/// Work the simulation hands the frame, run on the frame's world after the
/// step that sent it, in send order.
pub type FrameEvent = Box<dyn FnOnce(&mut World) + Send>;

/// The step (ms): the original's 12 ms (83 Hz).
pub const STEP_MS: u64 = (1000.0 / STEP_HZ) as u64;

/// The simulation's schedule: one fixed step of the simulation's world.
#[derive(ScheduleLabel, Debug, Clone, PartialEq, Eq, Hash)]
pub struct SimStep;

/// Work the fixed step does once at the start of each frame that runs ticks
/// (the bodies' frame start). In one world it runs before the frame's fixed
/// loop; behind the bridge it moves with the fixed step and runs at the
/// first step after a frame began, before that step's commands.
#[derive(ScheduleLabel, Debug, Clone, PartialEq, Eq, Hash)]
pub struct SimFrameStart;

/// Work the original runs before every step on its simulation thread: the
/// AI manager's frame routine is the loop's pre-step hook, called before
/// each 12 ms step (the rise and its restore, then the alarm zones). It runs
/// every fixed pass, before [`SimFrameStart`], in both worlds.
#[derive(ScheduleLabel, Debug, Clone, PartialEq, Eq, Hash)]
pub struct SimPreStep;

/// The order of [`SimFrameStart`]'s work, as the frame ran it before it
/// moved: what ran in `Update` (`Early`: nothing now, the rise went to
/// [`SimPreStep`]), then what ran in `PostUpdate` (`Late`: nothing now, the
/// hit chain and its readers went to the fixed step's post-step, as the
/// original deals a pending hit every step), then the bodies' own frame
/// start, which restarts their tick count.
#[derive(bevy::ecs::schedule::SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SimFrameStartSystems {
    Early,
    Late,
    Bodies,
}

/// The step of the fixed order that runs [`SimFrameStart`]: after the
/// lockstep, before the posted commands, on the first fixed pass of each
/// frame. It moves with the fixed step behind the bridge, so both worlds
/// run the frame start at the same point, in the same order.
#[derive(ScheduleLabel, Debug, Clone, PartialEq, Eq, Hash)]
pub struct FrameStartStep;

/// Runs [`SimPreStep`] on every fixed pass, then [`SimFrameStart`] on the
/// first fixed pass of a frame (Bevy's frame count, which the bridge
/// copies, tells the frames apart).
fn run_frame_start(world: &mut World, mut last: Local<Option<u32>>) {
    let _ = world.try_run_schedule(SimPreStep);
    let frame = world
        .get_resource::<bevy::diagnostic::FrameCount>()
        .map(|f| f.0);
    if frame.is_some() && *last == frame {
        return;
    }
    *last = frame;
    let _ = world.try_run_schedule(SimFrameStart);
}

/// Writes the frame hands the fixed step: each runs at the start of the
/// next fixed step, before `FixedFirst`, on the world that runs the step
/// (the frame's in one world, the simulation's behind the bridge, where
/// the queue is lent for the step). A frame system that decides a change to
/// simulation data posts it here instead of writing it, so the change lands
/// where the copy used to land it, and a simulation running apart from the
/// frame (stage 6) can take it in order.
#[derive(Resource, Default)]
pub struct FixedCommands {
    queued: Mutex<Vec<SimCommand>>,
    /// Behind the bridge: the frame's writes to snapshot data
    /// ([`FixedCommands::write`]), run on the simulation's copies before the
    /// frame's other changes are applied there.
    writes: Mutex<Vec<SimCommand>>,
    /// The bridge is on (the frame's copies of snapshot data are snapshots).
    bridged: bool,
}

impl FixedCommands {
    /// Queues `write` for the start of the next fixed step.
    pub fn post(&mut self, write: impl FnOnce(&mut World) + Send + 'static) {
        queue(&mut self.queued).push(Box::new(write));
    }

    /// Behind the bridge, queues `change` for the simulation's world, run
    /// before the frame's other changes are applied there (with the posted
    /// writes): a removal of a resource the simulation owns that a new
    /// insert from the frame may follow in the same step. In one world it
    /// does nothing (the frame's own change is the simulation's).
    pub fn post_first_bridged(&mut self, change: impl FnOnce(&mut World) + Send + 'static) {
        if self.bridged {
            queue(&mut self.writes).push(Box::new(change));
        }
    }

    /// Queues `system` with `input`, run once at the start of the next
    /// fixed step (a function system: it is cached in that world).
    pub fn run<I, M, S>(&mut self, system: S, input: I::Inner<'static>)
    where
        I: bevy::ecs::system::SystemInput + Send + 'static,
        I::Inner<'static>: Send,
        S: IntoSystem<I, (), M> + Send + 'static,
    {
        self.post(move |world| {
            if let Err(e) = world.run_system_cached_with(system, input) {
                warn!("fixed command: {e}");
            }
        });
    }

    /// A frame write to simulation data the frame reads as a snapshot.
    /// In one world `local` is the live value and is simply written. Behind
    /// the bridge the frame's copy is written without a change mark (the
    /// next copy out replaces it; same-frame readers see the write at once)
    /// and the same write is queued for the simulation's copy, where it runs
    /// at the start of the next step before the frame's other changes are
    /// applied. The original writes such data straight from its main
    /// thread; Exarust lands it at the next step, a step's difference at
    /// most. Returns what `write` returned on the frame's copy.
    pub fn write<T, R>(
        &mut self,
        entity: Entity,
        local: &mut Mut<T>,
        write: impl Fn(&mut T) -> R + Send + Sync + 'static,
    ) -> R
    where
        T: Component<Mutability = bevy::ecs::component::Mutable>,
    {
        if !self.bridged {
            return write(local);
        }
        let result = write(local.bypass_change_detection());
        queue(&mut self.writes).push(Box::new(move |world: &mut World| {
            if let Some(mut value) = world.get_mut::<T>(entity) {
                write(&mut value);
            }
        }));
        result
    }

    /// [`FixedCommands::write`] for two components of one entity written
    /// together (a mechanism and its body).
    pub fn write_pair<A, B, R>(
        &mut self,
        entity: Entity,
        a: &mut Mut<A>,
        b: &mut Mut<B>,
        write: impl Fn(&mut A, &mut B) -> R + Send + Sync + 'static,
    ) -> R
    where
        A: Component<Mutability = bevy::ecs::component::Mutable>,
        B: Component<Mutability = bevy::ecs::component::Mutable>,
    {
        if !self.bridged {
            return write(a, b);
        }
        let result = write(a.bypass_change_detection(), b.bypass_change_detection());
        queue(&mut self.writes).push(Box::new(move |world: &mut World| {
            let mut query = world.query::<(&mut A, &mut B)>();
            if let Ok((mut a, mut b)) = query.get_mut(world, entity) {
                write(&mut a, &mut b);
            }
        }));
        result
    }

    /// [`FixedCommands::write`] for one component of several entities
    /// written together (a door leaf and its partner).
    pub fn write_many<T, R, const N: usize>(
        &mut self,
        entities: [Entity; N],
        locals: &mut [Mut<T>; N],
        write: impl Fn([&mut T; N]) -> R + Send + Sync + 'static,
    ) -> R
    where
        T: Component<Mutability = bevy::ecs::component::Mutable>,
    {
        if !self.bridged {
            return write(locals.each_mut().map(|m| &mut **m));
        }
        let result = write(locals.each_mut().map(|m| m.bypass_change_detection()));
        queue(&mut self.writes).push(Box::new(move |world: &mut World| {
            let mut query = world.query::<&mut T>();
            if let Ok(mut values) = query.get_many_mut(world, entities) {
                write(values.each_mut().map(|m| &mut **m));
            }
        }));
        result
    }

    /// Marks the frame's copies of snapshot data as snapshots (the bridge).
    pub fn set_bridged(&mut self, bridged: bool) {
        self.bridged = bridged;
    }

    /// The bridge is on: the frame's copies of simulation data are copies.
    pub fn bridged(&self) -> bool {
        self.bridged
    }

    /// The queued snapshot writes, for the step's packet.
    pub fn take_writes(&mut self) -> Vec<SimCommand> {
        std::mem::take(queue(&mut self.writes))
    }

    /// The posted calls, for the step's packet (behind the bridge each
    /// side keeps its own queue).
    pub fn take_posts(&mut self) -> Vec<SimCommand> {
        std::mem::take(queue(&mut self.queued))
    }

    /// Queues calls another world posted, after the ones queued here.
    pub fn queue_posts(&mut self, posts: Vec<SimCommand>) {
        queue(&mut self.queued).extend(posts);
    }

    pub fn len(&mut self) -> usize {
        queue(&mut self.queued).len()
    }

    pub fn is_empty(&mut self) -> bool {
        queue(&mut self.queued).is_empty()
    }
}

/// [`FixedCommands::write`] for a resource the simulation owns, which the
/// frame reads as a snapshot (`bridge_snapshot_resource`): written at once
/// on the frame's copy without a change mark, and again on the
/// simulation's at the start of the next step. In one world, or without a
/// queue (a test's bare app), a plain write.
pub fn write_resource<T, R, L>(
    fixed: Option<&mut FixedCommands>,
    local: &mut L,
    write: impl Fn(&mut T) -> R + Send + Sync + 'static,
) -> R
where
    T: Resource + Component<Mutability = bevy::ecs::component::Mutable>,
    L: bevy::ecs::change_detection::DetectChangesMut<Inner = T> + std::ops::DerefMut<Target = T>,
{
    let Some(fixed) = fixed.filter(|f| f.bridged) else {
        return write(&mut **local);
    };
    let result = write(local.bypass_change_detection());
    queue(&mut fixed.writes).push(Box::new(move |world: &mut World| {
        if let Some(mut value) = world.get_resource_mut::<T>() {
            write(&mut value);
        }
    }));
    result
}

/// The simulation's writes to data the frame owns: the frame takes a value
/// such as the flow, the open screens or the sound bank in (copied in, or
/// not bridged at all), and a fixed-step system that changes it writes its
/// own copy at once and queues the same write for the frame, which runs it
/// after the step, in order. In one world (`bridged` false) a plain write.
/// Behind the bridge the simulation's world holds its own queue; the frame's
/// is unused.
#[derive(Resource, Default)]
pub struct FrameCommands {
    queued: Mutex<Vec<FrameEvent>>,
    bridged: bool,
}

impl FrameCommands {
    /// The simulation's queue behind the bridge.
    pub fn bridged() -> Self {
        Self {
            queued: Mutex::default(),
            bridged: true,
        }
    }

    /// Writes a resource the frame owns: the local copy at once (without a
    /// change mark behind the bridge, so it is not sent back), and the
    /// frame's after the step.
    pub fn write_resource<T, R>(
        &mut self,
        local: &mut ResMut<T>,
        write: impl Fn(&mut T) -> R + Send + Sync + 'static,
    ) -> R
    where
        T: Resource + Component<Mutability = bevy::ecs::component::Mutable>,
    {
        if !self.bridged {
            return write(local);
        }
        let result = write(local.bypass_change_detection());
        queue(&mut self.queued).push(Box::new(move |world: &mut World| {
            if let Some(mut value) = world.get_resource_mut::<T>() {
                write(&mut value);
            }
        }));
        result
    }

    /// [`Self::write_resource`] for a component of `entity` the frame owns.
    pub fn write<T, R>(
        &mut self,
        entity: Entity,
        local: &mut Mut<T>,
        write: impl Fn(&mut T) -> R + Send + Sync + 'static,
    ) -> R
    where
        T: Component<Mutability = bevy::ecs::component::Mutable>,
    {
        if !self.bridged {
            return write(local);
        }
        let result = write(local.bypass_change_detection());
        queue(&mut self.queued).push(Box::new(move |world: &mut World| {
            if let Some(mut value) = world.get_mut::<T>(entity) {
                write(&mut value);
            }
        }));
        result
    }

    /// Runs `event` on the frame's world after the step: behind the bridge
    /// right after the step's copy out, in one world at the end of the
    /// fixed step ([`FrameCommandsStep`]). For work only the frame does with
    /// the step's results, such as starting the sounds a step decided on.
    pub fn send(&mut self, event: impl FnOnce(&mut World) + Send + 'static) {
        queue(&mut self.queued).push(Box::new(event));
    }

    /// The queued writes, for the frame.
    pub fn take(&mut self) -> Vec<FrameEvent> {
        std::mem::take(queue(&mut self.queued))
    }
}

/// Adds the frame's [`FrameCommands`] (unbridged: plain writes) once.
pub fn add_frame_commands(app: &mut App) {
    if app.world().contains_resource::<FrameCommands>() {
        return;
    }
    use crate::sim_bridge::SimBridgeAppExt;
    app.init_resource::<FrameCommands>()
        .bridge_sim_only::<FrameCommands>();
    if app.world().contains_resource::<FixedMainScheduleOrder>() {
        app.init_schedule(FrameCommandsStep)
            .add_systems(FrameCommandsStep, run_frame_commands);
        app.world_mut()
            .resource_mut::<FixedMainScheduleOrder>()
            .insert_after(FixedLast, FrameCommandsStep);
    }
}

/// The schedule that runs the [`FrameCommands`] queued in one world, after
/// `FixedLast` (behind the bridge the simulation's queue goes to the frame
/// with the step's events instead, and this finds the queue bridged).
#[derive(ScheduleLabel, Debug, Clone, PartialEq, Eq, Hash)]
pub struct FrameCommandsStep;

pub fn run_frame_commands(world: &mut World) {
    let Some(mut commands) = world.get_resource_mut::<FrameCommands>() else {
        return;
    };
    if commands.bridged {
        return;
    }
    for event in commands.take() {
        event(world);
    }
}

/// The schedule that runs the [`FixedCommands`], before `FixedFirst`.
#[derive(ScheduleLabel, Debug, Clone, PartialEq, Eq, Hash)]
pub struct FixedCommandsStep;

pub fn run_fixed_commands(world: &mut World) {
    let Some(mut commands) = world.get_resource_mut::<FixedCommands>() else {
        return;
    };
    let queued = std::mem::take(queue(&mut commands.queued));
    for write in queued {
        write(world);
    }
}

/// Marks an app whose fixed step runs [`FixedCommands`].
#[derive(Resource)]
struct FixedCommandsRunner;

/// Adds [`FixedCommands`] and the step that runs them, once (every plugin
/// that posts calls it). An app without the fixed main schedule (a test's
/// bare app) gets the queue alone: its writes apply directly there.
pub fn add_fixed_commands(app: &mut App) {
    if app.world().contains_resource::<FixedCommandsRunner>() {
        return;
    }
    if !app.world().contains_resource::<FixedMainScheduleOrder>() {
        app.init_resource::<FixedCommands>();
        return;
    }
    use crate::sim_bridge::SimBridgeAppExt;
    app.insert_resource(FixedCommandsRunner)
        .init_resource::<FixedCommands>()
        .init_schedule(FixedCommandsStep)
        .add_systems(FixedCommandsStep, run_fixed_commands)
        // Each world keeps its own queue; the frame's posts and writes go
        // in its packet to the simulation (`crate::sim_bridge`).
        .bridge_own_resource::<FixedCommands>();
    app.world_mut()
        .resource_mut::<FixedMainScheduleOrder>()
        .insert_before(FixedFirst, FixedCommandsStep);
}

/// Marks an app whose frame runs [`SimFrameStart`].
#[derive(Resource)]
struct FrameStartRunner;

/// Adds [`SimFrameStart`] and its runner before the frame's fixed loop,
/// once (every plugin with frame-start work calls it).
pub fn add_frame_start(app: &mut App) {
    if app.world().contains_resource::<FrameStartRunner>() {
        return;
    }
    app.insert_resource(FrameStartRunner)
        .init_schedule(SimFrameStart)
        .init_schedule(SimPreStep)
        // Their systems write the same bodies: one at a time, in order.
        .edit_schedule(SimFrameStart, |schedule| {
            schedule.set_executor(bevy::ecs::schedule::SingleThreadedExecutor::new());
        })
        .edit_schedule(SimPreStep, |schedule| {
            schedule.set_executor(bevy::ecs::schedule::SingleThreadedExecutor::new());
        })
        .configure_sets(
            SimFrameStart,
            (
                SimFrameStartSystems::Early,
                SimFrameStartSystems::Late,
                SimFrameStartSystems::Bodies,
            )
                .chain(),
        )
        .init_schedule(FrameStartStep)
        .add_systems(FrameStartStep, run_frame_start);
    let mut order = app.world_mut().resource_mut::<FixedMainScheduleOrder>();
    let before = if order.labels.contains(&FixedCommandsStep.intern()) {
        FixedCommandsStep.intern()
    } else {
        FixedFirst.intern()
    };
    let at = order
        .labels
        .iter()
        .position(|l| *l == before)
        .unwrap_or(order.labels.len());
    order.labels.insert(at, FrameStartStep.intern());
}

/// The frame's schedule that steps the simulation in lockstep, run before
/// `FixedFirst` in every pass of `FixedMain`.
#[derive(ScheduleLabel, Debug, Clone, PartialEq, Eq, Hash)]
pub struct SimLockstep;

/// An object's name in both worlds.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SimId(pub u64);

/// Issues [`SimId`]s, in order, from the frame's world.
#[derive(Resource, Debug, Default)]
pub struct SimIds {
    next: u64,
}

impl SimIds {
    /// The next unused id.
    pub fn issue(&mut self) -> SimId {
        self.next += 1;
        SimId(self.next)
    }
}

/// One world's map between [`SimId`]s and its entities.
#[derive(Resource, Debug, Default)]
pub struct SimLinks {
    entities: HashMap<SimId, Entity>,
    ids: HashMap<Entity, SimId>,
}

impl SimLinks {
    /// Links `id` to `entity`, replacing either's earlier link.
    pub fn link(&mut self, id: SimId, entity: Entity) {
        self.unlink(id);
        if let Some(old) = self.ids.remove(&entity) {
            self.entities.remove(&old);
        }
        self.entities.insert(id, entity);
        self.ids.insert(entity, id);
    }

    /// Forgets `id`, returning the entity it named.
    pub fn unlink(&mut self, id: SimId) -> Option<Entity> {
        let entity = self.entities.remove(&id)?;
        self.ids.remove(&entity);
        Some(entity)
    }

    pub fn entity(&self, id: SimId) -> Option<Entity> {
        self.entities.get(&id).copied()
    }

    pub fn id(&self, entity: Entity) -> Option<SimId> {
        self.ids.get(&entity).copied()
    }

    pub fn len(&self) -> usize {
        self.entities.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entities.is_empty()
    }
}

/// The frame's queue of [`SimCommand`]s for the next step.
#[derive(Resource, Default)]
pub struct SimInbox {
    // Behind a mutex only so the queue is `Sync`; it is reached through
    // `&mut self`.
    queued: Mutex<Vec<SimCommand>>,
    posted: u64,
}

impl SimInbox {
    /// Queues `command` for the next step.
    pub fn post(&mut self, command: impl FnOnce(&mut World) + Send + 'static) {
        queue(&mut self.queued).push(Box::new(command));
        self.posted += 1;
    }

    /// Commands posted since the start.
    pub fn posted(&self) -> u64 {
        self.posted
    }

    /// Queues a command that spawns an entity in the simulation's world,
    /// built by `build`, named `id` there; the frame's `entity` is linked
    /// to `id` in `frame_links` at once.
    pub fn spawn_linked(
        &mut self,
        id: SimId,
        entity: Entity,
        frame_links: &mut SimLinks,
        build: impl FnOnce(&mut EntityWorldMut) + Send + 'static,
    ) {
        frame_links.link(id, entity);
        self.post(move |world| {
            let mut spawned = world.spawn(id);
            build(&mut spawned);
            let sim_entity = spawned.id();
            world.resource_mut::<SimLinks>().link(id, sim_entity);
        });
    }

    /// Queues a command that despawns `id`'s entity in the simulation's
    /// world.
    pub fn despawn_linked(&mut self, id: SimId, frame_links: &mut SimLinks) {
        frame_links.unlink(id);
        self.post(move |world| {
            if let Some(entity) = world.resource_mut::<SimLinks>().unlink(id) {
                world.despawn(entity);
            }
        });
    }

    fn take(&mut self) -> Vec<SimCommand> {
        std::mem::take(queue(&mut self.queued))
    }
}

/// The simulation's queue of [`FrameEvent`]s for the frame.
#[derive(Resource, Default)]
pub struct SimOutbox {
    queued: Mutex<Vec<FrameEvent>>,
}

/// A queue behind a mutex held by `&mut`: no lock is taken.
fn queue<T>(queue: &mut Mutex<Vec<T>>) -> &mut Vec<T> {
    queue.get_mut().unwrap_or_else(|e| e.into_inner())
}

impl SimOutbox {
    /// Queues `event` for the frame.
    pub fn send(&mut self, event: impl FnOnce(&mut World) + Send + 'static) {
        queue(&mut self.queued).push(Box::new(event));
    }
}

/// The simulation's clock: the steps taken and the game time they make.
#[derive(Resource, Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct SimClock {
    pub tick: u64,
    pub time_ms: u64,
}

/// What the simulation publishes after each step. It grows with the
/// stages: the bodies' poses, props, cloth, lights and the GUI's view.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct SimSnapshot {
    pub tick: u64,
    pub time_ms: u64,
    /// Objects linked in the simulation's world.
    pub linked: usize,
}

/// A latest-wins slot shared by the simulation (which publishes) and the
/// frame (which takes): neither waits on the other, and a snapshot the
/// frame did not take before the next one is replaced.
pub struct SnapshotBuffer<T> {
    slot: Arc<Mutex<Option<T>>>,
}

impl<T> Clone for SnapshotBuffer<T> {
    fn clone(&self) -> Self {
        Self {
            slot: self.slot.clone(),
        }
    }
}

impl<T> Default for SnapshotBuffer<T> {
    fn default() -> Self {
        Self {
            slot: Arc::new(Mutex::new(None)),
        }
    }
}

impl<T> SnapshotBuffer<T> {
    /// Replaces the slot's snapshot.
    pub fn publish(&self, snapshot: T) {
        *self.slot.lock().unwrap_or_else(|e| e.into_inner()) = Some(snapshot);
    }

    /// The newest snapshot not yet taken.
    pub fn take(&self) -> Option<T> {
        self.slot.lock().unwrap_or_else(|e| e.into_inner()).take()
    }
}

/// The frame's last two snapshots, to draw between.
#[derive(Resource, Debug)]
pub struct Snapshots<T> {
    pub previous: Option<T>,
    pub current: Option<T>,
}

impl<T> Default for Snapshots<T> {
    fn default() -> Self {
        Self {
            previous: None,
            current: None,
        }
    }
}

impl<T> Snapshots<T> {
    /// `snapshot` becomes the current one, the current one the previous.
    pub fn push(&mut self, snapshot: T) {
        self.previous = self.current.replace(snapshot);
    }
}

/// Where the simulation's world runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SimMode {
    /// On the frame's thread, inside its fixed step.
    Inline,
    /// On the `exa-sim` thread; the frame waits for each step.
    #[default]
    Thread,
    /// On the `exa-sim` thread, free of the frame, as the original's
    /// simulation thread: its own 12 ms real-clock step ([`crate::step_driver`]),
    /// asleep while the frame's packets say it is paused, the frame's packets
    /// drained before each step, a packet stamped with the step's tick and
    /// time sent after it. The frame neither steps nor waits: it applies
    /// every packet that arrived, in order, and sends its own.
    Free,
}

/// A step the frame asks for: its commands and, with the bridge, the
/// frame's changes.
struct StepRequest {
    commands: Vec<SimCommand>,
    bridge: Option<BridgeIn>,
}

/// A step's results, handed back to the frame.
struct Stepped {
    events: Vec<FrameEvent>,
    bridge: Option<BridgeOut>,
}

/// Builds the simulation's sub-app: its world with the clock, the links,
/// the outbox and the [`SimStep`] schedule (the clock's advance, then the
/// fixed step's schedules once the bridge has moved them).
///
/// `reserved` are ids the frame's world reserved for it: the world takes
/// its own entities from them, so its ids never meet the frame's
/// ([`crate::sim_bridge`]). Its first two entities (a new world's query
/// filters and schedules) are the frame's own two, which stay resources.
pub fn new_sim_app(reserved: &[Entity]) -> SubApp {
    let mut sub = SubApp::new();
    sub.world_mut().entity_allocator_mut().free_many(reserved);
    sub.init_resource::<SimClock>()
        .init_resource::<SimLinks>()
        .init_resource::<SimOutbox>()
        .init_schedule(SimStep)
        .add_systems(SimStep, (advance_clock, run_fixed_order).chain());
    if orphan_trace() {
        trace_orphans(sub.world_mut(), "simulation");
    }
    sub
}

/// Debug tool (`EXA_ORPHAN_TRACE=1`, environment): logs every `ChildOf`
/// inserted with a parent its world does not hold, with the side and the
/// child's components, before Bevy's relationship hook drops the link
/// ("relates to an entity that does not exist"). An entity the simulation
/// parents to a frame-only node (or the reverse) shows here.
fn orphan_trace() -> bool {
    std::env::var_os("EXA_ORPHAN_TRACE").is_some_and(|v| v != "0")
}

/// Watches `world`'s `ChildOf` inserts for [`orphan_trace`].
pub fn trace_orphans(world: &mut World, side: &'static str) {
    world.add_observer(
        move |insert: On<Insert<ChildOf>>,
              links: Query<&ChildOf>,
              entities: &bevy::ecs::entity::Entities,
              all: Query<EntityRef>,
              components: &bevy::ecs::component::Components| {
            let child = insert.event().entity;
            let Ok(link) = links.get(child) else {
                return;
            };
            let parent = link.parent();
            if entities.contains(parent) {
                return;
            }
            let names: Vec<String> = all
                .get(child)
                .map(|e| {
                    e.archetype()
                        .components()
                        .iter()
                        .filter_map(|&id| components.get_name(id).map(|n| n.to_string()))
                        .collect()
                })
                .unwrap_or_default();
            warn!("orphan trace ({side}): {child} takes ChildOf({parent}), which this world does not hold; it carries {names:?}");
        },
    );
}

/// Adds one step to the simulation's clock.
fn advance_clock(mut clock: ResMut<SimClock>) {
    clock.tick += 1;
    clock.time_ms += STEP_MS;
}

/// One step of the simulation's world: the commands in order, the
/// schedule, then the snapshot and the events.
fn step_world(
    world: &mut World,
    request: StepRequest,
    snapshots: &SnapshotBuffer<SimSnapshot>,
) -> Stepped {
    let bridged = request.bridge.is_some();
    let (coverage, apply_us) = apply_request(world, request);
    run_step(world, bridged, coverage, apply_us, snapshots)
}

/// The frame's packet on the simulation's world: its changes, then its
/// commands.
fn apply_request(world: &mut World, request: StepRequest) -> (Option<BridgeCoverage>, u64) {
    let (mut coverage, mut apply_us) = (None, 0);
    if let Some(bridge_in) = request.bridge {
        (coverage, apply_us, _) = sim_apply(world, bridge_in);
    }
    for command in request.commands {
        command(world);
    }
    (coverage, apply_us)
}

/// The step itself, then the snapshot and the events.
fn run_step(
    world: &mut World,
    bridged: bool,
    mut coverage: Option<BridgeCoverage>,
    apply_us: u64,
    snapshots: &SnapshotBuffer<SimSnapshot>,
) -> Stepped {
    world.run_schedule(SimStep);
    // The step's writes to the frame's data run on the frame first, before
    // the step's other events.
    let mut written = world
        .get_resource_mut::<FrameCommands>()
        .map(|mut f| f.take())
        .unwrap_or_default();
    let bridge = bridged.then(|| {
        let (packet, gather_us, pool) = sim_gather(world);
        BridgeOut {
            packet,
            coverage: coverage.take(),
            apply_us,
            gather_us,
            pool,
        }
    });
    let clock = *world.resource::<SimClock>();
    snapshots.publish(SimSnapshot {
        tick: clock.tick,
        time_ms: clock.time_ms,
        linked: world.resource::<SimLinks>().len(),
    });
    written.extend(std::mem::take(queue(
        &mut world.resource_mut::<SimOutbox>().queued,
    )));
    Stepped {
        events: written,
        bridge,
    }
}

/// The free-running simulation's own fixed-step clock: the frame's
/// `Time<Fixed>` no longer steps it, so its systems' `Time` and
/// `Time<Fixed>` advance by one timestep a step here, written without a
/// change mark so that they are not sent to the frame.
#[derive(Resource, Clone, Copy)]
struct FreeTime(Time<Fixed>);

fn advance_free_time(world: &mut World) {
    let mut own = match world.remove_resource::<FreeTime>() {
        Some(own) => own,
        None => FreeTime(
            world
                .get_resource::<Time<Fixed>>()
                .copied()
                .unwrap_or_else(|| Time::<Fixed>::from_hz(STEP_HZ)),
        ),
    };
    let step = own.0.timestep();
    own.0.advance_by(step);
    if let Some(mut fixed) = world.get_resource_mut::<Time<Fixed>>() {
        *fixed.bypass_change_detection() = own.0;
    } else {
        world.insert_resource(own.0);
    }
    let generic = own.0.as_generic();
    if let Some(mut time) = world.get_resource_mut::<Time>() {
        *time.bypass_change_detection() = generic;
    } else {
        world.insert_resource(generic);
    }
    world.insert_resource(own);
}

/// One free step: every frame packet that arrived since the last step, in
/// order (Bevy's messages are updated at most once a step, as the frame
/// updates its own once a frame), then the step.
fn free_step(
    world: &mut World,
    requests: Vec<FreeRequest>,
    snapshots: &SnapshotBuffer<SimSnapshot>,
) -> Stepped {
    let mut bridged = false;
    let (mut coverage, mut apply_us) = (None, 0);
    let mut updated = false;
    for FreeRequest { mut request, .. } in requests {
        if let Some(b) = request.bridge.as_mut() {
            bridged = true;
            if b.packet.update_messages {
                b.packet.update_messages = !updated;
                updated = true;
            }
        }
        let (c, us) = apply_request(world, request);
        coverage = coverage.or(c);
        apply_us += us;
    }
    if !bridged && sim_bridge_started(world) {
        sim_apply_nothing(world);
        bridged = true;
    }
    advance_free_time(world);
    run_step(world, bridged, coverage, apply_us, snapshots)
}

/// The free-running simulation's clock (ms): the real clock since the
/// shell started, less the held time, times the speed the frame asks for
/// (`1` but for Exarust's debug speed keys; the original's loop reads the
/// real clock unscaled).
#[derive(Debug, Clone, Copy)]
struct FreeClock {
    ms: f64,
    last: std::time::Instant,
    speed: f32,
    held_ms: f64,
}

impl FreeClock {
    /// Moves the clock to `now`: on by the real time since the last
    /// reading times the speed, or (held) not at all.
    fn advance(&mut self, now: std::time::Instant, held: bool) {
        let real = (now - self.last).as_secs_f64() * 1000.0;
        self.last = now;
        if held {
            self.held_ms += real;
        } else {
            self.ms += real * f64::from(self.speed);
        }
    }
}

/// The free-running simulation thread: the original's loop. While the
/// frame's packets say held it sleeps 11 ms a pass and the held time stays
/// out of its clock; otherwise, once a step is due on its 12 ms clock (more
/// than four steps late drops the backlog), it drains the frame's packets,
/// steps, and sends the step's packet stamped with its tick and time. Its
/// first step comes a step after the first packet that lets it go, as the
/// original's simulation starts paused. Exarust's debug keys: a speed
/// scales its clock, and single steps asked for while held run at once,
/// each moving the clock on by a step.
fn run_free(
    mut sub_app: SubApp,
    inbox: Receiver<FreeRequest>,
    outbox: Sender<FreeReply>,
    epoch: std::time::Instant,
    snapshots: SnapshotBuffer<SimSnapshot>,
) {
    use std::sync::mpsc::RecvTimeoutError;
    use std::time::{Duration, Instant};
    let mut pending: Vec<FreeRequest> = Vec::new();
    let mut held = true;
    let mut singles = 0u32;
    let mut acks = 0u32;
    let mut clock = FreeClock {
        ms: 0.0,
        last: epoch,
        speed: 1.0,
        held_ms: 0.0,
    };
    let mut driver: Option<StepDriver> = None;
    let mut dropped_ms = 0u64;
    let step_ms = STEP_MS as u32;
    // A packet's hold, speed and single steps take effect when it arrives
    // (the clock is read up to then with the old ones).
    let take = |r: FreeRequest,
                pending: &mut Vec<FreeRequest>,
                held: &mut bool,
                (singles, acks): (&mut u32, &mut u32),
                clock: &mut FreeClock| {
        clock.advance(Instant::now(), *held);
        *held = r.paused;
        *singles += r.steps;
        *acks += u32::from(r.ack);
        if r.speed > 0.0 && r.speed.is_finite() {
            clock.speed = r.speed;
        }
        pending.push(r);
    };
    loop {
        loop {
            match inbox.try_recv() {
                Ok(r) => take(
                    r,
                    &mut pending,
                    &mut held,
                    (&mut singles, &mut acks),
                    &mut clock,
                ),
                Err(std::sync::mpsc::TryRecvError::Empty) => break,
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    crate::exit_trace::stage("simulation thread: the frame closed the channel");
                    return;
                }
            }
        }
        // A hold the frame waits for: answered between steps, after every
        // step taken before it was sent.
        while acks > 0 {
            acks -= 1;
            if outbox.send(FreeReply::Held).is_err() {
                return;
            }
        }
        clock.advance(Instant::now(), held);
        let single = held && singles > 0;
        let wait = if held && !single {
            Duration::from_millis(u64::from(PAUSED_SLEEP_MS))
        } else {
            if single {
                // A single step: the clock moves on by the step, so the
                // step is due now and the next one a step later.
                singles -= 1;
                clock.ms = driver.map_or(clock.ms, |d| {
                    clock.ms.max(f64::from(
                        d.time_ms.wrapping_add(step_ms).wrapping_add(d.base_ms),
                    ))
                });
            }
            let reading = clock.ms as u32;
            let d = driver.get_or_insert_with(|| {
                // A single step before any other is due at once.
                StepDriver::new(
                    step_ms,
                    if single {
                        reading.wrapping_sub(step_ms)
                    } else {
                        reading
                    },
                )
            });
            match d.poll(reading) {
                Poll::Step {
                    dropped_ms: dropped,
                } => {
                    dropped_ms += u64::from(dropped);
                    if dropped > 0 {
                        debug!("simulation: {dropped} ms of backlog dropped");
                    }
                    let requests = std::mem::take(&mut pending);
                    let stepped = free_step(sub_app.world_mut(), requests, &snapshots);
                    d.stepped();
                    let stamp = FreeStamp {
                        tick: u64::from(d.ticks),
                        time_ms: d.time_ms,
                        base_ms: d.base_ms,
                        clock_ms: clock.ms,
                        wall_ms: (clock.last - epoch).as_secs_f64() * 1000.0,
                        speed: if held { 0.0 } else { clock.speed },
                        step_ms,
                    };
                    if u64::from(d.ticks).is_multiple_of(LOG_EVERY) {
                        info!(
                            "simulation free: {} steps, {dropped_ms} ms of backlog dropped, {:.0} ms paused",
                            d.ticks, clock.held_ms
                        );
                    }
                    if outbox
                        .send(FreeReply::Stepped(Box::new(FreeStepped { stepped, stamp })))
                        .is_err()
                    {
                        return;
                    }
                    continue;
                }
                // The original waits a millisecond short of the step on a
                // waitable timer; below a millisecond, a short sleep. The
                // wait is in clock time, the sleep in real time.
                Poll::Wait { ms: 0 } => Duration::from_micros(200),
                Poll::Wait { ms } => {
                    Duration::from_secs_f64(f64::from(ms) / 1000.0 / f64::from(clock.speed))
                        .min(Duration::from_millis(100))
                }
            }
        };
        match inbox.recv_timeout(wait) {
            Ok(r) => take(
                r,
                &mut pending,
                &mut held,
                (&mut singles, &mut acks),
                &mut clock,
            ),
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => {
                crate::exit_trace::stage("simulation thread: the frame closed the channel");
                return;
            }
        }
    }
}

enum Runner {
    Inline(Box<SubApp>),
    Thread {
        requests: Sender<StepRequest>,
        replies: Receiver<Stepped>,
        thread: Option<JoinHandle<()>>,
    },
    Free {
        requests: Sender<FreeRequest>,
        replies: Receiver<FreeReply>,
        thread: Option<JoinHandle<()>>,
    },
}

/// A frame's packet to the free-running simulation.
struct FreeRequest {
    request: StepRequest,
    /// The frame holds the simulation (the loading screen, a menu, a pause):
    /// the original's pause counter, raised.
    paused: bool,
    /// Exarust's debug speed (the frame's game-clock speed) and the single
    /// steps asked for since the last packet (the step key while paused).
    speed: f32,
    steps: u32,
    /// The frame waits for the simulation to answer that it is held.
    ack: bool,
}

/// What the free-running simulation sends the frame.
enum FreeReply {
    Stepped(Box<FreeStepped>),
    /// The answer to a hold the frame waits for: no step comes after it
    /// until the frame lets go.
    Held,
}

/// One free step's packet to the frame, stamped with its clock.
struct FreeStepped {
    stepped: Stepped,
    stamp: FreeStamp,
}

/// The free-running simulation's clock after a step: what the frame reads
/// to draw between the last two steps.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct FreeStamp {
    /// The step's tick and time (ms of simulation time).
    pub tick: u64,
    pub time_ms: u32,
    /// The clock reading the simulation's time counts from (ms; moved on
    /// when a backlog was dropped).
    pub base_ms: u32,
    /// The simulation's clock (ms, the held time left out) when the step
    /// was stamped, the real time since the shell started then, and the
    /// rate the clock runs at from there (0 while held).
    pub clock_ms: f64,
    pub wall_ms: f64,
    pub speed: f32,
    /// The step (ms).
    pub step_ms: u32,
}

impl FreeStamp {
    /// The original's render fraction: 1 + (frame clock − published
    /// time) / step, at most 2 and at least −2. `wall_ms` is the real
    /// time since the shell's start; the simulation's clock is carried on
    /// from the stamp at its rate. 1 draws the step before the last, 2 the
    /// last ([`overstep_of`]).
    pub fn fraction(&self, wall_ms: f64) -> f32 {
        let clock = self.clock_ms + (wall_ms - self.wall_ms) * f64::from(self.speed);
        let now = (clock as u64).wrapping_sub(u64::from(self.base_ms)) as u32;
        let step = f64::from(self.step_ms.max(1));
        let f = if now > self.time_ms {
            (f64::from(now - self.time_ms) / step + 1.0).min(2.0)
        } else {
            1.0 - f64::from(self.time_ms - now) / step
        };
        (f as f32).max(-2.0)
    }
}

/// The thread ends with the shell, its world dropped before the frame's
/// goes on (the simulation's world holds what the bridge moved there, such
/// as the trace files, which flush when dropped).
impl Drop for Runner {
    fn drop(&mut self) {
        crate::exit_trace::stage("sim shell: dropping, the channel closes");
        // Closing the channel ends the thread's loop.
        let thread = match self {
            Runner::Thread {
                requests, thread, ..
            } => {
                *requests = channel().0;
                thread.take()
            }
            Runner::Free {
                requests, thread, ..
            } => {
                *requests = channel().0;
                thread.take()
            }
            Runner::Inline(_) => None,
        };
        if let Some(thread) = thread {
            let _ = thread.join();
            crate::exit_trace::stage("sim shell: the simulation thread joined");
        }
    }
}

/// The simulation's world and where it runs (a resource of the frame's
/// world).
#[derive(Resource)]
pub struct SimShell {
    runner: Mutex<Runner>,
    snapshots: SnapshotBuffer<SimSnapshot>,
    mode: SimMode,
    /// The clock the free-running simulation and the frame both read.
    epoch: std::time::Instant,
}

impl SimShell {
    /// A shell around `sub_app`, run in `mode`.
    pub fn new(sub_app: SubApp, mode: SimMode) -> Self {
        let snapshots = SnapshotBuffer::default();
        let epoch = std::time::Instant::now();
        let runner = match mode {
            SimMode::Inline => Runner::Inline(Box::new(sub_app)),
            SimMode::Free => {
                let (requests, inbox) = channel::<FreeRequest>();
                let (outbox, replies) = channel();
                let published = snapshots.clone();
                let thread = std::thread::Builder::new()
                    .name("exa-sim".into())
                    .spawn(move || {
                        // The original's simulation thread enters the
                        // engine's floating-point mode first.
                        crate::float_mode::enter_logged("exa-sim");
                        run_free(sub_app, inbox, outbox, epoch, published);
                        crate::exit_trace::stage("simulation thread: its world dropped");
                    })
                    .expect("the simulation thread starts");
                Runner::Free {
                    requests,
                    replies,
                    thread: Some(thread),
                }
            }
            SimMode::Thread => {
                let (requests, inbox) = channel::<StepRequest>();
                let (outbox, replies) = channel();
                let published = snapshots.clone();
                let thread = std::thread::Builder::new()
                    .name("exa-sim".into())
                    .spawn(move || {
                        crate::float_mode::enter_logged("exa-sim");
                        let mut sub_app = sub_app;
                        // The frame drops its sender when it closes.
                        while let Ok(request) = inbox.recv() {
                            let stepped = step_world(sub_app.world_mut(), request, &published);
                            if outbox.send(stepped).is_err() {
                                break;
                            }
                        }
                    })
                    .expect("the simulation thread starts");
                Runner::Thread {
                    requests,
                    replies,
                    thread: Some(thread),
                }
            }
        };
        Self {
            runner: Mutex::new(runner),
            snapshots,
            mode,
            epoch,
        }
    }

    /// Milliseconds since the shell started (the real clock the free
    /// simulation's clock follows).
    pub fn clock_ms(&self) -> f64 {
        self.epoch.elapsed().as_secs_f64() * 1000.0
    }

    /// Free mode: sends the frame's packet without waiting.
    fn send_free(&self, request: FreeRequest) {
        let runner = self.runner.lock().unwrap_or_else(|e| e.into_inner());
        if let Runner::Free { requests, .. } = &*runner {
            // A closed thread (it panicked) has logged why.
            let _ = requests.send(request);
        }
    }

    /// Free mode: every step packet that arrived, in order.
    fn take_free(&self) -> Vec<FreeStepped> {
        let runner = self.runner.lock().unwrap_or_else(|e| e.into_inner());
        match &*runner {
            Runner::Free { replies, .. } => replies
                .try_iter()
                .filter_map(|r| match r {
                    FreeReply::Stepped(s) => Some(*s),
                    FreeReply::Held => None,
                })
                .collect(),
            _ => Vec::new(),
        }
    }

    /// Free mode: the step packets that come before the simulation's
    /// answer to a hold, in order (waiting for the answer, at most a few
    /// seconds).
    fn take_until_held(&self) -> Vec<FreeStepped> {
        let runner = self.runner.lock().unwrap_or_else(|e| e.into_inner());
        let Runner::Free { replies, .. } = &*runner else {
            return Vec::new();
        };
        let mut arrived = Vec::new();
        loop {
            match replies.recv_timeout(std::time::Duration::from_secs(10)) {
                Ok(FreeReply::Stepped(s)) => arrived.push(*s),
                Ok(FreeReply::Held) => return arrived,
                Err(_) => {
                    error!("simulation: no answer to the frame's hold");
                    return arrived;
                }
            }
        }
    }

    pub fn mode(&self) -> SimMode {
        self.mode
    }

    /// Runs one step and waits for it.
    fn step(&self, request: StepRequest) -> Stepped {
        let mut runner = self.runner.lock().unwrap_or_else(|e| e.into_inner());
        match &mut *runner {
            Runner::Inline(sub_app) => step_world(sub_app.world_mut(), request, &self.snapshots),
            Runner::Thread {
                requests, replies, ..
            } => {
                requests
                    .send(request)
                    .expect("the simulation thread is running");
                replies.recv().expect("the simulation thread stepped")
            }
            Runner::Free { .. } => panic!("a free-running simulation is not stepped by the frame"),
        }
    }

    /// The simulation's world, when it runs inline (tests and checks).
    pub fn with_world<R>(&self, f: impl FnOnce(&World) -> R) -> Option<R> {
        let runner = self.runner.lock().unwrap_or_else(|e| e.into_inner());
        match &*runner {
            Runner::Inline(sub_app) => Some(f(sub_app.world())),
            Runner::Thread { .. } | Runner::Free { .. } => None,
        }
    }
}

/// Steps counted by the frame and mismatches found against the snapshots
/// (a lockstep check: they must stay equal).
#[derive(Resource, Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct LockstepCheck {
    pub steps: u64,
    pub mismatches: u64,
}

/// Runs the simulation's world in lockstep with the frame's fixed step.
pub struct SimShellPlugin {
    pub mode: SimMode,
}

impl Plugin for SimShellPlugin {
    fn build(&self, app: &mut App) {
        // Bevy's own types are bridged whether the bridge is on or not, so
        // the boundary map's coverage counts them.
        crate::sim_bridge::bridge_bevy_types(app);
        let reserved = reserve_ids(app.world_mut(), POOL_BATCH);
        app.insert_resource(SimShell::new(new_sim_app(&reserved), self.mode))
            .init_resource::<SimInbox>()
            .init_resource::<SimIds>()
            .init_resource::<SimLinks>()
            .init_resource::<Snapshots<SimSnapshot>>()
            .init_resource::<LockstepCheck>()
            .init_resource::<SimPause>()
            .init_schedule(SimLockstep)
            .add_systems(SimLockstep, step_lockstep);
        if self.mode == SimMode::Free {
            // The frame's fixed loop no longer steps anything: the
            // simulation keeps its own clock; the frame exchanges packets
            // once a frame, where the loop ran.
            app.init_resource::<FreeLink>()
                .configure_sets(
                    RunFixedMainLoop,
                    (
                        RunFixedMainLoopSystems::FixedMainLoop.run_if(|| false),
                        SimExchange.in_set(RunFixedMainLoopSystems::AfterFixedMainLoop),
                    ),
                )
                .add_systems(RunFixedMainLoop, exchange_free.in_set(SimExchange));
        } else {
            app.configure_sets(
                RunFixedMainLoop,
                SimExchange.in_set(RunFixedMainLoopSystems::FixedMainLoop),
            );
        }
        add_frame_start(app);
        if orphan_trace() {
            trace_orphans(app.world_mut(), "frame");
        }
        app.world_mut()
            .resource_mut::<FixedMainScheduleOrder>()
            // First of all: the bridge moves the fixed step's schedules in
            // it, before any of them has run here.
            .labels
            .insert(0, SimLockstep.intern());
    }
}

/// Where the frame meets the simulation each frame: the fixed loop in
/// lockstep, the packet exchange when the simulation runs free. Frame
/// systems that read the step's results before the frame's own work (the
/// drawn controller, the cloth meshes) run after it.
#[derive(bevy::ecs::schedule::SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct SimExchange;

/// The frame's hold on the simulation: the original's pause counter, which
/// its screens raise (the loading screen, the menus, the creator, the end
/// of a session; context/re/sim-thread-input.md). `held` is what the game's
/// screens ask this frame; `raised` counts paired raises. A free-running
/// simulation reads it from each of the frame's packets and sleeps while it
/// is up; in lockstep the frame's own fixed loop holds instead.
#[derive(Resource, Debug, Default, Clone, Copy, PartialEq)]
pub struct SimPause {
    held: bool,
    raised: u32,
    /// Exarust's debug controls: the game clock's speed (0 for unset: 1)
    /// and the single steps asked for while paused, not yet sent.
    speed: f32,
    steps: u32,
    /// Held through a level's teardown and the load after it.
    teardown: bool,
}

impl SimPause {
    /// Whether a teardown's hold is up.
    pub fn in_teardown(&self) -> bool {
        self.teardown
    }

    /// Lets go of a teardown's hold (the next level is in).
    pub fn release_teardown(&mut self) {
        self.teardown = false;
    }

    /// Sets this frame's hold from the screens that show.
    pub fn hold(&mut self, held: bool) {
        self.held = held;
    }

    /// A paired raise (a screen that opens), lowered by [`Self::lower`].
    pub fn raise(&mut self) {
        self.raised += 1;
    }

    pub fn lower(&mut self) {
        self.raised = self.raised.saturating_sub(1);
    }

    pub fn paused(&self) -> bool {
        self.held || self.raised > 0 || self.teardown
    }

    /// The debug speed the simulation's clock runs at.
    pub fn set_speed(&mut self, speed: f32) {
        self.speed = speed;
    }

    pub fn speed(&self) -> f32 {
        if self.speed > 0.0 { self.speed } else { 1.0 }
    }

    /// One more single step while paused (the step key).
    pub fn step(&mut self) {
        self.steps += 1;
    }

    fn take_steps(&mut self) -> u32 {
        std::mem::take(&mut self.steps)
    }
}

/// The frame's side of a free-running simulation.
#[derive(Resource, Debug, Default)]
pub struct FreeLink {
    /// The last step packet applied.
    pub stamp: Option<FreeStamp>,
    /// The render fraction the frame draws at ([`FreeStamp::fraction`]).
    pub fraction: f32,
    /// Step packets applied, and frames that exchanged.
    pub steps: u64,
    pub frames: u64,
    /// The most step packets one frame applied.
    pub most_in_a_frame: usize,
    /// Step packets whose tick was not the next one.
    pub gaps: u64,
    /// The hold the frame last told the simulation (it starts held).
    pub paused: bool,
    /// Single steps sent whose packets have not arrived yet.
    pub awaiting: u32,
}

/// The frame's overstep (in steps) for the original's render fraction.
/// The original keeps three history entries per object (the last step's,
/// the one before, and the one before that) and draws between the two
/// older ones at the fraction (1 the step before the last, 2 that pair
/// carried on a step, which is about the last step): one step behind the
/// clock, as Exarust's lockstep frame draws between the last two ticks by
/// the time since the last. Exarust's bodies keep the last two entries, so
/// the same moment is drawn at fraction − 1 between them; a fraction below
/// 1 (a frame that ran ahead of the step's time) holds at the step before
/// the last.
pub fn overstep_of(fraction: f32) -> f32 {
    (fraction - 1.0).clamp(0.0, 1.0)
}

/// Free mode, once a frame: sends the frame's packet (its changes, its
/// posted calls, its hold) without waiting, applies every step packet that
/// arrived, in order, with its events, and sets the render fraction. The
/// fraction replaces the fixed loop's overstep: the frame's `Time<Fixed>`
/// carries it ([`overstep_of`]), so every drawing that reads
/// `overstep_fraction` draws the moment the original's frame draws.
pub fn exchange_free(world: &mut World) {
    let (paused, speed, steps) = world
        .get_resource_mut::<SimPause>()
        .map_or((false, 1.0, 0), |mut p| {
            (p.paused(), p.speed(), p.take_steps())
        });
    // Held, the frame keeps its changes and posts until it lets go (as the
    // lockstep frame does while no step runs), and tells the simulation
    // once; a single step asked for while held exchanges as an unheld
    // frame does until its packet arrived.
    let (was_paused, awaiting) = {
        let mut link = world.resource_mut::<FreeLink>();
        link.awaiting += steps;
        (std::mem::replace(&mut link.paused, paused), link.awaiting)
    };
    // Entering a hold, the frame waits for the simulation to answer it and
    // applies the steps taken before it, so none arrives after the frame
    // went on (a level's teardown, a load) to name what is gone.
    let entering = paused && !was_paused;
    let exchanging = !paused || awaiting > 0 || entering;
    let request = if exchanging {
        let bridge = frame_gather(world);
        let commands = world.resource_mut::<SimInbox>().take();
        Some(StepRequest { commands, bridge })
    } else {
        (!was_paused).then(|| StepRequest {
            commands: Vec::new(),
            bridge: None,
        })
    };
    // Held, the step packets that still arrive wait too: applying one
    // would settle the frame's changes before they were gathered.
    let arrived = {
        let shell = world.resource::<SimShell>();
        if let Some(request) = request {
            shell.send_free(FreeRequest {
                request,
                paused,
                speed,
                steps,
                ack: entering,
            });
        }
        if entering {
            shell.take_until_held()
        } else if exchanging {
            shell.take_free()
        } else {
            Vec::new()
        }
    };
    let count = arrived.len();
    if count == 0 && exchanging {
        crate::sim_bridge::frame_settle(world);
    }
    {
        let mut link = world.resource_mut::<FreeLink>();
        link.awaiting = link.awaiting.saturating_sub(count as u32);
    }
    for FreeStepped { stepped, stamp } in arrived {
        if let Some(out) = stepped.bridge {
            frame_apply(world, out);
            crate::sim_digest::write_frame_digest(world);
        }
        if let Some(mut bridge) = world.get_resource_mut::<FrameBridge>() {
            bridge.applied_step = stamp.tick;
        }
        for event in stepped.events {
            event(world);
        }
        let mut link = world.resource_mut::<FreeLink>();
        if link.stamp.is_some_and(|s| s.tick + 1 != stamp.tick) {
            link.gaps += 1;
        }
        link.stamp = Some(stamp);
        link.steps += 1;
        let steps = link.steps;
        if steps.is_multiple_of(LOG_EVERY) {
            let (frames, most, gaps) = (link.frames, link.most_in_a_frame, link.gaps);
            link.most_in_a_frame = 0;
            info!(
                "simulation free (frame): {steps} steps applied over {frames} frames, at most {most} in a frame, {gaps} gaps"
            );
            if let Some(cost) = world.get_resource::<FrameBridge>().map(|b| b.cost) {
                info!(
                    "simulation bridge: {:.0} us a step copying ({} values in, {} out over {} steps; \
                     gather in {} us, apply in {} us, gather out {} us, apply out {} us)",
                    cost.us_per_step(),
                    cost.values_in,
                    cost.values_out,
                    cost.steps,
                    cost.gather_in_us,
                    cost.apply_in_us,
                    cost.gather_out_us,
                    cost.apply_out_us
                );
            }
        }
    }
    if let Some(snapshot) = world.resource::<SimShell>().snapshots.take() {
        world
            .resource_mut::<Snapshots<SimSnapshot>>()
            .push(snapshot);
    }
    let clock = world.resource::<SimShell>().clock_ms();
    let mut link = world.resource_mut::<FreeLink>();
    link.frames += 1;
    link.most_in_a_frame = link.most_in_a_frame.max(count);
    // Paused, the original leaves its fraction where it was; a single
    // step is drawn as it ended.
    if paused && count == 0 {
        return;
    }
    let Some(stamp) = link.stamp else {
        return;
    };
    let fraction = if paused { 2.0 } else { stamp.fraction(clock) };
    link.fraction = fraction;
    if let Some(mut fixed) = world.get_resource_mut::<Time<Fixed>>() {
        let left = fixed.overstep();
        fixed.discard_overstep(left);
        let step = fixed.timestep();
        fixed.accumulate_overstep(step.mul_f32(overstep_of(fraction)));
    }
}

/// Holds a free-running simulation now and waits until it is held, its
/// queued steps applied (the original's level change raises the pause
/// counter before it drops the scene): run before a level's teardown. The
/// hold lasts until the game lets go of it ([`SimPause::release_teardown`]).
/// In lockstep nothing is in flight: no-op.
pub fn hold_and_drain(world: &mut World) {
    if !world.contains_resource::<FreeLink>() {
        return;
    }
    if let Some(mut pause) = world.get_resource_mut::<SimPause>() {
        pause.teardown = true;
    }
    exchange_free(world);
}

/// One simulation step for this pass of the fixed step: the inbox's
/// commands go over, the frame waits, then runs the step's events and keeps
/// its snapshot.
pub fn step_lockstep(world: &mut World) {
    if world.resource::<SimShell>().mode == SimMode::Free {
        return;
    }
    let bridge = frame_gather(world);
    let commands = world.resource_mut::<SimInbox>().take();
    let stepped = world
        .resource::<SimShell>()
        .step(StepRequest { commands, bridge });
    if let Some(out) = stepped.bridge {
        frame_apply(world, out);
        crate::sim_digest::write_frame_digest(world);
    }
    for event in stepped.events {
        event(world);
    }
    let snapshot = world.resource::<SimShell>().snapshots.take();
    let mut check = world.resource_mut::<LockstepCheck>();
    check.steps += 1;
    let steps = check.steps;
    if snapshot.as_ref().is_none_or(|s| s.tick != steps) {
        check.mismatches += 1;
        error!(
            "simulation lockstep: step {steps} published {:?}",
            snapshot.as_ref().map(|s| s.tick)
        );
    }
    let mismatches = check.mismatches;
    if steps.is_multiple_of(LOG_EVERY) {
        let mode = world.resource::<SimShell>().mode;
        info!("simulation lockstep ({mode:?}): {steps} steps, {mismatches} mismatches");
        if let Some(cost) = world.get_resource::<FrameBridge>().map(|b| b.cost) {
            info!(
                "simulation bridge: {:.0} us a step copying ({} values in, {} out over {} steps; \
                 gather in {} us, apply in {} us, gather out {} us, apply out {} us)",
                cost.us_per_step(),
                cost.values_in,
                cost.values_out,
                cost.steps,
                cost.gather_in_us,
                cost.apply_in_us,
                cost.gather_out_us,
                cost.apply_out_us
            );
        }
    }
    if let Some(snapshot) = snapshot {
        world
            .resource_mut::<Snapshots<SimSnapshot>>()
            .push(snapshot);
    }
}

/// The lockstep check is logged every this many steps.
pub const LOG_EVERY: u64 = 1000;

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::app::FixedMain;

    #[derive(Component, Debug, PartialEq)]
    struct Mass(f32);

    #[derive(Resource, Default)]
    struct Heard(Vec<u64>);

    fn app(mode: SimMode) -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_plugins(SimShellPlugin { mode })
            .init_resource::<Heard>();
        app
    }

    fn run_fixed(app: &mut App, passes: usize) {
        for _ in 0..passes {
            app.world_mut().run_schedule(FixedMain);
        }
    }

    #[test]
    fn each_pass_of_the_fixed_step_runs_one_simulation_step() {
        for mode in [SimMode::Inline, SimMode::Thread] {
            let mut app = app(mode);
            run_fixed(&mut app, 3);
            let w = app.world();
            assert_eq!(
                *w.resource::<LockstepCheck>(),
                LockstepCheck {
                    steps: 3,
                    mismatches: 0
                }
            );
            let snaps = w.resource::<Snapshots<SimSnapshot>>();
            assert_eq!(snaps.current.as_ref().map(|s| s.tick), Some(3));
            assert_eq!(
                snaps.previous.as_ref().map(|s| s.time_ms),
                Some(2 * STEP_MS)
            );
        }
    }

    #[test]
    fn commands_reach_the_next_step_in_post_order() {
        for mode in [SimMode::Inline, SimMode::Thread] {
            let mut app = app(mode);
            let world = app.world_mut();
            for k in 0..3u64 {
                world.resource_mut::<SimInbox>().post(move |w: &mut World| {
                    let tick = w.resource::<SimClock>().tick;
                    w.resource_mut::<SimOutbox>().send(move |f: &mut World| {
                        f.resource_mut::<Heard>().0.push(k * 100 + tick);
                    });
                });
            }
            run_fixed(&mut app, 2);
            // Run before the first step's schedule (tick 0), heard in order,
            // once.
            assert_eq!(app.world().resource::<Heard>().0, vec![0, 100, 200]);
        }
    }

    #[test]
    fn linked_objects_are_named_alike_in_both_worlds() {
        let mut app = app(SimMode::Inline);
        let world = app.world_mut();
        let frame_entity = world.spawn_empty().id();
        let id = world.resource_mut::<SimIds>().issue();
        world.resource_scope(|world, mut inbox: Mut<SimInbox>| {
            let mut links = world.resource_mut::<SimLinks>();
            inbox.spawn_linked(id, frame_entity, &mut links, |e| {
                e.insert(Mass(70.0));
            });
        });
        run_fixed(&mut app, 1);
        let world = app.world();
        assert_eq!(world.resource::<SimLinks>().entity(id), Some(frame_entity));
        let mass = world
            .resource::<SimShell>()
            .with_world(|sim| {
                let entity = sim.resource::<SimLinks>().entity(id).unwrap();
                sim.get::<Mass>(entity).map(|m| m.0)
            })
            .flatten();
        assert_eq!(mass, Some(70.0));
        assert_eq!(
            world
                .resource::<Snapshots<SimSnapshot>>()
                .current
                .as_ref()
                .map(|s| s.linked),
            Some(1)
        );

        let world = app.world_mut();
        world.resource_scope(|world, mut inbox: Mut<SimInbox>| {
            inbox.despawn_linked(id, &mut world.resource_mut::<SimLinks>());
        });
        run_fixed(&mut app, 1);
        let world = app.world();
        assert_eq!(world.resource::<SimLinks>().entity(id), None);
        let left = world
            .resource::<SimShell>()
            .with_world(|sim| sim.resource::<SimLinks>().len());
        assert_eq!(left, Some(0));
    }

    /// Runs the frame for `ms` milliseconds of real time.
    fn run_frames_for(app: &mut App, ms: u64) {
        let until = std::time::Instant::now() + std::time::Duration::from_millis(ms);
        while std::time::Instant::now() < until {
            app.update();
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
    }

    #[test]
    fn a_free_simulation_steps_on_its_own_clock_and_sleeps_while_held() {
        let mut app = app(SimMode::Free);
        app.world_mut().resource_mut::<SimPause>().hold(true);
        run_frames_for(&mut app, 120);
        assert_eq!(app.world().resource::<FreeLink>().steps, 0, "held: no step");
        app.world_mut().resource_mut::<SimPause>().hold(false);
        run_frames_for(&mut app, 240);
        // Let the last packets arrive.
        app.world_mut().resource_mut::<SimPause>().hold(true);
        run_frames_for(&mut app, 40);
        let link = app.world().resource::<FreeLink>();
        // 240 ms at 12 ms a step: 20, less the start and the scheduler's
        // noise; never more than the real clock allows.
        assert!(
            (8..=24).contains(&link.steps),
            "{} steps in 240 ms",
            link.steps
        );
        assert_eq!(link.gaps, 0);
        let stamp = link.stamp.unwrap();
        assert_eq!(stamp.tick, link.steps);
        assert_eq!(stamp.time_ms as u64, stamp.tick * STEP_MS);
        // The paused time stays out of the clock.
        assert!(stamp.wall_ms - stamp.clock_ms >= 100.0, "{stamp:?}");
    }

    #[test]
    fn a_held_free_simulation_takes_single_steps_and_runs_at_its_speed() {
        let mut app = app(SimMode::Free);
        app.world_mut().resource_mut::<SimPause>().hold(true);
        run_frames_for(&mut app, 60);
        for _ in 0..3 {
            app.world_mut().resource_mut::<SimPause>().step();
            run_frames_for(&mut app, 30);
        }
        run_frames_for(&mut app, 60);
        let link = app.world().resource::<FreeLink>();
        assert_eq!(link.steps, 3, "three single steps while held");
        assert_eq!(link.awaiting, 0);
        // Released at a quarter speed: about a step every 48 ms.
        let world = app.world_mut();
        world.resource_mut::<SimPause>().set_speed(0.25);
        world.resource_mut::<SimPause>().hold(false);
        run_frames_for(&mut app, 480);
        let steps = app.world().resource::<FreeLink>().steps - 3;
        assert!(
            (4..=12).contains(&steps),
            "{steps} steps in 480 ms at x0.25"
        );
    }

    #[test]
    fn a_free_simulation_takes_the_frames_commands_in_post_order() {
        let mut app = app(SimMode::Free);
        let world = app.world_mut();
        for k in 0..3u64 {
            world.resource_mut::<SimInbox>().post(move |w: &mut World| {
                w.resource_mut::<SimOutbox>().send(move |f: &mut World| {
                    f.resource_mut::<Heard>().0.push(k);
                });
            });
        }
        app.update();
        for k in 3..5u64 {
            app.world_mut()
                .resource_mut::<SimInbox>()
                .post(move |w: &mut World| {
                    w.resource_mut::<SimOutbox>().send(move |f: &mut World| {
                        f.resource_mut::<Heard>().0.push(k);
                    });
                });
        }
        run_frames_for(&mut app, 100);
        assert_eq!(app.world().resource::<Heard>().0, vec![0, 1, 2, 3, 4]);
    }

    #[test]
    fn the_overstep_draws_the_originals_moment() {
        // Fraction 1: the step before the last; 2: the last.
        assert_eq!(overstep_of(1.0), 0.0);
        assert_eq!(overstep_of(1.5), 0.5);
        assert_eq!(overstep_of(2.0), 1.0);
        assert_eq!(overstep_of(-2.0), 0.0);
    }

    #[test]
    fn the_render_fraction_is_the_originals() {
        let stamp = FreeStamp {
            tick: 10,
            time_ms: 120,
            base_ms: 1000,
            clock_ms: 1120.0,
            wall_ms: 1620.0,
            speed: 1.0,
            step_ms: 12,
        };
        // Stamped at the step's time: the clock carried on from there.
        assert_eq!(stamp.fraction(1620.0), 1.0);
        assert_eq!(stamp.fraction(1626.0), 1.5);
        // At most two, at least minus two.
        assert_eq!(stamp.fraction(1700.0), 2.0);
        assert_eq!(stamp.fraction(1614.0), 0.5);
        assert_eq!(stamp.fraction(1500.0), -2.0);
        // At half speed the clock runs half as fast from the stamp.
        let slow = FreeStamp {
            speed: 0.5,
            ..stamp
        };
        assert_eq!(slow.fraction(1632.0), 1.5);
    }

    #[test]
    fn the_buffer_keeps_only_the_newest_snapshot() {
        let buffer = SnapshotBuffer::default();
        buffer.publish(1);
        buffer.publish(2);
        assert_eq!(buffer.take(), Some(2));
        assert_eq!(buffer.take(), None);
        let mut snaps = Snapshots::default();
        snaps.push(1);
        snaps.push(2);
        assert_eq!((snaps.previous, snaps.current), (Some(1), Some(2)));
    }

    #[test]
    fn ids_are_issued_in_order_and_links_replace() {
        let mut ids = SimIds::default();
        let (a, b) = (ids.issue(), ids.issue());
        assert!(a < b);
        let mut links = SimLinks::default();
        let e = World::new().spawn_empty().id();
        links.link(a, e);
        links.link(b, e);
        assert_eq!(links.entity(a), None);
        assert_eq!(links.id(e), Some(b));
        assert_eq!(links.len(), 1);
    }
}
