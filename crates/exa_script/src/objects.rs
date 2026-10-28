//! Story glue of the game unit: using a scripted world object, its glow
//! pulse, the raise-dead revelation, and the drain of charged power
//! crystals.
//!
//! Using an object goes by its record's type byte: 3 runs its script (one
//! actor slot, the player; at most once per 2.4 s across all such objects),
//! pulses its glow (1.2 s up, 1.2 s down) and plays the sound named in the
//! record at volume 0.3; 4 raises its event; 0x0F and 0x12 are teleporter
//! pedestals and devices; object 588 (0x24C) runs a once-per-session hook.
//! The revelation (event 0xDA0115, story mode only) teaches power 0x10F if
//! the player lacks it and narrates `RVL_RaiseDead_S` or `RVL_RaiseDead_F`
//! after a second. Charged crystals (item 499) lose 2 charge per game
//! second; a source stays in the list while its socket still carries the
//! charge flag, and leaves it when the socket or the flag is gone.
//!
//! **The glow task.** A scheduled pulse is a task with the object, its
//! start, the fade length and the hold length; each frame it writes a level
//! into the object's own emissive mesh (type `& 0x5f == 9`), its emissive
//! children and its lights (type `& 0x5f == 0x41`: base intensity × level).
//! Cancel removes every task. The task list is global and ordered.
//!
//! **The crystal drain.** A first pass over the player's two hand slots,
//! every further held item and the inventory fills the source list: a
//! crystal stack in the inventory is filed as the inventory source (kind
//! 3), any other charged item not in a hand as a socket source (kind 4).
//! Once per game second every source is drained in turn; containers (kind
//! 1), one container record (kind 2), the player's inventory (kind 3) and
//! a socketed item (kind 4) each answer whether anything changed, and a
//! source that changed nothing is replaced by the last one.
//!
//! Evidence: the object-use, script-run, glow-task, revelation and crystal
//! routines of the game unit and the drain constant (REA on `Exanima.exe`
//! 0.9.5).

/// Minimum time between two scripted-object uses, ms (one timer for all).
pub const USE_INTERVAL: u32 = 2400;
/// Glow pulse ramp of a used object, ms.
pub const PULSE_FADE: u32 = 1200;
/// Volume of a used object's sound.
pub const USE_VOLUME: f32 = 0.3;
/// The revelation's event code.
pub const RAISE_DEAD_EVENT: u32 = 0xDA_0115;
/// The power the revelation teaches.
pub const RAISE_DEAD_POWER: u32 = 0x10F;
/// Delay of the revelation's narration, ms.
pub const REVELATION_DELAY: u32 = 1000;
/// Item id of a power crystal.
pub const CRYSTAL: u32 = 499;
/// Charge drained per game second.
pub const DRAIN: u8 = 2;
/// The object whose use runs a once-per-session hook.
pub const SESSION_HOOK_OBJECT: u32 = 588;
/// The charge flag of a socketed item's flag word (the weapon form and the
/// apparel form each keep a flag word with a charge byte beside it).
pub const CHARGE_FLAG: u16 = 0x10;
/// The pulse a scripted object's use schedules: fade and hold, ms. The use
/// passes it as the object handler does (fade 1200, hold 0).
pub const USE_PULSE: (u32, u32) = (PULSE_FADE, 0);

/// What using an object does.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Use {
    /// Run the object's script, pulse its glow, play its sound (if named).
    Script {
        sound: Option<String>,
    },
    /// Too soon after the last scripted use: refused.
    TooSoon,
    /// Raise the record's event.
    Event(u32),
    TeleporterPedestal,
    TeleporterDevice,
    /// The once-per-session hook (only the first time).
    SessionHook,
    /// Nothing special: the use goes ahead.
    Plain,
    /// Refused (the hook already ran).
    Refused,
}

/// What a glow pulse drives on a child object.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Channel {
    /// An emissive mesh (the original's type `& 0x5f == 9`): its intensity.
    Emissive,
    /// A light (type `& 0x5f == 0x41`): its intensity is the base times
    /// the pulse level.
    Light,
}

/// The channel a child object of `kind` (its type word) shows a pulse on.
pub fn channel(kind: u32) -> Option<Channel> {
    match kind & 0x5f {
        9 => Some(Channel::Emissive),
        0x41 => Some(Channel::Light),
        _ => None,
    }
}

/// A light's intensity during a pulse: its base intensity times the level.
pub fn light_intensity(base: f32, level: f32) -> f32 {
    base * level
}

