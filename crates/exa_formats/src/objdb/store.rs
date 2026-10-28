//! One tier of the object database as the original holds it: an id-sorted
//! table of `(id, flags, offset, size)` entries over one payload blob.
//! Layout and evidence: `docs/formats/OTHER.md` (Object database, Framing).
//!
//! The routines are ports of the original's: the loader keeps the table
//! and blob as the file has them (it does not sort or compact), the binary
//! search and the sorted insertion compare ids by 32-bit difference, a
//! payload of a new size is moved to the end of the blob after closing the
//! hole of the old one, and the writer saves the table (with the slot after
//! its last entry) and the blob as held. A loaded database therefore
//! writes back byte for byte.

use super::Record;
use super::record::u32_at;
use crate::{Error, Result};

const HEADER_LEN: usize = 8;
const ENTRY_LEN: usize = 16;

/// One table entry.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct TableEntry {
    pub id: u32,
    pub flags: u32,
    /// Where the payload starts in the blob.
    pub offset: u32,
    /// The payload's length; a record of size 0 has no view.
    pub size: u32,
}

impl TableEntry {
    fn read(b: &[u8]) -> Self {
        Self {
            id: u32_at(b, 0),
            flags: u32_at(b, 4),
            offset: u32_at(b, 8),
            size: u32_at(b, 12),
        }
    }

    fn bytes(self) -> [u8; ENTRY_LEN] {
        let mut out = [0; ENTRY_LEN];
        for (i, w) in [self.id, self.flags, self.offset, self.size]
            .into_iter()
            .enumerate()
        {
            out[i * 4..i * 4 + 4].copy_from_slice(&w.to_le_bytes());
        }
        out
    }
}

/// The records of one tier: the table, the slot after it and the blob.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Store {
    entries: Vec<TableEntry>,
    /// The table slot after the last entry, which the original's writer
    /// saves too: what the file held there (a stale entry left by a
    /// deletion, or heap bytes), what a deletion leaves there, and zero
    /// after the table grew (where the original writes whatever lies past
    /// its allocation).
    tail: [u8; ENTRY_LEN],
    /// A table shorter than one slot: the original counts −1 records and
    /// writes a table length of 0 back.
    negative: bool,
    blob: Vec<u8>,
}

impl Store {
    /// Reads a database as the original's loader does: `table_len / 16 −
    /// 1` records; with fewer than one nothing past the header is read
    /// (the writer saves an empty database as the header alone), otherwise
    /// the blob is everything after the table. The table is neither sorted
    /// nor checked against the blob: an entry outside it has no view. A
    /// database of an older layout (`0xDBAC0D08`, `0xDBAC0D07`,
    /// `0xDBAC0D06`) is then converted by the original's upgrade chain; any
    /// other magic is refused. The oldest layout draws each converted
    /// body's paint seed; this passes [`Self::parse_with_seeds`] a fixed
    /// stream.
    pub fn parse(b: &[u8]) -> Result<Self> {
        let mut seeds = fixed_paint_seeds();
        Self::parse_with_seeds(b, &mut seeds)
    }

    /// [`Self::parse`] with the source of the paint seeds an older body
    /// layout's conversion stores (the original draws them from its process
    /// RNG; a save loader can pass the game's own source here).
    pub fn parse_with_seeds(b: &[u8], paint_seeds: &mut dyn FnMut() -> u64) -> Result<Self> {
        let magic = crate::read_u32(b, 0)?;
        let table_len = crate::read_u32(b, 4)? as usize;
        if !(0xDBAC_0D06..=super::MAGIC).contains(&magic) {
            return Err(Error::BadMagic {
                found: magic,
                expected: super::MAGIC,
            });
        }
        let count = (table_len >> 4) as i64 - 1;
        let mut store = if count < 1 {
            Self {
                negative: count < 0,
                ..Self::default()
            }
        } else {
            let count = count as usize;
            let blob_start = HEADER_LEN + table_len;
            if blob_start > b.len() {
                return Err(Error::Truncated {
                    offset: HEADER_LEN,
                    needed: table_len,
                    len: b.len().saturating_sub(HEADER_LEN),
                });
            }
            let table = &b[HEADER_LEN..blob_start];
            Self {
                entries: (0..count)
                    .map(|i| TableEntry::read(&table[i * ENTRY_LEN..]))
                    .collect(),
                tail: table[count * ENTRY_LEN..(count + 1) * ENTRY_LEN]
                    .try_into()
                    .expect("16 bytes"),
                negative: false,
                blob: b[blob_start..].to_vec(),
            }
        };
        super::upgrade::upgrade(&mut store, magic, paint_seeds)?;
        Ok(store)
    }

