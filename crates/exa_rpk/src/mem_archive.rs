//! An `.rpk`-format pack held entirely in memory, whose entries can be
//! read, added, replaced and deleted in place.
//!
//! The original's `TMemBlockLibrary` is the material and shader library:
//! it reads the table and the whole payload area into memory, appends a
//! zeroed 32-byte entry for a new name in sorted order, replaces an entry's
//! payload by closing the old bytes' hole and appending the new ones (or
//! overwriting at the same length), and deletes by replacing with an empty
//! payload and closing the table up.
//!
//! Layout: [`crate::RPK_MAGIC`], `table_len: u32`, then
//! `table_len / 32` entries of `name[16]`, `offset: u32`, `size: u32`, 8
//! reserved bytes; offsets count from the end of the table. Evidence:
//! `docs/formats/RPK.md`.

use crate::{Error, RPK_MAGIC, Result, read_u32};

const HEADER_LEN: usize = 8;
const ENTRY_LEN: usize = 32;
const NAME_LEN: usize = 16;

/// One table entry over the archive's payload area.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MemEntry {
    /// The name field, NUL padded.
    pub name: [u8; NAME_LEN],
    /// Where the payload starts in the payload area.
    pub offset: u32,
    /// Its length.
    pub size: u32,
    /// The bytes the record carries past its size word; zero in every
    /// shipped pack, and kept as read when the library is written back.
    pub reserved: [u8; 8],
}

impl MemEntry {
    /// The name as text, cut at the first NUL.
    pub fn name_text(&self) -> String {
        let end = self.name.iter().position(|&c| c == 0).unwrap_or(NAME_LEN);
        self.name[..end].iter().map(|&c| c as char).collect()
    }

    /// The 16-byte name lower-cased, which is the order and the key the
    /// original's table uses.
    fn key(&self) -> [u8; NAME_LEN] {
        self.name.map(|b| b.to_ascii_lowercase())
    }

    fn write(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(&self.name);
        out.extend_from_slice(&self.offset.to_le_bytes());
        out.extend_from_slice(&self.size.to_le_bytes());
        out.extend_from_slice(&self.reserved);
    }
}

/// A mutable in-memory pack.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MemArchive {
    entries: Vec<MemEntry>,
    blob: Vec<u8>,
}

