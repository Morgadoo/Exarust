//! The retail save package (`SG_SaveWorld`) and the `#playerdata` section
//! both modes write (`XA_SaveGames`).
//!
//! A package is the magic `0xDA7AEA02`, a section count, a sentinel entry and
//! one 24-byte directory entry per section: a u32 offset, a u32 size and a
//! fixed 16-byte name. Section payloads start at the next `0x2000`
//! boundary after the entry before them. Writing a section that is already
//! there rewrites it at its place when the room up to the next section
//! suffices; otherwise the section is moved to the last gap it fits in, or
//! appended, and the old bytes are left alone. Deleting a section only
//! removes its directory entry. A write overlays the header, directory and
//! payloads on the bytes read, so gaps, deleted payloads and an old tail
//! survive and the file never shrinks.
//!
//! The `#playerdata` section is a fixed 52-byte header followed by the
//! player's character placement chunk and then tagged chunks. The story and
//! arena headers share their first 36 bytes and differ in the last 16. The
//! writer emits the chunks it knows in the mode's order; the reader resets
//! the journal, the global experience worth, the memory flags and the
//! transition state before reading, and takes the first chunk after the
//! header as the character whatever its id.
//!
//! Evidence and the guesses are in `docs/formats/SAVES.md`; the container
//! rules come from the package create, open, write and delete routines and
//! the two player-data readers and writers listed there.

/// The magic of every package, story and arena alike.
pub const PACKAGE_MAGIC: u32 = 0xDA7A_EA02;

/// A section's fixed name length.
pub const NAME_LEN: usize = 16;

/// The section alignment.
pub const SECTION_ALIGN: u32 = 0x2000;

/// The sentinel entry's size: `8 + 8·count` at creation, left at 0x10.
pub const SENTINEL_SIZE: u32 = 0x10;

/// The story `#playerdata` signature.
pub const STORY_SIGNATURE: u32 = 0x6F5A_EA14;

/// The arena `#playerdata` signature.
pub const ARENA_SIGNATURE: u32 = 0xE8AA_EA10;

/// The arena's current data version (the low byte of the saved word).
pub const ARENA_DATA_VERSION: u32 = 9;

/// The money word a new arena's save carries: the value the original's
/// writable global starts at. The arena loader copies a save's word back
/// into that global, so every later save writes the loaded value.
pub const NEW_ARENA_MONEY: u32 = 515;

/// The story writer always sets this bit in the flags word: the "started"
/// byte at bits 16 through 23 becomes 1.
pub const STORY_STARTED_FLAG: u32 = 0x1_0000;

/// A placed character's record word the story reader patches when the save
/// carries a start background of zero.
pub const CHARACTER_START_WORD_OFFSET: usize = 0x180;

/// The value the story reader writes at [`CHARACTER_START_WORD_OFFSET`] when
/// it is zero and the start background is zero.
pub const CHARACTER_START_WORD: u32 = 0x51_0000;

/// The title the story reader gives the journal's single default page.
pub const DEFAULT_JOURNAL_TITLE: &str = "New Entry";

/// The section holding the mode's player data.
pub const PLAYER_DATA: &[u8; NAME_LEN] = b"#playerdata\0\0\0\0\0";

/// The section holding the world object database.
pub const WORLD_OBJECT_DB: &[u8; NAME_LEN] = b"#worldobjdb\0\0\0\0\0";

/// The section holding the world character database.
pub const WORLD_CHARACTER_DB: &[u8; NAME_LEN] = b"#worldchrdb\0\0\0\0\0";

/// The chunk that opens the world object database export.
pub const WORLD_OBJECTS_CHUNK: u32 = 0xDBDB;

/// The chunk that follows it with the data manager's own table.
pub const DATA_MANAGER_CHUNK: u32 = 0xDBCD;

/// The chunk that opens the world character database export.
pub const WORLD_CHARACTERS_CHUNK: u32 = 0xCECD_DB00;

/// The world object database's RDB magic, the tag the compatibility check
/// compares.
pub const WORLD_OBJECT_DB_MAGIC: u32 = 0xDBAC_0D06;

/// The story slot file prefix.
pub const PREFIX_STORY: &str = "Exanima";

/// The arena slot file prefix.
pub const PREFIX_ARENA: &str = "Arena";

/// The longest prefix the name builder copies: the original copies the
/// prefix into a 16-byte buffer at most fifteen bytes long.
pub const PREFIX_MAX: usize = 15;

/// A save's extension.
pub const EXT_SAVE: &str = ".rsg";

/// A checkpoint's extension.
pub const EXT_CHECKPOINT: &str = ".rcp";

/// Which of a slot's two files it is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlotKind {
    /// The save itself (`.rsg`).
    Save,
    /// The death-reload point (`.rcp`).
    Checkpoint,
}

impl SlotKind {
    /// The extension, including its dot.
    pub fn extension(self) -> &'static str {
        match self {
            SlotKind::Save => EXT_SAVE,
            SlotKind::Checkpoint => EXT_CHECKPOINT,
        }
    }
}

/// Builds `<prefix><NNN><extension>` with the save number padded to three
/// digits. The prefix is cut to its first fifteen bytes, as the original's
/// name builder copies it into a fifteen-byte buffer. Numbers above 999 are
/// left whole, which the scanner never takes as a slot.
pub fn slot_file_name(prefix: &str, number: u32, kind: SlotKind) -> String {
    let bytes = prefix.as_bytes();
    let prefix = String::from_utf8_lossy(&bytes[..bytes.len().min(PREFIX_MAX)]);
    format!(
        "{prefix}{}{}",
        crate::save_chunk::format_save_number(number.clamp(0, i32::MAX as u32) as i32),
        kind.extension()
    )
}

/// The number the original's scanner reads from three bytes: no digit check,
/// just `(b0 − '0')·100 + (b1 − '0')·10 + (b2 − '0')` as a signed 32-bit
/// value.
pub fn slot_number(bytes: &[u8]) -> i32 {
    let digit = |index: usize| i32::from(bytes.get(index).copied().unwrap_or(0)) - 0x30;
    digit(0) * 100 + digit(1) * 10 + digit(2)
}

/// Whether two byte slices are equal ignoring ASCII case, as the scanner's
/// file-name compares do.
fn name_eq(left: &[u8], right: &[u8]) -> bool {
    left.len() == right.len()
        && left
            .iter()
            .zip(right)
            .all(|(a, b)| a.eq_ignore_ascii_case(b))
}

/// Reads a slot file name: the prefix (any ASCII case) plus exactly three
/// bytes holding a number from 1 through 999, then `.rsg` or `.rcp` (any
/// ASCII case). Anything else is not a slot.
pub fn parse_slot_file(file_name: &str, prefix: &str) -> Option<(u32, SlotKind)> {
    let bytes = file_name.as_bytes();
    let prefix = prefix.as_bytes();
    if bytes.len() != prefix.len() + 7 || !name_eq(&bytes[..prefix.len()], prefix) {
        return None;
    }
    let number = slot_number(&bytes[prefix.len()..]);
    if !(1..1000).contains(&number) {
        return None;
    }
    let extension = &bytes[prefix.len() + 3..];
    if name_eq(extension, EXT_SAVE.as_bytes()) {
        Some((number as u32, SlotKind::Save))
    } else if name_eq(extension, EXT_CHECKPOINT.as_bytes()) {
        Some((number as u32, SlotKind::Checkpoint))
    } else {
        None
    }
}

/// One section of a parsed (borrowed) package.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Section<'a> {
    pub name: [u8; NAME_LEN],
    pub payload: &'a [u8],
}

/// A parsed package.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Package<'a> {
    pub magic: u32,
    pub sections: Vec<Section<'a>>,
}

/// A package parse error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackageError {
    Short,
    BadMagic(u32),
    BadDirectory,
    SectionOutOfRange,
}

impl Package<'_> {
    /// Finds a section by its exact 16-byte name.
    pub fn section(&self, name: &[u8; NAME_LEN]) -> Option<&[u8]> {
        self.sections
            .iter()
            .find(|section| section.name == *name)
            .map(|section| section.payload)
    }

    /// Whether the package holds a section with this name.
    pub fn has_section(&self, name: &[u8; NAME_LEN]) -> bool {
        self.section(name).is_some()
    }
}

/// Parses a package. Entry 0 is the sentinel and is not exposed. A count of
/// zero or a negative count (the word is signed) reads as an empty package,
/// as the original's entry loop then runs no times.
pub fn parse(bytes: &[u8]) -> Result<Package<'_>, PackageError> {
    if bytes.len() < 8 {
        return Err(PackageError::Short);
    }
    let magic = u32::from_le_bytes(bytes[0..4].try_into().unwrap());
    if magic != PACKAGE_MAGIC {
        return Err(PackageError::BadMagic(magic));
    }
    let count = u32::from_le_bytes(bytes[4..8].try_into().unwrap()) as i32;
    // The count is signed in the original. A zero or negative count makes
    // its entry loop run no times, so the package has no sections.
    let count = count.max(0) as usize;
    let directory_end = 8usize
        .checked_add(count.checked_mul(24).ok_or(PackageError::BadDirectory)?)
        .ok_or(PackageError::BadDirectory)?;
    if directory_end > bytes.len() {
        return Err(PackageError::BadDirectory);
    }
    let mut sections = Vec::with_capacity(count.saturating_sub(1));
    for index in 1..count {
        let at = 8 + index * 24;
        let offset = u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap()) as usize;
        let size = u32::from_le_bytes(bytes[at + 4..at + 8].try_into().unwrap()) as usize;
        let mut name = [0u8; NAME_LEN];
        name.copy_from_slice(&bytes[at + 8..at + 24]);
        let end = offset
            .checked_add(size)
            .ok_or(PackageError::SectionOutOfRange)?;
        if end > bytes.len() {
            return Err(PackageError::SectionOutOfRange);
        }
        sections.push(Section {
            name,
            payload: &bytes[offset..end],
        });
    }
    Ok(Package { magic, sections })
}

/// One section of an owned package.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// Where the payload starts in the file.
    pub offset: u32,
    /// The section's name.
    pub name: [u8; NAME_LEN],
    /// The payload.
    pub data: Vec<u8>,
}

/// A package that can be edited and written back, with the original's
/// placement rules.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageImage {
    pub magic: u32,
    /// The sentinel entry's offset, size and name, kept as read.
    sentinel: (u32, u32, [u8; NAME_LEN]),
    entries: Vec<Entry>,
    /// The file as read, kept so that bytes no entry covers (gaps, a
    /// deleted section's payload, an old tail) survive a write, as the
    /// original leaves them and never truncates.
    original: Vec<u8>,
}

impl PackageImage {
    /// An empty package (the sentinel only).
    pub fn new() -> Self {
        Self {
            magic: PACKAGE_MAGIC,
            sentinel: (0, SENTINEL_SIZE, [0; NAME_LEN]),
            entries: Vec::new(),
            original: Vec::new(),
        }
    }

