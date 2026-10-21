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
    pub materials: [(u8, u8, u8); 4],
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
                (m[0], m[3], m[4])
            }),
            selection: std::array::from_fn(|i| b[0x50 + i]),
            stats: StatBytes::from_section(b)?,
        })
    }

    pub fn has_stored_choices(&self) -> bool {
        self.flags & WEAPON_STORED != 0
    }
}

impl Item {
    /// Body slots (bone indices) this item hides, as a bit mask.
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
            occupancy: [0; OCCUPANCY_BONES],
            weapon: None,
            armour: None,
            shield: None,
            door: None,
            operative: None,
            burden: None,
            equip_lock: 0,
        };
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
                    }
                    item.armour = Armour::parse(body);
                    item.burden = Some(Burden::parse(body));
                }
                WEAPON => item.weapon = Weapon::parse(body),
                SHIELD => item.shield = ShieldRecord::from_section(body),
                DOOR => item.door = Door::parse(body),
                OPERATIVE => item.operative = Operative::parse(body),
                _ => {}
            }
            pos += size;
        }
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

/// The object database's magic (`objdb.rdb`).
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
/// The section of an object's folder (its editor contents).
pub const FOLDER: u32 = 0x200;
/// The map record's own section; its name follows a zeroed volume.
pub const MAP: u32 = 0x8000_0B03;
/// The torch's extra section; zero in the one shipped record, meaning not
/// established.
pub const TORCH: u32 = 0x100;
/// The invisible container volume of older records; not shipped.
pub const VOLUME: u32 = 0x8000_AC07;
/// A container's contents: a count at `0x1C` and 12-byte child entries at
/// `0x20` (the id, then words whose meanings are not established).
pub const CONTAINER: u32 = 0x2_0000;

/// The tier an id selects: the level-local database (bit 31), the world
/// database (bit 30) or the base templates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Tier {
    Base,
    Local,
    World,
}

impl Tier {
    /// The tier an id belongs to. The local bit wins over the world bit.
    pub fn of(id: u32) -> Self {
        if id & LOCAL != 0 {
            Self::Local
        } else if id & WORLD != 0 {
            Self::World
        } else {
            Self::Base
        }
    }

    /// The record id inside a tagged id: the low 30 bits for the local and
    /// world tiers, the whole id for the base tier.
    pub fn index(id: u32) -> u32 {
        match Self::of(id) {
            Self::Base => id,
            _ => id & ID_MASK,
        }
    }

    /// The tag this tier adds to the ids it hands out.
    pub fn tag(self) -> u32 {
        match self {
            Self::Base => 0,
            Self::Local => LOCAL,
            Self::World => WORLD,
        }
    }
}

/// One section of a record payload: its tag and the bytes after the
/// `tag, size` header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Section<'a> {
    pub tag: u32,
    pub body: &'a [u8],
}

/// The sections of a record, in file order. A malformed size stops the
/// walk.
#[derive(Debug, Clone, Copy)]
pub struct Sections<'a> {
    rest: &'a [u8],
}

impl<'a> Iterator for Sections<'a> {
    type Item = Section<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let header = self.rest.get(..8)?;
        let tag = u32::from_le_bytes(header[0..4].try_into().expect("4 bytes"));
        let size = u32::from_le_bytes(header[4..8].try_into().expect("4 bytes")) as usize;
        if size < 8 {
            self.rest = &[];
            return None;
        }
        let body = self.rest.get(8..size)?;
        self.rest = self.rest.get(size..).unwrap_or(&[]);
        Some(Section { tag, body })
    }
}

/// The body range of the first section with `tag`.
fn section_range(payload: &[u8], tag: u32) -> Option<std::ops::Range<usize>> {
    let mut pos = 0;
    while let Some(header) = payload.get(pos..pos + 8) {
        let found = u32::from_le_bytes(header[0..4].try_into().expect("4 bytes"));
        let size = u32::from_le_bytes(header[4..8].try_into().expect("4 bytes")) as usize;
        if size < 8 {
            return None;
        }
        let end = pos.checked_add(size)?;
        let body = pos + 8..end;
        if body.end > payload.len() {
            return None;
        }
        if found == tag {
            return Some(body);
        }
        pos = end;
    }
    None
}

fn write_section_u32(payload: &mut [u8], tag: u32, offset: usize, value: u32) {
    if let Some(range) = section_range(payload, tag)
        && let Some(word) = payload.get_mut(range.start + offset..range.start + offset + 4)
    {
        word.copy_from_slice(&value.to_le_bytes());
    }
}

fn u32_at(b: &[u8], o: usize) -> u32 {
    b.get(o..o + 4)
        .map_or(0, |w| u32::from_le_bytes(w.try_into().expect("4 bytes")))
}

fn u16_at(b: &[u8], o: usize) -> u16 {
    b.get(o..o + 2)
        .map_or(0, |w| u16::from_le_bytes(w.try_into().expect("2 bytes")))
}

/// A record as the file holds it: its id, its table flags (the OR of its
/// section tags) and the chain of sections.
#[derive(Debug, Clone, Copy)]
pub struct Record<'a> {
    pub id: u32,
    /// The table's flags word: the OR of the tags of the sections the
    /// record holds. The original tests it before walking a section.
    pub flags: u32,
    payload: &'a [u8],
}

impl<'a> Record<'a> {
    pub fn new(id: u32, flags: u32, payload: &'a [u8]) -> Self {
        Self { id, flags, payload }
    }

    pub fn payload(&self) -> &'a [u8] {
        self.payload
    }

    pub fn sections(&self) -> Sections<'a> {
        Sections { rest: self.payload }
    }

    /// The body of the first section with `tag`.
    pub fn section(&self, tag: u32) -> Option<&'a [u8]> {
        section_range(self.payload, tag).map(|r| &self.payload[r])
    }

    /// Whether the table flags name `tag` (the original's fast test).
    pub fn has(&self, tag: u32) -> bool {
        self.flags & tag != 0
    }

    /// The header section: the display name, its word and the parent id.
    pub fn header(&self) -> Option<Header> {
        let b = self.section(NAME)?;
        if b.len() < 0x18 {
            return None;
        }
        Some(Header {
            name: name(b),
            flags: u32_at(b, 0x10),
            parent: u32_at(b, 0x14),
        })
    }

    /// The template an instance was made from (the header's word at
    /// `+0x14`), or 0 for a template itself.
    pub fn template(&self) -> u32 {
        self.section(NAME).map_or(0, |b| u32_at(b, 0x14))
    }

    /// The model section: the ground or icon model, its flags word, the
    /// ids of the display name and the description, and the shop fields.
    pub fn model(&self) -> Option<Model> {
        let b = self.section(MODEL)?;
        if b.len() < 0x1C {
            return None;
        }
        Some(Model {
            name: name(b),
            flags: u32_at(b, 0x10),
            name_id: u32_at(b, 0x14),
            description_id: u32_at(b, 0x18),
            rarity: b.get(0x1D).copied().unwrap_or(0),
            size: [u32_at(b, 0x20), u32_at(b, 0x24)],
        })
    }

    /// The folder section: the items an editor folder lists.
    pub fn folder(&self) -> Option<Folder> {
        let b = self.section(FOLDER)?;
        let count = u32_at(b, 0x0C) as usize;
        let entries = (0..count)
            .take_while(|i| b.get(0x10 + i * 16..0x10 + i * 16 + 16).is_some())
            .map(|i| {
                let e = &b[0x10 + i * 16..0x10 + i * 16 + 16];
                FolderEntry {
                    id: u32_at(e, 0),
                    words: [u16_at(e, 4), u16_at(e, 6), u16_at(e, 8)],
                    fractions: FractionWord(u16_at(e, 10)),
                    extra: u32_at(e, 12),
                }
            })
            .collect();
        Some(Folder { entries })
    }
}

/// The header section of a record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Header {
    /// The display name, 16 bytes at most, Latin-1.
    pub name: String,
    /// The header's word: bit 0 ([`INSTANCE_ON_USE`]) makes a template
    /// instantiate into the local tier when used; bits 16–23 are the
    /// material class the template migration compares.
    pub flags: u32,
    /// The parent: the editor folder for a global item, the source item
    /// for a level-local copy, and the template for an instance.
    pub parent: u32,
}

impl Header {
    /// Whether using the record instantiates it into the local tier.
    pub fn instance_on_use(&self) -> bool {
        self.flags & INSTANCE_ON_USE != 0
    }

    /// The material class the template migration keeps a body's appearance
    /// in step with.
    pub fn material_class(&self) -> u32 {
        (self.flags >> 16) & 0xFF
    }
}

