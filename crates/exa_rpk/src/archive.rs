use std::collections::HashMap;
use std::fs::File;
use std::path::{Path, PathBuf};

use memmap2::Mmap;

use crate::{Error, Result, read_u32};

/// `01 0C BF AF` on disk.
pub const RPK_MAGIC: u32 = 0xAFBF_0C01;

const HEADER_LEN: usize = 8;
const ENTRY_LEN: usize = 32;
const NAME_LEN: usize = 16;

/// One table-of-contents record.
///
/// Layout (32 bytes, little endian): `name[16]` NUL-padded (not terminated when
/// exactly 16 characters), `offset: u32` relative to the end of the table,
/// `size: u32`, then 8 bytes that are zero in every shipped pack.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub name: String,
    pub offset: u32,
    pub size: u32,
}

/// Parsed table of contents, independent of where the bytes live.
#[derive(Debug, Clone)]
struct Toc {
    data_base: usize,
    entries: Vec<Entry>,
    by_name: HashMap<String, usize>,
}

impl Toc {
    fn parse(data: &[u8]) -> Result<Self> {
        let magic = read_u32(data, 0)?;
        if magic != RPK_MAGIC {
            return Err(Error::BadMagic {
                found: magic,
                expected: RPK_MAGIC,
            });
        }
        let table_len = read_u32(data, 4)? as usize;
        if !table_len.is_multiple_of(ENTRY_LEN) {
            return Err(Error::Malformed(format!(
                "table length {table_len} is not a multiple of {ENTRY_LEN}"
            )));
        }
        let data_base = HEADER_LEN + table_len;
        if data_base > data.len() {
            return Err(Error::Truncated {
                offset: HEADER_LEN,
                needed: table_len,
                len: data.len(),
            });
        }

        let mut entries = Vec::with_capacity(table_len / ENTRY_LEN);
        let mut by_name = HashMap::with_capacity(table_len / ENTRY_LEN);
        for record in data[HEADER_LEN..data_base].as_chunks::<ENTRY_LEN>().0 {
            let raw_name = &record[..NAME_LEN];
            let end = raw_name.iter().position(|&c| c == 0).unwrap_or(NAME_LEN);
            let name: String = raw_name[..end].iter().map(|&c| c as char).collect();
            let offset = read_u32(record, 16)?;
            let size = read_u32(record, 20)?;
            let start = data_base + offset as usize;
            if start + size as usize > data.len() {
                return Err(Error::EntryOutOfBounds {
                    name,
                    offset: start,
                    size: size as usize,
                    len: data.len(),
                });
            }
            by_name.insert(name.to_ascii_lowercase(), entries.len());
            entries.push(Entry { name, offset, size });
        }
        Ok(Self {
            data_base,
            entries,
            by_name,
        })
    }

    /// Case-insensitive: packs store lowercase names while references inside
    /// models and materials use mixed case.
    fn find(&self, name: &str) -> Option<&Entry> {
        self.by_name
            .get(&name.to_ascii_lowercase())
            .map(|&i| &self.entries[i])
    }

    fn slice<'d>(&self, data: &'d [u8], entry: &Entry) -> &'d [u8] {
        let start = self.data_base + entry.offset as usize;
        &data[start..start + entry.size as usize]
    }
}

/// An archive borrowing its bytes, e.g. a pack nested inside another pack.
#[derive(Debug, Clone)]
pub struct Archive<'a> {
    data: &'a [u8],
    toc: Toc,
}

impl<'a> Archive<'a> {
    pub fn parse(data: &'a [u8]) -> Result<Self> {
        Ok(Self {
            data,
            toc: Toc::parse(data)?,
        })
    }

    pub fn entries(&self) -> &[Entry] {
        &self.toc.entries
    }

    pub fn len(&self) -> usize {
        self.toc.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.toc.entries.is_empty()
    }

    pub fn bytes(&self, entry: &Entry) -> &'a [u8] {
        self.toc.slice(self.data, entry)
    }

    pub fn find(&self, name: &str) -> Option<&Entry> {
        self.toc.find(name)
    }

    pub fn get(&self, name: &str) -> Option<&'a [u8]> {
        self.find(name).map(|e| self.bytes(e))
    }
}

/// A memory-mapped pack on disk with its table already parsed. Several packs
/// are larger than 1 GiB, so they are mapped rather than read.
pub struct MappedArchive {
    path: PathBuf,
    map: Mmap,
    toc: Toc,
}

impl MappedArchive {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        let io = |source| Error::Io {
            path: path.clone(),
            source,
        };
        let file = File::open(&path).map_err(io)?;
        // SAFETY: the game packs are read-only inputs; the mapping is never
        // written and the files are not expected to change while mapped.
        let map = unsafe { Mmap::map(&file) }.map_err(io)?;
        let toc = Toc::parse(&map)?;
        Ok(Self { path, map, toc })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// The whole mapped file, e.g. to build a borrowing [`Archive`].
    pub fn data(&self) -> &[u8] {
        &self.map
    }

    pub fn entries(&self) -> &[Entry] {
        &self.toc.entries
    }

    pub fn len(&self) -> usize {
        self.toc.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.toc.entries.is_empty()
    }

    pub fn bytes(&self, entry: &Entry) -> &[u8] {
        self.toc.slice(&self.map, entry)
    }

    pub fn find(&self, name: &str) -> Option<&Entry> {
        self.toc.find(name)
    }

    pub fn get(&self, name: &str) -> Option<&[u8]> {
        self.find(name).map(|e| self.bytes(e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build(entries: &[(&str, &[u8])]) -> Vec<u8> {
        let mut table = Vec::new();
        let mut blob = Vec::new();
        for (name, data) in entries {
            let mut n = [0u8; NAME_LEN];
            n[..name.len()].copy_from_slice(name.as_bytes());
            table.extend_from_slice(&n);
            table.extend_from_slice(&(blob.len() as u32).to_le_bytes());
            table.extend_from_slice(&(data.len() as u32).to_le_bytes());
            table.extend_from_slice(&[0; 8]);
            blob.extend_from_slice(data);
        }
        let mut out = RPK_MAGIC.to_le_bytes().to_vec();
        out.extend_from_slice(&(table.len() as u32).to_le_bytes());
        out.extend(table);
        out.extend(blob);
        out
    }

    #[test]
    fn parses_entries_and_full_width_names() {
        let bytes = build(&[("short", b"abc"), ("exactly16chars__", b"0123456789")]);
        let a = Archive::parse(&bytes).unwrap();
        assert_eq!(a.len(), 2);
        assert_eq!(a.get("SHORT"), Some(&b"abc"[..]));
        assert_eq!(a.get("exactly16chars__"), Some(&b"0123456789"[..]));
        assert!(a.get("missing").is_none());
    }

    #[test]
    fn rejects_bad_magic_and_out_of_bounds() {
        assert!(matches!(
            Archive::parse(&[0; 16]),
            Err(Error::BadMagic { .. })
        ));
        let mut bytes = build(&[("x", b"abc")]);
        bytes.truncate(bytes.len() - 1);
        assert!(matches!(
            Archive::parse(&bytes),
            Err(Error::EntryOutOfBounds { .. })
        ));
    }
}
