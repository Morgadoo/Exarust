//! The object string library (`objstrings.rdb`): the pool of display names
//! and description phrases the object database refers to by id.
//!
//! Layout and evidence: `docs/formats/OTHER.md` (Object database, String
//! library). Header: `magic: u32` (`0xAFBF0C02`), `table_len: u32`, then
//! `table_len / 16` entries of `u32` unused, `u32` references, `u32` offset
//! and `u32` length; the text follows the table and offsets count from there.
//! Ids are one-based: id 0 means "none" and an entry whose length is 0 is
//! empty. The last entry is garbage, as in the record databases, and is
//! skipped.
//!
//! Text is Latin-1, as everywhere else in the engine's data; [`Library::get`]
//! returns it as UTF-8. A freed entry keeps its slot and is reused by
//! [`Library::add`], which is what the original's library does.

use crate::data_stream::latin1_to_utf8;
use crate::{Error, Result};

/// The library file's magic.
pub const MAGIC: u32 = 0xAFBF_0C02;

const ENTRY_LEN: usize = 16;
const HEADER_LEN: usize = 8;

/// One text slot. `refs` is how many records use it: an entry with no
/// references is free, and its text may be reused.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct Entry {
    refs: u32,
    offset: u32,
    len: u32,
}

/// A string library: the text slots of one database, plus the text they
/// point into.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Library {
    entries: Vec<Entry>,
    text: Vec<u8>,
    /// The slot a freed entry left behind: [`Library::add`] starts looking
    /// for a free slot here, as the original keeps its high-water mark.
    free_hint: usize,
}

impl Library {
    /// Parses a library; the last table entry is skipped as garbage.
    pub fn parse(b: &[u8]) -> Result<Self> {
        let magic = crate::read_u32(b, 0)?;
        if magic != MAGIC {
            return Err(Error::BadMagic {
                found: magic,
                expected: MAGIC,
            });
        }
        let table_len = crate::read_u32(b, 4)? as usize;
        if !table_len.is_multiple_of(ENTRY_LEN) {
            return Err(Error::Malformed(format!(
                "string table length {table_len} is not a multiple of {ENTRY_LEN}"
            )));
        }
        let text_start = HEADER_LEN
            .checked_add(table_len)
            .filter(|&s| s <= b.len())
            .ok_or(Error::Truncated {
                offset: HEADER_LEN,
                needed: table_len,
                len: b.len().saturating_sub(HEADER_LEN),
            })?;
        let count = table_len / ENTRY_LEN;
        let count = count.saturating_sub(1);
        let text = &b[text_start..];
        let mut entries = Vec::with_capacity(count);
        for i in 0..count {
            let at = HEADER_LEN + i * ENTRY_LEN;
            let entry = b.get(at..at + ENTRY_LEN).ok_or(Error::Truncated {
                offset: at,
                needed: ENTRY_LEN,
                len: b.len(),
            })?;
            let word = |o: usize| u32::from_le_bytes(entry[o..o + 4].try_into().expect("4 bytes"));
            let e = Entry {
                refs: word(4),
                offset: word(8),
                len: word(12),
            };
            if (e.offset as usize)
                .checked_add(e.len as usize)
                .is_none_or(|end| end > text.len())
            {
                return Err(Error::Malformed(format!(
                    "string {} runs past the text ({} + {} of {})",
                    i + 1,
                    e.offset,
                    e.len,
                    text.len()
                )));
            }
            entries.push(e);
        }
        Ok(Self {
            entries,
            text: text.to_vec(),
            free_hint: 0,
        })
    }

    /// The number of text slots; ids run from 1 to this.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// The text of `id` in the file's Latin-1 bytes, or `None` for id 0, an
    /// id past the table or an empty entry.
    pub fn bytes(&self, id: u32) -> Option<&[u8]> {
        let e = self.entry(id)?;
        if e.len == 0 {
            return None;
        }
        let start = e.offset as usize;
        self.text.get(start..start + e.len as usize)
    }

    /// The text of `id` as UTF-8, or `None` for id 0, an id past the table
    /// or an empty entry.
    pub fn get(&self, id: u32) -> Option<String> {
        let raw = self.bytes(id)?;
        Some(String::from_utf8_lossy(&latin1_to_utf8(raw)).into_owned())
    }

    /// How many records use the entry (`0` for a free slot or a missing id).
    pub fn refs(&self, id: u32) -> u32 {
        self.entry(id).map_or(0, |e| e.refs)
    }

    fn entry(&self, id: u32) -> Option<&Entry> {
        if id == 0 {
            return None;
        }
        self.entries.get(id as usize - 1)
    }

