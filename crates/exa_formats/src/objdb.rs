//! Item records of the object database (`objdb.rdb`): names, the models
//! they are shown and worn as, what they occupy, and the appearance block
//! that colours apparel. Layout and evidence: `docs/formats/OTHER.md`
//! (Object database).
//!
//! A record payload is a chain of `u32 tag, u32 size` sections (the size
//! counts the 8-byte header): `0x10000000`
//! holds the display name, `0x00100000` the ground model's name, `0x1000`
//! the item body, which starts with the worn model's name, holds the
//! per-bone occupancy table at `0x10`, the protection and encumbrance at
//! `0x150..0x1A7`, the appearance block at bytes `0x1B0..0x214` and the
//! special kind, power flags and articulated set after it; `0x4000` a
//! weapon's section and `0x8000` a shield's; `0x40000` a door's and
//! `0x10000` an operative object's (levers, buttons, pressure plates, keys,
//! signal jobs and relays).
//!
//! [`Store`] holds one tier as the original does, an id-sorted table over a
//! payload blob, with its search, insertion, payload store, deletion and
//! writer (a loaded database writes back byte for byte). [`ObjectDb`] holds
//! the three tiers (base templates, a level's local objects and the
//! promoted world objects) and their string libraries, resolves tagged
//! ids, reads display names and descriptions through their string ids, and
//! instantiates, duplicates, promotes, removes and migrates records between
//! the tiers. [`Record`] exposes a payload's sections and their header and
//! model fields; [`objstrings::Library`](crate::objstrings::Library)
//! is the string pool. Layout and evidence: `docs/formats/OTHER.md` (Object
//! database).

use crate::rdb::Database;
use crate::weapon_record::ShieldRecord;
use crate::weapon_stats::StatBytes;
use crate::{Error, Result};

const NAME: u32 = 0x1000_0000;
const MODEL: u32 = 0x0010_0000;
const BODY: u32 = 0x1000;
const WEAPON: u32 = 0x4000;
const SHIELD: u32 = 0x8000;
const OPERATIVE: u32 = 0x1_0000;
const DOOR: u32 = 0x4_0000;
/// Model flag: the item's voxel lights are scaled by (1 − c/255)², with c
/// the condition byte of its placement (a salve's charge).
pub const CONDITION_SCALED: u32 = 1;
/// The weapon's stored part choices and materials are valid.
pub const WEAPON_STORED: u32 = 0x1000_0000;
const APPEARANCE_OFFSET: usize = 0x1B0;
/// Per-bone occupancy of apparel: a `u64` for each of bones 1..20, whose
/// byte L holds the sub-parts of the bone the item occupies on layer L.
const OCCUPANCY_OFFSET: usize = 0x10;
pub const OCCUPANCY_BONES: usize = 20;
/// The pelvis sub-part an innermost-layer item covers the groin with.
pub const GROIN: u8 = 0x02;
/// Per-slot layer bytes of apparel: 8 bytes per body slot (bone index).
const SLOT_LAYERS_OFFSET: usize = 0xA8;
pub const SLOTS: usize = 19;
/// A slot byte with this bit hides the body part of that slot.
pub const HIDES_BODY: u8 = 0x80;
const APPEARANCE_LEN: usize = 4 + LAYERS * LAYER_LEN;
/// Apparel protection: per-bone coverage (u32 percentages, bones 1..18),
/// then padding, slash, crush, pierce, coverage display, encumbrance and
/// hit-sound material bytes.
const COVERAGE_OFFSET: usize = 0x150;
const PROTECTION_OFFSET: usize = 0x1A0;
pub const BONES: usize = 18;
/// What a worn item weighs: its encumbrance byte, its special kind, the
/// powered flag word and charge byte, and the articulated set it belongs to.
const ENCUMBRANCE_OFFSET: usize = 0x1A5;
const SPECIAL_OFFSET: usize = 0x214;
const POWER_FLAGS_OFFSET: usize = 0x218;
const CHARGE_OFFSET: usize = 0x219;
const SET_OFFSET: usize = 0x220;
/// The powered flag (bit 0 of the flag byte) and the charge of a spent
/// crystal.
const POWERED: u8 = 1;
const SPENT: u8 = 0xFF;
const LAYERS: usize = 8;
const LAYER_LEN: usize = 12;

#[derive(Debug, Clone, PartialEq)]
pub struct Item {
    pub id: u32,
    pub name: String,
    /// The model the item shows on the ground or as an icon (an entry of
    /// `Objects.rpk` or `Objlib.rpk`, or `@` and a factory name); may be
    /// empty, as for the underwear.
    pub model: String,
    /// The model section's word after the name: bit 0 ([`CONDITION_SCALED`])
    /// dims the model's glow by the placed object's condition byte.
    pub model_flags: u32,
    /// The `Characters.rpk` entry the item is worn as, named by its body;
    /// empty without a body.
    pub worn_model: String,
    pub appearance: Option<Appearance>,
    /// Apparel's first layer byte per body slot: low bits the layer, bit
    /// `0x80` hides the body part, `0x40` its partial variant.
    pub slot_layers: [u8; SLOTS],
    /// The whole eight-byte coverage word per body slot (layer L is byte
    /// L): the character builder ors these over the worn set into the
    /// union its coverage pass tests each garment's occupancy against.
    pub slot_words: [u64; SLOTS],
    /// Sub-parts occupied per bone 1..20 (index 0 is the pelvis): byte L of
    /// each entry is layer L, 0 the innermost. Items that overlap here are
    /// not worn together. Zero when the body is too short to hold it.
    pub occupancy: [u64; OCCUPANCY_BONES],
    /// Seeds, condition and choices of a factory-made weapon.
    pub weapon: Option<Weapon>,
    /// What the item protects when worn.
    pub armour: Option<Armour>,
    /// What the item blocks with when held as a shield.
    pub shield: Option<ShieldRecord>,
    /// How the object works as a door.
    pub door: Option<Door>,
    /// What the object signals or does when operated.
    pub operative: Option<Operative>,
    /// What the item weighs on its wearer: every item with a body.
    pub burden: Option<Burden>,
    /// The byte `0x24` bytes on from the start of the model section (its
    /// header included; body offset `0x1C`), 0 without a model section or
    /// past the record. The equip conflict pass will not take off an
    /// overlapping worn item for which it is over [`EQUIP_LOCK_LIMIT`].
    /// Heavy furniture stores 200 or 255 and the golem suit and the ogre's
    /// armour 100, so it reads as a weight class (a guess); 121 shipped
    /// items are over the limit, 7 of them apparel.
    pub equip_lock: u8,
    /// The item has the torch section ([`TORCH`]): the equip pass holds it
    /// in the off hand on its own path.
    pub torch: bool,
    /// The object's volume section ([`VOLUME`]): its box extents, its flags
    /// and, when the flags' low byte is [`Volume::MARKER`], the entrance and
    /// previous level id of a level transition.
    pub volume: Option<Volume>,
    /// The item's rank word: the body's word at `0x1AB`, else a weapon's at
    /// `0x42`, else a shield's at `0x08` (see `docs/formats/OTHER.md`, Trade
    /// fields). The low five bits are the bulk in points and the low byte's
    /// top three bits the rank.
    pub rank_word: u16,
    /// The body's armour type byte at `0x1A8` (the arena's apparel class),
    /// 0 without a body.
    pub armour_type: u8,
}

