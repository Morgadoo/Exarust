//! The object string library (`objstrings.rdb`, and a level's own library
//! in its local-database chunk): the pool of display names and description
//! phrases the object database refers to by id.
//!
//! Layout and evidence: `docs/formats/OTHER.md` (Object database, String
//! library). Header: `magic: u32` (`0xAFBF0C02`), `table_len: u32`, then
//! `table_len / 16 − 1` entries of `u32` (unused), `u32` references, `u32`
//! offset and `u32` length, one more table slot the original writes back
//! as it holds it, and the text, whose offsets count from the end of the
//! table. Ids are one-based: id 0 means "none" and an entry whose length is
//! 0 reads as nothing.
//!
//! The routines are the original's: [`Library::alloc`] takes the first slot
//! without references at or after the free hint, else appends one;
//! [`Library::set`] closes the hole the old text leaves and appends the new
//! text (or overwrites it in place when the length is the same);
//! [`Library::release`] drops a reference and zeroes the slot at none,
//! leaving its text as a hole; [`Library::write`] saves the table and the
//! text as they are held, so a loaded library writes back byte for byte.
//!
//! Text is Latin-1, as everywhere else in the engine's data; [`Library::get`]
//! returns it as UTF-8.

use crate::data_stream::latin1_to_utf8;
use crate::{Error, Result};

/// The library file's magic.
pub const MAGIC: u32 = 0xAFBF_0C02;

const ENTRY_LEN: usize = 16;
const HEADER_LEN: usize = 8;

/// One text slot as the table holds it.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Entry {
    /// The first word: never read; zero in every shipped entry and zeroed
    /// when a slot is freed.
    pub unused: u32,
    /// How many records use the slot: a slot with none is free.
    pub refs: u32,
    /// Where the text starts in the text area.
    pub offset: u32,
    /// The text's length; 0 for no text.
    pub len: u32,
}

impl Entry {
    fn read(b: &[u8]) -> Self {
        let word = |o: usize| u32::from_le_bytes(b[o..o + 4].try_into().expect("4 bytes"));
        Self {
            unused: word(0),
            refs: word(4),
            offset: word(8),
            len: word(12),
        }
    }

    fn bytes(self) -> [u8; ENTRY_LEN] {
        let mut out = [0; ENTRY_LEN];
        for (i, w) in [self.unused, self.refs, self.offset, self.len]
            .into_iter()
            .enumerate()
        {
            out[i * 4..i * 4 + 4].copy_from_slice(&w.to_le_bytes());
        }
        out
    }
}

/// A string library: the text slots of one database, plus the text they
/// point into.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Library {
    entries: Vec<Entry>,
    /// The table slot after the last entry. The original's writer saves one
    /// slot more than it has entries; this is what that slot held when the
    /// library was read (zero once the table has grown, where the original
    /// writes whatever lies past its allocation).
    tail: [u8; ENTRY_LEN],
    /// A file whose table is shorter than one slot: the original counts −1
    /// entries and writes a table length of 0 back.
    negative: bool,
    text: Vec<u8>,
    /// Where [`Library::alloc`] starts looking for a free slot: the highest
    /// slot a release freed (0 after loading).
    free_hint: usize,
}

impl Library {
    /// Reads a library as the original's loader does: the entry count is
    /// `table_len / 16 − 1`; with fewer than one entry nothing past the
    /// header is read (the shipped empty libraries are the 8-byte header
    /// alone); otherwise the text is everything after the table. Entries
    /// are not checked against the text: one that points outside it reads
    /// as nothing, where the original reads past its buffer.
    pub fn parse(b: &[u8]) -> Result<Self> {
        let magic = crate::read_u32(b, 0)?;
        if magic != MAGIC {
            return Err(Error::BadMagic {
                found: magic,
                expected: MAGIC,
            });
        }
        let table_len = crate::read_u32(b, 4)? as usize;
        let count = (table_len >> 4) as i64 - 1;
        if count < 1 {
            return Ok(Self {
                negative: count < 0,
                ..Self::default()
            });
        }
        let count = count as usize;
        let text_start = HEADER_LEN + table_len;
        if text_start > b.len() {
            return Err(Error::Truncated {
                offset: HEADER_LEN,
                needed: table_len,
                len: b.len().saturating_sub(HEADER_LEN),
            });
        }
        let table = &b[HEADER_LEN..text_start];
        let entries = (0..count)
            .map(|i| Entry::read(&table[i * ENTRY_LEN..(i + 1) * ENTRY_LEN]))
            .collect();
        let tail = table[count * ENTRY_LEN..(count + 1) * ENTRY_LEN]
            .try_into()
            .expect("16 bytes");
        Ok(Self {
            entries,
            tail,
            negative: false,
            text: b[text_start..].to_vec(),
            free_hint: 0,
        })
    }

