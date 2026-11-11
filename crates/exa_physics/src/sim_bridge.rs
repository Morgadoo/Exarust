//! The lockstep copy bridge: the fixed step's schedules run in the
//! simulation's world while the rest of the game still keeps its data in the
//! frame's world (OpenSpec `bridge-the-simulation-world`).
//!
//! Stage 3 of the simulation thread. The schedules the fixed step runs
//! (`FixedFirst` … `FixedLast` and the ones they run, such as the character
//! tick and the brain chain) are taken out of the frame's world whole, with
//! their sets and order, and run by the simulation's world in each lockstep
//! step. Each data type those systems touch is registered with the bridge
//! (`SimBridgeAppExt`), and every step:
//!
//! 1. the frame **gathers** what changed in its world since the last step
//!    (values whose change tick is newer, removals, despawns, messages);
//! 2. the simulation **applies** it before its commands and its schedules;
//! 3. after the schedules the simulation gathers its own changes the same
//!    way, and the frame applies them.
//!
//! A value is copied only when it changed, and each side's own writes from
//! an apply are not sent back, so `Changed`, `Added` and `RemovedComponents`
//! see on each side what they would see in one world.
//!
//! **Entity ids are the same in both worlds.** The fixed step's data is full
//! of `Entity` fields (parts, holders, targets, the hierarchy), and its
//! systems sort by entity, so the bridge keeps one id space instead of
//! mapping each field: the frame's world hands out every id. An entity the
//! frame spawns is spawned at the same id in the simulation; the simulation
//! spawns its own entities from a pool of ids the frame reserved for it
//! ([`IdPool`]), and they are spawned at the same ids in the frame. A despawn
//! on one side is mirrored without freeing the id, so each id stays in one
//! allocator's free list.
//!
//! Types are bridged in one of three ways:
//! - **copy** (`Clone`): the changed values are cloned across;
//! - **lend** (`Default`): the value itself moves to the simulation for the
//!   step and back after it (no clone, every step), leaving a default in its
//!   place meanwhile; for large or uncloneable data (bodies, the collision
//!   world);
//! - **forward** (messages): each side's new messages are written on the
//!   other.
//!
//! The bridge is temporary: later stages turn the frame's writers into
//! commands and the copies out into snapshots.

use std::any::{Any, TypeId, type_name};
use std::collections::{BTreeSet, HashSet};
use std::marker::PhantomData;
use std::time::Instant;

use bevy::app::FixedMainScheduleOrder;
use bevy::ecs::change_detection::Tick;
use bevy::ecs::component::{ComponentId, Mutable};
use bevy::ecs::entity::EntityHashSet;
use bevy::ecs::lifecycle::RemovedComponentEntity;
use bevy::ecs::message::{MessageCursor, MessageRegistry, ShouldUpdateMessages};
use bevy::ecs::query::{ComponentAccessKind, QueryState};
use bevy::ecs::relationship::Relationship;
use bevy::ecs::resource::IsResource;
use bevy::ecs::schedule::{InternedScheduleLabel, ScheduleLabel};
use bevy::prelude::*;

/// What one type's half hands the other side for one step.
pub type Packet = Box<dyn Any + Send>;

/// Ids the frame reserves for the simulation at a time.
pub const POOL_BATCH: usize = 1024;
/// The pool is topped up when fewer ids than this are left.
pub const POOL_LOW: usize = 256;

/// One bridged type, registered on the frame's app; it makes a half for each
/// world.
pub trait BridgeType: Send + Sync + 'static {
    fn type_id(&self) -> TypeId;
    /// The type's name as the world's component registry gives it.
    fn name(&self) -> &'static str {
        self.half().name()
    }
    fn half(&self) -> Box<dyn BridgeHalf>;
}

/// One world's side of a bridged type. The same code runs on both sides:
/// it gathers its world's changes and applies the other side's.
pub trait BridgeHalf: Send + Sync + 'static {
    fn name(&self) -> &'static str;
    /// Registers the type in `world`; returns the id systems access it by.
    fn init(&mut self, world: &mut World) -> ComponentId;
    /// Reads the removals `world` recorded since the last call (the frame
    /// calls this every frame: removals are kept for two updates only).
    fn drain_removed(&mut self, world: &World);
    /// This world's changes since the last apply (everything when `all`),
    /// with the entities they name and the entities whose removal was seen.
    fn gather(&mut self, world: &mut World, ctx: GatherCtx) -> Gathered;
    /// Applies the other side's changes.
    fn apply(&mut self, world: &mut World, packet: Packet, ctx: ApplyCtx) -> usize;
    /// Free-running simulation, simulation side, since the last call: the
    /// frame's values of data both sides write that came after the
    /// simulation had written the same value in a step the frame had not
    /// seen yet: dropped (a drawn copy, the simulation's newer value wins)
    /// and applied (the frame's write wins, as in lockstep).
    fn take_overtaken(&mut self) -> (u64, u64) {
        (0, 0)
    }
    /// Free mode, frame side, since the last call: values the frame only
    /// touched (still as the simulation sent them), not sent.
    fn take_untouched(&mut self) -> u64 {
        0
    }
    /// After an apply: forgets the removals and messages the apply itself
    /// made, and marks the tick changes are measured from.
    fn settle(&mut self, world: &mut World, since: Tick);
    /// The hierarchy and transforms (`ChildOf`, `Children`'s order,
    /// `Transform`, `GlobalTransform`, `Name`): on the frame's side only the
    /// entities the simulation holds ([`SimShared`]) are scanned.
    fn hierarchy(&self) -> bool {
        false
    }
    /// A resource's half: spawns the resource's entity in `world` (empty)
    /// unless it has one, so the entity takes its id when the bridge
    /// starts, in the order the types were registered, not in whichever
    /// step the value first comes across, which would move every id handed
    /// out after it ([`crate::resource_hold`]).
    fn hold(&self, _world: &mut World) {}
    /// A resource's half: registers the resource's type in `world` when the
    /// type is bridged (the frame's app, as its plugins build), so that the
    /// frame's resource hold at the start of the second update holds its
    /// entity as well. A resource only the fixed step's systems name is
    /// otherwise not registered in the frame's world by then, and its first
    /// insert there (the navigation grid's movers, when the grid's async
    /// build finishes) took an id among the level's or the characters'.
    fn register(&self, _world: &mut World) {}
    /// Frame side: the frame's world never holds a live value of this type
    /// behind the bridge (`free`: the simulation runs free), so a frame
    /// system that reads it sees nothing ([`audit`]).
    fn frame_absent(&self, _free: bool) -> bool {
        false
    }
}

/// Frame side: an entity the simulation's world holds, because a bridged
/// type other than the hierarchy and transforms crossed for it (either
/// way), or it is an ancestor of one. Only these entities' hierarchy and
/// transforms are scanned and sent: the frame's other entities (meshes,
/// bones, lights, the GUI) stay home.
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct SimShared;

/// Frame side, during a gather: the entities marked [`SimShared`] in it,
/// whose hierarchy and transforms go whole.
#[derive(Resource, Default)]
struct NewlyShared(EntityHashSet);

/// The hierarchy types narrowed to [`SimShared`] entities.
fn is_hierarchy(id: TypeId) -> bool {
    id == TypeId::of::<Transform>()
        || id == TypeId::of::<GlobalTransform>()
        || id == TypeId::of::<ChildOf>()
        || id == TypeId::of::<Name>()
}

/// Whether the frame's hierarchy scans are narrowed to [`SimShared`]
/// entities (a debug aid turns it off: `EXA_BRIDGE_WIDE=1`, or
/// `EXA_BRIDGE_ALL=1`).
fn narrowed() -> bool {
    static WIDE: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    !*WIDE.get_or_init(|| std::env::var("EXA_BRIDGE_WIDE").is_ok_and(|v| v == "1"))
        && !copy_everything()
}

/// Marks `entity` and its ancestors [`SimShared`]; those newly marked go
/// into `newly`.
fn mark_shared(world: &mut World, entity: Entity, newly: &mut EntityHashSet) {
    let mut at = entity;
    loop {
        let Ok(mut e) = world.get_entity_mut(at) else {
            return;
        };
        if e.contains::<SimShared>() {
            return;
        }
        e.insert(SimShared);
        newly.insert(at);
        match e.get::<ChildOf>() {
            Some(parent) => at = parent.parent(),
            None => return,
        }
    }
}

/// One half's gathered changes.
pub struct Gathered {
    pub packet: Packet,
    /// Entities the packet gives a value to (spawned on the other side when
    /// missing).
    pub touched: Vec<Entity>,
    /// Entities whose removal of this type was seen (despawned ones among
    /// them are despawned on the other side).
    pub removed: Vec<Entity>,
    pub values: usize,
}

impl Gathered {
    fn empty() -> Self {
        Self {
            packet: Box::new(()),
            touched: Vec::new(),
            removed: Vec::new(),
            values: 0,
        }
    }
}

/// What a gather needs to know.
#[derive(Debug, Clone, Copy)]
pub struct GatherCtx {
    /// Send every value (the frame's first gather).
    pub all: bool,
    /// The gathering world is the simulation's.
    pub sim: bool,
    /// The simulation runs free of the frame (`crate::sim_app::SimMode::Free`).
    pub free: bool,
    /// Simulation side: the step just taken. Frame side: the frame
    /// packet's number.
    pub step: u64,
}

/// What an apply needs to know besides the packet.
#[derive(Debug, Clone, Copy)]
pub struct ApplyCtx {
    /// The receiving world is the simulation's.
    pub sim: bool,
    /// The frame updated its messages since the last step: the
    /// simulation's messages are updated too (Bevy's double buffer).
    pub update_messages: bool,
    /// Free-running simulation, simulation side: the last step the frame
    /// had applied when it gathered this packet. A frame value of data both
    /// sides write is dropped when the simulation wrote it in a later step
    /// (the frame's write was made against an older state).
    pub based_on: Option<u64>,
    /// The simulation runs free of the frame.
    pub free: bool,
    /// Free mode, frame side: the last frame packet the simulation had
    /// applied before the step that sent this packet.
    pub seen_seq: Option<u64>,
}

// ---------------------------------------------------------------- removals

/// Reads one component's removals from a world.
struct Removals {
    cursor: Option<MessageCursor<RemovedComponentEntity>>,
    seen: Vec<Entity>,
}

impl Removals {
    fn new() -> Self {
        Self {
            cursor: None,
            seen: Vec::new(),
        }
    }

    fn drain(&mut self, world: &World, id: ComponentId) {
        let Some(messages) = world.removed_components().get(id) else {
            return;
        };
        let cursor = self.cursor.get_or_insert_with(|| messages.get_cursor());
        self.seen
            .extend(cursor.read(messages).map(|r| Entity::from(r.clone())));
    }

    fn skip(&mut self, world: &World, id: ComponentId) {
        self.drain(world, id);
        self.seen.clear();
    }

    fn take(&mut self) -> Vec<Entity> {
        std::mem::take(&mut self.seen)
    }
}

// ---------------------------------------------------------------- components