    /// Adds `text` to the first free slot at or after the free hint, else
    /// to a new one, and returns its id. The entry starts with one
    /// reference.
    pub fn add(&mut self, text: &[u8]) -> u32 {
        let slot = (self.free_hint..self.entries.len())
            .find(|&i| self.entries[i].refs == 0)
            .unwrap_or_else(|| {
                self.entries.push(Entry::default());
                self.entries.len() - 1
            });
        self.entries[slot] = Entry {
            refs: 1,
            ..Entry::default()
        };
        self.write(slot, text);
        slot as u32 + 1
    }

    /// Replaces the text of `id`. Empty text frees the entry, as the
    /// original's setter does.
    pub fn set(&mut self, id: u32, text: &[u8]) -> bool {
        let Some(slot) = (id != 0)
            .then(|| id as usize - 1)
            .filter(|&s| s < self.entries.len())
        else {
            return false;
        };
        if text.is_empty() {
            self.release(id);
            return true;
        }
        self.write(slot, text);
        true
    }

    /// Records one more use of `id`.
    pub fn add_ref(&mut self, id: u32) {
        if let Some(slot) = (id != 0)
            .then(|| id as usize - 1)
            .filter(|&s| s < self.entries.len())
        {
            self.entries[slot].refs = self.entries[slot].refs.saturating_add(1);
        }
    }

    /// Drops one use of `id`; the slot's text becomes free at zero.
    pub fn release(&mut self, id: u32) {
        let Some(slot) = (id != 0)
            .then(|| id as usize - 1)
            .filter(|&s| s < self.entries.len())
        else {
            return;
        };
        let refs = &mut self.entries[slot].refs;
        *refs = refs.saturating_sub(1);
        if *refs == 0 {
            self.entries[slot] = Entry::default();
            if self.free_hint < slot {
                self.free_hint = slot;
            }
        }
    }

