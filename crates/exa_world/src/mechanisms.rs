//! The door and operative records a placed object carries, as the
//! mechanism rules in `exa_physics` take them.
//!
//! A placed object with a door section or an operative section keeps both,
//! under its placed id (tag bits included, as signals and door links name
//! objects), in a [`MechanismRecords`] component. A signal's dispatch reads
//! exactly these two records ([`exa_physics::mechanism::dispatch`]); a
//! lever or push surface builds its state from its operative record. An
//! object without a model (most relays and jobs) is not spawned, so its
//! records stay in the level's database for a dispatcher to look up by id.
//!
//! A door, lever or plate is a body hung from the world by its links: the
//! hinge pins and swing stop of a door, the axle pins and arm of a lever,
//! the slider links of a plate. A hinged door's leaf, a lever's arm and a
//! push surface register so and run their rules (`exa_physics::body`'s
//! `DoorBody`, `LeverBody` and `PlateBody`, set up from the records and the
//! placement's word, [`crate::bodies::Placement`]); a sliding door's leaf
//! does so only when the loader is asked for it (`SlideBody`), and stays
//! static level geometry otherwise.
//!
//! **Signals.** A lever's or plate's report sends a
//! [`MechanismSignal`] to each target it names; [`route_signals`] finds the
//! placed object whose id is the target and runs what its records ask
//! ([`MechanismRecords::dispatch`]): a door order (open, close and lock,
//! toggle, [`exa_physics::mechanism::open_doors`] and its siblings) on
//! every door body under the object's node, each woken, or a class 5 push
//! on the push surface under it. A class 6 fire job spawns a fire at the
//! node at the signalled size. A light job stores the signal's low bit, powers
//! the electric emitter under the node, and for an odd value with none there
//! starts the level device's arc. A relay forwards through the placed records:
//! a broadcast to every tagged listener, a pair to both of its targets, and a
//! combiner by the source record's bit, a sequence by the count in its value
//! word. A target with no record, or no body of the right kind under it (a
//! static sliding door), does nothing.
//!
//! Evidence: the object database's door and operative sections
//! (`docs/formats/OTHER.md`) and the records the original's door, lever
//! and push-surface set-up and the signal dispatch read: the same fields
//! in the same order.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use exa_formats::objdb::{self, Item};
use exa_physics::body::{BodyTicks, DoorBody, PlateBody, Ragdoll, SlideBody, WAKE_COUNT};
use exa_physics::mechanism::{Dispatch, DoorOrder, LockSounded, MechanismSignal};
use exa_physics::{door, lever, mechanism, plate, relay};

/// Routes the mechanisms' signals to their targets. Added by
/// [`crate::WorldPlugin`].
pub struct MechanismPlugin;

impl Plugin for MechanismPlugin {
    fn build(&self, app: &mut App) {
        bridge_mechanism_types(app);
        if !app.world().contains_resource::<Messages<MechanismSignal>>() {
            app.add_message::<MechanismSignal>();
        }
        if !app.world().contains_resource::<Messages<LockSounded>>() {
            app.add_message::<LockSounded>();
        }
        app.add_message::<OpenDoorsUnder>()
            .add_message::<LatchDoorsAt>()
            .add_message::<RelayForward>()
            .add_systems(
                FixedUpdate,
                (latch_doors_at, open_doors_under, route_signals)
                    .chain()
                    .in_set(exa_physics::body::AfterSolver::Level)
                    .before(send_relay_forwards),
            )
            // After the physics' follow-up (the levers' step sends them).
            .add_systems(
                FixedUpdate,
                send_relay_forwards.after(exa_physics::body::AfterSolver::Physics),
            );
        if std::env::var_os("EXA_MECHANISM_LIST").is_some() {
            app.add_systems(
                Update,
                list_mechanisms.in_set(exa_physics::FramePhase::Capture),
            );
        }
    }
}

/// `EXA_MECHANISM_LIST=1` (debug): once the level's sliding leaves stand
/// (with the hinged leaves' angles and words, which a save's load check
/// compares),
/// logs each one (its placed object's id, where its leaf is, its fraction
/// and state bits) and each lever (its id, where it is, its two targets),
/// so a scene can be framed on a lever and the gate it drives; then, every
/// 30 frames, each awake sliding leaf's fraction and place (`slide watch`).
fn list_mechanisms(
    mut listed: Local<bool>,
    mut frame: Local<u32>,
    slides: Query<(Entity, &SlideBody, &Ragdoll)>,
    levers: Query<(Entity, &exa_physics::body::LeverBody)>,
    doors: Query<(Entity, &exa_physics::body::DoorBody, &Ragdoll)>,
    parents: Query<&ChildOf>,
    records: Query<&MechanismRecords>,
) {
    let placed_id = |e: Entity| -> String {
        let mut up = Some(e);
        while let Some(node) = up {
            if let Ok(r) = records.get(node) {
                return format!("{:#x}", r.id);
            }
            up = parents.get(node).ok().map(ChildOf::parent);
        }
        "no record".to_string()
    };
    if !*listed {
        if slides.is_empty() {
            return;
        }
        *listed = true;
        for (e, glue, body) in &slides {
            let at = Vec3::from(body.parts[0].transform().translation);
            info!(
                "mechanism list: sliding leaf {e:?} of {} at ({:.2}, {:.2}, {:.2}) m, \
                 fraction {:.3} ({}), closed {}, latched {}, fast {}, locked {}",
                placed_id(e),
                at.x,
                at.y,
                at.z,
                glue.fraction(body),
                // The shipped rails run from the arch (fraction 0) into the
                // wall: the door's "closed" is the original's flag for the
                // rail's far end, where the arch is clear.
                if glue.at_placed_pose(body) {
                    "nearer its placed pose, in its arch"
                } else {
                    "nearer its rail's far end, clear of its arch"
                },
                glue.door.closed,
                glue.door.latched,
                glue.door.fast,
                glue.door.locked
            );
        }
        for (e, lever) in &levers {
            let at = lever.origin();
            info!(
                "mechanism list: lever {e:?} {:#x} at ({:.2}, {:.2}, {:.2}) m signals {:#x} and {:#x}, word {:#x}",
                lever.id,
                at.x,
                at.y,
                at.z,
                lever.record.target,
                lever.record.second,
                lever.lever.word
            );
        }
        for (e, glue, body) in &doors {
            let at = Vec3::from(body.parts[0].transform().translation);
            info!(
                "mechanism list: hinged leaf {e:?} of {} at ({:.2}, {:.2}, {:.2}) m, angle {:.3} (shut at {:.3}), closed {}, latched {}, word {:#x}",
                placed_id(e),
                at.x,
                at.y,
                at.z,
                glue.door.angle,
                glue.door.shape.closed(),
                glue.door.closed,
                glue.door.latched,
                glue.door.stored()
            );
        }
        return;
    }
    *frame += 1;
    if !(*frame).is_multiple_of(30) {
        return;
    }
    for (e, glue, body) in &slides {
        if body.asleep() {
            continue;
        }
        let at = Vec3::from(body.parts[0].transform().translation);
        info!(
            "slide watch: leaf {e:?} of {} fraction {:.3} at ({:.2}, {:.2}, {:.2}) m, closed {}",
            placed_id(e),
            glue.fraction(body),
            at.x,
            at.y,
            at.z,
            glue.door.closed
        );
    }
}