    /// Writes the tier as the original saves a database: the magic, a
    /// table length of `(records + 1) × 16`, and, when there is a record,
    /// the table, the slot after it and the blob as held.
    pub fn write(&self) -> Vec<u8> {
        let count = if self.negative && self.entries.is_empty() {
            -1
        } else {
            self.entries.len() as i64
        };
        let mut out = super::MAGIC.to_le_bytes().to_vec();
        out.extend((((count + 1) * ENTRY_LEN as i64) as u32).to_le_bytes());
        if count > 0 {
            for e in &self.entries {
                out.extend(e.bytes());
            }
            out.extend(self.tail);
            out.extend_from_slice(&self.blob);
        }
        out
    }

    /// A tier built from `(id, flags, payload)` records, inserted and
    /// stored in order as the original's routines would.
    pub fn from_records<'a>(records: impl IntoIterator<Item = (u32, u32, &'a [u8])>) -> Self {
        let mut store = Self::default();
        for (id, flags, payload) in records {
            let (_, at) = store.insert(id);
            store.entries[at].flags = flags;
            store.store(at, payload);
        }
        store
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// The table as held.
    pub fn entries(&self) -> &[TableEntry] {
        &self.entries
    }

    /// The payload blob as held.
    pub fn blob(&self) -> &[u8] {
        &self.blob
    }

    pub(crate) fn set_flags(&mut self, at: usize, flags: u32) {
        if let Some(e) = self.entries.get_mut(at) {
            e.flags = flags;
        }
    }

    /// The record at table index `at`, as the original's record view makes
    /// it: none for a size of 0 (or, in Exarust, a payload outside the
    /// blob).
    pub fn view(&self, at: usize) -> Option<Record<'_>> {
        let e = self.entries.get(at)?;
        if e.size == 0 {
            return None;
        }
        let start = e.offset as usize;
        let payload = self.blob.get(start..start.checked_add(e.size as usize)?)?;
        Some(Record::new(e.id, e.flags, payload))
    }

    /// Every record that has a view, in table order.
    pub fn records(&self) -> impl Iterator<Item = Record<'_>> {
        (0..self.entries.len()).filter_map(|i| self.view(i))
    }

    /// The table index of `id`, by the original's binary search (ids
    /// compared by their 32-bit difference).
    pub fn find_index(&self, id: u32) -> Option<usize> {
        let mut lo = 0i64;
        let mut hi = self.entries.len() as i64 - 1;
        while lo <= hi {
            let mid = (hi - lo) / 2 + lo;
            let diff = (id as i32).wrapping_sub(self.entries[mid as usize].id as i32);
            if diff < 0 {
                hi = mid - 1;
            } else if diff > 0 {
                lo = mid + 1;
            } else {
                return Some(mid as usize);
            }
        }
        None
    }

    /// The record with `id` when it has a view.
    pub fn find(&self, id: u32) -> Option<Record<'_>> {
        self.view(self.find_index(id)?)
    }

    /// The smallest positive id the tier lacks, as the original scans its
    /// table: past ids below the candidate, stopping at the first id above
    /// it.
    pub fn first_free_id(&self) -> u32 {
        let mut id = 1u32;
        for e in &self.entries {
            if id < e.id {
                break;
            }
            if e.id == id {
                id += 1;
            }
        }
        id
    }

    /// Inserts an empty entry for `id` at its sorted place, by the
    /// original's insertion search. Returns whether it was inserted and
    /// the entry's index (the existing one's for a duplicate id).
    pub fn insert(&mut self, id: u32) -> (bool, usize) {
        let mut lo = 0i64;
        let mut hi = self.entries.len() as i64 - 1;
        let mut mid = hi / 2;
        while lo <= hi {
            let diff = (id as i32).wrapping_sub(self.entries[mid as usize].id as i32);
            if diff < 0 {
                hi = mid - 1;
            } else if diff == 0 {
                return (false, mid as usize);
            } else {
                lo = mid + 1;
            }
            mid = lo + (hi - lo) / 2;
        }
        let at = mid.max(0) as usize;
        self.entries.insert(
            at,
            TableEntry {
                id,
                ..TableEntry::default()
            },
        );
        self.tail = [0; ENTRY_LEN];
        self.negative = false;
        (true, at)
    }

    /// Stores `payload` as the record at `at`, as the original does: the
    /// same size overwrites in place; another size removes the old bytes
    /// from the blob (every entry whose payload starts after them moves
    /// down) and appends the new payload at the end.
    pub fn store(&mut self, at: usize, payload: &[u8]) {
        let Some(&e) = self.entries.get(at) else {
            return;
        };
        let new_len = payload.len() as u32;
        if new_len != e.size {
            if e.size != 0 {
                let start = (e.offset as usize).min(self.blob.len());
                let end = (e.offset as usize + e.size as usize).min(self.blob.len());
                self.blob.drain(start..end);
                for other in &mut self.entries {
                    if e.offset < other.offset {
                        other.offset = other.offset.wrapping_sub(e.size);
                    }
                }
            }
            let offset = self.blob.len() as u32;
            self.blob.extend_from_slice(payload);
            let entry = &mut self.entries[at];
            entry.offset = offset;
            entry.size = new_len;
            return;
        }
        let start = e.offset as usize;
        if let Some(dst) = self.blob.get_mut(start..start + payload.len()) {
            dst.copy_from_slice(payload);
        }
    }

    /// Deletes the record at `at`, as the original does: its payload is
    /// stored empty (closing its hole) and the table closes up. The slot
    /// after the table then holds what the original's memory holds there:
    /// the deleted entry when it was the last, else the last entry again.
    pub fn remove_at(&mut self, at: usize) {
        if at >= self.entries.len() {
            return;
        }
        self.store(at, &[]);
        let removed = self.entries.remove(at);
        let stale = if at == self.entries.len() {
            removed
        } else {
            *self.entries.last().expect("an entry after the removed one")
        };
        self.tail = stale.bytes();
    }

    /// Removes the record with `id`; whether it was there.
    pub fn remove(&mut self, id: u32) -> bool {
        let Some(at) = self.find_index(id) else {
            return false;
        };
        self.remove_at(at);
        true
    }

    /// Empties the tier, as the original's clear does.
    pub fn clear(&mut self) {
        self.entries.clear();
        self.blob.clear();
        self.tail = [0; ENTRY_LEN];
        self.negative = false;
    }

    /// Rewrites one section in every record, as the original's record
    /// converters do: for each record holding a section with `tag`, `f`
    /// gets that section (header included) and a zeroed body of `body_len`
    /// bytes to fill; the old section is deleted and the new body appended
    /// as a section with the same tag.
    pub fn rewrite_sections(
        &mut self,
        tag: u32,
        body_len: u32,
        mut f: impl FnMut(&[u8], &mut [u8]),
    ) {
        for at in 0..self.entries.len() {
            let Some(record) = self.view(at) else {
                continue;
            };
            let Some(range) = super::record::section_at(record.payload(), tag) else {
                continue;
            };
            let mut body = vec![0u8; body_len as usize];
            f(&record.payload()[range], &mut body);
            let mut payload = record.payload().to_vec();
            super::record::remove_section(&mut payload, tag);
            let at_body = super::record::append_section(&mut payload, tag, body_len);
            payload[at_body..].copy_from_slice(&body);
            self.store(at, &payload);
        }
    }
}

