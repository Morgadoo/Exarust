//! The retail save package and chunk framing (`SG_SaveWorld`,
//! `XA_SaveGames`). Sections are aligned to 0x2000, addressed by a 24-byte
//! directory after the 8-byte header, and carry a fixed 16-byte name.

/// The story package magic (`ExanimaNNN.rsg`).
pub const MAGIC_STORY: u32 = 0xDA7A_EA02;
/// The arena package magic (`ArenaNNN.rsg`).
pub const MAGIC_ARENA: u32 = 0xDA7A_AA00;
/// A section's fixed name length.
pub const NAME_LEN: usize = 16;
/// The section alignment.
pub const SECTION_ALIGN: u32 = 0x2000;
/// The sentinel entry's size: `8 + 8·count` at creation, left at 0x10.
pub const SENTINEL_SIZE: u32 = 0x10;

/// One section of a package.
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
}

/// Parses a package. Entry 0 is the sentinel and is not exposed.
pub fn parse(bytes: &[u8]) -> Result<Package<'_>, PackageError> {
    if bytes.len() < 8 {
        return Err(PackageError::Short);
    }
    let magic = u32::from_le_bytes(bytes[0..4].try_into().unwrap());
    if magic != MAGIC_STORY && magic != MAGIC_ARENA {
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

fn round_up(value: u32) -> u32 {
    (value + SECTION_ALIGN - 1) & !(SECTION_ALIGN - 1)
}

/// Builds a package the same way the writer does: a sentinel entry, then
/// each section at the next 0x2000 boundary.
pub fn write_package(magic: u32, sections: &[(&[u8; NAME_LEN], &[u8])]) -> Vec<u8> {
    let count = sections.len() as u32 + 1;
    let mut out = Vec::new();
    out.extend_from_slice(&magic.to_le_bytes());
    out.extend_from_slice(&count.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&SENTINEL_SIZE.to_le_bytes());
    out.extend_from_slice(&[0u8; NAME_LEN]);
    let mut offset = SECTION_ALIGN;
    let mut entries = Vec::new();
    for (name, payload) in sections {
        entries.push((offset, payload.len() as u32, **name, *payload));
        offset = round_up(offset + payload.len() as u32);
    }
    for (entry_offset, size, name, _) in &entries {
        out.extend_from_slice(&entry_offset.to_le_bytes());
        out.extend_from_slice(&size.to_le_bytes());
        out.extend_from_slice(name);
    }
    for (entry_offset, _, _, payload) in entries {
        let at = entry_offset as usize;
        if out.len() < at {
            out.resize(at, 0);
        }
        out.extend_from_slice(payload);
    }
    out
}

/// One framed chunk: `u32 id, u32 len, payload`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Chunk<'a> {
    pub id: u32,
    pub payload: &'a [u8],
}

/// Parses consecutive chunks, skipping no bytes.
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

/// The player-data flavour.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Flavour {
    Story,
    Arena,
}

/// The 48-byte player-data header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerHeader {
    pub flavour: Flavour,
    pub version: u32,
    pub level: [u8; 16],
    pub clock: [u8; 12],
    pub field32: u32,
    pub field36: u32,
    pub field40: u32,
    pub field44: u32,
}

/// A player-header error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeaderError {
    Short,
    UnknownVersion(u32),
}

/// Parses the story (`0x6F5AEA14`) or arena (`0xE8AAEA10`) header.
pub fn parse_player_header(bytes: &[u8]) -> Result<PlayerHeader, HeaderError> {
    if bytes.len() < 48 {
        return Err(HeaderError::Short);
    }
    let version = u32::from_le_bytes(bytes[0..4].try_into().unwrap());
    let flavour = match version {
        0x6F5A_EA14 => Flavour::Story,
        0xE8AA_EA10 => Flavour::Arena,
        other => return Err(HeaderError::UnknownVersion(other)),
    };
    let mut level = [0u8; 16];
    level.copy_from_slice(&bytes[4..20]);
    let mut clock = [0u8; 12];
    clock.copy_from_slice(&bytes[20..32]);
    Ok(PlayerHeader {
        flavour,
        version,
        level,
        clock,
        field32: u32::from_le_bytes(bytes[32..36].try_into().unwrap()),
        field36: u32::from_le_bytes(bytes[36..40].try_into().unwrap()),
        field40: u32::from_le_bytes(bytes[40..44].try_into().unwrap()),
        field44: u32::from_le_bytes(bytes[44..48].try_into().unwrap()),
    })
}

/// A fixed 16-byte name from text.
pub fn name(text: &str) -> [u8; NAME_LEN] {
    let mut out = [0u8; NAME_LEN];
    let bytes = text.as_bytes();
    out[..bytes.len().min(NAME_LEN)].copy_from_slice(&bytes[..bytes.len().min(NAME_LEN)]);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_minimal_package_parses() {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&MAGIC_STORY.to_le_bytes());
        bytes.extend_from_slice(&1u32.to_le_bytes());
        bytes.extend_from_slice(&0u32.to_le_bytes());
        bytes.extend_from_slice(&0x10u32.to_le_bytes());
        bytes.extend_from_slice(&[0u8; 16]);
        let package = parse(&bytes).unwrap();
        assert_eq!(package.magic, MAGIC_STORY);
        assert!(package.sections.is_empty());
    }

    #[test]
    fn two_sections_are_aligned() {
        let bytes = write_package(
            MAGIC_ARENA,
            &[
                (&name("arenahub"), b"AB"),
                (&name("exanima04"), b"123456789"),
            ],
        );
        assert_eq!(&bytes[0..4], &MAGIC_ARENA.to_le_bytes());
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
    fn a_bad_section_is_rejected() {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&MAGIC_STORY.to_le_bytes());
        bytes.extend_from_slice(&2u32.to_le_bytes());
        bytes.extend_from_slice(&0u32.to_le_bytes());
        bytes.extend_from_slice(&0x10u32.to_le_bytes());
        bytes.extend_from_slice(&[0u8; 16]);
        bytes.extend_from_slice(&0x9999_9999u32.to_le_bytes());
        bytes.extend_from_slice(&1u32.to_le_bytes());
        bytes.extend_from_slice(&name("section"));
        assert_eq!(parse(&bytes), Err(PackageError::SectionOutOfRange));
    }

    #[test]
    fn chunks_round_trip() {
        let mut bytes = Vec::new();
        write_chunk(&mut bytes, 0x0010_0000, &[0x2A, 0, 0, 0]);
        let chunks = parse_chunks(&bytes).unwrap();
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].id, 0x0010_0000);
        assert_eq!(chunks[0].payload, &[0x2A, 0, 0, 0]);
    }

    #[test]
    fn the_player_headers() {
        let mut story = vec![0u8; 48];
        story[0..4].copy_from_slice(&0x6F5A_EA14u32.to_le_bytes());
        story[4..12].copy_from_slice(b"exanima0");
        story[12] = b'4';
        story[40..44].copy_from_slice(&0x10000u32.to_le_bytes());
        let header = parse_player_header(&story).unwrap();
        assert_eq!(header.flavour, Flavour::Story);
        assert_eq!(header.level, name("exanima04"));
        assert_eq!(header.field40, 0x10000);

        let mut arena = vec![0u8; 48];
        arena[0..4].copy_from_slice(&0xE8AA_EA10u32.to_le_bytes());
        arena[36..40].copy_from_slice(&9u32.to_le_bytes());
        let header = parse_player_header(&arena).unwrap();
        assert_eq!(header.flavour, Flavour::Arena);
        assert_eq!(header.field36, 9);
    }
}