/// The entities whose `T` changed since a tick, with their values.
type ChangedQuery<T> = QueryState<(Entity, &'static T), (Changed<T>, Without<IsResource>)>;

/// Every entity's value of a bridged component (not resources).
type ValueQuery<D> = QueryState<(Entity, D), Without<IsResource>>;

/// A component copied by clone when it changed.
pub struct CopyComponent<T: Component + Clone> {
    assign: fn(&mut EntityWorldMut, T),
    refs: fn(&T) -> Option<Entity>,
    /// Sent from the frame only (the simulation owns it once it arrives).
    inbound: bool,
    /// The frame writes it to draw (transforms, the drawn controller): a
    /// free-running simulation's newer value wins over the frame's.
    drawn: bool,
    /// A hash of a value's `Debug` text (free mode: tells a frame write
    /// that left the value as the simulation sent it).
    hash: fn(&T) -> u64,
    text: fn(&T) -> String,
    /// A snapshot: sent from the simulation; from the frame only when the
    /// frame inserts it (the frame's writes go as posted commands).
    snapshot: bool,
    _t: PhantomData<fn() -> T>,
}

struct CopyComponentHalf<T: Component + Clone> {
    assign: fn(&mut EntityWorldMut, T),
    refs: fn(&T) -> Option<Entity>,
    inbound: bool,
    drawn: bool,
    hash: fn(&T) -> u64,
    text: fn(&T) -> String,
    /// `EXA_BRIDGE_WRITES=<type name part>`, free mode, frame side: the
    /// values as last received, to log what a frame write changed.
    texts: bevy::ecs::entity::EntityHashMap<String>,
    logged: u32,
    /// Free mode, frame side: the hash of each entity's value as the
    /// simulation last sent it. A frame value still equal to it was only
    /// touched (a write of the same value, a mutable borrow) and is not
    /// sent: it would undo the simulation's newer writes.
    received: bevy::ecs::entity::EntityHashMap<u64>,
    /// Frame values not sent for that, since the last report.
    untouched: u64,
    snapshot: bool,
    /// The frame changed a snapshot without inserting it (a write not
    /// posted): warned once.
    warned: bool,
    added: Option<ValueQuery<Ref<'static, T>>>,
    id: Option<ComponentId>,
    query: Option<ValueQuery<Ref<'static, T>>>,
    /// The values changed since a tick, filtered in the storage's change
    /// ticks (no per-value `Ref`).
    changed: Option<ChangedQuery<T>>,
    /// The frame's changed values of [`SimShared`] entities (hierarchy
    /// types).
    shared_changed: Option<SharedChangedQuery<T>>,
    removals: Removals,
    since: Tick,
    /// Free mode, simulation side: the last step that wrote each entity's
    /// value (data both sides write).
    sim_written: bevy::ecs::entity::EntityHashMap<u64>,
    prune_at: usize,
    overtaken: (u64, u64),
    /// Free mode: when this side removed the component from an entity (the
    /// frame packet's number, or the simulation's step). A value the other
    /// side sent before it had seen that removal does not put it back.
    removed_at: bevy::ecs::entity::EntityHashMap<u64>,
}

/// Changed values of the entities the simulation holds.
type SharedChangedQuery<T> =
    QueryState<(Entity, &'static T), (Changed<T>, With<SimShared>, Without<IsResource>)>;

/// Changed values and removals of one component.
struct ComponentPacket<T> {
    removed: Vec<Entity>,
    values: Vec<(Entity, T, bool)>,
}

impl<T> ComponentPacket<T> {
    /// Puts the removals and values in entity order. The simulation applies
    /// them in that order: the order a type's values reach it moves those
    /// entities between archetypes, and the simulation's queries then visit
    /// them in that order (the order of a system's commands and of the
    /// trace's rows). The frame gathers in its own query order, which hangs
    /// on its load-time archetypes (meshes, lights and the GUI arrive in
    /// whatever order loading finishes), so without this one run's
    /// simulation could visit two bodies in the other order from another's.
    ///
    /// The order is the entities' index, ascending: the frame hands out
    /// fresh ids in that order, so it is mostly the order one world would
    /// have given them (spawn order).
    fn into_entity_order(mut self) -> Self {
        self.removed.sort_unstable_by_key(|e| e.index_u32());
        // Stable: an entity named twice keeps its last value last.
        self.values.sort_by_key(|(e, ..)| e.index_u32());
        self
    }
}

fn assign_mutable<T: Component<Mutability = Mutable>>(entity: &mut EntityWorldMut, value: T) {
    if let Some(mut current) = entity.get_mut::<T>() {
        *current = value;
    } else {
        entity.insert(value);
    }
}

fn assign_by_insert<T: Component>(entity: &mut EntityWorldMut, value: T) {
    entity.insert(value);
}

impl<T: Component + Clone> BridgeType for CopyComponent<T> {
    fn type_id(&self) -> TypeId {
        TypeId::of::<T>()
    }

    fn half(&self) -> Box<dyn BridgeHalf> {
        Box::new(CopyComponentHalf::<T> {
            assign: self.assign,
            refs: self.refs,
            inbound: self.inbound,
            drawn: self.drawn,
            hash: self.hash,
            text: self.text,
            texts: Default::default(),
            logged: 0,
            received: Default::default(),
            untouched: 0,
            snapshot: self.snapshot,
            warned: false,
            added: None,
            id: None,
            query: None,
            changed: None,
            shared_changed: None,
            removals: Removals::new(),
            since: Tick::new(0),
            sim_written: Default::default(),
            prune_at: 4096,
            overtaken: (0, 0),
            removed_at: Default::default(),
        })
    }
}

fn component_id<T: Component>(world: &mut World, slot: &mut Option<ComponentId>) -> ComponentId {
    *slot.get_or_insert_with(|| world.register_component::<T>())
}

/// Removals of entities that still exist without the component (a despawn
/// is sent apart).
fn live_removals<T: Component>(world: &World, removed: &[Entity]) -> Vec<Entity> {
    removed
        .iter()
        .copied()
        .filter(|&e| world.get_entity(e).is_ok_and(|e| !e.contains::<T>()))
        .collect()
}

fn apply_removals<T: Component>(world: &mut World, removed: &[Entity]) {
    for &e in removed {
        if let Ok(mut entity) = world.get_entity_mut(e) {
            entity.remove::<T>();
        }
    }
}

impl<T: Component + Clone> CopyComponentHalf<T> {
    /// Free mode: notes when this side removed the component.
    fn note_removed(&mut self, removed: &[Entity], ctx: GatherCtx) {
        if ctx.free {
            for &e in removed {
                self.removed_at.insert(e, ctx.step);
            }
        }
    }

    /// Free mode: whether a value for `e` (which lacks the component here)
    /// was sent before the other side had seen this side's removal: it
    /// would put the component back.
    fn removed_since(&mut self, e: Entity, seen: Option<u64>) -> bool {
        let Some(seen) = seen else {
            return false;
        };
        self.removed_at.get(&e).is_some_and(|&at| at > seen)
    }

    /// The frame's side of a hierarchy type: the changed values of the
    /// entities the simulation holds, and every value of those marked in
    /// this gather.
    fn gather_shared(&mut self, world: &mut World, ctx: GatherCtx) -> Gathered {
        let now = world.change_tick();
        let since = self.since;
        let Some(changed) = self.shared_changed.as_mut() else {
            return Gathered::empty();
        };
        let world: &World = world;
        let newly = &world.resource::<NewlyShared>().0;
        // SAFETY: a read-only query of the world; the state was made from it.
        let query = unsafe {
            changed.query_unchecked_with_ticks(world.as_unsafe_world_cell_readonly(), since, now)
        };
        let mut values: Vec<(Entity, T, bool)> = query
            .iter()
            .filter(|(e, _)| !newly.contains(e))
            .map(|(e, v)| (e, T::clone(v), true))
            .collect();
        values.extend(
            newly
                .iter()
                .filter_map(|&e| world.get::<T>(e).map(|v| (e, v.clone(), true))),
        );
        let seen = self.removals.take();
        let removed = live_removals::<T>(world, &seen);
        self.note_removed(&removed, ctx);
        let touched = values
            .iter()
            .flat_map(|(e, v, _)| [Some(*e), (self.refs)(v)])
            .flatten()
            .collect();
        let n = values.len() + removed.len();
        Gathered {
            packet: Box::new(ComponentPacket { removed, values }),
            touched,
            removed: seen,
            values: n,
        }
    }

    /// The frame's side of a snapshot: the values the frame inserted since
    /// the last copy out, and its removals; a value it only changed is not
    /// sent (warned once: a write that should have been posted).
    fn gather_inserted(&mut self, world: &mut World, ctx: GatherCtx) -> Gathered {
        let now = world.change_tick();
        let since = self.since;
        let Some(query) = self.added.as_mut() else {
            return Gathered::empty();
        };
        let mut changed_only = None;
        let values: Vec<(Entity, T, bool)> = query
            .iter(world)
            .filter_map(|(e, r)| {
                if r.added().is_newer_than(since, now) {
                    Some((e, T::clone(&r), true))
                } else {
                    if r.last_changed().is_newer_than(since, now) {
                        changed_only.get_or_insert(e);
                    }
                    None
                }
            })
            .collect();
        if let Some(e) = changed_only
            && !self.warned
        {
            self.warned = true;
            warn!(
                "simulation bridge: the frame changed {} of {e} without posting it (not sent)",
                type_name::<T>()
            );
        }
        let seen = self.removals.take();
        let removed = live_removals::<T>(world, &seen);
        self.note_removed(&removed, ctx);
        let touched = values.iter().map(|(e, ..)| *e).collect();
        let n = values.len() + removed.len();
        Gathered {
            packet: Box::new(ComponentPacket { removed, values }),
            touched,
            removed: seen,
            values: n,
        }
    }
}

impl<T: Component + Clone> BridgeHalf for CopyComponentHalf<T> {
    fn name(&self) -> &'static str {
        type_name::<T>()
    }

    fn init(&mut self, world: &mut World) -> ComponentId {
        let id = component_id::<T>(world, &mut self.id);
        self.query = Some(world.query_filtered());
        self.changed = Some(world.query_filtered());
        self.added = Some(world.query_filtered());
        self.shared_changed = Some(world.query_filtered());
        id
    }

    fn hierarchy(&self) -> bool {
        is_hierarchy(TypeId::of::<T>())
    }

    fn drain_removed(&mut self, world: &World) {
        if let Some(id) = self.id {
            self.removals.drain(world, id);
        }
    }

    fn gather(&mut self, world: &mut World, ctx: GatherCtx) -> Gathered {
        let all = ctx.all;
        self.drain_removed(world);
        if self.inbound && ctx.sim {
            self.removals.take();
            return Gathered::empty();
        }
        if self.snapshot && !ctx.sim && !all {
            return self.gather_inserted(world, ctx);
        }
        if !ctx.sim && world.contains_resource::<NewlyShared>() && self.hierarchy() {
            return self.gather_shared(world, ctx);
        }
        let now = world.change_tick();
        let since = self.since;
        let values: Vec<(Entity, T, bool)> = if all {
            let Some(query) = self.query.as_mut() else {
                return Gathered::empty();
            };
            query
                .iter(world)
                .map(|(e, v)| (e, T::clone(&v), true))
                .collect()
        } else {
            let Some(changed) = self.changed.as_mut() else {
                return Gathered::empty();
            };
            // SAFETY: a read-only query of a world this half holds `&mut`
            // to; the state was made from this world.
            let query = unsafe {
                changed.query_unchecked_with_ticks(
                    world.as_unsafe_world_cell_readonly(),
                    since,
                    now,
                )
            };
            query.iter().map(|(e, v)| (e, T::clone(v), true)).collect()
        };
        if ctx.sim && ctx.free && !self.snapshot {
            for (e, ..) in &values {
                self.sim_written.insert(*e, ctx.step);
            }
        }
        let mut values = values;
        if !ctx.sim && ctx.free && !all && !self.received.is_empty() {
            let (hash, received) = (self.hash, &self.received);
            let before = values.len();
            values.retain(|(e, v, _)| received.get(e) != Some(&hash(v)));
            self.untouched += (before - values.len()) as u64;
            if self.logged < 20 && writes_logged(type_name::<T>()) {
                for (e, v, _) in &values {
                    if let Some(was) = self.texts.get(e) {
                        let now = (self.text)(v);
                        let (a, b) = first_difference(was, &now);
                        info!(
                            "simulation bridge: the frame wrote {} of {e}: …{a}… -> …{b}…",
                            type_name::<T>()
                        );
                        self.logged += 1;
                    }
                }
            }
        }
        let seen = self.removals.take();
        let removed = live_removals::<T>(world, &seen);
        self.note_removed(&removed, ctx);
        // A relationship's target must exist on the other side too.
        let touched = values
            .iter()
            .flat_map(|(e, v, _)| [Some(*e), (self.refs)(v)])
            .flatten()
            .collect();
        let n = values.len() + removed.len();
        Gathered {
            packet: Box::new(ComponentPacket { removed, values }),
            touched,
            removed: seen,
            values: n,
        }
    }

    fn apply(&mut self, world: &mut World, packet: Packet, ctx: ApplyCtx) -> usize {
        let Ok(packet) = packet.downcast::<ComponentPacket<T>>() else {
            return 0;
        };
        let packet = if ctx.sim {
            packet.into_entity_order()
        } else {
            *packet
        };
        apply_removals::<T>(world, &packet.removed);
        if !ctx.sim && ctx.free && !self.drawn && !self.inbound && !self.snapshot {
            for e in &packet.removed {
                self.received.remove(e);
            }
            for (e, v, _) in &packet.values {
                self.received.insert(*e, (self.hash)(v));
            }
            if writes_logged(type_name::<T>()) {
                for (e, v, _) in &packet.values {
                    self.texts.insert(*e, (self.text)(v));
                }
            }
            if self.received.len() > self.prune_at {
                self.received
                    .retain(|e, _| world.entities().contains_spawned(*e));
                self.prune_at = (self.received.len() * 2).max(4096);
            }
        }
        if ctx.free && self.sim_written.len() > self.prune_at {
            // Despawned entities' marks go.
            self.sim_written
                .retain(|e, _| world.entities().contains_spawned(*e));
            self.prune_at = (self.sim_written.len() * 2).max(4096);
        }
        // Free mode: a frame value of data both sides write, after a
        // simulation write the frame had not seen yet. A drawn copy loses
        // to it; another wins, as in lockstep (counted).
        let base = ctx
            .based_on
            .filter(|_| ctx.sim && !self.inbound && !self.snapshot);
        let n = packet.values.len();
        let seen = if ctx.sim { ctx.based_on } else { ctx.seen_seq };
        if ctx.free
            && let Some(seen) = seen
        {
            self.removed_at.retain(|_, at| *at > seen);
        }
        for (e, value, _) in packet.values {
            if ctx.free
                && world.get_entity(e).is_ok_and(|w| !w.contains::<T>())
                && self.removed_since(e, seen)
            {
                continue;
            }
            if let Some(base) = base
                && self.sim_written.get(&e).is_some_and(|&step| step > base)
                && world.get::<T>(e).is_some()
            {
                if self.drawn {
                    self.overtaken.0 += 1;
                    continue;
                }
                self.overtaken.1 += 1;
            }
            if let Ok(mut entity) = world.get_entity_mut(e) {
                (self.assign)(&mut entity, value);
            }
        }
        n
    }

    fn take_overtaken(&mut self) -> (u64, u64) {
        std::mem::take(&mut self.overtaken)
    }

    fn take_untouched(&mut self) -> u64 {
        std::mem::take(&mut self.untouched)
    }

    fn settle(&mut self, world: &mut World, since: Tick) {
        if let Some(id) = self.id {
            self.removals.skip(world, id);
        }
        self.since = since;
    }
}

/// Whether `EXA_BRIDGE_WRITES` names (part of) `name`: a debug aid that
/// logs the first frame writes of that type a free simulation's newer
/// value met, by their first differing text.
fn writes_logged(name: &str) -> bool {
    static PART: std::sync::OnceLock<Option<String>> = std::sync::OnceLock::new();
    PART.get_or_init(|| std::env::var("EXA_BRIDGE_WRITES").ok())
        .as_deref()
        .is_some_and(|p| name.contains(p))
}

/// Where two texts first differ, with some context on each side.
fn first_difference(a: &str, b: &str) -> (String, String) {
    let at = a
        .char_indices()
        .zip(b.chars())
        .find(|((_, x), y)| x != y)
        .map_or(a.len().min(b.len()), |((i, _), _)| i);
    let from = a[..at].char_indices().rev().nth(40).map_or(0, |(i, _)| i);
    let cut = |t: &str| {
        t.get(from..)
            .unwrap_or("")
            .chars()
            .take(160)
            .collect::<String>()
    };
    (cut(a), cut(b))
}

fn debug_text<T: std::fmt::Debug>(value: &T) -> String {
    format!("{value:?}")
}

/// A hash of `value`'s `Debug` text, written into the hasher without a
/// string.
fn debug_hash<T: std::fmt::Debug>(value: &T) -> u64 {
    use std::hash::Hasher;
    struct Feed(std::collections::hash_map::DefaultHasher);
    impl std::fmt::Write for Feed {
        fn write_str(&mut self, s: &str) -> std::fmt::Result {
            self.0.write(s.as_bytes());
            Ok(())
        }
    }
    let mut feed = Feed(Default::default());
    let _ = std::fmt::Write::write_fmt(&mut feed, format_args!("{value:?}"));
    feed.0.finish()
}

/// A component lent for the step: the value moves across and back, a
/// default holding its place.
pub struct LendComponent<T>(PhantomData<fn() -> T>);

struct LendComponentHalf<T: Component<Mutability = Mutable> + Default> {
    id: Option<ComponentId>,
    query: Option<ValueQuery<Mut<'static, T>>>,
    removals: Removals,
    since: Tick,
}

impl<T: Component<Mutability = Mutable> + Default> BridgeType for LendComponent<T> {
    fn type_id(&self) -> TypeId {
        TypeId::of::<T>()
    }

    fn half(&self) -> Box<dyn BridgeHalf> {
        Box::new(LendComponentHalf::<T> {
            id: None,
            query: None,
            removals: Removals::new(),
            since: Tick::new(0),
        })
    }
}

impl<T: Component<Mutability = Mutable> + Default> BridgeHalf for LendComponentHalf<T> {
    fn name(&self) -> &'static str {
        type_name::<T>()
    }

    fn init(&mut self, world: &mut World) -> ComponentId {
        let id = component_id::<T>(world, &mut self.id);
        self.query = Some(world.query_filtered());
        id
    }

    fn drain_removed(&mut self, world: &World) {
        if let Some(id) = self.id {
            self.removals.drain(world, id);
        }
    }

    fn gather(&mut self, world: &mut World, ctx: GatherCtx) -> Gathered {
        let all = ctx.all;
        self.drain_removed(world);
        let now = world.change_tick();
        let since = self.since;
        let Some(query) = self.query.as_mut() else {
            return Gathered::empty();
        };
        let values: Vec<(Entity, T, bool)> = query
            .iter_mut(world)
            .map(|(e, mut v)| {
                let changed = all || v.last_changed().is_newer_than(since, now);
                (e, std::mem::take(v.bypass_change_detection()), changed)
            })
            .collect();
        let seen = self.removals.take();
        let removed = live_removals::<T>(world, &seen);
        let touched = values.iter().map(|(e, ..)| *e).collect();
        let n = values.len() + removed.len();
        Gathered {
            packet: Box::new(ComponentPacket { removed, values }),
            touched,
            removed: seen,
            values: n,
        }
    }

    fn apply(&mut self, world: &mut World, packet: Packet, ctx: ApplyCtx) -> usize {
        let Ok(packet) = packet.downcast::<ComponentPacket<T>>() else {
            return 0;
        };
        let packet = if ctx.sim {
            packet.into_entity_order()
        } else {
            *packet
        };
        apply_removals::<T>(world, &packet.removed);
        let n = packet.values.len();
        for (e, value, changed) in packet.values {
            let Ok(mut entity) = world.get_entity_mut(e) else {
                continue;
            };
            if let Some(mut current) = entity.get_mut::<T>() {
                *current.bypass_change_detection() = value;
                if changed {
                    current.set_changed();
                }
            } else {
                entity.insert(value);
            }
        }
        n
    }

    fn settle(&mut self, world: &mut World, since: Tick) {
        if let Some(id) = self.id {
            self.removals.skip(world, id);
        }
        self.since = since;
    }
}