/// The fixed paint-seed stream [`Store::parse`] hands the oldest body
/// layout's conversion: two 32-bit draws per seed, combined as the
/// original's 64-bit draw is (low word first). The original draws them from
/// the run-time library's Mersenne twister seeded from the clock; this
/// stand-in is a fixed linear congruential stream (a guess, chosen only so
/// a load is reproducible; no shipped file uses that layout).
fn fixed_paint_seeds() -> impl FnMut() -> u64 {
    let mut state = 0x2545_F491u32;
    move || {
        state = state.wrapping_mul(0x0808_8405).wrapping_add(1);
        let low = u64::from(state);
        state = state.wrapping_mul(0x0808_8405).wrapping_add(1);
        (u64::from(state) << 32) | low
    }
}

#[cfg(test)]
mod tests {
    use super::super::record::test_util::*;
    use super::*;
    use crate::objdb::{MAGIC, NAME};

    fn rec(name: &str) -> Vec<u8> {
        payload(&[(NAME, header(name, 0, 0))])
    }

    /// A database file with the given entries, slot and blob.
    fn file(entries: &[TableEntry], tail: [u8; 16], blob: &[u8]) -> Vec<u8> {
        let mut b = MAGIC.to_le_bytes().to_vec();
        b.extend((((entries.len() + 1) * 16) as u32).to_le_bytes());
        for e in entries {
            b.extend(e.bytes());
        }
        b.extend(tail);
        b.extend_from_slice(blob);
        b
    }