/// An object's volume section: a box the level places, and for a level
/// transition the words the stream compares against the session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Volume {
    /// Half extents in centimetres, in the order the section stores them.
    pub extents: [u16; 3],
    /// The section's flags word; only its low byte is read here.
    pub flags: u32,
    /// The entrance number (the session's arrival variable) a transition
    /// marker names.
    pub entrance: u32,
    /// The id, in `locales.rdb`, of the level the marker leads back from.
    pub previous_level: u32,
}

impl Volume {
    /// The flags word's low byte of a level-transition marker.
    pub const MARKER: u32 = 0x40;

    /// Reads a volume section's body: three 16-bit extents, a zeroed word,
    /// then the flags, the entrance and the previous level id.
    pub fn parse(b: &[u8]) -> Option<Self> {
        let b = b.get(..20)?;
        // The slice was checked to be 20 bytes, so these widths fix.
        let u16_at = |o: usize| u16::from_le_bytes(b[o..o + 2].try_into().expect("2 bytes"));
        let u32_at = |o: usize| u32::from_le_bytes(b[o..o + 4].try_into().expect("4 bytes"));
        Some(Self {
            extents: [u16_at(0), u16_at(2), u16_at(4)],
            flags: u32_at(8),
            entrance: u32_at(12),
            previous_level: u32_at(16),
        })
    }

    /// The level-transition marker's two session words, in the order the
    /// stream compares them: `[entrance, previous level]`. `None` for a
    /// volume that is not a marker.
    pub fn marker(&self) -> Option<[u32; 2]> {
        (self.flags & 0xFF == Self::MARKER).then_some([self.entrance, self.previous_level])
    }
}

/// An overlapping worn item whose [`Item::equip_lock`] is over this cannot
/// be displaced by a new one.
pub const EQUIP_LOCK_LIMIT: u8 = 50;
/// Where [`Item::equip_lock`] lies from the model section's start.
const EQUIP_LOCK_OFFSET: usize = 0x24;

/// What a worn item weighs on its wearer, as the original's armour pass
/// reads the item body: its encumbrance, the bones it covers and the
/// articulated set it belongs to, and its special kind.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Burden {
    /// ×0.0025 of the wearer's encumbrance. An item made from a template
    /// stores it divided by its condition already.
    pub encumbrance: u8,
    /// Bones 1..18 the item covers at all, bit `bone − 1`.
    pub bones: u32,
    /// The articulated set the piece belongs to (the item id of the set's
    /// central piece); 0 for an ordinary item.
    pub set: u32,
    /// Special kind: [`Burden::GOLEM_SUIT`], [`Burden::LIGHT_SHIELD`], …
    pub special: u8,
    /// Its power crystal is in and not spent (flag word bit 0 set, charge
    /// byte not 0xFF): a golem suit runs, the gauntlets lighten the shield.
    pub powered: bool,
}

impl Burden {
    /// A helm worn to see in the dark: it turns the player's own light on
    /// (the three night-vision helms of the shipped database).
    pub const SEES_IN_THE_DARK: u8 = 1;
    /// A worn talisman that grants its wearer a second wind at low health.
    pub const SECOND_WIND: u8 = 2;
    /// The golem suit: unpowered, it adds 2 to its wearer's encumbrance.
    pub const GOLEM_SUIT: u8 = 4;
    /// Gauntlets that make a held shield weightless while powered.
    pub const LIGHT_SHIELD: u8 = 6;

    /// The burden of an item body; bytes past its end read as zero.
    fn parse(body: &[u8]) -> Self {
        let byte = |o: usize| body.get(o).copied().unwrap_or(0);
        let word = |o: usize| {
            body.get(o..o + 4)
                .map_or(0, |w| u32::from_le_bytes(w.try_into().expect("4 bytes")))
        };
        let bones = (0..BONES)
            .filter(|&i| word(COVERAGE_OFFSET + 4 * i) != 0)
            .fold(0, |m, i| m | 1 << i);
        Self {
            encumbrance: byte(ENCUMBRANCE_OFFSET),
            bones,
            set: word(SET_OFFSET),
            special: byte(SPECIAL_OFFSET),
            powered: byte(POWER_FLAGS_OFFSET) & POWERED != 0 && byte(CHARGE_OFFSET) != SPENT,
        }
    }
}

/// The special kind byte of an item body ([`Burden::special`]); 0 when the
/// body is too short to hold it.
pub fn special_kind(body: &[u8]) -> u8 {
    body.get(SPECIAL_OFFSET).copied().unwrap_or(0)
}

/// Door flags (the door section's first word).
pub mod door {
    /// Bars itself shut when it closes (secret doors).
    pub const LATCHES: u32 = 0x2;
    pub const LOCKED: u32 = 0x4;
    /// Not a door to the hand's click.
    pub const NOT_CLICKABLE: u32 = 0x8;
    /// Opens and closes by itself when signalled.
    pub const MOTORISED: u32 = 0x10;
    /// A fast motor, toggling in 60 ticks.
    pub const FAST: u32 = 0x40;
    /// Signals 0 and 1 swap meaning.
    pub const INVERTED: u32 = 0x80;
    /// A sliding door held at its file position.
    pub const FROZEN: u32 = 0x100;
    /// Sets a scene-node flag (`0x80`) on the door's node; its meaning is
    /// not established.
    pub const NODE_MARK: u32 = 0x200;
}

/// The door section of an object (`0x40000`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Door {
    /// [`door`] flags.
    pub flags: u32,
    /// The object the door is linked to: the key that unlocks it, and the
    /// relay whose broadcast moves it. Ids carry the database tag bits
    /// ([`crate::rfc::PlacedObject`]); 0 for none.
    pub key: u32,
    /// Sound letters (`a` + letter in the sound's name, see [`door_sound`]):
    /// unlock and locked (`LkO`, `LkT`), slam (`slm`), hinge creak loop
    /// (`hng`) and unlatch (`opn`).
    pub lock_sound: u8,
    pub slam_sound: u8,
    pub hinge_sound: u8,
    pub unlatch_sound: u8,
}

impl Door {
    fn parse(b: &[u8]) -> Option<Self> {
        let b = b.get(..12)?;
        Some(Self {
            flags: u32::from_le_bytes(b[0..4].try_into().expect("4 bytes")),
            key: u32::from_le_bytes(b[4..8].try_into().expect("4 bytes")),
            lock_sound: b[8],
            slam_sound: b[9],
            hinge_sound: b[10],
            unlatch_sound: b[11],
        })
    }

    pub fn has(&self, flag: u32) -> bool {
        self.flags & flag != 0
    }
}

/// The name of a door sound: `door`, the letter as `a` + letter, `_` and
/// the suffix (`doora_slm`).
pub fn door_sound(letter: u8, suffix: &str) -> String {
    format!("door{}_{suffix}", letter.wrapping_add(b'a') as char)
}