// ---------------------------------------------------------------- resources

/// A resource copied by clone when it changed.
pub struct CopyResource<T>(PhantomData<fn() -> T>);
/// A resource the frame owns and the simulation only reads: copied in when
/// the frame changes it, never sent back.
pub struct CopyInResource<T>(PhantomData<fn() -> T>);
/// A resource the simulation owns and the frame only reads: copied out
/// when the simulation changes it, copied in only when the frame inserts
/// it.
pub struct SnapshotResource<T>(PhantomData<fn() -> T>);
/// A resource lent for the step.
pub struct LendResource<T>(PhantomData<fn() -> T>);

/// Which way a copied resource crosses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Direction {
    Both,
    /// Frame to simulation only.
    In,
    /// Simulation to frame, and the frame's inserts.
    Out,
}

struct ResourceHalf<T: Resource + Component<Mutability = Mutable>> {
    lend: bool,
    direction: Direction,
    /// The owner-side rule was broken once (warned once).
    warned: bool,
    /// The value to send: a clone, or (lent) the value itself, swapped for
    /// the spare.
    take: fn(&mut World, &mut Option<T>) -> Option<T>,
    /// A lent resource's place holder: the value an apply swapped out,
    /// swapped back in by the next gather (a default is built only once).
    spare: Option<T>,
    id: Option<ComponentId>,
    present: bool,
    since: Tick,
    /// Free mode, simulation side: the last step that wrote it (data both
    /// sides write), and the frame values dropped for it.
    sim_written: Option<u64>,
    overtaken: u64,
}

/// A resource's value, or its removal.
enum ResourcePacket<T> {
    Unchanged,
    Removed,
    Value(T, bool),
}

fn copied_half<T: Resource + Component<Mutability = Mutable> + Clone>(
    direction: Direction,
) -> Box<dyn BridgeHalf> {
    Box::new(ResourceHalf::<T> {
        lend: false,
        direction,
        warned: false,
        take: |world, _| world.get_resource::<T>().cloned(),
        spare: None,
        id: None,
        present: false,
        since: Tick::new(0),
        sim_written: None,
        overtaken: 0,
    })
}

impl<T: Resource + Component<Mutability = Mutable> + Clone> BridgeType for CopyResource<T> {
    fn type_id(&self) -> TypeId {
        TypeId::of::<T>()
    }

    fn half(&self) -> Box<dyn BridgeHalf> {
        copied_half::<T>(Direction::Both)
    }
}

impl<T: Resource + Component<Mutability = Mutable> + Clone> BridgeType for CopyInResource<T> {
    fn type_id(&self) -> TypeId {
        TypeId::of::<T>()
    }

    fn half(&self) -> Box<dyn BridgeHalf> {
        copied_half::<T>(Direction::In)
    }
}

impl<T: Resource + Component<Mutability = Mutable> + Clone> BridgeType for SnapshotResource<T> {
    fn type_id(&self) -> TypeId {
        TypeId::of::<T>()
    }

    fn half(&self) -> Box<dyn BridgeHalf> {
        copied_half::<T>(Direction::Out)
    }
}

impl<T: Resource + Component<Mutability = Mutable> + Default> BridgeType for LendResource<T> {
    fn type_id(&self) -> TypeId {
        TypeId::of::<T>()
    }

    fn half(&self) -> Box<dyn BridgeHalf> {
        Box::new(ResourceHalf::<T> {
            lend: true,
            direction: Direction::Both,
            warned: false,
            take: |world, spare| {
                let mut r = world.get_resource_mut::<T>()?;
                let mut value = spare.take().unwrap_or_default();
                std::mem::swap(&mut value, r.bypass_change_detection());
                Some(value)
            },
            spare: None,
            id: None,
            present: false,
            since: Tick::new(0),
            sim_written: None,
            overtaken: 0,
        })
    }
}

impl<T: Resource + Component<Mutability = Mutable>> BridgeHalf for ResourceHalf<T> {
    fn name(&self) -> &'static str {
        type_name::<T>()
    }

    fn init(&mut self, world: &mut World) -> ComponentId {
        *self
            .id
            .get_or_insert_with(|| world.register_component::<T>())
    }

    fn hold(&self, world: &mut World) {
        crate::resource_hold::hold_resource_entity::<T>(world);
    }

    fn register(&self, world: &mut World) {
        world.register_component::<T>();
    }

    fn frame_absent(&self, free: bool) -> bool {
        // A lent resource stays with the simulation when it runs free.
        self.lend && free
    }

    fn drain_removed(&mut self, _: &World) {}

    fn gather(&mut self, world: &mut World, ctx: GatherCtx) -> Gathered {
        if self.lend && ctx.free {
            return self.gather_moved(world, ctx);
        }
        let all = ctx.all;
        let now = world.change_tick();
        let since = self.since;
        let mut changed = world
            .get_resource_ref::<T>()
            .map(|r| all || r.last_changed().is_newer_than(since, now));
        // A one-way resource: the side that does not own it sends nothing
        // but the frame's inserts and removals of a snapshot; a change
        // there is a write that should have gone the other way.
        let refused = match self.direction {
            Direction::In if ctx.sim => {
                let sent = changed == Some(true);
                changed = changed.map(|_| false);
                sent
            }
            Direction::Out if !ctx.sim && !all => {
                let added = world
                    .get_resource_ref::<T>()
                    .is_some_and(|r| r.added().is_newer_than(since, now));
                let sent = changed == Some(true) && !added;
                changed = changed.map(|_| added);
                sent
            }
            _ => false,
        };
        if refused && !self.warned {
            self.warned = true;
            warn!(
                "simulation bridge: the {} changed {}, which the {} owns (not sent)",
                if ctx.sim { "simulation" } else { "frame" },
                type_name::<T>(),
                if ctx.sim { "frame" } else { "simulation" }
            );
        }
        if ctx.sim && ctx.free && self.direction == Direction::Both && changed == Some(true) {
            self.sim_written = Some(ctx.step);
        }
        let was = std::mem::replace(&mut self.present, changed.is_some());
        let packet = match changed {
            None if was => ResourcePacket::Removed,
            None => ResourcePacket::Unchanged,
            Some(changed) if self.lend || changed => match (self.take)(world, &mut self.spare) {
                Some(value) => ResourcePacket::Value(value, changed),
                None => ResourcePacket::Unchanged,
            },
            Some(_) => ResourcePacket::Unchanged,
        };
        let values = usize::from(!matches!(packet, ResourcePacket::Unchanged));
        Gathered {
            packet: Box::new(packet),
            touched: Vec::new(),
            removed: Vec::new(),
            values,
        }
    }

    fn apply(&mut self, world: &mut World, packet: Packet, ctx: ApplyCtx) -> usize {
        let Ok(packet) = packet.downcast::<ResourcePacket<T>>() else {
            return 0;
        };
        match *packet {
            ResourcePacket::Unchanged => 0,
            ResourcePacket::Removed => {
                world.remove_resource::<T>();
                1
            }
            ResourcePacket::Value(value, changed) => {
                if ctx.sim
                    && self.direction == Direction::Both
                    && !self.lend
                    && ctx
                        .based_on
                        .zip(self.sim_written)
                        .is_some_and(|(base, step)| step > base)
                {
                    // The simulation wrote it after the step the frame had
                    // seen; the frame's write wins, as in lockstep.
                    self.overtaken += 1;
                }
                match world.get_resource_mut::<T>() {
                    Some(mut current) if self.lend && !ctx.free => {
                        let old = std::mem::replace(current.bypass_change_detection(), value);
                        self.spare = Some(old);
                        if changed {
                            current.set_changed();
                        }
                    }
                    Some(mut current) => *current = value,
                    None => world.insert_resource(value),
                }
                1
            }
        }
    }

    fn take_overtaken(&mut self) -> (u64, u64) {
        (0, std::mem::take(&mut self.overtaken))
    }

    fn settle(&mut self, world: &mut World, since: Tick) {
        self.present = world.contains_resource::<T>();
        self.since = since;
    }
}

impl<T: Resource + Component<Mutability = Mutable>> ResourceHalf<T> {
    /// A lent resource when the simulation runs free: it cannot come back
    /// between steps, so it moves to the simulation for good. The frame
    /// sends it each time it is in the frame's world (inserted again, as a
    /// level load does) and the simulation never sends it back.
    fn gather_moved(&mut self, world: &mut World, ctx: GatherCtx) -> Gathered {
        let moved = if ctx.sim {
            None
        } else {
            world.remove_resource::<T>()
        };
        let packet = match moved {
            Some(value) => ResourcePacket::Value(value, true),
            None => ResourcePacket::Unchanged,
        };
        let values = usize::from(!matches!(packet, ResourcePacket::Unchanged));
        Gathered {
            packet: Box::new(packet),
            touched: Vec::new(),
            removed: Vec::new(),
            values,
        }
    }
}

// ---------------------------------------------------------------- messages

/// A message type both worlds write: each side's new messages are written
/// on the other.
pub struct ForwardMessage<M>(PhantomData<fn() -> M>);