/// A used device of type 0x12 plays its power-up sound (the teleporter
/// device handler does it).
pub fn device_power_up(kind: u8) -> bool {
    kind == 0x12
}

/// The scripted-object state of a session.
#[derive(Debug, Clone, Default)]
pub struct Objects {
    /// Clock (ms) of the last scripted use.
    pub last_use: u32,
    pub hook_done: bool,
    /// The glow pulses running, oldest first.
    pub pulses: Vec<Pulse>,
    /// The charge sources being drained once a game second.
    pub drain: Drain,
}

impl Objects {
    /// Use an object of `kind` (its record's type byte), id `object`, event
    /// word `event`, sound name `sound` (empty: none), at clock `now`.
    pub fn use_object(&mut self, kind: u8, object: u32, event: u32, sound: &str, now: u32) -> Use {
        match kind {
            3 => {
                if u64::from(now) < u64::from(self.last_use) + u64::from(USE_INTERVAL) {
                    return Use::TooSoon;
                }
                self.last_use = now;
                self.schedule_pulse(object, USE_PULSE.0, USE_PULSE.1, now);
                Use::Script {
                    sound: (!sound.is_empty()).then(|| sound.chars().take(15).collect()),
                }
            }
            0x0F => Use::TeleporterPedestal,
            0x12 => Use::TeleporterDevice,
            4 => Use::Event(event),
            _ if object == SESSION_HOOK_OBJECT => {
                if self.hook_done {
                    Use::Refused
                } else {
                    self.hook_done = true;
                    Use::SessionHook
                }
            }
            _ => Use::Plain,
        }
    }

    /// Queue a glow pulse for `object` (the original's task creation).
    pub fn schedule_pulse(&mut self, object: u32, fade: u32, hold: u32, now: u32) {
        self.pulses.push(Pulse {
            object,
            start: now,
            fade,
            hold,
        });
    }

    /// Cancel every glow pulse (a session reset).
    pub fn cancel_pulses(&mut self) {
        self.pulses.clear();
    }

    /// One frame of the glow tasks: each writes its level into the targets
    /// and ends once its time is up. The finished task is dropped from the
    /// list, as the task frees itself. `lookup` answers an object's type
    /// word and its children.
    pub fn tick_pulses(
        &mut self,
        now: u32,
        lookup: impl Fn(u32) -> (u32, Vec<GlowChild>),
    ) -> Vec<GlowWrite> {
        let mut writes = Vec::new();
        self.pulses.retain(|p| {
            let Some(level) = pulse(now.wrapping_sub(p.start), p.fade, p.hold) else {
                return false;
            };
            let (kind, children) = lookup(p.object);
            writes.extend(pulse_object_kind(p.object, kind, &children, level));
            true
        });
        writes
    }
}

/// The glow level of a pulse `t` ms after it started: up over `fade`,
/// held for `hold`, down over `fade`; `None` once it is over (the task
/// ends).
pub fn pulse(t: u32, fade: u32, hold: u32) -> Option<f32> {
    if u64::from(fade) * 2 + u64::from(hold) < u64::from(t) {
        return None;
    }
    Some(if t < fade {
        (f64::from(t) / f64::from(fade)) as f32
    } else if u64::from(fade) + u64::from(hold) < u64::from(t) {
        (1.0 - (f64::from(t) - f64::from(hold) - f64::from(fade)) / f64::from(fade)) as f32
    } else {
        1.0
    })
}

/// What the revelation does: `None` outside story mode, for another
/// event, or when the player already has the power; otherwise the node to
/// narrate (after [`REVELATION_DELAY`]) given whether the power could be
/// taught.
pub fn revelation(story: bool, event: u32, has_power: bool, taught: bool) -> Option<&'static str> {
    if !story || event != RAISE_DEAD_EVENT || has_power {
        return None;
    }
    Some(if taught {
        "RVL_RaiseDead_S"
    } else {
        "RVL_RaiseDead_F"
    })
}

/// One drain of a charge: what is left, clamped at 0. The source is not
/// dropped for reaching 0; the original keeps it while the socket's charge
/// flag is still set.
pub fn drain(charge: u8) -> u8 {
    charge.saturating_sub(DRAIN)
}

/// The actor slots a scripted object's script runs with: one slot, the
/// player (the object itself is the interpreter's owner).
pub fn script_actors(player: u32) -> Vec<Option<u32>> {
    vec![Some(player)]
}

