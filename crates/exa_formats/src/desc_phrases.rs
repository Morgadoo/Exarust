//! The item description phrase tables, read from the installed
//! executable (`Exanima.exe`, a Free Pascal Win64 PE image).
//!
//! The original builds the sentence an item card adds to an apparel or a
//! weapon description ("A well made gambeson in good condition.") from five
//! phrase tables and the eight armour-variant words. Exarust does not ship
//! that text: it reads the tables from the user's own executable.
//!
//! Each table row is 24 bytes in the image's data section: a pointer to a
//! constant Free Pascal ANSI string, a second 8-byte field that is zero in
//! every shipped row, and the row's inclusive mark range (two bytes, low
//! then high, on a 0–100 scale) followed by padding. A constant ANSI string
//! is its bytes and a terminating zero, preceded by its length as a signed
//! 64-bit word. The variant words are a run of eight such string pointers
//! after a null entry for variant 0.
//!
//! No address is used: a table is found by the pointer to its first phrase,
//! which is located by that phrase's text. "in great condition" starts one
//! table and is also the third row of another; the reader takes the
//! occurrence outside the tables it has already found.
//!
//! Evidence: the five tables' readers and their unit finaliser in the item
//! description unit of `Exanima.exe` 0.9.5 (REA), and a parse of the
//! installed image (`real_executable_has_every_table` below).

use crate::{Error, Result};

/// One phrase and the inclusive mark range (0–100) that may use it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PhraseRow {
    pub text: String,
    pub low: u8,
    pub high: u8,
}

/// Every description table the item card's sentences read.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DescPhrases {
    /// The first condition table (11 rows: "in flawless condition" …).
    pub flawless: Vec<PhraseRow>,
    /// The fallback condition table (10 rows: "in perfect condition" …).
    pub perfect: Vec<PhraseRow>,
    /// The quality table of dash-marked weapon names (8 rows).
    pub high: Vec<PhraseRow>,
    /// The condition table of dash-marked weapon names (6 rows).
    pub great: Vec<PhraseRow>,
    /// The general quality table (15 rows: "exceptionally well made" …).
    pub made: Vec<PhraseRow>,
    /// The armour-variant words of variants 1 to 8.
    pub variants: Vec<String>,
}

/// Rows of each table, in the order the original's sentence builders use
/// them: flawless, perfect, high, great, made.
pub const ROWS: [usize; 5] = [11, 10, 8, 6, 15];
/// The number of armour-variant words.
pub const VARIANTS: usize = 8;

/// The first phrase of each table, which anchors it.
const FIRST: [&str; 5] = [
    "in flawless condition",
    "in perfect condition",
    "very high quality",
    "in great condition",
    "exceptionally well made",
];
/// The first armour-variant word.
const FIRST_VARIANT: &str = "light";
/// A table row's size in bytes.
const ROW: u64 = 24;
/// The longest phrase the reader accepts.
const LONGEST: usize = 256;

impl DescPhrases {
    /// Reads every table from an executable's bytes.
    pub fn read(exe: &[u8]) -> Result<Self> {
        let image = PeImage::parse(exe)?;
        let mut found: Vec<(u64, u64)> = Vec::new();
        let mut tables: Vec<Vec<PhraseRow>> = Vec::new();
        for (first, rows) in FIRST.iter().zip(ROWS) {
            let (start, table) = image
                .phrase_table(first, rows, &found)
                .ok_or_else(|| Error::Malformed(format!("no phrase table starts {first:?}")))?;
            found.push((start, start + ROW * rows as u64));
            tables.push(table);
        }
        let variants = image
            .variant_words()
            .ok_or_else(|| Error::Malformed("no armour-variant words".into()))?;
        let mut tables = tables.into_iter();
        let mut next = || tables.next().unwrap_or_default();
        Ok(Self {
            flawless: next(),
            perfect: next(),
            high: next(),
            great: next(),
            made: next(),
            variants,
        })
    }
}

/// One section of a PE image.
#[derive(Debug, Clone, Copy)]
struct Section {
    /// Virtual address relative to the image base.
    rva: u64,
    /// File offset and size of the section's raw bytes.
    raw: usize,
    raw_len: usize,
    /// Initialised, writable data (where the tables live).
    writable: bool,
}

/// The few PE32+ facts the reader needs.
struct PeImage<'a> {
    bytes: &'a [u8],
    base: u64,
    sections: Vec<Section>,
}