struct MessageHalf<M: Message + Clone> {
    id: Option<ComponentId>,
    cursor: Option<MessageCursor<M>>,
}

impl<M: Message + Clone> BridgeType for ForwardMessage<M> {
    fn type_id(&self) -> TypeId {
        TypeId::of::<M>()
    }

    fn half(&self) -> Box<dyn BridgeHalf> {
        Box::new(MessageHalf::<M> {
            id: None,
            cursor: None,
        })
    }
}

impl<M: Message + Clone> BridgeHalf for MessageHalf<M> {
    fn name(&self) -> &'static str {
        type_name::<Messages<M>>()
    }

    fn init(&mut self, world: &mut World) -> ComponentId {
        if !world.contains_resource::<Messages<M>>() {
            world.init_resource::<Messages<M>>();
        }
        // From the oldest message kept: the ones written before the first
        // step reach the other side too.
        self.cursor = Some(world.resource::<Messages<M>>().get_cursor());
        *self
            .id
            .get_or_insert_with(|| world.register_component::<Messages<M>>())
    }

    fn drain_removed(&mut self, _: &World) {}

    fn gather(&mut self, world: &mut World, _: GatherCtx) -> Gathered {
        let (Some(cursor), Some(messages)) =
            (self.cursor.as_mut(), world.get_resource::<Messages<M>>())
        else {
            return Gathered::empty();
        };
        let sent: Vec<M> = cursor.read(messages).cloned().collect();
        let values = sent.len();
        Gathered {
            packet: Box::new(sent),
            touched: Vec::new(),
            removed: Vec::new(),
            values,
        }
    }

    fn apply(&mut self, world: &mut World, packet: Packet, ctx: ApplyCtx) -> usize {
        let Ok(sent) = packet.downcast::<Vec<M>>() else {
            return 0;
        };
        let Some(mut messages) = world.get_resource_mut::<Messages<M>>() else {
            return 0;
        };
        if ctx.sim && ctx.update_messages {
            messages.update();
        }
        let n = sent.len();
        messages.write_batch(*sent);
        n
    }

    fn settle(&mut self, world: &mut World, _: Tick) {
        if let (Some(cursor), Some(messages)) =
            (self.cursor.as_mut(), world.get_resource::<Messages<M>>())
        {
            cursor.read(messages).for_each(drop);
        }
    }
}

// ---------------------------------------------------------------- children

/// The order of each parent's `Children`: `ChildOf`'s hooks keep the set
/// on each side, in the order the relationships arrived, so the frame's
/// order (spawn order) is sent and applied after the relationships.
pub struct ChildrenOrder;

struct ChildrenOrderHalf {
    id: Option<ComponentId>,
    query: Option<ValueQuery<Ref<'static, Children>>>,
    changed: Option<ChangedQuery<Children>>,
    shared_changed: Option<SharedChangedQuery<Children>>,
    since: Tick,
}

impl BridgeType for ChildrenOrder {
    fn type_id(&self) -> TypeId {
        TypeId::of::<Children>()
    }

    fn half(&self) -> Box<dyn BridgeHalf> {
        Box::new(ChildrenOrderHalf {
            id: None,
            query: None,
            changed: None,
            shared_changed: None,
            since: Tick::new(0),
        })
    }
}

impl BridgeHalf for ChildrenOrderHalf {
    fn name(&self) -> &'static str {
        type_name::<Children>()
    }

    fn init(&mut self, world: &mut World) -> ComponentId {
        let id = component_id::<Children>(world, &mut self.id);
        self.query = Some(world.query_filtered());
        self.changed = Some(world.query_filtered());
        self.shared_changed = Some(world.query_filtered());
        id
    }

    fn hierarchy(&self) -> bool {
        true
    }

    fn drain_removed(&mut self, _: &World) {}

    fn gather(&mut self, world: &mut World, ctx: GatherCtx) -> Gathered {
        let now = world.change_tick();
        let since = self.since;
        let orders: Vec<(Entity, Vec<Entity>)> =
            if !ctx.sim && world.contains_resource::<NewlyShared>() {
                // The order among the children the simulation holds, of the
                // parents it holds: changed ones and those marked now.
                let Some(changed) = self.shared_changed.as_mut() else {
                    return Gathered::empty();
                };
                let world: &World = world;
                let newly = &world.resource::<NewlyShared>().0;
                let shared = |c: &Children| -> Vec<Entity> {
                    c.iter()
                        .filter(|&e| world.get::<SimShared>(e).is_some())
                        .collect()
                };
                // SAFETY: as in `CopyComponentHalf::gather`.
                let query = unsafe {
                    changed.query_unchecked_with_ticks(
                        world.as_unsafe_world_cell_readonly(),
                        since,
                        now,
                    )
                };
                let mut orders: Vec<(Entity, Vec<Entity>)> = query
                    .iter()
                    .filter(|(e, _)| !newly.contains(e))
                    .map(|(e, c)| (e, shared(c)))
                    .collect();
                orders.extend(
                    newly
                        .iter()
                        .filter_map(|&e| world.get::<Children>(e).map(|c| (e, shared(c)))),
                );
                orders
            } else if ctx.all {
                let Some(query) = self.query.as_mut() else {
                    return Gathered::empty();
                };
                query.iter(world).map(|(e, c)| (e, c.to_vec())).collect()
            } else {
                let Some(changed) = self.changed.as_mut() else {
                    return Gathered::empty();
                };
                // SAFETY: as in `CopyComponentHalf::gather`.
                let query = unsafe {
                    changed.query_unchecked_with_ticks(
                        world.as_unsafe_world_cell_readonly(),
                        since,
                        now,
                    )
                };
                query.iter().map(|(e, c)| (e, c.to_vec())).collect()
            };
        Gathered {
            values: orders.len(),
            packet: Box::new(orders),
            touched: Vec::new(),
            removed: Vec::new(),
        }
    }

    fn apply(&mut self, world: &mut World, packet: Packet, _: ApplyCtx) -> usize {
        let Ok(orders) = packet.downcast::<Vec<(Entity, Vec<Entity>)>>() else {
            return 0;
        };
        let mut n = 0;
        for (parent, order) in *orders {
            let Ok(mut entity) = world.get_entity_mut(parent) else {
                continue;
            };
            let Some(mut children) = entity.get_mut::<Children>() else {
                continue;
            };
            if **children == order[..] {
                continue;
            }
            // Only the children the order names move, among the places
            // they hold; the others (the frame's own) keep theirs.
            let at: bevy::ecs::entity::EntityHashMap<usize> =
                order.iter().enumerate().map(|(i, &e)| (e, i)).collect();
            let places: Vec<usize> = children
                .iter()
                .enumerate()
                .filter(|(_, e)| at.contains_key(e))
                .map(|(i, _)| i)
                .collect();
            let mut named: Vec<Entity> = places.iter().map(|&i| children[i]).collect();
            named.sort_by_key(|e| at[e]);
            if places.iter().zip(&named).all(|(&i, e)| children[i] == *e) {
                continue;
            }
            let mut list: Vec<Entity> = children.to_vec();
            for (&i, &e) in places.iter().zip(&named) {
                list[i] = e;
            }
            let rank: bevy::ecs::entity::EntityHashMap<usize> =
                list.iter().enumerate().map(|(i, &e)| (e, i)).collect();
            children.sort_by_key(|e| rank.get(e).copied().unwrap_or(usize::MAX));
            n += 1;
        }
        n
    }

    fn settle(&mut self, _: &mut World, since: Tick) {
        self.since = since;
    }
}

// ---------------------------------------------------------------- moved

/// A resource only the simulation's systems touch, which the frame inserts
/// (a trace file opened at start): moved to the simulation once it appears
/// in the frame's world, never sent back.
pub struct MoveResource<T>(PhantomData<fn() -> T>);

struct MoveResourceHalf<T: Resource> {
    id: Option<ComponentId>,
    _t: PhantomData<fn() -> T>,
}

impl<T: Resource> BridgeType for MoveResource<T> {
    fn type_id(&self) -> TypeId {
        TypeId::of::<T>()
    }

    fn half(&self) -> Box<dyn BridgeHalf> {
        Box::new(MoveResourceHalf::<T> {
            id: None,
            _t: PhantomData,
        })
    }
}

impl<T: Resource> BridgeHalf for MoveResourceHalf<T> {
    fn name(&self) -> &'static str {
        type_name::<T>()
    }

    fn init(&mut self, world: &mut World) -> ComponentId {
        component_id::<T>(world, &mut self.id)
    }

    fn hold(&self, world: &mut World) {
        crate::resource_hold::hold_resource_entity::<T>(world);
    }

    fn register(&self, world: &mut World) {
        world.register_component::<T>();
    }

    fn frame_absent(&self, _: bool) -> bool {
        true
    }

    fn drain_removed(&mut self, _: &World) {}

    fn gather(&mut self, world: &mut World, ctx: GatherCtx) -> Gathered {
        if ctx.sim {
            return Gathered::empty();
        }
        let moved = world.remove_resource::<T>();
        Gathered {
            values: usize::from(moved.is_some()),
            packet: Box::new(moved),
            touched: Vec::new(),
            removed: Vec::new(),
        }
    }

    fn apply(&mut self, world: &mut World, packet: Packet, _: ApplyCtx) -> usize {
        match packet.downcast::<Option<T>>() {
            Ok(moved) => match *moved {
                Some(value) => {
                    world.insert_resource(value);
                    1
                }
                None => 0,
            },
            Err(_) => 0,
        }
    }

    fn settle(&mut self, _: &mut World, _: Tick) {}
}

// ---------------------------------------------------------------- sim only

/// Data only the simulation's systems touch (a resource the step inserts
/// and reads within itself): never copied, listed as covered.
pub struct SimOnly<T>(PhantomData<fn() -> T>);

struct SimOnlyHalf<T: Component> {
    id: Option<ComponentId>,
    _t: PhantomData<fn() -> T>,
}

impl<T: Component> BridgeType for SimOnly<T> {
    fn type_id(&self) -> TypeId {
        TypeId::of::<T>()
    }

    fn half(&self) -> Box<dyn BridgeHalf> {
        Box::new(SimOnlyHalf::<T> {
            id: None,
            _t: PhantomData,
        })
    }
}

impl<T: Component> BridgeHalf for SimOnlyHalf<T> {
    fn name(&self) -> &'static str {
        type_name::<T>()
    }

    fn frame_absent(&self, _: bool) -> bool {
        // The frame keeps a queue of its own of the simulation's writes
        // (listed here only to cover the simulation's).
        TypeId::of::<T>() != TypeId::of::<crate::sim_app::FrameCommands>()
    }

    fn init(&mut self, world: &mut World) -> ComponentId {
        component_id::<T>(world, &mut self.id)
    }

    fn drain_removed(&mut self, _: &World) {}

    fn gather(&mut self, _: &mut World, _: GatherCtx) -> Gathered {
        Gathered::empty()
    }

    fn apply(&mut self, _: &mut World, _: Packet, _: ApplyCtx) -> usize {
        0
    }

    fn settle(&mut self, _: &mut World, _: Tick) {}
}

// ---------------------------------------------------------------- own

/// A resource each side keeps its own of, never copied (a scratch buffer
/// such as the gizmos the step draws before it collects them): the
/// simulation's world gets a default one when the bridge starts.
pub struct OwnResource<T>(PhantomData<fn() -> T>);

struct OwnResourceHalf<T: Resource + Default> {
    id: Option<ComponentId>,
    _t: PhantomData<fn() -> T>,
}

impl<T: Resource + Default> BridgeType for OwnResource<T> {
    fn type_id(&self) -> TypeId {
        TypeId::of::<T>()
    }

    fn half(&self) -> Box<dyn BridgeHalf> {
        Box::new(OwnResourceHalf::<T> {
            id: None,
            _t: PhantomData,
        })
    }
}

impl<T: Resource + Default> BridgeHalf for OwnResourceHalf<T> {
    fn name(&self) -> &'static str {
        type_name::<T>()
    }

    fn init(&mut self, world: &mut World) -> ComponentId {
        world.init_resource::<T>();
        component_id::<T>(world, &mut self.id)
    }

    fn hold(&self, world: &mut World) {
        crate::resource_hold::hold_resource_entity::<T>(world);
    }

    fn register(&self, world: &mut World) {
        world.register_component::<T>();
    }

    fn drain_removed(&mut self, _: &World) {}

    fn gather(&mut self, _: &mut World, _: GatherCtx) -> Gathered {
        Gathered::empty()
    }

    fn apply(&mut self, _: &mut World, _: Packet, _: ApplyCtx) -> usize {
        0
    }

    fn settle(&mut self, _: &mut World, _: Tick) {}
}

/// The gizmos the fixed step drew, as a snapshot for the frame to draw:
/// copied out (by appending into a fresh storage) when the step collected
/// them.
pub struct GizmoSnapshot<C>(PhantomData<fn() -> C>);

impl<C: bevy::gizmos::config::GizmoConfigGroup> BridgeType for GizmoSnapshot<C> {
    fn type_id(&self) -> TypeId {
        TypeId::of::<bevy::gizmos::gizmos::GizmoStorage<C, Fixed>>()
    }

    fn half(&self) -> Box<dyn BridgeHalf> {
        Box::new(
            ResourceHalf::<bevy::gizmos::gizmos::GizmoStorage<C, Fixed>> {
                lend: false,
                direction: Direction::Out,
                warned: false,
                take: |world, _| {
                    let drawn =
                        world.get_resource::<bevy::gizmos::gizmos::GizmoStorage<C, Fixed>>()?;
                    let mut copy = bevy::gizmos::gizmos::GizmoStorage::<C, Fixed>::default();
                    copy.append_storage(drawn);
                    Some(copy)
                },
                spare: None,
                id: None,
                present: false,
                since: Tick::new(0),
                sim_written: None,
                overtaken: 0,
            },
        )
    }
}