    /// Reads a package whole, keeping its directory order and offsets.
    pub fn read(bytes: &[u8]) -> Result<Self, PackageError> {
        let package = parse(bytes)?;
        // Rebuild from the directory so the offsets survive a round trip.
        let count = u32::from_le_bytes(bytes[4..8].try_into().unwrap()) as i32;
        let count = count.max(0) as usize;
        let mut sentinel = (0, SENTINEL_SIZE, [0u8; NAME_LEN]);
        if count >= 1 {
            let mut sentinel_name = [0u8; NAME_LEN];
            sentinel_name.copy_from_slice(&bytes[16..32]);
            sentinel = (
                u32::from_le_bytes(bytes[8..12].try_into().unwrap()),
                u32::from_le_bytes(bytes[12..16].try_into().unwrap()),
                sentinel_name,
            );
        }
        let mut entries = Vec::with_capacity(count.saturating_sub(1));
        for index in 1..count {
            let at = 8 + index * 24;
            let offset = u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap());
            let size = u32::from_le_bytes(bytes[at + 4..at + 8].try_into().unwrap()) as usize;
            let mut name = [0u8; NAME_LEN];
            name.copy_from_slice(&bytes[at + 8..at + 24]);
            entries.push(Entry {
                offset,
                name,
                data: bytes[offset as usize..offset as usize + size].to_vec(),
            });
        }
        Ok(Self {
            magic: package.magic,
            sentinel,
            entries,
            original: bytes.to_vec(),
        })
    }

    /// The sections in directory order.
    pub fn entries(&self) -> &[Entry] {
        &self.entries
    }

    /// A section's payload.
    pub fn section(&self, name: &[u8; NAME_LEN]) -> Option<&[u8]> {
        self.entries
            .iter()
            .find(|entry| entry.name == *name)
            .map(|entry| entry.data.as_slice())
    }

    /// Whether a section exists.
    pub fn has_section(&self, name: &[u8; NAME_LEN]) -> bool {
        self.section(name).is_some()
    }

    /// An entry's offset and size, the sentinel being index 0 as in the
    /// file's directory.
    fn slot(&self, index: usize) -> (u32, u32) {
        match index.checked_sub(1) {
            None => (self.sentinel.0, self.sentinel.1),
            Some(at) => (self.entries[at].offset, self.entries[at].data.len() as u32),
        }
    }

    /// Where a section of `size` bytes goes, as a directory index counting
    /// the sentinel as 0. The original walks every pair of consecutive
    /// entries, the sentinel included, and keeps the **last** pair whose room
    /// (the next offset minus this offset minus this size rounded up to the
    /// alignment, in wrapping 32-bit arithmetic) is strictly larger than
    /// `size`; the new entry goes after that pair's first entry. With no such
    /// pair it is appended.
    fn gap_index(&self, size: u32) -> usize {
        let count = self.entries.len() + 1;
        let mut found = count;
        for index in 0..count.saturating_sub(1) {
            let (here, used) = self.slot(index);
            let (next, _) = self.slot(index + 1);
            let room = next.wrapping_sub(here).wrapping_sub(round_up(used));
            if size < room {
                found = index + 1;
            }
        }
        found
    }

    /// The offset an entry at directory index `index` (sentinel = 0) is
    /// given: the entry before it plus its size, rounded up.
    fn offset_after(&self, index: usize) -> u32 {
        let (offset, size) = self.slot(index - 1);
        round_up(offset.wrapping_add(size))
    }

    /// Writes a section with the original's rules. A section that is not
    /// there is inserted at the gap index. An existing section that is not
    /// the last one and whose room up to the next entry (that offset minus
    /// its own) is below the new size is moved to the gap index, the entries
    /// between shifting over it. Whatever the case, the written entry's
    /// offset is then recomputed from the entry before it, so a section
    /// rewritten "in place" after a predecessor shrank moves down to the
    /// predecessor's new aligned end.
    pub fn put_section(&mut self, name: &[u8; NAME_LEN], data: &[u8]) {
        let size = data.len() as u32;
        let count = self.entries.len() + 1;
        let index = match self.entries.iter().position(|entry| entry.name == *name) {
            None => {
                let index = self.gap_index(size);
                self.entries.insert(
                    index - 1,
                    Entry {
                        offset: 0,
                        name: *name,
                        data: Vec::new(),
                    },
                );
                index
            }
            Some(at) => {
                let index = at + 1;
                let (here, _) = self.slot(index);
                let room = if index + 1 < count {
                    Some(self.slot(index + 1).0.wrapping_sub(here))
                } else {
                    None
                };
                match room {
                    Some(room) if room < size => {
                        let target = self.gap_index(size);
                        let entry = self.entries.remove(at);
                        let index = if index < target { target - 1 } else { target };
                        self.entries.insert(index - 1, entry);
                        index
                    }
                    _ => index,
                }
            }
        };
        let offset = self.offset_after(index);
        let entry = &mut self.entries[index - 1];
        entry.offset = offset;
        entry.data = data.to_vec();
    }

    /// Removes a section, leaving its bytes in place as the original does.
    pub fn remove_section(&mut self, name: &[u8; NAME_LEN]) -> bool {
        match self.entries.iter().position(|entry| entry.name == *name) {
            Some(at) => {
                self.entries.remove(at);
                true
            }
            None => false,
        }
    }

    /// The package's bytes: the header and directory written over the file
    /// as read, then every payload at its offset. Bytes no entry covers stay
    /// as they were and the file never shrinks below the bytes read, as the
    /// original's writer leaves gaps, deleted payloads and an old tail in
    /// place. A fresh image has zeroes in its gaps.
    pub fn to_bytes(&self) -> Vec<u8> {
        let count = self.entries.len() as u32 + 1;
        let header_len = 8 + (self.entries.len() + 1) * 24;
        let end = self
            .entries
            .iter()
            .map(|entry| entry.offset as usize + entry.data.len())
            .max()
            .unwrap_or(0);
        let mut out = self.original.clone();
        out.resize(out.len().max(header_len).max(end), 0);
        out[0..4].copy_from_slice(&self.magic.to_le_bytes());
        out[4..8].copy_from_slice(&count.to_le_bytes());
        out[8..12].copy_from_slice(&self.sentinel.0.to_le_bytes());
        out[12..16].copy_from_slice(&self.sentinel.1.to_le_bytes());
        out[16..32].copy_from_slice(&self.sentinel.2);
        for (index, entry) in self.entries.iter().enumerate() {
            let at = 8 + (index + 1) * 24;
            out[at..at + 4].copy_from_slice(&entry.offset.to_le_bytes());
            out[at + 4..at + 8].copy_from_slice(&(entry.data.len() as u32).to_le_bytes());
            out[at + 8..at + 24].copy_from_slice(&entry.name);
        }
        for entry in &self.entries {
            let at = entry.offset as usize;
            out[at..at + entry.data.len()].copy_from_slice(&entry.data);
        }
        out
    }
}

impl Default for PackageImage {
    fn default() -> Self {
        Self::new()
    }
}

/// Rounds up to the section alignment in wrapping 32-bit arithmetic, as the
/// original's directory words are.
fn round_up(value: u32) -> u32 {
    value.wrapping_add(SECTION_ALIGN - 1) & !(SECTION_ALIGN - 1)
}

/// Builds a package the way a first write does: a sentinel entry, then each
/// section at the next 0x2000 boundary.
pub fn write_package(magic: u32, sections: &[(&[u8; NAME_LEN], &[u8])]) -> Vec<u8> {
    let mut package = PackageImage::new();
    package.magic = magic;
    for (name, payload) in sections {
        package.put_section(name, payload);
    }
    package.to_bytes()
}

/// One framed chunk: `u32 id, u32 len, payload`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Chunk<'a> {
    pub id: u32,
    pub payload: &'a [u8],
}

/// An owned chunk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnedChunk {
    /// The chunk's tag.
    pub id: u32,
    /// The payload.
    pub data: Vec<u8>,
}

/// Parses consecutive chunks, skipping no bytes. A header that fits but
/// whose payload does not is an error.
pub fn parse_chunks(bytes: &[u8]) -> Result<Vec<Chunk<'_>>, PackageError> {
    let mut out = Vec::new();
    let mut at = 0;
    while at < bytes.len() {
        if at + 8 > bytes.len() {
            return Err(PackageError::Short);
        }
        let id = u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap());
        let len = u32::from_le_bytes(bytes[at + 4..at + 8].try_into().unwrap()) as usize;
        let start = at + 8;
        let end = start.checked_add(len).ok_or(PackageError::Short)?;
        if end > bytes.len() {
            return Err(PackageError::Short);
        }
        out.push(Chunk {
            id,
            payload: &bytes[start..end],
        });
        at = end;
    }
    Ok(out)
}

/// Writes one chunk.
pub fn write_chunk(out: &mut Vec<u8>, id: u32, payload: &[u8]) {
    out.extend_from_slice(&id.to_le_bytes());
    out.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    out.extend_from_slice(payload);
}

/// Which mode wrote a player-data section.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Flavour {
    Story,
    Arena,
}

/// The fixed 52-byte story header.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StoryHeader {
    /// The level the player was in (`exanima01` … `exanima08`).
    pub locale: [u8; NAME_LEN],
    /// The camera's target (cm).
    pub camera: [f32; 3],
    /// The camera's zoom.
    pub zoom: f32,
    /// The seed drawn when the save was created.
    pub seed: u32,
    /// The low byte is the start background; bit 16 is set by every writer.
    pub flags: u32,
    /// The game clock in seconds.
    pub clock: u32,
    /// A word the writer leaves at zero.
    pub reserved: u32,
}

/// The fixed 52-byte arena header.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ArenaHeader {
    /// The level the player was in (`arenahub`, `arena`, `arenae`).
    pub locale: [u8; NAME_LEN],
    /// The camera's target (cm).
    pub camera: [f32; 3],
    /// The camera's zoom.
    pub zoom: f32,
    /// The company's copper.
    pub money: u32,
    /// The arena data version; the loader compares its low byte with
    /// [`ARENA_DATA_VERSION`].
    pub data_version: u32,
    /// The game clock in seconds.
    pub clock: u32,
    /// The day counter.
    pub day: u32,
}

/// Either mode's header.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PlayerHeader {
    Story(StoryHeader),
    Arena(ArenaHeader),
}

impl PlayerHeader {
    /// Which mode wrote it.
    pub fn flavour(&self) -> Flavour {
        match self {
            PlayerHeader::Story(_) => Flavour::Story,
            PlayerHeader::Arena(_) => Flavour::Arena,
        }
    }

    /// The locale name.
    pub fn locale(&self) -> [u8; NAME_LEN] {
        match self {
            PlayerHeader::Story(header) => header.locale,
            PlayerHeader::Arena(header) => header.locale,
        }
    }

    /// The game clock in seconds.
    pub fn clock(&self) -> u32 {
        match self {
            PlayerHeader::Story(header) => header.clock,
            PlayerHeader::Arena(header) => header.clock,
        }
    }
}

/// A player-data error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerDataError {
    /// Fewer than the 52 header bytes.
    Short,
    /// The signature is not this mode's, or older than the story's.
    Signature(u32),
    /// Fewer than eight bytes where the character or a chunk header was due.
    Truncated,
}

/// The player's saved character placement, as the mode writes it: one chunk
/// whose payload the level exporter defines.
pub const CHARACTER_TAG: u32 = 0xCAC0_CA00;

/// The reference bit that sends the record lookup to the world character
/// database.
pub const CHARACTER_IN_WORLD_DB: u32 = 0x4000_0000;

/// Where an old arena save puts the player back.
pub const OLD_ARENA_POSITION: [f32; 3] = [0.0, 0.0, 300.0];

/// The player's saved placement, the payload of the character chunk, laid
/// out as the level exporter's placement writer writes it
/// ([`crate::rfc_write::write_character_state`]): the record reference, the
/// position (body x and z, floor height as y), the heading, the condition,
/// then sub-chunks the scene import picks up by id (the flat pose
/// `0xCAC0E100`, the motion state `0xCAC0EA00`, the pose tree `0xCAC0EB00`,
/// the attached objects `0xCAC0EC00` and the AI home `0xCACE0F77`; both
/// retail saves observed hold the flat pose and the motion state).
#[derive(Debug, Clone, PartialEq)]
pub struct CharacterPlacement {
    /// See [`PlayerData::character_reference`].
    pub reference: u32,
    /// The saved position (cm).
    pub position: [f32; 3],
    /// The heading in radians.
    pub heading: f32,
    /// The character's condition word (zero for a living player in both
    /// observed saves).
    pub condition: i32,
    /// The sub-chunks in order.
    pub chunks: Vec<OwnedChunk>,
}