/// The model section of a record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Model {
    /// The ground or icon model, 16 bytes at most, Latin-1.
    pub name: String,
    /// Bit 0 ([`CONDITION_SCALED`]) dims the model's lights by a placed
    /// copy's condition byte; bits 4–6 are the shop's rarity tier.
    pub flags: u32,
    /// The display name's string id ([`INHERIT`] = the template's).
    pub name_id: u32,
    /// The description's string id.
    pub description_id: u32,
    /// Hundredths of rarity, the shop's weight.
    pub rarity: u8,
    /// Two words a stocked item needs non-zero (`[guess]` the icon's size).
    pub size: [u32; 2],
}

/// A folder's the item list (`0x200`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Folder {
    pub entries: Vec<FolderEntry>,
}

/// One item an editor folder lists.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FolderEntry {
    /// The item record's id.
    pub id: u32,
    /// The three words after the id; their meanings are not established.
    pub words: [u16; 3],
    /// Five fractions packed in one word, used by the item randomisers.
    pub fractions: FractionWord,
    /// A trailing word, zero in every shipped entry.
    pub extra: u32,
}

/// The five fractions of a folder entry's packed word. The getters and
/// setters are the original's; the field meanings are not established, so
/// only the bit positions are named. A zero in the last three fields reads
/// as 0.125, a value the game uses as "leave the choice to the material
/// set".
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct FractionWord(pub u16);

impl FractionWord {
    /// Bits 0–3, ×0.25.
    pub fn level(self) -> f32 {
        (self.0 & 0xF) as f32 * 0.25
    }

    pub fn set_level(&mut self, v: f32) {
        self.0 = self.0 & !0xF | quantise(v, 4, 0.25, 0xF);
    }

    /// Bits 6–8, ×0.25.
    pub fn field_6(self) -> f32 {
        (self.0 & 0x1C0) as f32 / 64.0 * 0.25
    }

    pub fn set_field_6(&mut self, v: f32) {
        let n = (v * 4.0).round_ties_even() as u16;
        self.0 = self.0 & !0x1C0 | n.wrapping_shl(6);
    }

    /// Bits 9–10, ×0.25, a zero reading as 0.125.
    pub fn field_9(self) -> f32 {
        let n = (self.0 & 0x600) >> 9;
        if n == 0 { 0.125 } else { n as f32 * 0.25 }
    }

    pub fn set_field_9(&mut self, v: f32) {
        let bits = if v < 0.1875 {
            0
        } else if v < 0.625 {
            ((v * 4.0).round_ties_even() as u16).wrapping_shl(9) & 0x600
        } else {
            0x600
        };
        self.0 = self.0 & !0x600 | bits;
    }

    /// Bits 11–13, ×0.25.
    pub fn field_11(self) -> f32 {
        ((self.0 & 0x3800) >> 11) as f32 * 0.25
    }

    pub fn set_field_11(&mut self, v: f32) {
        let n = (v * 4.0).round_ties_even() as u16;
        self.0 = self.0 & !0x3800 | n.wrapping_shl(11);
    }

    /// Bits 14–15, ×0.25, a zero reading as 0.125.
    pub fn field_14(self) -> f32 {
        let n = self.0 >> 14;
        if n == 0 { 0.125 } else { n as f32 * 0.25 }
    }

    pub fn set_field_14(&mut self, v: f32) {
        let bits = if v < 0.2 {
            0
        } else {
            ((v * 4.0).round_ties_even() as u16).wrapping_shl(14) & 0xC000
        };
        self.0 = self.0 & !0xC000 | bits;
    }
}

/// Rounds `v` to a multiple of `step` and masks it to `bits` bits.
fn quantise(v: f32, _bits: u8, step: f32, mask: u16) -> u16 {
    ((v / step).round_ties_even() as i32 as u16) & mask
}

/// One record of an owned tier.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnedRecord {
    pub id: u32,
    pub flags: u32,
    pub payload: Vec<u8>,
}

impl OwnedRecord {
    pub fn view(&self) -> Record<'_> {
        Record::new(self.id, self.flags, &self.payload)
    }
}

/// The records of one tier, sorted by id as the file's table is.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Store {
    records: Vec<OwnedRecord>,
}

impl Store {
    /// Reads a parsed database into an owned tier.
    pub fn from_database(db: &Database<'_>) -> Self {
        let mut records: Vec<_> = db
            .records
            .iter()
            .filter(|r| r.id != 0)
            .map(|r| OwnedRecord {
                id: r.id,
                flags: r.flags,
                payload: db.payload(r).to_vec(),
            })
            .collect();
        records.sort_by_key(|r| r.id);
        Self { records }
    }

    /// A tier from owned records, sorted by id.
    pub fn from_records(mut records: Vec<OwnedRecord>) -> Self {
        records.sort_by_key(|r| r.id);
        Self { records }
    }

    pub fn records(&self) -> &[OwnedRecord] {
        &self.records
    }

    pub fn len(&self) -> usize {
        self.records.len()
    }

    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    /// The first record with `id`, by the original's binary search. Ids
    /// are unique in a well-formed table; an ill-formed one returns the
    /// probe that matched.
    pub fn find(&self, id: u32) -> Option<&OwnedRecord> {
        if id == 0 {
            return None;
        }
        let mut lo = 0usize;
        let mut hi = self.records.len();
        while lo < hi {
            let mid = lo + (hi - lo) / 2;
            let found = self.records[mid].id;
            if id < found {
                hi = mid;
            } else if id > found {
                lo = mid + 1;
            } else {
                return Some(&self.records[mid]);
            }
        }
        None
    }

    /// The smallest positive id the tier lacks: where an instance goes.
    pub fn first_free_id(&self) -> u32 {
        let mut id = 1;
        for record in &self.records {
            if record.id < id {
                continue;
            }
            if record.id > id {
                break;
            }
            id += 1;
        }
        id
    }

    /// Inserts a record at its sorted place; a duplicate id is refused.
    pub fn insert(&mut self, record: OwnedRecord) -> bool {
        let at = self.records.partition_point(|r| r.id < record.id);
        if self.records.get(at).is_some_and(|r| r.id == record.id) {
            return false;
        }
        self.records.insert(at, record);
        true
    }

    pub fn remove(&mut self, id: u32) -> Option<OwnedRecord> {
        let at = self.records.binary_search_by_key(&id, |r| r.id).ok()?;
        Some(self.records.remove(at))
    }

    fn get_mut(&mut self, id: u32) -> Option<&mut OwnedRecord> {
        let at = self.records.binary_search_by_key(&id, |r| r.id).ok()?;
        self.records.get_mut(at)
    }
}

/// The three tiers of the object database and their string libraries.
#[derive(Debug, Clone, Default)]
pub struct ObjectDb {
    /// The read-only templates of `objdb.rdb`.
    pub base: Store,
    /// A level's own objects (ids tagged [`LOCAL`]).
    pub local: Store,
    /// Objects promoted out of a level (ids tagged [`WORLD`]).
    pub world: Store,
    /// The base tier's string library (`objstrings.rdb`).
    pub strings: crate::objstrings::Library,
    pub local_strings: crate::objstrings::Library,
    pub world_strings: crate::objstrings::Library,
    /// Local ids a promotion moved, and their world ids, when recording is
    /// on.
    remap: Vec<(u32, u32)>,
    /// The same for usable objects of kind 0, which door links follow.
    link_remap: Vec<(u32, u32)>,
    /// Whether a promotion records the id pairs (the original's switch).
    remap_enabled: bool,
    /// Whether it records the kind-0 pairs for link rewriting.
    link_remap_enabled: bool,
}

impl ObjectDb {
    /// Parses the database and its string library. Older database magics
    /// need the original's upgrade chain, which is not reproduced, and are
    /// refused.
    pub fn parse(db: &[u8], strings: &[u8]) -> Result<Self> {
        let database = Database::parse(db)?;
        match database.magic {
            MAGIC => {}
            0xDBAC_0D07 | 0xDBAC_0D08 => {
                return Err(Error::Unsupported(format!(
                    "object database of the older layout {:#010x}",
                    database.magic
                )));
            }
            found => {
                return Err(Error::BadMagic {
                    found,
                    expected: MAGIC,
                });
            }
        }
        let strings = if strings.is_empty() {
            crate::objstrings::Library::default()
        } else {
            crate::objstrings::Library::parse(strings)?
        };
        Ok(Self {
            base: Store::from_database(&database),
            strings,
            ..Self::default()
        })
    }

    pub fn store(&self, tier: Tier) -> &Store {
        match tier {
            Tier::Base => &self.base,
            Tier::Local => &self.local,
            Tier::World => &self.world,
        }
    }

