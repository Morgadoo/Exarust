//! What a shop reads of an object-database item record: whether the item
//! may be stocked, the item a level-local copy was made from, how rare it is
//! and the rank it is sold at. Layout and evidence: `docs/formats/OTHER.md`
//! (Object database, Trade fields).
//!
//! A record payload is the same chain of `u32 tag, u32 size` sections that
//! [`crate::objdb`] reads. The first section of each tag is the one read.
//! A field past the end of its section reads as zero.

use crate::{Error, Result};

const NAME: u32 = 0x1000_0000;
const MODEL: u32 = 0x0010_0000;
const BODY: u32 = 0x1000;
const WEAPON: u32 = 0x4000;
const SHIELD: u32 = 0x8000;

/// Name section: the word after the name, whose bit 1 keeps the item off a
/// shop's shelves, and the parent id after it.
const LISTING_OFFSET: usize = 0x10;
const PARENT_OFFSET: usize = 0x14;
/// Model section: the flags word after the name, the rarity byte and the
/// two words a stocked item needs non-zero.
const MODEL_FLAGS_OFFSET: usize = 0x10;
const RARITY_OFFSET: usize = 0x1D;
const SIZE_OFFSET: usize = 0x20;
/// The byte whose top three bits give the rank, by the section that holds it.
const BODY_RANK_OFFSET: usize = 0x1AB;
const WEAPON_RANK_OFFSET: usize = 0x42;
const SHIELD_RANK_OFFSET: usize = 0x08;
/// The rank sits in the top three bits of its byte.
const RANK_SHIFT: u32 = 5;
/// The model flags' rarity tier, 0 to 7.
pub const TIER_MASK: u32 = 0x70;
const TIER_SHIFT: u32 = 4;
/// A listing word with this bit keeps the item out of every stock.
pub const UNLISTED: u32 = 0x2;

/// The shop fields of one item record.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TradeRecord {
    /// The name section's listing word; `None` without a name section.
    pub listing: Option<u32>,
    /// The id the name section names as the item's parent: the editor
    /// folder for a global item, the source item for a level-local copy.
    pub parent: u32,
    /// The model section's shop fields; `None` without a model section.
    pub model: Option<TradeModel>,
    /// The rank, 0 to 7: from the item body, else the weapon section, else
    /// the shield section, else 0.
    pub rank: u8,
}

/// The model section's fields a shop reads.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TradeModel {
    /// The model name's first byte is not zero.
    pub named: bool,
    /// The flags word after the name; bits `0x70` are the rarity tier.
    pub flags: u32,
    /// How rare the item is, in hundredths: a shop weighs it by
    /// `1 − rarity × 0.01`.
    pub rarity: u8,
    /// Two words a stocked item needs both non-zero. What they hold is not
    /// traced [guess: the icon's size].
    pub size: [u32; 2],
}

impl TradeModel {
    /// The rarity tier, 0 to 7, that prices a discovery.
    pub fn tier(&self) -> u8 {
        ((self.flags & TIER_MASK) >> TIER_SHIFT) as u8
    }
}

impl TradeRecord {
    /// Whether the listing word lets a shop stock the item: a name section
    /// without bit [`UNLISTED`].
    pub fn listed(&self) -> bool {
        self.listing.is_some_and(|word| word & UNLISTED == 0)
    }

    /// The shop fields of a record payload. A section shorter than its
    /// header or running past the payload is an error.
    pub fn parse(payload: &[u8]) -> Result<Self> {
        let mut name = None;
        let mut model = None;
        let mut body = None;
        let mut weapon = None;
        let mut shield = None;
        let mut pos = 0;
        while pos + 8 <= payload.len() {
            let tag = crate::read_u32(payload, pos)?;
            let size = crate::read_u32(payload, pos + 4)? as usize;
            if size < 8 {
                return Err(Error::Malformed(format!("item section of {size} bytes")));
            }
            let section = payload.get(pos + 8..pos + size).ok_or(Error::Truncated {
                offset: pos + 8,
                needed: size - 8,
                len: payload.len(),
            })?;
            let slot = match tag {
                NAME => &mut name,
                MODEL => &mut model,
                BODY => &mut body,
                WEAPON => &mut weapon,
                SHIELD => &mut shield,
                _ => {
                    pos += size;
                    continue;
                }
            };
            if slot.is_none() {
                *slot = Some(section);
            }
            pos += size;
        }
        let rank_byte = if let Some(b) = body {
            byte(b, BODY_RANK_OFFSET)
        } else if let Some(w) = weapon {
            byte(w, WEAPON_RANK_OFFSET)
        } else if let Some(s) = shield {
            byte(s, SHIELD_RANK_OFFSET)
        } else {
            0
        };
        Ok(Self {
            listing: name.map(|n| word(n, LISTING_OFFSET)),
            parent: name.map_or(0, |n| word(n, PARENT_OFFSET)),
            model: model.map(|m| TradeModel {
                named: byte(m, 0) != 0,
                flags: word(m, MODEL_FLAGS_OFFSET),
                rarity: byte(m, RARITY_OFFSET),
                size: [word(m, SIZE_OFFSET), word(m, SIZE_OFFSET + 4)],
            }),
            rank: rank_byte >> RANK_SHIFT,
        })
    }
}