impl CharacterPlacement {
    /// Reads a placement payload. The sub-chunks are read while their
    /// header fits.
    pub fn parse(bytes: &[u8]) -> Result<Self, PlayerDataError> {
        let mut at = 0;
        let reference = take_u32(bytes, &mut at)?;
        let position = [
            take_f32(bytes, &mut at)?,
            take_f32(bytes, &mut at)?,
            take_f32(bytes, &mut at)?,
        ];
        let heading = take_f32(bytes, &mut at)?;
        let condition = take_u32(bytes, &mut at)? as i32;
        let mut chunks = Vec::new();
        while at < bytes.len() {
            let id = take_u32(bytes, &mut at)?;
            let size = take_u32(bytes, &mut at)? as usize;
            chunks.push(OwnedChunk {
                id,
                data: take(bytes, &mut at, size)?.to_vec(),
            });
        }
        Ok(Self {
            reference,
            position,
            heading,
            condition,
            chunks,
        })
    }

    /// The payload's bytes.
    pub fn write(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&self.reference.to_le_bytes());
        for value in self.position {
            out.extend_from_slice(&value.to_le_bytes());
        }
        out.extend_from_slice(&self.heading.to_le_bytes());
        out.extend_from_slice(&self.condition.to_le_bytes());
        for chunk in &self.chunks {
            write_chunk(&mut out, chunk.id, &chunk.data);
        }
        out
    }
}

/// The journal chunk: a count and, per entry, an id and two strings.
pub const JOURNAL_CHUNK: u32 = 0x10;
/// The discovery chunk (the experience system's own format).
pub const DISCOVERY_CHUNK: u32 = 0x100;
/// The global experience snapshot.
pub const GLOBAL_XP_CHUNK: u32 = 0x100_000;
/// The memory-fragment flags.
pub const MEMORY_FLAGS_CHUNK: u32 = 0xD000_C001;
/// The 16-byte portal/respawn state.
pub const TRANSITION_CHUNK: u32 = 0xD000_C003;
/// The arena's hireable characters.
pub const HIREABLES_CHUNK: u32 = 0x10_000;
/// The arena's five-by-five arsenal.
pub const ARSENAL_CHUNK: u32 = 0xA010_0000;
/// The hub stash.
pub const HUB_STASH_CHUNK: u32 = 0xA020_0000;
/// The shop's stock ids.
pub const SHOP_STOCK_CHUNK: u32 = 0xA030_0000;
/// The company's employees.
pub const EMPLOYEES_CHUNK: u32 = 0xA040_0000;
/// The merchant's stock and specialisation.
pub const MERCHANT_CHUNK: u32 = 0xA050_0000;
/// The tournament state.
pub const TOURNAMENT_CHUNK: u32 = 0xA060_0000;
/// A chunk the arena reads and writes without a traced meaning.
pub const ARENA_EXTRA_CHUNK: u32 = 0xA0D1_0000;
/// The company record.
pub const COMPANY_CHUNK: u32 = 0xA0DC_0000;

/// The order the story's writer emits the chunks it knows: the placement
/// first, then discovery, the journal when it has pages, the global
/// experience, the memory flags and the transition state.
pub const STORY_CHUNK_ORDER: [u32; 5] = [
    DISCOVERY_CHUNK,
    JOURNAL_CHUNK,
    GLOBAL_XP_CHUNK,
    MEMORY_FLAGS_CHUNK,
    TRANSITION_CHUNK,
];

/// The order the arena's writer emits the chunks it knows: the placement
/// first, then the journal, the arsenal, the stash, the shop stock and the
/// arena tables. The observed retail arena save holds the arsenal, the shop
/// stock, the company record, the day record, the merchant, the hireables
/// and the tournament in that order; the employees' place among the tables
/// is not observed and follows their id here.
pub const ARENA_CHUNK_ORDER: [u32; 10] = [
    JOURNAL_CHUNK,
    ARSENAL_CHUNK,
    HUB_STASH_CHUNK,
    SHOP_STOCK_CHUNK,
    EMPLOYEES_CHUNK,
    COMPANY_CHUNK,
    ARENA_EXTRA_CHUNK,
    MERCHANT_CHUNK,
    HIREABLES_CHUNK,
    TOURNAMENT_CHUNK,
];

/// Exarust's own session payload inside `#playerdata`. A guess: no retail
/// chunk carries it, and the original ignores ids it does not know.
pub const EXARUST_CHUNK: u32 = 0xD0E8_0001;

/// One journal entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JournalRecord {
    /// The entry's id; Exarust's journal has none and writes its index.
    pub id: i32,
    /// The title.
    pub title: String,
    /// The body.
    pub text: String,
}

fn take<'a>(bytes: &'a [u8], at: &mut usize, n: usize) -> Result<&'a [u8], PlayerDataError> {
    let end = at.checked_add(n).ok_or(PlayerDataError::Truncated)?;
    let slice = bytes.get(*at..end).ok_or(PlayerDataError::Truncated)?;
    *at = end;
    Ok(slice)
}

fn take_u32(bytes: &[u8], at: &mut usize) -> Result<u32, PlayerDataError> {
    Ok(u32::from_le_bytes(take(bytes, at, 4)?.try_into().unwrap()))
}

fn take_f32(bytes: &[u8], at: &mut usize) -> Result<f32, PlayerDataError> {
    Ok(f32::from_le_bytes(take(bytes, at, 4)?.try_into().unwrap()))
}

fn take_name(bytes: &[u8], at: &mut usize) -> Result<[u8; NAME_LEN], PlayerDataError> {
    let mut name = [0u8; NAME_LEN];
    name.copy_from_slice(take(bytes, at, NAME_LEN)?);
    Ok(name)
}

/// A `#playerdata` section: the header, the optional character placement and
/// the tagged chunks in their written order.
#[derive(Debug, Clone, PartialEq)]
pub struct PlayerData {
    pub header: PlayerHeader,
    /// The character placement chunk's payload. The original always writes
    /// one; Exarust's own saves leave it out until it has a character.
    pub character: Option<Vec<u8>>,
    /// Every chunk after the character, in order, known or not.
    pub chunks: Vec<OwnedChunk>,
}

/// How a reader chooses the character placement chunk.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CharacterRule {
    /// The original's reader: the first chunk after the header is the
    /// character, whatever its id.
    FirstChunk,
    /// Only a chunk tagged [`CHARACTER_TAG`] is the character. Exarust's
    /// own saves leave the placement out, so their first chunk is the
    /// journal; this rule keeps those saves readable. It is a deliberate
    /// deviation from the original, which has no such case because it
    /// always writes a placement.
    Tagged,
}

impl PlayerData {
    /// An empty story player data for `locale`.
    pub fn story(locale: [u8; NAME_LEN]) -> Self {
        Self {
            header: PlayerHeader::Story(StoryHeader {
                locale,
                camera: [0.0; 3],
                zoom: 1.0,
                seed: 0,
                flags: STORY_STARTED_FLAG,
                clock: 0,
                reserved: 0,
            }),
            character: None,
            chunks: Vec::new(),
        }
    }

    /// An empty arena player data for `locale`.
    pub fn arena(locale: [u8; NAME_LEN]) -> Self {
        Self {
            header: PlayerHeader::Arena(ArenaHeader {
                locale,
                camera: [0.0; 3],
                zoom: 1.0,
                money: NEW_ARENA_MONEY,
                data_version: ARENA_DATA_VERSION,
                clock: 0,
                day: 0,
            }),
            character: None,
            chunks: Vec::new(),
        }
    }

    /// Reads a `#playerdata` section with the given mode's signature rule
    /// and Exarust's tagged-character rule (see [`PlayerData::parse_retail`]
    /// for the original's).
    ///
    /// The story reader accepts any signature at or above its own, so a
    /// newer save still loads; the arena reader takes its signature exactly.
    /// The character is read first when the section starts with its chunk
    /// (the original takes the first chunk after the header as the
    /// character whatever its id; Exarust only does so for the character's
    /// own id, because its own saves may leave the placement out). The rest
    /// are read while **more** than eight bytes remain, as the original's
    /// loop does, so a tail of up to eight bytes (an empty chunk at the very
    /// end included) is ignored.
    pub fn parse(bytes: &[u8], flavour: Flavour) -> Result<Self, PlayerDataError> {
        Self::parse_with(bytes, flavour, CharacterRule::Tagged)
    }

    /// Reads a `#playerdata` section exactly as the original does: the first
    /// chunk after the 52-byte header is the character placement whatever
    /// its id. A section with no chunk at all has no character and is what
    /// the original's loader refuses.
    pub fn parse_retail(bytes: &[u8], flavour: Flavour) -> Result<Self, PlayerDataError> {
        Self::parse_with(bytes, flavour, CharacterRule::FirstChunk)
    }