/// Where a charge sits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    /// Crystals in the level's containers (the original's kind 1).
    Containers,
    /// Crystals in one container record (kind 2, the record's id).
    Container(u32),
    /// Crystals in the player's inventory (kind 3).
    Inventory,
    /// A socketed weapon or apparel item (kind 4, the item's id).
    Socketed(u32),
}

impl Source {
    /// The kind word the original's source list stores beside the id.
    pub fn code(self) -> u8 {
        match self {
            Source::Containers => 1,
            Source::Container(_) => 2,
            Source::Inventory => 3,
            Source::Socketed(_) => 4,
        }
    }

    /// The id the source list stores (0 where the kind is enough).
    pub fn id(self) -> u32 {
        match self {
            Source::Containers | Source::Inventory => 0,
            Source::Container(id) | Source::Socketed(id) => id,
        }
    }
}

/// The charge sources being drained.
#[derive(Debug, Clone, Default)]
pub struct Drain {
    pub sources: Vec<Source>,
}

impl Drain {
    /// Add a source once.
    pub fn add(&mut self, s: Source) {
        if !self.sources.contains(&s) {
            self.sources.push(s);
        }
    }

    /// One game second: drain every source with `drain_source` (which
    /// reports whether the socket still has a charge flag to drain); a
    /// source whose probe answers false is replaced by the last one, as
    /// the original swaps it out.
    pub fn tick(&mut self, mut drain_source: impl FnMut(Source) -> bool) {
        let mut i = 0;
        while i < self.sources.len() {
            if drain_source(self.sources[i]) {
                i += 1;
            } else {
                self.sources.swap_remove(i);
            }
        }
    }
}

/// One glow pulse task: the object it drives, its start and the fade and
/// hold lengths (ms).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pulse {
    pub object: u32,
    pub start: u32,
    pub fade: u32,
    pub hold: u32,
}

/// A child object of a pulsing object, as the pulse walk reads it: its type
/// word and, for a light, its base intensity.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GlowChild {
    pub kind: u32,
    pub base: f32,
}

impl GlowChild {
    pub fn new(kind: u32, base: f32) -> Self {
        Self { kind, base }
    }
}

/// What one frame of a pulse writes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GlowWrite {
    /// The object's own emissive mesh (every submesh follows it).
    Mesh { object: u32, level: f32 },
    /// An emissive child mesh.
    ChildMesh {
        object: u32,
        child: usize,
        level: f32,
    },
    /// A child light: its base intensity times the level.
    Light {
        object: u32,
        child: usize,
        intensity: f32,
    },
}

/// The writes of one glow frame for an object of `kind` with `children`:
/// the object's own emissive mesh when `kind & 0x5f == 9`, then each child
/// whose type `& 0x5f` is above 8 — an emissive mesh (9) or a light (0x41,
/// intensity = base × level). Other child types are not touched.
pub fn pulse_object_kind(
    object: u32,
    kind: u32,
    children: &[GlowChild],
    level: f32,
) -> Vec<GlowWrite> {
    let mut out = Vec::new();
    if channel(kind) == Some(Channel::Emissive) {
        out.push(GlowWrite::Mesh { object, level });
    }
    for (i, child) in children.iter().enumerate() {
        let child_kind = child.kind & 0x5f;
        if child_kind <= 8 {
            continue;
        }
        match child_kind {
            9 => out.push(GlowWrite::ChildMesh {
                object,
                child: i,
                level,
            }),
            0x41 => out.push(GlowWrite::Light {
                object,
                child: i,
                intensity: light_intensity(child.base, level),
            }),
            _ => {}
        }
    }
    out
}

/// A socketed item's charge: the flag word and the charge byte beside it,
/// in either the weapon or the apparel form.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Socket {
    pub item: u32,
    pub flags: u16,
    pub charge: u8,
}

impl Socket {
    pub fn new(item: u32, flags: u16, charge: u8) -> Self {
        Self {
            item,
            flags,
            charge,
        }
    }

    /// One drain: two off the charge down to zero. Answers whether the
    /// charge flag is still set (the socket stays a source).
    pub fn drain(&mut self) -> bool {
        if self.flags & CHARGE_FLAG == 0 {
            return false;
        }
        self.charge = self.charge.saturating_sub(DRAIN);
        true
    }
}

/// An item stack as the container and inventory lists store it: the item
/// id and a word whose bits 16–23 are the charge.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Stack {
    pub item: u32,
    pub word: u32,
}

