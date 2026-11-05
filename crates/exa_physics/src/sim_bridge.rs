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
    /// After an apply: forgets the removals and messages the apply itself
    /// made, and marks the tick changes are measured from.
    fn settle(&mut self, world: &mut World, since: Tick);
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
}

/// What an apply needs to know besides the packet.
#[derive(Debug, Clone, Copy)]
pub struct ApplyCtx {
    /// The receiving world is the simulation's.
    pub sim: bool,
    /// The frame updated its messages since the last step: the
    /// simulation's messages are updated too (Bevy's double buffer).
    pub update_messages: bool,
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

/// Every entity's value of a bridged component (not resources).
type ValueQuery<D> = QueryState<(Entity, D), Without<IsResource>>;

/// A component copied by clone when it changed.
pub struct CopyComponent<T: Component + Clone> {
    assign: fn(&mut EntityWorldMut, T),
    refs: fn(&T) -> Option<Entity>,
    _t: PhantomData<fn() -> T>,
}

struct CopyComponentHalf<T: Component + Clone> {
    assign: fn(&mut EntityWorldMut, T),
    refs: fn(&T) -> Option<Entity>,
    id: Option<ComponentId>,
    query: Option<ValueQuery<Ref<'static, T>>>,
    removals: Removals,
    since: Tick,
}

/// Changed values and removals of one component.
struct ComponentPacket<T> {
    removed: Vec<Entity>,
    values: Vec<(Entity, T, bool)>,
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
            id: None,
            query: None,
            removals: Removals::new(),
            since: Tick::new(0),
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

impl<T: Component + Clone> BridgeHalf for CopyComponentHalf<T> {
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
            .iter(world)
            .filter(|(_, v)| all || v.last_changed().is_newer_than(since, now))
            .map(|(e, v)| (e, T::clone(&v), true))
            .collect();
        let seen = self.removals.take();
        let removed = live_removals::<T>(world, &seen);
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