    /// Reads a `#playerdata` section with an explicit character rule.
    pub fn parse_with(
        bytes: &[u8],
        flavour: Flavour,
        rule: CharacterRule,
    ) -> Result<Self, PlayerDataError> {
        let mut at = 0;
        let signature = take_u32(bytes, &mut at)?;
        let header = match flavour {
            Flavour::Story => {
                if signature < STORY_SIGNATURE {
                    return Err(PlayerDataError::Signature(signature));
                }
                PlayerHeader::Story(StoryHeader {
                    locale: take_name(bytes, &mut at)?,
                    camera: [
                        take_f32(bytes, &mut at)?,
                        take_f32(bytes, &mut at)?,
                        take_f32(bytes, &mut at)?,
                    ],
                    zoom: take_f32(bytes, &mut at)?,
                    seed: take_u32(bytes, &mut at)?,
                    flags: take_u32(bytes, &mut at)?,
                    clock: take_u32(bytes, &mut at)?,
                    reserved: take_u32(bytes, &mut at)?,
                })
            }
            Flavour::Arena => {
                if signature != ARENA_SIGNATURE {
                    return Err(PlayerDataError::Signature(signature));
                }
                PlayerHeader::Arena(ArenaHeader {
                    locale: take_name(bytes, &mut at)?,
                    camera: [
                        take_f32(bytes, &mut at)?,
                        take_f32(bytes, &mut at)?,
                        take_f32(bytes, &mut at)?,
                    ],
                    zoom: take_f32(bytes, &mut at)?,
                    money: take_u32(bytes, &mut at)?,
                    data_version: take_u32(bytes, &mut at)?,
                    clock: take_u32(bytes, &mut at)?,
                    day: take_u32(bytes, &mut at)?,
                })
            }
        };
        let mut character = None;
        if bytes.len() >= at + 8 {
            let id = u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap());
            if rule == CharacterRule::FirstChunk || id == CHARACTER_TAG {
                at += 4;
                let size = take_u32(bytes, &mut at)? as usize;
                character = Some(take(bytes, &mut at, size)?.to_vec());
            }
        }
        let mut chunks = Vec::new();
        while at + 8 < bytes.len() {
            let id = take_u32(bytes, &mut at)?;
            let size = take_u32(bytes, &mut at)? as usize;
            chunks.push(OwnedChunk {
                id,
                data: take(bytes, &mut at, size)?.to_vec(),
            });
        }
        Ok(Self {
            header,
            character,
            chunks,
        })
    }

    /// The section's bytes.
    pub fn write(&self) -> Vec<u8> {
        let mut out = Vec::new();
        match &self.header {
            PlayerHeader::Story(header) => {
                out.extend_from_slice(&STORY_SIGNATURE.to_le_bytes());
                out.extend_from_slice(&header.locale);
                for value in header.camera {
                    out.extend_from_slice(&value.to_le_bytes());
                }
                out.extend_from_slice(&header.zoom.to_le_bytes());
                out.extend_from_slice(&header.seed.to_le_bytes());
                out.extend_from_slice(&header.flags.to_le_bytes());
                out.extend_from_slice(&header.clock.to_le_bytes());
                out.extend_from_slice(&header.reserved.to_le_bytes());
            }
            PlayerHeader::Arena(header) => {
                out.extend_from_slice(&ARENA_SIGNATURE.to_le_bytes());
                out.extend_from_slice(&header.locale);
                for value in header.camera {
                    out.extend_from_slice(&value.to_le_bytes());
                }
                out.extend_from_slice(&header.zoom.to_le_bytes());
                out.extend_from_slice(&header.money.to_le_bytes());
                out.extend_from_slice(&header.data_version.to_le_bytes());
                out.extend_from_slice(&header.clock.to_le_bytes());
                out.extend_from_slice(&header.day.to_le_bytes());
            }
        }
        if let Some(character) = &self.character {
            write_chunk(&mut out, CHARACTER_TAG, character);
        }
        for chunk in &self.chunks {
            write_chunk(&mut out, chunk.id, &chunk.data);
        }
        out
    }

    /// The section's bytes with the chunks in the mode's writer order: the
    /// placement first, then the ids the writer knows in
    /// [`STORY_CHUNK_ORDER`] or [`ARENA_CHUNK_ORDER`], each in its existing
    /// order (a repeated id keeps its repeats), then every other chunk in
    /// the order it was read. A chunk the writer would not emit is still
    /// written: this only sorts, it does not invent or drop ids.
    pub fn write_ordered(&self) -> Vec<u8> {
        let order: &[u32] = match self.header.flavour() {
            Flavour::Story => &STORY_CHUNK_ORDER,
            Flavour::Arena => &ARENA_CHUNK_ORDER,
        };
        let mut sorted = self.clone();
        let mut rest = Vec::with_capacity(self.chunks.len());
        sorted.chunks.clear();
        for &id in order {
            for chunk in self.chunks.iter().filter(|chunk| chunk.id == id) {
                sorted.chunks.push(chunk.clone());
            }
        }
        for chunk in &self.chunks {
            if !order.contains(&chunk.id) {
                rest.push(chunk.clone());
            }
        }
        sorted.chunks.extend(rest);
        sorted.write()
    }

    /// The journal the reader ends up with: the stored chunk when there is
    /// one, otherwise the single default page the reader resets it to (id
    /// zero, title [`DEFAULT_JOURNAL_TITLE`], empty text). A malformed
    /// journal chunk reads as the default, as the original's reader leaves
    /// the reset journal in place.
    pub fn journal_or_default(&self) -> Result<Vec<JournalRecord>, PlayerDataError> {
        match self.journal() {
            Ok(Some(records)) => Ok(records),
            _ => Ok(vec![JournalRecord {
                id: 0,
                title: DEFAULT_JOURNAL_TITLE.to_string(),
                text: String::new(),
            }]),
        }
    }

    /// The global experience the reader ends up with: the chunk's first
    /// word, or zero when the chunk is absent (the write of the default
    /// worth, zero, at load). A short chunk reads as zero as well.
    pub fn global_experience(&self) -> u32 {
        self.chunk(GLOBAL_XP_CHUNK)
            .and_then(|bytes| bytes.get(0..4))
            .map(|word| u32::from_le_bytes(word.try_into().unwrap()))
            .unwrap_or(0)
    }

    /// The memory-fragment flags the reader ends up with: the chunk's first
    /// word, or zero when absent.
    pub fn memory_flags(&self) -> u32 {
        self.chunk(MEMORY_FLAGS_CHUNK)
            .and_then(|bytes| bytes.get(0..4))
            .map(|word| u32::from_le_bytes(word.try_into().unwrap()))
            .unwrap_or(0)
    }

    /// The portal/respawn state the reader ends up with: the chunk's first
    /// sixteen bytes, or all zeroes when absent.
    pub fn transition_state(&self) -> [u8; 16] {
        let mut state = [0u8; 16];
        if let Some(bytes) = self.chunk(TRANSITION_CHUNK) {
            let len = bytes.len().min(16);
            state[..len].copy_from_slice(&bytes[..len]);
        }
        state
    }

    /// The word the story reader writes into the player record when the
    /// save's start background is zero and the record's word at
    /// [`CHARACTER_START_WORD_OFFSET`] is zero. Returns the word the reader
    /// leaves at that offset.
    pub fn story_start_word(&self, record_word: u32) -> u32 {
        let background = match &self.header {
            PlayerHeader::Story(header) => header.flags & 0xFF,
            PlayerHeader::Arena(_) => return record_word,
        };
        if background == 0 && record_word == 0 {
            CHARACTER_START_WORD
        } else {
            record_word
        }
    }

    /// A chunk's payload.
    pub fn chunk(&self, id: u32) -> Option<&[u8]> {
        self.chunks
            .iter()
            .find(|chunk| chunk.id == id)
            .map(|chunk| chunk.data.as_slice())
    }

    /// Replaces a chunk, or appends it when it is not there.
    pub fn set_chunk(&mut self, id: u32, data: Vec<u8>) {
        match self.chunks.iter_mut().find(|chunk| chunk.id == id) {
            Some(chunk) => chunk.data = data,
            None => self.chunks.push(OwnedChunk { id, data }),
        }
    }

    /// Removes a chunk.
    pub fn remove_chunk(&mut self, id: u32) -> bool {
        let len = self.chunks.len();
        self.chunks.retain(|chunk| chunk.id != id);
        self.chunks.len() != len
    }

    /// The journal, when the section has one.
    pub fn journal(&self) -> Result<Option<Vec<JournalRecord>>, PlayerDataError> {
        match self.chunk(JOURNAL_CHUNK) {
            Some(bytes) => parse_journal(bytes).map(Some),
            None => Ok(None),
        }
    }

    /// The character reference the placement chunk starts with: bit 30
    /// picks the database the record is looked up in (set: the world
    /// character database the save's `#worldchrdb` restores; clear: the
    /// other character database), the low 28 bits are its id. A reference
    /// the lookup does not find makes the original's loader refuse the save.
    pub fn character_reference(&self) -> Option<u32> {
        let word = self.character.as_ref()?.get(0..4)?;
        Some(u32::from_le_bytes(word.try_into().ok()?))
    }

    /// The id of the player's character record, as the placement chunk
    /// stores it.
    pub fn character_record_id(&self) -> Option<u32> {
        self.character_reference().map(|word| word & 0x0FFF_FFFF)
    }

    /// Whether the player's record lives in the world character database
    /// (bit 30 of the reference).
    pub fn character_in_world_db(&self) -> Option<bool> {
        self.character_reference()
            .map(|word| word & CHARACTER_IN_WORLD_DB != 0)
    }

    /// The character placement, decoded.
    pub fn placement(&self) -> Option<Result<CharacterPlacement, PlayerDataError>> {
        self.character.as_deref().map(CharacterPlacement::parse)
    }

    /// The mask the character screen loads a save with: the journal and the
    /// global experience snapshot only.
    pub const CHARACTER_SCREEN_MASK: u32 = 0x10_0010;

    /// Applies a load mask: the original ands each chunk id with the mask,
    /// ignores every result at or below `0xF` and dispatches the rest on the
    /// **masked** id. The kept chunks therefore carry `id & mask` (with the
    /// full mask nothing changes; with a narrow one an unrelated id can land
    /// on a known one).
    pub fn apply_mask(&mut self, mask: u32) {
        self.chunks.retain_mut(|chunk| {
            chunk.id &= mask;
            chunk.id > 0xF
        });
    }

    /// Replaces the journal chunk; an empty journal removes it, as the
    /// original only writes the chunk when it has entries.
    pub fn set_journal(&mut self, records: &[JournalRecord]) {
        if records.is_empty() {
            self.remove_chunk(JOURNAL_CHUNK);
        } else {
            self.set_chunk(JOURNAL_CHUNK, write_journal(records));
        }
    }

    /// Whether the arena's saved data version is older than the current one
    /// (the low byte below [`ARENA_DATA_VERSION`]).
    pub fn arena_data_is_old(&self) -> bool {
        match &self.header {
            PlayerHeader::Arena(header) => (header.data_version & 0xFF) < ARENA_DATA_VERSION,
            PlayerHeader::Story(_) => false,
        }
    }

    /// The story loader forces the level to `exanima05` when the save was
    /// written mid-transition (the state word is 2).
    pub fn story_transition_locale(&self) -> Option<[u8; NAME_LEN]> {
        let state = self
            .chunk(TRANSITION_CHUNK)
            .and_then(|bytes| bytes.get(0..4))
            .map(|word| u32::from_le_bytes(word.try_into().unwrap()))?;
        if state == 2 {
            Some(name("exanima05"))
        } else {
            None
        }
    }

    /// The arena loader's version conversion: an old save drops the
    /// hireables and shop-stock chunks, which the loader skips, and puts the
    /// player's saved position back to `(0, 0, 300)`. The package also drops
    /// its `arenahub` section; that is the caller's step.
    pub fn convert_arena(&mut self) -> bool {
        if !self.arena_data_is_old() {
            return false;
        }
        self.remove_chunk(HIREABLES_CHUNK);
        self.remove_chunk(SHOP_STOCK_CHUNK);
        if let Some(character) = &mut self.character
            && let Some(position) = character.get_mut(4..16)
        {
            for (slot, value) in position
                .as_chunks_mut::<4>()
                .0
                .iter_mut()
                .zip(OLD_ARENA_POSITION)
            {
                *slot = value.to_le_bytes();
            }
        }
        true
    }
}

/// Writes a journal chunk: the count, then each id and its two strings.
pub fn write_journal(records: &[JournalRecord]) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&(records.len() as u32).to_le_bytes());
    for record in records {
        out.extend_from_slice(&record.id.to_le_bytes());
        write_string(&mut out, record.title.as_bytes());
        write_string(&mut out, record.text.as_bytes());
    }
    out
}

/// Writes a `#worldobjdb` section: the object database export chunk, then
/// the data manager's chunk, both always (the save writes the section only
/// when the object database has records).
pub fn world_object_db(objects: &[u8], data_manager: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    write_chunk(&mut out, WORLD_OBJECTS_CHUNK, objects);
    write_chunk(&mut out, DATA_MANAGER_CHUNK, data_manager);
    out
}

/// Writes a `#worldchrdb` section: one character database export chunk.
pub fn world_character_db(characters: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    write_chunk(&mut out, WORLD_CHARACTERS_CHUNK, characters);
    out
}

/// Reads a journal chunk. Strings are length-prefixed byte runs; invalid
/// UTF-8 is replaced rather than refused, as the original stores raw text.
pub fn parse_journal(bytes: &[u8]) -> Result<Vec<JournalRecord>, PlayerDataError> {
    let mut at = 0;
    // The count is a signed word; the original hands a negative one to a
    // dynamic-array resize, which makes the array empty, so no entry is
    // read.
    let count = take_u32(bytes, &mut at)? as i32;
    if count <= 0 {
        return Ok(Vec::new());
    }
    let count = count as usize;
    let mut records = Vec::with_capacity(count.min(bytes.len() / 8));
    for _ in 0..count {
        let id = take_u32(bytes, &mut at)? as i32;
        let title = take_string(bytes, &mut at)?;
        let text = take_string(bytes, &mut at)?;
        records.push(JournalRecord { id, title, text });
    }
    Ok(records)
}

fn write_string(out: &mut Vec<u8>, text: &[u8]) {
    out.extend_from_slice(&(text.len() as u32).to_le_bytes());
    out.extend_from_slice(text);
}

fn take_string(bytes: &[u8], at: &mut usize) -> Result<String, PlayerDataError> {
    let len = take_u32(bytes, at)? as usize;
    let raw = take(bytes, at, len)?;
    Ok(String::from_utf8_lossy(raw).into_owned())
}

