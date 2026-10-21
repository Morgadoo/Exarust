//! A record payload of the object database: its chain of sections, the
//! header, model and folder fields, and the original's section editing
//! routines. Layout and evidence: `docs/formats/OTHER.md` (Object
//! database).

use super::packed::FractionWord;
use super::{BODY, FOLDER, Item, MODEL, NAME, WEAPON};
use crate::Result;

/// One section of a record payload: its tag and the bytes after the
/// `tag, size` header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Section<'a> {
    pub tag: u32,
    pub body: &'a [u8],
}

/// The sections of a record, in file order. A size below the 8-byte header
/// or one that runs past the payload stops the walk (the original would
/// loop or read past the record).
#[derive(Debug, Clone, Copy)]
pub struct Sections<'a> {
    rest: &'a [u8],
}

impl<'a> Iterator for Sections<'a> {
    type Item = Section<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let header = self.rest.get(..8)?;
        let tag = u32::from_le_bytes(header[0..4].try_into().expect("4 bytes"));
        let size = u32::from_le_bytes(header[4..8].try_into().expect("4 bytes")) as usize;
        let body = (size >= 8).then(|| self.rest.get(8..size)).flatten();
        let Some(body) = body else {
            self.rest = &[];
            return None;
        };
        self.rest = &self.rest[size..];
        Some(Section { tag, body })
    }
}

/// The byte range of the first section with `tag`, header included, as
/// the original's section finder walks the chain.
pub(crate) fn section_at(payload: &[u8], tag: u32) -> Option<std::ops::Range<usize>> {
    let mut pos = 0;
    while let Some(header) = payload.get(pos..pos + 8) {
        let found = u32_at(header, 0);
        let size = u32_at(header, 4) as usize;
        if size < 8 {
            return None;
        }
        let end = pos.checked_add(size)?;
        if end > payload.len() {
            return None;
        }
        if found == tag {
            return Some(pos..end);
        }
        pos = end;
    }
    None
}

/// The body range (after the 8-byte header) of the first section with
/// `tag`.
pub(crate) fn section_range(payload: &[u8], tag: u32) -> Option<std::ops::Range<usize>> {
    section_at(payload, tag).map(|r| r.start + 8..r.end)
}

pub(crate) fn u32_at(b: &[u8], o: usize) -> u32 {
    b.get(o..o + 4)
        .map_or(0, |w| u32::from_le_bytes(w.try_into().expect("4 bytes")))
}

pub(crate) fn u16_at(b: &[u8], o: usize) -> u16 {
    b.get(o..o + 2)
        .map_or(0, |w| u16::from_le_bytes(w.try_into().expect("2 bytes")))
}

pub(crate) fn set_u32(b: &mut [u8], at: usize, value: u32) {
    if let Some(word) = b.get_mut(at..at + 4) {
        word.copy_from_slice(&value.to_le_bytes());
    }
}

pub(crate) fn set_u16(b: &mut [u8], at: usize, value: u16) {
    if let Some(word) = b.get_mut(at..at + 2) {
        word.copy_from_slice(&value.to_le_bytes());
    }
}

/// Copies `len` bytes at `at` from `src` to `dst` when both hold them.
pub(crate) fn copy_bytes(src: &[u8], dst: &mut [u8], at: usize, len: usize) {
    if let (Some(s), Some(d)) = (src.get(at..at + len), dst.get_mut(at..at + len)) {
        d.copy_from_slice(s);
    }
}

/// Writes a word into the first section with `tag`, at `offset` in its
/// body.
pub(crate) fn set_section_u32(payload: &mut [u8], tag: u32, offset: usize, value: u32) {
    if let Some(range) = section_range(payload, tag) {
        set_u32(&mut payload[range], offset, value);
    }
}

/// A record as the table and blob hold it: its id, its table flags and the
/// chain of sections.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Record<'a> {
    pub id: u32,
    /// The table's flags word. The databases' writer stores the OR of the
    /// record's section tags here; the original tests it before walking a
    /// section in its flagged lookup.
    pub flags: u32,
    payload: &'a [u8],
}