    fn store_mut(&mut self, tier: Tier) -> &mut Store {
        match tier {
            Tier::Base => &mut self.base,
            Tier::Local => &mut self.local,
            Tier::World => &mut self.world,
        }
    }

    /// The string library a tier's own strings live in.
    pub fn library(&self, tier: Tier) -> &crate::objstrings::Library {
        match tier {
            Tier::Base => &self.strings,
            Tier::Local => &self.local_strings,
            Tier::World => &self.world_strings,
        }
    }

    /// The record an id names, in the tier its tag bits select.
    pub fn find(&self, id: u32) -> Option<&OwnedRecord> {
        self.store(Tier::of(id)).find(Tier::index(id))
    }

    pub fn view(&self, id: u32) -> Option<Record<'_>> {
        self.find(id).map(OwnedRecord::view)
    }

    /// The body of the first section with `tag`, as the original's section
    /// resolver walks it (no flag test).
    pub fn section(&self, id: u32, tag: u32) -> Option<&[u8]> {
        self.view(id)?.section(tag)
    }

    /// Whether the record holds a section with `tag`, as the original's
    /// fast check reads it: the table flags first, then the walk.
    pub fn has_section(&self, id: u32, tag: u32) -> bool {
        let Some(record) = self.find(id) else {
            return false;
        };
        record.flags & tag != 0 && section_range(&record.payload, tag).is_some()
    }

    /// The string id of the record's display name, or description.
    fn string_id(&self, id: u32, description: bool) -> Option<u32> {
        let model = self.find(id)?.view().model()?;
        Some(if description {
            model.description_id
        } else {
            model.name_id
        })
    }

    /// The display name of an object, following the string-id rules:
    /// 0 none, [`INHERIT`] the template's field (read from the shared
    /// library), a top bit the record's own tier library, anything else
    /// the shared library.
    pub fn name(&self, id: u32) -> Option<String> {
        self.string(id, false)
    }

    /// The description of an object, by the same rules as [`Self::name`].
    pub fn description(&self, id: u32) -> Option<String> {
        self.string(id, true)
    }

    fn string(&self, id: u32, description: bool) -> Option<String> {
        let record = self.find(id)?;
        let raw = self.string_id(id, description)?;
        if raw == 0 {
            return None;
        }
        if raw == INHERIT {
            let template = record.view().template();
            if template == 0 {
                return None;
            }
            let raw = self.string_id(template, description)?;
            return self.strings.get(raw & ID_MASK);
        }
        if raw & 0x8000_0000 != 0 {
            return self
                .library(Tier::of(id))
                .get(raw & ID_MASK)
                .or_else(|| self.strings.get(raw & ID_MASK));
        }
        self.strings.get(raw)
    }

    /// The smallest positive id the tier lacks.
    pub fn first_free_id(&self, tier: Tier) -> u32 {
        self.store(tier).first_free_id()
    }

    /// Clones a base template into `tier`, as the original's instancing
    /// does: the table flags come along, the header names the template and
    /// the model's name and description inherit.
    pub fn clone_template_into(&mut self, id: u32, tier: Tier) -> Option<u32> {
        if tier == Tier::Base {
            return None;
        }
        let src = self.base.find(id)?;
        let mut payload = src.payload.clone();
        write_section_u32(&mut payload, NAME, 0x14, id);
        write_section_u32(&mut payload, MODEL, 0x14, INHERIT);
        write_section_u32(&mut payload, MODEL, 0x18, INHERIT);
        let new_id = self.store(tier).first_free_id();
        let record = OwnedRecord {
            id: new_id,
            flags: src.flags,
            payload,
        };
        self.store_mut(tier)
            .insert(record)
            .then_some(new_id | tier.tag())
    }

    /// The template an instance should name: the record's header template
    /// when that template names a non-empty model, else the record itself.
    fn instance_template(&self, id: u32) -> u32 {
        let Some(record) = self.find(id) else {
            return id;
        };
        let template = record.view().template();
        if template == 0 {
            return id;
        }
        let Some(model) = self.find(template).and_then(|r| r.view().model()) else {
            return id;
        };
        if model.name.is_empty() { id } else { template }
    }

    /// Instantiates a base template into the local tier when its header
    /// flags mark it for use, copying the template's coverage and
    /// protection and carrying the description's own string across.
    pub fn instantiate(&mut self, id: u32) -> Option<u32> {
        if Tier::of(id) != Tier::Base {
            return None;
        }
        let src = self.base.find(id)?;
        if !src.view().header().is_some_and(|h| h.instance_on_use()) {
            return None;
        }
        let template = self.instance_template(id);
        let mut payload = src.payload.clone();
        write_section_u32(&mut payload, NAME, 0x14, template);
        if let Some(model) = src.view().model() {
            if template == id {
                write_section_u32(&mut payload, MODEL, 0x14, INHERIT);
                write_section_u32(&mut payload, MODEL, 0x18, INHERIT);
            } else {
                let tpl_payload = self.base.find(template).map(|r| r.payload.clone());
                if let Some(tpl_payload) = tpl_payload {
                    copy_body_blocks(&tpl_payload, &mut payload);
                    write_section_u32(&mut payload, MODEL, 0x14, INHERIT);
                    let tpl_model = Record::new(template, 0, &tpl_payload)
                        .model()
                        .unwrap_or_else(|| model.clone());
                    let description = if tpl_model.description_id == model.description_id {
                        INHERIT
                    } else {
                        self.strings
                            .bytes(tpl_model.description_id & ID_MASK)
                            .map(|text| {
                                let text = text.to_vec();
                                let string = self.local_strings.add(&text);
                                string | 0x8000_0000
                            })
                            .unwrap_or(INHERIT)
                    };
                    write_section_u32(&mut payload, MODEL, 0x18, description);
                }
            }
        }
        let new_id = self.local.first_free_id();
        let record = OwnedRecord {
            id: new_id,
            flags: src.flags,
            payload,
        };
        self.local.insert(record).then_some(new_id | LOCAL)
    }

    /// Clones a local instance, or the template it was made from when the
    /// instance is itself a plain template copy, as duplicating an item in
    /// the world does.
    pub fn duplicate_local(&mut self, id: u32) -> Option<u32> {
        if Tier::of(id) != Tier::Local {
            return None;
        }
        let src = self.local.find(Tier::index(id))?.clone();
        let view = src.view();
        if !view.header().is_some_and(|h| h.instance_on_use()) {
            let template = view.template();
            return if template == 0 {
                None
            } else {
                self.clone_template_into(template, Tier::Local)
            };
        }
        let mut payload = src.payload.clone();
        if let Some(model) = view.model() {
            for raw in [model.name_id, model.description_id] {
                if raw > INHERIT {
                    self.local_strings.release(raw & ID_MASK);
                }
            }
            write_section_u32(&mut payload, MODEL, 0x14, INHERIT);
            write_section_u32(&mut payload, MODEL, 0x18, INHERIT);
        }
        let new_id = self.local.first_free_id();
        self.local
            .insert(OwnedRecord {
                id: new_id,
                flags: src.flags,
                payload,
            })
            .then_some(new_id | LOCAL)
    }

    /// Promotes a local record, and every child in its container, into the
    /// world tier with new ids. Returns the new id. A world record only
    /// refreshes its children and comes back unchanged.
    pub fn promote(&mut self, id: u32) -> Option<u32> {
        match Tier::of(id) {
            Tier::World => {
                let mut payload = self.world.find(Tier::index(id))?.payload.clone();
                self.promote_children(&mut payload)?;
                self.world.get_mut(Tier::index(id))?.payload = payload;
                Some(id)
            }
            Tier::Base => Some(id),
            Tier::Local => {
                if self.remap_enabled
                    && let Some(&(_, world)) = self.remap.iter().find(|(local, _)| *local == id)
                {
                    return Some(world);
                }
                let src = self.local.find(Tier::index(id))?.clone();
                let mut payload = src.payload.clone();
                self.promote_children(&mut payload)?;
                let view = Record::new(src.id, src.flags, &src.payload);
                if let Some(model) = view.model() {
                    for (field, raw) in [(0x14, model.name_id), (0x18, model.description_id)] {
                        let mut new = raw;
                        if raw > INHERIT {
                            let text = self.local_strings.bytes(raw & ID_MASK).map(<[u8]>::to_vec);
                            self.local_strings.release(raw & ID_MASK);
                            new = match text {
                                Some(text) => self.world_strings.add(&text) | 0x8000_0000,
                                None => INHERIT,
                            };
                        }
                        write_section_u32(&mut payload, MODEL, field, new);
                    }
                }
                let kind_zero = view
                    .section(OPERATIVE)
                    .and_then(|b| b.get(1))
                    .is_some_and(|&kind| kind == 0);
                let new_id = self.world.first_free_id();
                self.world.insert(OwnedRecord {
                    id: new_id,
                    flags: src.flags,
                    payload,
                });
                self.local.remove(Tier::index(id));
                let world_id = new_id | WORLD;
                if self.remap_enabled {
                    self.remap.push((id, world_id));
                }
                if self.link_remap_enabled && kind_zero {
                    self.link_remap.push((id, world_id));
                }
                Some(world_id)
            }
        }
    }

    /// Promotes every child id in a record's container section, writing the
    /// new ids back.
    fn promote_children(&mut self, payload: &mut [u8]) -> Option<()> {
        let Some(range) = section_range(payload, CONTAINER) else {
            return Some(());
        };
        let count = u32_at(&payload[range.clone()], 0x1C) as usize;
        for i in 0..count {
            let at = 0x20 + i * 12;
            let Some(entry) = payload.get(range.start + at..range.start + at + 12) else {
                break;
            };
            let child = u32_at(entry, 0);
            let promoted = self.promote(child)?;
            payload[range.start + at..range.start + at + 4]
                .copy_from_slice(&promoted.to_le_bytes());
        }
        Some(())
    }

    /// The world id a promoted local id became, while recording is on.
    pub fn remapped(&self, id: u32) -> Option<u32> {
        self.remap
            .iter()
            .find(|(local, _)| *local == id)
            .map(|&(_, world)| world)
    }

    /// Whether promotions record the id pairs.
    pub fn set_remaps(&mut self, remaps: bool) {
        self.remap_enabled = remaps;
        if !remaps {
            self.remap.clear();
        }
    }

    /// Whether promotions also record kind-0 usable objects for door links.
    pub fn set_link_remaps(&mut self, remaps: bool) {
        self.link_remap_enabled = remaps;
        if !remaps {
            self.link_remap.clear();
        }
    }

    /// Rewrites the linked-object word of every local door through the
    /// kind-0 remaps, as loading an arena save does. Returns how many
    /// doors changed.
    pub fn apply_link_remaps(&mut self) -> usize {
        let ids: Vec<u32> = self
            .local
            .records
            .iter()
            .filter(|r| r.flags & DOOR != 0)
            .map(|r| r.id)
            .collect();
        let mut changed = 0;
        for id in ids {
            let Some(record) = self.local.get_mut(id) else {
                continue;
            };
            let Some(range) = section_range(&record.payload, DOOR) else {
                continue;
            };
            let link = u32_at(&record.payload[range.clone()], 0x0C);
            let Some(&(_, world)) = self.link_remap.iter().find(|(local, _)| *local == link) else {
                continue;
            };
            record.payload[range.start + 0x0C..range.start + 0x10]
                .copy_from_slice(&world.to_le_bytes());
            changed += 1;
        }
        changed
    }

    /// Refreshes an instance's sections from its template, as the original
    /// does after loading a database: the body's appearance, coverage and
    /// protection, the weapon's stat bytes unless its choices are fixed,
    /// the container's capacity words, the door's `0x200` flag and the
    /// model's name. Returns whether the record changed.
    pub fn migrate(&mut self, id: u32) -> bool {
        let tier = Tier::of(id);
        if tier == Tier::Base {
            return false;
        }
        let Some(src) = self.store(tier).find(Tier::index(id)) else {
            return false;
        };
        let template_id = src.view().template();
        if template_id == 0 {
            return false;
        }
        let Some(template) = self.find(template_id).cloned() else {
            return false;
        };
        let src_flags = src.flags;
        let mut payload = src.payload.clone();
        let material_changed = {
            let src_class = Record::new(id, src_flags, &payload)
                .header()
                .map(|h| h.material_class())
                .unwrap_or(0);
            let tpl_class = template
                .view()
                .header()
                .map(|h| h.material_class())
                .unwrap_or(0);
            src_class != tpl_class
        };
        let tpl = template.view();
        let tags: Vec<u32> = Record::new(id, src_flags, &payload)
            .sections()
            .map(|s| s.tag)
            .collect();
        for tag in tags {
            if tag <= 0xFFF {
                continue;
            }
            let Some(tpl_body) = tpl.section(tag) else {
                continue;
            };
            let Some(range) = section_range(&payload, tag) else {
                continue;
            };
            let body = &mut payload[range];
            match tag {
                BODY => {
                    if material_changed {
                        migrate_appearance(body, tpl_body);
                    }
                    let tpl_word = u32_at(tpl_body, 0x1B4);
                    if tpl_word == 0xFFFF_FFFF {
                        set_u32(body, 0x1AC, 0xFFFF_FFFF);
                        copy_bytes(tpl_body, body, 0x1B1, 1);
                        for i in 0..8 {
                            let material = body.get(0x1B8 + i * 12).copied().unwrap_or(0);
                            let mapped = if (1..=26).contains(&material) {
                                0x88
                            } else {
                                0
                            };
                            if let Some(byte) = body.get_mut(0x1BB + i * 12) {
                                *byte = mapped;
                            }
                        }
                        for i in 0..3 {
                            let at = 0x218 + i * 2;
                            let keep = tpl_body.get(0x220 + i * 2).is_some_and(|b| b & 0x80 != 0);
                            if let Some(byte) = body.get_mut(at) {
                                if keep {
                                    *byte |= 0x80;
                                } else {
                                    *byte = 0;
                                }
                            }
                        }
                    } else if u32_at(body, 0x1AC) == 0xFFFF_FFFF {
                        let n = body.len().min(tpl_body.len());
                        body[..n].copy_from_slice(&tpl_body[..n]);
                    }
                    for (at, len) in [
                        (0x00, 0x10),
                        (0x10, 0xA0),
                        (0xB0, 0xA0),
                        (0x150, 0x50),
                        (0x1A6, 0x03),
                    ] {
                        copy_bytes(tpl_body, body, at, len);
                    }
                    copy_bytes(tpl_body, body, 0x220, 4);
                }
                WEAPON => {
                    if u32_at(tpl_body, 0x0C) == 0xFFFF_FFFF {
                        for at in [0x0C, 0x10] {
                            set_u32(body, at, 0xFFFF_FFFF);
                        }
                        copy_bytes(tpl_body, body, 0x14, 4);
                    }
                    let flags = u32_at(tpl_body, 0);
                    set_u32(
                        body,
                        0,
                        (u32_at(body, 0) & 0xF000_0000) | (flags & 0x0FFF_FFFF),
                    );
                    if !weapon_is_default(body) {
                        for (at, len) in [
                            (0x30, 0x03),
                            (0x33, 0x01),
                            (0x34, 0x08),
                            (0x3C, 0x04),
                            (0x40, 0x01),
                            (0x42, 0x01),
                        ] {
                            copy_bytes(tpl_body, body, at, len);
                        }
                    }
                }
                CONTAINER => {
                    copy_bytes(tpl_body, body, 0x00, 4);
                    copy_bytes(tpl_body, body, 0x08, 2);
                    copy_bytes(tpl_body, body, 0x0A, 2);
                }
                DOOR => {
                    let flags = u32_at(tpl_body, 0) & 0x200;
                    set_u32(body, 0, u32_at(body, 0) | flags);
                }
                MODEL => {
                    if material_changed {
                        copy_bytes(tpl_body, body, 0x00, 0x28);
                        set_u32(body, 0x14, INHERIT);
                        set_u32(body, 0x18, INHERIT);
                    } else {
                        copy_bytes(tpl_body, body, 0x00, 0x10);
                        copy_bytes(tpl_body, body, 0x10, 4);
                        for at in [0x1C, 0x1D] {
                            copy_bytes(tpl_body, body, at, 1);
                        }
                        copy_bytes(tpl_body, body, 0x1E, 2);
                    }
                }
                _ => {}
            }
        }
        if let Some(record) = self.store_mut(tier).get_mut(Tier::index(id)) {
            record.payload = payload;
            return true;
        }
        false
    }

    /// Migrates every local and world record; returns how many changed.
    pub fn migrate_all(&mut self) -> usize {
        let ids: Vec<u32> = self
            .local
            .records()
            .iter()
            .map(|r| r.id | LOCAL)
            .chain(self.world.records().iter().map(|r| r.id | WORLD))
            .collect();
        ids.into_iter().filter(|&id| self.migrate(id)).count()
    }
}