/// Operative classes and kinds (the operative section's first two bytes).
pub mod operative {
    /// The class of the devices that read an item on their `[` slot:
    /// teleporters, reconstruction devices and the pedestal hand.
    pub const DEVICE: u8 = 2;
    pub const KEY: u8 = 1;
    pub const LEVER: u8 = 3;
    /// Buttons and pressure plates.
    pub const PUSH_SURFACE: u8 = 5;
    /// A teleporter: the destination screen.
    pub const TELEPORTER: u8 = 0x0F;
    /// A reconstruction device: the scan before a transition.
    pub const RES_DEVICE: u8 = 0x12;
    /// A deferred job run on a signal: kind [`FIRE`] or [`LIGHT`].
    pub const JOB: u8 = 6;
    /// Forwards signals: kind 0 and 3 combine inputs into a code, 1
    /// broadcasts to every object linked to it, 2 signals both targets, 5
    /// counts a sequence.
    pub const RELAY: u8 = 7;
    /// Job kind: lights a fire of the stored size.
    pub const FIRE: u8 = 0xC;
    /// Job kind: turns a light on or off.
    pub const LIGHT: u8 = 0xD;
    /// Lever kind: the lever fades a light under its node instead of
    /// signalling.
    pub const LEVER_LIGHT: u8 = 6;
    /// Push-surface kind: the signals are inverted.
    pub const PUSH_INVERTED: u8 = 2;
}

/// A teleporter's record holds up to this many destination level ids,
/// from byte 32 of its operative body.
pub const DESTINATIONS: usize = 7;

/// The operative section of an object (`0x10000`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Operative {
    /// [`operative`] class.
    pub class: u8,
    /// Kind within the class.
    pub kind: u8,
    /// Class-specific flags (a lever's starting value and forced detent, a
    /// push surface's one-shot and second target, a relay's inversion).
    pub flags: u16,
    /// A job's fire size, a relay's state or a source's bit.
    pub value: u16,
    /// A lever's auto-return time in units of 100 ms (0: stays).
    pub time: u16,
    /// The object signalled first.
    pub target: u32,
    /// The second target, or a relay's code.
    pub second: u32,
    /// A teleporter's symbol atlas name (16 bytes, zero padded), from
    /// bytes 16..32 of the body; empty when the section is short.
    pub atlas: [u8; 16],
    /// A teleporter's destination level ids (the locale list's ids), from
    /// byte 32 of the body; 0 for an empty entry.
    pub destinations: [u32; DESTINATIONS],
}

impl Operative {
    fn parse(b: &[u8]) -> Option<Self> {
        let full = b.get(..60);
        let b = b.get(..16)?;
        let u16_at = |o: usize| u16::from_le_bytes(b[o..o + 2].try_into().expect("2 bytes"));
        let u32_at = |o: usize| u32::from_le_bytes(b[o..o + 4].try_into().expect("4 bytes"));
        let mut atlas = [0u8; 16];
        let mut destinations = [0u32; DESTINATIONS];
        if let Some(full) = full {
            atlas.copy_from_slice(&full[0x10..0x20]);
            for (i, d) in destinations.iter_mut().enumerate() {
                let at = 0x20 + 4 * i;
                *d = u32::from_le_bytes(full[at..at + 4].try_into().expect("4 bytes"));
            }
        }
        Some(Self {
            class: b[0],
            kind: b[1],
            flags: u16_at(2),
            value: u16_at(4),
            time: u16_at(6),
            target: u32_at(8),
            second: u32_at(12),
            atlas,
            destinations,
        })
    }

    /// A lever's auto-return time in milliseconds.
    pub fn return_ms(&self) -> u32 {
        u32::from(self.time) * 100
    }

    /// The teleporter's symbol atlas name, without its zero padding. A name
    /// that is not UTF-8 reads as empty.
    pub fn atlas_name(&self) -> &str {
        let end = self
            .atlas
            .iter()
            .position(|&b| b == 0)
            .unwrap_or(self.atlas.len());
        std::str::from_utf8(&self.atlas[..end]).unwrap_or("")
    }

    /// The non-zero destination level ids, in the record's order.
    pub fn reachable(&self) -> impl Iterator<Item = u32> + '_ {
        self.destinations.iter().copied().filter(|&id| id != 0)
    }
}

/// The body class an object's model is given: what the original makes of
/// the rigid body of its model's root.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Mechanism {
    Door,
    Lever,
    PushSurface,
}

/// Protection of a worn item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Armour {
    /// Coverage per bone 1..18, percent.
    pub coverage: [u8; BONES],
    /// Padding (×0.005) and slash, crush and pierce resistances (×1/140).
    pub padding: u8,
    pub slash: u8,
    pub crush: u8,
    pub pierce: u8,
    /// ×0.0025 of the wearer's encumbrance.
    pub encumbrance: u8,
    /// Hit sound class: 0 flesh, 1 cloth, 2 leather, 3 mail, 4 plate, 6 golem.
    pub material: u8,
}

impl Armour {
    fn parse(body: &[u8]) -> Option<Self> {
        let cov = body.get(COVERAGE_OFFSET..COVERAGE_OFFSET + 4 * BONES)?;
        let p = body.get(PROTECTION_OFFSET..PROTECTION_OFFSET + 7)?;
        let coverage: [u8; BONES] = std::array::from_fn(|i| {
            u32::from_le_bytes(cov[4 * i..4 * i + 4].try_into().expect("4 bytes")).min(100) as u8
        });
        coverage.iter().any(|&c| c != 0).then_some(Self {
            coverage,
            padding: p[0],
            slash: p[1],
            crush: p[2],
            pierce: p[3],
            encumbrance: p[5],
            material: p[6],
        })
    }
}

/// The weapon section of an item (`0x4000`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Weapon {
    pub flags: u32,
    pub seeds: [u32; 2],
    pub quality: u8,
    pub damage: u8,
    pub dirt: u8,
    /// Up to four material slots: (material, finish, colour).
    pub materials: [(u8, u8, u8, u8); 4],
    /// 1-based variant per factory part (0: none).
    pub selection: [u8; 10],
    /// Weight, balance, slash, crush, pierce, thrust… (`weapon_stats`).
    pub stats: StatBytes,
}

impl Weapon {
    fn parse(b: &[u8]) -> Option<Self> {
        if b.len() < 0x5A {
            return None;
        }
        let u32_at = |o: usize| u32::from_le_bytes(b[o..o + 4].try_into().expect("4 bytes"));
        Some(Self {
            flags: u32_at(0),
            seeds: [u32_at(4), u32_at(8)],
            quality: b[0x0C],
            damage: b[0x0D],
            dirt: b[0x0E],
            materials: std::array::from_fn(|k| {
                let m = &b[0x10 + 8 * k..0x18 + 8 * k];
                (m[0], m[3], m[4], m[7])
            }),
            selection: std::array::from_fn(|i| b[0x50 + i]),
            stats: StatBytes::from_section(b)?,
        })
    }

    pub fn has_stored_choices(&self) -> bool {
        self.flags & WEAPON_STORED != 0
    }

    /// The choices the section stores, in the factory's form.
    pub fn choices(&self) -> crate::factory::Choices {
        crate::factory::Choices {
            selection: self.selection.to_vec(),
            areas: self
                .materials
                .iter()
                .map(
                    |&(material, finish, colour, mode)| crate::factory::AreaChoice {
                        material,
                        finish,
                        colour,
                        mode,
                    },
                )
                .collect(),
        }
    }

