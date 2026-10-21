//! The generic id-keyed `.rdb` database object and its sorted id→offset
//! index, as the original's `TUIDManager` and `TUIDDataSetC` work.
//!
//! The database is the mutable counterpart of [`crate::rdb::Database`]:
//! a count, a table of 16-byte records, the payload blob and the magic the
//! file is expected to carry. It supports the operations the original's
//! manager has: binary search by id, a record view, and replacing a record's
//! payload (in place at the same length, else the old bytes close up and the
//! new ones go to the end of the blob). Loading checks the magic against the
//! expected one and, on a mismatch, gives the upgrade hook a chance before
//! clearing the table.
//!
//! The index is the original's separate sorted array of 8-byte entries: an
//! id and an offset into its own blob. Each payload starts with a `u32`
//! length; the entries stay sorted by id, a delete clears the payload and
//! closes the entry up, and a payload of a new length moves the bytes after
//! it and fixes the offsets.
//!
//! Layout evidence: `docs/formats/OTHER.md` (Object database, UID manager)
//! and `docs/formats/RFC.md` (locales).

use crate::{Error, Result, read_u32};

/// A database record: the fields of one 16-byte table entry.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct UidRecord {
    pub id: u32,
    pub flags: u32,
    /// Where the payload starts in the blob.
    pub offset: u32,
    /// Its length; 0 for an empty record.
    pub size: u32,
}

impl UidRecord {
    fn read(b: &[u8]) -> Self {
        let word = |o: usize| u32::from_le_bytes(b[o..o + 4].try_into().expect("4 bytes"));
        Self {
            id: word(0),
            flags: word(4),
            offset: word(8),
            size: word(12),
        }
    }

    fn write(&self, out: &mut Vec<u8>) {
        for word in [self.id, self.flags, self.offset, self.size] {
            out.extend_from_slice(&word.to_le_bytes());
        }
    }
}

/// The upgrade hook a database may carry: it takes the manager after a file
/// with another magic has been read into it and reports whether the layout
/// was understood.
pub type Upgrade = fn(&mut UidDatabase) -> bool;

/// A mutable id-keyed database.
#[derive(Debug, Clone, Default)]
pub struct UidDatabase {
    records: Vec<UidRecord>,
    blob: Vec<u8>,
    /// The magic a loaded file must carry.
    pub expected_magic: u32,
    /// What to try when a file's magic differs (the original's obsolete
    /// format converters).
    pub upgrade: Option<Upgrade>,
}

impl UidDatabase {
    /// An empty database expecting `magic`.
    pub fn new(expected_magic: u32) -> Self {
        Self {
            expected_magic,
            ..Self::default()
        }
    }

    /// Reads a database as the original's loader does: `table_len / 16 − 1`
    /// records (the table's last slot is not a record), and the rest of the
    /// file as the blob. A magic other than [`Self::expected_magic`] runs
    /// the upgrade hook; if it is absent or fails, the table is cleared as
    /// the original does after its "Invalid or obsolete database format."
    /// message.
    pub fn load(&mut self, bytes: &[u8]) -> Result<()> {
        let magic = read_u32(bytes, 0)?;
        let table_len = read_u32(bytes, 4)? as usize;
        if !table_len.is_multiple_of(16) {
            return Err(Error::Malformed(format!(
                "rdb table length {table_len} is not a multiple of 16"
            )));
        }
        let table_end = 8usize
            .checked_add(table_len)
            .filter(|&end| end <= bytes.len())
            .ok_or(Error::Truncated {
                offset: 8,
                needed: table_len,
                len: bytes.len(),
            })?;
        let count = (table_len >> 4).saturating_sub(1);
        self.records = (0..count)
            .map(|i| UidRecord::read(&bytes[8 + i * 16..8 + (i + 1) * 16]))
            .collect();
        self.blob = bytes[table_end..].to_vec();
        if magic != self.expected_magic {
            let upgraded = self.upgrade.is_some_and(|upgrade| upgrade(self));
            if !upgraded {
                // The original logs "Invalid or obsolete database format."
                // and clears the record count, keeping the blob it read.
                self.records.clear();
            }
        }
        Ok(())
    }

    /// The records, sorted by id as the file stores them.
    pub fn records(&self) -> &[UidRecord] {
        &self.records
    }

    /// The payload blob.
    pub fn blob(&self) -> &[u8] {
        &self.blob
    }

