//! `races.rdb`: the character races. Each record (624 bytes) names the race,
//! its body model stem (`Skel` → `skelbase.rfc` in `Resource.rpk`) and its
//! movesets (the shared `Default` and the race's own, such as `Golem`),
//! followed by body parameters and sound names not decoded yet.
//!
//! A character record's race byte is the race record's id minus one.

use crate::rdb::Database;
use crate::{Error, Result};

/// A race record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Race {
    pub id: u32,
    pub name: String,
    /// Body model stem.
    pub stem: String,
    /// The shared and the race's own moveset.
    pub movesets: [String; 2],
}

const NAME: usize = 0x00;
const STEM: usize = 0x20;
const MOVESETS: [usize; 2] = [0x38, 0x48];
const RECORD: usize = 0x270;

fn text(b: &[u8], at: usize, len: usize) -> String {
    let field = &b[at..at + len];
    let end = field.iter().position(|&c| c == 0).unwrap_or(len);
    String::from_utf8_lossy(&field[..end]).into_owned()
}

impl Race {
    /// The body model entry in `Resource.rpk`.
    pub fn model(&self) -> String {
        format!("{}base.rfc", self.stem.to_ascii_lowercase())
    }

    /// The race's own moveset.
    pub fn moveset(&self) -> &str {
        &self.movesets[1]
    }
}

/// Every race in the database, in table order (duplicate ids keep the
/// first; empty entries are skipped).
pub fn parse(bytes: &[u8]) -> Result<Vec<Race>> {
    let db = Database::parse(bytes)?;
    let mut races: Vec<Race> = Vec::new();
    for r in &db.records {
        let b = db.payload(r);
        // The table ends with an empty entry.
        if b.is_empty() {
            continue;
        }
        if b.len() < RECORD {
            return Err(Error::Malformed(format!(
                "race record {} is {} bytes",
                r.id,
                b.len()
            )));
        }
        if races.iter().any(|x| x.id == r.id) {
            continue;
        }
        races.push(Race {
            id: r.id,
            name: text(b, NAME, 0x20),
            stem: text(b, STEM, 0x18),
            movesets: MOVESETS.map(|at| text(b, at, 0x10)),
        });
    }
    Ok(races)
}

/// The race of a character record's race byte.
pub fn of_character(races: &[Race], race: u8) -> Option<&Race> {
    races.iter().find(|r| r.id == u32::from(race) + 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(name: &str, stem: &str, own: &str) -> Vec<u8> {
        let mut b = vec![0u8; RECORD];
        b[NAME..NAME + name.len()].copy_from_slice(name.as_bytes());
        b[STEM..STEM + stem.len()].copy_from_slice(stem.as_bytes());
        b[MOVESETS[0]..MOVESETS[0] + 7].copy_from_slice(b"Default");
        b[MOVESETS[1]..MOVESETS[1] + own.len()].copy_from_slice(own.as_bytes());
        b
    }

    #[test]
    fn races_by_character_byte() {
        let records = [
            record("Human", "Human", "Default"),
            record("Golem Guard", "Golem", "Golem"),
        ];
        let mut file = Vec::new();
        file.extend_from_slice(&0xDBCB_0D00u32.to_le_bytes());
        file.extend_from_slice(&32u32.to_le_bytes());
        for (i, id) in [1u32, 17].iter().enumerate() {
            file.extend_from_slice(&id.to_le_bytes());
            file.extend_from_slice(&0u32.to_le_bytes());
            file.extend_from_slice(&((i * RECORD) as u32).to_le_bytes());
            file.extend_from_slice(&(RECORD as u32).to_le_bytes());
        }
        for r in &records {
            file.extend_from_slice(r);
        }
        let races = parse(&file).unwrap();
        let golem = of_character(&races, 16).unwrap();
        assert_eq!(golem.model(), "golembase.rfc");
        assert_eq!(golem.moveset(), "Golem");
        assert_eq!(of_character(&races, 0).unwrap().name, "Human");
        assert!(of_character(&races, 1).is_none());
    }

    #[test]
    fn short_records_are_rejected() {
        let mut file = Vec::new();
        file.extend_from_slice(&0u32.to_le_bytes());
        file.extend_from_slice(&16u32.to_le_bytes());
        for v in [1u32, 0, 0, 8] {
            file.extend_from_slice(&v.to_le_bytes());
        }
        file.extend_from_slice(&[0u8; 8]);
        assert!(parse(&file).is_err());
    }
}