impl Record<'_> {
    /// Decodes the record's item fields, as [`Item::parse`] does.
    pub fn item(&self) -> Result<Item> {
        Item::parse(self.id, self.payload)
    }
}

impl ObjectDb {
    /// The base tier decoded into items, in table order. What the game's
    /// callers read of `objdb.rdb`.
    pub fn items(&self) -> Vec<Item> {
        self.base
            .records()
            .iter()
            .filter_map(|r| r.view().item().ok())
            .collect()
    }

    /// The first record whose display name matches `name`, ignoring case.
    /// An Exarust convenience: the original looks objects up by id only and
    /// uses this name as an editor label, so the first in table order wins.
    pub fn find_by_name(&self, name: &str) -> Option<&OwnedRecord> {
        self.base.records().iter().find(|r| {
            let view = r.view();
            view.header()
                .is_some_and(|h| h.name.eq_ignore_ascii_case(name))
                || self
                    .name(r.id)
                    .is_some_and(|n| n.eq_ignore_ascii_case(name))
        })
    }

    /// The first record whose model name matches `model`, ignoring case.
    pub fn find_by_model(&self, model: &str) -> Option<&OwnedRecord> {
        self.base.records().iter().find(|r| {
            r.view()
                .model()
                .is_some_and(|m| m.name.eq_ignore_ascii_case(model))
        })
    }