    /// The number of records.
    pub fn len(&self) -> usize {
        self.records.len()
    }

    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    /// The sorted position of `id`, found by binary search over the ids.
    pub fn find(&self, id: u32) -> Option<usize> {
        let mut low = 0usize;
        let mut high = self.records.len().checked_sub(1)?;
        loop {
            if high < low {
                return None;
            }
            let mid = low + (high - low) / 2;
            match id.cmp(&self.records[mid].id) {
                std::cmp::Ordering::Less => {
                    high = mid.checked_sub(1)?;
                }
                std::cmp::Ordering::Greater => low = mid + 1,
                std::cmp::Ordering::Equal => return Some(mid),
            }
        }
    }

    /// The record at `index`.
    pub fn record(&self, index: usize) -> Option<&UidRecord> {
        self.records.get(index)
    }

    /// The payload of the record at `index`, or `None` for an empty record
    /// or one whose bytes lie outside the blob.
    pub fn payload(&self, index: usize) -> Option<&[u8]> {
        let record = self.records.get(index)?;
        if record.size == 0 {
            return None;
        }
        let start = record.offset as usize;
        self.blob
            .get(start..start.checked_add(record.size as usize)?)
    }

    /// The payload of `id`.
    pub fn payload_of(&self, id: u32) -> Option<&[u8]> {
        self.payload(self.find(id)?)
    }

    /// Replaces the payload at `index`, as the original's routine does: the
    /// same length overwrites in place; another length closes the old bytes'
    /// hole (the offsets of later payloads move down) and appends the new
    /// ones. An empty payload leaves the record empty.
    pub fn replace(&mut self, index: usize, payload: &[u8]) {
        let Some(record) = self.records.get(index).copied() else {
            return;
        };
        let new_len = payload.len() as u32;
        if new_len != record.size {
            if record.size != 0 {
                let start = (record.offset as usize).min(self.blob.len());
                let end = (record.offset as usize + record.size as usize).min(self.blob.len());
                self.blob.drain(start..end);
                for r in &mut self.records {
                    if record.offset < r.offset {
                        r.offset = r.offset.wrapping_sub(record.size);
                    }
                }
            }
            let at = self.blob.len() as u32;
            self.blob.extend_from_slice(payload);
            let r = &mut self.records[index];
            r.offset = at;
            r.size = new_len;
            return;
        }
        let start = record.offset as usize;
        if let Some(dst) = self.blob.get_mut(start..start + payload.len()) {
            dst.copy_from_slice(payload);
        }
    }

    /// Replaces the payload of `id`; `None` when there is no such record.
    pub fn replace_of(&mut self, id: u32, payload: &[u8]) -> Option<()> {
        let index = self.find(id)?;
        self.replace(index, payload);
        Some(())
    }

    /// The database's bytes: magic, the table with its extra trailing slot
    /// (zeroed), and the blob. The original's manager does not write itself
    /// back; this is the layout of a file it could have read.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = self.expected_magic.to_le_bytes().to_vec();
        out.extend((((self.records.len() + 1) * 16) as u32).to_le_bytes());
        for r in &self.records {
            r.write(&mut out);
        }
        out.extend([0; 16]);
        out.extend_from_slice(&self.blob);
        out
    }
}

/// One entry of the sorted id→offset index: the id and the payload's
/// offset in the index's blob, or `-1` for an entry without a payload.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct IndexEntry {
    pub id: i32,
    pub offset: i32,
}

/// The original's separate sorted id→offset index and its blob. Payloads
/// start with a `u32` length, so the entry's offset points at that word.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct UidIndex {
    entries: Vec<IndexEntry>,
    data: Vec<u8>,
}

impl UidIndex {
    pub fn new() -> Self {
        Self::default()
    }

    /// The entries, sorted by id.
    pub fn entries(&self) -> &[IndexEntry] {
        &self.entries
    }