    /// What a factory needs from this item to build its weapon. `metal` is
    /// the item palette's metal colour, which the painted-steel area class
    /// takes ([`crate::factory::DEFAULT_METAL`] when no palette is known).
    pub fn spec(&self, metal: u8) -> crate::factory::WeaponSpec {
        crate::factory::WeaponSpec {
            flags: self.flags,
            seeds: self.seeds,
            quality: self.quality,
            metal,
            choices: self.has_stored_choices().then(|| self.choices()),
        }
    }

    /// Writes choices back, as the original does when it rolls and stores
    /// them on a fresh item; the flags then mark them stored and shape-rerolled.
    pub fn apply_choices(&mut self, choices: &crate::factory::Choices, seeds: [u32; 2]) {
        self.seeds = seeds;
        self.flags |= crate::factory::STORED_AND_REROLLED;
        for (slot, value) in self.selection.iter_mut().zip(&choices.selection) {
            *slot = *value;
        }
        for (slot, area) in self.materials.iter_mut().zip(&choices.areas) {
            *slot = (area.material, area.finish, area.colour, area.mode);
        }
    }
}

impl Item {
    /// The coverage word of body slot `slot` (1..18).
    pub fn slot_word(&self, slot: usize) -> u64 {
        self.slot_words.get(slot).copied().unwrap_or(0)
    }

    /// Body slots (bone indices) this item hides, as a bit mask: a slot
    /// whose first (innermost) layer byte has the body-hide bit. The
    /// character builder tests only that byte of the worn set's coverage
    /// union when it decides which body parts to merge.
    pub fn hidden_slots(&self) -> u32 {
        self.slot_layers
            .iter()
            .enumerate()
            .filter(|(_, b)| **b & HIDES_BODY != 0)
            .fold(0, |m, (s, _)| m | 1 << s)
    }

    /// The share of its voxel lights' intensity a placed copy keeps:
    /// (1 − c/255)² with c bits 16–23 of the placement's flags word, for
    /// items whose model flags carry [`CONDITION_SCALED`]; 1 otherwise.
    pub fn light_share(&self, placement_flags: u32) -> f32 {
        if self.model_flags & CONDITION_SCALED == 0 {
            return 1.0;
        }
        let c = ((placement_flags >> 16) & 0xFF) as f32;
        (1.0 - c / 255.0).powi(2)
    }

    /// Whether the item covers the groin on the innermost layer, as
    /// trousers, hose and braies do.
    pub fn covers_groin(&self) -> bool {
        self.occupancy[0] as u8 & GROIN != 0
    }

    /// The record of a teleporter: a device whose operative kind opens the
    /// destination screen.
    pub fn teleporter(&self) -> Option<Operative> {
        self.operative
            .filter(|o| o.class == operative::DEVICE && o.kind == operative::TELEPORTER)
    }

    /// The record of a reconstruction device: a device whose operative kind
    /// scans a body before a transition.
    pub fn res_device(&self) -> Option<Operative> {
        self.operative
            .filter(|o| o.class == operative::DEVICE && o.kind == operative::RES_DEVICE)
    }

    /// The mechanism its model's rigid body becomes: a lever or push
    /// surface by its operative class, else a door when it has a door
    /// section. An operative object of another class stays a plain body
    /// even with a door section.
    pub fn mechanism(&self) -> Option<Mechanism> {
        match (self.operative, self.door) {
            (Some(o), _) => match o.class {
                operative::LEVER => Some(Mechanism::Lever),
                operative::PUSH_SURFACE => Some(Mechanism::PushSurface),
                _ => None,
            },
            (None, Some(_)) => Some(Mechanism::Door),
            (None, None) => None,
        }
    }
}

/// How an item is painted: condition and up to eight material layers, each
/// covering the area of one mask texture.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Appearance {
    pub quality: u8,
    pub damage: u8,
    pub dirt: u8,
    pub layers: [Layer; LAYERS],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Layer {
    /// Mask texture number (`<stem>m<n>`); 0 = unused layer.
    pub mask: u8,
    /// Material set to draw from when `material` is 0.
    pub random_set: u8,
    pub material: u8,
    /// Palette index (family × 8 + shade).
    pub colour: u8,
}

impl Appearance {
    pub fn parse(b: &[u8]) -> Result<Self> {
        let b = b.get(..APPEARANCE_LEN).ok_or(Error::Truncated {
            offset: 0,
            needed: APPEARANCE_LEN,
            len: b.len(),
        })?;
        let layers = std::array::from_fn(|i| {
            let l = &b[4 + i * LAYER_LEN..4 + (i + 1) * LAYER_LEN];
            Layer {
                mask: l[1],
                random_set: l[2],
                material: l[4],
                colour: l[8],
            }
        });
        Ok(Self {
            quality: b[0],
            damage: b[1],
            dirt: b[2],
            layers,
        })
    }

    /// Quality, damage and dirt as the painter uses them, 0..=1.
    pub fn condition(&self) -> (f32, f32, f32) {
        let q = self.quality as f32 / 255.0;
        let d = self.damage as f32 / 255.0;
        let r = self.dirt as f32 / 255.0;
        (q, d / (2.0 - d), (r * r + r) / 2.0)
    }

    /// The layers the original's texture job paints: every one of the
    /// eight that names a mask (its loop runs while the index is below 8).
    pub fn used_layers(&self) -> impl Iterator<Item = &Layer> {
        self.layers.iter().filter(|l| l.mask != 0)
    }
}