impl MemArchive {
    /// Reads a pack from memory: the magic and the table are checked, the
    /// payload area is everything after the table. Entries are kept as they
    /// are (an entry pointing outside the payload reads as nothing), and
    /// are sorted by the lower-cased name as the original's table is.
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        let magic = read_u32(bytes, 0)?;
        if magic != RPK_MAGIC {
            return Err(Error::BadMagic {
                found: magic,
                expected: RPK_MAGIC,
            });
        }
        let table_len = read_u32(bytes, 4)? as usize;
        if !table_len.is_multiple_of(ENTRY_LEN) {
            return Err(Error::Malformed(format!(
                "table length {table_len} is not a multiple of {ENTRY_LEN}"
            )));
        }
        let table_end = HEADER_LEN
            .checked_add(table_len)
            .filter(|&end| end <= bytes.len())
            .ok_or(Error::Truncated {
                offset: HEADER_LEN,
                needed: table_len,
                len: bytes.len(),
            })?;
        let mut entries = Vec::with_capacity(table_len / ENTRY_LEN);
        for record in bytes[HEADER_LEN..table_end].as_chunks::<ENTRY_LEN>().0 {
            let mut name = [0u8; NAME_LEN];
            name.copy_from_slice(&record[..NAME_LEN]);
            entries.push(MemEntry {
                name,
                offset: read_u32(record, 16)?,
                size: read_u32(record, 20)?,
                reserved: record[24..32].try_into().expect("8 bytes"),
            });
        }
        Ok(Self {
            entries,
            blob: bytes[table_end..].to_vec(),
        })
    }

    /// An empty pack.
    pub fn new() -> Self {
        Self::default()
    }

    /// The entries, sorted by their lower-cased names.
    pub fn entries(&self) -> &[MemEntry] {
        &self.entries
    }

    /// The payload area.
    pub fn blob(&self) -> &[u8] {
        &self.blob
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// The sorted position of `name` (ASCII case ignored, as the original's
    /// 16-byte lower-cased comparison).
    pub fn find(&self, name: &str) -> Option<usize> {
        self.search(&name_key(name)).ok()
    }

    /// The payload of `name`.
    pub fn get(&self, name: &str) -> Option<&[u8]> {
        self.bytes(self.find(name)?)
    }

    /// The payload at `index`; `None` for an empty entry or one pointing
    /// outside the payload area.
    pub fn bytes(&self, index: usize) -> Option<&[u8]> {
        let entry = self.entries.get(index)?;
        if entry.size == 0 {
            return None;
        }
        let start = entry.offset as usize;
        self.blob
            .get(start..start.checked_add(entry.size as usize)?)
    }

    /// Reads an entry as the original's in-memory "read entry" slot does:
    /// empty payloads report none.
    pub fn read_entry(&self, index: usize) -> Option<&[u8]> {
        self.bytes(index)
    }

    fn search(&self, key: &[u8; NAME_LEN]) -> std::result::Result<usize, usize> {
        self.entries.binary_search_by(|e| e.key().cmp(key))
    }

    /// Adds a name with no payload, at its sorted place, as the original's
    /// sorted insertion does; the caller then replaces the payload. An
    /// existing name returns its index.
    pub fn add(&mut self, name: &str) -> usize {
        let key = name_key(name);
        match self.search(&key) {
            Ok(index) => index,
            Err(index) => {
                self.entries.insert(
                    index,
                    MemEntry {
                        name: key,
                        offset: 0,
                        size: 0,
                        reserved: [0; 8],
                    },
                );
                index
            }
        }
    }

    /// Replaces the payload at `index`, as the original's editor does: the
    /// same length overwrites in place; another length closes the old
    /// bytes' hole, moves the payloads after it down and appends the new
    /// bytes at the end.
    pub fn replace(&mut self, index: usize, payload: &[u8]) {
        let Some(entry) = self.entries.get(index).copied() else {
            return;
        };
        if payload.len() as u32 != entry.size {
            if entry.size != 0 {
                let start = (entry.offset as usize).min(self.blob.len());
                let end = (entry.offset as usize + entry.size as usize).min(self.blob.len());
                self.blob.drain(start..end);
                for e in &mut self.entries {
                    if entry.offset < e.offset {
                        e.offset = e.offset.wrapping_sub(entry.size);
                    }
                }
            }
            let at = self.blob.len() as u32;
            self.blob.extend_from_slice(payload);
            let e = &mut self.entries[index];
            e.offset = at;
            e.size = payload.len() as u32;
            return;
        }
        let start = entry.offset as usize;
        if let Some(dst) = self.blob.get_mut(start..start + payload.len()) {
            dst.copy_from_slice(payload);
        }
    }

    /// Replaces the payload of `name`, adding the name when it is absent.
    pub fn set(&mut self, name: &str, payload: &[u8]) -> usize {
        let index = self.add(name);
        self.replace(index, payload);
        index
    }

    /// Deletes the entry at `index`: an empty payload closes its bytes and
    /// the entry leaves the table.
    pub fn delete(&mut self, index: usize) -> Option<MemEntry> {
        if index >= self.entries.len() {
            return None;
        }
        self.replace(index, &[]);
        Some(self.entries.remove(index))
    }

    /// Sets the entry's name (the original's "set file name" slot), without
    /// re-sorting: the caller is the one that gives a new library its name.
    pub fn set_name(&mut self, index: usize, name: &str) -> bool {
        let Some(entry) = self.entries.get_mut(index) else {
            return false;
        };
        entry.name = name_key(name);
        true
    }

    /// The pack's bytes as it is held: magic, the table and the payload
    /// area. Entries keep their order and offsets.
    pub fn write(&self) -> Vec<u8> {
        let mut out = RPK_MAGIC.to_le_bytes().to_vec();
        out.extend(((self.entries.len() * ENTRY_LEN) as u32).to_le_bytes());
        for e in &self.entries {
            e.write(&mut out);
        }
        out.extend_from_slice(&self.blob);
        out
    }
}

