//! The level's mechanisms and the signals that drive them: operative
//! records, the dispatcher that reads a target's record, the jobs and
//! relays a signal starts, and the level-load setup of an operative object.
//!
//! A signal names a target, a source and a value. A door record wins over
//! an operative record on the same object: −1 toggles the doors under the
//! node by their latches, any other value is inverted when the door's flag
//! 0x80 is set, and then 0 closes and locks and 1 opens. An operative of
//! class 6 starts a deferred job: kind 0xC lights a fire of the stored size
//! when the value is not 0, kind 0xD turns a light on or off. Class 7 is a
//! relay, by kind: 0 combines a bit, 1 broadcasts, 2 forwards to both
//! targets, 3 combines and forwards, and 5 counts a sequence. Any other
//! class is ignored. Class 5 forces the push surface under the node.
//!
//! At load an operative turns its effects off when its flag 0x40 is set. A
//! light job with the value's low bit set starts an electric effect. A fire
//! record of class 8 lights a fire unless the scene's daylight exceeds
//! 0.12, in which case the light under the node is switched off instead.
//!
//! The door, relay and light operations work on the bodies and nodes the
//! caller hands in, in the order the original walks them.

use crate::door::{self, Door, Record as DoorRecord, Sound};

/// Operative classes.
pub const KEY: u8 = 1;
pub const LEVER: u8 = 3;
/// Buttons and pressure plates.
pub const PUSH_SURFACE: u8 = 5;
/// A deferred job run on a signal.
pub const JOB: u8 = 6;
/// Forwards or combines signals.
pub const RELAY: u8 = 7;
/// A fire job.
pub const FIRE: u8 = 0xC;
/// A light job.
pub const LIGHT: u8 = 0xD;
/// The class a fire record has at load.
pub const FIRE_CLASS: u8 = 8;
/// Operative flag: switch the node's effects off at load.
pub const SWITCH_OFF: u16 = 0x40;
/// The daylight above which a fire record's light is put out instead.
pub const DAYLIGHT: f32 = 0.12;
/// The class low bits of a light node.
pub const LIGHT_CLASS: u32 = 1;
/// The class bits of a lit node (`class & 0x5f`).
pub const LIT: u32 = 9;
/// The class bits of an electric object.
pub const ELECTRIC: u32 = 0x41;

/// An operative record: what an object does when signalled.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Record {
    pub class: u8,
    pub kind: u8,
    /// Class-specific flags.
    pub flags: u16,
    /// A job's fire size, a relay's state or a source's bit.
    pub value: u16,
    /// A lever's auto-return time in units of 100 ms; 0 stays.
    pub time: u16,
    /// The object signalled first.
    pub target: u32,
    /// The second target, or a relay's code.
    pub second: u32,
}

/// The value that toggles the doors under a node.
pub const TOGGLE: i32 = -1;

/// A door order from a signal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DoorOrder {
    CloseAndLock,
    Open,
    /// Toggle by the latch of each door under the node.
    Toggle,
}

/// A relay's kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Relay {
    /// Subtype 0: set a bit and compare the state with the code.
    Combine,
    /// Subtype 1: broadcast to every object listening to the relay.
    Broadcast,
    /// Subtype 2: forward the signal to both targets.
    Both,
    /// Subtype 3: set a bit, then signal the code or forward.
    Second,
    /// Subtype 5: count a sequence.
    Sequence,
}

/// A deferred job.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Job {
    Fire { size: u16 },
    Light { on: bool },
}

/// What a signal asks of its target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dispatch {
    None,
    Doors(DoorOrder),
    Job(Job),
    Relay(Relay),
    /// The push surface under the node.
    Plate {
        pushed: bool,
    },
}

impl Record {
    /// The auto-return time in milliseconds.
    pub fn return_ms(&self) -> u32 {
        u32::from(self.time) * 100
    }
}