impl Item {
    pub fn parse(id: u32, payload: &[u8]) -> Result<Self> {
        let mut item = Item {
            id,
            name: String::new(),
            model: String::new(),
            model_flags: 0,
            worn_model: String::new(),
            appearance: None,
            slot_layers: [0; SLOTS],
            slot_words: [0; SLOTS],
            occupancy: [0; OCCUPANCY_BONES],
            weapon: None,
            armour: None,
            shield: None,
            door: None,
            operative: None,
            burden: None,
            equip_lock: 0,
            torch: false,
            volume: None,
            rank_word: 0,
            armour_type: 0,
        };
        let mut body_rank = None;
        let mut weapon_rank = None;
        let mut shield_rank = None;
        let mut pos = 0;
        while pos + 8 <= payload.len() {
            let tag = crate::read_u32(payload, pos)?;
            let size = crate::read_u32(payload, pos + 4)? as usize;
            if size < 8 {
                return Err(Error::Malformed(format!("item section of {size} bytes")));
            }
            let body = payload.get(pos + 8..pos + size).ok_or(Error::Truncated {
                offset: pos + 8,
                needed: size - 8,
                len: payload.len(),
            })?;
            match tag {
                NAME => item.name = name(body),
                MODEL => {
                    item.equip_lock = pos
                        .checked_add(EQUIP_LOCK_OFFSET)
                        .and_then(|at| payload.get(at))
                        .copied()
                        .unwrap_or(0);
                    item.model = name(body);
                    item.model_flags = body
                        .get(16..20)
                        .map_or(0, |w| u32::from_le_bytes(w.try_into().expect("4 bytes")));
                }
                BODY => {
                    item.worn_model = name(body);
                    body_rank = body
                        .get(0x1AB..0x1AD)
                        .map(|w| u16::from_le_bytes(w.try_into().expect("2 bytes")));
                    item.armour_type = body.get(0x1A8).copied().unwrap_or(0);
                    let table = OCCUPANCY_OFFSET..OCCUPANCY_OFFSET + OCCUPANCY_BONES * 8;
                    if let Some(t) = body.get(table) {
                        item.occupancy = std::array::from_fn(|b| {
                            u64::from_le_bytes(t[8 * b..8 * b + 8].try_into().expect("8 bytes"))
                        });
                    }
                    item.appearance = body
                        .get(APPEARANCE_OFFSET..)
                        .and_then(|b| Appearance::parse(b).ok());
                    if let Some(t) = body.get(SLOT_LAYERS_OFFSET..SLOT_LAYERS_OFFSET + SLOTS * 8) {
                        item.slot_layers = std::array::from_fn(|s| t[s * 8]);
                        item.slot_words = std::array::from_fn(|s| {
                            u64::from_le_bytes(t[s * 8..s * 8 + 8].try_into().expect("8 bytes"))
                        });
                    }
                    item.armour = Armour::parse(body);
                    item.burden = Some(Burden::parse(body));
                }
                WEAPON => {
                    weapon_rank = body
                        .get(0x42..0x44)
                        .map(|w| u16::from_le_bytes(w.try_into().expect("2 bytes")));
                    item.weapon = Weapon::parse(body);
                }
                SHIELD => {
                    shield_rank = body
                        .get(0x08..0x0A)
                        .map(|w| u16::from_le_bytes(w.try_into().expect("2 bytes")));
                    item.shield = ShieldRecord::from_section(body);
                }
                DOOR => item.door = Door::parse(body),
                OPERATIVE => item.operative = Operative::parse(body),
                TORCH => item.torch = true,
                VOLUME => item.volume = Volume::parse(body),
                _ => {}
            }
            pos += size;
        }
        item.rank_word = body_rank.or(weapon_rank).or(shield_rank).unwrap_or(0);
        Ok(item)
    }
}

fn name(b: &[u8]) -> String {
    let raw = &b[..b.len().min(16)];
    let end = raw.iter().position(|&c| c == 0).unwrap_or(raw.len());
    raw[..end].iter().map(|&c| c as char).collect()
}

/// Every item record of an object database that parses.
pub fn items(db: &Database<'_>) -> Vec<Item> {
    db.records
        .iter()
        .filter_map(|r| Item::parse(r.id, db.payload(r)).ok())
        .collect()
}

/// The first painted item of `items` worn as the `Characters.rpk` entry
/// `model` (case ignored): what a garment given only by its model is.
pub fn worn_as<'a>(items: &'a [Item], model: &str) -> Option<&'a Item> {
    items
        .iter()
        .find(|i| i.appearance.is_some() && i.worn_model.eq_ignore_ascii_case(model))
}

mod migrate;
mod packed;
mod record;
#[cfg(test)]
mod shipped;
mod store;
mod tiers;
mod upgrade;

pub use migrate::{material_set, variant_scales};
pub use packed::{
    FractionWord, Quantised, amount_add, amount_bytes, amount_compact, amount_expand, amount_sub,
    amount_total, amount_units,
};
pub use record::{
    Header, Model, Record, Section, Sections, append_section, remove_section, resize_section,
};
pub use store::{Store, TableEntry};
pub use tiers::{ObjectDb, Tier};

/// The object database's magic (`objdb.rdb` and a level's local database).
pub const MAGIC: u32 = 0xDBAC_0D09;
/// The id tag of the local tier: a level's own objects.
pub const LOCAL: u32 = 0x8000_0000;
/// The id tag of the world tier: objects promoted out of a level.
pub const WORLD: u32 = 0x4000_0000;
/// The two id tag bits.
pub const TAG_MASK: u32 = 0xC000_0000;
/// The record id inside a tagged id.
pub const ID_MASK: u32 = 0x3FFF_FFFF;
/// The model section's string id that means "inherit the template's".
pub const INHERIT: u32 = 0x8000_0000;
/// Header flag: the record is instantiated into the local tier when used.
pub const INSTANCE_ON_USE: u32 = 1;
/// A random table's section (`crate::containers::RandomTable`): its rows
/// carry the condition word [`FractionWord`] packs.
pub const RANDOM_TABLE: u32 = 0x200;
/// The map record's own section; its name follows a zeroed volume.
pub const MAP: u32 = 0x8000_0B03;
/// The torch's extra section; zero in the one shipped record, meaning not
/// established.
pub const TORCH: u32 = 0x100;
/// The object's volume section: a box, and for the level-transition marker
/// the entrance and previous level its flags word names (shipped in the
/// levels' own item records).
pub const VOLUME: u32 = 0x8000_AC07;
/// A container's contents: a count at body `0x14` and 12-byte child entries
/// from `0x18` (the child's id, then two words whose meanings are not
/// established).
pub const CONTAINER: u32 = 0x2_0000;

#[cfg(test)]
mod tests {
    use super::*;

    fn section(tag: u32, body: &[u8]) -> Vec<u8> {
        let mut v = tag.to_le_bytes().to_vec();
        v.extend((body.len() as u32 + 8).to_le_bytes());
        v.extend_from_slice(body);
        v
    }

    fn padded(s: &str, len: usize) -> Vec<u8> {
        let mut v = s.as_bytes().to_vec();
        v.resize(len, 0);
        v
    }

    #[test]
    fn item_names_and_appearance() {
        let mut body = vec![0u8; APPEARANCE_OFFSET];
        body[SLOT_LAYERS_OFFSET + 10 * 8] = 0x83; // hides the head
        body.extend([0x70, 0x40, 0x52, 0x92]);
        body.extend([0, 1, 12, 1, 13, 0, 0, 0, 3, 0, 0, 0]);
        body.extend([0, 2, 10, 1, 7, 0, 0, 0, 0x36, 0, 0, 0]);
        body.resize(APPEARANCE_OFFSET + APPEARANCE_LEN, 0);
        let mut p = section(NAME, &padded("Breastplate A", 0x20));
        p.extend(section(MODEL, &padded("BrPlate A", 0x30)));
        p.extend(section(BODY, &body));
        let item = Item::parse(1, &p).unwrap();
        assert_eq!(item.name, "Breastplate A");
        assert_eq!(item.model, "BrPlate A");
        let a = item.appearance.unwrap();
        assert_eq!(a.used_layers().count(), 2);
        assert_eq!(a.layers[0].material, 13);
        assert_eq!(a.layers[1].colour, 0x36);
        assert_eq!(item.hidden_slots(), 1 << 10);
        let (q, _, _) = a.condition();
        assert!((q - 0x70 as f32 / 255.0).abs() < 1e-6);
    }