    fn apply(&mut self, world: &mut World, packet: Packet, _: ApplyCtx) -> usize {
        let Ok(packet) = packet.downcast::<ComponentPacket<T>>() else {
            return 0;
        };
        apply_removals::<T>(world, &packet.removed);
        let n = packet.values.len();
        for (e, value, _) in packet.values {
            if let Ok(mut entity) = world.get_entity_mut(e) {
                (self.assign)(&mut entity, value);
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

    fn apply(&mut self, world: &mut World, packet: Packet, _: ApplyCtx) -> usize {
        let Ok(packet) = packet.downcast::<ComponentPacket<T>>() else {
            return 0;
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
/// A resource lent for the step.
pub struct LendResource<T>(PhantomData<fn() -> T>);

struct ResourceHalf<T: Resource + Component<Mutability = Mutable>> {
    lend: bool,
    /// The value to send: a clone, or (lent) the value itself, swapped for
    /// the spare.
    take: fn(&mut World, &mut Option<T>) -> Option<T>,
    /// A lent resource's place holder: the value an apply swapped out,
    /// swapped back in by the next gather (a default is built only once).
    spare: Option<T>,
    id: Option<ComponentId>,
    present: bool,
    since: Tick,
}

/// A resource's value, or its removal.
enum ResourcePacket<T> {
    Unchanged,
    Removed,
    Value(T, bool),
}

impl<T: Resource + Component<Mutability = Mutable> + Clone> BridgeType for CopyResource<T> {
    fn type_id(&self) -> TypeId {
        TypeId::of::<T>()
    }

    fn half(&self) -> Box<dyn BridgeHalf> {
        Box::new(ResourceHalf::<T> {
            lend: false,
            take: |world, _| world.get_resource::<T>().cloned(),
            spare: None,
            id: None,
            present: false,
            since: Tick::new(0),
        })
    }
}

impl<T: Resource + Component<Mutability = Mutable> + Default> BridgeType for LendResource<T> {
    fn type_id(&self) -> TypeId {
        TypeId::of::<T>()
    }

    fn half(&self) -> Box<dyn BridgeHalf> {
        Box::new(ResourceHalf::<T> {
            lend: true,
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

    fn drain_removed(&mut self, _: &World) {}

    fn gather(&mut self, world: &mut World, ctx: GatherCtx) -> Gathered {
        let all = ctx.all;
        let now = world.change_tick();
        let changed = world
            .get_resource_ref::<T>()
            .map(|r| all || r.last_changed().is_newer_than(self.since, now));
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

    fn apply(&mut self, world: &mut World, packet: Packet, _: ApplyCtx) -> usize {
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
                match world.get_resource_mut::<T>() {
                    Some(mut current) if self.lend => {
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

    fn settle(&mut self, world: &mut World, since: Tick) {
        self.present = world.contains_resource::<T>();
        self.since = since;
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
        id
    }

    fn drain_removed(&mut self, _: &World) {}

    fn gather(&mut self, world: &mut World, ctx: GatherCtx) -> Gathered {
        let now = world.change_tick();
        let since = self.since;
        let Some(query) = self.query.as_mut() else {
            return Gathered::empty();
        };
        let orders: Vec<(Entity, Vec<Entity>)> = query
            .iter(world)
            .filter(|(_, c)| ctx.all || c.last_changed().is_newer_than(since, now))
            .map(|(e, c)| (e, c.to_vec()))
            .collect();
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
            let at: bevy::ecs::entity::EntityHashMap<usize> =
                order.iter().enumerate().map(|(i, &e)| (e, i)).collect();
            children.sort_by_key(|e| at.get(e).copied().unwrap_or(usize::MAX));
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

// ---------------------------------------------------------------- registry

/// The bridged types, registered on the frame's app.
#[derive(Resource, Default)]
pub struct SimBridgeTypes {
    types: Vec<Box<dyn BridgeType>>,
    seen: HashSet<TypeId>,
}

impl SimBridgeTypes {
    fn add(&mut self, t: impl BridgeType) {
        if self.seen.insert(t.type_id()) {
            self.types.push(Box::new(t));
        }
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
    fn bridge_copy<T: Component<Mutability = Mutable> + Clone>(&mut self) -> &mut Self;
    /// A relationship such as `ChildOf` (immutable) copied by re-inserting
    /// it when it changed; its target is spawned on the other side too.
    fn bridge_relationship<T: Relationship + Clone>(&mut self) -> &mut Self;
    /// A component lent to the simulation for each step.
    fn bridge_lend<T: Component<Mutability = Mutable> + Default>(&mut self) -> &mut Self;
    /// A resource copied when it changed.
    fn bridge_copy_resource<T: Resource + Component<Mutability = Mutable> + Clone>(
        &mut self,
    ) -> &mut Self;
    /// A resource lent to the simulation for each step.
    fn bridge_lend_resource<T: Resource + Component<Mutability = Mutable> + Default>(
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
}

impl SimBridgeAppExt for App {
    fn bridge_copy<T: Component<Mutability = Mutable> + Clone>(&mut self) -> &mut Self {
        add_type(
            self,
            CopyComponent::<T> {
                assign: assign_mutable::<T>,
                refs: |_| None,
                _t: PhantomData,
            },
        )
    }

    fn bridge_relationship<T: Relationship + Clone>(&mut self) -> &mut Self {
        add_type(
            self,
            CopyComponent::<T> {
                assign: assign_by_insert::<T>,
                refs: |r| Some(r.get()),
                _t: PhantomData,
            },
        )
    }

    fn bridge_lend<T: Component<Mutability = Mutable> + Default>(&mut self) -> &mut Self {
        add_type(self, LendComponent::<T>(PhantomData))
    }

    fn bridge_copy_resource<T: Resource + Component<Mutability = Mutable> + Clone>(
        &mut self,
    ) -> &mut Self {
        add_type(self, CopyResource::<T>(PhantomData))
    }

    fn bridge_lend_resource<T: Resource + Component<Mutability = Mutable> + Default>(
        &mut self,
    ) -> &mut Self {
        add_type(self, LendResource::<T>(PhantomData))
    }

    fn bridge_message<M: Message + Clone>(&mut self) -> &mut Self {
        add_type(self, ForwardMessage::<M>(PhantomData))
    }

    fn bridge_children_order(&mut self) -> &mut Self {
        add_type(self, ChildrenOrder)
    }

    fn bridge_sim_only<T: Component>(&mut self) -> &mut Self {
        add_type(self, SimOnly::<T>(PhantomData))
    }

    fn bridge_move_resource<T: Resource>(&mut self) -> &mut Self {
        add_type(self, MoveResource::<T>(PhantomData))
    }
}

fn add_type(app: &mut App, t: impl BridgeType) -> &mut App {
    app.world_mut()
        .get_resource_or_init::<SimBridgeTypes>()
        .add(t);
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
    update_messages: bool,
    pub values: usize,
}

/// One world's halves and their bookkeeping.
pub struct BridgeSide {
    halves: Vec<Box<dyn BridgeHalf>>,
    ids: Vec<ComponentId>,
    first: bool,
    sim: bool,
    /// Per half: values gathered, values applied, nanoseconds (both).
    stats: Vec<(u64, u64, u64)>,
    gathers: u64,
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
            stats: Vec::new(),
            gathers: 0,
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
    }

    fn drain_removed(&mut self, world: &World) {
        for half in &mut self.halves {
            half.drain_removed(world);
        }
    }

    fn gather(&mut self, world: &mut World) -> SidePacket {
        let ctx = GatherCtx {
            all: std::mem::take(&mut self.first) || copy_everything(),
            sim: self.sim,
        };
        let mut packets = Vec::with_capacity(self.halves.len());
        let mut touched = EntityHashSet::default();
        let mut despawned = EntityHashSet::default();
        let mut values = 0;
        self.stats.resize(self.halves.len(), (0, 0, 0));
        for (half, stat) in self.halves.iter_mut().zip(&mut self.stats) {
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
            packets.push(g.packet);
        }
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
            update_messages: false,
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
            ensure_spawned(world, e);
        }
        let ctx = ApplyCtx {
            sim,
            update_messages: packet.update_messages,
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
    /// Reserved ids the simulation had left after the last step.
    pool_left: usize,
    pub cost: BridgeCost,
    pub coverage: Option<BridgeCoverage>,
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

/// Runs the moved fixed-step schedules in order (in `SimStep`).
pub fn run_fixed_order(world: &mut World) {
    let Some(order) = world.get_resource::<SimFixedOrder>().cloned() else {
        return;
    };
    for label in order.0 {
        let _ = world.try_run_schedule(label);
    }
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
        app.init_resource::<SimBridgeTypes>()
            .insert_resource(FrameBridge {
                extra: self.extra.clone(),
                fixed_order: Vec::new(),
                side: None,
                frames: 0,
                last_frames: 0,
                pool_left: 0,
                cost: BridgeCost::default(),
                coverage: None,
            })
            .add_systems(First, count_frame)
            .add_systems(Last, drain_frame_removals);
        bridge_bevy_types(app);
    }
}

/// Counts the frame's updates; on the first, before any fixed step, reads
/// the fixed step's order (inside `FixedMain` it is out of the world).
/// Bevy's own types the fixed step reads and writes: the hierarchy (its
/// `Children` follow `ChildOf` by its hooks), transforms, names and clocks.
pub fn bridge_bevy_types(app: &mut App) {
    app.bridge_relationship::<ChildOf>()
        .bridge_children_order()
        .bridge_copy::<Transform>()
        .bridge_copy::<GlobalTransform>()
        .bridge_copy::<Name>()
        .bridge_copy_resource::<Time>()
        .bridge_copy_resource::<Time<Fixed>>()
        .bridge_copy_resource::<Time<Virtual>>()
        .bridge_copy_resource::<Time<Real>>()
        .bridge_lend_resource::<bevy::gizmos::gizmos::GizmoStorage<bevy::gizmos::aabb::AabbGizmoConfigGroup, ()>>()
        .bridge_lend_resource::<bevy::gizmos::gizmos::GizmoStorage<bevy::gizmos::aabb::AabbGizmoConfigGroup, bevy::time::Fixed>>()
        .bridge_lend_resource::<bevy::gizmos::gizmos::GizmoStorage<bevy::gizmos::config::DefaultGizmoConfigGroup, ()>>()
        .bridge_lend_resource::<bevy::gizmos::gizmos::GizmoStorage<bevy::gizmos::config::DefaultGizmoConfigGroup, bevy::time::Fixed>>()
        .bridge_lend_resource::<bevy::gizmos::gizmos::GizmoStorage<bevy::gizmos::frustum::FrustumGizmoConfigGroup, ()>>()
        .bridge_lend_resource::<bevy::gizmos::gizmos::GizmoStorage<bevy::gizmos::frustum::FrustumGizmoConfigGroup, bevy::time::Fixed>>()
        .bridge_lend_resource::<bevy::gizmos::gizmos::GizmoStorage<bevy::gizmos::skinned_mesh_bounds::SkinnedMeshBoundsGizmoConfigGroup, ()>>()
        .bridge_lend_resource::<bevy::gizmos::gizmos::GizmoStorage<bevy::gizmos::skinned_mesh_bounds::SkinnedMeshBoundsGizmoConfigGroup, bevy::time::Fixed>>()
        .bridge_lend_resource::<bevy::gizmos::gizmos::GizmoStorage<bevy::light::gizmos::LightGizmoConfigGroup, ()>>()
        .bridge_lend_resource::<bevy::gizmos::gizmos::GizmoStorage<bevy::light::gizmos::LightGizmoConfigGroup, bevy::time::Fixed>>();
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
            side.init(world);
            bridge.side = Some(side);
            if bridge.fixed_order.is_empty() {
                // No frame has run its `First` (a test running `FixedMain`).
                bridge.fixed_order = FixedMainScheduleOrder::default().labels;
            }
            Some(take_schedules(
                world,
                &bridge.fixed_order,
                &bridge.extra,
                sim_halves,
            ))
        } else {
            None
        };
        let side = bridge.side.as_mut()?;
        let mut packet = side.gather(world);
        packet.update_messages = bridge.frames != bridge.last_frames;
        bridge.last_frames = bridge.frames;
        if setup.is_some() || bridge.pool_left < POOL_LOW {
            packet.reserved = reserve_ids(world, POOL_BATCH);
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
    }
}

/// Simulation side, before its commands and schedules: installs the setup on
/// the first step, then applies the frame's changes.
pub fn sim_apply(world: &mut World, bridge_in: BridgeIn) -> (Option<BridgeCoverage>, u64) {
    let started = Instant::now();
    let mut coverage = None;
    if let Some(setup) = bridge_in.setup {
        let c = install(world, setup);
        report_coverage(&c);
        if !c.missing.is_empty() {
            // An incomplete bridge: a system whose data is missing is
            // skipped with one warning instead of stopping the game.
            world.insert_resource(bevy::ecs::error::FallbackErrorHandler(warn_once));
        }
        coverage = Some(c);
    }
    let BridgeIn { packet, .. } = bridge_in;
    world.resource_scope(|world, mut bridge: Mut<SimBridge>| {
        let reserved = packet.reserved.clone();
        if !reserved.is_empty() {
            world.entity_allocator_mut().free_many(&reserved);
            bridge.pool.free.extend(reserved);
        }
        bridge.side.apply(world, packet, true);
    });
    (coverage, started.elapsed().as_micros() as u64)
}

fn install(world: &mut World, setup: BridgeSetup) -> BridgeCoverage {
    let mut side = BridgeSide::new(setup.halves, false);
    side.init(world);
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
        world.register_component::<MessageRegistry>(),
        world.register_component::<crate::sim_app::SimClock>(),
        world.register_component::<crate::sim_app::SimLinks>(),
        world.register_component::<crate::sim_app::SimOutbox>(),
        world.register_component::<SimFixedOrder>(),
    ] {
        covered.insert(id);
    }
    world.insert_resource(SimFixedOrder(setup.fixed_order));
    world.insert_resource(SimBridge {
        side,
        pool: IdPool::default(),
        archetypes_seen: 0,
        unbridged_seen: covered.clone(),
    });
    // Build the moved schedules now, to read what their systems touch.
    for mut schedule in setup.schedules {
        let label = schedule.label();
        coverage.schedules.push(format!("{label:?}"));
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
    for kind in kinds {
        let id = match kind {
            ComponentAccessKind::Shared(id)
            | ComponentAccessKind::Exclusive(id)
            | ComponentAccessKind::Archetypal(id) => id,
        };
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
    let (packet, pool) = world.resource_scope(|world, mut bridge: Mut<SimBridge>| {
        let bridge = &mut *bridge;
        report_unbridged_components(world, bridge);
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
    for archetype in archetypes.iter() {
        if resource.is_some_and(|r| archetype.contains(r)) {
            continue;
        }
        for &id in archetype.components() {
            if bridge.unbridged_seen.insert(id) {
                let name = world
                    .components()
                    .get_name(id)
                    .map_or_else(|| format!("{id:?}"), |n| n.to_string());
                warn!(
                    "simulation bridge: the simulation's entities carry {name}, which is not bridged"
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

#[cfg(test)]
#[path = "sim_bridge_test.rs"]
mod tests;