/// Dispatch a signal to a target. A door record wins over an operative
/// record on the same object; a missing target does nothing.
pub fn dispatch(
    door_record: Option<&DoorRecord>,
    operative: Option<&Record>,
    value: i32,
) -> Dispatch {
    if let Some(record) = door_record {
        if value == TOGGLE {
            return Dispatch::Doors(DoorOrder::Toggle);
        }
        let value = if record.has(door::INVERTED) {
            !value & 1
        } else {
            value
        };
        return Dispatch::Doors(if value == 0 {
            DoorOrder::CloseAndLock
        } else {
            DoorOrder::Open
        });
    }
    let Some(record) = operative else {
        return Dispatch::None;
    };
    match record.class {
        JOB => job(record.kind, value, record.value),
        RELAY => match record.kind {
            0 => Dispatch::Relay(Relay::Combine),
            1 => Dispatch::Relay(Relay::Broadcast),
            2 => Dispatch::Relay(Relay::Both),
            3 => Dispatch::Relay(Relay::Second),
            5 => Dispatch::Relay(Relay::Sequence),
            _ => Dispatch::None,
        },
        PUSH_SURFACE => Dispatch::Plate { pushed: value != 0 },
        _ => Dispatch::None,
    }
}

/// The deferred job a value runs for a job record. A fire job starts only
/// when the value is not 0; a light job sets its stored bit either way.
pub fn job(kind: u8, value: i32, stored: u16) -> Dispatch {
    match kind {
        FIRE if value != 0 => Dispatch::Job(Job::Fire { size: stored }),
        LIGHT => Dispatch::Job(Job::Light { on: value & 1 != 0 }),
        _ => Dispatch::None,
    }
}

/// What a fire record does at load.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Setup {
    None,
    /// The record was missing.
    Missing,
    /// The record's flag 0x40 switches the node's effects off.
    SwitchOff,
    /// A light job with its value's low bit set starts an electric effect.
    Electricity,
    /// A light who is put out because the daylight is up.
    Daylight,
    /// A fire is lit, of this size.
    Fire {
        size: u16,
    },
}

/// The level-load part of an operative record.
pub fn level_setup(record: Option<&Record>, daylight: f32) -> Setup {
    let Some(record) = record else {
        return Setup::Missing;
    };
    if record.flags & SWITCH_OFF != 0 {
        return Setup::SwitchOff;
    }
    if record.kind == LIGHT {
        if record.value & 1 != 0 {
            return Setup::Electricity;
        }
    } else if record.kind == FIRE && record.class == FIRE_CLASS {
        if DAYLIGHT < daylight {
            return Setup::Daylight;
        }
        return Setup::Fire { size: record.value };
    }
    Setup::None
}

/// What the door operations leave for the caller to play: the sounds, in
/// the order the doors produced them.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Sounds(pub Vec<Sound>);

/// Open the doors under a node. The unlock sound plays once when there is
/// a door, before any of them is woken. Only a latched or motorised door
/// takes the command.
pub fn open_doors(doors: &mut [Door], _now_ms: i32) -> Sounds {
    let mut sounds = Sounds::default();
    if doors.is_empty() {
        return sounds;
    }
    sounds.0.push(Sound {
        volume: 0.15,
        volume_spread: 0.1,
        pitch: 0.9,
        pitch_spread: 0.15,
    });
    for door in doors {
        door.wake();
    }
    sounds
}

/// Close and lock the doors under a node. A motorised hinge aims shut for
/// 300 ticks, or 100 when fast; a fast slider drives shut. A slider that is
/// not latched plays its lock sound; a shut leaf latches.
pub fn close_and_lock_doors(doors: &mut [Door]) -> Sounds {
    let mut sounds = Sounds::default();
    for door in doors {
        if let Some(sound) = door.close_and_lock() {
            sounds.0.push(sound);
        }
    }
    sounds
}

/// Toggle the doors under a node: each door in turn decides by its latch
/// whether the whole group is closed and locked or opened, as the original
/// re-walks the node for every door it finds.
pub fn toggle_doors(doors: &mut [Door], now_ms: i32) -> Sounds {
    let mut sounds = Sounds::default();
    for i in 0..doors.len() {
        if doors[i].latched {
            sounds.0.extend(open_doors(doors, now_ms).0);
        } else {
            sounds.0.extend(close_and_lock_doors(doors).0);
        }
    }
    sounds
}

/// Latch or unlatch every door under a node, and set the object word's bit
/// 0. Returns whether any door took it.
pub fn latch_doors(doors: &mut [Door], value: bool) -> bool {
    let mut any = false;
    for door in doors {
        door.latched = value;
        if value {
            door.saved |= 1;
        } else {
            door.saved &= !1;
        }
        any = true;
    }
    any
}

