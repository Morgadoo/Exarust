//! Where a signal goes, as a single target's summary.
//!
//! A missing target does nothing. A door toggles on −1: an unlatched leaf
//! is closed and a latched leaf is opened. Any other value closes on 0 and
//! opens otherwise. An inverted door flips that 0 and 1. −1 still toggles.
//! A door wins over an operative record on the same object.
//!
//! A fire job starts, at its stored size, only when the value is not 0. A
//! light job sets the stored bit to the value's low bit. A relay of kind
//! 0, 1, 2, 3 or 5 is named. Any other kind is ignored. A plate is pushed
//! exactly when the value is not 0.
//!
//! The rules themselves are [`crate::mechanism::dispatch`]'s, which reads
//! whole records; this module keeps the summary a caller that holds only a
//! target's class and kind needs, and the door toggle's leaf rule.
//!
//! Evidence: the signal dispatch, the deferred job, and the toggle of the
//! doors under a node. Kinds 0, 2, 3 and 5 run in the relay module, the door
//! orders in the door module.

use crate::door::{self, Record as DoorRecord};
use crate::mechanism::{self, Dispatch, Job, Relay};

/// What the signal found.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Target {
    pub found: bool,
    /// A door record. It wins over [`class`].
    pub door: bool,
    /// The door flips 0 and 1.
    pub inverted: bool,
    pub class: u8,
    pub kind: u8,
    /// A fire's size, or a light's stored bit.
    pub stored: u16,
}

/// What the signal asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Order {
    None,
    /// Close the doors under the node.
    Close,
    /// Open the doors under the node.
    Open,
    /// An unlatched leaf closes. A latched leaf opens.
    Toggle,
    /// Start a fire of this size.
    Fire {
        size: u16,
    },
    /// The light's stored bit after this signal.
    Light {
        on: bool,
    },
    /// A relay of this kind. Kinds 0, 2, 3 and 5 are run by the relay module.
    Relay {
        kind: u8,
    },
    /// The plate is driven while `pushed` is set.
    Plate {
        pushed: bool,
    },
}

/// Send `value` to `target`. A light job writes the stored bit back.
pub fn signal(target: &mut Target, value: i32) -> Order {
    if !target.found {
        return Order::None;
    }
    if target.door {
        let record = DoorRecord {
            flags: if target.inverted { door::INVERTED } else { 0 },
            ..DoorRecord::default()
        };
        return match mechanism::dispatch(Some(&record), None, value) {
            Dispatch::Doors(mechanism::DoorOrder::CloseAndLock) => Order::Close,
            Dispatch::Doors(mechanism::DoorOrder::Open) => Order::Open,
            Dispatch::Doors(mechanism::DoorOrder::Toggle) => Order::Toggle,
            _ => Order::None,
        };
    }
    let record = mechanism::Record {
        class: target.class,
        kind: target.kind,
        value: target.stored,
        ..mechanism::Record::default()
    };
    match mechanism::dispatch(None, Some(&record), value) {
        Dispatch::Job(Job::Fire { size }) => Order::Fire { size },
        Dispatch::Job(Job::Light { on }) => {
            if on {
                target.stored |= 1;
            } else {
                target.stored &= !1;
            }
            Order::Light { on }
        }
        Dispatch::Relay(Relay::Combine) => Order::Relay { kind: 0 },
        Dispatch::Relay(Relay::Broadcast) => Order::Relay { kind: 1 },
        Dispatch::Relay(Relay::Both) => Order::Relay { kind: 2 },
        Dispatch::Relay(Relay::Second) => Order::Relay { kind: 3 },
        Dispatch::Relay(Relay::Sequence) => Order::Relay { kind: 5 },
        Dispatch::Plate { pushed } => Order::Plate { pushed },
        Dispatch::None | Dispatch::Doors(_) => Order::None,
    }
}

/// One door under a toggled node. An unlatched leaf closes. A latched leaf opens.
pub fn toggle_leaf(latched: bool) -> Order {
    if latched { Order::Open } else { Order::Close }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Operative class of a deferred fire or light.
    const JOB: u8 = 6;
    /// Operative class of a relay.
    const RELAY: u8 = 7;
    /// Operative class of a pressure plate.
    const PLATE: u8 = 5;
    /// Job kind that starts a fire.
    const FIRE: u8 = 0xC;
    /// Job kind that turns a light on or off.
    const LIGHT: u8 = 0xD;
    /// The value that toggles the doors under a node.
    const TOGGLE: i32 = -1;

    fn target() -> Target {
        Target {
            found: true,
            door: false,
            inverted: false,
            class: 0,
            kind: 0,
            stored: 0,
        }
    }

    #[test]
    fn a_signal_opens_a_door_and_starts_a_fire() {
        let mut missing = target();
        missing.found = false;
        missing.door = true;
        assert_eq!(signal(&mut missing, 1), Order::None);

        let mut door = target();
        door.door = true;
        door.class = JOB;
        door.kind = FIRE;
        door.stored = 200;
        assert_eq!(signal(&mut door, 0), Order::Close);
        assert_eq!(signal(&mut door, 1), Order::Open);
        assert_eq!(signal(&mut door, 2), Order::Open);
        assert_eq!(signal(&mut door, TOGGLE), Order::Toggle);
        assert_eq!(toggle_leaf(false), Order::Close);
        assert_eq!(toggle_leaf(true), Order::Open);

        let mut flipped = door;
        flipped.inverted = true;
        assert_eq!(signal(&mut flipped, 0), Order::Open);
        assert_eq!(signal(&mut flipped, 1), Order::Close);
        assert_eq!(signal(&mut flipped, TOGGLE), Order::Toggle);

        let mut fire = target();
        fire.class = JOB;
        fire.kind = FIRE;
        fire.stored = 200;
        assert_eq!(signal(&mut fire, 0), Order::None);
        assert_eq!(signal(&mut fire, 1), Order::Fire { size: 200 });
        assert_eq!(fire.stored, 200);

        let mut light = target();
        light.class = JOB;
        light.kind = LIGHT;
        light.stored = 0x10;
        assert_eq!(signal(&mut light, 1), Order::Light { on: true });
        assert_eq!(light.stored, 0x11);
        assert_eq!(signal(&mut light, 2), Order::Light { on: false });
        assert_eq!(light.stored, 0x10);

        let mut other_job = target();
        other_job.class = JOB;
        other_job.kind = 0;
        assert_eq!(signal(&mut other_job, 1), Order::None);

        for kind in [0_u8, 1, 2, 3, 5] {
            let mut relay = target();
            relay.class = RELAY;
            relay.kind = kind;
            assert_eq!(signal(&mut relay, 1), Order::Relay { kind });
        }
        let mut ignored = target();
        ignored.class = RELAY;
        ignored.kind = 4;
        assert_eq!(signal(&mut ignored, 1), Order::None);

        let mut plate = target();
        plate.class = PLATE;
        assert_eq!(signal(&mut plate, 0), Order::Plate { pushed: false });
        assert_eq!(signal(&mut plate, -1), Order::Plate { pushed: true });
    }
}