    /// The blob.
    pub fn data(&self) -> &[u8] {
        &self.data
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Inserts `id` at its sorted place, with no payload, as the original's
    /// insertion does. `found` is the position of an id already present,
    /// which is not inserted again.
    pub fn insert(&mut self, id: i32) -> std::result::Result<usize, usize> {
        match self.entries.binary_search_by_key(&id, |e| e.id) {
            Ok(pos) => Err(pos),
            Err(pos) => {
                self.entries.insert(pos, IndexEntry { id, offset: -1 });
                Ok(pos)
            }
        }
    }

    /// The sorted position of `id`.
    pub fn find(&self, id: i32) -> Option<usize> {
        self.entries.binary_search_by_key(&id, |e| e.id).ok()
    }

    /// The bytes at the entry's offset (its leading length word included),
    /// or `None` for an entry without a payload.
    pub fn bytes(&self, index: usize) -> Option<&[u8]> {
        let offset = self.entries.get(index)?.offset;
        if offset < 0 {
            return None;
        }
        let start = offset as usize;
        self.data.get(start..)
    }

    /// The `u32` length at the entry's offset.
    pub fn payload_len(&self, index: usize) -> Option<u32> {
        read_u32(self.bytes(index)?, 0).ok()
    }

    /// The payload after the entry's leading length word.
    pub fn payload(&self, index: usize) -> Option<&[u8]> {
        let bytes = self.bytes(index)?;
        let len = read_u32(bytes, 0).ok()? as usize;
        bytes.get(4..4 + len)
    }

    /// Deletes the entry at `index`: its payload is cleared, the entry is
    /// removed and the entries after it move down.
    pub fn delete(&mut self, index: usize) {
        if index >= self.entries.len() {
            return;
        }
        self.set_payload(index, &[]);
        self.entries.remove(index);
    }

    /// Stores `payload` for the entry at `index`, as the original does: a
    /// new length moves the bytes after the old payload, fixes every later
    /// offset, appends the payload at the blob's end and writes its length;
    /// the same length (and any non-empty payload) overwrites in place.
    pub fn set_payload(&mut self, index: usize, payload: &[u8]) {
        let Some(entry) = self.entries.get(index).copied() else {
            return;
        };
        let old_len = if entry.offset < 0 {
            0
        } else {
            let at = entry.offset as usize;
            self.data
                .get(at..at + 4)
                .map(|b| u32::from_le_bytes(b.try_into().expect("4 bytes")) as usize)
                .unwrap_or(0)
        };
        let new_len = payload.len();
        if old_len != new_len {
            if old_len != 0 {
                let start = (entry.offset as usize).min(self.data.len());
                let end = (start + old_len + 4).min(self.data.len());
                self.data.drain(start..end);
                for e in &mut self.entries {
                    if entry.offset >= 0 && entry.offset < e.offset {
                        e.offset = e.offset.wrapping_sub((old_len + 4) as i32);
                    }
                }
            }
            let at = self.data.len() as i32;
            self.data.extend_from_slice(&(new_len as u32).to_le_bytes());
            self.data.extend_from_slice(payload);
            self.entries[index].offset = at;
            return;
        }
        if old_len != 0 {
            let start = entry.offset as usize + 4;
            if let Some(dst) = self.data.get_mut(start..start + new_len) {
                dst.copy_from_slice(payload);
            }
        } else if !payload.is_empty() {
            // A same-length empty payload writes nothing.
        }
    }

    /// Sets the blob's data wholesale, keeping the entries and their
    /// offsets (the original's "set data").
    pub fn set_data(&mut self, data: Vec<u8>) {
        self.data = data;
    }

    /// Takes the blob out.
    pub fn into_data(self) -> Vec<u8> {
        self.data
    }
}

/// The magic of `locales.rdb`.
pub const LOCALE_MAGIC: u32 = 0xDBAE_0D01;

#[cfg(test)]
mod tests {
    use super::*;

    fn database(records: &[(u32, &[u8])]) -> Vec<u8> {
        let mut blob = Vec::new();
        let mut table = Vec::new();
        for &(id, payload) in records {
            table.extend(id.to_le_bytes());
            table.extend(0u32.to_le_bytes());
            table.extend((blob.len() as u32).to_le_bytes());
            table.extend((payload.len() as u32).to_le_bytes());
            blob.extend_from_slice(payload);
        }
        table.extend([0xAA; 16]);
        let mut b = LOCALE_MAGIC.to_le_bytes().to_vec();
        b.extend((table.len() as u32).to_le_bytes());
        b.extend_from_slice(&table);
        b.extend_from_slice(&blob);
        b
    }

    #[test]
    fn loads_one_record_per_table_slot_less_the_last_and_finds_by_id() {
        let file = database(&[(3, b"three"), (10, b"ten"), (42, b"forty-two")]);
        let mut db = UidDatabase::new(LOCALE_MAGIC);
        db.load(&file).unwrap();
        assert_eq!(db.len(), 3);
        assert_eq!(db.find(10), Some(1));
        assert_eq!(db.find(11), None);
        assert_eq!(db.find(3), Some(0));
        assert_eq!(db.find(42), Some(2));
        assert_eq!(db.payload_of(10), Some(&b"ten"[..]));
        // The extra table slot is not kept by the in-memory form, so a
        // write is canonical: parsing it back gives the same records.
        let mut again = UidDatabase::new(LOCALE_MAGIC);
        again.load(&db.to_bytes()).unwrap();
        assert_eq!(again.records(), db.records());
        assert_eq!(again.blob(), db.blob());
    }

