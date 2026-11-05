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

use std::collections::HashMap;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

use bevy::app::{FixedMainScheduleOrder, SubApp};
use bevy::ecs::schedule::ScheduleLabel;
use bevy::prelude::*;

use crate::STEP_HZ;
use crate::sim_bridge::{
    BridgeIn, BridgeOut, FrameBridge, POOL_BATCH, frame_apply, frame_gather, reserve_ids,
    run_fixed_order, sim_apply, sim_gather,
};

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
/// [`SimPreStep`]), then what ran in `PostUpdate` (the hit chain and its
/// readers), then the bodies' own frame start, which queues the blows the
/// others sent.
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

    pub fn len(&mut self) -> usize {
        queue(&mut self.queued).len()
    }

    pub fn is_empty(&mut self) -> bool {
        queue(&mut self.queued).is_empty()
    }
}

/// The schedule that runs the [`FixedCommands`], before `FixedFirst`.
#[derive(ScheduleLabel, Debug, Clone, PartialEq, Eq, Hash)]
pub struct FixedCommandsStep;

fn run_fixed_commands(world: &mut World) {
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
        .bridge_lend_opaque_resource::<FixedCommands>();
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
    sub
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
    let (mut coverage, mut apply_us) = (None, 0);
    if let Some(bridge_in) = request.bridge {
        (coverage, apply_us, _) = sim_apply(world, bridge_in);
    }
    for command in request.commands {
        command(world);
    }
    world.run_schedule(SimStep);
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
    Stepped {
        events: std::mem::take(queue(&mut world.resource_mut::<SimOutbox>().queued)),
        bridge,
    }
}

enum Runner {
    Inline(Box<SubApp>),
    Thread {
        requests: Sender<StepRequest>,
        replies: Receiver<Stepped>,
        thread: Option<JoinHandle<()>>,
    },
}

/// The thread ends with the shell, its world dropped before the frame's
/// goes on (the simulation's world holds what the bridge moved there, such
/// as the trace files, which flush when dropped).
impl Drop for Runner {
    fn drop(&mut self) {
        if let Runner::Thread {
            requests, thread, ..
        } = self
        {
            // Closing the channel ends the thread's loop.
            let (closed, _) = channel();
            *requests = closed;
            if let Some(thread) = thread.take() {
                let _ = thread.join();
            }
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
}

impl SimShell {
    /// A shell around `sub_app`, run in `mode`.
    pub fn new(sub_app: SubApp, mode: SimMode) -> Self {
        let snapshots = SnapshotBuffer::default();
        let runner = match mode {
            SimMode::Inline => Runner::Inline(Box::new(sub_app)),
            SimMode::Thread => {
                let (requests, inbox) = channel::<StepRequest>();
                let (outbox, replies) = channel();
                let published = snapshots.clone();
                let thread = std::thread::Builder::new()
                    .name("exa-sim".into())
                    .spawn(move || {
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
        }
    }

    /// The simulation's world, when it runs inline (tests and checks).
    pub fn with_world<R>(&self, f: impl FnOnce(&World) -> R) -> Option<R> {
        let runner = self.runner.lock().unwrap_or_else(|e| e.into_inner());
        match &*runner {
            Runner::Inline(sub_app) => Some(f(sub_app.world())),
            Runner::Thread { .. } => None,
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
            .init_schedule(SimLockstep)
            .add_systems(SimLockstep, step_lockstep);
        add_frame_start(app);
        app.world_mut()
            .resource_mut::<FixedMainScheduleOrder>()
            // First of all: the bridge moves the fixed step's schedules in
            // it, before any of them has run here.
            .labels
            .insert(0, SimLockstep.intern());
    }
}

/// One simulation step for this pass of the fixed step: the inbox's
/// commands go over, the frame waits, then runs the step's events and keeps
/// its snapshot.
pub fn step_lockstep(world: &mut World) {
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