    #[test]
    fn a_loaded_database_writes_back_as_it_was() {
        // Payloads out of table order, a hole and trailing bytes in the
        // blob, a stale slot after the table: all of it survives.
        let entries = [
            TableEntry {
                id: 2,
                flags: NAME,
                offset: 40,
                size: 32,
            },
            TableEntry {
                id: 5,
                flags: NAME,
                offset: 0,
                size: 32,
            },
        ];
        let mut blob = rec("Five");
        blob.extend([0xEE; 8]);
        blob.extend(rec("Two"));
        blob.extend([1, 2, 3]);
        let b = file(&entries, [0x5A; 16], &blob);
        let store = Store::parse(&b).unwrap();
        assert_eq!(store.len(), 2);
        assert_eq!(store.find(2).unwrap().header().unwrap().name, "Two");
        assert_eq!(store.find(5).unwrap().header().unwrap().name, "Five");
        assert_eq!(store.write(), b);
    }

    #[test]
    fn short_tables_and_errors() {
        let mut empty = MAGIC.to_le_bytes().to_vec();
        empty.extend(16u32.to_le_bytes());
        let store = Store::parse(&empty).unwrap();
        assert!(store.is_empty());
        assert_eq!(store.write(), empty);
        let mut none = MAGIC.to_le_bytes().to_vec();
        none.extend(0u32.to_le_bytes());
        assert_eq!(Store::parse(&none).unwrap().write(), none);
        assert!(matches!(Store::parse(&[]), Err(Error::Truncated { .. })));
        let mut old = empty.clone();
        old[0..4].copy_from_slice(&0xDBAC_0D06u32.to_le_bytes());
        assert!(Store::parse(&old).is_ok());
        let mut bad = empty.clone();
        bad[0..4].copy_from_slice(&0x1234_5678u32.to_le_bytes());
        assert!(matches!(Store::parse(&bad), Err(Error::BadMagic { .. })));
        let mut short = MAGIC.to_le_bytes().to_vec();
        short.extend(48u32.to_le_bytes());
        short.extend([0; 20]);
        assert!(matches!(Store::parse(&short), Err(Error::Truncated { .. })));
        // An entry outside the blob, or of size 0, has no view.
        let b = file(
            &[
                TableEntry {
                    id: 1,
                    flags: 0,
                    offset: 100,
                    size: 4,
                },
                TableEntry {
                    id: 2,
                    flags: 0,
                    offset: 0,
                    size: 0,
                },
            ],
            [0; 16],
            &[0; 8],
        );
        let store = Store::parse(&b).unwrap();
        assert_eq!(store.find_index(1), Some(0));
        assert!(store.find(1).is_none() && store.find(2).is_none());
        assert_eq!(store.records().count(), 0);
    }

