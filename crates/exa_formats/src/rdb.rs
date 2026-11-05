//! `.rdb` indexed record databases (`races`, `objdb`, `charroles`, `locales`).
//!
//! Header: `magic: u32`, `table_len: u32`, then a table of `table_len / 16`
//! slots of `id: u32`, `flags: u32`, `offset: u32`, `size: u32`. The last slot
//! is not a record: the original's loader counts `table_len / 16 − 1` records
//! and its writer saves the slot after the last entry as it lies in memory (a
//! stale entry or heap bytes; every shipped `.rdb` and every level database
//! ends with one). Offsets are relative to the end of the table. Record
//! payloads are database-specific.
//!
//! Evidence: the object database unit's database load (the count is the
//! table length shifted down by four, less one); `docs/formats/OTHER.md`
//! (Object database, Framing).

use crate::{Error, Result, read_u32};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Record {
    pub id: u32,
    pub flags: u32,
    pub offset: u32,
    pub size: u32,
}

#[derive(Debug, Clone)]
pub struct Database<'a> {
    pub magic: u32,
    data: &'a [u8],
    base: usize,
    pub records: Vec<Record>,
}

impl<'a> Database<'a> {
    pub fn parse(b: &'a [u8]) -> Result<Self> {
        let magic = read_u32(b, 0)?;
        let table_len = read_u32(b, 4)? as usize;
        if !table_len.is_multiple_of(16) {
            return Err(Error::Malformed(format!(
                "rdb table length {table_len} is not a multiple of 16"
            )));
        }
        let base = 8 + table_len;
        // The slot after the last entry is not read as a record.
        let count = (table_len / 16).saturating_sub(1);
        let mut records = Vec::with_capacity(count);
        for i in (8..).step_by(16).take(count) {
            let r = Record {
                id: read_u32(b, i)?,
                flags: read_u32(b, i + 4)?,
                offset: read_u32(b, i + 8)?,
                size: read_u32(b, i + 12)?,
            };
            // An entry the original keeps but whose payload lies outside
            // the data has no view.
            if (base + r.offset as usize)
                .checked_add(r.size as usize)
                .is_some_and(|end| end <= b.len())
            {
                records.push(r);
            }
        }
        Ok(Self {
            magic,
            data: b,
            base,
            records,
        })
    }

    pub fn payload(&self, r: &Record) -> &'a [u8] {
        let start = self.base + r.offset as usize;
        &self.data[start..start + r.size as usize]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_pointing_outside_the_data_are_skipped() {
        let mut b = 0xDBAC_0D09u32.to_le_bytes().to_vec();
        b.extend(48u32.to_le_bytes());
        for r in [[1u32, 0, 0, 4], [2, 0, 0x4000_0000, 8], [0, 0, 0, 0]] {
            b.extend(r.iter().flat_map(|v| v.to_le_bytes()));
        }
        b.extend([7, 7, 7, 7]);
        let db = Database::parse(&b).unwrap();
        assert_eq!(db.records.len(), 1);
        assert_eq!(db.payload(&db.records[0]), &[7, 7, 7, 7]);
    }

    /// The table's last slot is the writer's copy of whatever followed the
    /// last entry: never a record, even when it reads as one that fits.
    #[test]
    fn the_slot_after_the_last_entry_is_not_a_record() {
        let mut b = 0xDBAC_0D09u32.to_le_bytes().to_vec();
        b.extend(32u32.to_le_bytes());
        for r in [[1u32, 0, 0, 4], [9, 0, 0, 2]] {
            b.extend(r.iter().flat_map(|v| v.to_le_bytes()));
        }
        b.extend([7, 7, 7, 7]);
        let db = Database::parse(&b).unwrap();
        assert_eq!(db.records.iter().map(|r| r.id).collect::<Vec<_>>(), [1]);
        // A table of one slot holds no record; an empty one neither.
        let mut one = 0xDBAC_0D09u32.to_le_bytes().to_vec();
        one.extend(16u32.to_le_bytes());
        one.extend([0u8; 16]);
        assert!(Database::parse(&one).unwrap().records.is_empty());
        let mut empty = 0xDBAC_0D09u32.to_le_bytes().to_vec();
        empty.extend(0u32.to_le_bytes());
        assert!(Database::parse(&empty).unwrap().records.is_empty());
    }
}