/// Builds a fixed 16-byte name from text.
pub fn name(text: &str) -> [u8; NAME_LEN] {
    let mut out = [0u8; NAME_LEN];
    let bytes = text.as_bytes();
    out[..bytes.len().min(NAME_LEN)].copy_from_slice(&bytes[..bytes.len().min(NAME_LEN)]);
    out
}

/// Whether a story save cannot be started from, the check the character
/// creator runs while the save's started flag is clear: a save with an
/// `exanima06` section is from a newer campaign layout, and one with an
/// `exanima04` section is only usable when its world object database has
/// the current magic as its third word (the first word of the object
/// export chunk's payload). A missing database passes; a section too short
/// to hold the magic counts as a mismatch here, where the original reads
/// past the buffer.
pub fn story_start_incompatible(package: &PackageImage, started: bool) -> bool {
    if started {
        return false;
    }
    if package.has_section(&name("exanima06")) {
        return true;
    }
    if !package.has_section(&name("exanima04")) {
        return false;
    }
    match package.section(WORLD_OBJECT_DB) {
        Some(section) => match section.get(8..12) {
            Some(word) => u32::from_le_bytes(word.try_into().unwrap()) != WORLD_OBJECT_DB_MAGIC,
            None => true,
        },
        None => false,
    }
}