    /// Writes a tier as the original saves a database: magic, the table
    /// (with its trailing empty slot) and the payloads. An empty tier
    /// writes the header alone, as the original's writer does.
    pub fn write(&self, tier: Tier) -> Vec<u8> {
        let store = self.store(tier);
        let count = store.len();
        let mut out = MAGIC.to_le_bytes().to_vec();
        let table_len = ((count + 1) * 16) as u32;
        out.extend(table_len.to_le_bytes());
        if count == 0 {
            return out;
        }
        let mut offset = 0u32;
        for record in store.records() {
            out.extend(record.id.to_le_bytes());
            out.extend(record.flags.to_le_bytes());
            out.extend(offset.to_le_bytes());
            out.extend((record.payload.len() as u32).to_le_bytes());
            offset += record.payload.len() as u32;
        }
        out.extend([0u8; 16]);
        for record in store.records() {
            out.extend_from_slice(&record.payload);
        }
        out
    }
}

/// A value the original stores quantised in a small bit field. The
/// getters and setters are the routines item descriptions, the trade
/// system and the arena share; the fields belong to item variant records
/// whose meanings are not established, so they are named by position.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Quantised(pub u16);

impl Quantised {
    /// Bits 0–1, 0 to 3 (the original rounds and clamps its setter).
    pub fn count(self) -> u8 {
        (self.0 & 3) as u8
    }

    pub fn set_count(&mut self, value: f32) {
        let v = value.clamp(0.0, 3.0).round_ties_even() as u16;
        self.0 = self.0 & !3 | v & 3;
    }

    /// Bits 2–6 with bit 7 as the sign, a fraction of 31, −1 to 1.
    pub fn signed(self) -> f32 {
        let magnitude = ((self.0 & 0x7C) >> 2) as f32 / 31.0;
        if self.0 & 0x80 != 0 {
            -magnitude
        } else {
            magnitude
        }
    }

    pub fn set_signed(&mut self, value: f32) {
        let magnitude = (value.abs() * 31.0) as u16 & 0x1F;
        self.0 = self.0 & !0xFC | magnitude << 2;
        if value < 0.0 {
            self.0 |= 0x80;
        } else {
            self.0 &= !0x80;
        }
    }

    /// Bits 5–7: the top three bits of the low byte.
    pub fn high3(self) -> u8 {
        (self.0 >> 5) as u8 & 7
    }

    /// Bits 0–4.
    pub fn low5(self) -> u8 {
        (self.0 & 0x1F) as u8
    }

    /// A three-level fraction at `index` 0 (bits 0–2), 1 (bits 3–5) or 2
    /// (bits 6–7, only two bits): 0, 0.125, 0.25, 0.5, 0.75 or 1.
    pub fn fraction(self, index: usize) -> f32 {
        let (shift, mask) = FRACTION_FIELDS[index];
        fraction_of((self.0 & mask) >> shift)
    }

    /// Stores a fraction; the original rounds by eighths, folds values
    /// above 0.25 down one step and caps at the field's widest value.
    pub fn set_fraction(&mut self, index: usize, value: f32) {
        let (shift, mask) = FRACTION_FIELDS[index];
        let max = (mask >> shift).min(5);
        let mut n = (value * 8.0).round_ties_even() as u16;
        if n > 1 {
            n = (n >> 1) + 1;
        }
        n = n.min(max);
        self.0 = self.0 & !mask | n << shift;
    }
}

/// The shift and mask of each three-level fraction field.
const FRACTION_FIELDS: [(u16, u16); 3] = [(0, 0x7), (3, 0x38), (6, 0xC0)];

/// The value of a three-level fraction's raw bits: 0 and 1 read as 0 and
/// 0.125, the rest as (n − 1)/4.
fn fraction_of(bits: u16) -> f32 {
    if bits < 2 {
        bits as f32 * 0.125
    } else {
        (bits - 1) as f32 * 0.25
    }
}

/// What the original calls a weapon's default state: the two words after
/// the flags are non-zero and the byte at `+0xF` is zero.
fn weapon_is_default(body: &[u8]) -> bool {
    u32_at(body, 4) != 0 && u32_at(body, 8) != 0 && body.get(0x0F).copied().unwrap_or(1) == 0
}

fn set_u32(b: &mut [u8], at: usize, value: u32) {
    if let Some(word) = b.get_mut(at..at + 4) {
        word.copy_from_slice(&value.to_le_bytes());
    }
}

fn copy_bytes(src: &[u8], dst: &mut [u8], at: usize, len: usize) {
    if let (Some(s), Some(d)) = (src.get(at..at + len), dst.get_mut(at..at + len)) {
        d.copy_from_slice(s);
    }
}

/// Copies the body's occupancy, slot table and coverage/protection blocks
/// from a template into an instance's body.
fn copy_body_blocks(template: &[u8], payload: &mut [u8]) {
    let Some(tpl_range) = section_range(template, BODY) else {
        return;
    };
    let Some(dst_range) = section_range(payload, BODY) else {
        return;
    };
    let tpl_body = &template[tpl_range];
    let dst = &mut payload[dst_range];
    for (at, len) in [(0x10, 0xA0), (0xB0, 0xA0), (0x150, 0x50)] {
        copy_bytes(tpl_body, dst, at, len);
    }
}

/// Copies a template's appearance layers into an instance's appearance,
/// keeping a layer's material when the template's colour is left at zero.
/// The material-set check (`[guess]`) is not ported: an instance keeps its
/// stored material.
fn migrate_appearance(body: &mut [u8], template: &[u8]) {
    for i in 0..8 {
        let at = 0x1B4 + i * 12;
        let Some(tpl_layer) = template.get(at..at + 12) else {
            break;
        };
        let Some(layer) = body.get_mut(at..at + 12) else {
            break;
        };
        layer[0] = tpl_layer[0];
        layer[1] = tpl_layer[1];
        if tpl_layer[1] != 0 {
            layer[2] = tpl_layer[2];
        }
        layer[3] = tpl_layer[3];
        if tpl_layer[8] != 0 {
            layer[8] = tpl_layer[8];
        }
        if layer[0xB] != 5 {
            layer[0xB] = tpl_layer[0xB];
            if tpl_layer[0xB] == 5 {
                layer[8] = tpl_layer[8];
            }
        }
    }
}

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

#[cfg(test)]
mod record_tests {
    use super::*;

    fn padded(s: &str, len: usize) -> Vec<u8> {
        let mut v = s.as_bytes().to_vec();
        v.resize(len, 0);
        v
    }

    fn section(tag: u32, body: &[u8]) -> Vec<u8> {
        let mut v = tag.to_le_bytes().to_vec();
        v.extend((body.len() as u32 + 8).to_le_bytes());
        v.extend_from_slice(body);
        v
    }

    fn record(id: u32, flags: u32, sections: &[(u32, Vec<u8>)]) -> OwnedRecord {
        OwnedRecord {
            id,
            flags,
            payload: sections.iter().flat_map(|(t, b)| section(*t, b)).collect(),
        }
    }

    fn header(name: &str, flags: u32, parent: u32) -> Vec<u8> {
        let mut b = padded(name, 16);
        b.extend(flags.to_le_bytes());
        b.extend(parent.to_le_bytes());
        b
    }

    fn model(name: &str, flags: u32, name_id: u32, description_id: u32) -> Vec<u8> {
        let mut b = padded(name, 16);
        b.extend(flags.to_le_bytes());
        b.extend(name_id.to_le_bytes());
        b.extend(description_id.to_le_bytes());
        b.resize(0x28, 0);
        b
    }

    fn db() -> ObjectDb {
        ObjectDb::default()
    }

