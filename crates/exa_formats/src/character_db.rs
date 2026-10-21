//! The character databases: the local (`0x80000000`) and world
//! (`0x40000000`) stores that keep character records by id, parse each into
//! a shared object the first time it is looked up, count its holders, hand
//! out new ids, release, delete, re-serialise live characters and save.
//!
//! The on-disk form is the original's (`0xDBACCD00`): the magic, the byte
//! size of a table of 16-byte entries with one sentinel entry at the end
//! (`(count + 1) × 16`), then the entries (id, a word the loader does not
//! read, payload offset, payload length) and the payload bytes. A save with
//! no characters writes the magic and the table size only.
//!
//! Lookups are a binary search over the id-sorted table. A lookup whose
//! entry carries bytes and has no live object parses them into a
//! [`SharedCharacter`] and stores it; later lookups return the same object
//! and count one more holder. A live object's id is stamped with its
//! database's tag. `flush` rewrites the payload from every live object
//! (serialising it) and from the stored bytes of the others; `save` writes
//! the table and payload after a flush.
//!
//! Evidence: the original's two character databases and their lookup, add,
//! release, delete, payload, load, flush and save routines.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};

use crate::characters::{CharacterRecord, ID_MASK};
use crate::{Error, Result};

/// The local database's tag.
pub const LOCAL_TAG: u32 = 0x8000_0000;
/// The world database's tag.
pub const WORLD_TAG: u32 = 0x4000_0000;
/// The save magic.
pub const SAVE_MAGIC: u32 = 0xDBAC_CD00;
/// One table entry's size: id, an unread word, the payload offset and its
/// length.
pub const ENTRY: usize = 16;
/// The first id a new character takes.
pub const FIRST_ID: u32 = 1;

/// A character object a database keeps live: its record, shared by its
/// holders, and the count of lookups.
#[derive(Debug)]
pub struct SharedCharacter {
    record: Mutex<CharacterRecord>,
    holders: AtomicUsize,
}

impl SharedCharacter {
    fn new(record: CharacterRecord) -> Self {
        Self {
            record: Mutex::new(record),
            holders: AtomicUsize::new(1),
        }
    }

    /// The record. A poisoned lock still yields the record: the database
    /// never leaves it half-written.
    pub fn record(&self) -> MutexGuard<'_, CharacterRecord> {
        self.record
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// The record's id, tagged with its database.
    pub fn id(&self) -> u32 {
        self.record().id
    }

    /// How many lookups hold this object.
    pub fn holders(&self) -> usize {
        self.holders.load(Ordering::Relaxed)
    }

    fn hold(&self) -> usize {
        if self.holders.load(Ordering::Relaxed) == 0 {
            self.holders.store(1, Ordering::Relaxed);
            1
        } else {
            self.holders.fetch_add(1, Ordering::Relaxed) + 1
        }
    }
}

/// One table entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Entry {
    id: u32,
    offset: usize,
    len: usize,
}

/// A character database.
#[derive(Debug)]
pub struct CharacterDb {
    tag: u32,
    entries: Vec<Entry>,
    payload: Vec<u8>,
    live: Vec<Option<Arc<SharedCharacter>>>,
}

impl CharacterDb {
    /// A new, empty database whose live objects carry `tag`.
    pub fn new(tag: u32) -> Self {
        Self {
            tag,
            entries: Vec::new(),
            payload: Vec::new(),
            live: Vec::new(),
        }
    }

    /// The database's tag.
    pub fn tag(&self) -> u32 {
        self.tag
    }

