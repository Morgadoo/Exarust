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
//! The records are all that reaches the running game for now. A door,
//! lever or plate is a body hung from the world by its links: the hinge
//! pins and swing stop of a door, the axle pins and arm of a lever, the
//! slider links of a plate. `exa_physics` has the body-side world-link laws
//! (`exa_physics::body::world`) but no level registers them yet, so the
//! bodies stay static level geometry ([`crate::bodies::Placement`]) and
//! nothing produces the per-tick heading, handle speed, travel fraction or
//! arm angle the steps read. Until then no system ticks a mechanism or
//! routes its signals.
//!
//! Evidence: the object database's door and operative sections
//! (`docs/formats/OTHER.md`) and the records the original's door, lever
//! and push-surface set-up and the signal dispatch read: the same fields
//! in the same order.

use bevy::prelude::*;
use exa_formats::objdb::{self, Item};
use exa_physics::{door, lever, mechanism, plate};

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
