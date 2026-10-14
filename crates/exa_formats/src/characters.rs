//! Characters placed in a level (chunk `0xCAC00001`) and their records in
//! the level's own character database. Layout and evidence:
//! `docs/formats/RFC.md` (Starts, characters and placed objects).

use crate::rdb::Database;
use crate::{Error, Result};

pub const CHARACTERS: u32 = 0xCAC0_0001;
const CHARACTER_DB: u32 = 0xCECD_DB00;
const PLACEMENT: u32 = 0xCAC0_CA00;
const POSE: u32 = 0xCAC0_EB00;
const LEGACY_POSE: u32 = 0xCAC0_E100;
/// Condition of a dead body, placed from its pose.
pub const DEAD: i32 = 0xA00;
const CENTIMETRES_PER_METRE: f32 = 100.0;
/// Record ids carry flags in their top bits.
pub const ID_MASK: u32 = 0x0FFF_FFFF;
pub const WORLD_DB: u32 = 0x4000_0000;
/// Item ids of the level's own object database.
pub const LOCAL_ITEM: u32 = 0x8000_0000;

/// A character standing (or lying) in a level.
#[derive(Debug, Clone, PartialEq)]
pub struct Placement {
    pub id: u32,
    /// Feet position, metres.
    pub position: [f32; 3],
    /// Facing: the character looks along (sin yaw, 0, cos yaw).
    pub yaw: f32,
    pub condition: i32,
    /// Per-bone 3×4 matrices (rows, translation in metres) of a posed body,
    /// in depth-first order of the skeleton.
    pub pose: Vec<[[f32; 3]; 4]>,
}

impl Placement {
    pub fn is_dead(&self) -> bool {
        self.condition == DEAD
    }

    pub fn record(&self) -> u32 {
        self.id & ID_MASK
    }
}

/// A character's record: who they are and what they wear.
#[derive(Debug, Clone, PartialEq)]
pub struct CharacterRecord {
    pub id: u32,
    pub name: String,
    pub surname: String,
    pub female: bool,
    pub race: u8,
    pub voice: String,
    /// Items held in the hands (two loadouts), 0 = none.
    pub hands: [u32; 4],
    /// Worn apparel items.
    pub worn: Vec<u32>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct LevelCharacters {
    pub records: Vec<CharacterRecord>,
    pub placements: Vec<Placement>,
}

impl LevelCharacters {
    pub fn record(&self, id: u32) -> Option<&CharacterRecord> {
        self.records.iter().find(|r| r.id == id & ID_MASK)
    }

    pub fn parse(b: &[u8]) -> Result<Self> {
        let mut r = Reader { b, pos: 0 };
        let count = r.i32()?.max(0) as usize;
        let mut out = Self::default();
        while r.remaining() >= 8 {
            let tag = r.u32()?;
            let size = r.u32()? as usize;
            let body = r.bytes(size)?;
            match tag {
                CHARACTER_DB => {
                    let db = Database::parse(body)?;
                    out.records = db
                        .records
                        .iter()
                        .filter_map(|rec| CharacterRecord::parse(rec.id, db.payload(rec)).ok())
                        .collect();
                }
                PLACEMENT => out.placements.push(read_placement(body)?),
                _ => {}
            }
        }
        if out.placements.len() > count {
            out.placements.truncate(count);
        }
        Ok(out)
    }
}

fn read_placement(b: &[u8]) -> Result<Placement> {
    let mut r = Reader { b, pos: 0 };
    let id = r.u32()?;
    let position = [r.f32()?, r.f32()?, r.f32()?].map(|v| v / CENTIMETRES_PER_METRE);
    let yaw = r.f32()?;
    let condition = r.i32()?;
    let mut pose = Vec::new();
    while r.remaining() >= 8 {
        let tag = r.u32()?;
        let size = r.u32()? as usize;
        let body = r.bytes(size)?;
        match tag {
            POSE => read_bone_tree(&mut Reader { b: body, pos: 0 }, &mut pose)?,
            LEGACY_POSE if pose.is_empty() => {
                let mut p = Reader { b: body, pos: 0 };
                while p.remaining() >= 48 {
                    pose.push(p.matrix()?);
                }
            }
            _ => {}
        }
    }
    Ok(Placement {
        id,
        position,
        yaw,
        condition,
        pose,
    })
}

/// A bone and its children: matrix, a word, child count, children.
fn read_bone_tree(r: &mut Reader<'_>, out: &mut Vec<[[f32; 3]; 4]>) -> Result<()> {
    out.push(r.matrix()?);
    r.u32()?;
    let children = r.i32()?;
    if !(0..=64).contains(&children) || out.len() > 256 {
        return Err(Error::Malformed(format!(
            "pose node with {children} children"
        )));
    }
    for _ in 0..children {
        read_bone_tree(r, out)?;
    }
    Ok(())
}

impl CharacterRecord {
    /// Parses the fields up to the worn apparel (record versions 5–7).
    pub fn parse(id: u32, b: &[u8]) -> Result<Self> {
        let mut r = Reader { b, pos: 0 };
        let tag = r.u32()?;
        if tag & 0xFFFF_FF00 != 0x00CD_AC00 {
            return Err(Error::BadMagic {
                found: tag,
                expected: 0x00CD_AC07,
            });
        }
        let version = tag & 0xFF;
        r.u32()?;
        let name = r.name()?;
        let surname = r.name()?;
        r.bytes(4 * 4)?; // physique, height, age
        r.bytes(4 + 4 + 4)?; // skin colour, hair style, hair colour
        if version >= 3 {
            r.bytes(4)?; // face styles
        }
        let voice = r.name()?;
        r.f32()?; // pitch
        let profession = r.u32()?;
        r.i32()?; // last locale
        let mut hands = [0u32; 4];
        for h in &mut hands {
            *h = r.u32()?;
            r.u32()?;
        }
        let worn_count = r.i32()?;
        if !(0..=64).contains(&worn_count) {
            return Err(Error::Malformed(format!("{worn_count} worn items")));
        }
        let mut worn = Vec::with_capacity(worn_count as usize);
        for _ in 0..worn_count {
            worn.push(r.u32()?);
            r.u32()?;
        }
        Ok(Self {
            id,
            name,
            surname,
            female: profession & 1 != 0,
            race: (profession >> 8) as u8,
            voice,
            hands,
            worn,
        })
    }
}

struct Reader<'a> {
    b: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    fn remaining(&self) -> usize {
        self.b.len() - self.pos
    }

