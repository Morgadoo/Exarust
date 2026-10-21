//! Shader libraries (`.flb`) and the text programs they hold.
//!
//! `Resource.rpk` carries two of them: `shading.flb` (the surface, light,
//! ambient, fog, reflection, GI and effect programs) and `textures.flb` (the
//! procedural texture generators). The container is the pack table of
//! contents; every entry's bytes are rotated by a position-keyed stream
//! cipher, so identical plaintext prefixes give identical ciphertext
//! prefixes. Layout and evidence: `docs/formats/FLB.md`.
//!
//! Entry text is kept as the engine keeps it: the decoded bytes less up to
//! four trailing NULs of padding. The expander walks the text by its length;
//! GL sees the finished program up to its first NUL. A parser returns an
//! error, never a panic, on malformed input.

use std::collections::HashMap;

use crate::{Error, Result, read_u32};

/// The library's table-of-contents magic, the same word as a pack's.
pub const FLB_MAGIC: u32 = 0xAFBF_0C01;
/// Bytes per table-of-contents record.
const ENTRY_LEN: usize = 32;
/// Bytes of an entry's name, NUL-padded, not terminated at exactly this
/// length.
const NAME_LEN: usize = 16;
/// The cipher's starting key, restored for every entry.
const CIPHER_KEY: u32 = 0x2BCB_E389;
/// The multiplier of the cipher's wrapping linear congruential key stream.
const CIPHER_MULTIPLIER: u32 = 0x10A8_60C1;

/// One decoded program of a library.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShaderEntry {
    /// The entry name from the table of contents, e.g. `shadeopt_f`.
    pub name: String,
    /// The program text: the decoded bytes, one character per byte, less
    /// the trailing NUL padding.
    pub source: String,
}

/// A decoded `.flb` shader library.
#[derive(Debug, Clone, Default)]
pub struct ShaderLibrary {
    entries: Vec<ShaderEntry>,
    by_name: HashMap<String, usize>,
}

impl ShaderLibrary {
    /// Parse a library from the bytes of a `.flb` entry.
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        let magic = read_u32(bytes, 0)?;
        if magic != FLB_MAGIC {
            return Err(Error::BadMagic {
                found: magic,
                expected: FLB_MAGIC,
            });
        }
        let table_len = read_u32(bytes, 4)? as usize;
        if !table_len.is_multiple_of(ENTRY_LEN) {
            return Err(Error::Malformed(format!(
                "shader table length {table_len} is not a multiple of {ENTRY_LEN}"
            )));
        }
        let data_base = 8 + table_len;
        if data_base > bytes.len() {
            return Err(Error::Truncated {
                offset: 8,
                needed: table_len,
                len: bytes.len(),
            });
        }
        let mut entries = Vec::with_capacity(table_len / ENTRY_LEN);
        let mut by_name = HashMap::with_capacity(table_len / ENTRY_LEN);
        for record in bytes[8..data_base].as_chunks::<ENTRY_LEN>().0 {
            let raw_name = &record[..NAME_LEN];
            let end = raw_name.iter().position(|&c| c == 0).unwrap_or(NAME_LEN);
            let name: String = raw_name[..end].iter().map(|&c| c as char).collect();
            let offset = read_u32(record, 16)? as usize;
            let size = read_u32(record, 20)? as usize;
            let start = data_base.checked_add(offset).ok_or_else(|| {
                Error::Malformed(format!("entry {name:?} offset {offset} overflows"))
            })?;
            let end = start.saturating_add(size);
            let encoded = bytes.get(start..end).ok_or_else(|| {
                Error::Malformed(format!(
                    "entry {name:?} runs past the library ({start}+{size} > {})",
                    bytes.len()
                ))
            })?;
            let source = decode_entry(encoded);
            by_name.insert(name.clone(), entries.len());
            entries.push(ShaderEntry {
                name,
                source: source.iter().map(|&c| c as char).collect(),
            });
        }
        Ok(Self { entries, by_name })
    }

    pub fn entries(&self) -> &[ShaderEntry] {
        &self.entries
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// The program named `name`; names are compared exactly, as the engine
    /// compares them.
    pub fn get(&self, name: &str) -> Option<&str> {
        self.by_name
            .get(name)
            .map(|&i| self.entries[i].source.as_str())
    }

    /// The index of the program named `name`, in table order.
    pub fn index_of(&self, name: &str) -> Option<usize> {
        self.by_name.get(name).copied()
    }
}