fn byte(b: &[u8], at: usize) -> u8 {
    b.get(at).copied().unwrap_or(0)
}

fn word(b: &[u8], at: usize) -> u32 {
    b.get(at..at + 4)
        .map_or(0, |w| u32::from_le_bytes([w[0], w[1], w[2], w[3]]))
}

/// The OR of every section tag in a record payload: the flags word a shop
/// tests against the kinds it stocks. A section that is short, runs past
/// the payload or has a header under eight bytes stops the walk (the tags
/// read before it still count), so a malformed payload yields what was
/// legible rather than an error.
#[must_use]
pub fn section_flags(payload: &[u8]) -> u32 {
    let mut flags = 0u32;
    let mut pos = 0usize;
    while pos + 8 <= payload.len() {
        let tag = word(payload, pos);
        let size = word(payload, pos + 4) as usize;
        if size < 8 || pos + size > payload.len() {
            break;
        }
        flags |= tag;
        pos += size;
    }
    flags
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

    /// A stocked helm: listed, made from item 812, tier 3 (flags 0x31),
    /// rarity 40, rank 6 from its body byte 0xd4.
    #[test]
    fn a_helm_reads_its_shop_fields() {
        let mut name = b"Helm A".to_vec();
        name.resize(0x18, 0);
        name[0x14..0x18].copy_from_slice(&812u32.to_le_bytes());
        let mut model = b"HelmA".to_vec();
        model.resize(0x28, 0);
        model[0x10..0x14].copy_from_slice(&0x31u32.to_le_bytes());
        model[0x1D] = 40;
        model[0x20..0x24].copy_from_slice(&2u32.to_le_bytes());
        model[0x24..0x28].copy_from_slice(&3u32.to_le_bytes());
        let mut body = vec![0u8; 0x1B0];
        body[0x1AB] = 0xd4;
        let mut p = section(NAME, &name);
        p.extend(section(MODEL, &model));
        p.extend(section(BODY, &body));
        let r = TradeRecord::parse(&p).unwrap();
        assert!(r.listed());
        assert_eq!(r.parent, 812);
        let m = r.model.unwrap();
        assert!(m.named);
        assert_eq!((m.tier(), m.rarity, m.size), (3, 40, [2, 3]));
        assert_eq!(r.rank, 6);
    }

    /// The body's rank wins over the weapon's; a weapon alone gives its
    /// byte 0x42, a shield alone its byte 8; nothing gives 0.
    #[test]
    fn the_rank_comes_from_the_first_kind_present() {
        let mut weapon = vec![0u8; 0x43];
        weapon[0x42] = 0x60;
        let mut shield = vec![0u8; 0x50];
        shield[8] = 0xa0;
        let mut body = vec![0u8; 0x1AC];
        body[0x1AB] = 0x20;
        let mut both = section(WEAPON, &weapon);
        both.extend(section(BODY, &body));
        assert_eq!(TradeRecord::parse(&both).unwrap().rank, 1);
        assert_eq!(
            TradeRecord::parse(&section(WEAPON, &weapon)).unwrap().rank,
            3
        );
        assert_eq!(
            TradeRecord::parse(&section(SHIELD, &shield)).unwrap().rank,
            5
        );
        let none = TradeRecord::parse(&[]).unwrap();
        assert_eq!((none.rank, none.listing, none.model), (0, None, None));
        assert!(!none.listed());
    }

    /// Bit 1 of the listing word unlists; a short model section reads zero.
    #[test]
    fn an_unlisted_item_and_a_short_section() {
        let mut name = vec![0u8; 0x14];
        name[0x10] = 0x2;
        let mut p = section(NAME, &name);
        p.extend(section(MODEL, b"X"));
        let r = TradeRecord::parse(&p).unwrap();
        assert!(!r.listed());
        assert_eq!(r.parent, 0);
        assert_eq!(r.model.unwrap().size, [0, 0]);
    }

    #[test]
    fn malformed_sections_are_errors() {
        let mut short = NAME.to_le_bytes().to_vec();
        short.extend(4u32.to_le_bytes());
        assert!(TradeRecord::parse(&short).is_err());
        let mut long = NAME.to_le_bytes().to_vec();
        long.extend(64u32.to_le_bytes());
        assert!(TradeRecord::parse(&long).is_err());
    }

    /// The kinds word is the OR of the payload's section tags, and a
    /// malformed tail leaves only the tags read before it.
    #[test]
    fn the_section_tags_are_the_kinds_word() {
        let mut p = section(NAME, &[0u8; 4]);
        p.extend(section(MODEL, &[0u8; 4]));
        p.extend(section(BODY, &[0u8; 4]));
        assert_eq!(section_flags(&p), NAME | MODEL | BODY);
        assert_eq!(section_flags(&[]), 0);
        // A header whose size runs past the payload stops the walk.
        let mut cut = p.clone();
        cut.extend(BODY.to_le_bytes());
        cut.extend(64u32.to_le_bytes());
        assert_eq!(section_flags(&cut), NAME | MODEL | BODY);
        // A header under eight bytes stops it before its tag counts.
        let mut short = p.clone();
        short.extend(WEAPON.to_le_bytes());
        short.extend(4u32.to_le_bytes());
        assert_eq!(section_flags(&short), NAME | MODEL | BODY);
    }
}