// ---------------------------------------------------------------- registry

/// The bridged types, registered on the frame's app.
#[derive(Resource, Default)]
pub struct SimBridgeTypes {
    types: Vec<Box<dyn BridgeType>>,
    seen: HashSet<TypeId>,
    /// Makers of each digestible type's digester (`crate::sim_digest`).
    digests: Vec<DigestMaker>,
    /// Frame systems allowed to read a type the frame's world does not
    /// hold ([`audit`]).
    allowed: Vec<audit::AllowedRead>,
}

/// Makes a digester for one bridged type.
pub type DigestMaker = fn() -> Box<dyn crate::sim_digest::Digester>;

impl SimBridgeTypes {
    fn add(&mut self, t: impl BridgeType, digest: Option<DigestMaker>) {
        if self.seen.insert(t.type_id()) {
            self.types.push(Box::new(t));
            self.digests.extend(digest);
        }
    }

    /// A fresh digester for every digestible bridged type.
    pub fn digesters(&self) -> Vec<Box<dyn crate::sim_digest::Digester>> {
        self.digests.iter().map(|make| make()).collect()
    }

    pub fn len(&self) -> usize {
        self.types.len()
    }

    pub fn is_empty(&self) -> bool {
        self.types.is_empty()
    }

    /// The bridged types' names, plus the ones they cover (`Children`
    /// follows `ChildOf`; Bevy's message registry is signalled by the
    /// frame).
    pub fn covered_names(&self) -> BTreeSet<String> {
        let mut names: BTreeSet<String> = self.types.iter().map(|t| t.name().to_owned()).collect();
        if names.contains(type_name::<ChildOf>()) {
            names.insert(type_name::<Children>().to_owned());
        }
        names.insert(type_name::<MessageRegistry>().to_owned());
        names
    }

    fn halves(&self) -> Vec<Box<dyn BridgeHalf>> {
        self.types.iter().map(|t| t.half()).collect()
    }
}

/// Registers bridged types. Harmless without the bridge: the list is only
/// read when the bridge starts.
pub trait SimBridgeAppExt {
    /// A component copied when it changed.
    fn bridge_copy<T: Component<Mutability = Mutable> + Clone + std::fmt::Debug>(
        &mut self,
    ) -> &mut Self;
    /// [`Self::bridge_copy`] for a component the frame writes to draw it
    /// (transforms, the drawn controller): when the simulation runs free, a
    /// frame value that comes after a simulation write the frame had not
    /// seen is dropped (in lockstep there is none).
    fn bridge_copy_drawn<T: Component<Mutability = Mutable> + Clone + std::fmt::Debug>(
        &mut self,
    ) -> &mut Self;
    /// A relationship such as `ChildOf` (immutable) copied by re-inserting
    /// it when it changed; its target is spawned on the other side too.
    fn bridge_relationship<T: Relationship + Clone + std::fmt::Debug>(&mut self) -> &mut Self;
    /// A component the frame builds and the simulation owns from then on:
    /// copied in when the frame changes it, never sent back (the frame
    /// reads a snapshot instead).
    fn bridge_copy_in<T: Component<Mutability = Mutable> + Clone + std::fmt::Debug>(
        &mut self,
    ) -> &mut Self;
    /// A component the simulation owns, which the frame reads as a
    /// snapshot: copied out when the simulation changes it, copied in only
    /// when the frame inserts it; the frame's writes go as posted commands
    /// (`crate::sim_app::FixedCommands::write`).
    fn bridge_snapshot<T: Component<Mutability = Mutable> + Clone + std::fmt::Debug>(
        &mut self,
    ) -> &mut Self;
    /// A component lent to the simulation for each step.
    fn bridge_lend<T: Component<Mutability = Mutable> + Default + std::fmt::Debug>(
        &mut self,
    ) -> &mut Self;
    /// A resource copied when it changed.
    fn bridge_copy_resource<
        T: Resource + Component<Mutability = Mutable> + Clone + std::fmt::Debug,
    >(
        &mut self,
    ) -> &mut Self;
    /// A resource the frame owns and the simulation only reads: copied in
    /// when the frame changes it (or removes it), never sent back; a
    /// simulation change is warned of once.
    fn bridge_copy_in_resource<
        T: Resource + Component<Mutability = Mutable> + Clone + std::fmt::Debug,
    >(
        &mut self,
    ) -> &mut Self;
    /// A resource the simulation owns and the frame reads as a snapshot:
    /// copied out when the simulation changes it, copied in only when the
    /// frame inserts it; a frame change is warned of once and not sent.
    fn bridge_snapshot_resource<
        T: Resource + Component<Mutability = Mutable> + Clone + std::fmt::Debug,
    >(
        &mut self,
    ) -> &mut Self;
    /// A resource lent to the simulation for each step.
    fn bridge_lend_resource<
        T: Resource + Component<Mutability = Mutable> + Default + std::fmt::Debug,
    >(
        &mut self,
    ) -> &mut Self;
    /// A lent resource without `Debug` (Bevy's gizmo storages, the collision
    /// world): left out of the digest (`crate::sim_digest`).
    fn bridge_lend_opaque_resource<T: Resource + Component<Mutability = Mutable> + Default>(
        &mut self,
    ) -> &mut Self;
    /// A copied resource left out of the digest (large read-only data:
    /// the game's files, the motion set).
    fn bridge_copy_opaque_resource<T: Resource + Component<Mutability = Mutable> + Clone>(
        &mut self,
    ) -> &mut Self;
    /// A message type forwarded both ways.
    fn bridge_message<M: Message + Clone>(&mut self) -> &mut Self;
    /// The order of `Children` (after `ChildOf`).
    fn bridge_children_order(&mut self) -> &mut Self;
    /// A component or resource only the simulation's systems touch: not
    /// copied.
    fn bridge_sim_only<T: Component>(&mut self) -> &mut Self;
    /// A resource the frame inserts and only the simulation's systems
    /// touch: moved to the simulation when it appears.
    fn bridge_move_resource<T: Resource>(&mut self) -> &mut Self;
    /// A resource each side keeps its own of, never copied.
    fn bridge_own_resource<T: Resource + Default>(&mut self) -> &mut Self;
    /// A gizmo group the fixed step draws into: each side keeps its own
    /// drawing buffer, and the step's collected gizmos reach the frame as
    /// a snapshot.
    fn bridge_gizmos<C: bevy::gizmos::config::GizmoConfigGroup>(&mut self) -> &mut Self;
}

impl SimBridgeAppExt for App {
    fn bridge_copy<T: Component<Mutability = Mutable> + Clone + std::fmt::Debug>(
        &mut self,
    ) -> &mut Self {
        add_digested(
            self,
            CopyComponent::<T> {
                assign: assign_mutable::<T>,
                refs: |_| None,
                inbound: false,
                hash: debug_hash::<T>,
                text: debug_text::<T>,
                drawn: false,
                snapshot: false,
                _t: PhantomData,
            },
            Some(crate::sim_digest::component::<T>),
        )
    }

    fn bridge_copy_drawn<T: Component<Mutability = Mutable> + Clone + std::fmt::Debug>(
        &mut self,
    ) -> &mut Self {
        add_digested(
            self,
            CopyComponent::<T> {
                assign: assign_mutable::<T>,
                refs: |_| None,
                inbound: false,
                hash: debug_hash::<T>,
                text: debug_text::<T>,
                drawn: true,
                snapshot: false,
                _t: PhantomData,
            },
            Some(crate::sim_digest::component::<T>),
        )
    }

    fn bridge_copy_in<T: Component<Mutability = Mutable> + Clone + std::fmt::Debug>(
        &mut self,
    ) -> &mut Self {
        add_digested(
            self,
            CopyComponent::<T> {
                assign: assign_mutable::<T>,
                refs: |_| None,
                inbound: true,
                hash: debug_hash::<T>,
                text: debug_text::<T>,
                drawn: false,
                snapshot: false,
                _t: PhantomData,
            },
            Some(crate::sim_digest::component::<T>),
        )
    }

    fn bridge_snapshot<T: Component<Mutability = Mutable> + Clone + std::fmt::Debug>(
        &mut self,
    ) -> &mut Self {
        add_digested(
            self,
            CopyComponent::<T> {
                assign: assign_mutable::<T>,
                refs: |_| None,
                inbound: false,
                hash: debug_hash::<T>,
                text: debug_text::<T>,
                drawn: false,
                snapshot: true,
                _t: PhantomData,
            },
            Some(crate::sim_digest::component::<T>),
        )
    }

    fn bridge_relationship<T: Relationship + Clone + std::fmt::Debug>(&mut self) -> &mut Self {
        add_digested(
            self,
            CopyComponent::<T> {
                assign: assign_by_insert::<T>,
                refs: |r| Some(r.get()),
                inbound: false,
                hash: debug_hash::<T>,
                text: debug_text::<T>,
                drawn: false,
                snapshot: false,
                _t: PhantomData,
            },
            Some(crate::sim_digest::component::<T>),
        )
    }

    fn bridge_lend<T: Component<Mutability = Mutable> + Default + std::fmt::Debug>(
        &mut self,
    ) -> &mut Self {
        add_digested(
            self,
            LendComponent::<T>(PhantomData),
            Some(crate::sim_digest::component::<T>),
        )
    }

    fn bridge_copy_resource<
        T: Resource + Component<Mutability = Mutable> + Clone + std::fmt::Debug,
    >(
        &mut self,
    ) -> &mut Self {
        add_digested(
            self,
            CopyResource::<T>(PhantomData),
            Some(crate::sim_digest::resource::<T>),
        )
    }

    fn bridge_copy_in_resource<
        T: Resource + Component<Mutability = Mutable> + Clone + std::fmt::Debug,
    >(
        &mut self,
    ) -> &mut Self {
        add_digested(
            self,
            CopyInResource::<T>(PhantomData),
            Some(crate::sim_digest::resource::<T>),
        )
    }

    fn bridge_snapshot_resource<
        T: Resource + Component<Mutability = Mutable> + Clone + std::fmt::Debug,
    >(
        &mut self,
    ) -> &mut Self {
        add_digested(
            self,
            SnapshotResource::<T>(PhantomData),
            Some(crate::sim_digest::resource::<T>),
        )
    }

    fn bridge_lend_resource<
        T: Resource + Component<Mutability = Mutable> + Default + std::fmt::Debug,
    >(
        &mut self,
    ) -> &mut Self {
        add_digested(
            self,
            LendResource::<T>(PhantomData),
            Some(crate::sim_digest::resource::<T>),
        )
    }

    fn bridge_lend_opaque_resource<T: Resource + Component<Mutability = Mutable> + Default>(
        &mut self,
    ) -> &mut Self {
        add_type(self, LendResource::<T>(PhantomData))
    }

    fn bridge_copy_opaque_resource<T: Resource + Component<Mutability = Mutable> + Clone>(
        &mut self,
    ) -> &mut Self {
        add_type(self, CopyResource::<T>(PhantomData))
    }

    fn bridge_message<M: Message + Clone>(&mut self) -> &mut Self {
        add_type(self, ForwardMessage::<M>(PhantomData))
    }

    fn bridge_children_order(&mut self) -> &mut Self {
        add_digested(
            self,
            ChildrenOrder,
            Some(crate::sim_digest::component::<Children>),
        )
    }

    fn bridge_sim_only<T: Component>(&mut self) -> &mut Self {
        add_type(self, SimOnly::<T>(PhantomData))
    }

    fn bridge_own_resource<T: Resource + Default>(&mut self) -> &mut Self {
        add_type(self, OwnResource::<T>(PhantomData))
    }

    fn bridge_gizmos<C: bevy::gizmos::config::GizmoConfigGroup>(&mut self) -> &mut Self {
        add_type(
            self,
            OwnResource::<bevy::gizmos::gizmos::GizmoStorage<C, ()>>(PhantomData),
        );
        add_type(self, GizmoSnapshot::<C>(PhantomData))
    }

    fn bridge_move_resource<T: Resource>(&mut self) -> &mut Self {
        add_type(self, MoveResource::<T>(PhantomData))
    }
}

fn add_type(app: &mut App, t: impl BridgeType) -> &mut App {
    add_digested(app, t, None)
}

fn add_digested(app: &mut App, t: impl BridgeType, digest: Option<DigestMaker>) -> &mut App {
    t.half().register(app.world_mut());
    app.world_mut()
        .get_resource_or_init::<SimBridgeTypes>()
        .add(t, digest);
    app
}

// ---------------------------------------------------------------- the two sides

/// Ids the frame reserved for the simulation's own entities, not yet used.
#[derive(Debug, Default)]
pub struct IdPool {
    free: EntityHashSet,
}

impl IdPool {
    pub fn len(&self) -> usize {
        self.free.len()
    }

    pub fn is_empty(&self) -> bool {
        self.free.is_empty()
    }
}

/// Reserves `n` never-used ids from `world`'s allocator (ids it reused are
/// put back), for the other world to spawn its own entities at.
pub fn reserve_ids(world: &mut World, n: usize) -> Vec<Entity> {
    let mut fresh = Vec::with_capacity(n);
    let mut reused = Vec::new();
    while fresh.len() < n {
        let e = world.entity_allocator().alloc();
        if e.generation() == bevy::ecs::entity::EntityGeneration::FIRST {
            fresh.push(e);
        } else {
            reused.push(e);
        }
    }
    world.entity_allocator_mut().free_many(&reused);
    fresh
}