impl Stack {
    pub fn new(item: u32, charge: u8) -> Self {
        Self {
            item,
            word: u32::from(charge) << 16,
        }
    }

    /// The stack's charge.
    pub fn charge(&self) -> u8 {
        ((self.word & 0x00ff_0000) >> 16) as u8
    }

    /// One drain of a crystal stack: two off the charge, clamped at zero.
    /// Answers whether the charge was reduced (the original reports a
    /// change only then; a stack at or below two is cleared without being
    /// reported). A stack of another item is left alone.
    pub fn drain(&mut self) -> bool {
        if self.item != CRYSTAL {
            return false;
        }
        let charge = self.charge();
        if u32::from(DRAIN) < u32::from(charge) {
            self.word = (self.word & 0xff00_ffff) | (u32::from(charge - DRAIN) << 16);
            true
        } else {
            self.word &= 0xff00_ffff;
            false
        }
    }
}

/// The item data the crystal drain walks, as the original's object
/// database and inventory expose it.
#[derive(Debug, Clone, Default)]
pub struct Charges {
    /// Every item's socket, by item id (the object database).
    pub sockets: Vec<Socket>,
    /// Every loaded container's crystal stacks.
    pub containers: Vec<Vec<Stack>>,
    /// One container record's stacks, by record id.
    pub records: Vec<(u32, Vec<Stack>)>,
    /// The player's inventory stacks.
    pub inventory: Vec<Stack>,
}

impl Charges {
    /// The socketed item `item`'s drain (the object database lookup).
    fn drain_item(&mut self, item: u32) -> bool {
        self.sockets
            .iter_mut()
            .find(|s| s.item == item)
            .is_some_and(Socket::drain)
    }

    /// Every container's stacks (kind 1).
    fn drain_containers(&mut self) -> bool {
        let mut changed = false;
        for list in &mut self.containers {
            for stack in list {
                changed |= stack.drain();
            }
        }
        changed
    }

    /// One container record's stacks (kind 2).
    fn drain_record(&mut self, id: u32) -> bool {
        let mut changed = false;
        if let Some((_, list)) = self.records.iter_mut().find(|(r, _)| *r == id) {
            for stack in list {
                changed |= stack.drain();
            }
        }
        changed
    }

    /// The player's inventory stacks (kind 3).
    fn drain_inventory(&mut self) -> bool {
        let mut changed = false;
        for stack in &mut self.inventory {
            changed |= stack.drain();
        }
        changed
    }

    /// Drain the source once; answers whether anything changed.
    pub fn drain(&mut self, source: Source) -> bool {
        match source {
            Source::Containers => self.drain_containers(),
            Source::Container(id) => self.drain_record(id),
            Source::Inventory => self.drain_inventory(),
            Source::Socketed(id) => self.drain_item(id),
        }
    }
}

/// The player's item slots the collect pass reads: the two hand slots, the
/// further held list and the inventory's item ids.
#[derive(Debug, Clone, Default)]
pub struct Slots {
    /// The weapon slot and the other hand's item (0: empty).
    pub hands: [u32; 2],
    pub held: Vec<u32>,
    pub inventory: Vec<u32>,
}