    /// A volume section: three extents, a zeroed word, the flags, the
    /// entrance and the previous level id. Only a marker's flags low byte
    /// gives the stream a pair to compare.
    #[test]
    fn a_level_transition_marker_is_a_volume_section() {
        let mut body = Vec::new();
        for e in [50u16, 50, 100] {
            body.extend(e.to_le_bytes());
        }
        body.extend(0u16.to_le_bytes());
        body.extend(0x1040u32.to_le_bytes());
        body.extend(7u32.to_le_bytes());
        body.extend(11u32.to_le_bytes());
        body.resize(0x20, 0);
        let item = Item::parse(1, &section(VOLUME, &body)).unwrap();
        let volume = item.volume.unwrap();
        assert_eq!(volume.extents, [50, 50, 100]);
        assert_eq!(volume.flags, 0x1040);
        assert_eq!(volume.marker(), Some([7, 11]));
        // A volume of another kind is not a transition marker.
        body[8..12].copy_from_slice(&0x1000u32.to_le_bytes());
        let other = Item::parse(1, &section(VOLUME, &body)).unwrap();
        assert_eq!(other.volume.unwrap().marker(), None);
        // A short body keeps no volume.
        let short = Item::parse(1, &section(VOLUME, &[0u8; 18])).unwrap();
        assert!(short.volume.is_none());
        // An object without the section has none.
        assert!(
            Item::parse(1, &section(NAME, &padded("Box", 0x20)))
                .unwrap()
                .volume
                .is_none()
        );
    }

    #[test]
    fn armour_of_a_breastplate() {
        let mut body = vec![0u8; APPEARANCE_OFFSET];
        for (bone, pct) in [(8usize, 40u32), (9, 75), (11, 30), (14, 30)] {
            let at = COVERAGE_OFFSET + 4 * (bone - 1);
            body[at..at + 4].copy_from_slice(&pct.to_le_bytes());
        }
        body[PROTECTION_OFFSET..PROTECTION_OFFSET + 7]
            .copy_from_slice(&[60, 100, 60, 80, 0, 12, 4]);
        let item = Item::parse(1, &section(BODY, &body)).unwrap();
        let a = item.armour.unwrap();
        assert_eq!(a.coverage[8], 75);
        assert_eq!(a.coverage[0], 0);
        assert_eq!((a.padding, a.slash, a.crush, a.pierce), (60, 100, 60, 80));
        assert_eq!((a.encumbrance, a.material), (12, 4));
        // Nothing covered: no armour.
        let bare = Item::parse(1, &section(BODY, &vec![0u8; APPEARANCE_OFFSET])).unwrap();
        assert!(bare.armour.is_none());
    }

    /// A plate cuirass of an articulated set (its central piece item 445),
    /// and the same body cut short before the set word.
    #[test]
    fn burden_of_an_articulated_piece() {
        let mut body = vec![0u8; SET_OFFSET + 4];
        for bone in [8usize, 9, 11, 14] {
            let at = COVERAGE_OFFSET + 4 * (bone - 1);
            body[at..at + 4].copy_from_slice(&50u32.to_le_bytes());
        }
        body[ENCUMBRANCE_OFFSET] = 80;
        body[SET_OFFSET..SET_OFFSET + 4].copy_from_slice(&445u32.to_le_bytes());
        let item = Item::parse(446, &section(BODY, &body)).unwrap();
        let b = item.burden.unwrap();
        assert_eq!((b.encumbrance, b.set, b.special), (80, 445, 0));
        assert_eq!(b.bones, 1 << 7 | 1 << 8 | 1 << 10 | 1 << 13);
        assert!(!b.powered);
        // Bytes past a short body read as zero.
        let short = Item::parse(446, &section(BODY, &body[..SPECIAL_OFFSET])).unwrap();
        assert_eq!(short.burden.unwrap().set, 0);
        // No body, no burden.
        let name = Item::parse(1, &section(NAME, &padded("Book", 0x20))).unwrap();
        assert!(name.burden.is_none());
    }

    /// A golem suit as the database stores it (flag byte 0x80: unpowered),
    /// powered with a charge, and with a spent crystal.
    #[test]
    fn powered_specials() {
        let suit = |flags: u8, charge: u8| {
            let mut body = vec![0u8; SET_OFFSET + 4];
            body[SPECIAL_OFFSET] = Burden::GOLEM_SUIT;
            body[POWER_FLAGS_OFFSET] = flags;
            body[CHARGE_OFFSET] = charge;
            Item::parse(498, &section(BODY, &body))
                .unwrap()
                .burden
                .unwrap()
        };
        assert_eq!(suit(0x80, 0).special, Burden::GOLEM_SUIT);
        assert!(!suit(0x80, 0).powered);
        assert!(suit(0x81, 0x40).powered);
        assert!(!suit(0x81, 0xFF).powered);
    }

    /// The special kind byte at 0x214 of an item body; 0 past a short body.
    #[test]
    fn the_special_kind_byte() {
        let mut body = vec![0u8; SET_OFFSET + 4];
        body[SPECIAL_OFFSET] = Burden::SEES_IN_THE_DARK;
        assert_eq!(special_kind(&body), 1);
        assert_eq!(special_kind(&body[..SPECIAL_OFFSET]), 0);
    }

    /// The installed game's items of special kind 1 (they see in the dark)
    /// are the three night-vision helms (skipped without the game).
    #[test]
    fn the_installed_night_vision_helms() {
        let Some(dir) = exa_rpk::locate_install(None) else {
            return;
        };
        let game = exa_rpk::GameData::open(&dir).unwrap();
        let resource = game.pack(exa_rpk::PackKind::Resource).unwrap();
        let db = Database::parse(resource.get("objdb.rdb").unwrap()).unwrap();
        let mut seeing: Vec<String> = items(&db)
            .into_iter()
            .filter(|i| {
                i.burden
                    .is_some_and(|b| b.special == Burden::SEES_IN_THE_DARK)
            })
            .map(|i| i.name)
            .collect();
        seeing.sort();
        assert_eq!(seeing, ["UCA Helm A", "UGA Helm FB", "UW Helm NV"]);
    }

    /// An apparel body worn as `worn`, with per-bone occupancy entries
    /// (bone, u64) and appearance layers (mask, set, material, colour).
    fn apparel_body(worn: &str, occupancy: &[(usize, u64)], layers: &[[u8; 4]]) -> Vec<u8> {
        let mut body = padded(worn, APPEARANCE_OFFSET);
        for &(bone, bits) in occupancy {
            let at = OCCUPANCY_OFFSET + 8 * (bone - 1);
            body[at..at + 8].copy_from_slice(&bits.to_le_bytes());
        }
        body.extend([160, 64, 0, 0]);
        for &[mask, set, material, colour] in layers {
            body.extend([0, mask, set, 0, material, 0, 0, 0, colour, 0, 0, 0]);
        }
        body.resize(APPEARANCE_OFFSET + APPEARANCE_LEN, 0);
        body
    }

    #[test]
    fn worn_model_is_named_by_the_body() {
        let mut p = section(NAME, &padded("Trouser Qui A", 0x20));
        p.extend(section(MODEL, &padded("Trouser_At", 0x30)));
        p.extend(section(BODY, &apparel_body("Trouser_A", &[(1, 0x03)], &[])));
        let item = Item::parse(0x11d, &p).unwrap();
        assert_eq!(item.model, "Trouser_At");
        assert_eq!(item.worn_model, "Trouser_A");
    }