/// Spawns an empty entity at `e`'s id in `world` unless it is there,
/// despawning a stale entity at the same index first and moving the index's
/// generation up to `e`'s. Returns whether `e` is spawned afterwards.
pub fn ensure_spawned(world: &mut World, e: Entity) -> bool {
    for _ in 0..(1 << 16) {
        let current = world.entities().resolve_from_index(e.index());
        let spawned = world.entities().is_index_spawned(e.index());
        if spawned && current == e {
            return true;
        }
        if spawned {
            despawn_unfreed(world, current);
            continue;
        }
        let ahead = e
            .generation()
            .to_bits()
            .wrapping_sub(current.generation().to_bits());
        if ahead == 0 {
            return world.spawn_empty_at(e).is_ok();
        }
        if ahead > u32::MAX / 2 {
            // This world is already past `e`: it is gone here.
            return false;
        }
        // Step the index's generation on (spawn and despawn unfreed).
        if world.spawn_empty_at(current).is_err() {
            return false;
        }
        world.despawn_no_free(current);
    }
    false
}

/// [`ensure_spawned`] on the frame's side, which hands out every id: an
/// entity of the frame's that took the index since (a newer generation) or
/// a resource is never despawned for a value the simulation sent. A free
/// simulation's step can name an entity the frame despawned while the step
/// ran; that value is stale and is dropped.
pub fn ensure_spawned_on_frame(world: &mut World, e: Entity) -> bool {
    let current = world.entities().resolve_from_index(e.index());
    if world.entities().is_index_spawned(e.index()) && current != e {
        let newer = current
            .generation()
            .to_bits()
            .wrapping_sub(e.generation().to_bits());
        let resource = world
            .get_entity(current)
            .is_ok_and(|c| c.contains::<IsResource>());
        if resource || (newer != 0 && newer <= u32::MAX / 2) {
            return false;
        }
    }
    ensure_spawned(world, e)
}

/// Despawns `e` without freeing its id and without despawning its children
/// with it (each side's children are mirrored on their own).
pub fn despawn_unfreed(world: &mut World, e: Entity) {
    let Ok(mut entity) = world.get_entity_mut(e) else {
        return;
    };
    entity.remove::<Children>();
    world.despawn_no_free(e);
}

/// A debug aid (`EXA_BRIDGE_ALL=1`): every value crosses every step, changed
/// or not, to tell a change the bridge missed from other differences.
fn copy_everything() -> bool {
    static ALL: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ALL.get_or_init(|| std::env::var("EXA_BRIDGE_ALL").is_ok_and(|v| v == "1"))
}

/// One side's gathered changes for a step.
pub struct SidePacket {
    packets: Vec<Packet>,
    touched: Vec<Entity>,
    despawned: Vec<Entity>,
    /// Ids the frame reserves for the simulation (frame → sim only).
    reserved: Vec<Entity>,
    /// The frame's writes to snapshot data, run first (frame → sim only).
    writes: Vec<crate::sim_app::SimCommand>,
    /// The frame's posted calls (`crate::sim_app::FixedCommands::post`),
    /// queued on the simulation's own queue (frame → sim only).
    posts: Vec<crate::sim_app::SimCommand>,
    pub update_messages: bool,
    /// Free mode (frame → sim): the last step the frame had applied.
    based_on: Option<u64>,
    /// Frame → sim: the frame packet's number; sim → frame: the last frame
    /// packet the simulation had applied.
    seq: u64,
    pub values: usize,
}

impl SidePacket {
    /// A packet that changes nothing, for `n` halves: applied before a
    /// step no frame packet came for, so the next gather counts from it.
    fn empty(n: usize) -> Self {
        Self {
            packets: (0..n).map(|_| Box::new(()) as Packet).collect(),
            touched: Vec::new(),
            despawned: Vec::new(),
            reserved: Vec::new(),
            writes: Vec::new(),
            posts: Vec::new(),
            update_messages: false,
            based_on: None,
            seq: 0,
            values: 0,
        }
    }
}

/// One world's halves and their bookkeeping.
pub struct BridgeSide {
    halves: Vec<Box<dyn BridgeHalf>>,
    ids: Vec<ComponentId>,
    first: bool,
    sim: bool,
    /// The simulation runs free of the frame.
    free: bool,
    /// Simulation side: the step the next gather follows.
    step: u64,
    /// Frame side: the packets gathered; simulation side: the last frame
    /// packet applied.
    seq: u64,
    /// Free mode, simulation side: per half, frame values that came after
    /// a simulation write the frame had not seen: dropped, applied.
    overtaken: Vec<(u64, u64)>,
    /// Per half: values gathered, values applied, nanoseconds (both).
    stats: Vec<(u64, u64, u64)>,
    gathers: u64,
    /// Frame side: ancestors of entities the simulation sent, marked
    /// [`SimShared`] at the apply; their hierarchy goes whole at the next
    /// gather.
    pending_shared: EntityHashSet,
}

impl BridgeSide {
    /// `first`: the first gather sends every value (the frame's side; the
    /// simulation's world starts empty).
    fn new(halves: Vec<Box<dyn BridgeHalf>>, first: bool) -> Self {
        Self {
            halves,
            ids: Vec::new(),
            first,
            sim: !first,
            free: false,
            step: 0,
            seq: 0,
            overtaken: Vec::new(),
            stats: Vec::new(),
            gathers: 0,
            pending_shared: EntityHashSet::default(),
        }
    }

    fn init(&mut self, world: &mut World) {
        self.ids = self.halves.iter_mut().map(|h| h.init(world)).collect();
    }

    /// The costliest types over the last steps (each side's gather and
    /// apply), then the counters start again.
    fn log_stats(&mut self) {
        let side = if self.sim { "simulation" } else { "frame" };
        let mut top: Vec<(usize, (u64, u64, u64))> =
            self.stats.iter().copied().enumerate().collect();
        top.sort_by_key(|(_, s)| std::cmp::Reverse(s.2));
        let steps = crate::sim_app::LOG_EVERY;
        let line: Vec<String> = top
            .iter()
            .take(10)
            .map(|&(i, (gathered, applied, ns))| {
                format!(
                    "{} {:.0} us ({:.1} out, {:.1} in a step)",
                    self.halves[i].name(),
                    ns as f64 / 1000.0 / steps as f64,
                    gathered as f64 / steps as f64,
                    applied as f64 / steps as f64
                )
            })
            .collect();
        info!(
            "simulation bridge, {side} side, costliest types: {}",
            line.join("; ")
        );
        self.stats.iter_mut().for_each(|s| *s = (0, 0, 0));
        for (which, pick) in [
            ("dropped", (|o: (u64, u64)| o.0) as fn((u64, u64)) -> u64),
            ("applied", |o: (u64, u64)| o.1),
        ] {
            let mut top: Vec<(usize, u64)> = self
                .overtaken
                .iter()
                .map(|&o| pick(o))
                .enumerate()
                .filter(|&(_, n)| n > 0)
                .collect();
            if top.is_empty() {
                continue;
            }
            top.sort_by_key(|&(_, n)| std::cmp::Reverse(n));
            let line: Vec<String> = top
                .iter()
                .take(12)
                .map(|&(i, n)| format!("{} {n}", self.halves[i].name()))
                .collect();
            info!(
                "simulation bridge: frame values after a newer simulation write ({which}): {}",
                line.join("; ")
            );
        }
        self.overtaken.iter_mut().for_each(|n| *n = (0, 0));
        let mut untouched: Vec<(usize, u64)> = self
            .halves
            .iter_mut()
            .map(|h| h.take_untouched())
            .enumerate()
            .filter(|&(_, n)| n > 0)
            .collect();
        if !untouched.is_empty() {
            untouched.sort_by_key(|&(_, n)| std::cmp::Reverse(n));
            let line: Vec<String> = untouched
                .iter()
                .take(12)
                .map(|&(i, n)| format!("{} {n}", self.halves[i].name()))
                .collect();
            info!(
                "simulation bridge: frame values only touched since the simulation sent them (not sent): {}",
                line.join("; ")
            );
        }
    }

    fn drain_removed(&mut self, world: &World) {
        for half in &mut self.halves {
            half.drain_removed(world);
        }
    }

    fn gather(&mut self, world: &mut World) -> SidePacket {
        if !self.sim {
            self.seq += 1;
        }
        let ctx = GatherCtx {
            all: std::mem::take(&mut self.first) || copy_everything(),
            sim: self.sim,
            free: self.free,
            step: if self.sim { self.step } else { self.seq },
        };
        let mut packets: Vec<Option<Packet>> = (0..self.halves.len()).map(|_| None).collect();
        let mut touched = EntityHashSet::default();
        let mut despawned = EntityHashSet::default();
        let mut values = 0;
        self.stats.resize(self.halves.len(), (0, 0, 0));
        // The frame narrows its hierarchy scans: the other types first,
        // then the entities they name (and their ancestors) are marked
        // shared, then the hierarchy of the shared entities.
        let narrow = !self.sim && narrowed();
        for pass in 0..2 {
            if narrow && pass == 1 {
                let mut newly = std::mem::take(&mut self.pending_shared);
                let mut named: Vec<Entity> = touched.iter().copied().collect();
                named.sort_unstable();
                for e in named {
                    mark_shared(world, e, &mut newly);
                }
                newly.retain(|&e| world.entities().contains_spawned(e));
                world.insert_resource(NewlyShared(newly));
            }
            for ((half, stat), slot) in self
                .halves
                .iter_mut()
                .zip(&mut self.stats)
                .zip(&mut packets)
            {
                let late = narrow && half.hierarchy();
                if late != (pass == 1) {
                    continue;
                }
                let started = Instant::now();
                let g = half.gather(world, ctx);
                stat.0 += g.values as u64;
                stat.2 += started.elapsed().as_nanos() as u64;
                values += g.values;
                touched.extend(g.touched);
                despawned.extend(
                    g.removed
                        .into_iter()
                        .filter(|&e| !world.entities().contains_spawned(e)),
                );
                *slot = Some(g.packet);
            }
        }
        if narrow {
            world.remove_resource::<NewlyShared>();
        }
        let packets: Vec<Packet> = packets
            .into_iter()
            .map(|p| p.unwrap_or_else(|| Box::new(())))
            .collect();
        let mut touched: Vec<Entity> = touched.into_iter().collect();
        touched.sort_unstable();
        let mut despawned: Vec<Entity> = despawned.into_iter().collect();
        despawned.sort_unstable();
        self.gathers += 1;
        if self.gathers.is_multiple_of(crate::sim_app::LOG_EVERY) {
            self.log_stats();
        }
        SidePacket {
            packets,
            touched,
            despawned,
            reserved: Vec::new(),
            writes: Vec::new(),
            posts: Vec::new(),
            update_messages: false,
            based_on: None,
            seq: self.seq,
            values,
        }
    }

    /// Applies the other side's packet: despawns, spawns, then each type in
    /// registration order; then settles every half.
    fn apply(&mut self, world: &mut World, packet: SidePacket, sim: bool) -> usize {
        for &e in &packet.despawned {
            if world.entities().contains_spawned(e) {
                despawn_unfreed(world, e);
            }
        }
        for &e in &packet.touched {
            if sim {
                ensure_spawned(world, e);
            } else {
                ensure_spawned_on_frame(world, e);
            }
        }
        // The frame holds what the simulation sent as shared: those
        // entities are in sync; their ancestors go whole at the next gather.
        if !sim && narrowed() {
            for &e in &packet.touched {
                if let Ok(mut entity) = world.get_entity_mut(e)
                    && !entity.contains::<SimShared>()
                {
                    entity.insert(SimShared);
                    if let Some(parent) = entity.get::<ChildOf>().map(ChildOf::parent) {
                        mark_shared(world, parent, &mut self.pending_shared);
                    }
                }
            }
        }
        // The frame packet the simulation applies now (empty packets carry
        // none).
        if sim && packet.seq > 0 {
            self.seq = packet.seq;
        }
        let ctx = ApplyCtx {
            sim,
            update_messages: packet.update_messages,
            based_on: packet.based_on,
            free: self.free,
            seen_seq: (!sim && self.free).then_some(packet.seq),
        };
        let mut n = 0;
        self.stats.resize(self.halves.len(), (0, 0, 0));
        for ((half, p), stat) in self
            .halves
            .iter_mut()
            .zip(packet.packets)
            .zip(&mut self.stats)
        {
            let started = Instant::now();
            let applied = half.apply(world, p, ctx);
            // Relationship hooks queue their `Children` edits: apply them
            // before the next type (the children's order comes after).
            world.flush();
            stat.1 += applied as u64;
            stat.2 += started.elapsed().as_nanos() as u64;
            n += applied;
        }
        world.flush();
        if self.free && sim {
            self.overtaken.resize(self.halves.len(), (0, 0));
            for (half, n) in self.halves.iter_mut().zip(&mut self.overtaken) {
                let (dropped, applied) = half.take_overtaken();
                n.0 += dropped;
                n.1 += applied;
            }
        }
        let since = world.change_tick();
        for half in &mut self.halves {
            half.settle(world, since);
        }
        world.increment_change_tick();
        n
    }
}