/// A scene node as the operative routines see it: its class bits, its
/// children, and the light and effect state under it.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Node {
    pub class: u32,
    pub children: Vec<usize>,
    /// A light's flag word and fade rate, when the node has a light.
    pub light: Option<Light>,
    /// The node's effect intensity.
    pub intensity: f32,
    /// An electric object's life and whether it is on.
    pub life: i32,
    pub powered: bool,
}

/// A light source's flags and fade.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Light {
    pub flags: u32,
    /// The intensity change a signal sets.
    pub fade: f32,
}

/// The first node at or under `root` whose `class & mask == class`, in
/// child order.
pub fn find(nodes: &[Node], root: usize, mask: u32, class: u32) -> Option<usize> {
    let node = nodes.get(root)?;
    if node.class & mask == class {
        return Some(root);
    }
    for &child in &node.children {
        if let Some(found) = find(nodes, child, mask, class) {
            return Some(found);
        }
    }
    None
}

/// Switch a node and its direct children's lights off, and the intensity of
/// an electric child: the level-load switch-off.
pub fn switch_off(nodes: &mut [Node], root: usize) {
    if let Some(node) = nodes.get_mut(root) {
        if node.class & 0x5f == LIT {
            node.intensity = 0.0;
            if let Some(light) = node.light.as_mut() {
                light.flags |= 0x10;
                light.fade = 0.0;
            }
        }
        let children = node.children.clone();
        for child in children {
            if let Some(child) = nodes.get_mut(child) {
                let bits = child.class & 0x5f;
                if bits == LIT {
                    child.intensity = 0.0;
                    if let Some(light) = child.light.as_mut() {
                        light.flags |= 0x10;
                        light.fade = 0.0;
                    }
                } else if bits == ELECTRIC {
                    child.intensity = 0.0;
                }
            }
        }
    }
}

/// Switch off every light under the node and its direct children, as the
/// fire setup does when the day is up.
pub fn put_out(nodes: &mut [Node], root: usize) {
    if let Some(node) = nodes.get_mut(root) {
        if node.class & 0x5f == LIT {
            node.intensity = 0.0;
            if let Some(light) = node.light.as_mut() {
                light.flags |= 0x10;
                light.fade = 0.0;
            }
        }
        let children = node.children.clone();
        for child in children {
            if let Some(child) = nodes.get_mut(child)
                && child.class & 0x5f == LIT
            {
                child.intensity = 0.0;
                if let Some(light) = child.light.as_mut() {
                    light.flags |= 0x10;
                    light.fade = 0.0;
                }
            }
        }
    }
}

/// The shared environment the levers and push surfaces run in: retention
/// 0.001 per second in every band, and no gravity.
pub const MECHANISM_RETENTION: f32 = 0.001;
/// The environment's step rate, the original's tick.
pub const MECHANISM_HZ: f32 = 83.0;
/// The speed bands' thresholds, in cm/s.
pub const FAST_CM: f32 = 40.0;
pub const SLOW_CM: f32 = 20.0;

/// How much of its speed a mechanism body keeps in one tick.
pub fn retention() -> f32 {
    MECHANISM_RETENTION.powf(1.0 / MECHANISM_HZ)
}

/// A fire's radius from the stored size: hundredths of a metre, at least
/// 0.25 and at most 10.
pub fn fire_size(stored: u16) -> f32 {
    (f32::from(stored) * 0.01).clamp(0.25, 10.0)
}

/// Trigger the electric object directly under a node (a light job with its
/// value's low bit set). Returns whether one was there; an object that is
/// already on is left alone, and a missing one would spawn an effect of its
/// own (not modelled here).
pub fn electric(nodes: &mut [Node], root: usize, _value: i32) -> bool {
    let Some(node) = nodes.get(root) else {
        return false;
    };
    let children = node.children.clone();
    for child in children {
        let Some(child) = nodes.get_mut(child) else {
            continue;
        };
        if child.class & 0xff00_0000 == 0x2100_0000 {
            if child.powered {
                return true;
            }
            child.life = 10;
            child.powered = true;
            return true;
        }
    }
    false
}

/// An object a finished sequence clears.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Clearable {
    pub id: u32,
    pub class: u32,
    pub flags: u16,
    /// The operative target it listens to.
    pub listens_to: u32,
}

