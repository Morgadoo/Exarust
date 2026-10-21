//! The retail save package (`SG_SaveWorld`) and the `#playerdata` section
//! both modes write (`XA_SaveGames`).
//!
//! A package is the magic `0xDA7AEA02`, a section count, a sentinel entry and
//! one 24-byte directory entry per section: a u32 offset, a u32 size and a
//! fixed 16-byte name. Section payloads start at the next `0x2000`
//! boundary. Writing a section that is already there rewrites it in place
//! when the room up to the next section suffices; otherwise the section is
//! relocated into the first gap it fits in, or appended, and the old bytes
//! are left alone. Deleting a section only removes its directory entry.
//!
//! The `#playerdata` section is a fixed 48-byte header followed by the
//! player's character placement chunk and then tagged chunks. The story and
//! arena headers share their first 32 bytes and differ in the last 16.
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
/// digits. Numbers above 999 are left whole, which the scanner never takes
/// as a slot.
pub fn slot_file_name(prefix: &str, number: u32, kind: SlotKind) -> String {
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

/// Parses a package. Entry 0 is the sentinel and is not exposed.
pub fn parse(bytes: &[u8]) -> Result<Package<'_>, PackageError> {
    if bytes.len() < 8 {
        return Err(PackageError::Short);
    }
    let magic = u32::from_le_bytes(bytes[0..4].try_into().unwrap());
    if magic != PACKAGE_MAGIC {
        return Err(PackageError::BadMagic(magic));
    }
    let count = u32::from_le_bytes(bytes[4..8].try_into().unwrap()) as usize;
    if count == 0 {
        return Err(PackageError::BadDirectory);
    }
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
    entries: Vec<Entry>,
}

impl PackageImage {
    /// An empty package (the sentinel only).
    pub fn new() -> Self {
        Self {
            magic: PACKAGE_MAGIC,
            entries: Vec::new(),
        }
    }