    fn bytes(&mut self, n: usize) -> Result<&'a [u8]> {
        let s = self.b.get(self.pos..self.pos + n).ok_or(Error::Truncated {
            offset: self.pos,
            needed: n,
            len: self.b.len(),
        })?;
        self.pos += n;
        Ok(s)
    }

    fn u32(&mut self) -> Result<u32> {
        Ok(u32::from_le_bytes(
            self.bytes(4)?.try_into().expect("4 bytes"),
        ))
    }

    fn i32(&mut self) -> Result<i32> {
        Ok(self.u32()? as i32)
    }

    fn f32(&mut self) -> Result<f32> {
        Ok(f32::from_bits(self.u32()?))
    }

    fn name(&mut self) -> Result<String> {
        let raw = self.bytes(16)?;
        let end = raw.iter().position(|&c| c == 0).unwrap_or(16);
        Ok(raw[..end].iter().map(|&c| c as char).collect())
    }

    fn matrix(&mut self) -> Result<[[f32; 3]; 4]> {
        let mut m = [[0.0; 3]; 4];
        for row in &mut m {
            for v in row.iter_mut() {
                *v = self.f32()?;
            }
        }
        for v in &mut m[3] {
            *v /= CENTIMETRES_PER_METRE;
        }
        Ok(m)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn u32s(v: &[u32]) -> Vec<u8> {
        v.iter().flat_map(|x| x.to_le_bytes()).collect()
    }

    fn name(s: &str) -> Vec<u8> {
        let mut n = s.as_bytes().to_vec();
        n.resize(16, 0);
        n
    }

    fn record() -> Vec<u8> {
        let mut b = u32s(&[0x00CD_AC07, 0]);
        b.extend(name("Zombie03"));
        b.extend(name(""));
        b.extend([0u8; 16 + 12 + 4]);
        b.extend(name("vi01"));
        b.extend(1.0f32.to_le_bytes());
        b.extend(u32s(&[0x0001_0001, 0]));
        b.extend(u32s(&[0x17, 0, 0, 0, 0, 0, 0, 0]));
        b.extend(u32s(&[2, 0x8000_0010, 0, 0x8000_00CE, 0]));
        b
    }

    #[test]
    fn character_record_up_to_worn_items() {
        let r = CharacterRecord::parse(2, &record()).unwrap();
        assert_eq!(r.name, "Zombie03");
        assert_eq!(r.voice, "vi01");
        assert!(r.female);
        assert_eq!(r.race, 0);
        assert_eq!(r.hands[0], 0x17);
        assert_eq!(r.worn, [0x8000_0010, 0x8000_00CE]);
    }

    #[test]
    fn placements_and_their_database() {
        let rec = record();
        let mut db = u32s(&[0xDBAC_CD00, 16, 2, 0, 0, rec.len() as u32]);
        db.extend(&rec);
        let mut place = u32s(&[0x8000_0002]);
        for v in [2500.0f32, 0.0, -1000.0, 1.5] {
            place.extend(v.to_le_bytes());
        }
        place.extend((DEAD).to_le_bytes());
        let mut b = 1i32.to_le_bytes().to_vec();
        b.extend(u32s(&[CHARACTER_DB, db.len() as u32]));
        b.extend(&db);
        b.extend(u32s(&[PLACEMENT, place.len() as u32]));
        b.extend(&place);
        let c = LevelCharacters::parse(&b).unwrap();
        let p = &c.placements[0];
        assert_eq!(p.position, [25.0, 0.0, -10.0]);
        assert!(p.is_dead());
        assert_eq!(c.record(p.id).unwrap().name, "Zombie03");
    }
}
