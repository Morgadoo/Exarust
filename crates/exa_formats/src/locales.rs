//! `locales.rdb`: the level-name table the level loaders and save-game
//! transitions look levels up in.
//!
//! It is an id-keyed database ([`UidDatabase`]) with magic `0xDBAE0D01`,
//! whose records start with a 16-byte NUL-padded name. The original creates
//! it lazily, loads it from the resource pack, finds an id by walking the
//! records and comparing the names, and copies a record's 16 name bytes.
//!
//! Layout evidence: `docs/formats/RFC.md` (locales) and the generic `.rdb`
//! index in [`crate::rdb`].

use crate::Result;
use crate::uid::{LOCALE_MAGIC, UidDatabase};

/// The bytes of a record's name field.
pub const NAME_LEN: usize = 16;

/// A locale database.
#[derive(Debug, Clone, Default)]
pub struct LocaleDb {
    db: UidDatabase,
}

impl LocaleDb {
    /// Reads `locales.rdb`.
    pub fn load(bytes: &[u8]) -> Result<Self> {
        let mut db = UidDatabase::new(LOCALE_MAGIC);
        db.load(bytes)?;
        Ok(Self { db })
    }

    /// The underlying database.
    pub fn database(&self) -> &UidDatabase {
        &self.db
    }

    /// The first record whose name is `name`, as an id (0 when none).
    ///
    /// The original lower-cases each stored 16-byte name and compares all
    /// 16 bytes with the caller's 16-byte query as it is given (the callers
    /// hand it a name already in lower case). Here the query is lower-cased
    /// too, so a level name in any case finds its record; a name longer than
    /// 16 bytes is cut and a shorter one is padded with NULs. A miss is 0,
    /// as record ids start at 1.
    pub fn find(&self, name: &str) -> u32 {
        let mut query = [0u8; NAME_LEN];
        for (slot, byte) in query.iter_mut().zip(name.bytes()) {
            *slot = byte.to_ascii_lowercase();
        }
        for index in 0..self.db.len() {
            let Some(payload) = self.db.payload(index) else {
                continue;
            };
            let stored = name_of(payload);
            if stored == query {
                return self.db.records()[index].id;
            }
        }
        0
    }

    /// The 16 name bytes of `id`, as the original copies them.
    pub fn name(&self, id: u32) -> Option<[u8; NAME_LEN]> {
        let payload = self.db.payload_of(id)?;
        let field = payload.get(..NAME_LEN)?;
        let mut name = [0; NAME_LEN];
        name.copy_from_slice(field);
        Some(name)
    }

    /// The name of `id` as text: NUL-padded, one byte per character.
    pub fn name_text(&self, id: u32) -> Option<String> {
        self.name(id).map(|b| {
            let end = b.iter().position(|&c| c == 0).unwrap_or(NAME_LEN);
            b[..end].iter().map(|&c| c as char).collect()
        })
    }
}

/// A record payload's name field, 16 bytes wide, ASCII-lower-cased as the
/// original compares it.
fn name_of(payload: &[u8]) -> [u8; NAME_LEN] {
    let mut field = [0u8; NAME_LEN];
    for (slot, byte) in field.iter_mut().zip(payload.iter().copied()) {
        *slot = byte.to_ascii_lowercase();
    }
    field
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A locales file: records `(id, name)` in order, each record a 16-byte
    /// name followed by a 4-byte word.
    fn locales(records: &[(u32, &str)]) -> Vec<u8> {
        let mut blob = Vec::new();
        let mut table = Vec::new();
        for &(id, name) in records {
            let mut field = [0u8; NAME_LEN];
            field[..name.len()].copy_from_slice(name.as_bytes());
            table.extend(id.to_le_bytes());
            table.extend(0u32.to_le_bytes());
            table.extend((blob.len() as u32).to_le_bytes());
            table.extend(((NAME_LEN + 4) as u32).to_le_bytes());
            blob.extend_from_slice(&field);
            blob.extend_from_slice(&7u32.to_le_bytes());
        }
        table.extend([0; 16]);
        let mut b = LOCALE_MAGIC.to_le_bytes().to_vec();
        b.extend((table.len() as u32).to_le_bytes());
        b.extend(table);
        b.extend(blob);
        b
    }

    #[test]
    fn names_are_found_exactly_and_copied_sixteen_bytes_wide() {
        let db = LocaleDb::load(&locales(&[(1, "Underworld0"), (7, "Arena")])).unwrap();
        assert_eq!(db.find("Underworld0"), 1);
        assert_eq!(db.find("Arena"), 7);
        assert_eq!(db.find("arena"), 7);
        assert_eq!(db.find("Missing"), 0);
        assert_eq!(db.name_text(7).as_deref(), Some("Arena"));
        assert_eq!(db.name(7).unwrap(), *b"Arena\0\0\0\0\0\0\0\0\0\0\0");
        assert_eq!(db.name_text(1).as_deref(), Some("Underworld0"));
        assert_eq!(db.name(9), None);
    }

    #[test]
    fn an_empty_or_bad_file_finds_nothing() {
        let db = LocaleDb::load(&locales(&[])).unwrap();
        assert_eq!(db.find("Arena"), 0);
        assert_eq!(db.name(1), None);
        assert!(LocaleDb::load(&[]).is_err());
        let mut bad = locales(&[(1, "Arena")]);
        bad[0] ^= 0xFF;
        // A wrong magic clears the table, as the original's manager does.
        let db = LocaleDb::load(&bad).unwrap();
        assert_eq!(db.find("Arena"), 0);
    }

    /// The shipped table: 17 records and their level names.
    #[test]
    fn the_shipped_locales() {
        let Some(dir) = exa_rpk::locate_install(None) else {
            return;
        };
        let game = exa_rpk::GameData::open(&dir).unwrap();
        let file = game
            .pack(exa_rpk::PackKind::Resource)
            .unwrap()
            .get("locales.rdb")
            .unwrap();
        let db = LocaleDb::load(file).unwrap();
        assert_eq!(db.database().len(), 17);
        assert_eq!(db.find("Exanima05_Sw"), 9);
        assert_eq!(db.find("exanima01"), 2);
        assert_eq!(db.name_text(1).as_deref(), Some("Exanima"));
        assert_eq!(db.name_text(17).as_deref(), Some("Arak_Ar"));
        assert_eq!(db.find("nope"), 0);
    }
}
