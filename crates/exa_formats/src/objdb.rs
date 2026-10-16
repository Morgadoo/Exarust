//! Item records of the object database (`objdb.rdb`): names, the models
//! they are shown and worn as, what they occupy, and the appearance block
//! that colours apparel. Layout and evidence: `docs/formats/OTHER.md`
//! (Object database).
//!
//! A record payload is a chain of `u32 tag, u32 size` sections (the size
//! counts the 8-byte header): `0x10000000`
//! holds the display name, `0x00100000` the ground model's name, `0x1000`
//! the item body, which starts with the worn model's name, holds the
//! per-bone occupancy table at `0x10` and the appearance block at bytes
//! `0x1B0..0x214`; `0x4000` a weapon's section and `0x8000` a shield's;
//! `0x40000` a door's and `0x10000` an operative object's (levers,
//! buttons, pressure plates, keys, signal jobs and relays).

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
    pub const KEY: u8 = 1;
    pub const LEVER: u8 = 3;
    /// Buttons and pressure plates.
    pub const PUSH_SURFACE: u8 = 5;
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
}

impl Operative {
    fn parse(b: &[u8]) -> Option<Self> {
        let b = b.get(..16)?;
        let u16_at = |o: usize| u16::from_le_bytes(b[o..o + 2].try_into().expect("2 bytes"));
        let u32_at = |o: usize| u32::from_le_bytes(b[o..o + 4].try_into().expect("4 bytes"));
        Some(Self {
            class: b[0],
            kind: b[1],
            flags: u16_at(2),
            value: u16_at(4),
            time: u16_at(6),
            target: u32_at(8),
            second: u32_at(12),
        })
    }

    /// A lever's auto-return time in milliseconds.
    pub fn return_ms(&self) -> u32 {
        u32::from(self.time) * 100
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
        let shield = Item::parse(80, &p).unwrap().shield.unwrap();
        assert_eq!(
            (shield.weight, shield.block, shield.bulk),
            (15.0 * 0.01, 0.5, 8.0 * 0.01)
        );
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
