//! Item records of the object database (`objdb.rdb`): names, the model they
//! show, and the appearance block that colours apparel. Layout and evidence:
//! `docs/formats/OTHER.md` (Object database).
//!
//! A record payload is a chain of `u32 tag, u32 size` sections (the size
//! counts the 8-byte header): `0x10000000`
//! holds the display name, `0x00100000` the model name, `0x1000` the item
//! body whose bytes `0x1B0..0x214` are the appearance block.

use crate::rdb::Database;
use crate::{Error, Result};

const NAME: u32 = 0x1000_0000;
const MODEL: u32 = 0x0010_0000;
const BODY: u32 = 0x1000;
const APPEARANCE_OFFSET: usize = 0x1B0;
/// Per-slot layer bytes of apparel: 8 bytes per body slot (bone index).
const SLOT_LAYERS_OFFSET: usize = 0xA8;
pub const SLOTS: usize = 19;
/// A slot byte with this bit hides the body part of that slot.
pub const HIDES_BODY: u8 = 0x80;
const APPEARANCE_LEN: usize = 4 + LAYERS * LAYER_LEN;
const LAYERS: usize = 8;
const LAYER_LEN: usize = 12;

#[derive(Debug, Clone, PartialEq)]
pub struct Item {
    pub id: u32,
    pub name: String,
    /// Model entry (e.g. in `Characters.rpk` for apparel); may be empty.
    pub model: String,
    pub appearance: Option<Appearance>,
    /// Apparel's first layer byte per body slot: low bits the layer, bit
    /// `0x80` hides the body part, `0x40` its partial variant.
    pub slot_layers: [u8; SLOTS],
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
            appearance: None,
            slot_layers: [0; SLOTS],
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
                MODEL => item.model = name(body),
                BODY => {
                    item.appearance = body
                        .get(APPEARANCE_OFFSET..)
                        .and_then(|b| Appearance::parse(b).ok());
                    if let Some(t) = body.get(SLOT_LAYERS_OFFSET..SLOT_LAYERS_OFFSET + SLOTS * 8) {
                        item.slot_layers = std::array::from_fn(|s| t[s * 8]);
                    }
                }
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
    fn truncated_section_is_an_error() {
        let mut p = NAME.to_le_bytes().to_vec();
        p.extend(100u32.to_le_bytes());
        assert!(Item::parse(1, &p).is_err());
    }
}