/// The data this plugin's fixed-step systems touch, for the simulation's
/// bridge (`exa_physics::sim_bridge`).
fn bridge_mechanism_types(app: &mut App) {
    use exa_physics::sim_bridge::SimBridgeAppExt;
    app.bridge_message::<exa_physics::mechanism::LockSounded>()
        .bridge_message::<crate::mechanisms::OpenDoorsUnder>()
        .bridge_message::<crate::mechanisms::LatchDoorsAt>()
        .bridge_message::<crate::mechanisms::RelayForward>()
        .bridge_snapshot::<exa_physics::body::plate_body::PlateBody>()
        .bridge_snapshot::<exa_physics::body::slide_body::SlideBody>()
        .bridge_copy::<crate::arc::ElectricDevice>()
        .bridge_copy::<crate::mechanisms::MechanismRecords>();
}

/// An order to latch or unlatch the doors of the first placed door object
/// standing within ten centimetres of a spot, on x and on z (`fabs` of
/// each difference below 10 cm), as the arena does with the door at the
/// physician's hub spot. The object's word takes the value in bit 0 and
/// every door body under it its latch ([`mechanism::latch_doors`]).
#[derive(Message, Debug, Clone, Copy, PartialEq)]
pub struct LatchDoorsAt {
    /// The spot's x and z, in centimetres.
    pub x_cm: f32,
    pub z_cm: f32,
    pub latched: bool,
}

/// How many steps an order waits for its door's body to be registered (the
/// loader builds the bodies a few frames after the level's nodes) before it
/// is dropped: four seconds.
const LATCH_WAIT_STEPS: u32 = 240;

/// Runs each [`LatchDoorsAt`]: the first placed object with a door record
/// within ten centimetres of the spot has the door bodies under it latched
/// or unlatched. An order whose object or door body is not there yet waits
/// for it ([`LATCH_WAIT_STEPS`]); one that never finds them does nothing.
pub fn latch_doors_at(
    mut orders: MessageReader<LatchDoorsAt>,
    mut waiting: Local<Vec<(LatchDoorsAt, u32)>>,
    objects: Query<(Entity, &MechanismRecords, &GlobalTransform)>,
    children: Query<&Children>,
    mut bodies: Query<MechanismParts>,
) {
    waiting.extend(orders.read().map(|order| (*order, 0)));
    if waiting.is_empty() {
        return;
    }
    let mut left = Vec::new();
    for (order, waited) in waiting.drain(..) {
        // The original walks the level's groups in order and takes the
        // first door object at the spot; the hub holds one there.
        let found = objects
            .iter()
            .filter(|(_, records, _)| records.door.is_some())
            .find(|(_, _, at)| {
                let at = at.translation() * 100.0;
                (at.x - order.x_cm).abs() < 10.0 && (at.z - order.z_cm).abs() < 10.0
            })
            .map(|(e, records, _)| (e, records.id));
        let latched = found.and_then(|(node, id)| {
            let nodes = under(node, &children);
            let mut doors: Vec<(Entity, door::Door)> = nodes
                .iter()
                .filter_map(|&e| {
                    let (_, hinged, sliding, _) = bodies.get(e).ok()?;
                    hinged
                        .map(|g| g.door)
                        .or(sliding.map(|g| g.door))
                        .map(|d| (e, d))
                })
                .collect();
            if doors.is_empty() {
                return None;
            }
            let word = doors[0].1.saved as u32;
            let mut leaves: Vec<door::Door> = doors.iter().map(|(_, d)| *d).collect();
            let word = mechanism::latch_doors(&mut leaves, word, order.latched);
            for ((e, d), mut changed) in doors.iter_mut().zip(leaves) {
                changed.saved = (changed.saved & !1) | (word & 1) as i32;
                *d = changed;
                // The original only writes the latch byte; the body is
                // not woken.
                if let Ok((_, hinged, sliding, _)) = bodies.get_mut(*e) {
                    if let Some(mut glue) = hinged {
                        glue.door = changed;
                    } else if let Some(mut glue) = sliding {
                        glue.door = changed;
                    }
                }
            }
            Some((id, doors.len()))
        });
        match latched {
            Some((id, count)) => info!(
                "mechanism: {} the {count} door(s) of {id:#x} at ({}, {}) cm",
                if order.latched {
                    "latched"
                } else {
                    "unlatched"
                },
                order.x_cm,
                order.z_cm
            ),
            None if waited < LATCH_WAIT_STEPS => left.push((order, waited + 1)),
            None => info!(
                "mechanism: no door body at ({}, {}) cm to {}",
                order.x_cm,
                order.z_cm,
                if order.latched { "latch" } else { "unlatch" }
            ),
        }
    }
    *waiting = left;
}

/// An order to open the doors under a placed object's node, as the
/// original's "open the doors under a node" takes it straight from the
/// player's deferred use (a consumable's target word, 2100 ms after the
/// double click), without the signal's door-flag inversion. The first door
/// found under the node plays its `LkO` sound once ([`LockSounded`]).
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub struct OpenDoorsUnder {
    /// The placed object's database id, tag bits included.
    pub target: u32,
}

/// Runs each [`OpenDoorsUnder`]: the door bodies under the placed object
/// with that id are opened (latched or motorised ones) and woken, the
/// first of them, in the walk's order, sounding its lock once. An id no
/// placed object carries does nothing.
pub fn open_doors_under(
    mut orders: MessageReader<OpenDoorsUnder>,
    ticks: Option<Res<BodyTicks>>,
    placed: Query<(Entity, &crate::PlacedItem)>,
    children: Query<&Children>,
    mut bodies: Query<MechanismParts>,
    mut sounds: MessageWriter<LockSounded>,
) {
    let now = ticks.map_or(0, |t| t.millis() as i32);
    for order in orders.read() {
        let Some(node) = crate::object_entity(placed.iter(), order.target) else {
            info!("mechanism: no placed object {:#x} to open", order.target);
            continue;
        };
        let nodes = under(node, &children);
        sounds.write_batch(order_doors(DoorOrder::Open, &nodes, &mut bodies, now));
    }
}

/// The entities at and under `node`, the node first, parents before
/// children (bounded, so a cycle in bad input ends).
fn under(node: Entity, children: &Query<&Children>) -> Vec<Entity> {
    let mut found = vec![node];
    let mut k = 0;
    while k < found.len() && found.len() < 4096 {
        if let Ok(kids) = children.get(found[k]) {
            found.extend(kids.iter());
        }
        k += 1;
    }
    found
}

