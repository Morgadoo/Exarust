//! `.rdb` indexed record databases (`races`, `objdb`, `charroles`, `locales`).
//!
//! Header: `magic: u32`, `table_len: u32`, then `table_len / 16` records of
//! `id: u32`, `flags: u32`, `offset: u32`, `size: u32`. Offsets are relative to
//! the end of the table. Record payloads are database-specific.

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
        let mut records = Vec::with_capacity(table_len / 16);
        for i in (8..base).step_by(16) {
            let r = Record {
                id: read_u32(b, i)?,
                flags: read_u32(b, i + 4)?,
                offset: read_u32(b, i + 8)?,
                size: read_u32(b, i + 12)?,
            };
            if base + r.offset as usize + r.size as usize > b.len() {
                return Err(Error::Truncated {
                    offset: base + r.offset as usize,
                    needed: r.size as usize,
                    len: b.len(),
                });
            }
            records.push(r);
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