    #[test]
    fn item_without_ground_model() {
        let body = apparel_body(
            "Braies_A",
            &[(1, 0x03), (2, 0x01), (5, 0x01)],
            &[[1, 3, 1, 1], [2, 3, 1, 1]],
        );
        let mut p = section(NAME, &padded("Braies_A", 0x20));
        p.extend(section(BODY, &body));
        let item = Item::parse(8, &p).unwrap();
        assert!(item.model.is_empty());
        assert_eq!(item.worn_model, "Braies_A");
        assert!(item.covers_groin());
        assert_eq!(item.hidden_slots(), 0);
        assert_eq!(item.occupancy[1], 0x01);
        let a = item.appearance.unwrap();
        assert_eq!((a.quality, a.damage, a.dirt), (160, 64, 0));
        assert!(a.used_layers().all(|l| l.material == 1 && l.colour == 1));
    }

    #[test]
    fn groin_cover_is_the_innermost_layer() {
        let covers = |pelvis: u64| {
            let p = section(BODY, &apparel_body("X", &[(1, pelvis)], &[]));
            Item::parse(1, &p).unwrap().covers_groin()
        };
        // Mail legs occupy the pelvis on layer 1 only.
        assert!(!covers(0x0300));
        assert!(!covers(0x01));
        assert!(covers(0x02));
    }

    #[test]
    fn short_body_has_no_occupancy() {
        let mut body = padded("Braies_A", 0x20);
        body[OCCUPANCY_OFFSET] = 0x03;
        let item = Item::parse(8, &section(BODY, &body)).unwrap();
        assert_eq!(item.worn_model, "Braies_A");
        assert_eq!(item.occupancy, [0; OCCUPANCY_BONES]);
        assert!(!item.covers_groin());
        assert!(item.appearance.is_none());
    }

    #[test]
    fn condition_scales_the_lights_of_marked_models() {
        let mut model = padded("JarSalve A01", 16);
        model.extend(0x51u32.to_le_bytes());
        let salve = Item::parse(0x12, &section(MODEL, &model)).unwrap();
        assert_eq!(salve.model, "JarSalve A01");
        assert_eq!(salve.model_flags, 0x51);
        assert_eq!(salve.light_share(0), 1.0);
        // An almost spent salve (condition byte 0xEE).
        let spent = salve.light_share(0x00EE_0000);
        assert!((spent - (17.0f32 / 255.0).powi(2)).abs() < 1e-6);
        // Other bytes of the word leave it alone.
        assert_eq!(salve.light_share(0xFF00_FFFF), 1.0);
        // A model without the flag, and a section too short for the word.
        let mut lamp = padded("Lamp", 16);
        lamp.extend(0x50u32.to_le_bytes());
        let lamp = Item::parse(1, &section(MODEL, &lamp)).unwrap();
        assert_eq!(lamp.light_share(0x00EE_0000), 1.0);
        let short = Item::parse(1, &section(MODEL, &padded("Jar", 16))).unwrap();
        assert_eq!(short.model_flags, 0);
    }

    #[test]
    fn shield_section() {
        let mut body = [0u8; 80];
        body[4..7].copy_from_slice(&[15, 50, 8]);
        let mut p = section(NAME, &padded("Buckler A", 0x20));
        p.extend(section(SHIELD, &body));
        let item = Item::parse(80, &p).unwrap();
        // Scaled in double precision, as the original builds the record
        // (15 × 0.01f would be 0.14999999).
        let shield = item.shield.unwrap();
        assert_eq!(
            (shield.weight, shield.block, shield.bulk),
            (0.15, 0.5, 0.08)
        );
        assert!(item.burden.is_none());
        // A round shield's bulk 60 is 0.6 (0.59999996 in single precision).
        body[4..7].copy_from_slice(&[50, 58, 60]);
        let round = Item::parse(15, &section(SHIELD, &body)).unwrap();
        assert_eq!(round.shield.unwrap().bulk, 0.6);
        // Too short for the three bytes: no shield.
        let short = Item::parse(80, &section(SHIELD, &[0; 6])).unwrap();
        assert!(short.shield.is_none());
    }

    /// A prison portcullis: locked, motorised, fast, sound letter 5.
    #[test]
    fn door_section() {
        let mut body = 0x254u32.to_le_bytes().to_vec();
        body.extend(0x8000_0069u32.to_le_bytes());
        body.extend([5, 5, 5, 5]);
        body.extend([0; 8]);
        let mut p = section(NAME, &padded("PortCullis UWA", 0x20));
        p.extend(section(DOOR, &body));
        let item = Item::parse(0x64, &p).unwrap();
        let door = item.door.unwrap();
        assert!(door.has(door::LOCKED) && door.has(door::MOTORISED) && door.has(door::FAST));
        assert!(door.has(door::NODE_MARK) && !door.has(door::LATCHES));
        assert_eq!(door.key, 0x8000_0069);
        assert_eq!(door_sound(door.slam_sound, "slm"), "doorf_slm");
        assert_eq!(door_sound(0, "LkO"), "doora_LkO");
        assert_eq!(item.mechanism(), Some(Mechanism::Door));
        // Too short for the four sound letters: no door.
        let short = Item::parse(1, &section(DOOR, &[0; 11])).unwrap();
        assert!(short.door.is_none() && short.mechanism().is_none());
    }