/// What [`route_signals`] reads and moves of the bodies under a target.
type MechanismParts = (
    &'static mut Ragdoll,
    Option<&'static mut DoorBody>,
    Option<&'static mut SlideBody>,
    Option<&'static mut PlateBody>,
);

/// Runs each mechanism signal on its target's records: a door order on the
/// door bodies under the target's node (each woken), a push on its push
/// surface, a fire job (a fire is spawned at the node), or a relay that
/// forwards to the listeners or targets its records name. A light job stores
/// the signal's low bit and powers the electric emitter under the node.
/// A relay's forwarded signal, written after [`route_signals`] has finished
/// reading the incoming queue.
#[derive(Message, Clone, Copy)]
pub(crate) struct RelayForward(MechanismSignal);

/// Writes the relays' forwards into the signal queue.
pub(crate) fn send_relay_forwards(
    mut forwards: MessageReader<RelayForward>,
    mut signals: MessageWriter<MechanismSignal>,
) {
    for forward in forwards.read() {
        signals.write(forward.0);
    }
}

/// The child list and the electric emitters a signal walks.
#[derive(SystemParam)]
struct NodeLinks<'w, 's> {
    children: Query<'w, 's, &'static Children>,
    devices: Query<'w, 's, &'static mut crate::arc::ElectricDevice>,
}

#[allow(private_interfaces, clippy::too_many_arguments)]
pub fn route_signals(
    mut signals: MessageReader<MechanismSignal>,
    mut commands: Commands,
    mut forwards: MessageWriter<RelayForward>,
    mut locks: MessageWriter<LockSounded>,
    ticks: Option<Res<BodyTicks>>,
    mut objects: Query<(Entity, &mut MechanismRecords)>,
    mut links: NodeLinks,
    mut bodies: Query<MechanismParts>,
) {
    let now = ticks.map_or(0, |t| t.millis() as i32);
    let incoming: Vec<MechanismSignal> = signals.read().copied().collect();
    for signal in &incoming {
        let Some((node, records)) = objects
            .iter()
            .find(|(_, r)| r.id == signal.target)
            .map(|(e, r)| (e, *r))
        else {
            info!(
                "mechanism: {:#x} signals {} to {:#x}, which has no record here",
                signal.source, signal.value, signal.target
            );
            continue;
        };
        let dispatch = records.dispatch(signal.value);
        info!(
            "mechanism: {:#x} signals {} to {:#x}: {:?}",
            signal.source, signal.value, signal.target, dispatch
        );
        let nodes = under(node, &links.children);
        match dispatch {
            Dispatch::Doors(order) => {
                locks.write_batch(order_doors(order, &nodes, &mut bodies, now));
            }
            Dispatch::Plate { pushed } => {
                for &e in &nodes {
                    if let Ok((mut body, _, _, Some(mut glue))) = bodies.get_mut(e) {
                        glue.push(&mut body, pushed);
                        break;
                    }
                }
            }
            Dispatch::Relay(kind) => {
                let sent = relay_forwards(kind, signal, records, &mut objects);
                for forward in sent {
                    forwards.write(RelayForward(MechanismSignal {
                        target: forward.target,
                        source: forward.source,
                        value: forward.value,
                    }));
                }
            }
            Dispatch::Job(mechanism::Job::Fire { size }) => {
                // A fire at the node's position, of the signalled size
                // (hundredths, or 2 for a zero word); the node's own lights
                // are left as they are.
                crate::fire::spawn_fire(&mut commands, node, mechanism::job_fire_size(size));
            }
            Dispatch::Job(mechanism::Job::Light { .. }) => {
                light_job(node, signal, &mut objects, &mut links, &mut commands);
            }
            Dispatch::None => {}
        }
    }
}

/// A light job. The signal's low bit is stored in the operative value word.
/// The first direct child that is an electric emitter is left alone when it
/// is already on, and otherwise has its life set to 10 and its state to 1.
/// With no emitter, an odd value starts the level device's arc as a child,
/// in the node's local space, along its third axis for the record's length.
fn light_job(
    node: Entity,
    signal: &MechanismSignal,
    objects: &mut Query<(Entity, &mut MechanismRecords)>,
    links: &mut NodeLinks,
    commands: &mut Commands,
) {
    let Some(fields) = ({
        let found = objects
            .iter_mut()
            .find(|(_, other)| other.id == signal.target);
        found.and_then(|(_, mut stored)| {
            let record = stored.operative.as_mut()?;
            if signal.value & 1 == 0 {
                record.value &= !1;
            } else {
                record.value |= 1;
            }
            Some(device_fields(record))
        })
    }) else {
        return;
    };
    if let Ok(kids) = links.children.get(node) {
        for child in kids.iter() {
            if let Ok(mut device) = links.devices.get_mut(child) {
                if device.state & 1 != 0 {
                    return;
                }
                device.life = 10;
                device.state = 1;
                return;
            }
        }
    }
    if !fields.active {
        return;
    }
    start_device_arc(commands, node, fields);
}

/// The device-arc fields an operative record holds: the value word's low bit,
/// the duration and length packed in the first target, the branch count in
/// the second, and the flag byte above the record's flags.
pub(crate) fn device_fields(record: &mechanism::Record) -> exa_render::electricity::DeviceFields {
    exa_render::electricity::DeviceFields {
        active: record.value & 1 != 0,
        duration: record.target as u16,
        length: (record.target >> 16) as u16,
        branches: record.second as i16,
        flags: (record.flags >> 8) as u8,
    }
}

/// What a placed fire record does once the scene's daylight scale is known.
/// Above the daylight threshold the node's own light and its direct
/// children's are put out. Otherwise a fire of the stored size is spawned,
/// lifted by the stored height when the node has no light of its own, and
/// the light it replaces is put out.
pub(crate) fn apply_load_fires(world: &mut World, daylight: f32) {
    let mut fires = Vec::new();
    let mut douse = Vec::new();
    let mut records = world.query::<(Entity, &MechanismRecords)>();
    for (entity, records) in records.iter(world) {
        let Some(operative) = records.operative else {
            continue;
        };
        match mechanism::level_setup(Some(&operative), daylight).setup {
            mechanism::Setup::Fire { size, lift } => {
                fires.push((
                    entity,
                    mechanism::fire_size(size),
                    mechanism::fire_lift(lift),
                ));
            }
            mechanism::Setup::Daylight => douse.push(entity),
            _ => {}
        }
    }
    drop(records);
    for entity in &douse {
        put_out_light(world, *entity);
    }
    let placed: Vec<(Entity, Vec3)> = fires
        .iter()
        .map(|(entity, _, lift)| {
            let shift = light_shift(world, *entity).unwrap_or(Vec3::new(0.0, *lift, 0.0));
            (*entity, shift)
        })
        .collect();
    let mut before = Vec::new();
    for (entity, _) in &placed {
        before.extend(direct_children(world, *entity));
    }
    let mut queue = bevy::ecs::world::CommandQueue::default();
    {
        let mut commands = Commands::new(&mut queue, world);
        for (entity, size, _) in &fires {
            crate::fire::spawn_fire(&mut commands, *entity, *size);
        }
    }
    queue.apply(world);
    for (entity, _, _) in &fires {
        put_out_light(world, *entity);
    }
    for (entity, shift) in placed {
        let spawned: Vec<Entity> = direct_children(world, entity)
            .into_iter()
            .filter(|child| !before.contains(child))
            .collect();
        for child in spawned {
            if let Some(mut transform) = world.get_mut::<Transform>(child) {
                transform.translation += shift;
            }
        }
    }
}