/// A name as the table's 16-byte lower-cased key: the first 16 bytes, NUL
/// padded.
fn name_key(name: &str) -> [u8; NAME_LEN] {
    let mut key = [0u8; NAME_LEN];
    for (slot, byte) in key.iter_mut().zip(name.bytes()) {
        *slot = byte.to_ascii_lowercase();
    }
    key
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pack(entries: &[(&str, &[u8])]) -> Vec<u8> {
        let mut table = Vec::new();
        let mut blob = Vec::new();
        for &(name, payload) in entries {
            let mut key = [0u8; NAME_LEN];
            key[..name.len()].copy_from_slice(name.as_bytes());
            table.extend_from_slice(&key);
            table.extend_from_slice(&(blob.len() as u32).to_le_bytes());
            table.extend_from_slice(&(payload.len() as u32).to_le_bytes());
            table.extend_from_slice(&[0; 8]);
            blob.extend_from_slice(payload);
        }
        let mut out = RPK_MAGIC.to_le_bytes().to_vec();
        out.extend((table.len() as u32).to_le_bytes());
        out.extend(table);
        out.extend(blob);
        out
    }

    #[test]
    fn reads_names_case_insensitively_and_writes_back() {
        let bytes = pack(&[("Alpha", b"one"), ("beta", b"two")]);
        let mut archive = MemArchive::parse(&bytes).unwrap();
        assert_eq!(archive.get("ALPHA"), Some(&b"one"[..]));
        assert_eq!(archive.get("Beta"), Some(&b"two"[..]));
        assert_eq!(archive.write(), bytes);
        assert_eq!(archive.add("alpha"), 0);
    }

    #[test]
    fn adding_sorts_and_replacing_moves_the_payload_like_the_original() {
        let mut archive = MemArchive::parse(&pack(&[("a", b"AA"), ("c", b"CC")])).unwrap();
        let b = archive.add("b");
        assert_eq!(b, 1);
        archive.replace(b, b"BBBB");
        assert_eq!(
            archive
                .entries()
                .iter()
                .map(|e| e.name_text())
                .collect::<Vec<_>>(),
            ["a", "b", "c"]
        );
        assert_eq!(archive.get("a"), Some(&b"AA"[..]));
        assert_eq!(archive.get("b"), Some(&b"BBBB"[..]));
        assert_eq!(archive.get("c"), Some(&b"CC"[..]));
        // Same length overwrites in place.
        archive.replace(b, b"bbbb");
        assert_eq!(archive.get("b"), Some(&b"bbbb"[..]));
        // A shorter payload closes its hole and moves c down.
        archive.replace(b, b"B");
        assert_eq!(archive.get("c"), Some(&b"CC"[..]));
        assert_eq!(archive.get("b"), Some(&b"B"[..]));
        // The written pack parses back to the same contents.
        let again = MemArchive::parse(&archive.write()).unwrap();
        assert_eq!(again, archive);
    }

    #[test]
    fn deleting_empties_the_payload_and_closes_the_table() {
        let mut archive =
            MemArchive::parse(&pack(&[("a", b"AA"), ("b", b"BB"), ("c", b"CC")])).unwrap();
        let removed = archive.delete(1).unwrap();
        assert_eq!(removed.name_text(), "b");
        assert_eq!(archive.len(), 2);
        assert_eq!(archive.get("a"), Some(&b"AA"[..]));
        assert_eq!(archive.get("c"), Some(&b"CC"[..]));
        assert_eq!(archive.get("b"), None);
        assert_eq!(archive.delete(9), None);
        // A fresh name with no payload reads as nothing (the in-memory
        // read-entry rule).
        archive.add("d");
        assert_eq!(archive.read_entry(2), None);
        assert_eq!(archive.bytes(2), None);
    }

    #[test]
    fn bad_files_are_errors_not_panics() {
        assert!(matches!(
            MemArchive::parse(&[0; 8]),
            Err(Error::BadMagic { .. })
        ));
        let mut bytes = pack(&[("a", b"x")]);
        bytes[4] = 17;
        assert!(matches!(
            MemArchive::parse(&bytes),
            Err(Error::Malformed(_))
        ));
        let mut bytes = pack(&[("a", b"x")]);
        bytes.truncate(12);
        assert!(matches!(
            MemArchive::parse(&bytes),
            Err(Error::Truncated { .. })
        ));
        // An entry past the payload reads as nothing.
        let mut bytes = pack(&[("a", b"x")]);
        let end = bytes.len();
        bytes.truncate(end - 1);
        let archive = MemArchive::parse(&bytes).unwrap();
        assert_eq!(archive.get("a"), None);
    }

    /// Every shipped material library reads and writes back byte for byte,
    /// and looks an entry up case-insensitively.
    #[test]
    fn the_shipped_material_libraries() {
        use crate::{GameData, PackKind, locate_install};
        let Some(dir) = locate_install(None) else {
            return;
        };
        let game = GameData::open(&dir).unwrap();
        let resource = game.pack(PackKind::Resource).unwrap();
        let mut libraries = 0;
        for path in &game.project().material_libraries {
            let name = path.rsplit(['\\', '/']).next().unwrap();
            let bytes = resource.get(name).unwrap();
            let archive = MemArchive::parse(bytes).unwrap();
            assert_eq!(archive.write(), bytes, "{name}");
            for entry in archive.entries() {
                assert_eq!(
                    archive
                        .find(&entry.name_text())
                        .map(|i| &archive.entries()[i]),
                    Some(entry),
                    "{name}: {}",
                    entry.name_text()
                );
            }
            libraries += 1;
        }
        assert_eq!(libraries, 3);
    }
}