fn u16_at(b: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_le_bytes(b.get(at..at + 2)?.try_into().ok()?))
}

fn u32_at(b: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(b.get(at..at + 4)?.try_into().ok()?))
}

fn u64_at(b: &[u8], at: usize) -> Option<u64> {
    Some(u64::from_le_bytes(b.get(at..at + 8)?.try_into().ok()?))
}

/// The section flag of initialised data and of a writable section.
const INITIALISED: u32 = 0x40;
const WRITABLE: u32 = 0x8000_0000;

impl<'a> PeImage<'a> {
    fn parse(bytes: &'a [u8]) -> Result<Self> {
        let bad = |what: &str| Error::Malformed(format!("not a PE32+ image: {what}"));
        if bytes.get(..2) != Some(b"MZ") {
            return Err(bad("no MZ header"));
        }
        let pe = u32_at(bytes, 0x3c).ok_or_else(|| bad("short header"))? as usize;
        if bytes.get(pe..pe + 4) != Some(b"PE\0\0") {
            return Err(bad("no PE signature"));
        }
        let count = u16_at(bytes, pe + 6).ok_or_else(|| bad("short header"))? as usize;
        let optional = u16_at(bytes, pe + 20).ok_or_else(|| bad("short header"))? as usize;
        let opt = pe + 24;
        if u16_at(bytes, opt) != Some(0x20b) {
            return Err(bad("not a 64-bit optional header"));
        }
        let base = u64_at(bytes, opt + 24).ok_or_else(|| bad("short optional header"))?;
        let mut sections = Vec::with_capacity(count);
        for index in 0..count {
            let at = opt + optional + index * 40;
            let field = |o: usize| u32_at(bytes, at + o).ok_or_else(|| bad("short section table"));
            let flags = field(36)?;
            sections.push(Section {
                rva: u64::from(field(12)?),
                raw: field(20)? as usize,
                raw_len: field(16)? as usize,
                writable: flags & INITIALISED != 0 && flags & WRITABLE != 0,
            });
        }
        Ok(Self {
            bytes,
            base,
            sections,
        })
    }