    #[test]
    fn a_wrong_magic_clears_the_table_unless_the_upgrade_takes_it() {
        let mut file = database(&[(1, b"x")]);
        file[0] = 0x08;
        let mut db = UidDatabase::new(LOCALE_MAGIC);
        db.load(&file).unwrap();
        assert!(db.is_empty());
        assert_eq!(db.blob(), b"x");

        fn upgrade(db: &mut UidDatabase) -> bool {
            db.records.clear();
            true
        }
        let mut db = UidDatabase::new(LOCALE_MAGIC);
        db.upgrade = Some(upgrade);
        db.load(&file).unwrap();
        assert!(db.is_empty());
    }

    #[test]
    fn replacing_a_payload_closes_or_overwrites_like_the_library() {
        let mut db = UidDatabase::new(LOCALE_MAGIC);
        db.load(&database(&[(1, b"aa"), (2, b"bbbb"), (3, b"cc")]))
            .unwrap();
        // Same length overwrites in place.
        db.replace_of(2, b"BBBB").unwrap();
        assert_eq!(db.payload_of(2), Some(&b"BBBB"[..]));
        assert_eq!(db.blob(), b"aaBBBBcc");
        // A shorter payload closes the hole and moves later offsets down.
        db.replace_of(2, b"b").unwrap();
        assert_eq!(db.blob(), b"aaccb");
        assert_eq!(db.payload_of(3), Some(&b"cc"[..]));
        assert_eq!(db.payload_of(2), Some(&b"b"[..]));
        // A longer one appends at the end.
        db.replace_of(1, b"long").unwrap();
        assert_eq!(db.blob(), b"ccblong");
        assert_eq!(db.payload_of(1), Some(&b"long"[..]));
        assert_eq!(db.payload_of(2), Some(&b"b"[..]));
        assert_eq!(db.payload_of(3), Some(&b"cc"[..]));
        // Empty payload leaves the record empty and missing.
        db.replace_of(3, b"").unwrap();
        assert_eq!(db.payload_of(3), None);
        assert_eq!(db.records()[2].size, 0);
    }

    #[test]
    fn missing_and_short_payloads_are_none_and_bad_files_are_errors() {
        let mut db = UidDatabase::new(LOCALE_MAGIC);
        assert!(matches!(db.load(&[]), Err(Error::Truncated { .. })));
        let mut file = database(&[(1, b"x")]);
        file[4..8].copy_from_slice(&17u32.to_le_bytes());
        assert!(matches!(db.load(&file), Err(Error::Malformed(_))));
        let mut file = database(&[(1, b"x")]);
        file.truncate(20);
        assert!(matches!(db.load(&file), Err(Error::Truncated { .. })));
        // A record pointing outside the blob reads as nothing.
        let mut db = UidDatabase::new(LOCALE_MAGIC);
        db.load(&database(&[(1, b"x")])).unwrap();
        db.records[0].offset = 1000;
        assert_eq!(db.payload_of(1), None);
        assert_eq!(db.payload(9), None);
        db.replace(9, b"y");
        assert_eq!(db.len(), 1);
    }

    #[test]
    fn the_sorted_index_inserts_finds_and_deletes() {
        let mut index = UidIndex::new();
        assert_eq!(index.insert(20), Ok(0));
        assert_eq!(index.insert(5), Ok(0));
        assert_eq!(index.insert(12), Ok(1));
        assert_eq!(index.insert(12), Err(1));
        assert_eq!(index.find(12), Some(1));
        assert_eq!(index.find(9), None);
        index.set_payload(1, b"twelve");
        assert_eq!(index.payload(1), Some(&b"twelve"[..]));
        assert_eq!(index.payload_len(1), Some(6));
        assert_eq!(index.payload(0), None);
        // A longer payload appends and fixes later offsets.
        index.set_payload(0, b"twenty");
        assert_eq!(index.payload(0), Some(&b"twenty"[..]));
        assert_eq!(index.payload(1), Some(&b"twelve"[..]));
        assert_eq!(index.payload(2), None);
        // The same length overwrites in place.
        index.set_payload(1, b"TWELVE");
        assert_eq!(index.payload(1), Some(&b"TWELVE"[..]));
        // Deleting clears the payload and closes the entry up.
        index.delete(1);
        assert_eq!(
            index.entries().iter().map(|e| e.id).collect::<Vec<_>>(),
            [5, 20]
        );
        assert_eq!(index.payload(0), Some(&b"twenty"[..]));
        assert_eq!(index.payload(1), None);
        index.delete(9);
        assert_eq!(index.len(), 2);
        assert!(UidIndex::new().insert(1).is_ok());
    }
}
