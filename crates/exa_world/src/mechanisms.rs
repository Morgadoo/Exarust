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
//! on the push surface under it. A class 6 fire job puts the node's lights
//! out and spawns a fire at the signalled size; a light job scales the
//! lights under the node on or off. A relay is logged and not run: relays
//! need the level's listener lists, which no system holds yet. A
//! target with no record, or no body of the right kind under it (a static
//! sliding door), does nothing.
//!
//! Evidence: the object database's door and operative sections
//! (`docs/formats/OTHER.md`) and the records the original's door, lever
//! and push-surface set-up and the signal dispatch read: the same fields
//! in the same order.

use bevy::prelude::*;
use exa_formats::objdb::{self, Item};
use exa_physics::body::{
    BodySystems, BodyTicks, DoorBody, PlateBody, Ragdoll, SlideBody, WAKE_COUNT,
};
use exa_physics::mechanism::{Dispatch, DoorOrder, LockSounded, MechanismSignal};
use exa_physics::{door, lever, mechanism, plate};

/// Routes the mechanisms' signals to their targets. Added by
/// [`crate::WorldPlugin`].
pub struct MechanismPlugin;

impl Plugin for MechanismPlugin {
    fn build(&self, app: &mut App) {
        if !app.world().contains_resource::<Messages<MechanismSignal>>() {
            app.add_message::<MechanismSignal>();
        }
        if !app.world().contains_resource::<Messages<LockSounded>>() {
            app.add_message::<LockSounded>();
        }
        app.add_message::<OpenDoorsUnder>().add_systems(
            FixedUpdate,
            (route_signals, open_doors_under).after(BodySystems),
        );
    }
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
        if let Some(sound) = first_lock_sound(&nodes, &bodies) {
            sounds.write(sound);
        }
        order_doors(DoorOrder::Open, &nodes, &mut bodies, now);
    }
}

/// The lock sound of the first door among `nodes`: its record's `LkO`
/// sound at the leaf's first part. [guess: the original plays it at a
/// point the door controller keeps; the leaf's origin stands in]
fn first_lock_sound(nodes: &[Entity], bodies: &Query<MechanismParts>) -> Option<LockSounded> {
    nodes.iter().find_map(|&e| {
        let (body, hinged, sliding, _) = bodies.get(e).ok()?;
        let door = hinged.map(|g| g.door).or(sliding.map(|g| g.door))?;
        Some(LockSounded {
            name: door::sound_name(door.record.lock_sound, "LkO"),
            position: body.origin_at(0).unwrap_or_default(),
        })
    })
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
/// surface, a fire job (a fire is spawned at the node and its own lights
/// put out), a light job (the lights under the node are scaled on or off)
/// or a relay/job that nothing else runs.
pub fn route_signals(
    mut signals: MessageReader<MechanismSignal>,
    mut commands: Commands,
    ticks: Option<Res<BodyTicks>>,
    objects: Query<(Entity, &MechanismRecords)>,
    children: Query<&Children>,
    mut bodies: Query<MechanismParts>,
    mut lights: Query<&mut crate::light_source::LightSource>,
) {
    let now = ticks.map_or(0, |t| t.millis() as i32);
    for signal in signals.read() {
        let Some((node, records)) = objects.iter().find(|(_, r)| r.id == signal.target) else {
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
        let nodes = under(node, &children);
        match dispatch {
            Dispatch::Doors(order) => order_doors(order, &nodes, &mut bodies, now),
            Dispatch::Plate { pushed } => {
                for &e in &nodes {
                    if let Ok((mut body, _, _, Some(mut glue))) = bodies.get_mut(e) {
                        glue.push(&mut body, pushed);
                        break;
                    }
                }
            }
            Dispatch::Relay(_) => {}
            Dispatch::Job(mechanism::Job::Fire { size }) => {
                // The fire replaces the node's own lights, then grows at
                // the signalled size (hundredths, or 2 for a zero word).
                for &e in &nodes {
                    if let Ok(mut light) = lights.get_mut(e) {
                        light.set_scale(0.0);
                    }
                }
                crate::fire::spawn_fire(&mut commands, node, mechanism::job_fire_size(size));
            }
            Dispatch::Job(mechanism::Job::Light { on }) => {
                let scale = if on { 1.0 } else { 0.0 };
                for &e in &nodes {
                    if let Ok(mut light) = lights.get_mut(e) {
                        light.set_scale(scale);
                    }
                }
            }
            Dispatch::None => {}
        }
    }
}

/// A door order on the door bodies (hinged or sliding) among `nodes`, in
/// their order, each woken to run it.
fn order_doors(order: DoorOrder, nodes: &[Entity], bodies: &mut Query<MechanismParts>, now: i32) {
    let mut found: Vec<(Entity, door::Door)> = Vec::new();
    for &e in nodes {
        if let Ok((_, hinged, sliding, _)) = bodies.get(e) {
            if let Some(glue) = hinged {
                found.push((e, glue.door));
            } else if let Some(glue) = sliding {
                found.push((e, glue.door));
            }
        }
    }
    if found.is_empty() {
        return;
    }
    let mut doors: Vec<door::Door> = found.iter().map(|(_, d)| *d).collect();
    match order {
        DoorOrder::Open => {
            mechanism::open_doors(&mut doors, now);
        }
        DoorOrder::CloseAndLock => {
            mechanism::close_and_lock_doors(&mut doors, 0);
        }
        DoorOrder::Toggle => {
            mechanism::toggle_doors(&mut doors, 0, now);
        }
    }
    for ((e, _), changed) in found.iter().zip(doors) {
        if let Ok((mut body, hinged, sliding, _)) = bodies.get_mut(*e) {
            if let Some(mut glue) = hinged {
                glue.door = changed;
            } else if let Some(mut glue) = sliding {
                glue.door = changed;
            }
            body.wake_to(WAKE_COUNT);
        }
    }
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
}