/// The ids a finished sequence signals 0: every object of the listener
/// class whose target is the relay and which does not skip clearing.
pub fn listeners_to_clear(listeners: &[Clearable], relay_id: u32) -> Vec<u32> {
    listeners
        .iter()
        .filter(|l| l.class & 0x7000 == 0x4000 && l.listens_to == relay_id && l.flags & 0x2 == 0)
        .map(|l| l.id)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::door::{self, Link, Shape};

    fn door_record(flags: u32) -> DoorRecord {
        DoorRecord {
            flags,
            ..DoorRecord::default()
        }
    }

    fn operative(class: u8, kind: u8) -> Record {
        Record {
            class,
            kind,
            target: 5,
            second: 6,
            ..Record::default()
        }
    }

    #[test]
    fn a_signal_prefers_doors_and_inverts_them_when_asked() {
        let doors = door_record(0);
        assert_eq!(
            dispatch(Some(&doors), None, 0),
            Dispatch::Doors(DoorOrder::CloseAndLock)
        );
        assert_eq!(
            dispatch(Some(&doors), None, 5),
            Dispatch::Doors(DoorOrder::Open)
        );
        assert_eq!(
            dispatch(Some(&doors), None, TOGGLE),
            Dispatch::Doors(DoorOrder::Toggle)
        );
        let inverted = door_record(door::INVERTED);
        assert_eq!(
            dispatch(Some(&inverted), None, 0),
            Dispatch::Doors(DoorOrder::Open)
        );
        assert_eq!(
            dispatch(Some(&inverted), None, 1),
            Dispatch::Doors(DoorOrder::CloseAndLock)
        );
        assert_eq!(
            dispatch(Some(&inverted), None, TOGGLE),
            Dispatch::Doors(DoorOrder::Toggle)
        );
        // The door wins over an operative record on the same object.
        assert_eq!(
            dispatch(Some(&doors), Some(&operative(PUSH_SURFACE, 1)), 1),
            Dispatch::Doors(DoorOrder::Open)
        );
        assert_eq!(dispatch(None, None, 1), Dispatch::None);
    }

    #[test]
    fn a_signal_starts_jobs_relays_and_plates_by_its_records_class() {
        let fire = operative(JOB, FIRE);
        let sized = Record { value: 200, ..fire };
        assert_eq!(
            dispatch(None, Some(&sized), 1),
            Dispatch::Job(Job::Fire { size: 200 })
        );
        assert_eq!(dispatch(None, Some(&sized), 0), Dispatch::None);
        let light = operative(JOB, LIGHT);
        assert_eq!(
            dispatch(None, Some(&light), 3),
            Dispatch::Job(Job::Light { on: true })
        );
        assert_eq!(
            dispatch(None, Some(&light), 2),
            Dispatch::Job(Job::Light { on: false })
        );
        for (kind, relay) in [
            (0, Relay::Combine),
            (1, Relay::Broadcast),
            (2, Relay::Both),
            (3, Relay::Second),
            (5, Relay::Sequence),
        ] {
            assert_eq!(
                dispatch(None, Some(&operative(RELAY, kind)), 1),
                Dispatch::Relay(relay)
            );
        }
        assert_eq!(
            dispatch(None, Some(&operative(RELAY, 4)), 1),
            Dispatch::None
        );
        assert_eq!(
            dispatch(None, Some(&operative(PUSH_SURFACE, 1)), 0),
            Dispatch::Plate { pushed: false }
        );
        assert_eq!(
            dispatch(None, Some(&operative(PUSH_SURFACE, 1)), -1),
            Dispatch::Plate { pushed: true }
        );
        assert_eq!(dispatch(None, Some(&operative(KEY, 1)), 1), Dispatch::None);
    }

    #[test]
    fn a_fire_record_lights_up_unless_the_day_is_up() {
        assert_eq!(level_setup(None, 0.0), Setup::Missing);
        let switch = Record {
            flags: SWITCH_OFF,
            ..operative(JOB, FIRE)
        };
        assert_eq!(level_setup(Some(&switch), 0.0), Setup::SwitchOff);
        let electric = Record {
            value: 1,
            ..operative(JOB, LIGHT)
        };
        assert_eq!(level_setup(Some(&electric), 0.0), Setup::Electricity);
        assert_eq!(level_setup(Some(&operative(JOB, LIGHT)), 0.0), Setup::None);
        let fire = Record {
            class: FIRE_CLASS,
            value: 250,
            ..operative(JOB, FIRE)
        };
        assert_eq!(level_setup(Some(&fire), 0.0), Setup::Fire { size: 250 });
        assert_eq!(level_setup(Some(&fire), 0.12), Setup::Fire { size: 250 });
        assert_eq!(level_setup(Some(&fire), 0.13), Setup::Daylight);
        let other_class = Record { class: 3, ..fire };
        assert_eq!(level_setup(Some(&other_class), 0.0), Setup::None);
    }

    fn sliding() -> Shape {
        Shape {
            sliding: true,
            travel_link: 0,
            sign: 1.0,
            radius: 0.0,
            half_swing: 0.0,
            stop_slack: 0.0,
            axis: bevy::math::Vec3::X,
        }
    }

    fn hinge() -> Shape {
        Shape {
            sliding: false,
            travel_link: 0,
            sign: 1.0,
            radius: 100.0,
            half_swing: std::f32::consts::FRAC_PI_6,
            stop_slack: 50.0,
            axis: bevy::math::Vec3::X,
        }
    }

    fn shutter(flags: u32) -> Door {
        Door::new(
            DoorRecord {
                flags,
                ..DoorRecord::default()
            },
            hinge(),
            0,
            0.52,
            0.0,
        )
    }

    #[test]
    fn the_doors_under_a_node_open_once_close_and_latch() {
        let mut leaves = [shutter(0), shutter(door::MOTORISED)];
        let sounds = open_doors(&mut leaves, 0);
        assert_eq!(sounds.0.len(), 1);
        assert_eq!(leaves[1].mode, door::MODE_WAIT);
        assert_eq!(leaves[1].countdown, door::SIGNAL_WAIT);
        assert_eq!(leaves[0].mode, door::MODE_IDLE);

        let mut sliders = [
            Door::new(DoorRecord::default(), sliding(), 0, 0.0, 1.0),
            Door::new(
                DoorRecord {
                    flags: door::FAST,
                    ..DoorRecord::default()
                },
                sliding(),
                0,
                0.0,
                1.0,
            ),
        ];
        let sounds = close_and_lock_doors(&mut sliders);
        assert_eq!(sounds.0.len(), 2);
        assert_eq!(sliders[0].slide.mode, crate::door::SlideMode::Idle);
        assert_eq!(sliders[1].slide.mode, crate::door::SlideMode::Close);
        assert_eq!(sliders[1].slide.countdown, door::SHUT_FAST);
        assert!(sliders[0].latched && sliders[1].latched);

        let mut plain = [shutter(0)];
        latch_doors(&mut plain, true);
        assert!(plain[0].latched && plain[0].saved & 1 == 1);
        latch_doors(&mut plain, false);
        assert!(!plain[0].latched && plain[0].saved & 1 == 0);
    }

    /// The toggle walks the node's doors in order: an unlatched door closes
    /// and locks the group, a latched one opens it, and the next door then
    /// decides again. A single unlatched door therefore closes and locks
    /// itself and stays latched.
    #[test]
    fn a_toggle_lets_each_doors_latch_decide_for_the_group() {
        let mut one = [shutter(0)];
        toggle_doors(&mut one, 0);
        assert!(one[0].latched);

        let mut latched = [shutter(0)];
        latched[0].latched = true;
        toggle_doors(&mut latched, 0);
        // The open is a 120-tick wait: the latch clears only when it ends.
        assert!(latched[0].latched);
        assert_eq!(latched[0].mode, door::MODE_WAIT);
        assert_eq!(latched[0].countdown, door::SIGNAL_WAIT);
        let mut tick = door::Tick::new(0, 0.52, 0.0, 0.0);
        let mut opened = None;
        for _ in 0..door::SIGNAL_WAIT {
            let done = latched[0].step(tick, None);
            if done.opened.is_some() {
                opened = done.opened;
            }
            tick.now_ms += 1;
        }
        assert!(opened.is_some(), "the wait ends with an open");
        assert!(!latched[0].latched);
        assert!(!latched[0].closed);
        // A plain hinge is only shoved open; its open command ends at once.
        assert_eq!(latched[0].mode, door::MODE_IDLE);

        // A motorised slider waits the same 120 ticks, then drives open.
        let mut slider = [Door::new(
            DoorRecord {
                flags: door::MOTORISED,
                ..DoorRecord::default()
            },
            sliding(),
            0,
            0.0,
            1.0,
        )];
        open_doors(&mut slider, 0);
        assert_eq!(slider[0].countdown, door::SIGNAL_WAIT);
        let mut tick = door::Tick::new(0, 0.0, 0.0, 1.0);
        for _ in 0..door::SIGNAL_WAIT {
            slider[0].step(tick, None);
            tick.now_ms += 1;
        }
        assert!(!slider[0].latched);
        assert_eq!(slider[0].mode, door::MODE_OPEN);
        // The open command starts at 300 and the same tick counts it down.
        assert_eq!(slider[0].slide.countdown, door::SLIDE_MOTOR_TICKS - 1);
    }

    #[test]
    fn a_node_tree_finds_lights_and_switches_them_off() {
        let mut nodes = vec![
            Node {
                class: 0x1000,
                children: vec![1, 3],
                ..Node::default()
            },
            Node {
                class: 0x05,
                children: vec![2],
                ..Node::default()
            },
            Node {
                class: LIT,
                light: Some(Light::default()),
                intensity: 1.0,
                ..Node::default()
            },
            Node {
                class: ELECTRIC,
                intensity: 1.0,
                ..Node::default()
            },
        ];
        assert_eq!(find(&nodes, 0, 0x5f, 5), Some(1));
        assert_eq!(find(&nodes, 0, 0xfff, LIGHT_CLASS), None);
        switch_off(&mut nodes, 0);
        assert_eq!(nodes[3].intensity, 0.0);
        switch_off(&mut nodes, 1);
        assert_eq!(nodes[2].intensity, 0.0);
        assert_eq!(nodes[2].light.unwrap().flags, 0x10);

        nodes[2].intensity = 1.0;
        put_out(&mut nodes, 1);
        assert_eq!(nodes[2].intensity, 0.0);

        nodes[0].class = LIGHT_CLASS;
        assert_eq!(find(&nodes, 0, 0xfff, LIGHT_CLASS), Some(0));
    }

    #[test]
    fn a_finished_sequence_clears_its_listeners_only() {
        let listeners = [
            Clearable {
                id: 1,
                class: 0x4000,
                flags: 0,
                listens_to: 8,
            },
            Clearable {
                id: 2,
                class: 0x4000,
                flags: 2,
                listens_to: 8,
            },
            Clearable {
                id: 3,
                class: 0x4000,
                flags: 0,
                listens_to: 9,
            },
            Clearable {
                id: 4,
                class: 0,
                flags: 0,
                listens_to: 8,
            },
        ];
        assert_eq!(listeners_to_clear(&listeners, 8), vec![1]);
    }

    #[test]
    fn a_fire_size_is_clamped_and_an_electric_object_lights_once() {
        assert!((fire_size(250) - 2.5).abs() < 1e-6);
        assert!((fire_size(1) - 0.25).abs() < 1e-6);
        assert!((fire_size(2000) - 10.0).abs() < 1e-6);
        assert!((fire_size(25) - 0.25).abs() < 1e-6);
        assert!((retention() - 0.9201).abs() < 1e-3);

        let mut nodes = vec![
            Node {
                class: 0x1000,
                children: vec![1],
                ..Node::default()
            },
            Node {
                class: 0x2100_0000,
                life: 0,
                powered: false,
                ..Node::default()
            },
        ];
        assert!(electric(&mut nodes, 0, 1));
        assert_eq!(nodes[1].life, 10);
        assert!(nodes[1].powered);
        nodes[1].life = 3;
        assert!(electric(&mut nodes, 0, 1));
        assert_eq!(nodes[1].life, 3, "an object already on is left alone");
        // A found object is triggered whatever the value; only a missing
        // one would use the value to decide whether to spawn an effect.
        assert!(electric(&mut nodes, 0, 0));
    }

    /// The shapes are the same as the door's own tests use; keep the imports
    /// honest.
    #[test]
    fn a_link_has_two_points() {
        let link = Link {
            on_body: [1.0, 2.0, 3.0, 4.0],
            on_parent: [1.0, 0.0, 0.0, 0.0],
            slack: 1.0,
            flags: 0,
        };
        let rest = [
            bevy::math::Vec3::ZERO,
            bevy::math::Vec3::X,
            bevy::math::Vec3::Y,
            bevy::math::Vec3::Z,
        ];
        assert_eq!(link.point(&rest), bevy::math::Vec3::new(2.0, 3.0, 4.0));
    }
}