/// Counters of the bridge's work: values sent each way and the time each
/// half of the copy took (µs, summed over steps).
#[derive(Resource, Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct BridgeCost {
    pub steps: u64,
    pub values_in: u64,
    pub values_out: u64,
    pub gather_in_us: u64,
    pub apply_in_us: u64,
    pub gather_out_us: u64,
    pub apply_out_us: u64,
    pub spawned_out: u64,
    pub despawned_out: u64,
}

impl BridgeCost {
    /// Microseconds of copying per step, all four halves.
    pub fn us_per_step(&self) -> f64 {
        let total = self.gather_in_us + self.apply_in_us + self.gather_out_us + self.apply_out_us;
        total as f64 / self.steps.max(1) as f64
    }
}

/// The schedules the bridge moved, the coverage of their systems' data, and
/// the systems that take the whole world.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct BridgeCoverage {
    pub schedules: Vec<String>,
    pub systems: usize,
    pub bridged: BTreeSet<String>,
    pub missing: BTreeSet<String>,
    pub whole_world: BTreeSet<String>,
    /// Schedules that had already run on the frame's world (not moved).
    pub already_built: Vec<String>,
}

/// What the frame hands the simulation's world when the bridge starts.
pub struct BridgeSetup {
    schedules: Vec<Schedule>,
    fixed_order: Vec<InternedScheduleLabel>,
    halves: Vec<Box<dyn BridgeHalf>>,
    already_built: Vec<String>,
    free: bool,
}

/// The frame's side of the bridge (a resource of the frame's world).
#[derive(Resource)]
pub struct FrameBridge {
    /// Schedules to move besides `FixedMainScheduleOrder`'s.
    extra: Vec<InternedScheduleLabel>,
    /// `FixedMainScheduleOrder`'s labels, read when the app is built.
    fixed_order: Vec<InternedScheduleLabel>,
    side: Option<BridgeSide>,
    frames: u64,
    last_frames: u64,
    /// Reserved ids the simulation had left after the last step (counting
    /// the ones reserved since, which it has not reported yet).
    pool_left: usize,
    pub cost: BridgeCost,
    pub coverage: Option<BridgeCoverage>,
    /// The simulation runs free of the frame (`crate::sim_app::SimMode::Free`).
    pub free: bool,
    /// Free mode: the last step whose packet the frame applied.
    pub applied_step: u64,
    /// The frame's readers of types its world does not hold ([`audit`]).
    pub audit: audit::FrameReadAudit,
}

impl FrameBridge {
    pub fn started(&self) -> bool {
        self.side.is_some()
    }
}

/// The simulation's side (a resource of the simulation's world).
#[derive(Resource)]
pub struct SimBridge {
    side: BridgeSide,
    pool: IdPool,
    /// Archetypes already checked for components the bridge does not carry.
    archetypes_seen: usize,
    unbridged_seen: HashSet<ComponentId>,
}

/// The fixed step's schedules, in order, in the simulation's world.
#[derive(Resource, Debug, Default, Clone)]
pub struct SimFixedOrder(pub Vec<InternedScheduleLabel>);

/// Runs the moved fixed-step schedules in order (in `SimStep`). With
/// `EXA_SIM_TIMING=1` each schedule's wall time is summed and logged every
/// 1000 steps (`StepTiming`).
pub fn run_fixed_order(world: &mut World) {
    let Some(order) = world.get_resource::<SimFixedOrder>().cloned() else {
        return;
    };
    if !step_timing() {
        for label in order.0 {
            let _ = world.try_run_schedule(label);
        }
        return;
    }
    let mut spent = Vec::with_capacity(order.0.len());
    for label in order.0 {
        let started = Instant::now();
        let _ = world.try_run_schedule(label);
        spent.push((label, started.elapsed().as_micros() as u64));
    }
    let mut timing = world.get_resource_or_init::<StepTiming>();
    timing.steps += 1;
    for (label, us) in spent {
        match timing.schedules.iter_mut().find(|(l, _)| *l == label) {
            Some((_, total)) => *total += us,
            None => timing.schedules.push((label, us)),
        }
    }
    if timing.steps.is_multiple_of(crate::sim_app::LOG_EVERY) {
        let steps = timing.steps as f64;
        let line: Vec<String> = timing
            .schedules
            .iter()
            .filter(|(_, us)| *us > 0)
            .map(|(l, us)| format!("{l:?} {:.2}", *us as f64 / 1000.0 / steps))
            .collect();
        let total: u64 = timing.schedules.iter().map(|(_, us)| us).sum();
        info!(
            "simulation step timing (ms a step, last {} steps): {:.2} in all; {}",
            timing.steps,
            total as f64 / 1000.0 / steps,
            line.join(", ")
        );
        timing.steps = 0;
        timing.schedules.iter_mut().for_each(|(_, us)| *us = 0);
    }
}

/// Whether the fixed step's schedules are timed (`EXA_SIM_TIMING=1`).
fn step_timing() -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| std::env::var("EXA_SIM_TIMING").is_ok_and(|v| v == "1"))
}

/// The fixed step's schedules' wall time, summed (µs), and the steps.
#[derive(Resource, Default)]
struct StepTiming {
    steps: u64,
    schedules: Vec<(InternedScheduleLabel, u64)>,
}

/// The frame's half of a step's bridge traffic, in.
pub struct BridgeIn {
    pub setup: Option<BridgeSetup>,
    pub packet: SidePacket,
}

/// The simulation's half, out.
pub struct BridgeOut {
    pub packet: SidePacket,
    pub coverage: Option<BridgeCoverage>,
    pub apply_us: u64,
    pub gather_us: u64,
    pub pool: usize,
}

/// Adds the bridge to an app that has the simulation's shell; `extra` are
/// the schedules the fixed step runs from inside its own (they move too).
pub struct SimBridgePlugin {
    pub extra: Vec<InternedScheduleLabel>,
}

impl Plugin for SimBridgePlugin {
    fn build(&self, app: &mut App) {
        let free = app
            .world()
            .get_resource::<crate::sim_app::SimShell>()
            .is_some_and(|s| s.mode() == crate::sim_app::SimMode::Free);
        app.init_resource::<SimBridgeTypes>()
            .insert_resource(FrameBridge {
                extra: [
                    crate::sim_app::SimPreStep.intern(),
                    crate::sim_app::SimFrameStart.intern(),
                ]
                .into_iter()
                .chain(self.extra.iter().copied())
                .collect(),
                fixed_order: Vec::new(),
                side: None,
                frames: 0,
                last_frames: 0,
                pool_left: 0,
                cost: BridgeCost::default(),
                coverage: None,
                free,
                applied_step: 0,
                audit: default(),
            })
            .add_systems(First, (count_frame, audit::audit_frame_reads))
            .add_systems(Last, (drain_frame_removals, audit::audit_frame_reads));
        // The frame hands out every id, the simulation's included: its
        // resources' entities take theirs at a fixed moment.
        app.add_plugins(crate::resource_hold::ResourceHoldPlugin);
        bridge_bevy_types(app);
        app.world_mut()
            .get_resource_or_init::<crate::sim_app::FixedCommands>()
            .set_bridged(true);
    }
}

/// Counts the frame's updates; on the first, before any fixed step, reads
/// the fixed step's order (inside `FixedMain` it is out of the world).
/// Bevy's own types the fixed step reads and writes: the hierarchy (its
/// `Children` follow `ChildOf` by its hooks), transforms, names and clocks.
pub fn bridge_bevy_types(app: &mut App) {
    app.bridge_relationship::<ChildOf>()
        .bridge_children_order()
        .bridge_copy_drawn::<Transform>()
        .bridge_copy_drawn::<GlobalTransform>()
        .bridge_copy::<Name>()
        .bridge_copy_resource::<bevy::diagnostic::FrameCount>()
        .bridge_copy_resource::<Time>()
        .bridge_copy_resource::<Time<Fixed>>()
        .bridge_copy_resource::<Time<Virtual>>()
        .bridge_copy_resource::<Time<Real>>()
        .bridge_gizmos::<bevy::gizmos::aabb::AabbGizmoConfigGroup>()
        .bridge_gizmos::<bevy::gizmos::config::DefaultGizmoConfigGroup>()
        .bridge_gizmos::<bevy::gizmos::frustum::FrustumGizmoConfigGroup>()
        .bridge_gizmos::<bevy::gizmos::skinned_mesh_bounds::SkinnedMeshBoundsGizmoConfigGroup>()
        .bridge_gizmos::<bevy::light::gizmos::LightGizmoConfigGroup>();
}

fn count_frame(mut bridge: ResMut<FrameBridge>, order: Option<Res<FixedMainScheduleOrder>>) {
    bridge.frames += 1;
    if bridge.fixed_order.is_empty() {
        bridge.fixed_order = order.map_or_else(
            || FixedMainScheduleOrder::default().labels,
            |o| o.labels.clone(),
        );
    }
}

/// Reads the frame's removals every frame (they are kept two updates).
fn drain_frame_removals(world: &mut World) {
    world.resource_scope(|world, mut bridge: Mut<FrameBridge>| {
        if let Some(side) = bridge.side.as_mut() {
            side.drain_removed(world);
        }
    });
}

/// Frame side, before a step: starts the bridge on the first one (moving the
/// schedules), then gathers the frame's changes. `None` without the bridge.
pub fn frame_gather(world: &mut World) -> Option<BridgeIn> {
    if !world.contains_resource::<FrameBridge>() {
        return None;
    }
    world.resource_scope(|world, mut bridge: Mut<FrameBridge>| {
        let started = Instant::now();
        let setup = if bridge.side.is_none() {
            let halves = world
                .get_resource::<SimBridgeTypes>()
                .map(|t| (t.halves(), t.halves()));
            let (frame_halves, sim_halves) = halves.unwrap_or_default();
            let mut side = BridgeSide::new(frame_halves, true);
            side.free = bridge.free;
            side.init(world);
            bridge.side = Some(side);
            if bridge.fixed_order.is_empty() {
                // No frame has run its `First` (a test running `FixedMain`).
                bridge.fixed_order = FixedMainScheduleOrder::default().labels;
            }
            let mut setup = take_schedules(world, &bridge.fixed_order, &bridge.extra, sim_halves);
            setup.free = bridge.free;
            Some(setup)
        } else {
            None
        };
        // The frame's writes to snapshot data and its posted calls: each
        // side keeps its own queue, the frame's go in the packet.
        let (writes, posts) = world
            .get_resource_mut::<crate::sim_app::FixedCommands>()
            .map(|mut f| (f.take_writes(), f.take_posts()))
            .unwrap_or_default();
        let side = bridge.side.as_mut()?;
        let mut packet = side.gather(world);
        packet.writes = writes;
        packet.posts = posts;
        packet.update_messages = bridge.frames != bridge.last_frames;
        packet.based_on = bridge.free.then_some(bridge.applied_step);
        bridge.last_frames = bridge.frames;
        if setup.is_some() || bridge.pool_left < POOL_LOW {
            packet.reserved = reserve_ids(world, POOL_BATCH);
            // Counted until the simulation reports again (a free-running
            // simulation may not step for many frames).
            bridge.pool_left += POOL_BATCH;
        }
        bridge.cost.values_in += packet.values as u64;
        bridge.cost.gather_in_us += started.elapsed().as_micros() as u64;
        Some(BridgeIn { setup, packet })
    })
}

/// Takes the fixed step's schedules out of the frame's world.
fn take_schedules(
    world: &mut World,
    order: &[InternedScheduleLabel],
    extra: &[InternedScheduleLabel],
    halves: Vec<Box<dyn BridgeHalf>>,
) -> BridgeSetup {
    let fixed_order: Vec<InternedScheduleLabel> = order
        .iter()
        .copied()
        .filter(|l| *l != crate::sim_app::SimLockstep.intern())
        .collect();
    let mut schedules = Vec::new();
    let mut already_built = Vec::new();
    let mut schedules_res = world.resource_mut::<Schedules>();
    for &label in fixed_order.iter().chain(extra) {
        let built = schedules_res
            .get(label)
            .is_some_and(|s| s.systems().is_ok());
        if built {
            already_built.push(format!("{label:?}"));
            continue;
        }
        if let Some(schedule) = schedules_res.remove(label) {
            schedules.push(schedule);
        }
    }
    BridgeSetup {
        schedules,
        fixed_order,
        halves,
        already_built,
        free: false,
    }
}

/// Simulation side, before its commands and schedules: installs the setup on
/// the first step, then applies the frame's changes.
/// Applies the frame's changes; returns the coverage (first step), the
/// time it took, and whether a frame began since the last step.
pub fn sim_apply(world: &mut World, bridge_in: BridgeIn) -> (Option<BridgeCoverage>, u64, bool) {
    let started = Instant::now();
    let mut coverage = None;
    let BridgeIn { setup, mut packet } = bridge_in;
    // The frame's newly reserved ids are the allocator's before the install
    // spawns anything (the held resources' entities take pool ids too).
    let reserved = std::mem::take(&mut packet.reserved);
    world.entity_allocator_mut().free_many(&reserved);
    if let Some(setup) = setup {
        let c = install(world, setup);
        report_coverage(&c);
        if !c.missing.is_empty() {
            if cfg!(debug_assertions) {
                // Debug and test builds stop: a lane that adds a fixed-step
                // system must register what it touches.
                panic!(
                    "simulation bridge: the moved systems touch types no plugin registered: {:?}",
                    c.missing
                );
            } else {
                // A play build goes on: a system whose data is missing is
                // skipped with one warning.
                world.insert_resource(bevy::ecs::error::FallbackErrorHandler(warn_once));
            }
        }
        coverage = Some(c);
    }
    let frame_began = packet.update_messages;
    // The writes to snapshot data land on the simulation's own values
    // first, the simulation's own queued ones before the frame's; one to a
    // value the frame inserted since is a no-op, the inserted value
    // (written already) arriving next.
    let own = world
        .get_resource_mut::<crate::sim_app::FixedCommands>()
        .map(|mut f| f.take_writes())
        .unwrap_or_default();
    for write in own.into_iter().chain(std::mem::take(&mut packet.writes)) {
        write(world);
    }
    let posts = std::mem::take(&mut packet.posts);
    world.resource_scope(|world, mut bridge: Mut<SimBridge>| {
        // Those the install spawned already are dropped from the pool at
        // the step's gather, as every reserved id the step spawns.
        bridge.pool.free.extend(reserved);
        bridge.side.apply(world, packet, true);
    });
    // The frame's posted calls after the simulation's own (they run in the
    // fixed order's `FixedCommandsStep`).
    if let Some(mut fixed) = world.get_resource_mut::<crate::sim_app::FixedCommands>() {
        fixed.queue_posts(posts);
    }
    (coverage, started.elapsed().as_micros() as u64, frame_began)
}