    /// A section's raw bytes, clipped to the file.
    fn raw(&self, section: &Section) -> &'a [u8] {
        let start = section.raw.min(self.bytes.len());
        let end = section
            .raw
            .saturating_add(section.raw_len)
            .min(self.bytes.len());
        &self.bytes[start..end]
    }

    /// The file bytes at a virtual address, up to the end of its section.
    fn at(&self, address: u64) -> Option<&'a [u8]> {
        let rva = address.checked_sub(self.base)?;
        self.sections.iter().find_map(|section| {
            let offset = usize::try_from(rva.checked_sub(section.rva)?).ok()?;
            let raw = self.raw(section);
            (offset < raw.len()).then(|| &raw[offset..])
        })
    }

    /// The constant ANSI string a pointer names, or `None` when it is not one.
    fn ansi(&self, address: u64) -> Option<&'a [u8]> {
        let length = u64_at(self.at(address.checked_sub(8)?)?, 0)?;
        let length = usize::try_from(length).ok().filter(|&n| n <= LONGEST)?;
        let bytes = self.at(address)?.get(..length + 1)?;
        (bytes[length] == 0).then(|| &bytes[..length])
    }

    /// Addresses of every constant ANSI string equal to `text`.
    fn strings(&self, text: &str) -> Vec<u64> {
        let mut needle = text.as_bytes().to_vec();
        needle.push(0);
        let mut found = Vec::new();
        for section in &self.sections {
            let raw = self.raw(section);
            for at in 8..raw.len().saturating_sub(needle.len() - 1) {
                if raw[at..].starts_with(&needle) && u64_at(raw, at - 8) == Some(text.len() as u64)
                {
                    found.push(self.base + section.rva + at as u64);
                }
            }
        }
        found
    }

    /// Addresses of every aligned pointer to `target` in writable data.
    fn pointers(&self, target: u64) -> Vec<u64> {
        let mut found = Vec::new();
        for section in self.sections.iter().filter(|s| s.writable) {
            let raw = self.raw(section);
            for at in (0..raw.len().saturating_sub(7)).step_by(8) {
                if u64_at(raw, at) == Some(target) {
                    found.push(self.base + section.rva + at as u64);
                }
            }
        }
        found
    }

    fn text(&self, address: u64) -> Option<String> {
        let bytes = self.ansi(address)?;
        Some(bytes.iter().map(|&b| char::from(b)).collect())
    }

    /// The table of `rows` rows whose first row names `first`, outside the
    /// `skip` ranges, and where it starts.
    fn phrase_table(
        &self,
        first: &str,
        rows: usize,
        skip: &[(u64, u64)],
    ) -> Option<(u64, Vec<PhraseRow>)> {
        self.strings(first)
            .into_iter()
            .flat_map(|string| self.pointers(string))
            .filter(|start| !skip.iter().any(|&(lo, hi)| (lo..hi).contains(start)))
            .find_map(|start| {
                let table = (0..rows as u64)
                    .map(|row| {
                        let record = self.at(start + row * ROW)?.get(..ROW as usize)?;
                        Some(PhraseRow {
                            text: self.text(u64_at(record, 0)?)?,
                            low: record[16],
                            high: record[17],
                        })
                    })
                    .collect::<Option<Vec<_>>>()?;
                Some((start, table))
            })
    }

    /// The eight variant words: a null entry, then eight string pointers.
    fn variant_words(&self) -> Option<Vec<String>> {
        self.strings(FIRST_VARIANT)
            .into_iter()
            .flat_map(|string| self.pointers(string))
            .find_map(|start| {
                if u64_at(self.at(start.checked_sub(8)?)?, 0)? != 0 {
                    return None;
                }
                (0..VARIANTS as u64)
                    .map(|index| self.text(u64_at(self.at(start + index * 8)?, 0)?))
                    .collect()
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BASE: u64 = 0x4000_0000;
    const DATA_RVA: u32 = 0x1000;
    const TEXT_RVA: u32 = 0x3000;
    const HEADERS: usize = 0x200;
    const SPAN: usize = 0x2000;

    /// A two-section PE32+ image: writable data, then read-only strings.
    struct Builder {
        data: Vec<u8>,
        strings: Vec<u8>,
    }

    impl Builder {
        fn new() -> Self {
            Self {
                data: Vec::new(),
                strings: Vec::new(),
            }
        }

        /// Adds a constant string and returns its address.
        fn string(&mut self, text: &str) -> u64 {
            while !self.strings.len().is_multiple_of(8) {
                self.strings.push(0);
            }
            self.strings.extend_from_slice(&u64::MAX.to_le_bytes());
            self.strings
                .extend_from_slice(&(text.len() as u64).to_le_bytes());
            let at = BASE + u64::from(TEXT_RVA) + self.strings.len() as u64;
            self.strings.extend_from_slice(text.as_bytes());
            self.strings.push(0);
            at
        }

        fn word(&mut self, value: u64) {
            self.data.extend_from_slice(&value.to_le_bytes());
        }

        fn table(&mut self, rows: &[(&str, u8, u8)]) {
            for &(text, low, high) in rows {
                let at = self.string(text);
                self.word(at);
                self.word(0);
                self.data.extend_from_slice(&[low, high, 0, 0, 0, 0, 0, 0]);
            }
        }

        fn build(self) -> Vec<u8> {
            let mut out = vec![0u8; HEADERS + 2 * SPAN];
            out[..2].copy_from_slice(b"MZ");
            out[0x3c..0x40].copy_from_slice(&0x40u32.to_le_bytes());
            out[0x40..0x44].copy_from_slice(b"PE\0\0");
            out[0x46..0x48].copy_from_slice(&2u16.to_le_bytes());
            out[0x54..0x56].copy_from_slice(&0xf0u16.to_le_bytes());
            let opt = 0x58;
            out[opt..opt + 2].copy_from_slice(&0x20bu16.to_le_bytes());
            out[opt + 24..opt + 32].copy_from_slice(&BASE.to_le_bytes());
            let table = opt + 0xf0;
            for (index, (rva, flags)) in [(DATA_RVA, 0xC000_0040u32), (TEXT_RVA, 0x4000_0040)]
                .into_iter()
                .enumerate()
            {
                let at = table + index * 40;
                out[at + 8..at + 12].copy_from_slice(&(SPAN as u32).to_le_bytes());
                out[at + 12..at + 16].copy_from_slice(&rva.to_le_bytes());
                out[at + 16..at + 20].copy_from_slice(&(SPAN as u32).to_le_bytes());
                let raw = (HEADERS + index * SPAN) as u32;
                out[at + 20..at + 24].copy_from_slice(&raw.to_le_bytes());
                out[at + 36..at + 40].copy_from_slice(&flags.to_le_bytes());
            }
            out[HEADERS..HEADERS + self.data.len()].copy_from_slice(&self.data);
            let strings = HEADERS + SPAN;
            out[strings..strings + self.strings.len()].copy_from_slice(&self.strings);
            out
        }
    }

    fn rows(first: &str, count: usize) -> Vec<(String, u8, u8)> {
        (0..count)
            .map(|i| {
                let text = if i == 0 {
                    first.to_string()
                } else {
                    format!("{first} {i}")
                };
                (text, i as u8, i as u8 + 1)
            })
            .collect()
    }

    fn synthetic() -> Vec<u8> {
        let mut b = Builder::new();
        b.word(0);
        let mut words = Vec::new();
        for i in 0..VARIANTS {
            words.push(b.string(&format!("{FIRST_VARIANT}{i}")));
        }
        // The first word must read exactly "light".
        words[0] = b.string(FIRST_VARIANT);
        for at in words {
            b.word(at);
        }
        for (index, (first, count)) in FIRST.iter().zip(ROWS).enumerate() {
            let mut table = rows(first, count);
            // The perfect table's third row repeats the great table's first.
            if index == 1 {
                table[2].0 = FIRST[3].to_string();
            }
            let borrowed: Vec<(&str, u8, u8)> =
                table.iter().map(|(t, l, h)| (t.as_str(), *l, *h)).collect();
            b.table(&borrowed);
        }
        b.build()
    }

    #[test]
    fn reads_every_table_and_the_variant_words() {
        let phrases = DescPhrases::read(&synthetic()).unwrap();
        assert_eq!(phrases.flawless.len(), 11);
        assert_eq!(phrases.perfect.len(), 10);
        assert_eq!(phrases.high.len(), 8);
        assert_eq!(phrases.made.len(), 15);
        assert_eq!(
            phrases.flawless[3],
            PhraseRow {
                text: "in flawless condition 3".into(),
                low: 3,
                high: 4
            }
        );
        // The great table is the one outside the perfect table.
        assert_eq!(phrases.great.len(), 6);
        assert_eq!(phrases.great[1].text, "in great condition 1");
        assert_eq!(phrases.perfect[2].text, "in great condition");
        assert_eq!(phrases.variants[0], "light");
        assert_eq!(phrases.variants[7], "light7");
    }

    #[test]
    fn malformed_images_are_errors() {
        assert!(DescPhrases::read(b"").is_err());
        assert!(DescPhrases::read(b"MZ").is_err());
        let mut image = synthetic();
        image.truncate(HEADERS + SPAN);
        assert!(DescPhrases::read(&image).is_err());
        let mut image = synthetic();
        image[0x58] = 0x0b;
        image[0x59] = 0x01;
        assert!(DescPhrases::read(&image).is_err());
    }

    /// The installed executable: every table with its row count, ranges on
    /// the 0–100 scale and the eight variant words.
    #[test]
    fn real_executable_has_every_table() {
        let Ok(dir) = std::env::var("EXANIMA_DIR") else {
            eprintln!("EXANIMA_DIR not set; skipping");
            return;
        };
        let Ok(exe) = std::fs::read(std::path::Path::new(&dir).join("Exanima.exe")) else {
            eprintln!("no Exanima.exe in EXANIMA_DIR; skipping");
            return;
        };
        let phrases = DescPhrases::read(&exe).unwrap();
        let tables = [
            &phrases.flawless,
            &phrases.perfect,
            &phrases.high,
            &phrases.great,
            &phrases.made,
        ];
        for (table, (rows, first)) in tables.iter().zip(ROWS.iter().zip(FIRST)) {
            assert_eq!(table.len(), *rows);
            assert_eq!(table[0].text, first);
            assert!(table.iter().all(|r| r.low <= r.high && r.high <= 100));
        }
        // Condition tables start at mark 0, quality tables end at 100.
        assert_eq!(
            (phrases.flawless[0].low, phrases.flawless[10].high),
            (0, 100)
        );
        assert_eq!((phrases.perfect[0].low, phrases.perfect[9].high), (0, 100));
        assert_eq!((phrases.made[0].high, phrases.made[14].low), (100, 0));
        assert_eq!(phrases.variants.len(), VARIANTS);
        assert_eq!(phrases.variants[0], "light");
    }
}