/// The collect pass: try the two hand slots, every further held item, then
/// the inventory. A crystal stack in the inventory files the inventory
/// source (once, as the list dedups by kind and id); any other inventory
/// item not in a hand is tried as a socket. `drain_item` is the object
/// database's socket drain.
pub fn collect(
    slots: &Slots,
    drain_item: &mut impl FnMut(u32) -> bool,
    add: &mut impl FnMut(Source),
) {
    for &item in &slots.hands {
        if item != 0 && drain_item(item) {
            add(Source::Socketed(item));
        }
    }
    for &item in &slots.held {
        if item != 0 && drain_item(item) {
            add(Source::Socketed(item));
        }
    }
    for &item in &slots.inventory {
        if item == CRYSTAL {
            add(Source::Inventory);
        } else if item != 0 && !slots.hands.contains(&item) && drain_item(item) {
            add(Source::Socketed(item));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scripted_objects_wait_two_point_four_seconds() {
        let mut o = Objects::default();
        assert_eq!(
            o.use_object(3, 7, 0, "lever01", 5000),
            Use::Script {
                sound: Some("lever01".into())
            }
        );
        assert_eq!(o.use_object(3, 8, 0, "", 7399), Use::TooSoon);
        assert_eq!(o.use_object(3, 8, 0, "", 7400), Use::Script { sound: None });
        assert_eq!(
            o.use_object(4, 9, 0xC1_0EC0, "", 7401),
            Use::Event(0xC1_0EC0)
        );
        assert_eq!(o.use_object(0, 588, 0, "", 0), Use::SessionHook);
        assert_eq!(o.use_object(0, 588, 0, "", 0), Use::Refused);
        assert_eq!(o.use_object(0x12, 1, 0, "", 0), Use::TeleporterDevice);
        assert_eq!(o.use_object(1, 1, 0, "", 0), Use::Plain);
    }

    #[test]
    fn the_pulse_rises_holds_and_falls() {
        assert_eq!(pulse(600, PULSE_FADE, 0), Some(0.5));
        assert_eq!(pulse(1200, PULSE_FADE, 0), Some(1.0));
        assert_eq!(pulse(1800, PULSE_FADE, 0), Some(0.5));
        assert_eq!(pulse(2400, PULSE_FADE, 0), Some(0.0));
        assert_eq!(pulse(2401, PULSE_FADE, 0), None);
        assert_eq!(pulse(1500, 1000, 1000), Some(1.0));
    }

    #[test]
    fn the_revelation_narrates_success_or_failure() {
        assert_eq!(
            revelation(true, RAISE_DEAD_EVENT, false, true),
            Some("RVL_RaiseDead_S")
        );
        assert_eq!(
            revelation(true, RAISE_DEAD_EVENT, false, false),
            Some("RVL_RaiseDead_F")
        );
        assert_eq!(revelation(true, RAISE_DEAD_EVENT, true, true), None);
        assert_eq!(revelation(false, RAISE_DEAD_EVENT, false, true), None);
    }

    #[test]
    fn crystals_drain_two_a_second() {
        assert_eq!(drain(10), 8);
        assert_eq!(drain(3), 1);
        assert_eq!(drain(2), 0);
        assert_eq!(drain(1), 0);
        assert_eq!(script_actors(7), [Some(7)]);
        assert_eq!(Source::Containers.code(), 1);
        assert_eq!(Source::Container(9).code(), 2);
        assert_eq!(Source::Container(9).id(), 9);
        assert_eq!(Source::Inventory.code(), 3);
        assert_eq!(Source::Socketed(5).code(), 4);
        assert!(device_power_up(0x12));
        assert!(!device_power_up(0x0F));
        assert_eq!(channel(0x29), Some(Channel::Emissive));
        assert_eq!(channel(0x41), Some(Channel::Light));
        assert_eq!(channel(0x40), None);
        assert_eq!(light_intensity(2.0, 0.5), 1.0);
        let mut d = Drain::default();
        d.add(Source::Inventory);
        d.add(Source::Socketed(5));
        d.add(Source::Inventory);
        d.add(Source::Containers);
        assert_eq!(d.sources.len(), 3);
        // The inventory runs dry: the last source takes its place.
        d.tick(|s| s != Source::Inventory);
        assert_eq!(d.sources, [Source::Containers, Source::Socketed(5)]);
    }

    #[test]
    fn a_used_object_schedules_its_glow() {
        let mut o = Objects::default();
        o.use_object(3, 7, 0, "", 5000);
        assert_eq!(
            o.pulses,
            [Pulse {
                object: 7,
                start: 5000,
                fade: 1200,
                hold: 0
            }]
        );
        // The level rises, holds and falls; the task ends past 2.4 s.
        let children = vec![GlowChild::new(0x09, 0.0), GlowChild::new(0x41, 2.0)];
        let first = o.tick_pulses(5600, |_| (0x09, children.clone()));
        assert_eq!(
            first,
            [
                GlowWrite::Mesh {
                    object: 7,
                    level: 0.5
                },
                GlowWrite::ChildMesh {
                    object: 7,
                    child: 0,
                    level: 0.5
                },
                GlowWrite::Light {
                    object: 7,
                    child: 1,
                    intensity: 1.0
                },
            ]
        );
        // At 2.4 s the level is 0, one ms later the task is gone.
        assert_eq!(o.tick_pulses(7400, |_| (0x09, children.clone())).len(), 3);
        assert!(o.tick_pulses(7401, |_| (0x09, children.clone())).is_empty());
        assert!(o.pulses.is_empty());
        o.schedule_pulse(1, 1200, 0, 0);
        o.cancel_pulses();
        assert!(o.pulses.is_empty());
    }

    #[test]
    fn only_emissive_meshes_and_lights_take_a_pulse() {
        let children = [
            GlowChild::new(0x09, 0.0),
            GlowChild::new(0x41, 4.0),
            GlowChild::new(0x08, 9.0),
            GlowChild::new(0x49, 4.0),
        ];
        let writes = pulse_object_kind(3, 0x29, &children, 0.25);
        assert_eq!(
            writes,
            [
                GlowWrite::Mesh {
                    object: 3,
                    level: 0.25
                },
                GlowWrite::ChildMesh {
                    object: 3,
                    child: 0,
                    level: 0.25
                },
                GlowWrite::Light {
                    object: 3,
                    child: 1,
                    intensity: 1.0
                },
            ]
        );
        // A kind that is not emissive writes its children only.
        assert_eq!(
            pulse_object_kind(3, 0x40, &children, 0.0),
            [
                GlowWrite::ChildMesh {
                    object: 3,
                    child: 0,
                    level: 0.0
                },
                GlowWrite::Light {
                    object: 3,
                    child: 1,
                    intensity: 0.0
                },
            ]
        );
    }

    #[test]
    fn socketed_and_stacked_charges_drain_two() {
        let mut s = Socket::new(9, 0, 5);
        assert!(!s.drain());
        assert_eq!(s.charge, 5);
        s.flags = CHARGE_FLAG;
        assert!(s.drain());
        assert_eq!(s.charge, 3);
        assert!(s.drain());
        assert_eq!(s.charge, 1);
        // The socket keeps answering while its charge flag is set, even at
        // zero: the source stays listed.
        assert!(s.drain());
        assert_eq!(s.charge, 0);
        assert!(s.drain());
        assert_eq!(s.charge, 0);

        let mut stack = Stack::new(CRYSTAL, 10);
        assert_eq!(stack.charge(), 10);
        assert!(stack.drain());
        assert_eq!(stack.word, 8 << 16);
        stack.word = 3 << 16;
        assert!(stack.drain());
        assert_eq!(stack.charge(), 1);
        assert!(!stack.drain());
        assert_eq!(stack.charge(), 0);
        // Another item is left alone.
        let mut other = Stack::new(7, 9);
        assert!(!other.drain());
        assert_eq!(other.charge(), 9);
    }

    #[test]
    fn the_drain_walks_containers_records_and_the_inventory() {
        let mut c = Charges {
            sockets: vec![Socket::new(4, CHARGE_FLAG, 5)],
            containers: vec![vec![Stack::new(CRYSTAL, 6), Stack::new(1, 2)]],
            records: vec![(9, vec![Stack::new(CRYSTAL, 4)])],
            inventory: vec![Stack::new(CRYSTAL, 3)],
        };
        assert!(c.drain(Source::Containers));
        assert_eq!(c.containers[0][0].charge(), 4);
        assert_eq!(c.containers[0][1].charge(), 2);
        // Two drains run a record's stack down; the second clears it
        // without reporting a change.
        assert!(c.drain(Source::Container(9)));
        assert_eq!(c.records[0].1[0].charge(), 2);
        assert!(!c.drain(Source::Container(9)));
        assert_eq!(c.records[0].1[0].charge(), 0);
        assert!(!c.drain(Source::Container(10)));
        assert!(c.drain(Source::Inventory));
        assert_eq!(c.inventory[0].charge(), 1);
        assert!(!c.drain(Source::Inventory));
        assert_eq!(c.inventory[0].charge(), 0);
        assert!(c.drain(Source::Socketed(4)));
        assert_eq!(c.sockets[0].charge, 3);
        assert!(!c.drain(Source::Socketed(5)));
    }

    #[test]
    fn the_collect_pass_files_hands_held_and_inventory_sources() {
        let slots = Slots {
            hands: [4, 8],
            held: vec![5],
            inventory: vec![CRYSTAL, 4, 9, 6],
        };
        let mut charged = vec![4u32, 5, 9];
        let mut drains = |item: u32| {
            let had = charged.contains(&item);
            charged.retain(|&i| i != item);
            had
        };
        let mut found = Vec::new();
        collect(&slots, &mut drains, &mut |s| found.push(s));
        // 4 and 8 are tried as hands (4 charged, 8 not); 5 as held; the
        // inventory's crystal is an inventory source, 4 is already a hand,
        // 9 and 6 are sockets (only 9 still charged).
        assert_eq!(
            found,
            [
                Source::Socketed(4),
                Source::Socketed(5),
                Source::Inventory,
                Source::Socketed(9)
            ]
        );
    }
}