    /// A lever returning after 5 s, a combination relay and an inverted
    /// button.
    #[test]
    fn operative_section() {
        let operative = |bytes: [u8; 8], target: u32, second: u32| {
            let mut body = bytes.to_vec();
            body.extend(target.to_le_bytes());
            body.extend(second.to_le_bytes());
            body.resize(60, 0);
            Item::parse(1, &section(OPERATIVE, &body)).unwrap()
        };
        let lever = operative([3, 1, 2, 0, 0, 0, 50, 0], 0x8000_00D0, 0);
        let o = lever.operative.unwrap();
        assert_eq!(
            (o.class, o.kind, o.flags, o.time),
            (operative::LEVER, 1, 2, 50)
        );
        assert_eq!(o.return_ms(), 5000);
        assert_eq!(o.target, 0x8000_00D0);
        assert_eq!(lever.mechanism(), Some(Mechanism::Lever));
        let relay = operative([7, 0, 8, 0, 0, 0, 0, 0], 0x8000_00AF, 3);
        let o = relay.operative.unwrap();
        assert_eq!((o.class, o.flags, o.second), (operative::RELAY, 8, 3));
        assert_eq!(relay.mechanism(), None);
        let button = operative([5, 2, 2, 0, 0, 0, 0, 0], 0x8000_00B4, 0);
        let o = button.operative.unwrap();
        assert_eq!(
            (o.class, o.kind),
            (operative::PUSH_SURFACE, operative::PUSH_INVERTED)
        );
        assert_eq!(button.mechanism(), Some(Mechanism::PushSurface));
        let fire = operative([6, 12, 0, 0, 176, 0, 0, 0], 0, 0);
        let o = fire.operative.unwrap();
        assert_eq!(
            (o.class, o.kind, o.value),
            (operative::JOB, operative::FIRE, 176)
        );
        // Too short for the second target: none.
        let short = Item::parse(
            1,
            &section(OPERATIVE, &[3, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
        )
        .unwrap();
        assert!(short.operative.is_none());
    }

    /// A teleporter's operative body names its symbol atlas and the level
    /// ids its screen offers: the four ids of the shipped teleporter, with
    /// the unused entries zero.
    #[test]
    fn teleporter_destinations() {
        let mut body = vec![operative::DEVICE, operative::TELEPORTER, 0x40, 0];
        body.extend([0; 12]);
        body.extend(padded("PrtlSmblA.rfi", 16));
        for id in [7u32, 10, 12, 13, 0, 0, 0] {
            body.extend(id.to_le_bytes());
        }
        assert_eq!(body.len(), 60);
        let item = Item::parse(0x555, &section(OPERATIVE, &body)).unwrap();
        let o = item.teleporter().unwrap();
        assert_eq!(o.class, operative::DEVICE);
        assert_eq!(o.flags, 0x40);
        assert_eq!(o.atlas_name(), "PrtlSmblA.rfi");
        assert_eq!(o.reachable().collect::<Vec<_>>(), [7, 10, 12, 13]);
        assert!(item.res_device().is_none());
        // The same body with the reconstruction device's kind: no screen.
        body[1] = operative::RES_DEVICE;
        let item = Item::parse(0x556, &section(OPERATIVE, &body)).unwrap();
        assert!(item.teleporter().is_none());
        assert_eq!(item.res_device().unwrap().kind, 0x12);

        // A short operative body has no atlas and no destinations.
        let short = Item::parse(1, &section(OPERATIVE, &body[..16])).unwrap();
        let o = short.operative.unwrap();
        assert_eq!(o.atlas_name(), "");
        assert_eq!(o.reachable().count(), 0);
    }

    /// The operative class decides before the door section: a lever with a
    /// door section is a lever, a key with one a plain body.
    #[test]
    fn operative_class_decides_the_mechanism() {
        let mut door = 0u32.to_le_bytes().to_vec();
        door.resize(20, 0);
        let with = |class: u8| {
            let mut op = vec![class, 1];
            op.resize(60, 0);
            let mut p = section(OPERATIVE, &op);
            p.extend(section(DOOR, &door));
            Item::parse(1, &p).unwrap().mechanism()
        };
        assert_eq!(with(operative::LEVER), Some(Mechanism::Lever));
        assert_eq!(with(operative::PUSH_SURFACE), Some(Mechanism::PushSurface));
        assert_eq!(with(operative::KEY), None);
    }

    /// The texture job reads all eight layers (no shipped item fills the
    /// eighth, so the shipped looks do not depend on it).
    #[test]
    fn every_layer_with_a_mask_is_read() {
        let mut layers = [Layer::default(); LAYERS];
        for (i, layer) in layers.iter_mut().enumerate() {
            layer.mask = i as u8 + 1;
        }
        let appearance = Appearance {
            quality: 0,
            damage: 0,
            dirt: 0,
            layers,
        };
        let masks: Vec<u8> = appearance.used_layers().map(|l| l.mask).collect();
        assert_eq!(masks, [1, 2, 3, 4, 5, 6, 7, 8]);
        let Some(dir) = exa_rpk::locate_install(None) else {
            return;
        };
        let game = exa_rpk::GameData::open(&dir).unwrap();
        let resource = game.pack(exa_rpk::PackKind::Resource).unwrap();
        let db = Database::parse(resource.get("objdb.rdb").unwrap()).unwrap();
        let eight = items(&db)
            .iter()
            .filter(|i| i.appearance.is_some_and(|a| a.layers[LAYERS - 1].mask != 0))
            .count();
        assert_eq!(eight, 0);
    }

    /// The door and operative records of the installed `objdb.rdb` and of
    /// the first level's own database (skipped without the game).
    #[test]
    fn shipped_mechanism_records() {
        use crate::rfc::PlacedObject;
        use std::collections::BTreeMap;
        let Some(dir) = exa_rpk::locate_install(None) else {
            return;
        };
        let game = exa_rpk::GameData::open(&dir).unwrap();
        let resource = game.pack(exa_rpk::PackKind::Resource).unwrap();
        let tally = |items: &[Item]| {
            let mut kinds: BTreeMap<(u8, u8), usize> = BTreeMap::new();
            let mut mechanisms: BTreeMap<String, usize> = BTreeMap::new();
            for item in items {
                assert!(item.door.is_none() || item.operative.is_none());
                if let Some(o) = item.operative {
                    *kinds.entry((o.class, o.kind)).or_default() += 1;
                }
                if let Some(m) = item.mechanism() {
                    *mechanisms.entry(format!("{m:?}")).or_default() += 1;
                }
            }
            let doors = items.iter().filter(|i| i.door.is_some()).count();
            (doors, kinds, mechanisms)
        };
        let db = Database::parse(resource.get("objdb.rdb").unwrap()).unwrap();
        let (doors, kinds, mechanisms) = tally(&items(&db));
        assert_eq!(doors, 47);
        assert_eq!(kinds.values().sum::<usize>(), 49);
        assert_eq!(kinds[&(operative::KEY, 0)], 15);
        assert_eq!(
            (
                mechanisms["Door"],
                mechanisms["Lever"],
                mechanisms["PushSurface"]
            ),
            (47, 6, 5)
        );
        let level = crate::rfc::Scene::parse(resource.get("exanima01.rfc").unwrap()).unwrap();
        let local = &level.local_items;
        let (doors, kinds, mechanisms) = tally(local);
        assert_eq!(doors, 43);
        assert_eq!(kinds.values().sum::<usize>(), 72);
        let count = |class: u8, kind: u8| kinds.get(&(class, kind)).copied().unwrap_or(0);
        assert_eq!((count(3, 1), count(3, 12)), (39, 1));
        assert_eq!((count(5, 1), count(5, 2)), (11, 1));
        assert_eq!(count(operative::JOB, operative::FIRE), 6);
        let relays: usize = (0..8).map(|k| count(operative::RELAY, k)).sum();
        assert_eq!(relays, 8);
        assert_eq!(count(1, 0) + count(1, 1), 6);
        assert_eq!(
            (
                mechanisms["Door"],
                mechanisms["Lever"],
                mechanisms["PushSurface"]
            ),
            (43, 40, 12)
        );
        // The objects doors link to are keys, levers and relays of the
        // level's own database; one door names an id the database lacks.
        let by_id: BTreeMap<u32, &Item> = local.iter().map(|i| (i.id, i)).collect();
        let (mut linked, mut dangling): (BTreeMap<u8, usize>, usize) = (BTreeMap::new(), 0);
        for door in local.iter().filter_map(|i| i.door).filter(|d| d.key != 0) {
            assert_eq!(door.key & 0xC000_0000, 0x8000_0000);
            match by_id.get(&(door.key & PlacedObject::ID_MASK)) {
                Some(other) => *linked.entry(other.operative.unwrap().class).or_default() += 1,
                None => dangling += 1,
            }
        }
        assert_eq!(
            linked.into_iter().collect::<Vec<_>>(),
            [(1, 6), (3, 4), (7, 7)]
        );
        assert_eq!(dangling, 1);
        // The first fire job lights a fire of size 200.
        let fire = by_id[&0x15].operative.unwrap();
        assert_eq!((fire.class, fire.kind, fire.value), (6, 12, 200));
    }

    #[test]
    fn truncated_section_is_an_error() {
        let mut p = NAME.to_le_bytes().to_vec();
        p.extend(100u32.to_le_bytes());
        assert!(Item::parse(1, &p).is_err());
    }
}