impl<'a> Record<'a> {
    pub fn new(id: u32, flags: u32, payload: &'a [u8]) -> Self {
        Self { id, flags, payload }
    }

    pub fn payload(&self) -> &'a [u8] {
        self.payload
    }

    pub fn sections(&self) -> Sections<'a> {
        Sections { rest: self.payload }
    }

    /// The body of the first section with `tag`.
    pub fn section(&self, tag: u32) -> Option<&'a [u8]> {
        section_range(self.payload, tag).map(|r| &self.payload[r])
    }

    /// Whether the table flags name `tag`.
    pub fn has(&self, tag: u32) -> bool {
        self.flags & tag != 0
    }

    /// Whether any section's tag shares a bit with `mask`, as the original's
    /// mask test walks the chain.
    pub fn has_any(&self, mask: u32) -> bool {
        self.sections().any(|s| s.tag & mask != 0)
    }

    /// The header section: the display name, its word and the parent id.
    pub fn header(&self) -> Option<Header> {
        let b = self.section(NAME)?;
        if b.len() < 0x18 {
            return None;
        }
        Some(Header {
            name: super::name(b),
            flags: u32_at(b, 0x10),
            parent: u32_at(b, 0x14),
        })
    }

    /// The header's parent word found by walking to the header section:
    /// the template an instance was made from, 0 for none.
    pub fn template(&self) -> u32 {
        self.section(NAME).map_or(0, |b| u32_at(b, 0x14))
    }

    /// The word at payload offset `0x18`: the header's flags word when the
    /// header is the first section, which several of the original's
    /// routines read directly (every shipped record starts with it).
    pub fn header_word(&self) -> u32 {
        u32_at(self.payload, 0x18)
    }

    /// The word at payload offset `0x1C`: the header's parent word when
    /// the header is the first section, read directly as the original's
    /// string resolver, template helpers and body copy do.
    pub fn parent_word(&self) -> u32 {
        u32_at(self.payload, 0x1C)
    }

    /// The model section: the ground or icon model, its flags word, the
    /// ids of the display name and the description, and the shop fields.
    pub fn model(&self) -> Option<Model> {
        let b = self.section(MODEL)?;
        if b.len() < 0x1C {
            return None;
        }
        Some(Model {
            name: super::name(b),
            flags: u32_at(b, 0x10),
            name_id: u32_at(b, 0x14),
            description_id: u32_at(b, 0x18),
            class: b.get(0x1C).copied().unwrap_or(0),
            rarity: b.get(0x1D).copied().unwrap_or(0),
            value: u16_at(b, 0x1E),
            size: [u32_at(b, 0x20), u32_at(b, 0x24)],
        })
    }

    /// The folder section: the items an editor folder lists.
    pub fn folder(&self) -> Option<Folder> {
        let b = self.section(FOLDER)?;
        let count = u32_at(b, 0x0C) as usize;
        let entries = (0..count)
            .map_while(|i| b.get(0x10 + i * 16..0x10 + i * 16 + 16))
            .map(|e| FolderEntry {
                id: u32_at(e, 0),
                words: [u16_at(e, 4), u16_at(e, 6), u16_at(e, 8)],
                fractions: FractionWord(u16_at(e, 10)),
                extra: u32_at(e, 12),
            })
            .collect();
        Some(Folder { entries })
    }

    /// The number of weapon variants the record's section names: the low
    /// five bits of its word at `0x42`, one less for the half-sword form
    /// (`flags & 0x30 == 0x10`), never below zero.
    pub fn weapon_variants(&self) -> Option<u8> {
        let body = self.section(WEAPON)?;
        let word = (u16_at(body, 0x42) & 0x1F) as u8;
        Some(if u32_at(body, 0) & 0x30 == 0x10 {
            word.saturating_sub(1)
        } else {
            word
        })
    }

    /// Whether the record has an item body.
    pub fn has_body(&self) -> bool {
        self.section(BODY).is_some()
    }

    /// Decodes the record's item fields, as [`Item::parse`] does.
    pub fn item(&self) -> Result<Item> {
        Item::parse(self.id, self.payload)
    }
}