    #[test]
    fn sections_and_flags() {
        let r = record(
            1,
            NAME | MODEL,
            &[
                (NAME, header("Sword", 1, 0)),
                (MODEL, model("SwordWorld", 0x11, 7, 8)),
                (0x9999, vec![1, 2, 3]),
            ],
        );
        let view = r.view();
        let tags: Vec<u32> = view.sections().map(|s| s.tag).collect();
        assert_eq!(tags, [NAME, MODEL, 0x9999]);
        assert!(view.has(NAME) && view.has(MODEL) && !view.has(BODY));
        assert_eq!(view.section(0x9999), Some(&[1, 2, 3][..]));
        let h = view.header().unwrap();
        assert_eq!((h.name.as_str(), h.flags, h.parent), ("Sword", 1, 0));
        assert!(h.instance_on_use());
        let m = view.model().unwrap();
        assert_eq!(m.name, "SwordWorld");
        assert_eq!((m.flags, m.name_id, m.description_id), (0x11, 7, 8));
        // A malformed size stops the walk instead of looping.
        let mut bad = NAME.to_le_bytes().to_vec();
        bad.extend(4u32.to_le_bytes());
        bad.extend([0; 8]);
        let view = Record::new(1, NAME, &bad);
        assert_eq!(view.sections().count(), 0);
        assert_eq!(view.section(NAME), None);
    }

    #[test]
    fn tiers_lookup_and_free_ids() {
        let mut store = Store::default();
        assert_eq!(store.first_free_id(), 1);
        for id in [1, 3] {
            assert!(store.insert(record(id, 0, &[(NAME, header("x", 0, 0))])));
        }
        assert_eq!(store.find(1).unwrap().id, 1);
        assert_eq!(store.find(2), None);
        assert_eq!(store.find(3).unwrap().id, 3);
        assert_eq!(store.first_free_id(), 2);
        assert!(!store.insert(record(1, 0, &[])));
        assert_eq!(store.remove(1).unwrap().id, 1);
        assert_eq!(store.first_free_id(), 1);
        assert_eq!(Tier::of(5), Tier::Base);
        assert_eq!(Tier::of(LOCAL | 5), Tier::Local);
        assert_eq!(Tier::of(WORLD | 5), Tier::World);
        // The local bit wins over the world bit.
        assert_eq!(Tier::of(LOCAL | WORLD | 5), Tier::Local);
        assert_eq!(Tier::index(5), 5);
        assert_eq!(Tier::index(WORLD | 5), 5);
        assert_eq!(Tier::tag(Tier::World), WORLD);
        let db = db();
        assert_eq!(db.find(WORLD | 5), None);
    }

    #[test]
    fn string_ids_follow_inherit_own_and_shared() {
        let mut db = db();
        let shared = db.strings.add(b"Sword");
        assert_eq!(shared, 1);
        let shared_desc = db.strings.add(b"A sword.");
        let own = db.local_strings.add(b"Local name");
        let template = record(
            10,
            NAME | MODEL,
            &[
                (NAME, header("Template", 0, 0)),
                (MODEL, model("Ground", 0, shared, shared_desc)),
            ],
        );
        db.base.insert(template);
        assert_eq!(db.name(10).as_deref(), Some("Sword"));
        assert_eq!(db.description(10).as_deref(), Some("A sword."));
        // An own string lives in the tier's library.
        db.local.insert(record(
            1,
            NAME | MODEL,
            &[
                (NAME, header("Local", 0, 10)),
                (MODEL, model("Ground", 0, own | 0x8000_0000, 0)),
            ],
        ));
        assert_eq!(db.name(LOCAL | 1).as_deref(), Some("Local name"));
        assert_eq!(db.description(LOCAL | 1), None);
        // An inheriting instance reads the template's field from the shared
        // library.
        db.local.insert(record(
            2,
            NAME | MODEL,
            &[
                (NAME, header("Instance", 0, 10)),
                (MODEL, model("Ground", 0, INHERIT, INHERIT)),
            ],
        ));
        assert_eq!(db.name(LOCAL | 2).as_deref(), Some("Sword"));
        assert_eq!(db.description(LOCAL | 2).as_deref(), Some("A sword."));
        // A record without a model section has no name.
        db.base
            .insert(record(11, NAME, &[(NAME, header("Folder", 0, 0))]));
        assert_eq!(db.name(11), None);
    }

    #[test]
    fn instancing_and_cloning() {
        let mut db = db();
        db.strings.add(b"Sword");
        db.strings.add(b"A sword.");
        let mut body = vec![0u8; 0x224];
        body[0x150 + 4 * 8..0x150 + 4 * 9].copy_from_slice(&50u32.to_le_bytes());
        body[0x1A6] = 4;
        let mut payload = vec![];
        payload.extend(section(NAME, &header("Sword", INSTANCE_ON_USE, 0)));
        payload.extend(section(MODEL, &model("Ground", 0, 1, 2)));
        payload.extend(section(BODY, &body));
        let flags = NAME | MODEL | BODY;
        db.base.insert(OwnedRecord {
            id: 10,
            flags,
            payload,
        });
        let first = db.instantiate(10).unwrap();
        assert_eq!(first, LOCAL | 1);
        let view = db.find(first).unwrap().view();
        assert_eq!(view.template(), 10);
        assert!(view.header().unwrap().instance_on_use());
        assert_eq!(view.model().unwrap().name, "Ground");
        assert_eq!(view.model().unwrap().name_id, INHERIT);
        assert_eq!(view.model().unwrap().description_id, INHERIT);
        assert_eq!(db.section(first, BODY).unwrap()[0x150 + 4 * 8], 50);
        assert_eq!(db.find(first).unwrap().flags, flags);
        // A second instantiation takes the next local id.
        assert_eq!(db.instantiate(10).unwrap(), LOCAL | 2);
        // Duplicating an instance clones it under a new local id.
        let copy = db.duplicate_local(first).unwrap();
        assert_eq!(copy, LOCAL | 3);
        assert_eq!(db.section(copy, BODY).unwrap()[0x1A6], 4);
        // A world id is not instantiated and a base id without the flag is
        // left alone.
        assert_eq!(db.instantiate(WORLD | 10), None);
        db.base.insert(record(
            12,
            NAME | MODEL,
            &[
                (NAME, header("Plain", 0, 0)),
                (MODEL, model("Plain", 0, 0, 0)),
            ],
        ));
        assert_eq!(db.instantiate(12), None);
        // Cloning a template names it in the header and inherits the strings.
        let clone = db.clone_template_into(10, Tier::World).unwrap();
        assert_eq!(clone, WORLD | 1);
        let view = db.find(clone).unwrap().view();
        assert_eq!(view.template(), 10);
        assert_eq!(view.model().unwrap().name_id, INHERIT);
        assert_eq!(view.model().unwrap().description_id, INHERIT);
        assert_eq!(db.clone_template_into(10, Tier::Base), None);
    }

    #[test]
    fn promotion_moves_strings_children_and_ids() {
        let mut db = db();
        let own = db.local_strings.add(b"Local name");
        db.local.insert(record(
            1,
            NAME | MODEL,
            &[
                (NAME, header("Child", 0, 0)),
                (MODEL, model("Ground", 0, own | 0x8000_0000, 0)),
            ],
        ));
        let mut container = vec![0u8; 0x20 + 12];
        container[0x1C..0x20].copy_from_slice(&1u32.to_le_bytes());
        container[0x20..0x24].copy_from_slice(&(LOCAL | 1).to_le_bytes());
        db.local.insert(record(
            2,
            NAME | CONTAINER,
            &[(NAME, header("Container", 0, 0)), (CONTAINER, container)],
        ));
        db.set_remaps(true);
        db.set_link_remaps(true);
        // The child is promoted first, so it takes the first world id.
        let world = db.promote(LOCAL | 2).unwrap();
        assert_eq!(world, WORLD | 2);
        assert!(db.find(LOCAL | 2).is_none());
        assert_eq!(db.remapped(LOCAL | 1), Some(WORLD | 1));
        assert_eq!(db.remapped(LOCAL | 2), Some(WORLD | 2));
        // The child id inside the container follows.
        let body = db.section(WORLD | 2, CONTAINER).unwrap();
        assert_eq!(u32_at(body, 0x20), WORLD | 1);
        // The own string moved into the world library and the record still
        // names it.
        assert_eq!(db.name(WORLD | 1).as_deref(), Some("Local name"));
        assert_eq!(db.world_strings.refs(1), 1);
        // Promoting again while recording returns the recorded id.
        assert_eq!(db.promote(LOCAL | 1), Some(WORLD | 1));
        // With recording off, promotion still moves the record.
        db.set_remaps(false);
        db.local
            .insert(record(9, NAME, &[(NAME, header("X", 0, 0))]));
        let promoted = db.promote(LOCAL | 9).unwrap();
        assert_eq!(promoted, WORLD | 3);
        assert_eq!(db.remapped(LOCAL | 9), None);
        assert_eq!(db.promote(7), Some(7));
    }