/// Where a fire sits when the node already has a light: that light's local
/// translation. `None` when the fire uses the record's lift instead.
fn light_shift(world: &mut World, entity: Entity) -> Option<Vec3> {
    if world
        .get::<crate::light_source::LightSource>(entity)
        .is_some()
    {
        return Some(Vec3::ZERO);
    }
    let child = direct_children(world, entity).into_iter().find(|child| {
        world
            .get::<crate::light_source::LightSource>(*child)
            .is_some()
    })?;
    Some(
        world
            .get::<Transform>(child)
            .map(|transform| transform.translation)
            .unwrap_or(Vec3::ZERO),
    )
}

/// Clears the intensity of the node's light and of each direct child's.
fn direct_children(world: &mut World, parent: Entity) -> Vec<Entity> {
    let mut query = world.query::<(Entity, &ChildOf)>();
    let mut found = Vec::new();
    for (entity, child_of) in query.iter(world) {
        if child_of.parent() == parent {
            found.push(entity);
        }
    }
    found
}

pub(crate) fn put_out_light(world: &mut World, entity: Entity) {
    let hosts = direct_children(world, entity);
    for host in std::iter::once(entity).chain(hosts) {
        if let Some(mut source) = world
            .entity_mut(host)
            .get_mut::<crate::light_source::LightSource>()
        {
            source.intensity = 0.0;
        }
    }
}

/// Starts the level device's arc as a child of `node`, in the node's local
/// space, along its third axis for the record's length. A fresh emitter is
/// off until a later signal catches it.
pub(crate) fn start_device_arc(
    commands: &mut Commands,
    node: Entity,
    fields: exa_render::electricity::DeviceFields,
) -> Entity {
    let length = f32::from(fields.length);
    let axis = Vec3::Z;
    let end = fields.end_point(Vec3::ZERO, axis).unwrap_or(axis * length);
    let spec = exa_render::electricity::ArcSpec::device_fields(fields, axis);
    let spawned = crate::arc::spawn_arc(commands, Some(node), spec, Vec3::ZERO, end);
    commands
        .entity(spawned)
        .insert(crate::arc::ElectricDevice::default());
    spawned
}

/// What a relay sends on from `signal`, read from the placed records.
///
/// A broadcast walks every placed record in query order. A pair sends the
/// same value to both of the relay's targets. A combiner takes its input
/// bit from the source record's value word and is left unchanged when the
/// source has no record. A sequence keeps its count in that same word and a
/// wrong order in the out-of-order flag. These records sit on placed objects,
/// the only nodes a finished sequence clears.
fn relay_forwards(
    kind: mechanism::Relay,
    signal: &MechanismSignal,
    records: MechanismRecords,
    objects: &mut Query<(Entity, &mut MechanismRecords)>,
) -> Vec<relay::Forward> {
    let Some(operative) = records.operative else {
        return Vec::new();
    };
    match kind {
        mechanism::Relay::Broadcast => {
            let listeners: Vec<relay::Listener> = objects
                .iter()
                .map(|(_, other)| relay::Listener {
                    id: other.id,
                    door_link: other.door.as_ref().map(|door| door.key),
                    listens_to: other.operative.map(|record| record.target),
                })
                .collect();
            relay::broadcast(signal.target, signal.source, signal.value, &listeners)
        }
        mechanism::Relay::Both => relay::forward_both(
            true,
            operative.target,
            operative.second,
            signal.source,
            signal.value,
        )
        .into_iter()
        .flatten()
        .collect(),
        mechanism::Relay::Combine | mechanism::Relay::Second => {
            let mut combiner = relay::Combiner {
                state: operative.value,
                flags: operative.flags,
                target: operative.target,
                code: operative.second,
            };
            let source = objects.iter().find_map(|(_, other)| {
                (other.id == signal.source)
                    .then_some(other.operative)
                    .flatten()
            });
            let sent = if kind == mechanism::Relay::Combine {
                relay::combine(
                    &mut combiner,
                    signal.target,
                    source.map(|record| record.value),
                    signal.value,
                )
            } else {
                relay::combine_else(
                    &mut combiner,
                    signal.target,
                    source.map(|record| record.value),
                    source.map_or(0, |record| record.second),
                    signal.source,
                    signal.value,
                )
            };
            if let Some((_, mut stored)) = objects
                .iter_mut()
                .find(|(_, other)| other.id == signal.target)
                && let Some(record) = stored.operative.as_mut()
            {
                record.value = combiner.state;
            }
            sent.into_iter().collect()
        }
        mechanism::Relay::Sequence => {
            let mut seq = relay::Sequence {
                word: operative.value,
                wrong: operative.flags & relay::WRONG != 0,
                first: operative.target,
                second: operative.second,
            };
            let source = objects.iter().find_map(|(_, other)| {
                (other.id == signal.source)
                    .then_some(other.operative)
                    .flatten()
            });
            let tick = relay::sequence(
                &mut seq,
                signal.target,
                source.map(|record| record.value),
                signal.value,
            );
            let mut sent: Vec<_> = tick.forward.into_iter().collect();
            if tick.clear_listeners {
                sent.extend(objects.iter().filter_map(|(_, other)| {
                    let record = other.operative?;
                    relay::sequence_clears(
                        relay::LISTENER_CLASS,
                        record.flags,
                        record.target,
                        signal.target,
                    )
                    .then_some(relay::Forward {
                        target: other.id,
                        source: signal.target,
                        value: 0,
                    })
                }));
            }
            if let Some((_, mut stored)) = objects
                .iter_mut()
                .find(|(_, other)| other.id == signal.target)
                && let Some(record) = stored.operative.as_mut()
            {
                record.value = seq.word;
                record.flags = if seq.wrong {
                    record.flags | relay::WRONG
                } else {
                    record.flags & !relay::WRONG
                };
            }
            sent
        }
    }
}