/// Simulation side, free mode: before a step no frame packet came for,
/// settles every half (an apply of nothing), so the step's gather counts
/// the step's changes only.
pub fn sim_apply_nothing(world: &mut World) {
    world.resource_scope(|world, mut bridge: Mut<SimBridge>| {
        let n = bridge.side.halves.len();
        bridge.side.apply(world, SidePacket::empty(n), true);
    });
}

/// Frame side, free mode: after a gather no step packet answered this
/// frame, settles every half (an apply of nothing), so the next gather
/// sends only what the frame changes from now on.
pub fn frame_settle(world: &mut World) {
    if !world.contains_resource::<FrameBridge>() {
        return;
    }
    world.resource_scope(|world, mut bridge: Mut<FrameBridge>| {
        if let Some(side) = bridge.side.as_mut() {
            let n = side.halves.len();
            side.apply(world, SidePacket::empty(n), false);
        }
    });
}

/// Simulation side: whether the bridge has started (its first packet came).
pub fn sim_bridge_started(world: &World) -> bool {
    world.contains_resource::<SimBridge>()
}

fn install(world: &mut World, setup: BridgeSetup) -> BridgeCoverage {
    let mut side = BridgeSide::new(setup.halves, false);
    side.free = setup.free;
    side.init(world);
    for half in &side.halves {
        half.hold(world);
    }
    // The simulation's own queue of posted calls and writes; its writes
    // stay snapshot writes, as the frame's queue's were when it was lent.
    if let Some(mut fixed) = world.get_resource_mut::<crate::sim_app::FixedCommands>() {
        fixed.set_bridged(true);
    }
    let bridged_ids: HashSet<ComponentId> = side.ids.iter().copied().collect();
    let mut coverage = BridgeCoverage {
        already_built: setup.already_built,
        bridged: side.halves.iter().map(|h| h.name().to_owned()).collect(),
        ..default()
    };
    // `Children` follows `ChildOf` (its hooks keep it).
    let mut covered = bridged_ids.clone();
    if world
        .components()
        .component_id::<ChildOf>()
        .is_some_and(|id| bridged_ids.contains(&id))
    {
        covered.insert(world.register_component::<Children>());
    }
    // The shell's own resources, and Bevy's message registry: its fixed-step
    // signal is sent by the frame after each step (`frame_apply`).
    for id in [
        // Bevy's default query filters (every query has them).
        world.register_component::<bevy::ecs::entity_disabling::Disabled>(),
        world.register_component::<IsResource>(),
        world.register_component::<MessageRegistry>(),
        world.register_component::<crate::sim_app::SimClock>(),
        world.register_component::<crate::sim_app::SimLinks>(),
        world.register_component::<crate::sim_app::SimOutbox>(),
        world.register_component::<crate::sim_app::FrameCommands>(),
        world.register_component::<SimFixedOrder>(),
    ] {
        covered.insert(id);
    }
    // The simulation's writes to the frame's data go back by its own queue.
    world.insert_resource(crate::sim_app::FrameCommands::bridged());
    world.insert_resource(SimFixedOrder(setup.fixed_order));
    // Bevy's own bookkeeping of a changed hierarchy is not reported.
    let mut unbridged_seen = covered.clone();
    unbridged_seen
        .insert(world.register_component::<bevy::transform::components::TransformTreeChanged>());
    world.insert_resource(SimBridge {
        side,
        pool: IdPool::default(),
        archetypes_seen: 0,
        unbridged_seen,
    });
    // Build the moved schedules now, to read what their systems touch.
    // The step runs its systems one at a time on the simulation's own
    // thread, as the original's step runs on its thread, off the task pool
    // the frame's systems and the renderer share (behind the bridge the
    // shared pool made a step wait for the frame's work: exanima01 29.5 ms
    // a step on the pool, 18.7 on its own; every body's every tick the same
    // in lockstep). `EXA_SIM_EXECUTOR=multi` puts the pool back (a debug
    // aid).
    let single = !std::env::var("EXA_SIM_EXECUTOR").is_ok_and(|v| v == "multi");
    for mut schedule in setup.schedules {
        let label = schedule.label();
        coverage.schedules.push(format!("{label:?}"));
        if single {
            schedule.set_executor(bevy::ecs::schedule::SingleThreadedExecutor::new());
        }
        // The conditions first: the build moves them out of the graph.
        schedule.graph_mut().initialize(world);
        read_condition_access(world, &schedule, &covered, &mut coverage);
        match schedule.initialize(world) {
            Err(e) => error!("simulation bridge: {label:?} did not build: {e}"),
            Ok(_) => read_access(world, &schedule, &covered, &mut coverage),
        }
        world.resource_mut::<Schedules>().insert(schedule);
    }
    coverage
}

/// Adds what `schedule`'s systems touch to `coverage`.
fn read_access(
    world: &World,
    schedule: &Schedule,
    covered: &HashSet<ComponentId>,
    coverage: &mut BridgeCoverage,
) {
    let label = schedule.label();
    let Ok(systems) = schedule.systems_with_access() else {
        return;
    };
    for (_, system) in systems {
        coverage.systems += 1;
        let name = format!("{label:?}: {}", system.system().name());
        note_access(world, name, system.access(), covered, coverage);
    }
}

/// Adds what the run conditions of `schedule`'s systems and sets read (a
/// condition on data no system reads, such as `resource_exists`, would
/// quietly be false). Read from the graph after it initialized them,
/// before the build moves them out.
fn read_condition_access(
    world: &World,
    schedule: &Schedule,
    covered: &HashSet<ComponentId>,
    coverage: &mut BridgeCoverage,
) {
    let label = schedule.label();
    let graph = schedule.graph();
    let systems = graph
        .systems
        .iter()
        .map(|(_, s, c)| (s.name().to_string(), c));
    let sets = graph
        .system_sets
        .iter()
        .map(|(_, s, c)| (format!("{s:?}"), c));
    for (owner, conditions) in systems.chain(sets) {
        for condition in conditions {
            let name = format!(
                "{label:?}: condition {} of {owner}",
                condition.condition.name()
            );
            note_access(world, name, &condition.access, covered, coverage);
        }
    }
}

fn note_access(
    world: &World,
    name: String,
    access: &bevy::ecs::system::SystemAccess,
    covered: &HashSet<ComponentId>,
    coverage: &mut BridgeCoverage,
) {
    if access.is_exclusive() {
        coverage.whole_world.insert(name);
        return;
    }
    let set = access.to_filtered_access_set();
    let Ok(kinds) = set.combined_access().try_iter_access() else {
        coverage.whole_world.insert(name);
        return;
    };
    // Query filters (`With`, `Without`) are not in the combined access but
    // decide which entities match: a marker the bridge does not carry would
    // match nothing (or everything) in the simulation's world.
    let filters: Vec<ComponentId> = set
        .filtered_accesses()
        .iter()
        .flat_map(|f| f.with_filters().chain(f.without_filters()))
        .collect();
    let ids = kinds
        .map(|kind| match kind {
            ComponentAccessKind::Shared(id)
            | ComponentAccessKind::Exclusive(id)
            | ComponentAccessKind::Archetypal(id) => id,
        })
        .chain(filters);
    for id in ids {
        if !covered.contains(&id) {
            let type_name = world
                .components()
                .get_name(id)
                .map_or_else(|| format!("{id:?}"), |n| n.to_string());
            coverage.missing.insert(type_name);
        }
    }
}

/// Simulation side, after its schedules: gathers its changes for the frame,
/// counts the reserved ids it used, and clears its removal buffers.
pub fn sim_gather(world: &mut World) -> (SidePacket, u64, usize) {
    let started = Instant::now();
    let step = world
        .get_resource::<crate::sim_app::SimClock>()
        .map_or(0, |c| c.tick);
    let (packet, pool) = world.resource_scope(|world, mut bridge: Mut<SimBridge>| {
        let bridge = &mut *bridge;
        report_unbridged_components(world, bridge);
        bridge.side.step = step;
        let packet = bridge.side.gather(world);
        // Reserved ids now spawned belong to the simulation's entities;
        // ids it freed this step go back to the pool.
        bridge
            .pool
            .free
            .retain(|e| !world.entities().contains_spawned(*e));
        for &e in &packet.despawned {
            let freed = world.entities().resolve_from_index(e.index());
            bridge.pool.free.insert(freed);
        }
        (packet, bridge.pool.len())
    });
    world.clear_trackers();
    (packet, started.elapsed().as_micros() as u64, pool)
}

/// Warns once for each component the simulation's entities carry that the
/// bridge does not (one its systems insert by command and never read, which
/// the frame would then never see).
fn report_unbridged_components(world: &World, bridge: &mut SimBridge) {
    let archetypes = world.archetypes();
    if archetypes.len() == bridge.archetypes_seen {
        return;
    }
    bridge.archetypes_seen = archetypes.len();
    let resource = world.components().component_id::<IsResource>();
    // One-shot systems (the frame's posted writes) are entities too.
    let system = world
        .components()
        .component_id::<bevy::ecs::system::SystemIdMarker>();
    for archetype in archetypes.iter() {
        if resource.is_some_and(|r| archetype.contains(r))
            || system.is_some_and(|s| archetype.contains(s))
        {
            continue;
        }
        for &id in archetype.components() {
            if bridge.unbridged_seen.insert(id) {
                let name_of = |id: ComponentId| {
                    world
                        .components()
                        .get_name(id)
                        .map_or_else(|| format!("{id:?}"), |n| n.to_string())
                };
                let with: Vec<String> =
                    archetype.components().iter().map(|&c| name_of(c)).collect();
                warn!(
                    "simulation bridge: the simulation's entities carry {}, which is not bridged (with {})",
                    name_of(id),
                    with.join(", ")
                );
            }
        }
    }
}

/// Frame side, after the step: applies the simulation's changes, keeps the
/// cost, and tops the simulation's id pool up for the next step.
pub fn frame_apply(world: &mut World, out: BridgeOut) {
    let started = Instant::now();
    world.resource_scope(|world, mut bridge: Mut<FrameBridge>| {
        let spawned = out
            .packet
            .touched
            .iter()
            .filter(|&&e| !world.entities().contains_spawned(e))
            .count();
        let despawned = out.packet.despawned.len();
        let values = out.packet.values;
        if let Some(side) = bridge.side.as_mut() {
            side.apply(world, out.packet, false);
        }
        let cost = &mut bridge.cost;
        cost.steps += 1;
        cost.values_out += values as u64;
        cost.spawned_out += spawned as u64;
        cost.despawned_out += despawned as u64;
        cost.apply_in_us += out.apply_us;
        cost.gather_out_us += out.gather_us;
        cost.apply_out_us += started.elapsed().as_micros() as u64;
        if let Some(coverage) = out.coverage {
            bridge.coverage = Some(coverage);
        }
        bridge.pool_left = out.pool;
    });
    // The frame's message update waits for a fixed step to have run (Bevy's
    // signal system ran in `FixedPostUpdate`, which moved).
    if let Some(mut registry) = world.get_resource_mut::<MessageRegistry>()
        && registry.should_update != ShouldUpdateMessages::Always
    {
        registry.should_update = ShouldUpdateMessages::Ready;
    }
}

/// Warns once for each system that fails (an incomplete bridge's).
fn warn_once(error: bevy::ecs::error::BevyError, ctx: bevy::ecs::error::ErrorContext) {
    static SEEN: std::sync::Mutex<Option<HashSet<String>>> = std::sync::Mutex::new(None);
    let name = ctx.name().to_string();
    let mut seen = SEEN.lock().unwrap_or_else(|e| e.into_inner());
    if seen.get_or_insert_with(HashSet::new).insert(name.clone()) {
        warn!("simulation bridge: {name} skipped: {error}");
    }
}

fn report_coverage(c: &BridgeCoverage) {
    info!(
        "simulation bridge: moved {:?} ({} systems), {} types bridged, {} whole-world systems",
        c.schedules,
        c.systems,
        c.bridged.len(),
        c.whole_world.len()
    );
    if !c.already_built.is_empty() {
        error!(
            "simulation bridge: {:?} had already run on the frame's world and stay there",
            c.already_built
        );
    }
    if !c.missing.is_empty() {
        warn!(
            "simulation bridge: {} types the moved systems touch are not bridged: {:?}",
            c.missing.len(),
            c.missing
        );
    }
}

#[path = "sim_bridge_audit.rs"]
pub mod audit;

#[cfg(test)]
#[path = "sim_bridge_test.rs"]
mod tests;