    #[test]
    fn door_links_follow_kind_zero_remaps() {
        let mut db = db();
        let mut operative = vec![0u8; 0x3C];
        operative[0] = 1; // key
        operative[1] = 0;
        db.local.insert(record(
            1,
            NAME | OPERATIVE,
            &[(NAME, header("Key", 0, 0)), (OPERATIVE, operative)],
        ));
        let mut door = vec![0u8; 20];
        door[0x0C..0x10].copy_from_slice(&(LOCAL | 1).to_le_bytes());
        db.local.insert(record(
            2,
            NAME | DOOR,
            &[(NAME, header("Door", 0, 0)), (DOOR, door)],
        ));
        db.set_remaps(true);
        db.set_link_remaps(true);
        let key = db.promote(LOCAL | 1).unwrap();
        assert_eq!(key, WORLD | 1);
        assert_eq!(db.apply_link_remaps(), 1);
        assert_eq!(u32_at(db.section(LOCAL | 2, DOOR).unwrap(), 0x0C), key);
        // A second pass changes nothing.
        assert_eq!(db.apply_link_remaps(), 0);
    }

    #[test]
    fn migration_refreshes_from_the_template() {
        let mut db = db();
        let shared = db.strings.add(b"Worn");
        let mut template_body = vec![0u8; 0x224];
        for bone in [8usize, 9] {
            let at = 0x150 + 4 * (bone - 1);
            template_body[at..at + 4].copy_from_slice(&50u32.to_le_bytes());
        }
        let mut template_model = model("Worn", 0x10, shared, 0);
        template_model[0x1D] = 40;
        let mut template_weapon = vec![0u8; 0x60];
        for (at, v) in [
            (0x30u8, 10u8),
            (0x31, 20),
            (0x32, 127),
            (0x33, 30),
            (0x40, 128),
            (0x42, 0x45),
        ] {
            template_weapon[at as usize] = v;
        }
        db.base.insert(record(
            10,
            NAME | MODEL | BODY | WEAPON,
            &[
                (NAME, header("Template", 0, 0)),
                (MODEL, template_model),
                (BODY, template_body),
                (WEAPON, template_weapon),
            ],
        ));
        let mut own_body = vec![0u8; 0x224];
        own_body[0x4..0x8].copy_from_slice(&1u32.to_le_bytes());
        own_body[0x8..0xC].copy_from_slice(&2u32.to_le_bytes());
        own_body[0xF] = 0;
        db.local.insert(record(
            1,
            NAME | MODEL | BODY | WEAPON,
            &[
                (NAME, header("Instance", 0, 10)),
                (MODEL, model("Other", 0, 9, 9)),
                (BODY, own_body),
                (WEAPON, vec![0u8; 0x60]),
            ],
        ));
        assert!(db.migrate(LOCAL | 1));
        let view = db.find(LOCAL | 1).unwrap().view();
        assert_eq!(view.model().unwrap().name, "Worn");
        assert_eq!(view.model().unwrap().flags, 0x10);
        assert_eq!(view.model().unwrap().name_id, 9); // kept
        assert_eq!(view.model().unwrap().rarity, 40);
        let body = db.section(LOCAL | 1, BODY).unwrap();
        assert_eq!(u32_at(body, 0x150 + 4 * 7), 50);
        let weapon = db.section(LOCAL | 1, WEAPON).unwrap();
        assert_eq!(&weapon[0x30..0x34], &[10, 20, 127, 30]);
        assert_eq!(weapon[0x42], 0x45);
        // A base record never migrates.
        assert!(!db.migrate(10));
        // The appearance follows a material-class change.
        db.base.insert(record(
            11,
            NAME | MODEL | BODY,
            &[
                (NAME, header("Tpl", 0x12_0000, 0)),
                (MODEL, model("Worn", 0x10, 0, 0)),
                (BODY, {
                    let mut b = vec![0u8; 0x224];
                    b[0x1B0] = 1;
                    b
                }),
            ],
        ));
        db.local.insert(record(
            2,
            NAME | MODEL | BODY,
            &[
                (NAME, header("Inst", 0x11_0000, 11)),
                (MODEL, model("Other", 0, 9, 9)),
                (BODY, vec![0u8; 0x224]),
            ],
        ));
        assert!(db.migrate(LOCAL | 2));
        assert_eq!(
            db.section(LOCAL | 2, MODEL).unwrap()[0x14..0x1C],
            [0, 0, 0, 0x80, 0, 0, 0, 0x80]
        );
        assert_eq!(db.migrate_all(), 2);
    }

    #[test]
    fn folder_entries_and_fractions() {
        let mut b = vec![0u8; 0x10 + 2 * 16];
        b[0x0C..0x10].copy_from_slice(&2u32.to_le_bytes());
        b[0x10..0x14].copy_from_slice(&7u32.to_le_bytes());
        b[0x14..0x16].copy_from_slice(&0x640Au16.to_le_bytes());
        b[0x18..0x1A].copy_from_slice(&0x80u16.to_le_bytes());
        b[0x1A..0x1C].copy_from_slice(&0x5284u16.to_le_bytes());
        b[0x20..0x24].copy_from_slice(&9u32.to_le_bytes());
        let payload: Vec<u8> = section(NAME, &header("Folder", 0, 0))
            .into_iter()
            .chain(section(FOLDER, &b))
            .collect();
        let view = Record::new(1, NAME | FOLDER, &payload);
        let folder = view.folder().unwrap();
        assert_eq!(folder.entries.len(), 2);
        let e = folder.entries[0];
        assert_eq!((e.id, e.words, e.extra), (7, [0x640A, 0, 0x80], 0));
        assert_eq!(e.fractions.level(), 1.0);
        assert_eq!(e.fractions.field_6(), 0.5);
        assert_eq!(e.fractions.field_9(), 0.25);
        assert_eq!(e.fractions.field_11(), 0.5);
        assert_eq!(e.fractions.field_14(), 0.25);
        assert_eq!(folder.entries[1].fractions, FractionWord(0));
        assert_eq!(folder.entries[1].fractions.field_9(), 0.125);
        assert_eq!(folder.entries[1].fractions.field_14(), 0.125);
        let mut f = FractionWord(0);
        f.set_level(0.75);
        f.set_field_6(0.25);
        f.set_field_9(0.5);
        f.set_field_11(0.75);
        f.set_field_14(0.5);
        assert_eq!(f.level(), 0.75);
        assert_eq!(f.field_6(), 0.25);
        assert_eq!(f.field_9(), 0.5);
        assert_eq!(f.field_11(), 0.75);
        assert_eq!(f.field_14(), 0.5);
        f.set_field_9(0.125);
        assert_eq!(f.field_9(), 0.125);
        f.set_field_14(0.125);
        assert_eq!(f.field_14(), 0.125);
        // A short folder stops at its last whole entry.
        let mut short = b;
        short[0x0C..0x10].copy_from_slice(&100u32.to_le_bytes());
        let payload = section(FOLDER, &short);
        let view = Record::new(1, FOLDER, &payload);
        assert_eq!(view.folder().unwrap().entries.len(), 2);
    }

    #[test]
    fn database_errors_are_total() {
        assert!(matches!(
            ObjectDb::parse(&[], &[]),
            Err(Error::Truncated { .. })
        ));
        let mut b = MAGIC.to_le_bytes().to_vec();
        b.extend(16u32.to_le_bytes());
        b.extend([0u8; 16]);
        let db = ObjectDb::parse(&b, &[]).unwrap();
        assert!(db.base.is_empty());
        let mut old = b.clone();
        old[0..4].copy_from_slice(&0xDBAC_0D07u32.to_le_bytes());
        assert!(matches!(
            ObjectDb::parse(&old, &[]),
            Err(Error::Unsupported(_))
        ));
        let mut bad = b.clone();
        bad[0..4].copy_from_slice(&0x1234_5678u32.to_le_bytes());
        assert!(matches!(
            ObjectDb::parse(&bad, &[]),
            Err(Error::BadMagic { .. })
        ));
        // A section whose size is below its header cannot loop.
        let payload = {
            let mut p = NAME.to_le_bytes().to_vec();
            p.extend(4u32.to_le_bytes());
            p
        };
        let view = Record::new(1, NAME, &payload);
        assert_eq!(view.sections().count(), 0);
        assert!(view.header().is_none());
    }