    #[test]
    fn search_insert_and_free_ids() {
        let mut store = Store::default();
        assert_eq!(store.first_free_id(), 1);
        assert_eq!(store.insert(3), (true, 0));
        assert_eq!(store.insert(1), (true, 0));
        assert_eq!(store.insert(7), (true, 2));
        assert_eq!(store.insert(5), (true, 2));
        assert_eq!(store.insert(5), (false, 2));
        let ids: Vec<u32> = store.entries().iter().map(|e| e.id).collect();
        assert_eq!(ids, [1, 3, 5, 7]);
        assert_eq!(store.find_index(5), Some(2));
        assert_eq!(store.find_index(4), None);
        assert_eq!(store.first_free_id(), 2);
        // Ids compare by 32-bit difference, as the original's search does:
        // 0x8000_0001 − 1 is negative there, so the probe goes left.
        let mut store = Store::default();
        store.insert(1);
        store.insert(0x8000_0001);
        assert_eq!(
            store.entries().iter().map(|e| e.id).collect::<Vec<_>>(),
            [0x8000_0001, 1]
        );
        assert_eq!(store.find_index(0x8000_0001), Some(0));
    }

    #[test]
    fn storing_moves_a_resized_payload_to_the_end() {
        let mut store = Store::from_records([
            (1, NAME, &rec("A")[..]),
            (2, NAME, &rec("B")[..]),
            (3, NAME, &rec("C")[..]),
        ]);
        let offsets = |s: &Store| s.entries().iter().map(|e| e.offset).collect::<Vec<_>>();
        assert_eq!(offsets(&store), [0, 32, 64]);
        // A longer payload for record 1: its 32 bytes go, B and C move
        // down, and the new 36 bytes go to the end.
        let mut longer = rec("A2");
        longer.extend([7; 4]);
        store.store(0, &longer);
        assert_eq!(offsets(&store), [64, 0, 32]);
        assert_eq!(store.blob().len(), 100);
        assert_eq!(store.find(1).unwrap().payload(), &longer[..]);
        // The same size overwrites in place.
        store.store(1, &rec("Bb"));
        assert_eq!(offsets(&store), [64, 0, 32]);
        assert_eq!(store.find(2).unwrap().header().unwrap().name, "Bb");
    }

    #[test]
    fn deleting_leaves_the_original_s_stale_slot() {
        let records = [
            (1, NAME, &rec("A")[..]),
            (2, NAME, &rec("B")[..]),
            (3, NAME, &rec("C")[..]),
        ];
        // Deleting a middle record: the slot after the table repeats the
        // last entry (C, moved down from offset 64 to 32).
        let mut store = Store::from_records(records);
        store.remove(2);
        let out = store.write();
        assert_eq!(u32_at(&out, 4), 3 * 16);
        assert_eq!(
            TableEntry::read(&out[8 + 32..]),
            TableEntry {
                id: 3,
                flags: NAME,
                offset: 32,
                size: 32
            }
        );
        // Deleting the last record: the slot holds it, emptied (its offset
        // the blob's new end, its size 0).
        let mut store = Store::from_records(records);
        store.remove(3);
        let out = store.write();
        assert_eq!(
            TableEntry::read(&out[8 + 32..]),
            TableEntry {
                id: 3,
                flags: NAME,
                offset: 64,
                size: 0
            }
        );
        assert_eq!(Store::parse(&out).unwrap().len(), 2);
        // Clearing: the header alone.
        store.clear();
        assert_eq!(store.write().len(), 8);
    }

    #[test]
    fn rewriting_a_section_in_every_record() {
        let mut store = Store::from_records([
            (
                1,
                NAME,
                &payload(&[(NAME, header("A", 0, 0)), (0x40, vec![1, 2])])[..],
            ),
            (2, NAME, &rec("B")[..]),
        ]);
        store.rewrite_sections(0x40, 3, |old, new| {
            assert_eq!(old, &[0x40, 0, 0, 0, 10, 0, 0, 0, 1, 2]);
            new.copy_from_slice(&[old[8] + 1, old[9] + 1, 9]);
        });
        assert_eq!(store.find(1).unwrap().section(0x40), Some(&[2, 3, 9][..]));
        assert_eq!(store.find(2).unwrap().section(0x40), None);
    }
}
