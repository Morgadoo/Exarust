//! The raw-data resources (`RT_RCDATA`) an installation's executable
//! carries.
//!
//! The original keeps a few images inside `Exanima.exe` instead of a pack:
//! the GUI fonts (`Rayform06R_rfi` … `Rayform16R_rfi`), the glyph sheet
//! (`GlphCommon_rfi`) and the GUI symbols (`GUISymbl_rfi`). An image
//! reference whose source kind is "module resource" is looked up with the
//! system's resource finder by name (case-insensitive) as raw data, and its
//! bytes are an ordinary `.rfi`. Exarust reads the same resources from the
//! installed executable at run time; nothing is shipped.
//!
//! The parser walks the PE resource tree: the raw-data type, then each named
//! entry, then its first language. Malformed input is an error, never a
//! panic.

use std::collections::BTreeMap;

use crate::{Error, Result};

/// The resource type of raw data.
pub const RAW_DATA: u32 = 10;
/// The data directory slot of the resource tree.
const RESOURCE_DIRECTORY: usize = 2;
/// The most entries one resource directory may hold.
const MAX_ENTRIES: usize = 4096;

/// The named raw-data resources of one executable, by upper-case name.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ModuleResources {
    entries: BTreeMap<String, Vec<u8>>,
}

fn u16_at(b: &[u8], offset: usize) -> Result<u16> {
    b.get(offset..offset.saturating_add(2))
        .filter(|s| s.len() == 2)
        .map(|s| u16::from_le_bytes([s[0], s[1]]))
        .ok_or(Error::Truncated {
            offset,
            needed: 2,
            len: b.len(),
        })
}