    /// The shipped database: its record and tag counts, the flags' rule and
    /// the folder and string sections.
    #[test]
    fn shipped_object_database() {
        use std::collections::BTreeMap;
        let Some(dir) = exa_rpk::locate_install(None) else {
            return;
        };
        let game = exa_rpk::GameData::open(&dir).unwrap();
        let resource = game.pack(exa_rpk::PackKind::Resource).unwrap();
        let db = ObjectDb::parse(
            resource.get("objdb.rdb").unwrap(),
            resource.get("objstrings.rdb").unwrap(),
        )
        .unwrap();
        assert_eq!(db.base.len(), 1423);
        assert_eq!(db.strings.len(), 1124);
        assert_eq!(db.name(2).as_deref(), Some("Leather Gloves"));
        assert_eq!(
            db.description(2).as_deref(),
            Some("A pair of thick leather gauntlets.")
        );
        assert_eq!(db.name(31).as_deref(), Some("Torch"));
        assert_eq!(db.description(31).as_deref(), Some("A simple torch."));
        // Record 8 (the braies) has no ground model; 425 does.
        assert!(db.view(8).unwrap().model().is_none());
        assert!(db.view(8).unwrap().section(BODY).is_some());
        assert_eq!(db.view(425).unwrap().model().unwrap().name, "Braies_A");
        let mut tags: BTreeMap<u32, usize> = BTreeMap::new();
        let mut folders = 0;
        let mut inconsistent = 0;
        let mut unresolved = 0;
        for r in db.base.records() {
            let view = r.view();
            let mut or = 0;
            for s in view.sections() {
                *tags.entry(s.tag).or_default() += 1;
                or |= s.tag;
                if s.tag == FOLDER {
                    folders += 1;
                }
            }
            if or != r.flags && r.id != 1076 {
                inconsistent += 1;
            }
            if let Some(m) = view.model() {
                for raw in [m.name_id, m.description_id] {
                    if raw != 0 && raw != INHERIT && db.strings.bytes(raw & ID_MASK).is_none() {
                        unresolved += 1;
                    }
                }
            }
        }
        assert_eq!(inconsistent, 0);
        assert_eq!(unresolved, 0);
        assert_eq!(folders, 24);
        let expected: [(u32, usize); 11] = [
            (0x1000_0000, 1423),
            (0x0010_0000, 1403),
            (0x1000, 388),
            (0x4000, 208),
            (0x2_0000, 137),
            (0x1_0000, 49),
            (0x4_0000, 47),
            (0x200, 24),
            (0x8000, 22),
            (0x100, 1),
            (0x8000_0B03, 1),
        ];
        assert_eq!(tags.len(), expected.len());
        for (tag, count) in expected {
            assert_eq!(tags.get(&tag).copied(), Some(count), "tag {tag:#x}");
        }
        // Every folder's count fits its body.
        for r in db.base.records() {
            if let Some(folder) = r.view().folder() {
                // The first entry is the folder's own header: id 0.
                assert!(folder.entries.first().is_some_and(|e| e.id == 0));
                assert!(folder.entries.len() <= 0x4F);
            }
        }
    }
}

#[cfg(test)]
mod api_tests {
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

    fn record(id: u32, flags: u32, sections: &[(u32, Vec<u8>)]) -> OwnedRecord {
        OwnedRecord {
            id,
            flags,
            payload: sections.iter().flat_map(|(t, b)| section(*t, b)).collect(),
        }
    }

    fn header(name: &str) -> Vec<u8> {
        let mut b = padded(name, 16);
        b.extend(0u32.to_le_bytes());
        b.extend(0u32.to_le_bytes());
        b
    }

    fn model(name: &str) -> Vec<u8> {
        let mut b = padded(name, 16);
        b.extend(0u32.to_le_bytes());
        b.extend(0u32.to_le_bytes());
        b.extend(0u32.to_le_bytes());
        b
    }

    fn base(records: Vec<OwnedRecord>) -> ObjectDb {
        ObjectDb {
            base: Store::from_records(records),
            ..ObjectDb::default()
        }
    }

    #[test]
    fn name_and_model_lookups() {
        let db = base(vec![
            record(
                1,
                NAME | MODEL,
                &[(NAME, header("Sword")), (MODEL, model("Sword_A"))],
            ),
            record(
                2,
                NAME | MODEL,
                &[(NAME, header("Sword A")), (MODEL, model("Sword_B"))],
            ),
        ]);
        eprintln!(
            "headers {:?}",
            db.base
                .records()
                .iter()
                .map(|r| (r.id, r.view().header().map(|h| h.name)))
                .collect::<Vec<_>>()
        );
        assert_eq!(db.find_by_name("Sword A").unwrap().id, 2);
        assert_eq!(db.find_by_name("sword a").unwrap().id, 2);
        assert_eq!(db.find_by_name("nope"), None);
        assert_eq!(db.find_by_model("sword_b").unwrap().id, 2);
        assert_eq!(db.find_by_model("Sword_A").unwrap().id, 1);
    }

    #[test]
    fn item_view_decodes_the_record() {
        let mut body = vec![0u8; 0x224];
        body[0..9].copy_from_slice(b"Braies_A\0");
        let db = base(vec![record(
            8,
            NAME | BODY,
            &[(NAME, header("Braies_A")), (BODY, body)],
        )]);
        let items = db.items();
        assert_eq!(items.len(), 1);
        assert_eq!((items[0].id, items[0].name.as_str()), (8, "Braies_A"));
        assert_eq!(items[0].worn_model, "Braies_A");
        assert_eq!(db.base.records()[0].view().item().unwrap().id, 8);
    }

    #[test]
    fn quantised_fields_round_trip() {
        let mut q = Quantised(0);
        q.set_count(2.6);
        assert_eq!(q.count(), 3);
        q.set_count(-1.0);
        assert_eq!(q.count(), 0);
        q.set_count(9.0);
        assert_eq!(q.count(), 3);
        q.set_signed(-0.5);
        assert!((q.signed() + 0.5).abs() <= 1.0 / 31.0);
        q.set_signed(1.0);
        assert!((q.signed() - 1.0).abs() < 1e-6);
        assert_eq!(q.count(), 3);
        // The three-level fractions: 0, 0.125, 0.25, 0.5, 0.75, 1.
        let mut q = Quantised(0);
        q.set_fraction(0, 0.0);
        assert_eq!(q.fraction(0), 0.0);
        q.set_fraction(0, 0.125);
        assert_eq!(q.fraction(0), 0.125);
        q.set_fraction(0, 0.375);
        assert_eq!(q.fraction(0), 0.25);
        q.set_fraction(0, 0.5);
        assert_eq!(q.fraction(0), 0.5);
        q.set_fraction(0, 0.625);
        assert_eq!(q.fraction(0), 0.5);
        q.set_fraction(0, 0.75);
        assert_eq!(q.fraction(0), 0.75);
        q.set_fraction(0, 1.0);
        assert_eq!(q.fraction(0), 1.0);
        q.set_fraction(0, 2.0);
        assert_eq!(q.fraction(0), 1.0);
        q.set_fraction(1, 0.75);
        assert_eq!(q.fraction(1), 0.75);
        q.set_fraction(2, 1.0);
        assert_eq!(q.fraction(2), 0.5);
        // A two-bit field cannot hold 0.75.
        q.set_fraction(2, 0.75);
        assert_eq!(q.fraction(2), 0.5);
        assert_eq!(Quantised(5 << 5).high3(), 5);
        assert_eq!(Quantised(0x1F).low5(), 31);
    }

    #[test]
    fn writer_round_trips_and_keeps_flags() {
        let db = base(vec![
            record(
                1,
                NAME | MODEL,
                &[(NAME, header("A")), (MODEL, model("A_Model"))],
            ),
            record(5, NAME, &[(NAME, header("B"))]),
        ]);
        let bytes = db.write(Tier::Base);
        let parsed = ObjectDb::parse(&bytes, &[]).unwrap();
        assert_eq!(parsed.base.len(), 2);
        for (a, b) in db.base.records().iter().zip(parsed.base.records()) {
            assert_eq!((a.id, a.flags, &a.payload), (b.id, b.flags, &b.payload));
        }
        // The table carries the original's trailing empty slot.
        assert_eq!(u32::from_le_bytes(bytes[4..8].try_into().unwrap()), 3 * 16);
        // An empty tier writes the header alone; that is not a parseable
        // file, and the parser says so instead of reading past the end.
        let empty = ObjectDb::default().write(Tier::Local);
        assert_eq!(empty.len(), 8);
        assert!(ObjectDb::parse(&empty, &[]).is_err());
    }
}