/// A door order on the door bodies (hinged or sliding) among `nodes`, in
/// their order, each woken to run it.
/// Runs a door order on the door bodies among `nodes` and returns the lock
/// sounds it plays, in the order the doors produced them: an open sounds
/// the first door's lock set once, a close and lock each unlatched sliding
/// leaf's, a toggle whichever its doors chose. The doors under a node
/// share their object's word: a close that finds a shut leaf sets its bit 0
/// on every door's copy (the latch a save carries), as the original sets it
/// on the node's word.
fn order_doors(
    order: DoorOrder,
    nodes: &[Entity],
    bodies: &mut Query<MechanismParts>,
    now: i32,
) -> Vec<LockSounded> {
    let mut found: Vec<(Entity, door::Door, Vec3)> = Vec::new();
    for &e in nodes {
        if let Ok((body, hinged, sliding, _)) = bodies.get(e) {
            let at = body.origin_at(0).unwrap_or_default();
            if let Some(glue) = hinged {
                found.push((e, glue.door, at));
            } else if let Some(glue) = sliding {
                found.push((e, glue.door, at));
            }
        }
    }
    if found.is_empty() {
        return Vec::new();
    }
    let mut doors: Vec<door::Door> = found.iter().map(|(_, d, _)| *d).collect();
    let word = doors[0].saved as u32;
    let (sounds, word) = match order {
        DoorOrder::Open => (mechanism::open_doors(&mut doors, now), word),
        DoorOrder::CloseAndLock => mechanism::close_and_lock_doors(&mut doors, word),
        DoorOrder::Toggle => mechanism::toggle_doors(&mut doors, word, now),
    };
    let played = sounds
        .0
        .iter()
        .filter_map(|&(k, _)| {
            let (_, door, at) = found.get(k)?;
            Some(LockSounded {
                name: door::sound_name(door.record.lock_sound, "LkO"),
                position: *at,
            })
        })
        .collect();
    for ((e, _, _), mut changed) in found.iter().zip(doors) {
        if word & 1 != 0 {
            changed.saved |= 1;
        }
        if let Ok((mut body, hinged, sliding, _)) = bodies.get_mut(*e) {
            if let Some(mut glue) = hinged {
                glue.door = changed;
            } else if let Some(mut glue) = sliding {
                glue.door = changed;
            }
            body.wake_to(WAKE_COUNT);
        }
    }
    played
}

/// The records of a placed object that a signal or a mechanism reads.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct MechanismRecords {
    /// The placed id, tag bits included.
    pub id: u32,
    /// The body class the records give the object's model.
    pub body: Option<objdb::Mechanism>,
    pub door: Option<door::Record>,
    pub operative: Option<mechanism::Record>,
}

impl MechanismRecords {
    /// The records of `item` placed under `id`; `None` when it has neither
    /// a door nor an operative section.
    pub fn of(id: u32, item: &Item) -> Option<Self> {
        if item.door.is_none() && item.operative.is_none() {
            return None;
        }
        Some(Self {
            id,
            body: item.mechanism(),
            door: item.door.as_ref().map(door_record),
            operative: item.operative.as_ref().map(operative_record),
        })
    }

    /// Where a signal of `value` to this object goes.
    pub fn dispatch(&self, value: i32) -> mechanism::Dispatch {
        mechanism::dispatch(self.door.as_ref(), self.operative.as_ref(), value)
    }

    /// The push surface's record, for a plate.
    pub fn plate(&self) -> Option<plate::Record> {
        self.operative.as_ref().map(plate_record)
    }

    /// The lever's record, for a lever.
    pub fn lever(&self) -> Option<lever::Record> {
        self.operative.as_ref().map(lever_record)
    }
}

/// A door section as the door rules take it.
pub fn door_record(section: &objdb::Door) -> door::Record {
    door::Record {
        flags: section.flags,
        key: section.key,
        lock_sound: section.lock_sound,
        slam_sound: section.slam_sound,
        hinge_sound: section.hinge_sound,
        unlatch_sound: section.unlatch_sound,
    }
}

/// An operative section as the dispatch and the jobs take it.
pub fn operative_record(section: &objdb::Operative) -> mechanism::Record {
    mechanism::Record {
        class: section.class,
        kind: section.kind,
        flags: section.flags,
        value: section.value,
        time: section.time,
        target: section.target,
        second: section.second,
    }
}

/// A push surface's part of its operative record.
pub fn plate_record(record: &mechanism::Record) -> plate::Record {
    plate::Record {
        kind: record.kind,
        flags: record.flags,
        target: record.target,
        second: record.second,
    }
}