fn u32_at(b: &[u8], offset: usize) -> Result<u32> {
    b.get(offset..offset.saturating_add(4))
        .filter(|s| s.len() == 4)
        .map(|s| u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
        .ok_or(Error::Truncated {
            offset,
            needed: 4,
            len: b.len(),
        })
}

/// One section's mapping: its virtual address and span, and where its raw
/// bytes sit in the file.
struct Section {
    address: u32,
    span: u32,
    file: u32,
    raw: u32,
}

/// A directory's entries: (name word, offset word) pairs.
fn directory(b: &[u8], tree: usize, at: usize) -> Result<Vec<(u32, u32)>> {
    let base = tree
        .checked_add(at)
        .ok_or_else(|| Error::Malformed("resource directory out of range".into()))?;
    let named = usize::from(u16_at(b, base + 12)?);
    let ids = usize::from(u16_at(b, base + 14)?);
    let count = named + ids;
    if count > MAX_ENTRIES {
        return Err(Error::Malformed(format!(
            "resource directory with {count} entries"
        )));
    }
    (0..count)
        .map(|i| {
            let entry = base + 16 + i * 8;
            Ok((u32_at(b, entry)?, u32_at(b, entry + 4)?))
        })
        .collect()
}

impl ModuleResources {
    /// Reads the named raw-data resources of a PE image (32- or 64-bit).
    /// An image without a resource tree has none.
    pub fn parse(b: &[u8]) -> Result<Self> {
        if b.get(..2) != Some(b"MZ") {
            return Err(Error::Malformed("not an executable (no MZ)".into()));
        }
        let pe = u32_at(b, 0x3c)? as usize;
        if b.get(pe..pe.saturating_add(4)) != Some(b"PE\0\0") {
            return Err(Error::Malformed("not a PE image".into()));
        }
        let sections = usize::from(u16_at(b, pe + 6)?);
        let optional_size = usize::from(u16_at(b, pe + 20)?);
        let optional = pe + 24;
        let (count_at, directories_at) = match u16_at(b, optional)? {
            0x10b => (optional + 92, optional + 96),
            0x20b => (optional + 108, optional + 112),
            magic => {
                return Err(Error::Malformed(format!(
                    "optional header magic {magic:#06x}"
                )));
            }
        };
        let directories = u32_at(b, count_at)? as usize;
        if directories <= RESOURCE_DIRECTORY {
            return Ok(Self::default());
        }
        let rva = u32_at(b, directories_at + RESOURCE_DIRECTORY * 8)?;
        if rva == 0 {
            return Ok(Self::default());
        }
        let table = optional + optional_size;
        let sections = (0..sections)
            .map(|i| {
                let s = table + i * 40;
                let span = u32_at(b, s + 8)?;
                let raw = u32_at(b, s + 16)?;
                Ok(Section {
                    address: u32_at(b, s + 12)?,
                    span: span.max(raw),
                    file: u32_at(b, s + 20)?,
                    raw,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        let to_file = |rva: u32, size: u32| -> Result<usize> {
            let section = sections
                .iter()
                .find(|s| rva >= s.address && rva - s.address < s.span)
                .ok_or_else(|| Error::Malformed(format!("address {rva:#x} in no section")))?;
            let within = rva - section.address;
            if u64::from(within) + u64::from(size) > u64::from(section.raw) {
                return Err(Error::Malformed(format!(
                    "resource at {rva:#x} runs past its section's raw bytes"
                )));
            }
            Ok(section.file as usize + within as usize)
        };
        let tree = to_file(rva, 16)?;
        let mut entries = BTreeMap::new();
        for (kind, offset) in directory(b, tree, 0)? {
            if kind != RAW_DATA || offset & 0x8000_0000 == 0 {
                continue;
            }
            for (name, offset) in directory(b, tree, (offset & 0x7fff_ffff) as usize)? {
                if name & 0x8000_0000 == 0 || offset & 0x8000_0000 == 0 {
                    continue;
                }
                let at = tree + (name & 0x7fff_ffff) as usize;
                let length = usize::from(u16_at(b, at)?);
                let units = (0..length)
                    .map(|i| u16_at(b, at + 2 + i * 2))
                    .collect::<Result<Vec<_>>>()?;
                let key = String::from_utf16_lossy(&units).to_ascii_uppercase();
                let languages = directory(b, tree, (offset & 0x7fff_ffff) as usize)?;
                let Some(&(_, data)) = languages.iter().find(|(_, o)| o & 0x8000_0000 == 0) else {
                    continue;
                };
                let data = tree + data as usize;
                let (address, size) = (u32_at(b, data)?, u32_at(b, data + 4)?);
                let start = to_file(address, size)?;
                let bytes = b
                    .get(start..start + size as usize)
                    .ok_or(Error::Truncated {
                        offset: start,
                        needed: size as usize,
                        len: b.len(),
                    })?;
                entries.entry(key).or_insert_with(|| bytes.to_vec());
            }
        }
        Ok(Self { entries })
    }

    /// A raw-data resource by name, matched without regard to case as the
    /// system's resource finder does.
    pub fn get(&self, name: &str) -> Option<&[u8]> {
        self.entries
            .get(&name.to_ascii_uppercase())
            .map(Vec::as_slice)
    }

    /// The resources' names, upper case.
    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.entries.keys().map(String::as_str)
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A 64-bit PE image with one section holding a resource tree of two
    /// raw-data entries and one icon-type entry (synthetic bytes).
    fn image() -> Vec<u8> {
        let mut b = vec![0u8; 0x400];
        b[0..2].copy_from_slice(b"MZ");
        b[0x3c..0x40].copy_from_slice(&0x80u32.to_le_bytes());
        b[0x80..0x84].copy_from_slice(b"PE\0\0");
        let put16 =
            |b: &mut Vec<u8>, at: usize, v: u16| b[at..at + 2].copy_from_slice(&v.to_le_bytes());
        let put32 =
            |b: &mut Vec<u8>, at: usize, v: u32| b[at..at + 4].copy_from_slice(&v.to_le_bytes());
        put16(&mut b, 0x86, 1); // one section
        put16(&mut b, 0x94, 240); // optional header size
        let optional = 0x98;
        put16(&mut b, optional, 0x20b);
        put32(&mut b, optional + 108, 16);
        // The resource directory at 0x1000, 0x200 long.
        put32(&mut b, optional + 112 + 16, 0x1000);
        put32(&mut b, optional + 112 + 20, 0x200);
        let section = optional + 240;
        put32(&mut b, section + 8, 0x200);
        put32(&mut b, section + 12, 0x1000);
        put32(&mut b, section + 16, 0x200);
        put32(&mut b, section + 20, 0x200);
        let r = 0x200;
        // Root: two id entries, type 3 (ignored) and type 10.
        put16(&mut b, r + 14, 2);
        put32(&mut b, r + 16, 3);
        put32(&mut b, r + 20, 0x8000_0000 | 0x30);
        put32(&mut b, r + 24, RAW_DATA);
        put32(&mut b, r + 28, 0x8000_0000 | 0x30);
        // Names: two named entries.
        put16(&mut b, r + 0x30 + 12, 2);
        put32(&mut b, r + 0x30 + 16, 0x8000_0000 | 0x100);
        put32(&mut b, r + 0x30 + 20, 0x8000_0000 | 0x60);
        put32(&mut b, r + 0x30 + 24, 0x8000_0000 | 0x120);
        put32(&mut b, r + 0x30 + 28, 0x8000_0000 | 0x78);
        // Languages: one id entry each, pointing at a data entry.
        put16(&mut b, r + 0x60 + 14, 1);
        put32(&mut b, r + 0x60 + 16, 2057);
        put32(&mut b, r + 0x60 + 20, 0x90);
        put16(&mut b, r + 0x78 + 14, 1);
        put32(&mut b, r + 0x78 + 16, 2057);
        put32(&mut b, r + 0x78 + 20, 0xa0);
        // Data entries: RVA and size.
        put32(&mut b, r + 0x90, 0x1000 + 0x180);
        put32(&mut b, r + 0x94, 4);
        put32(&mut b, r + 0xa0, 0x1000 + 0x190);
        put32(&mut b, r + 0xa4, 3);
        let name = |b: &mut Vec<u8>, at: usize, text: &str| {
            put16(b, at, text.len() as u16);
            for (i, c) in text.encode_utf16().enumerate() {
                put16(b, at + 2 + i * 2, c);
            }
        };
        name(&mut b, r + 0x100, "FONT_RFI");
        name(&mut b, r + 0x120, "SYMBOLS_RFI");
        b[r + 0x180..r + 0x184].copy_from_slice(b"abcd");
        b[r + 0x190..r + 0x193].copy_from_slice(b"xyz");
        b
    }

    #[test]
    fn raw_data_resources_are_found_by_name_in_any_case() {
        let resources = ModuleResources::parse(&image()).unwrap();
        assert_eq!(resources.get("Font_rfi"), Some(&b"abcd"[..]));
        assert_eq!(resources.get("symbols_RFI"), Some(&b"xyz"[..]));
        assert_eq!(resources.get("missing"), None);
        assert_eq!(
            resources.names().collect::<Vec<_>>(),
            ["FONT_RFI", "SYMBOLS_RFI"]
        );
    }

    #[test]
    fn malformed_images_are_errors() {
        assert!(ModuleResources::parse(b"").is_err());
        assert!(ModuleResources::parse(b"MZ").is_err());
        let mut b = image();
        // A data entry pointing past its section.
        b[0x200 + 0x94..0x200 + 0x98].copy_from_slice(&0x1000u32.to_le_bytes());
        assert!(ModuleResources::parse(&b).is_err());
        // Every truncation is an error or a smaller result, never a panic.
        let full = image();
        for len in 0..full.len() {
            let _ = ModuleResources::parse(&full[..len]);
        }
    }
}