/// The header section of a record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Header {
    /// The display name, 16 bytes at most, Latin-1.
    pub name: String,
    /// The header's word: bit 0 ([`super::INSTANCE_ON_USE`]) makes a
    /// template instantiate into the local tier when used; bits 16–23 are
    /// the material class the template migration compares.
    pub flags: u32,
    /// The parent: the editor folder for a global item, the source item
    /// for a level-local copy, and the template for an instance.
    pub parent: u32,
}

impl Header {
    /// Whether using the record instantiates it into the local tier.
    pub fn instance_on_use(&self) -> bool {
        self.flags & super::INSTANCE_ON_USE != 0
    }

    /// The material class the template migration keeps a body's appearance
    /// in step with.
    pub fn material_class(&self) -> u32 {
        (self.flags >> 16) & 0xFF
    }
}

/// The model section of a record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Model {
    /// The ground or icon model, 16 bytes at most, Latin-1.
    pub name: String,
    /// Bit 0 ([`super::CONDITION_SCALED`]) dims the model's lights by a
    /// placed copy's condition byte; bits 4–6 are the shop's rarity tier.
    pub flags: u32,
    /// The display name's string id ([`super::INHERIT`] = the template's).
    pub name_id: u32,
    /// The description's string id.
    pub description_id: u32,
    /// The class byte the original's model-class helper returns; its
    /// meaning is not established.
    pub class: u8,
    /// Hundredths of rarity, the shop's weight.
    pub rarity: u8,
    /// The value in the compact amount form (`amount_expand`), which the
    /// condition refresh scales.
    pub value: u16,
    /// Two words a stocked item needs non-zero (`[guess]` the icon's size).
    pub size: [u32; 2],
}

/// A folder's item list (`0x200`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Folder {
    pub entries: Vec<FolderEntry>,
}

/// One item an editor folder lists.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FolderEntry {
    /// The item record's id.
    pub id: u32,
    /// The three words after the id; their meanings are not established.
    pub words: [u16; 3],
    /// Five fractions packed in one word, used by the item randomisers.
    pub fractions: FractionWord,
    /// A trailing word, zero in every shipped entry.
    pub extra: u32,
}

/// Removes the first section with `tag` from a payload, as the original's
/// section delete does. Returns whether one was found.
pub fn remove_section(payload: &mut Vec<u8>, tag: u32) -> bool {
    let Some(range) = section_at(payload, tag) else {
        return false;
    };
    payload.drain(range);
    true
}

/// Resizes the first section with `tag` to a body of `body_len` bytes, as
/// the original's section resize does, and returns the section's offset.
/// Shrinking (or keeping the size) writes the new size word and closes the
/// gap. Growing opens the gap by moving the bytes after the section up,
/// and leaves the size word and the gap's bytes as they were: the original
/// does not update them on that path either.
pub fn resize_section(payload: &mut Vec<u8>, tag: u32, body_len: u32) -> Option<usize> {
    let range = section_at(payload, tag)?;
    let start = range.start;
    let old = range.len();
    let new = body_len as usize + 8;
    if old < new {
        let tail = payload.len() - range.end;
        payload.resize(payload.len() + new - old, 0);
        payload.copy_within(start + old..start + old + tail, start + new);
    } else {
        set_u32(payload, start + 4, new as u32);
        payload.copy_within(start + old.., start + new);
        payload.truncate(payload.len() - (old - new));
    }
    Some(start)
}

/// Appends a section with `tag` and a zeroed body of `body_len` bytes, as
/// the original's section append does (its new body is left
/// uninitialised), and returns the body's offset.
pub fn append_section(payload: &mut Vec<u8>, tag: u32, body_len: u32) -> usize {
    let at = payload.len();
    payload.extend(tag.to_le_bytes());
    payload.extend((body_len + 8).to_le_bytes());
    payload.resize(at + 8 + body_len as usize, 0);
    at + 8
}