    /// How many records the table holds.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Every id in table order.
    pub fn ids(&self) -> impl Iterator<Item = u32> + '_ {
        self.entries.iter().map(|e| e.id)
    }

    /// The table index of `id`, by binary search.
    fn index(&self, id: u32) -> Option<usize> {
        self.entries.binary_search_by_key(&id, |e| e.id).ok()
    }

    /// The entry with `id`, if the table has it.
    pub fn contains(&self, id: u32) -> bool {
        self.index(id).is_some()
    }

    /// The character with `id`: the live object, counting one more holder;
    /// or its stored bytes parsed into a new object, counting its first.
    /// Entries with no bytes, missing ids and records that do not parse
    /// give nothing.
    pub fn get(&mut self, id: u32) -> Option<Arc<SharedCharacter>> {
        let index = self.index(id)?;
        if let Some(live) = &self.live[index] {
            live.hold();
            return Some(live.clone());
        }
        let entry = self.entries[index];
        if entry.len == 0 {
            return None;
        }
        let bytes = self
            .payload
            .get(entry.offset..entry.offset.checked_add(entry.len)?)?;
        let record = CharacterRecord::parse(id | self.tag, bytes).ok()?;
        let live = Arc::new(SharedCharacter::new(record));
        self.live[index] = Some(live.clone());
        Some(live)
    }

    /// The record's id without the database tag.
    pub fn untagged(id: u32) -> u32 {
        id & ID_MASK
    }

    /// The first id from [`FIRST_ID`] the table does not hold, as the
    /// original's allocator walks the table and steps over a collision.
    pub fn free_id(&self) -> u32 {
        let mut id = FIRST_ID;
        for entry in &self.entries {
            if id < entry.id {
                break;
            }
            if id == entry.id {
                id += 1;
            }
        }
        id
    }

    /// Adds `record` under `id`, or under the first free id when `id` is 0.
    /// The record's id is stamped with the tag. Returns the id, or nothing
    /// when that id is taken.
    pub fn add(&mut self, id: u32, record: CharacterRecord) -> Option<u32> {
        let id = if id == 0 { self.free_id() } else { id };
        let index = self.entries.partition_point(|e| e.id < id);
        if self.entries.get(index).is_some_and(|e| e.id == id) {
            return None;
        }
        let mut record = record;
        record.id = id | self.tag;
        let bytes = record.to_bytes().ok()?;
        let offset = self.payload.len();
        let len = bytes.len();
        self.payload.extend_from_slice(&bytes);
        self.entries.insert(index, Entry { id, offset, len });
        let live = Arc::new(SharedCharacter::new(record));
        // The added object is the caller's; no lookup holds it yet.
        live.holders.store(0, Ordering::Relaxed);
        self.live.insert(index, Some(live));
        Some(id)
    }

    /// Releases the character `id`: its live object is dropped (the stored
    /// bytes remain, so a later lookup parses a fresh one) and the object's
    /// record id is cleared. Returns whether a live object was released.
    pub fn release(&mut self, id: u32) -> bool {
        let Some(index) = self.index(id) else {
            return false;
        };
        let Some(live) = self.live[index].take() else {
            return false;
        };
        live.record().id = 0;
        true
    }

    /// Deletes the character `id`: its table entry goes and its bytes are
    /// removed. Returns whether it was there.
    pub fn delete(&mut self, id: u32) -> bool {
        let Some(index) = self.index(id) else {
            return false;
        };
        let entry = self.entries.remove(index);
        self.live.remove(index);
        self.remove_bytes(entry.offset, entry.len);
        true
    }

    /// Replaces the stored bytes of character `id` (the original's set
    /// payload), keeping the live object alone. Returns whether it was
    /// there.
    pub fn set_payload(&mut self, id: u32, bytes: &[u8]) -> bool {
        let Some(index) = self.index(id) else {
            return false;
        };
        let entry = self.entries[index];
        self.remove_bytes(entry.offset, entry.len);
        let offset = self.payload.len();
        self.payload.extend_from_slice(bytes);
        self.entries[index] = Entry {
            id,
            offset,
            len: bytes.len(),
        };
        true
    }

    /// Removes `len` bytes at `offset`, pulling every later entry's offset
    /// back by the same amount.
    fn remove_bytes(&mut self, offset: usize, len: usize) {
        let end = offset.saturating_add(len).min(self.payload.len());
        let offset = offset.min(end);
        self.payload.drain(offset..end);
        let removed = end - offset;
        for entry in &mut self.entries {
            if entry.offset > offset {
                entry.offset = entry.offset.saturating_sub(removed);
            }
        }
    }

    /// Every live character's record is serialised into the payload again
    /// (the original's flush); stores that are not live keep their bytes.
    pub fn flush(&mut self) -> Result<()> {
        let mut payload = Vec::new();
        for index in 0..self.entries.len() {
            let id = self.entries[index].id;
            let bytes = match &self.live[index] {
                Some(live) => {
                    let record = live.record();
                    record.to_bytes()?
                }
                None => self.stored(index).unwrap_or_default().to_vec(),
            };
            let offset = payload.len();
            let len = bytes.len();
            payload.extend_from_slice(&bytes);
            self.entries[index] = Entry { id, offset, len };
        }
        self.payload = payload;
        Ok(())
    }

    /// The stored bytes of table index `index`.
    fn stored(&self, index: usize) -> Option<&[u8]> {
        let entry = self.entries.get(index)?;
        self.payload
            .get(entry.offset..entry.offset.checked_add(entry.len)?)
    }

    /// The database in its save form: the magic, the table size, the
    /// entries and their payload. The live characters are flushed first.
    pub fn save(&mut self) -> Result<Vec<u8>> {
        self.flush()?;
        let table_bytes = (self.entries.len() + 1) * ENTRY;
        let mut out = Vec::with_capacity(8 + table_bytes + self.payload.len());
        out.extend_from_slice(&SAVE_MAGIC.to_le_bytes());
        out.extend_from_slice(&(table_bytes as u32).to_le_bytes());
        if !self.entries.is_empty() {
            for entry in &self.entries {
                out.extend_from_slice(&entry.id.to_le_bytes());
                out.extend_from_slice(&0u32.to_le_bytes());
                out.extend_from_slice(&(entry.offset as u32).to_le_bytes());
                out.extend_from_slice(&(entry.len as u32).to_le_bytes());
            }
            // The sentinel entry the table's size counts.
            out.extend_from_slice(&[0u8; ENTRY]);
            out.extend_from_slice(&self.payload);
        }
        Ok(out)
    }

    /// Reads a save into a database with `tag`. The magic, the table size,
    /// the id order and every payload range are checked; a header-only
    /// empty save is accepted.
    pub fn load(tag: u32, bytes: &[u8]) -> Result<Self> {
        let magic = crate::read_u32(bytes, 0)?;
        if magic != SAVE_MAGIC {
            return Err(Error::BadMagic {
                found: magic,
                expected: SAVE_MAGIC,
            });
        }
        let table_bytes = crate::read_u32(bytes, 4)? as usize;
        if table_bytes < ENTRY || table_bytes % ENTRY != 0 {
            return Err(Error::Malformed(format!(
                "character database table of {table_bytes} bytes"
            )));
        }
        let count = table_bytes / ENTRY - 1;
        let mut db = Self::new(tag);
        if bytes.len() == 8 {
            // The empty save writes the header alone.
            if count != 0 {
                return Err(Error::Truncated {
                    offset: 8,
                    needed: table_bytes,
                    len: bytes.len(),
                });
            }
            return Ok(db);
        }
        let table_end = 8usize
            .checked_add(table_bytes)
            .filter(|&end| end <= bytes.len())
            .ok_or(Error::Truncated {
                offset: 8,
                needed: table_bytes,
                len: bytes.len(),
            })?;
        let payload = &bytes[table_end..];
        let mut previous = None;
        for i in 0..count {
            let at = 8 + i * ENTRY;
            let id = crate::read_u32(bytes, at)?;
            let offset = crate::read_u32(bytes, at + 8)? as usize;
            let len = crate::read_u32(bytes, at + 12)? as usize;
            if previous.is_some_and(|p| id <= p) {
                return Err(Error::Malformed("character ids out of order".into()));
            }
            previous = Some(id);
            if offset.saturating_add(len) > payload.len() {
                return Err(Error::Malformed(format!(
                    "character {id} payload {offset}+{len} past {}",
                    payload.len()
                )));
            }
            db.entries.push(Entry { id, offset, len });
        }
        db.payload = payload.to_vec();
        db.live = vec![None; count];
        Ok(db)
    }

    /// Reads an RDB of character records (a level's or a save's
    /// `#worldchrdb` chunk) into a database with `tag`, storing each
    /// record's bytes.
    pub fn from_rdb(tag: u32, bytes: &[u8]) -> Result<Self> {
        let db = crate::rdb::Database::parse(bytes)?;
        let mut out = Self::new(tag);
        for (i, r) in db.records.iter().enumerate() {
            let bytes = db.payload(r);
            if bytes.is_empty() {
                continue;
            }
            let offset = out.payload.len();
            out.payload.extend_from_slice(bytes);
            out.entries.push(Entry {
                id: r.id,
                offset,
                len: bytes.len(),
            });
            out.live.push(None);
            let _ = i;
        }
        out.entries.sort_by_key(|e| e.id);
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(name: &str) -> CharacterRecord {
        let mut r = CharacterRecord::new();
        r.name = name.into();
        r
    }

    /// Two records save and load back with their ids, names and payloads,
    /// the magic and the sentinel entry.
    #[test]
    fn a_database_saves_and_loads() {
        let mut db = CharacterDb::new(LOCAL_TAG);
        assert_eq!(db.add(0, record("Ada")), Some(1));
        assert_eq!(db.add(0, record("Bran")), Some(2));
        assert_eq!(db.len(), 2);
        // The live record carries the tag; the untagged id is the table id.
        let ada = db.get(1).unwrap();
        assert_eq!(ada.id(), LOCAL_TAG | 1);
        assert_eq!(CharacterDb::untagged(ada.id()), 1);
        assert_eq!(ada.record().name, "Ada");
        let bytes = db.save().unwrap();
        assert_eq!(&bytes[..4], &SAVE_MAGIC.to_le_bytes());
        // (2 + 1) entries of 16 bytes after the 8-byte header.
        assert_eq!(&bytes[4..8], &48u32.to_le_bytes());
        assert_eq!(bytes.len(), 8 + 48 + db.payload.len());

        let mut loaded = CharacterDb::load(LOCAL_TAG, &bytes).unwrap();
        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded.ids().collect::<Vec<_>>(), [1, 2]);
        assert_eq!(loaded.get(1).unwrap().record().name, "Ada");
        assert_eq!(loaded.get(2).unwrap().record().name, "Bran");
        assert!(loaded.get(3).is_none());
    }

    /// A lookup parses lazily and counts its holders; a release drops the
    /// object and the next lookup parses a fresh one.
    #[test]
    fn lookups_are_lazy_and_count_holders() {
        let mut db = CharacterDb::new(WORLD_TAG);
        db.add(7, record("Cy")).unwrap();
        let first = db.get(7).unwrap();
        assert_eq!(first.holders(), 1);
        let second = db.get(7).unwrap();
        assert_eq!(second.holders(), 2);
        assert!(Arc::ptr_eq(&first, &second));
        assert!(db.release(7));
        assert_eq!(first.holders(), 2);
        assert!(!db.release(7));
        // Released: the live object id is cleared, but a new lookup parses
        // the stored bytes again.
        assert_eq!(first.record().id, 0);
        let again = db.get(7).unwrap();
        assert_eq!(again.holders(), 1);
        assert_eq!(again.record().name, "Cy");
    }

    /// New ids step over the table's collisions and gaps.
    #[test]
    fn new_ids_skip_taken_numbers() {
        let mut db = CharacterDb::new(LOCAL_TAG);
        assert_eq!(db.free_id(), 1);
        db.add(0, record("a")).unwrap();
        db.add(0, record("b")).unwrap();
        assert_eq!(db.free_id(), 3);
        db.add(0, record("c")).unwrap();
        assert_eq!(db.ids().collect::<Vec<_>>(), [1, 2, 3]);
        assert_eq!(db.free_id(), 4);
        assert_eq!(db.add(2, record("d")), None);
    }

    /// Setting a payload keeps the live record; deleting removes the entry
    /// and its bytes; a later flush serialises the live changes.
    #[test]
    fn payloads_flush_and_delete() {
        let mut db = CharacterDb::new(LOCAL_TAG);
        db.add(0, record("old")).unwrap();
        db.add(0, record("keep")).unwrap();
        assert!(db.set_payload(1, &record("stored").to_bytes().unwrap()));
        // The live object still reads old until the DB is released; the
        // stored bytes then read back as set.
        assert_eq!(db.get(1).unwrap().record().name, "old");
        db.release(1);
        assert_eq!(db.get(1).unwrap().record().name, "stored");
        assert!(db.delete(1));
        assert!(!db.contains(1));
        assert_eq!(db.get(2).unwrap().record().name, "keep");
        // A live change reaches the stored form on flush.
        db.get(2).unwrap().record().name = "renamed".into();
        db.flush().unwrap();
        db.release(2);
        assert_eq!(db.get(2).unwrap().record().name, "renamed");
    }

    /// The total parser refuses a bad magic, a bad table and a payload past
    /// its end, and handles the empty save's header alone.
    #[test]
    fn malformed_saves_are_errors() {
        let mut db = CharacterDb::new(LOCAL_TAG);
        assert!(db.save().is_ok());
        assert!(db.save().unwrap().len() == 8);
        assert!(CharacterDb::load(LOCAL_TAG, &db.save().unwrap()).is_ok());
        let mut bad = db.save().unwrap();
        bad[1] = 0;
        assert!(CharacterDb::load(LOCAL_TAG, &bad).is_err());
        let mut bad = db.save().unwrap();
        bad[4] = 15;
        assert!(CharacterDb::load(LOCAL_TAG, &bad).is_err());
        db.add(0, record("x")).unwrap();
        let good = db.save().unwrap();
        for n in 0..good.len() {
            let _ = CharacterDb::load(LOCAL_TAG, &good[..n]);
        }
        let mut bad = good.clone();
        // The first entry's payload length runs past the payload.
        bad[8 + ENTRY - 4..8 + ENTRY].copy_from_slice(&0xFFFFu32.to_le_bytes());
        assert!(CharacterDb::load(LOCAL_TAG, &bad).is_err());
    }
}