    /// Appends `text` to the text area and points the entry at it.
    fn write(&mut self, slot: usize, text: &[u8]) {
        let old = self.entries[slot];
        let old_end = old.offset as usize + old.len as usize;
        let new_len = text.len() as u32;
        if old.len as usize != text.len()
            && let Some(tail) = self.text.get(old_end..).map(<[u8]>::to_vec)
        {
            // Close the hole the old text leaves and shift the entries after
            // it, as the original compacts its text area.
            self.text.truncate(old.offset as usize);
            self.text.extend_from_slice(&tail);
            for entry in &mut self.entries {
                if entry.len != 0 && entry.offset as usize >= old_end && entry.offset > old.offset {
                    entry.offset -= old.len;
                }
            }
        }
        let at = self.text.len() as u32;
        self.text.extend_from_slice(text);
        let entry = &mut self.entries[slot];
        entry.offset = at;
        entry.len = new_len;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A library file with the given `(refs, text)` entries and the
    /// trailing garbage entry every shipped file has.
    fn library(entries: &[(u32, &str)]) -> Vec<u8> {
        let mut text = Vec::new();
        let mut table = Vec::new();
        for &(refs, s) in entries {
            let offset = text.len() as u32;
            text.extend_from_slice(s.as_bytes());
            table.extend(0u32.to_le_bytes());
            table.extend(refs.to_le_bytes());
            table.extend(offset.to_le_bytes());
            table.extend((s.len() as u32).to_le_bytes());
        }
        table.extend([0u8; ENTRY_LEN]);
        let mut b = MAGIC.to_le_bytes().to_vec();
        b.extend((table.len() as u32).to_le_bytes());
        b.extend_from_slice(&table);
        b.extend_from_slice(&text);
        b
    }

    #[test]
    fn entries_are_one_based_and_empty_ones_are_missing() {
        let b = library(&[(3, "Sword"), (1, ""), (2, "Two-handed Sword")]);
        let lib = Library::parse(&b).unwrap();
        assert_eq!(lib.len(), 3);
        assert_eq!(lib.get(0), None);
        assert_eq!(lib.get(1).as_deref(), Some("Sword"));
        assert_eq!(lib.get(2), None);
        assert_eq!(lib.get(3).as_deref(), Some("Two-handed Sword"));
        assert_eq!(lib.get(4), None);
        assert_eq!(lib.refs(1), 3);
        assert_eq!(lib.refs(2), 1);
        assert_eq!(lib.refs(4), 0);
    }

    #[test]
    fn text_is_latin_1() {
        let mut t = MAGIC.to_le_bytes().to_vec();
        let table_len = 32u32;
        t.extend(table_len.to_le_bytes());
        t.extend(0u32.to_le_bytes());
        t.extend(1u32.to_le_bytes());
        t.extend(0u32.to_le_bytes());
        t.extend(4u32.to_le_bytes());
        t.extend([0u8; ENTRY_LEN]);
        t.extend([b'c', b'a', b'f', 0xe9]);
        let lib = Library::parse(&t).unwrap();
        assert_eq!(lib.get(1).as_deref(), Some("café"));
        assert_eq!(lib.bytes(1), Some(&[b'c', b'a', b'f', 0xe9][..]));
    }

    #[test]
    fn adding_reuses_a_freed_slot_from_the_hint() {
        let mut lib = Library::parse(&library(&[(1, "Sword"), (1, "Maul")])).unwrap();
        assert_eq!(lib.add(b"Sword"), 3);
        assert_eq!(lib.get(3).as_deref(), Some("Sword"));
        lib.release(3);
        assert_eq!(lib.refs(3), 0);
        assert_eq!(lib.get(3), None);
        // The freed third slot is the hint, so the next add reuses it.
        assert_eq!(lib.add(b"Helm"), 3);
        assert_eq!(lib.get(3).as_deref(), Some("Helm"));
        assert_eq!(lib.add(b"Shield"), 4);
        assert_eq!(lib.len(), 4);
    }

    #[test]
    fn replacing_text_keeps_the_ids_of_the_other_entries() {
        let mut lib = Library::parse(&library(&[
            (1, "Sword"),
            (1, "Two-handed Sword"),
            (1, "Maul"),
        ]))
        .unwrap();
        assert!(lib.set(2, b"Axe"));
        assert_eq!(lib.get(1).as_deref(), Some("Sword"));
        assert_eq!(lib.get(2).as_deref(), Some("Axe"));
        assert_eq!(lib.get(3).as_deref(), Some("Maul"));
        // Growing again shifts the later text.
        assert!(lib.set(2, b"Two-handed Sword"));
        assert_eq!(lib.get(3).as_deref(), Some("Maul"));
        // Empty text frees the slot.
        assert!(lib.set(2, b""));
        assert_eq!(lib.get(2), None);
        assert_eq!(lib.refs(2), 0);
        assert!(!lib.set(9, b"x"));
    }

    #[test]
    fn reference_counting() {
        let mut lib = Library::parse(&library(&[(1, "Sword")])).unwrap();
        lib.add_ref(1);
        assert_eq!(lib.refs(1), 2);
        lib.release(1);
        assert_eq!(lib.get(1).as_deref(), Some("Sword"));
        lib.release(1);
        assert_eq!(lib.get(1), None);
        // Releasing a missing id changes nothing.
        lib.release(0);
        lib.release(9);
        lib.add_ref(0);
        assert_eq!(lib.len(), 1);
    }

    #[test]
    fn malformed_files_are_errors() {
        let good = library(&[(1, "Sword")]);
        assert!(matches!(Library::parse(&[]), Err(Error::Truncated { .. })));
        let mut bad_magic = good.clone();
        bad_magic[0] ^= 0xFF;
        assert!(matches!(
            Library::parse(&bad_magic),
            Err(Error::BadMagic { .. })
        ));
        let mut short = good.clone();
        short[4] = 8; // table length 8, not a multiple of 16
        assert!(matches!(Library::parse(&short), Err(Error::Malformed(_))));
        let mut past = good.clone();
        // The first entry's length runs past the text area.
        past[8 + 12..8 + 16].copy_from_slice(&1000u32.to_le_bytes());
        assert!(matches!(Library::parse(&past), Err(Error::Malformed(_))));
        let mut truncated = good.clone();
        truncated.truncate(20);
        assert!(Library::parse(&truncated).is_err());
        // An empty but well-formed library.
        assert!(Library::parse(&library(&[])).unwrap().is_empty());
    }

    /// The shipped library: every entry lies in the text area (the parse
    /// checks it), the ids are dense and the known strings are where the
    /// records point.
    #[test]
    fn shipped_string_library() {
        let Some(dir) = exa_rpk::locate_install(None) else {
            return;
        };
        let game = exa_rpk::GameData::open(&dir).unwrap();
        let resource = game.pack(exa_rpk::PackKind::Resource).unwrap();
        let lib = Library::parse(resource.get("objstrings.rdb").unwrap()).unwrap();
        assert_eq!(lib.len(), 1124);
        assert_eq!(lib.get(1).as_deref(), Some("Sword"));
        assert_eq!(lib.get(2).as_deref(), Some("Two-handed Sword"));
        assert_eq!(lib.get(88).as_deref(), Some("Breastplate"));
        assert_eq!(lib.get(89).as_deref(), Some("Plate Helm"));
        assert_eq!(lib.get(285).as_deref(), Some("Falchion"));
        assert_eq!(lib.get(0xaa).as_deref(), Some("Torch"));
        assert_eq!(lib.get(0xb6).as_deref(), Some("A simple torch."));
        assert!(lib.refs(0) == 0 && lib.refs(1) > 0);
        assert!(lib.refs(1125) == 0);
    }
}