#[cfg(test)]
pub(crate) mod test_util {
    /// A section: tag, size (body + 8) and body.
    pub fn section(tag: u32, body: &[u8]) -> Vec<u8> {
        let mut v = tag.to_le_bytes().to_vec();
        v.extend((body.len() as u32 + 8).to_le_bytes());
        v.extend_from_slice(body);
        v
    }

    pub fn padded(s: &str, len: usize) -> Vec<u8> {
        let mut v = s.as_bytes().to_vec();
        v.resize(len, 0);
        v
    }

    pub fn payload(sections: &[(u32, Vec<u8>)]) -> Vec<u8> {
        sections.iter().flat_map(|(t, b)| section(*t, b)).collect()
    }

    /// A header body: name[16], flags word, parent word.
    pub fn header(name: &str, flags: u32, parent: u32) -> Vec<u8> {
        let mut b = padded(name, 16);
        b.extend(flags.to_le_bytes());
        b.extend(parent.to_le_bytes());
        b
    }

    /// A 40-byte model body: name[16], flags, name id, description id.
    pub fn model(name: &str, flags: u32, name_id: u32, description_id: u32) -> Vec<u8> {
        let mut b = padded(name, 16);
        b.extend(flags.to_le_bytes());
        b.extend(name_id.to_le_bytes());
        b.extend(description_id.to_le_bytes());
        b.resize(0x28, 0);
        b
    }
}

#[cfg(test)]
mod tests {
    use super::test_util::*;
    use super::*;
    use crate::objdb::{CONTAINER, OPERATIVE};

    #[test]
    fn sections_header_and_model() {
        let mut m = model("SwordWorld", 0x11, 7, 8);
        m[0x1C] = 3;
        m[0x1D] = 40;
        m[0x1E..0x20].copy_from_slice(&0x4123u16.to_le_bytes());
        let p = payload(&[
            (NAME, header("Sword", 1, 9)),
            (MODEL, m),
            (0x9999, vec![1, 2, 3]),
        ]);
        let view = Record::new(1, NAME | MODEL, &p);
        let tags: Vec<u32> = view.sections().map(|s| s.tag).collect();
        assert_eq!(tags, [NAME, MODEL, 0x9999]);
        assert!(view.has(NAME) && view.has(MODEL) && !view.has(BODY));
        assert!(view.has_any(0x1) && view.has_any(MODEL) && !view.has_any(0x2000));
        assert_eq!(view.section(0x9999), Some(&[1, 2, 3][..]));
        let h = view.header().unwrap();
        assert_eq!((h.name.as_str(), h.flags, h.parent), ("Sword", 1, 9));
        assert!(h.instance_on_use());
        // With the header first, its words sit at payload 0x18 and 0x1C.
        assert_eq!((view.header_word(), view.parent_word()), (1, 9));
        assert_eq!(view.template(), 9);
        let m = view.model().unwrap();
        assert_eq!(m.name, "SwordWorld");
        assert_eq!((m.flags, m.name_id, m.description_id), (0x11, 7, 8));
        assert_eq!((m.class, m.rarity, m.value), (3, 40, 0x4123));
    }

    #[test]
    fn malformed_chains_stop_the_walk() {
        // A size below the header.
        let mut bad = NAME.to_le_bytes().to_vec();
        bad.extend(4u32.to_le_bytes());
        bad.extend([0; 8]);
        let view = Record::new(1, NAME, &bad);
        assert_eq!(view.sections().count(), 0);
        assert_eq!(view.section(NAME), None);
        assert!(view.header().is_none());
        // A size past the payload.
        let mut past = NAME.to_le_bytes().to_vec();
        past.extend(100u32.to_le_bytes());
        assert_eq!(Record::new(1, NAME, &past).sections().count(), 0);
        assert_eq!(Record::new(1, NAME, &past).parent_word(), 0);
    }