/// Undo the entry cipher: every 32-bit word is rotated left by a value the
/// key stream picks, the key restarting at [`CIPHER_KEY`] for each entry.
///
/// The engine decodes `length / 4` whole words; a trailing partial word is
/// left zero here (no shipped entry has one).
fn decode_entry(encoded: &[u8]) -> Vec<u8> {
    let mut out = vec![0u8; encoded.len()];
    let mut key = CIPHER_KEY;
    let words = encoded.len() / 4;
    for (index, word) in encoded[..words * 4].as_chunks::<4>().0.iter().enumerate() {
        key = key.wrapping_mul(CIPHER_MULTIPLIER);
        let mut rotation = (key >> 13) & 0x1f;
        if (key >> 13) & 7 == 0 {
            rotation |= 4;
        }
        let cipher = u32::from_le_bytes(*word);
        let plain = if rotation == 0 {
            cipher
        } else {
            cipher.rotate_left(rotation)
        };
        out[index * 4..index * 4 + 4].copy_from_slice(&plain.to_le_bytes());
    }
    // The engine trims the entry's NUL padding: at most four bytes, and only
    // when the last one is a NUL.
    if out.last() == Some(&0) {
        for _ in 0..4 {
            if out.last() == Some(&0) {
                out.pop();
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Inverse of the engine's rotation, for synthetic libraries only.
    fn encode(plain: &[u8]) -> Vec<u8> {
        let mut out = Vec::with_capacity(plain.len());
        let mut key = CIPHER_KEY;
        for word in plain.chunks(4) {
            key = key.wrapping_mul(CIPHER_MULTIPLIER);
            let mut rotation = (key >> 13) & 0x1f;
            if (key >> 13) & 7 == 0 {
                rotation |= 4;
            }
            let mut w = [0u8; 4];
            w[..word.len()].copy_from_slice(word);
            let value = u32::from_le_bytes(w);
            let cipher = if rotation == 0 {
                value
            } else {
                value.rotate_right(rotation)
            };
            out.extend_from_slice(&cipher.to_le_bytes());
        }
        out
    }

    fn library(entries: &[(&str, &[u8])]) -> Vec<u8> {
        let table_len = entries.len() * ENTRY_LEN;
        let mut out = Vec::new();
        out.extend_from_slice(&FLB_MAGIC.to_le_bytes());
        out.extend_from_slice(&(table_len as u32).to_le_bytes());
        let mut offset = 0u32;
        for (name, plain) in entries {
            let mut record = [0u8; ENTRY_LEN];
            let bytes = name.as_bytes();
            record[..bytes.len().min(NAME_LEN)]
                .copy_from_slice(&bytes[..bytes.len().min(NAME_LEN)]);
            record[16..20].copy_from_slice(&offset.to_le_bytes());
            // The engine decodes whole words, so shipped entries are padded
            // to four bytes.
            let padded = plain.len().div_ceil(4) * 4;
            record[20..24].copy_from_slice(&(padded as u32).to_le_bytes());
            out.extend_from_slice(&record);
            offset += padded as u32;
        }
        for (_, plain) in entries {
            out.extend_from_slice(&encode(plain));
        }
        out
    }

    #[test]
    fn decodes_a_pinned_word() {
        // "AB\0\0" under the first key.
        let cipher = [0x00, 0x00, 0x82, 0x84];
        assert_eq!(decode_entry(&cipher), b"AB");
    }

    #[test]
    fn reads_entries_and_keeps_their_order() {
        let bytes = library(&[
            ("shading_v", b"in vec3 vPos;\r\n"),
            ("shadeopt_f", b"void main() {}\r\n"),
        ]);
        let lib = ShaderLibrary::parse(&bytes).unwrap();
        assert_eq!(lib.len(), 2);
        assert_eq!(lib.entries()[0].name, "shading_v");
        assert_eq!(lib.entries()[1].name, "shadeopt_f");
        assert_eq!(lib.get("shading_v"), Some("in vec3 vPos;\r\n"));
        assert_eq!(lib.get("Shading_v"), None);
        assert_eq!(lib.index_of("shadeopt_f"), Some(1));
    }

    #[test]
    fn a_sixteen_character_name_need_not_end_in_nul() {
        let name = "abcdefghijklmnop";
        let bytes = library(&[(name, b"text")]);
        let lib = ShaderLibrary::parse(&bytes).unwrap();
        assert_eq!(lib.entries()[0].name, name);
    }

    #[test]
    fn text_keeps_an_embedded_nul_and_drops_the_padding() {
        let bytes = library(&[("prog", b"abc\0def"), ("pad", b"abc\0\0\0\0\0")]);
        let lib = ShaderLibrary::parse(&bytes).unwrap();
        assert_eq!(lib.get("prog"), Some("abc\0def"));
        // Four NULs go; a fifth stays.
        assert_eq!(lib.get("pad"), Some("abc\0"));
    }

    #[test]
    fn rejects_a_bad_magic() {
        let mut bytes = library(&[("a", b"x")]);
        bytes[0] = 0;
        assert!(matches!(
            ShaderLibrary::parse(&bytes),
            Err(Error::BadMagic { .. })
        ));
    }

    #[test]
    fn rejects_a_misaligned_table() {
        let mut bytes = library(&[("a", b"x")]);
        bytes[4] = 0x21;
        assert!(matches!(
            ShaderLibrary::parse(&bytes),
            Err(Error::Malformed(_))
        ));
    }

    #[test]
    fn rejects_an_entry_out_of_bounds() {
        let mut bytes = library(&[("a", b"x")]);
        bytes[8 + 16..8 + 20].copy_from_slice(&1000u32.to_le_bytes());
        assert!(matches!(
            ShaderLibrary::parse(&bytes),
            Err(Error::Malformed(_))
        ));
    }

    #[test]
    fn rejects_a_short_header() {
        assert!(matches!(
            ShaderLibrary::parse(&[0x01, 0x0c]),
            Err(Error::Truncated { .. })
        ));
    }

    #[test]
    fn reads_the_installed_shading_library() {
        let Some(dir) = exa_rpk::locate_install(None) else {
            return;
        };
        let game = exa_rpk::GameData::open(&dir).unwrap();
        let resource = game.pack(exa_rpk::PackKind::Resource).unwrap();
        let bytes = resource.get("shading.flb").expect("shading.flb");
        let lib = ShaderLibrary::parse(bytes).unwrap();
        assert!(lib.len() >= 54);
        for name in [
            "shading_v",
            "shading_f",
            "shadeopt_f",
            "shadesss_f",
            "shadehar_f",
            "shadetrl_f",
            "lights",
            "ambient",
            "normal",
            "speculargi",
            "fog",
            "reflection",
            "refraction",
        ] {
            assert!(lib.get(name).is_some(), "missing {name}");
        }
        assert!(lib.get("shadeopt_f").unwrap().contains("void main()"));
        assert!(!lib.get("shading_v").unwrap().starts_with('#'));
    }
}