/// A 16-byte name as text, cut at the first zero.
pub fn name_text(name: &[u8; NAME_LEN]) -> String {
    let len = name.iter().position(|&byte| byte == 0).unwrap_or(NAME_LEN);
    String::from_utf8_lossy(&name[..len]).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_minimal_package_parses() {
        let package = PackageImage::new().to_bytes();
        assert_eq!(package.len(), 32);
        let parsed = parse(&package).unwrap();
        assert_eq!(parsed.magic, PACKAGE_MAGIC);
        assert!(parsed.sections.is_empty());
    }

    #[test]
    fn two_sections_are_aligned() {
        let bytes = write_package(
            PACKAGE_MAGIC,
            &[
                (&name("arenahub"), b"AB"),
                (&name("exanima04"), b"123456789"),
            ],
        );
        assert_eq!(&bytes[0..4], &PACKAGE_MAGIC.to_le_bytes());
        assert_eq!(u32::from_le_bytes(bytes[4..8].try_into().unwrap()), 3);
        let package = parse(&bytes).unwrap();
        assert_eq!(package.sections.len(), 2);
        assert_eq!(package.section(&name("arenahub")), Some(&b"AB"[..]));
        assert_eq!(package.section(&name("exanima04")), Some(&b"123456789"[..]));
        let first = u32::from_le_bytes(bytes[32..36].try_into().unwrap());
        let second = u32::from_le_bytes(bytes[56..60].try_into().unwrap());
        assert_eq!((first, second), (0x2000, 0x4000));
        assert_eq!(bytes.len(), 0x4009);
    }

    #[test]
    fn a_new_section_takes_a_gap_and_a_grown_one_moves_past_the_end() {
        let mut package = PackageImage::new();
        package.put_section(&name("c"), &[0u8; 0x10]);
        package.put_section(&name("b"), &[0u8; 0x10]);
        package.put_section(&name("a"), &[0u8; 0x10]);
        assert_eq!(
            package
                .entries()
                .iter()
                .map(|e| e.offset)
                .collect::<Vec<_>>(),
            [0x2000, 0x4000, 0x6000]
        );
        // Deleting only removes the directory entry; the bytes stay.
        assert!(package.remove_section(&name("b")));
        assert_eq!(package.entries().len(), 2);
        // A new section fits into the freed 0x2000 room before `a`.
        package.put_section(&name("d"), &[0u8; 0x1000]);
        assert_eq!(package.entries()[1].name, name("d"));
        assert_eq!(package.entries()[1].offset, 0x4000);
        // `c` no longer fits before `d`, so it moves past `a`.
        package.put_section(&name("c"), &[0u8; 0x2001]);
        assert_eq!(
            package
                .entries()
                .iter()
                .map(|e| (e.name, e.offset))
                .collect::<Vec<_>>(),
            [
                (name("d"), 0x4000),
                (name("a"), 0x6000),
                (name("c"), 0x8000)
            ]
        );
        let bytes = package.to_bytes();
        let parsed = PackageImage::read(&bytes).unwrap();
        assert_eq!(parsed.section(&name("c")), Some(&[0u8; 0x2001][..]));
        assert_eq!(parsed.entries()[2].offset, 0x8000);
        // The last section always rewrites in place, growing over the end.
        package.put_section(&name("c"), &[1u8; 0x3000]);
        assert_eq!(package.entries()[2].offset, 0x8000);
        assert_eq!(package.entries()[2].data.len(), 0x3000);
    }

    /// A package read from a hand-built directory of `(offset, size, name)`
    /// entries whose payloads are zeroes.
    fn laid_out(entries: &[(u32, u32, &str)]) -> PackageImage {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&PACKAGE_MAGIC.to_le_bytes());
        bytes.extend_from_slice(&(entries.len() as u32 + 1).to_le_bytes());
        bytes.extend_from_slice(&0u32.to_le_bytes());
        bytes.extend_from_slice(&SENTINEL_SIZE.to_le_bytes());
        bytes.extend_from_slice(&[0u8; NAME_LEN]);
        for (offset, size, label) in entries {
            bytes.extend_from_slice(&offset.to_le_bytes());
            bytes.extend_from_slice(&size.to_le_bytes());
            bytes.extend_from_slice(&name(label));
        }
        let end = entries
            .iter()
            .map(|(offset, size, _)| (offset + size) as usize)
            .max()
            .unwrap_or(0);
        bytes.resize(bytes.len().max(end), 0);
        PackageImage::read(&bytes).unwrap()
    }

    fn offsets(package: &PackageImage) -> Vec<(String, u32)> {
        package
            .entries()
            .iter()
            .map(|entry| (name_text(&entry.name), entry.offset))
            .collect()
    }

    #[test]
    fn the_room_after_the_sentinel_is_a_gap_too() {
        // The sentinel ends at 0x10, aligned 0x2000; the first section sits
        // at 0x4000, so 0x2000 bytes are free before it.
        let mut package = laid_out(&[(0x4000, 0x10, "d"), (0x6000, 0x10, "a")]);
        package.put_section(&name("e"), &[1; 0x100]);
        assert_eq!(
            offsets(&package),
            [
                ("e".to_string(), 0x2000),
                ("d".to_string(), 0x4000),
                ("a".to_string(), 0x6000)
            ]
        );
    }

    #[test]
    fn a_new_section_takes_the_last_gap_that_is_strictly_larger() {
        // Two 0x2000 gaps: after x (0x2000..0x4000 used by alignment, free to
        // 0x6000) and after y (free to 0xA000).
        let layout = [
            (0x2000, 0x10, "x"),
            (0x6000, 0x10, "y"),
            (0xA000, 0x10, "z"),
        ];
        let mut package = laid_out(&layout);
        package.put_section(&name("w"), &[1; 0x100]);
        // The later gap wins: after y, at y's aligned end.
        assert_eq!(package.entries()[2].name, name("w"));
        assert_eq!(package.entries()[2].offset, 0x8000);
        // A section exactly as large as the room does not fit and is
        // appended after z.
        let mut package = laid_out(&layout);
        package.put_section(&name("w"), &[1; 0x2000]);
        assert_eq!(package.entries()[3].name, name("w"));
        assert_eq!(package.entries()[3].offset, 0xC000);
    }

    #[test]
    fn a_rewrite_recomputes_the_offset_from_the_entry_before() {
        let mut package = laid_out(&[(0x2000, 0x3000, "a"), (0x6000, 0x10, "b")]);
        // `a` shrinks in place: it stays at the sentinel's aligned end.
        package.put_section(&name("a"), &[2; 0x10]);
        assert_eq!(package.entries()[0].offset, 0x2000);
        // `b` is rewritten "in place", which puts it at `a`'s new aligned
        // end, not where it was.
        package.put_section(&name("b"), &[3; 0x10]);
        assert_eq!(package.entries()[1].offset, 0x4000);
        let bytes = package.to_bytes();
        assert_eq!(&bytes[0x4000..0x4010], &[3; 0x10]);
    }

    #[test]
    fn a_read_package_keeps_its_sentinel() {
        let mut bytes = PackageImage::new().to_bytes();
        bytes[12..16].copy_from_slice(&0x18u32.to_le_bytes());
        bytes[16] = b'?';
        let mut package = PackageImage::read(&bytes).unwrap();
        assert_eq!(package.to_bytes(), bytes);
        // The first section follows the sentinel's own size.
        package.put_section(&name("s"), b"x");
        assert_eq!(package.entries()[0].offset, 0x2000);
    }

    #[test]
    fn an_out_of_order_directory_writes_each_section_at_its_offset() {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&PACKAGE_MAGIC.to_le_bytes());
        bytes.extend_from_slice(&3u32.to_le_bytes());
        bytes.extend_from_slice(&0u32.to_le_bytes());
        bytes.extend_from_slice(&SENTINEL_SIZE.to_le_bytes());
        bytes.extend_from_slice(&[0u8; NAME_LEN]);
        bytes.extend_from_slice(&0x4000u32.to_le_bytes());
        bytes.extend_from_slice(&2u32.to_le_bytes());
        bytes.extend_from_slice(&name("a"));
        bytes.extend_from_slice(&0x2000u32.to_le_bytes());
        bytes.extend_from_slice(&3u32.to_le_bytes());
        bytes.extend_from_slice(&name("b"));
        bytes.resize(0x2000, 0);
        bytes.extend_from_slice(b"abc");
        bytes.resize(0x4000, 0);
        bytes.extend_from_slice(b"AB");
        let image = PackageImage::read(&bytes).unwrap();
        let out = image.to_bytes();
        assert_eq!(&out[0x2000..0x2003], b"abc");
        assert_eq!(&out[0x4000..0x4002], b"AB");
        assert_eq!(out.len(), 0x4002);
        assert_eq!(PackageImage::read(&out).unwrap(), image);
    }

    #[test]
    fn a_bad_section_is_rejected() {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&PACKAGE_MAGIC.to_le_bytes());
        bytes.extend_from_slice(&2u32.to_le_bytes());
        bytes.extend_from_slice(&0u32.to_le_bytes());
        bytes.extend_from_slice(&0x10u32.to_le_bytes());
        bytes.extend_from_slice(&[0u8; 16]);
        bytes.extend_from_slice(&0x9999_9999u32.to_le_bytes());
        bytes.extend_from_slice(&1u32.to_le_bytes());
        bytes.extend_from_slice(&name("section"));
        assert_eq!(parse(&bytes), Err(PackageError::SectionOutOfRange));
        assert_eq!(parse(b""), Err(PackageError::Short));
        let mut wrong = bytes.clone();
        wrong[0] = 0;
        assert_eq!(parse(&wrong), Err(PackageError::BadMagic(0xDA7AEA00)));
    }

    #[test]
    fn chunks_round_trip() {
        let mut bytes = Vec::new();
        write_chunk(&mut bytes, 0x0010_0000, &[0x2A, 0, 0, 0]);
        let chunks = parse_chunks(&bytes).unwrap();
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].id, 0x0010_0000);
        assert_eq!(chunks[0].payload, &[0x2A, 0, 0, 0]);
        assert_eq!(parse_chunks(&[1, 2, 3]), Err(PackageError::Short));
    }

    #[test]
    fn a_count_of_zero_is_an_empty_package() {
        for count in [0u32, (-1i32) as u32] {
            let mut bytes = PACKAGE_MAGIC.to_le_bytes().to_vec();
            bytes.extend_from_slice(&count.to_le_bytes());
            let package = parse(&bytes).unwrap();
            assert!(package.sections.is_empty());
            let image = PackageImage::read(&bytes).unwrap();
            assert!(image.entries().is_empty());
            // Writing it back adds the sentinel the original's reset writes.
            let out = image.to_bytes();
            assert_eq!(u32::from_le_bytes(out[4..8].try_into().unwrap()), 1);
            assert_eq!(u32::from_le_bytes(out[12..16].try_into().unwrap()), 0x10);
        }
    }

    #[test]
    fn a_slot_name_cuts_the_prefix_at_fifteen_bytes() {
        assert_eq!(
            slot_file_name("Exanima", 1, SlotKind::Save),
            "Exanima001.rsg"
        );
        assert_eq!(
            slot_file_name("ABCDEFGHIJKLMNOPQRSTUV", 7, SlotKind::Checkpoint),
            "ABCDEFGHIJKLMNO007.rcp"
        );
        // Both extensions and the number rules are unchanged.
        assert_eq!(slot_file_name("Arena", 999, SlotKind::Save), "Arena999.rsg");
        assert_eq!(slot_file_name("Arena", 0, SlotKind::Save), "Arena000.rsg");
    }

    #[test]
    fn the_arena_header_starts_with_the_writers_money_word() {
        let data = PlayerData::arena(name("arenahub"));
        let PlayerHeader::Arena(header) = data.header else {
            unreachable!()
        };
        assert_eq!(header.money, NEW_ARENA_MONEY);
        assert_eq!(header.money, 515);
        assert_eq!(header.data_version, 9);
        let story = PlayerData::story(name("exanima01"));
        let PlayerHeader::Story(header) = story.header else {
            unreachable!()
        };
        assert_eq!(header.flags, STORY_STARTED_FLAG);
        assert_eq!(header.flags >> 16, 1);
    }

    #[test]
    fn the_story_writer_order_is_discovery_journal_xp_flags_transition() {
        let mut data = PlayerData::story(name("exanima01"));
        // Deliberately out of the writer's order, with an unknown id too.
        data.set_chunk(
            TRANSITION_CHUNK,
            [2u32, 0, 0, 0].map(u32::to_le_bytes).concat(),
        );
        data.set_chunk(EXARUST_CHUNK, vec![9]);
        data.set_chunk(GLOBAL_XP_CHUNK, 44u32.to_le_bytes().to_vec());
        data.set_chunk(JOURNAL_CHUNK, write_journal(&[]));
        data.set_chunk(MEMORY_FLAGS_CHUNK, 7u32.to_le_bytes().to_vec());
        data.set_chunk(DISCOVERY_CHUNK, vec![0; 8]);
        let ordered = data.write_ordered();
        let read = PlayerData::parse(&ordered, Flavour::Story).unwrap();
        assert_eq!(
            read.chunks.iter().map(|chunk| chunk.id).collect::<Vec<_>>(),
            [
                DISCOVERY_CHUNK,
                JOURNAL_CHUNK,
                GLOBAL_XP_CHUNK,
                MEMORY_FLAGS_CHUNK,
                TRANSITION_CHUNK,
                EXARUST_CHUNK
            ]
        );
        // The plain writer keeps the insertion order.
        assert_eq!(
            data.write()[52..56],
            TRANSITION_CHUNK.to_le_bytes(),
            "the first chunk is the one set first"
        );
    }

    #[test]
    fn the_arena_writer_order_puts_the_tables_after_shop_and_stash() {
        let mut data = PlayerData::arena(name("arenahub"));
        data.set_chunk(TOURNAMENT_CHUNK, vec![1]);
        data.set_chunk(HIREABLES_CHUNK, vec![2]);
        data.set_chunk(COMPANY_CHUNK, vec![3]);
        data.set_chunk(SHOP_STOCK_CHUNK, vec![4]);
        data.set_chunk(ARSENAL_CHUNK, vec![5]);
        data.set_chunk(JOURNAL_CHUNK, write_journal(&[]));
        data.set_chunk(EMPLOYEES_CHUNK, vec![6]);
        data.set_chunk(ARENA_EXTRA_CHUNK, vec![7]);
        data.set_chunk(MERCHANT_CHUNK, vec![8]);
        data.set_chunk(HUB_STASH_CHUNK, vec![9]);
        let ordered = data.write_ordered();
        let read = PlayerData::parse(&ordered, Flavour::Arena).unwrap();
        assert_eq!(
            read.chunks.iter().map(|chunk| chunk.id).collect::<Vec<_>>(),
            [
                JOURNAL_CHUNK,
                ARSENAL_CHUNK,
                HUB_STASH_CHUNK,
                SHOP_STOCK_CHUNK,
                EMPLOYEES_CHUNK,
                COMPANY_CHUNK,
                ARENA_EXTRA_CHUNK,
                MERCHANT_CHUNK,
                HIREABLES_CHUNK,
                TOURNAMENT_CHUNK
            ]
        );
    }

    #[test]
    fn an_absent_journal_reads_as_the_default_page() {
        let data = PlayerData::story(name("exanima01"));
        assert_eq!(
            data.journal_or_default().unwrap(),
            vec![JournalRecord {
                id: 0,
                title: "New Entry".to_string(),
                text: String::new(),
            }]
        );
        let mut with = data.clone();
        with.set_journal(&[JournalRecord {
            id: 3,
            title: "A".to_string(),
            text: "B".to_string(),
        }]);
        assert_eq!(with.journal_or_default().unwrap()[0].id, 3);
        // A journal chunk that does not parse leaves the default.
        let mut broken = data.clone();
        broken.set_chunk(JOURNAL_CHUNK, vec![1, 0, 0, 0]);
        assert_eq!(broken.journal_or_default().unwrap()[0].id, 0);
        // The other chunks' absent defaults: global worth 0, memory flags
        // 0, the transition state all zeroes.
        assert_eq!(data.global_experience(), 0);
        assert_eq!(data.memory_flags(), 0);
        assert_eq!(data.transition_state(), [0u8; 16]);
        let mut with = data.clone();
        with.set_chunk(GLOBAL_XP_CHUNK, 44u32.to_le_bytes().to_vec());
        with.set_chunk(MEMORY_FLAGS_CHUNK, 3u32.to_le_bytes().to_vec());
        with.set_chunk(
            TRANSITION_CHUNK,
            [2u32, 1, 0, 0].map(u32::to_le_bytes).concat(),
        );
        assert_eq!(with.global_experience(), 44);
        assert_eq!(with.memory_flags(), 3);
        assert_eq!(
            with.transition_state(),
            [2u32, 1, 0, 0].map(u32::to_le_bytes).concat()[..]
        );
        // A chunk shorter than its word reads as zero.
        let mut short = data.clone();
        short.set_chunk(GLOBAL_XP_CHUNK, vec![1]);
        short.set_chunk(MEMORY_FLAGS_CHUNK, vec![1]);
        short.set_chunk(TRANSITION_CHUNK, vec![2]);
        assert_eq!(short.global_experience(), 0);
        assert_eq!(short.memory_flags(), 0);
        assert_eq!(&short.transition_state()[..2], &[2, 0]);
    }

    #[test]
    fn the_story_reader_patches_the_record_word_for_a_zero_background() {
        let mut data = PlayerData::story(name("exanima01"));
        assert_eq!(data.story_start_word(0), CHARACTER_START_WORD);
        assert_eq!(data.story_start_word(5), 5);
        let PlayerHeader::Story(header) = &mut data.header else {
            unreachable!()
        };
        header.flags = 2;
        assert_eq!(data.story_start_word(0), 0);
        let arena = PlayerData::arena(name("arenahub"));
        assert_eq!(arena.story_start_word(0), 0);
    }

    #[test]
    fn the_retail_reader_takes_the_first_chunk_whatever_its_id() {
        // A section whose first chunk is the journal: the original reads it
        // as the character, Exarust's tagged rule does not.
        let mut data = PlayerData::story(name("exanima01"));
        data.set_chunk(JOURNAL_CHUNK, write_journal(&[]));
        let bytes = data.write();
        let tagged = PlayerData::parse(&bytes, Flavour::Story).unwrap();
        assert_eq!(tagged.character, None);
        assert_eq!(tagged.chunks.len(), 1);
        let retail = PlayerData::parse_retail(&bytes, Flavour::Story).unwrap();
        assert!(retail.character.is_some());
        assert!(retail.chunks.is_empty());
        // A placement with the writer's tag reads the same either way.
        let mut placed = PlayerData::story(name("exanima01"));
        placed.character = Some(vec![1, 0, 0, 0]);
        placed.set_chunk(JOURNAL_CHUNK, write_journal(&[]));
        let bytes = placed.write();
        assert_eq!(
            PlayerData::parse(&bytes, Flavour::Story).unwrap().character,
            PlayerData::parse_retail(&bytes, Flavour::Story)
                .unwrap()
                .character
        );
    }

    #[test]
    fn slot_names_use_three_bytes_and_either_extension() {
        assert_eq!(
            slot_file_name(PREFIX_STORY, 1, SlotKind::Save),
            "Exanima001.rsg"
        );
        assert_eq!(
            slot_file_name(PREFIX_ARENA, 999, SlotKind::Checkpoint),
            "Arena999.rcp"
        );
        assert_eq!(
            parse_slot_file("Exanima001.rsg", PREFIX_STORY),
            Some((1, SlotKind::Save))
        );
        assert_eq!(
            parse_slot_file("arena999.RCP", PREFIX_ARENA),
            Some((999, SlotKind::Checkpoint))
        );
        for (file, reason) in [
            ("Exanima000.rsg", "zero"),
            ("Exanima1000.rsg", "four digits"),
            ("Exanima01.rsg", "two digits"),
            ("Arena001.rsg", "other prefix"),
            ("Exanima001.rsx", "other extension"),
            ("Exanima001.rsg.bak", "longer"),
        ] {
            assert_eq!(parse_slot_file(file, PREFIX_STORY), None, "{reason}");
        }
        // The original reads three bytes arithmetically, not as digits.
        assert_eq!(
            parse_slot_file("Exanima1:2.rsg", PREFIX_STORY),
            Some((202, SlotKind::Save))
        );
    }

    #[test]
    fn the_story_start_check_refuses_newer_and_old_object_databases() {
        let mut package = PackageImage::new();
        assert!(!story_start_incompatible(&package, false));
        assert!(!story_start_incompatible(&package, true));
        // An exanima06 section is a newer campaign.
        package.put_section(&name("exanima06"), &[]);
        assert!(story_start_incompatible(&package, false));
        assert!(!story_start_incompatible(&package, true));
        // Exanima04 with a current object database is fine.
        let mut package = PackageImage::new();
        package.put_section(&name("exanima04"), &[]);
        let current = world_object_db(&WORLD_OBJECT_DB_MAGIC.to_le_bytes(), &[]);
        package.put_section(WORLD_OBJECT_DB, &current);
        assert!(!story_start_incompatible(&package, false));
        // The magic is the section's third word, not its first: the bare
        // magic at the start is a mismatch.
        package.put_section(WORLD_OBJECT_DB, &WORLD_OBJECT_DB_MAGIC.to_le_bytes());
        assert!(story_start_incompatible(&package, false));
        // A mismatched magic, a short section or a missing database differ.
        let older = world_object_db(&0xDBAC_0D05u32.to_le_bytes(), &[]);
        package.put_section(WORLD_OBJECT_DB, &older);
        assert!(story_start_incompatible(&package, false));
        package.put_section(WORLD_OBJECT_DB, &[0, 1]);
        assert!(story_start_incompatible(&package, false));
        package.remove_section(WORLD_OBJECT_DB);
        assert!(!story_start_incompatible(&package, false));
    }

    #[test]
    fn an_old_arena_package_drops_its_hub_section() {
        let mut data = PlayerData::arena(name("arenahub"));
        let mut package = PackageImage::new();
        package.put_section(&name("arenahub"), b"hub");
        package.put_section(PLAYER_DATA, &data.write());
        assert!(!data.convert_arena());
        assert!(package.has_section(&name("arenahub")));
        let PlayerHeader::Arena(header) = &mut data.header else {
            unreachable!()
        };
        header.data_version = 9;
        assert!(!data.convert_arena());
        let PlayerHeader::Arena(header) = &mut data.header else {
            unreachable!()
        };
        header.data_version = 4;
        assert!(data.convert_arena());
        package.remove_section(&name("arenahub"));
        assert!(!package.has_section(&name("arenahub")));
    }

    #[test]
    fn a_story_section_round_trips_with_its_chunks() {
        let mut data = PlayerData::story(name("exanima04"));
        let PlayerHeader::Story(header) = &mut data.header else {
            unreachable!()
        };
        header.seed = 7;
        header.flags = 0x10003;
        header.clock = 900;
        header.camera = [1.0, 2.0, 3.0];
        header.zoom = 1.5;
        data.character = Some(vec![0x2A, 0, 0, 0x40]);
        data.set_journal(&[JournalRecord {
            id: 4,
            title: "Day one".to_string(),
            text: "The pit.".to_string(),
        }]);
        data.set_chunk(DISCOVERY_CHUNK, vec![1, 0, 0, 0]);
        data.set_chunk(GLOBAL_XP_CHUNK, 5000u32.to_le_bytes().to_vec());
        data.set_chunk(MEMORY_FLAGS_CHUNK, 3u32.to_le_bytes().to_vec());
        data.set_chunk(
            TRANSITION_CHUNK,
            [2u32, 5, 0, 0].map(u32::to_le_bytes).concat(),
        );

        let bytes = data.write();
        let read = PlayerData::parse(&bytes, Flavour::Story).unwrap();
        assert_eq!(read, data);
        let PlayerHeader::Story(header) = &read.header else {
            unreachable!()
        };
        assert_eq!(header.seed, 7);
        assert_eq!(header.flags & 0xFF, 3);
        assert_eq!((header.flags & 0xFF_0000) >> 16, 1);
        assert_eq!(header.clock, 900);
        assert_eq!(read.character, Some(vec![0x2A, 0, 0, 0x40]));
        assert_eq!(read.character_reference(), Some(0x4000_002A));
        assert_eq!(read.character_record_id(), Some(42));
        assert_eq!(read.character_in_world_db(), Some(true));
        assert_eq!(
            read.journal().unwrap().unwrap(),
            vec![JournalRecord {
                id: 4,
                title: "Day one".to_string(),
                text: "The pit.".to_string(),
            }]
        );
        assert_eq!(read.story_transition_locale(), Some(name("exanima05")));
        assert_eq!(
            read.chunk(GLOBAL_XP_CHUNK),
            Some(&5000u32.to_le_bytes()[..])
        );
        // The chunk order is the writer's.
        let ids: Vec<u32> = read.chunks.iter().map(|chunk| chunk.id).collect();
        assert_eq!(
            ids,
            vec![
                JOURNAL_CHUNK,
                DISCOVERY_CHUNK,
                GLOBAL_XP_CHUNK,
                MEMORY_FLAGS_CHUNK,
                TRANSITION_CHUNK
            ]
        );
    }

    #[test]
    fn both_headers_are_fifty_two_bytes() {
        // Signature, locale, camera target, zoom, then four mode words; the
        // retail saves' first chunk starts at byte 52.
        assert_eq!(PlayerData::story(name("exanima01")).write().len(), 52);
        assert_eq!(PlayerData::arena(name("arenahub")).write().len(), 52);
        let mut data = PlayerData::story(name("exanima01"));
        data.character = Some(vec![1, 0, 0, 0x40]);
        assert_eq!(&data.write()[52..56], &CHARACTER_TAG.to_le_bytes());
    }

    #[test]
    fn the_signature_rules_differ_between_the_modes() {
        let story = PlayerData::story(name("exanima01")).write();
        assert!(PlayerData::parse(&story, Flavour::Story).is_ok());
        // The story reader accepts newer signatures.
        let mut newer = story.clone();
        newer[0..4].copy_from_slice(&0x6F5A_EA15u32.to_le_bytes());
        assert!(PlayerData::parse(&newer, Flavour::Story).is_ok());
        // The arena reader takes its signature exactly.
        let arena = PlayerData::arena(name("arenahub")).write();
        assert!(PlayerData::parse(&arena, Flavour::Arena).is_ok());
        assert_eq!(
            PlayerData::parse(&arena, Flavour::Story)
                .unwrap()
                .header
                .flavour(),
            Flavour::Story
        );
        assert_eq!(
            PlayerData::parse(&story, Flavour::Arena),
            Err(PlayerDataError::Signature(STORY_SIGNATURE))
        );
        let mut older = story;
        older[0..4].copy_from_slice(&0x6F5A_EA13u32.to_le_bytes());
        assert_eq!(
            PlayerData::parse(&older, Flavour::Story),
            Err(PlayerDataError::Signature(0x6F5A_EA13))
        );
        assert_eq!(
            PlayerData::parse(&[1, 2, 3], Flavour::Story),
            Err(PlayerDataError::Truncated)
        );
    }

    #[test]
    fn the_world_database_sections_are_chunk_containers() {
        let objects = world_object_db(&WORLD_OBJECT_DB_MAGIC.to_le_bytes(), &[4, 5]);
        let chunks = parse_chunks(&objects).unwrap();
        assert_eq!(chunks.len(), 2);
        assert_eq!(chunks[0].id, WORLD_OBJECTS_CHUNK);
        assert_eq!(chunks[0].payload, &WORLD_OBJECT_DB_MAGIC.to_le_bytes());
        assert_eq!(chunks[1].id, DATA_MANAGER_CHUNK);
        assert_eq!(chunks[1].payload, &[4, 5]);
        // Both chunks are written even when empty.
        let empty = world_object_db(&[], &[]);
        assert_eq!(empty.len(), 16);
        let ids: Vec<u32> = parse_chunks(&empty).unwrap().iter().map(|c| c.id).collect();
        assert_eq!(ids, [WORLD_OBJECTS_CHUNK, DATA_MANAGER_CHUNK]);
        let characters = world_character_db(&[1, 2, 3]);
        let chunks = parse_chunks(&characters).unwrap();
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].id, WORLD_CHARACTERS_CHUNK);
        assert_eq!(chunks[0].payload, &[1, 2, 3]);
    }

    #[test]
    fn the_character_screen_mask_keeps_the_journal_and_global_experience() {
        let mut data = PlayerData::story(name("exanima02"));
        data.set_chunk(DISCOVERY_CHUNK, vec![1]);
        data.set_chunk(JOURNAL_CHUNK, write_journal(&[]));
        data.set_chunk(GLOBAL_XP_CHUNK, 7u32.to_le_bytes().to_vec());
        data.set_chunk(MEMORY_FLAGS_CHUNK, 1u32.to_le_bytes().to_vec());
        data.set_chunk(
            TRANSITION_CHUNK,
            [1u32, 0, 0, 0].map(u32::to_le_bytes).concat(),
        );
        data.apply_mask(PlayerData::CHARACTER_SCREEN_MASK);
        let ids: Vec<u32> = data.chunks.iter().map(|chunk| chunk.id).collect();
        assert_eq!(ids, vec![JOURNAL_CHUNK, GLOBAL_XP_CHUNK]);
        let mut all = PlayerData::story(name("exanima02"));
        all.set_chunk(0x100, vec![1]);
        all.apply_mask(!0);
        assert_eq!(all.chunks.len(), 1);
        // Ids at or below 0xF never pass, even unmasked.
        all.set_chunk(0xF, vec![1]);
        all.apply_mask(!0);
        assert_eq!(all.chunks.len(), 1);
        // The reader dispatches on the masked id: 0x110 under the character
        // screen's mask reads as the journal, and the arena's arsenal as the
        // global experience snapshot.
        let mut odd = PlayerData::story(name("exanima02"));
        odd.set_chunk(0x110, vec![9]);
        odd.set_chunk(ARSENAL_CHUNK, vec![8]);
        odd.apply_mask(PlayerData::CHARACTER_SCREEN_MASK);
        let ids: Vec<u32> = odd.chunks.iter().map(|chunk| chunk.id).collect();
        assert_eq!(ids, vec![JOURNAL_CHUNK, GLOBAL_XP_CHUNK]);
    }

    #[test]
    fn an_old_arena_drops_its_skipped_chunks() {
        let mut data = PlayerData::arena(name("arenahub"));
        data.set_chunk(HIREABLES_CHUNK, vec![1]);
        data.set_chunk(SHOP_STOCK_CHUNK, vec![2]);
        data.set_chunk(ARSENAL_CHUNK, vec![3]);
        assert!(!data.arena_data_is_old());
        assert!(!data.convert_arena());
        let PlayerHeader::Arena(header) = &mut data.header else {
            unreachable!()
        };
        header.data_version = 8;
        assert!(data.arena_data_is_old());
        assert!(data.convert_arena());
        assert!(data.chunk(HIREABLES_CHUNK).is_none());
        assert!(data.chunk(SHOP_STOCK_CHUNK).is_none());
        assert_eq!(data.chunk(ARSENAL_CHUNK), Some(&[3u8][..]));
    }

    #[test]
    fn an_old_arena_puts_the_player_back_at_the_start_position() {
        let mut data = PlayerData::arena(name("arenahub"));
        let placement = CharacterPlacement {
            reference: CHARACTER_IN_WORLD_DB | 1,
            position: [-168.5, 0.25, 137.0],
            heading: -2.4,
            condition: 0,
            chunks: vec![OwnedChunk {
                id: 0xCAC0_EA00,
                data: vec![0; 12],
            }],
        };
        data.character = Some(placement.write());
        let PlayerHeader::Arena(header) = &mut data.header else {
            unreachable!()
        };
        header.data_version = 0x0108;
        assert!(data.convert_arena());
        let moved = data.placement().unwrap().unwrap();
        assert_eq!(moved.position, OLD_ARENA_POSITION);
        assert_eq!(moved.heading, -2.4);
        assert_eq!(moved.chunks, placement.chunks);
    }

    #[test]
    fn a_placement_reads_its_reference_position_and_sub_chunks() {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&0x4000_0001u32.to_le_bytes());
        for value in [8183.75f32, -6.0, 3961.0, -1.5] {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        bytes.extend_from_slice(&0u32.to_le_bytes());
        write_chunk(&mut bytes, 0xCAC0_E100, &[1, 2, 3]);
        write_chunk(&mut bytes, 0xCAC0_EA00, &[0; 12]);
        let placement = CharacterPlacement::parse(&bytes).unwrap();
        assert_eq!(placement.reference, 0x4000_0001);
        assert_eq!(placement.position, [8183.75, -6.0, 3961.0]);
        assert_eq!(placement.heading, -1.5);
        assert_eq!(placement.condition, 0);
        assert_eq!(
            placement.chunks.iter().map(|c| c.id).collect::<Vec<_>>(),
            [0xCAC0_E100, 0xCAC0_EA00]
        );
        assert_eq!(placement.write(), bytes);
        // A reference without bit 30 looks in the other database.
        let mut data = PlayerData::story(name("exanima01"));
        data.character = Some(bytes.clone());
        assert_eq!(data.character_in_world_db(), Some(true));
        data.character.as_mut().unwrap()[3] = 0;
        assert_eq!(data.character_in_world_db(), Some(false));
        assert_eq!(data.character_record_id(), Some(1));
        // A cut placement is an error, not a panic.
        assert_eq!(
            CharacterPlacement::parse(&bytes[..30]),
            Err(PlayerDataError::Truncated)
        );
        assert_eq!(
            CharacterPlacement::parse(&bytes[..10]),
            Err(PlayerDataError::Truncated)
        );
    }

    #[test]
    fn the_exporters_placement_reads_back_as_the_saved_character() {
        use crate::rfc_write::{ChunkWriter, DEAD, ExportCharacter, write_character_state};
        let character = ExportCharacter {
            record: CHARACTER_IN_WORLD_DB | 7,
            body_position: [120.0, 55.0, -40.0],
            floor_height: 12.5,
            heading: 1.25,
            condition: DEAD,
            ..ExportCharacter::default()
        };
        let mut writer = ChunkWriter::new();
        write_character_state(&mut writer, &character, true).unwrap();
        let chunk = writer.into_bytes();
        // The exporter writes the whole character chunk; its payload is the
        // placement the player data carries.
        assert_eq!(&chunk[0..4], &CHARACTER_TAG.to_le_bytes());
        let mut data = PlayerData::story(name("exanima01"));
        data.character = Some(chunk[8..].to_vec());
        let bytes = data.write();
        assert_eq!(&bytes[52..], &chunk[..]);
        let placement = PlayerData::parse(&bytes, Flavour::Story)
            .unwrap()
            .placement()
            .unwrap()
            .unwrap();
        assert_eq!(placement.reference, CHARACTER_IN_WORLD_DB | 7);
        assert_eq!(placement.position, [120.0, 12.5, -40.0]);
        assert_eq!(placement.heading, 1.25);
        assert_eq!(placement.condition, DEAD);
        assert_eq!(
            placement.chunks.iter().map(|c| c.id).collect::<Vec<_>>(),
            [0xCAC0_E100]
        );
    }

    #[test]
    fn a_tail_of_eight_bytes_is_not_read_as_a_chunk() {
        let mut data = PlayerData::story(name("exanima01"));
        data.set_chunk(GLOBAL_XP_CHUNK, 5u32.to_le_bytes().to_vec());
        let mut bytes = data.write();
        // An empty chunk at the very end: exactly eight bytes remain, which
        // the original's "more than eight" loop does not enter.
        write_chunk(&mut bytes, MEMORY_FLAGS_CHUNK, &[]);
        let read = PlayerData::parse(&bytes, Flavour::Story).unwrap();
        assert_eq!(read.chunks.len(), 1);
        // Nine bytes do enter it, and the empty chunk is read.
        bytes.push(0);
        let read = PlayerData::parse(&bytes, Flavour::Story).unwrap();
        assert_eq!(read.chunks.len(), 2);
        assert_eq!(read.chunk(MEMORY_FLAGS_CHUNK), Some(&[][..]));
    }

    /// A story save laid out byte by byte as the original's writers lay it
    /// out (synthetic values, the shape of a retail save): `#worldchrdb`,
    /// `#playerdata` and the level, each at the next 0x2000 boundary.
    pub(crate) fn retail_shaped_story() -> Vec<u8> {
        fn word(out: &mut Vec<u8>, value: u32) {
            out.extend_from_slice(&value.to_le_bytes());
        }
        let mut characters = Vec::new();
        word(&mut characters, WORLD_CHARACTERS_CHUNK);
        word(&mut characters, 8);
        word(&mut characters, 0xDBAC_CD00);
        word(&mut characters, 1);
        let mut player = Vec::new();
        word(&mut player, STORY_SIGNATURE);
        player.extend_from_slice(b"exanima01\0\0\0\0\0\0\0");
        for value in [7034.0f32, 1532.5, 4748.5, 800.0] {
            player.extend_from_slice(&value.to_le_bytes());
        }
        word(&mut player, 0x02A6_5335); // seed
        word(&mut player, 0x1_0000); // started, background 0
        word(&mut player, 8684); // clock
        word(&mut player, 0);
        word(&mut player, CHARACTER_TAG);
        word(&mut player, 24 + 8 + 4 + 8 + 12);
        word(&mut player, CHARACTER_IN_WORLD_DB | 1);
        for value in [8183.75f32, -6.0, 3961.0, -1.5] {
            player.extend_from_slice(&value.to_le_bytes());
        }
        word(&mut player, 0);
        word(&mut player, 0xCAC0_E100);
        word(&mut player, 4);
        word(&mut player, 0x0102_0304);
        word(&mut player, 0xCAC0_EA00);
        word(&mut player, 12);
        player.extend_from_slice(&[0; 12]);
        word(&mut player, DISCOVERY_CHUNK);
        word(&mut player, 4);
        word(&mut player, 0);
        word(&mut player, JOURNAL_CHUNK);
        word(&mut player, 4 + 4 + 4 + 9 + 4);
        word(&mut player, 1);
        word(&mut player, 0);
        word(&mut player, 9);
        player.extend_from_slice(b"New Entry");
        word(&mut player, 0);
        word(&mut player, GLOBAL_XP_CHUNK);
        word(&mut player, 4);
        word(&mut player, 44);
        let level = b"level bytes".to_vec();
        let mut out = Vec::new();
        word(&mut out, PACKAGE_MAGIC);
        word(&mut out, 4);
        word(&mut out, 0);
        word(&mut out, SENTINEL_SIZE);
        out.extend_from_slice(&[0; NAME_LEN]);
        let sections: [(&[u8; NAME_LEN], &Vec<u8>, u32); 3] = [
            (WORLD_CHARACTER_DB, &characters, 0x2000),
            (PLAYER_DATA, &player, 0x4000),
            (&name("exanima01"), &level, 0x6000),
        ];
        for (label, payload, offset) in sections {
            word(&mut out, offset);
            word(&mut out, payload.len() as u32);
            out.extend_from_slice(label);
        }
        for (_, payload, offset) in sections {
            out.resize(offset as usize, 0);
            out.extend_from_slice(payload);
        }
        out
    }

    #[test]
    fn a_rewrite_keeps_gaps_deleted_payloads_and_the_old_tail() {
        let mut bytes = write_package(
            PACKAGE_MAGIC,
            &[(&name("a"), &[1u8; 0x10]), (&name("b"), &[2u8; 0x10])],
        );
        bytes.resize(0x4020, 0);
        bytes[0x3000] = 0x77;
        bytes[0x4010..0x4020].fill(0xAA);
        let mut image = PackageImage::read(&bytes).unwrap();
        assert!(image.remove_section(&name("b")));
        image.put_section(&name("a"), &[3u8; 8]);
        let out = image.to_bytes();
        assert_eq!(out.len(), 0x4020, "the file never shrinks");
        assert_eq!(out[0x3000], 0x77, "a gap keeps its byte");
        assert_eq!(
            &out[0x4000..0x4010],
            &[2u8; 0x10],
            "a deleted payload stays"
        );
        assert_eq!(&out[0x4010..0x4020], &[0xAAu8; 0x10], "the old tail stays");
        assert_eq!(&out[0x2000..0x2008], &[3u8; 8]);
    }

    #[test]
    fn a_retail_shaped_save_round_trips_byte_for_byte() {
        let bytes = retail_shaped_story();
        // Read and written back unchanged.
        let package = PackageImage::read(&bytes).unwrap();
        assert_eq!(package.to_bytes(), bytes);
        // Written from scratch in the original's section order, it comes
        // out identical: the same offsets, sentinel and gaps.
        let mut rebuilt = PackageImage::new();
        for entry in package.entries() {
            rebuilt.put_section(&entry.name, &entry.data);
        }
        assert_eq!(rebuilt.to_bytes(), bytes);
        // The player data reads with its placement first and rewrites to
        // the same bytes.
        let section = package.section(PLAYER_DATA).unwrap();
        let data = PlayerData::parse(section, Flavour::Story).unwrap();
        assert_eq!(data.write(), section);
        assert_eq!(data.character_record_id(), Some(1));
        assert_eq!(data.character_in_world_db(), Some(true));
        let placement = data.placement().unwrap().unwrap();
        assert_eq!(placement.position, [8183.75, -6.0, 3961.0]);
        assert_eq!(
            data.chunks.iter().map(|c| c.id).collect::<Vec<_>>(),
            [DISCOVERY_CHUNK, JOURNAL_CHUNK, GLOBAL_XP_CHUNK]
        );
        assert_eq!(
            data.journal().unwrap().unwrap(),
            vec![JournalRecord {
                id: 0,
                title: "New Entry".to_string(),
                text: String::new(),
            }]
        );
        let PlayerHeader::Story(header) = data.header else {
            unreachable!()
        };
        assert_eq!((header.flags & 0xFF, header.flags >> 16), (0, 1));
        assert_eq!(header.clock, 8684);
    }

    #[test]
    fn retail_saves_round_trip_when_a_save_folder_is_given() {
        // A real-data check: `EXANIMA_SAVES` names a folder of the user's
        // own saves (`%APPDATA%\Exanima`). Every package must rebuild byte
        // for byte and its player data must rewrite unchanged. It skips
        // when the variable is not set; nothing is copied.
        let Ok(dir) = std::env::var("EXANIMA_SAVES") else {
            return;
        };
        let Ok(entries) = std::fs::read_dir(&dir) else {
            return;
        };
        for entry in entries.flatten() {
            let file = entry.file_name().to_string_lossy().to_lowercase();
            if !(file.ends_with(".rsg") || file.ends_with(".rcp")) {
                continue;
            }
            let bytes = std::fs::read(entry.path()).unwrap();
            let package = PackageImage::read(&bytes).unwrap();
            assert_eq!(package.to_bytes(), bytes, "{file}");
            let Some(section) = package.section(PLAYER_DATA) else {
                continue;
            };
            let signature = u32::from_le_bytes(section[0..4].try_into().unwrap());
            let flavour = if signature == ARENA_SIGNATURE {
                Flavour::Arena
            } else {
                Flavour::Story
            };
            let data = PlayerData::parse(section, flavour).unwrap();
            assert_eq!(data.write(), section, "{file}");
            assert!(data.placement().unwrap().is_ok(), "{file}");
        }
    }

    #[test]
    fn the_retail_executable_names_the_save_sections() {
        // A real-data check: the section names and both extensions are in
        // the shipped executable. It skips when the game is not configured,
        // and no bytes are copied into the repository.
        let Ok(dir) = std::env::var("EXANIMA_DIR") else {
            return;
        };
        let Ok(exe) = std::fs::read(std::path::Path::new(&dir).join("Exanima.exe")) else {
            return;
        };
        for needle in [
            &b"#playerdata"[..],
            &b"#worldobjdb"[..],
            &b"#worldchrdb"[..],
            &b".rsg"[..],
            &b".rcp"[..],
            &b"Exanima"[..],
            &b"Arena"[..],
        ] {
            assert!(
                exe.windows(needle.len()).any(|window| window == needle),
                "missing {needle:?}"
            );
        }
    }

    #[test]
    fn the_journal_ignores_trailing_bytes_and_counts() {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&1u32.to_le_bytes());
        bytes.extend_from_slice(&(-2i32).to_le_bytes());
        write_string(&mut bytes, b"A");
        write_string(&mut bytes, b"B");
        bytes.extend_from_slice(&[0xFF; 5]);
        let records = parse_journal(&bytes).unwrap();
        assert_eq!(
            records,
            vec![JournalRecord {
                id: -2,
                title: "A".to_string(),
                text: "B".to_string(),
            }]
        );
        // A count larger than the data is an error, not a panic.
        let short = 2u32.to_le_bytes();
        assert_eq!(parse_journal(&short), Err(PlayerDataError::Truncated));
        // A negative count resizes the array to nothing and reads no entry.
        assert_eq!(parse_journal(&(-1i32).to_le_bytes()).unwrap(), vec![]);
        assert_eq!(parse_journal(&0u32.to_le_bytes()).unwrap(), vec![]);
    }
}