    #[test]
    fn folder_entries() {
        let mut b = vec![0u8; 0x10 + 2 * 16];
        b[0x0C..0x10].copy_from_slice(&2u32.to_le_bytes());
        b[0x10..0x14].copy_from_slice(&7u32.to_le_bytes());
        b[0x14..0x16].copy_from_slice(&0x640Au16.to_le_bytes());
        b[0x18..0x1A].copy_from_slice(&0x80u16.to_le_bytes());
        b[0x1A..0x1C].copy_from_slice(&0x5284u16.to_le_bytes());
        b[0x20..0x24].copy_from_slice(&9u32.to_le_bytes());
        let p = payload(&[(NAME, header("Folder", 0, 0)), (FOLDER, b.clone())]);
        let folder = Record::new(1, NAME | FOLDER, &p).folder().unwrap();
        assert_eq!(folder.entries.len(), 2);
        let e = folder.entries[0];
        assert_eq!((e.id, e.words, e.extra), (7, [0x640A, 0, 0x80], 0));
        assert_eq!(e.fractions, FractionWord(0x5284));
        assert_eq!(folder.entries[1].id, 9);
        // A count past the body stops at the last whole entry.
        b[0x0C..0x10].copy_from_slice(&100u32.to_le_bytes());
        let p = section(FOLDER, &b);
        assert_eq!(
            Record::new(1, FOLDER, &p).folder().unwrap().entries.len(),
            2
        );
    }

    #[test]
    fn weapon_variant_count() {
        let mut weapon = vec![0u8; 0x60];
        weapon[0x42] = 0xE5; // low five bits: 5
        let p = payload(&[(NAME, header("Sword", 0, 0)), (WEAPON, weapon.clone())]);
        assert_eq!(Record::new(1, 0, &p).weapon_variants(), Some(5));
        weapon[0] = 0x10; // the half-sword form
        let p = payload(&[(NAME, header("Sword", 0, 0)), (WEAPON, weapon.clone())]);
        assert_eq!(Record::new(1, 0, &p).weapon_variants(), Some(4));
        weapon[0x42] = 0;
        let p = payload(&[(WEAPON, weapon)]);
        assert_eq!(Record::new(1, 0, &p).weapon_variants(), Some(0));
        let p = payload(&[(NAME, header("Sword", 0, 0))]);
        assert_eq!(Record::new(1, 0, &p).weapon_variants(), None);
    }

    #[test]
    fn section_editing() {
        let original = payload(&[
            (NAME, header("A", 0, 0)),
            (CONTAINER, vec![1, 2, 3, 4]),
            (OPERATIVE, vec![9, 9]),
        ]);
        // Delete.
        let mut p = original.clone();
        assert!(remove_section(&mut p, CONTAINER));
        assert_eq!(
            p,
            payload(&[(NAME, header("A", 0, 0)), (OPERATIVE, vec![9, 9])])
        );
        assert!(!remove_section(&mut p, CONTAINER));
        // Shrink: the size word follows and the tail moves down.
        let mut p = original.clone();
        let at = resize_section(&mut p, CONTAINER, 2).unwrap();
        assert_eq!(at, 32);
        assert_eq!(
            p,
            payload(&[
                (NAME, header("A", 0, 0)),
                (CONTAINER, vec![1, 2]),
                (OPERATIVE, vec![9, 9]),
            ])
        );
        // Grow: the tail moves up by two, the size word stays at 12 and the
        // gap keeps the bytes that were there (the operative section's
        // first two header bytes).
        let mut p = original.clone();
        resize_section(&mut p, CONTAINER, 6).unwrap();
        assert_eq!(p.len(), original.len() + 2);
        assert_eq!(u32_at(&p, 36), 12);
        assert_eq!(&p[40..44], &[1, 2, 3, 4]);
        assert_eq!(&p[44..46], &original[44..46]);
        assert_eq!(&p[46..], &original[44..]);
        assert_eq!(resize_section(&mut p, 0x7777, 1), None);
        // Append.
        let mut p = payload(&[(NAME, header("A", 0, 0))]);
        let body = append_section(&mut p, WEAPON, 4);
        assert_eq!(body, 32 + 8);
        assert_eq!(
            Record::new(1, 0, &p).section(WEAPON),
            Some(&[0, 0, 0, 0][..])
        );
    }
}
