//! Turns a loaded level section into the exporter's scene, which is what a
//! save stores as the visited level's section.
//!
//! The structured reader ([`crate::rfc`]) keeps the nodes, meshes, the grid
//! and the placements in metres, but drops a few records the exporter writes
//! back: the material payloads, the level's object and string tables, the
//! environment record, and the lattice spacing of a mesh's GI points (it
//! keeps the points only). [`LevelRaw`] scans those from the section's
//! chunks, so the conversion needs no second parse of the whole file.
//!
//! Conversion starts here: [`LevelRaw::scan`] is the first piece, and the
//! scene builder over a [`crate::rfc::Scene`] follows it.

use crate::chunk::{Chunks, list_children, rfc_chunks};
use crate::{Error, Result};

/// Root chunks read here.
const MATERIAL_LIST: u32 = 0x0000_BA00;
const MATERIAL: u32 = 0x0000_BA01;
const LOCAL_DATABASE: u32 = 0x0000_DBC0;
const LOCAL_OBJECTS: u32 = 0x0000_DBDB;
const LOCAL_STRINGS: u32 = 0x0000_DBCD;
const ENVIRONMENT: u32 = 0xEC00_DC00;

/// The GI lattice spacing (cm) every shipped mesh with points stores: the
/// points sample a 40 cm lattice (326 point chunks over four shipped levels
/// all carry it). The reader keeps the points but not the word, so the
/// conversion writes this measured value back.
pub const GI_LATTICE_SPACING: u32 = 40;

/// The records of a level section that the structured reader does not keep,
/// as the exporter needs them.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LevelRaw {
    /// The section's material list: each material's 16-byte stored name and
    /// the payload after it, in file order.
    pub materials: Vec<([u8; 16], Vec<u8>)>,
    /// The local object database's record body (sub-chunk `0xDBDB`): its
    /// magic, table length, table and data, as the exporter writes them.
    pub object_database: Option<Vec<u8>>,
    /// The local string table's body (sub-chunk `0xDBCD`).
    pub object_strings: Option<Vec<u8>>,
    /// The environment record's body (`0xEC00DC00`, 0xF8 bytes in the
    /// shipped files).
    pub environment: Option<Vec<u8>>,
}

impl LevelRaw {
    /// Scans a level section (a chunk tree). Unknown root chunks are
    /// skipped, as the reader skips them; a malformed list is an error.
    pub fn scan(section: &[u8]) -> Result<Self> {
        let mut out = Self::default();
        for chunk in rfc_chunks(section)? {
            let chunk = chunk?;
            match chunk.tag {
                MATERIAL_LIST => out.read_materials(&chunk)?,
                LOCAL_DATABASE => out.read_database(&chunk)?,
                ENVIRONMENT => out.environment = Some(chunk.data.to_vec()),
                _ => {}
            }
        }
        Ok(out)
    }

    fn read_materials(&mut self, list: &crate::chunk::Chunk<'_>) -> Result<()> {
        let Some((count, children)) = list_children(list) else {
            return Err(Error::Malformed("material list is not a list".into()));
        };
        self.materials.reserve(count as usize);
        for child in children {
            if child.tag != MATERIAL {
                continue;
            }
            let mut name = [0u8; 16];
            name.copy_from_slice(child.data.get(..16).ok_or(Error::Truncated {
                offset: child.offset,
                needed: 16,
                len: child.data.len(),
            })?);
            self.materials.push((name, child.data[16..].to_vec()));
        }
        Ok(())
    }

    fn read_database(&mut self, db: &crate::chunk::Chunk<'_>) -> Result<()> {
        for sub in Chunks::new(db.data, 0) {
            let sub = sub?;
            match sub.tag {
                LOCAL_OBJECTS => self.object_database = Some(sub.data.to_vec()),
                LOCAL_STRINGS => self.object_strings = Some(sub.data.to_vec()),
                _ => {}
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chunk(tag: u32, payload: &[u8]) -> Vec<u8> {
        let mut v = tag.to_le_bytes().to_vec();
        v.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        v.extend_from_slice(payload);
        v
    }

    fn name(s: &str) -> [u8; 16] {
        let mut n = [0u8; 16];
        n[..s.len()].copy_from_slice(s.as_bytes());
        n
    }

    fn section(roots: &[Vec<u8>]) -> Vec<u8> {
        let mut v = crate::chunk::RFC_MAGIC.to_le_bytes().to_vec();
        for r in roots {
            v.extend_from_slice(r);
        }
        v
    }

    #[test]
    fn the_scanner_keeps_materials_databases_and_the_environment() {
        let mut list = 2u32.to_le_bytes().to_vec();
        let mut wood = name("Wood").to_vec();
        wood.extend_from_slice(&[1, 2, 3]);
        list.extend(chunk(MATERIAL, &wood));
        // A non-material child is skipped, as the reader's list walk does.
        list.extend(chunk(0x1234, b"skip"));

        let mut db = chunk(LOCAL_OBJECTS, b"magicdb");
        db.extend(chunk(LOCAL_STRINGS, b"strings"));
        db.extend(chunk(0xDBCE, b"objects"));

        let env = vec![7u8; 0xF8];
        let bytes = section(&[
            chunk(MATERIAL_LIST, &list),
            chunk(LOCAL_DATABASE, &db),
            chunk(ENVIRONMENT, &env),
            chunk(0x9999, b"unknown"),
        ]);
        let raw = LevelRaw::scan(&bytes).unwrap();
        assert_eq!(raw.materials, [(name("Wood"), vec![1, 2, 3])]);
        assert_eq!(raw.object_database.as_deref(), Some(&b"magicdb"[..]));
        assert_eq!(raw.object_strings.as_deref(), Some(&b"strings"[..]));
        assert_eq!(raw.environment.as_deref(), Some(&env[..]));
    }

    #[test]
    fn an_empty_and_a_malformed_section() {
        let empty = LevelRaw::scan(&section(&[])).unwrap();
        assert_eq!(empty, LevelRaw::default());
        assert!(LevelRaw::scan(b"not a tree").is_err());
        // A list that is not shaped like a list is an error, not a panic.
        let bytes = section(&[chunk(MATERIAL_LIST, b"\x02\x00\x00\x00")]);
        assert!(LevelRaw::scan(&bytes).is_err());
        // A material record shorter than its name is truncated.
        let mut list = 1u32.to_le_bytes().to_vec();
        list.extend(chunk(MATERIAL, b"short"));
        let bytes = section(&[chunk(MATERIAL_LIST, &list)]);
        assert!(matches!(
            LevelRaw::scan(&bytes),
            Err(Error::Truncated { needed: 16, .. })
        ));
    }
}