    /// Writes the library as the original saves it: the magic, a table
    /// length of `(entries + 1) × 16`, and, when there is an entry, the
    /// entries, the slot after them and the text area as held.
    pub fn write(&self) -> Vec<u8> {
        let count = if self.negative && self.entries.is_empty() {
            -1
        } else {
            self.entries.len() as i64
        };
        let mut out = MAGIC.to_le_bytes().to_vec();
        out.extend((((count + 1) * ENTRY_LEN as i64) as u32).to_le_bytes());
        if count > 0 {
            for e in &self.entries {
                out.extend(e.bytes());
            }
            out.extend(self.tail);
            out.extend_from_slice(&self.text);
        }
        out
    }

    /// The number of text slots; ids run from 1 to this.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Empties the library as the original's clear does: no slots, no free
    /// hint and no text. The table's trailing slot is forgotten with them.
    pub fn clear(&mut self) {
        *self = Self::default();
    }

    /// The slots as the table holds them.
    pub fn entries(&self) -> &[Entry] {
        &self.entries
    }

    /// The text area, holes included.
    pub fn text(&self) -> &[u8] {
        &self.text
    }

    /// The text of `id` in the file's Latin-1 bytes, or `None` for id 0, an
    /// id past the table, an entry without text or one whose text lies
    /// outside the text area.
    pub fn bytes(&self, id: u32) -> Option<&[u8]> {
        let e = self.entry(id)?;
        if e.len == 0 {
            return None;
        }
        let start = e.offset as usize;
        self.text.get(start..start.checked_add(e.len as usize)?)
    }

    /// The text of `id` as UTF-8, by the rules of [`Library::bytes`]: the
    /// original's getter fails for id 0 and for an entry without text.
    pub fn get(&self, id: u32) -> Option<String> {
        let raw = self.bytes(id)?;
        Some(String::from_utf8_lossy(&latin1_to_utf8(raw)).into_owned())
    }

    /// How many records use the entry (`0` for a free slot or a missing id).
    pub fn refs(&self, id: u32) -> u32 {
        self.entry(id).map_or(0, |e| e.refs)
    }

    fn slot(&self, id: u32) -> Option<usize> {
        let slot = (id as usize).checked_sub(1)?;
        (slot < self.entries.len()).then_some(slot)
    }

    fn entry(&self, id: u32) -> Option<&Entry> {
        self.slot(id).map(|s| &self.entries[s])
    }

    /// Takes a slot for a new string, as the original's allocator does:
    /// the first slot without references from the free hint on gets one
    /// reference (its offset and length stay as they are, zero for a
    /// released slot); with none, a zeroed slot with one reference is
    /// appended. Returns the slot's id.
    pub fn alloc(&mut self) -> u32 {
        if let Some(slot) =
            (self.free_hint..self.entries.len()).find(|&i| self.entries[i].refs == 0)
        {
            self.entries[slot].refs = 1;
            return slot as u32 + 1;
        }
        self.entries.push(Entry {
            refs: 1,
            ..Entry::default()
        });
        self.tail = [0; ENTRY_LEN];
        self.negative = false;
        self.entries.len() as u32
    }

    /// Allocates a slot and sets its text: what the original's callers do
    /// to give a record an own string. Returns the id.
    pub fn add(&mut self, text: &[u8]) -> u32 {
        let id = self.alloc();
        self.set(id, text);
        id
    }

    /// Replaces the text of `id`, as the original's setter does. Empty text
    /// releases the slot and gives `false`. Text of another length closes
    /// the old text's hole (every entry whose text starts after it moves
    /// down) and goes to the end of the text area; text of the same length
    /// overwrites the old in place.
    pub fn set(&mut self, id: u32, text: &[u8]) -> bool {
        if text.is_empty() {
            self.release(id);
            return false;
        }
        let Some(slot) = self.slot(id) else {
            return false;
        };
        let old = self.entries[slot];
        let new_len = text.len() as u32;
        if new_len != old.len {
            if old.len != 0 {
                let start = (old.offset as usize).min(self.text.len());
                let end = (old.offset as usize + old.len as usize).min(self.text.len());
                self.text.drain(start..end);
                for e in &mut self.entries {
                    if old.offset < e.offset {
                        e.offset = e.offset.wrapping_sub(old.len);
                    }
                }
            }
            let at = self.text.len() as u32;
            self.text.extend_from_slice(text);
            let e = &mut self.entries[slot];
            e.offset = at;
            e.len = new_len;
            return true;
        }
        let start = old.offset as usize;
        if let Some(dst) = self.text.get_mut(start..start + text.len()) {
            dst.copy_from_slice(text);
        }
        true
    }