/// A lever's part of its operative record.
pub fn lever_record(record: &mechanism::Record) -> lever::Record {
    lever::Record {
        kind: record.kind,
        flags: record.flags,
        target: record.target,
        second: record.second,
        time: record.time,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use exa_physics::mechanism::{Dispatch, DoorOrder};

    /// The order finds its placed object by id and walks the nodes under
    /// it; a target without doors, or an id no object carries, is left.
    #[test]
    fn the_open_order_runs_on_the_placed_object_by_id() {
        let mut app = App::new();
        app.add_message::<OpenDoorsUnder>()
            .add_message::<LockSounded>()
            .add_systems(Update, open_doors_under);
        let object = app
            .world_mut()
            .spawn(crate::PlacedItem {
                id: 4,
                tagged_id: objdb::LOCAL | 4,
                condition: 0,
            })
            .id();
        app.world_mut().spawn(ChildOf(object));
        app.world_mut().write_message(OpenDoorsUnder {
            target: objdb::LOCAL | 4,
        });
        app.world_mut().write_message(OpenDoorsUnder { target: 9 });
        app.update();
        assert!(app.world().get_entity(object).is_ok());
        // No door under it: no lock sound.
        assert!(app.world().resource::<Messages<LockSounded>>().is_empty());
    }

    /// A sliding leaf on level rails, as the shipped portcullis's.
    fn sliding_leaf(lock_sound: u8) -> (Ragdoll, SlideBody) {
        use exa_physics::body::{PendingSlide, RigidLink, RigidPart, world_link};
        let slider = |x: f32, y: f32| RigidLink {
            on_body: Vec3::new(x, y, 0.0),
            on_parent: Vec3::new(x, y, 0.0),
            slack: 1.7,
            strength: 0.0,
            smoothing: 0.01,
            flags: world_link::SLIDER,
            axis: Vec3::new(-1.7, 0.0, 0.0),
        };
        let mut body = Ragdoll::rigid(&[RigidPart {
            entity: Entity::PLACEHOLDER,
            parent: None,
            world: bevy::math::Affine3A::from_translation(Vec3::new(3.0, 0.0, 0.0)),
            rest: [
                Vec3::new(0.0, 0.0, 0.1),
                Vec3::new(0.0, 0.0, -0.1),
                Vec3::new(1.0, 2.0, 0.0),
                Vec3::new(-1.0, 2.0, 0.0),
            ],
            point_masses: [4.0; 4],
            parent_world: bevy::math::Affine3A::IDENTITY,
            mobility: 0.048,
            spheres: vec![(Vec3::new(0.0, 1.0, 0.0), 0.06)],
            origin: Vec3::new(3.0, 0.0, 0.0),
            links: vec![slider(0.7, 0.2), slider(0.7, 1.9), slider(-0.7, 1.0)],
            mesh: None,
            triangles: None,
        }]);
        let pending = PendingSlide {
            record: door::Record {
                lock_sound,
                ..door::Record::default()
            },
            saved: 0,
            handle_height: 1.0,
        };
        let glue = SlideBody::new(&mut body, &pending).expect("a sliding leaf");
        (body, glue)
    }

    /// A signal that closes and locks a gate sounds each unlatched sliding
    /// leaf's lock set at the leaf, and a signalled open sounds the first
    /// door's once; a shut leaf latches and the group's word keeps bit 0.
    #[test]
    fn a_signalled_door_order_plays_its_lock_sounds() {
        let mut app = App::new();
        app.add_plugins(MechanismPlugin);
        let gate = 0x8000_0030;
        let object = app
            .world_mut()
            .spawn((
                MechanismRecords {
                    id: gate,
                    body: Some(objdb::Mechanism::Door),
                    door: Some(door::Record::default()),
                    operative: None,
                },
                Transform::default(),
            ))
            .id();
        let leaf = app
            .world_mut()
            .spawn((sliding_leaf(2), ChildOf(object)))
            .id();
        let shut = app
            .world()
            .get::<SlideBody>(leaf)
            .unwrap()
            .door
            .slide
            .closed;
        let signal = |value| MechanismSignal {
            target: gate,
            source: 0x8000_0031,
            value,
        };
        // Value 0 on a plain door record closes and locks.
        app.world_mut().write_message(signal(0));
        app.world_mut().run_schedule(FixedUpdate);
        let sounded: Vec<LockSounded> = app
            .world_mut()
            .resource_mut::<Messages<LockSounded>>()
            .drain()
            .collect();
        assert_eq!(sounded.len(), 1, "the unlatched leaf sounds its lock");
        assert_eq!(sounded[0].name, "doorc_LkO");
        assert!((sounded[0].position - Vec3::new(3.0, 0.0, 0.0)).length() < 1e-3);
        let door = app.world().get::<SlideBody>(leaf).unwrap().door;
        assert_eq!(door.latched, shut);
        assert_eq!(door.saved & 1, i32::from(shut));

        // An open sounds the first door once.
        app.world_mut().write_message(signal(1));
        app.world_mut().run_schedule(FixedUpdate);
        let sounded: Vec<LockSounded> = app
            .world_mut()
            .resource_mut::<Messages<LockSounded>>()
            .drain()
            .collect();
        assert_eq!(sounded.len(), 1);
        assert_eq!(sounded[0].name, "doorc_LkO");
    }

    /// The arena's order unlatches the door object standing within ten
    /// centimetres of the spot (x and z), its word losing bit 0; an order
    /// that arrives before the door body is registered waits for it, and
    /// an object outside the ten centimetres is left.
    #[test]
    fn a_latch_order_finds_the_door_at_its_spot() {
        let mut app = App::new();
        app.add_plugins(MechanismPlugin);
        let records = |id| MechanismRecords {
            id,
            body: Some(objdb::Mechanism::Door),
            door: Some(door::Record::default()),
            operative: None,
        };
        let at = |x: f32, z: f32| GlobalTransform::from(Transform::from_xyz(x, 0.0, z));
        let near = app
            .world_mut()
            .spawn((records(0x8000_0040), at(-20.95, 9.05)))
            .id();
        let far = app
            .world_mut()
            .spawn((records(0x8000_0041), at(-20.85, 9.0)))
            .id();
        app.world_mut().write_message(LatchDoorsAt {
            x_cm: -2100.0,
            z_cm: 900.0,
            latched: false,
        });
        // No body yet: the order waits.
        app.world_mut().run_schedule(FixedUpdate);
        let latched_leaf = || {
            let (body, mut glue) = sliding_leaf(0);
            glue.door.latched = true;
            glue.door.saved = 0x101;
            (body, glue)
        };
        let leaf = app.world_mut().spawn((latched_leaf(), ChildOf(near))).id();
        let other = app.world_mut().spawn((latched_leaf(), ChildOf(far))).id();
        app.world_mut().run_schedule(FixedUpdate);
        let door = app.world().get::<SlideBody>(leaf).unwrap().door;
        assert!(!door.latched);
        assert_eq!(door.saved, 0x100);
        let untouched = app.world().get::<SlideBody>(other).unwrap().door;
        assert!(untouched.latched);
    }

    fn item(door: Option<objdb::Door>, operative: Option<objdb::Operative>) -> Item {
        Item {
            door,
            operative,
            ..Item::parse(1, &[]).expect("an empty item")
        }
    }

    fn operative(class: u8, kind: u8, flags: u16) -> objdb::Operative {
        objdb::Operative {
            class,
            kind,
            flags,
            value: 3,
            time: 50,
            target: 0x8000_00D0,
            second: 0x8000_00D1,
            atlas: [0; 16],
            destinations: [0; objdb::DESTINATIONS],
        }
    }

    #[test]
    fn a_placed_door_keeps_its_record_and_dispatches_by_it() {
        assert_eq!(MechanismRecords::of(7, &item(None, None)), None);
        let section = objdb::Door {
            flags: objdb::door::INVERTED | objdb::door::MOTORISED,
            key: 0x8000_00AF,
            lock_sound: 1,
            slam_sound: 2,
            hinge_sound: 3,
            unlatch_sound: 4,
        };
        let records = MechanismRecords::of(0x8000_0010, &item(Some(section), None)).unwrap();
        assert_eq!(records.body, Some(objdb::Mechanism::Door));
        let record = records.door.unwrap();
        assert!(record.has(door::INVERTED) && record.has(door::MOTORISED));
        assert_eq!(record.key, 0x8000_00AF);
        assert_eq!(
            (
                record.lock_sound,
                record.slam_sound,
                record.hinge_sound,
                record.unlatch_sound
            ),
            (1, 2, 3, 4)
        );
        // Inverted: 1 closes and locks, 0 opens, -1 toggles.
        assert_eq!(
            records.dispatch(1),
            Dispatch::Doors(DoorOrder::CloseAndLock)
        );
        assert_eq!(records.dispatch(0), Dispatch::Doors(DoorOrder::Open));
        assert_eq!(records.dispatch(-1), Dispatch::Doors(DoorOrder::Toggle));
    }

    #[test]
    fn a_lever_and_a_plate_read_their_operative_record() {
        let lever = MechanismRecords::of(
            0x8000_0020,
            &item(None, Some(operative(objdb::operative::LEVER, 1, 0x11))),
        )
        .unwrap();
        assert_eq!(lever.body, Some(objdb::Mechanism::Lever));
        let record = lever.lever().unwrap();
        assert_eq!((record.kind, record.flags, record.time), (1, 0x11, 50));
        assert_eq!((record.target, record.second), (0x8000_00D0, 0x8000_00D1));

        let button = MechanismRecords::of(
            0x8000_0021,
            &item(
                None,
                Some(operative(
                    objdb::operative::PUSH_SURFACE,
                    objdb::operative::PUSH_INVERTED,
                    plate::ONE_SHOT,
                )),
            ),
        )
        .unwrap();
        assert_eq!(button.body, Some(objdb::Mechanism::PushSurface));
        let plate = plate::Plate::new(&button.plate().unwrap(), 0);
        assert!(plate.invert && plate.one_shot && plate.has_target);
        assert_eq!(button.dispatch(1), Dispatch::Plate { pushed: true });

        // A fire job is not a body, and its value is the fire's size.
        let fire = MechanismRecords::of(
            0x8000_0015,
            &item(
                None,
                Some(objdb::Operative {
                    value: 200,
                    ..operative(objdb::operative::JOB, objdb::operative::FIRE, 0)
                }),
            ),
        )
        .unwrap();
        assert_eq!(fire.body, None);
        assert_eq!(
            fire.dispatch(1),
            Dispatch::Job(mechanism::Job::Fire { size: 200 })
        );
    }

    /// A broadcast reaches a tagged door whose link is the relay, including
    /// the source, and skips a door linked elsewhere and an untagged id.
    #[test]
    fn a_broadcast_reaches_the_doors_linked_to_it() {
        let relay_id = 0x8000_0100;
        let linked = 0x8000_0010;
        let source = 0x8000_0011;
        let elsewhere = 0x8000_0012;
        let untagged = 0x10;
        let mut app = App::new();
        app.add_plugins(MechanismPlugin);
        let door = |id, key| MechanismRecords {
            id,
            body: Some(objdb::Mechanism::Door),
            door: Some(door::Record {
                key,
                ..door::Record::default()
            }),
            operative: None,
        };
        app.world_mut().spawn((
            MechanismRecords {
                id: relay_id,
                body: None,
                door: None,
                operative: Some(mechanism::Record {
                    class: mechanism::RELAY,
                    kind: 1,
                    ..mechanism::Record::default()
                }),
            },
            Transform::default(),
        ));
        for records in [
            door(linked, relay_id),
            door(source, relay_id),
            door(elsewhere, 0x8000_0200),
            door(untagged, relay_id),
        ] {
            app.world_mut().spawn((records, Transform::default()));
        }
        #[derive(Resource, Default)]
        struct Caught(Vec<MechanismSignal>);
        #[derive(Resource)]
        struct From(u32);
        fn catch_forwards(
            mut read: MessageReader<RelayForward>,
            from: Res<From>,
            mut caught: ResMut<Caught>,
        ) {
            caught.0.extend(
                read.read()
                    .map(|forward| forward.0)
                    .filter(|signal| signal.source == from.0),
            );
        }
        app.insert_resource(Caught::default());
        app.insert_resource(From(relay_id));
        app.add_systems(FixedUpdate, catch_forwards.after(route_signals));
        app.world_mut().write_message(MechanismSignal {
            target: relay_id,
            source,
            value: 1,
        });
        app.world_mut().run_schedule(FixedUpdate);
        let sent = app.world().resource::<Caught>().0.clone();
        assert_eq!(
            sent,
            vec![
                MechanismSignal {
                    target: linked,
                    source: relay_id,
                    value: 1,
                },
                MechanismSignal {
                    target: source,
                    source: relay_id,
                    value: 1,
                },
            ]
        );
    }

    /// The count is the relay's value word and a repeated mark sets the
    /// out-of-order flag, leaving every other flag bit. A zero mark finishes:
    /// the second target until eight ordered steps, then the first, and every
    /// listener except one carrying the skip flag is cleared with 0.
    #[test]
    fn a_sequence_counts_in_the_value_word_and_clears_its_listeners() {
        let relay_id = 0x8000_0100;
        let source_id = 0x8000_0011;
        let first = 0x8000_0020;
        let second = 0x8000_0021;
        let listener = 0x8000_0030;
        let skipped = 0x8000_0031;
        let elsewhere = 0x8000_0032;
        let mut app = App::new();
        app.add_plugins(MechanismPlugin);
        let relay = app
            .world_mut()
            .spawn(MechanismRecords {
                id: relay_id,
                body: None,
                door: None,
                operative: Some(mechanism::Record {
                    class: mechanism::RELAY,
                    kind: 5,
                    flags: mechanism::SWITCH_OFF,
                    target: first,
                    second,
                    ..mechanism::Record::default()
                }),
            })
            .id();
        let source = app
            .world_mut()
            .spawn(MechanismRecords {
                id: source_id,
                body: None,
                door: None,
                operative: Some(mechanism::Record {
                    value: 1,
                    ..mechanism::Record::default()
                }),
            })
            .id();
        let listening = |id, flags, target| MechanismRecords {
            id,
            body: None,
            door: None,
            operative: Some(mechanism::Record {
                flags,
                target,
                ..mechanism::Record::default()
            }),
        };
        app.world_mut().spawn(listening(listener, 0, relay_id));
        app.world_mut()
            .spawn(listening(skipped, relay::SKIP_CLEAR, relay_id));
        app.world_mut().spawn(listening(elsewhere, 0, 0x8000_0099));

        #[derive(Resource, Default)]
        struct Caught(Vec<MechanismSignal>);
        fn catch_forwards(mut read: MessageReader<RelayForward>, mut caught: ResMut<Caught>) {
            caught.0.extend(read.read().map(|forward| forward.0));
        }
        app.insert_resource(Caught::default());
        app.add_systems(FixedUpdate, catch_forwards.after(route_signals));

        let stored = |app: &App| {
            app.world()
                .get::<MechanismRecords>(relay)
                .unwrap()
                .operative
                .unwrap()
        };
        let signal = |app: &mut App, value: i32| {
            app.world_mut().write_message(MechanismSignal {
                target: relay_id,
                source: source_id,
                value,
            });
            app.world_mut().run_schedule(FixedUpdate);
            std::mem::take(&mut app.world_mut().resource_mut::<Caught>().0)
        };
        let set_mark = |app: &mut App, mark: u16| {
            app.world_mut()
                .get_mut::<MechanismRecords>(source)
                .unwrap()
                .operative
                .as_mut()
                .unwrap()
                .value = mark;
        };

        // A value of 0, and a source with no record, leave the counter.
        assert!(signal(&mut app, 0).is_empty());
        assert_eq!(stored(&app).value, 0);
        app.world_mut().write_message(MechanismSignal {
            target: relay_id,
            source: 7,
            value: 1,
        });
        app.world_mut().run_schedule(FixedUpdate);
        assert!(std::mem::take(&mut app.world_mut().resource_mut::<Caught>().0).is_empty());
        assert_eq!(
            (stored(&app).value, stored(&app).flags),
            (0, mechanism::SWITCH_OFF)
        );

        assert!(signal(&mut app, 1).is_empty());
        assert_eq!(stored(&app).value, 0x110);
        assert_eq!(stored(&app).flags, mechanism::SWITCH_OFF);

        // The same mark again flags the order, and keeps the switch-off bit.
        assert!(signal(&mut app, 1).is_empty());
        assert_eq!(stored(&app).value, 0x120);
        assert_eq!(stored(&app).flags, mechanism::SWITCH_OFF | relay::WRONG);

        set_mark(&mut app, 0);
        assert_eq!(
            signal(&mut app, 1),
            vec![
                MechanismSignal {
                    target: second,
                    source: relay_id,
                    value: 1,
                },
                MechanismSignal {
                    target: listener,
                    source: relay_id,
                    value: 0,
                },
            ]
        );
        assert_eq!(stored(&app).value, 0);
        assert_eq!(stored(&app).flags, mechanism::SWITCH_OFF);

        for (step, mark) in [1_u16, 2, 4, 8, 16, 32, 64, 128].into_iter().enumerate() {
            set_mark(&mut app, mark);
            assert!(signal(&mut app, 1).is_empty(), "step {step} forwarded");
        }
        assert_eq!(stored(&app).flags, mechanism::SWITCH_OFF);
        set_mark(&mut app, 0);
        assert_eq!(
            signal(&mut app, 1),
            vec![
                MechanismSignal {
                    target: first,
                    source: relay_id,
                    value: 1,
                },
                MechanismSignal {
                    target: listener,
                    source: relay_id,
                    value: 0,
                },
            ]
        );
        assert_eq!(stored(&app).value, 0);
        assert_eq!(stored(&app).flags, mechanism::SWITCH_OFF);
    }

    /// An even signal only clears the low bit. An odd one sets it and, with
    /// no emitter under the node, starts the device arc. The next signal
    /// finds that emitter off, writes its life and state, and does not start
    /// another; once it is on, a signal leaves it.
    #[test]
    fn a_light_job_stores_its_bit_and_starts_one_device_arc() {
        let id = 0x8000_0040;
        let mut app = App::new();
        app.add_plugins(MechanismPlugin);
        let node = app
            .world_mut()
            .spawn(MechanismRecords {
                id,
                body: None,
                door: None,
                operative: Some(mechanism::Record {
                    class: mechanism::JOB,
                    kind: mechanism::LIGHT,
                    value: 0x4,
                    target: 1000 | (200 << 16),
                    ..mechanism::Record::default()
                }),
            })
            .id();
        let send = |app: &mut App, value: i32| {
            app.world_mut().write_message(MechanismSignal {
                target: id,
                source: 1,
                value,
            });
            app.world_mut().run_schedule(FixedUpdate);
            app.world_mut().flush();
        };
        let word = |app: &App| {
            app.world()
                .get::<MechanismRecords>(node)
                .unwrap()
                .operative
                .unwrap()
                .value
        };
        let devices = |app: &App| -> Vec<_> {
            app.world()
                .get::<Children>(node)
                .into_iter()
                .flat_map(|kids| kids.iter())
                .filter_map(|child| {
                    let device = app.world().get::<crate::arc::ElectricDevice>(child)?;
                    let arc = &app.world().get::<crate::arc::ArcEffect>(child)?.arc;
                    Some((*device, arc.start, arc.end, arc.spec.sound.clone()))
                })
                .collect()
        };

        send(&mut app, 0);
        assert_eq!(word(&app), 0x4);
        assert!(devices(&app).is_empty());

        send(&mut app, 1);
        assert_eq!(word(&app), 0x5);
        assert_eq!(
            devices(&app),
            vec![(
                crate::arc::ElectricDevice::default(),
                Vec3::ZERO,
                Vec3::new(0.0, 0.0, 200.0),
                Some("electricity01".to_string()),
            )]
        );

        send(&mut app, 0);
        assert_eq!(word(&app), 0x4);
        let found = devices(&app);
        assert_eq!(found.len(), 1);
        assert_eq!((found[0].0.state, found[0].0.life), (1, 10));

        send(&mut app, 1);
        assert_eq!(word(&app), 0x5);
        let found = devices(&app);
        assert_eq!(found.len(), 1);
        assert_eq!((found[0].0.state, found[0].0.life), (1, 10));
    }

    /// Below the daylight threshold a fire record spawns a fire and lifts it
    /// by the stored height. Above it the fire is not spawned and the node's
    /// light is put out.
    #[test]
    fn a_load_fire_lights_until_the_daylight_is_up() {
        let bare = |target| {
            let mut world = World::new();
            world.spawn(MechanismRecords {
                id: 1,
                body: None,
                door: None,
                operative: Some(mechanism::Record {
                    class: mechanism::FIRE_CLASS,
                    kind: mechanism::FIRE,
                    target,
                    ..mechanism::Record::default()
                }),
            });
            world
        };
        let with_light = |target| {
            let mut world = bare(target);
            let entity = world
                .query::<(Entity, &MechanismRecords)>()
                .iter(&world)
                .next()
                .expect("the record")
                .0;
            world
                .entity_mut(entity)
                .insert(crate::light_source::LightSource::new(0, 4.0));
            world
        };
        let flame_y = |world: &mut World| {
            let entity = world
                .query::<(Entity, &MechanismRecords)>()
                .iter(world)
                .next()
                .unwrap()
                .0;
            direct_children(world, entity)
                .into_iter()
                .find_map(|child| {
                    world
                        .get::<crate::fire::Fire>(child)
                        .map(|_| world.get::<Transform>(child).unwrap().translation.y)
                })
        };

        let mut dark = bare(200);
        apply_load_fires(&mut dark, 0.0);
        let mut lifted = bare(200 | (50 << 16));
        apply_load_fires(&mut lifted, 0.0);
        let low = flame_y(&mut dark).expect("a fire in the dark");
        let high = flame_y(&mut lifted).expect("a lifted fire");
        assert!((high - low - 0.5).abs() < 1e-5, "{low} -> {high}");

        let mut lit = with_light(200);
        apply_load_fires(&mut lit, 0.0);
        assert!(flame_y(&mut lit).is_some());
        let entity = lit
            .query::<(Entity, &MechanismRecords)>()
            .iter(&lit)
            .next()
            .unwrap()
            .0;
        assert_eq!(
            lit.get::<crate::light_source::LightSource>(entity)
                .unwrap()
                .intensity,
            0.0
        );

        let mut day = with_light(200);
        apply_load_fires(&mut day, 1.0);
        assert!(flame_y(&mut day).is_none());
        let entity = day
            .query::<(Entity, &MechanismRecords)>()
            .iter(&day)
            .next()
            .unwrap()
            .0;
        assert_eq!(
            day.get::<crate::light_source::LightSource>(entity)
                .unwrap()
                .intensity,
            0.0
        );
    }
}