    /// Reads a package whole, keeping its directory order and offsets.
    pub fn read(bytes: &[u8]) -> Result<Self, PackageError> {
        let package = parse(bytes)?;
        // Rebuild from the directory so the offsets survive a round trip.
        let count = u32::from_le_bytes(bytes[4..8].try_into().unwrap()) as usize;
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
            entries,
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

    /// The index of the first gap the payload fits in, or the append index:
    /// the original scans consecutive sections and inserts after the first
    /// whose aligned end leaves `size` bytes before the next section.
    fn gap_index(&self, size: u32) -> usize {
        for index in 0..self.entries.len().saturating_sub(1) {
            let here = self.entries[index].offset;
            let used = self.entries[index].data.len() as u32;
            let gap = self.entries[index + 1].offset.saturating_sub(here);
            if size <= gap.saturating_sub(round_up(used)) {
                return index + 1;
            }
        }
        self.entries.len()
    }

    /// Inserts a section at `index`, placing it at the next boundary after
    /// the sentinel or the section before it.
    fn insert_at(&mut self, index: usize, name: [u8; NAME_LEN], data: Vec<u8>) {
        let offset = match index.checked_sub(1) {
            Some(previous) => {
                let entry = &self.entries[previous];
                round_up(entry.offset + entry.data.len() as u32)
            }
            None => round_up(SENTINEL_SIZE),
        };
        self.entries.insert(index, Entry { offset, name, data });
    }

    /// Writes a section with the original's rules: an existing last section
    /// is rewritten in place; another existing section is rewritten in place
    /// when the room up to the next one suffices, and relocated otherwise.
    pub fn put_section(&mut self, name: &[u8; NAME_LEN], data: &[u8]) {
        let size = data.len() as u32;
        if let Some(at) = self.entries.iter().position(|entry| entry.name == *name) {
            let last = at + 1 == self.entries.len();
            let room = self
                .entries
                .get(at + 1)
                .map(|next| next.offset.saturating_sub(self.entries[at].offset))
                .unwrap_or(0);
            if last || room >= size {
                self.entries[at].data = data.to_vec();
                return;
            }
            let index = self.gap_index(size);
            let entry = self.entries.remove(at);
            let index = if index > at { index - 1 } else { index };
            self.insert_at(index, entry.name, data.to_vec());
            return;
        }
        let index = self.gap_index(size);
        self.insert_at(index, *name, data.to_vec());
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

    /// The package's bytes: the header, the directory and every payload at
    /// its offset, with zeroes where nothing was written.
    pub fn to_bytes(&self) -> Vec<u8> {
        let count = self.entries.len() as u32 + 1;
        let mut out = Vec::new();
        out.extend_from_slice(&self.magic.to_le_bytes());
        out.extend_from_slice(&count.to_le_bytes());
        out.extend_from_slice(&0u32.to_le_bytes());
        out.extend_from_slice(&SENTINEL_SIZE.to_le_bytes());
        out.extend_from_slice(&[0u8; NAME_LEN]);
        for entry in &self.entries {
            out.extend_from_slice(&entry.offset.to_le_bytes());
            out.extend_from_slice(&(entry.data.len() as u32).to_le_bytes());
            out.extend_from_slice(&entry.name);
        }
        let end = self
            .entries
            .iter()
            .map(|entry| entry.offset as usize + entry.data.len())
            .max()
            .unwrap_or(0);
        out.resize(out.len().max(end), 0);
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

fn round_up(value: u32) -> u32 {
    value.saturating_add(SECTION_ALIGN - 1) & !(SECTION_ALIGN - 1)
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

/// The fixed 48-byte story header.
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

/// The fixed 48-byte arena header.
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
    /// Fewer than the 48 header bytes.
    Short,
    /// The signature is not this mode's, or older than the story's.
    Signature(u32),
    /// Fewer than eight bytes where the character or a chunk header was due.
    Truncated,
}

/// The player's saved character placement, as the mode writes it: one chunk
/// whose payload the level exporter defines.
pub const CHARACTER_TAG: u32 = 0xCAC0_CA00;

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

impl PlayerData {
    /// An empty story player data for `locale`.
    pub fn story(locale: [u8; NAME_LEN]) -> Self {
        Self {
            header: PlayerHeader::Story(StoryHeader {
                locale,
                camera: [0.0; 3],
                zoom: 1.0,
                seed: 0,
                flags: 0x1_0000,
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
                money: 0,
                data_version: ARENA_DATA_VERSION,
                clock: 0,
                day: 0,
            }),
            character: None,
            chunks: Vec::new(),
        }
    }

    /// Reads a `#playerdata` section with the given mode's signature rule.
    ///
    /// The story reader accepts any signature at or above its own, so a
    /// newer save still loads; the arena reader takes its signature exactly.
    /// The character is read first when the section starts with its chunk;
    /// the rest are read while a whole header fits, and a tail of up to
    /// seven bytes is ignored as the original's loop does.
    pub fn parse(bytes: &[u8], flavour: Flavour) -> Result<Self, PlayerDataError> {
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
            if id == CHARACTER_TAG {
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

    /// The mask the character screen loads a save with: the journal and the
    /// global experience snapshot only.
    pub const CHARACTER_SCREEN_MASK: u32 = 0x10_0010;

    /// Applies a load mask: the original ands each chunk id with the mask
    /// and ignores every result at or below `0xF`, so the chunks the mask
    /// zeroes are not imported.
    pub fn apply_mask(&mut self, mask: u32) {
        self.chunks.retain(|chunk| (chunk.id & mask) > 0xF);
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
    /// hireables and shop-stock chunks, which the loader skips. The package
    /// also drops its `arenahub` section; that is the caller's step.
    pub fn convert_arena(&mut self) -> bool {
        if !self.arena_data_is_old() {
            return false;
        }
        self.remove_chunk(HIREABLES_CHUNK);
        self.remove_chunk(SHOP_STOCK_CHUNK);
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
/// the data manager's chunk. Each is written only when it has bytes.
pub fn world_object_db(objects: &[u8], data_manager: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    if !objects.is_empty() {
        write_chunk(&mut out, WORLD_OBJECTS_CHUNK, objects);
    }
    if !data_manager.is_empty() {
        write_chunk(&mut out, DATA_MANAGER_CHUNK, data_manager);
    }
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
    let count = take_u32(bytes, &mut at)? as usize;
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
/// the current magic. A missing database or a section too short to hold
/// the magic counts as a mismatch here; the original reads past the buffer.
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
        Some(section) => match section.get(0..4) {
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
        package.put_section(WORLD_OBJECT_DB, &WORLD_OBJECT_DB_MAGIC.to_le_bytes());
        assert!(!story_start_incompatible(&package, false));
        // A mismatched magic, a short section or a missing database differ.
        package.put_section(WORLD_OBJECT_DB, &0xDBA_C0D05u32.to_le_bytes());
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
        data.character = Some(vec![9, 8, 7]);
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
        assert_eq!(read.character, Some(vec![9, 8, 7]));
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
        assert!(world_object_db(&[], &[]).is_empty());
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
    }
}