    /// Records one more use of `id`.
    pub fn add_ref(&mut self, id: u32) {
        if let Some(slot) = self.slot(id) {
            self.entries[slot].refs = self.entries[slot].refs.wrapping_add(1);
        }
    }

    /// Drops one use of `id`, as the original's release does: at no uses
    /// left the slot is zeroed (its text stays behind as a hole) and the
    /// free hint moves up to it if it is higher.
    pub fn release(&mut self, id: u32) {
        let Some(slot) = self.slot(id) else {
            return;
        };
        let e = &mut self.entries[slot];
        e.refs = e.refs.wrapping_sub(1);
        if e.refs == 0 {
            *e = Entry::default();
            if self.free_hint < slot {
                self.free_hint = slot;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A library file with the given `(refs, text)` entries and a trailing
    /// slot, as every shipped file has.
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
        table.extend([0xAB; ENTRY_LEN]);
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
        // The file writes back as it was read, trailing slot included.
        assert_eq!(lib.write(), b);
    }

    #[test]
    fn text_is_latin_1() {
        let mut t = MAGIC.to_le_bytes().to_vec();
        t.extend(32u32.to_le_bytes());
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
    fn the_allocator_scans_from_the_free_hint() {
        let mut lib = Library::parse(&library(&[(1, "Sword"), (1, "Maul"), (1, "Axe")])).unwrap();
        // Freeing slot 3 (index 2) raises the hint to 2; freeing slot 1
        // (index 0) leaves it there, so slot 1 is skipped.
        lib.release(3);
        lib.release(1);
        assert_eq!(lib.refs(1), 0);
        assert_eq!(lib.alloc(), 3);
        assert_eq!(lib.refs(3), 1);
        assert_eq!(lib.get(3), None);
        // Nothing free from the hint on: a new slot is appended.
        assert_eq!(lib.alloc(), 4);
        assert_eq!(lib.len(), 4);
        assert_eq!(lib.refs(1), 0);
    }

    #[test]
    fn setting_text_closes_the_hole_or_overwrites_in_place() {
        // Text area: "Sword" 0..5, "Two-handed Sword" 5..21, "Maul" 21..25.
        let mut lib = Library::parse(&library(&[
            (1, "Sword"),
            (1, "Two-handed Sword"),
            (1, "Maul"),
        ]))
        .unwrap();
        assert!(lib.set(2, b"Axe"));
        // The 16 bytes go, "Maul" moves to 5, "Axe" is appended at 9.
        assert_eq!(lib.text(), b"SwordMaulAxe");
        assert_eq!(lib.entries()[2].offset, 5);
        assert_eq!(lib.entries()[1].offset, 9);
        assert_eq!(lib.get(1).as_deref(), Some("Sword"));
        assert_eq!(lib.get(2).as_deref(), Some("Axe"));
        assert_eq!(lib.get(3).as_deref(), Some("Maul"));
        // Same length: in place.
        assert!(lib.set(1, b"Spear"));
        assert_eq!(lib.text(), b"SpearMaulAxe");
        // Empty text releases the slot and reports false.
        assert!(!lib.set(2, b""));
        assert_eq!(lib.get(2), None);
        assert_eq!(lib.refs(2), 0);
        // The released text stays as a hole.
        assert_eq!(lib.text(), b"SpearMaulAxe");
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
        assert_eq!(lib.entries()[0], Entry::default());
        // A slot without references wraps below zero instead of freeing, as
        // the original's 32-bit count does.
        lib.release(1);
        assert_eq!(lib.refs(1), u32::MAX);
        // Missing ids change nothing.
        lib.release(0);
        lib.release(9);
        lib.add_ref(0);
        assert_eq!(lib.len(), 1);
    }

    #[test]
    fn clearing_empties_the_slots_the_hint_and_the_text() {
        let mut lib = Library::parse(&library(&[(1, "Sword")])).unwrap();
        lib.set(1, b"Spear");
        lib.clear();
        assert!(lib.is_empty());
        assert_eq!(lib.text(), b"");
        assert_eq!(lib.get(1), None);
        assert_eq!(lib.write(), {
            let mut b = MAGIC.to_le_bytes().to_vec();
            b.extend(16u32.to_le_bytes());
            b
        });
    }

    #[test]
    fn short_tables_and_errors() {
        // The shipped empty library: magic and a one-slot table length, no
        // table bytes.
        let mut empty = MAGIC.to_le_bytes().to_vec();
        empty.extend(16u32.to_le_bytes());
        let lib = Library::parse(&empty).unwrap();
        assert!(lib.is_empty());
        assert_eq!(lib.write(), empty);
        // A table length below one slot counts −1 entries and writes 0.
        let mut none = MAGIC.to_le_bytes().to_vec();
        none.extend(0u32.to_le_bytes());
        assert_eq!(Library::parse(&none).unwrap().write(), none);
        assert!(matches!(Library::parse(&[]), Err(Error::Truncated { .. })));
        let good = library(&[(1, "Sword")]);
        let mut bad_magic = good.clone();
        bad_magic[0] ^= 0xFF;
        assert!(matches!(
            Library::parse(&bad_magic),
            Err(Error::BadMagic { .. })
        ));
        // A table that runs past the file.
        let mut truncated = good.clone();
        truncated.truncate(20);
        assert!(matches!(
            Library::parse(&truncated),
            Err(Error::Truncated { .. })
        ));
        // An entry past the text reads as nothing.
        let mut past = good.clone();
        past[8 + 12..8 + 16].copy_from_slice(&1000u32.to_le_bytes());
        let lib = Library::parse(&past).unwrap();
        assert_eq!(lib.get(1), None);
        // Growing the table forgets the trailing slot's bytes.
        let mut lib = Library::parse(&good).unwrap();
        assert_eq!(lib.add(b"Maul"), 2);
        let out = lib.write();
        assert_eq!(&out[8 + 32..8 + 48], &[0; 16]);
        assert_eq!(
            Library::parse(&out).unwrap().get(2).as_deref(),
            Some("Maul")
        );
    }

    /// The shipped libraries: `objstrings.rdb` and every level's own one
    /// parse and write back byte for byte.
    #[test]
    fn shipped_string_libraries() {
        let Some(dir) = exa_rpk::locate_install(None) else {
            return;
        };
        let game = exa_rpk::GameData::open(&dir).unwrap();
        let resource = game.pack(exa_rpk::PackKind::Resource).unwrap();
        let file = resource.get("objstrings.rdb").unwrap();
        let lib = Library::parse(file).unwrap();
        assert_eq!(lib.len(), 1124);
        assert_eq!(lib.write(), file);
        assert_eq!(lib.get(1).as_deref(), Some("Sword"));
        assert_eq!(lib.get(2).as_deref(), Some("Two-handed Sword"));
        assert_eq!(lib.get(88).as_deref(), Some("Breastplate"));
        assert_eq!(lib.get(89).as_deref(), Some("Plate Helm"));
        assert_eq!(lib.get(285).as_deref(), Some("Falchion"));
        assert_eq!(lib.get(170).as_deref(), Some("Torch"));
        assert_eq!(lib.get(182).as_deref(), Some("A simple torch."));
        assert!(lib.entries().iter().all(|e| e.unused == 0 && e.refs > 0));
        let mut levels = 0;
        for e in resource.entries() {
            let Ok(chunks) = crate::chunk::rfc_chunks(resource.bytes(e)) else {
                continue;
            };
            for c in chunks.flatten() {
                if c.tag != crate::rfc::LOCAL_DATABASE {
                    continue;
                }
                let mut pos = 0;
                while pos + 8 <= c.data.len() {
                    let tag = crate::read_u32(c.data, pos).unwrap();
                    let size = crate::read_u32(c.data, pos + 4).unwrap() as usize;
                    let body = &c.data[pos + 8..pos + 8 + size];
                    if tag == 0xDBCD {
                        let lib =
                            Library::parse(body).unwrap_or_else(|err| panic!("{}: {err}", e.name));
                        assert_eq!(lib.write(), body, "{}", e.name);
                        levels += 1;
                    }
                    pos += 8 + size;
                }
            }
        }
        assert_eq!(levels, 22);
    }
}
