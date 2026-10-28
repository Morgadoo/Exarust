//! An Exarust save of the session, stored in a retail-shaped package.
//!
//! The file is a save package ([`exa_formats::save_package`]) whose
//! `#playerdata` section uses the original's story or arena header with the
//! fields Exarust has, a journal chunk in the original's layout and one
//! Exarust chunk holding everything else (the mode, the story's journal and
//! used topics, the arena company and its roster, the caster's focus). The
//! story's level is the locale, the arena's locale is `arenahub`, and the
//! arena's copper and day are the header's money and day.
//!
//! The original's own character placement, discovery, world databases and
//! level sections are not written yet: Exarust has no world to export.
//! A short, mistyped or otherwise malformed file is an error. The Exarust
//! chunk's own payload is version 2 (version 1 had no discovery,
//! experience-worth, memory or transition fields).

use exa_formats::arena_save::{self, ArenaTables};
use exa_formats::save_package::{
    self, ARENA_DATA_VERSION, EXARUST_CHUNK, Flavour, JournalRecord, PLAYER_DATA, PackageImage,
    PlayerData, PlayerHeader,
};
use exa_script::story::{Entry, Journal, Story};

use crate::career::{Company, Member, Profession};

const MAGIC: &[u8; 4] = b"EXRS";
const VERSION: u16 = 2;

/// The arena's single locale.
pub const ARENA_LOCALE: &str = "arenahub";

/// Which mode the session is in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Story,
    Arena,
}

/// What a save restores.
#[derive(Debug, Clone, PartialEq)]
pub struct Session {
    pub mode: Mode,
    pub story: Story,
    pub company: Company,
    pub focus: f32,
    /// The game clock in seconds.
    pub clock: u32,
    /// The story's save seed; zero in the arena.
    pub seed: u32,
    /// The character creation background the story save was started from.
    pub start_background: u32,
    /// The story's discovery chunk (the experience system's own format).
    /// None when the session has no experience state; a fresh story save
    /// then writes the empty chunk the original always writes.
    pub discovery: Option<Vec<u8>>,
    /// The account's experience worth saved in the global-experience chunk.
    /// None keeps the chunk a save already has, else the default (zero).
    pub global_experience: Option<u32>,
    /// The memory-fragment flags; zero is not written.
    pub memory_flags: Option<u32>,
    /// The sixteen-byte portal/respawn state; all zeroes are not written.
    pub transition: Option<[u8; 16]>,
    /// The arena's own table chunks. `None` keeps whatever a save already
    /// holds; `Some` replaces them in the writer's order (an empty stash,
    /// shop or staff list, or a merchant a non-empty staff lacks, is
    /// removed).
    pub arena_tables: Option<ArenaTables>,
}

impl Session {
    pub fn story() -> Self {
        Self {
            mode: Mode::Story,
            story: Story::begin(),
            company: Company::new(""),
            focus: 1.0,
            clock: 0,
            seed: 0,
            start_background: 0,
            discovery: None,
            global_experience: None,
            memory_flags: None,
            transition: None,
            arena_tables: None,
        }
    }

    pub fn career(name: impl Into<String>) -> Self {
        Self {
            mode: Mode::Arena,
            story: Story::begin(),
            company: Company::new(name),
            focus: 1.0,
            clock: 0,
            seed: 0,
            start_background: 0,
            discovery: None,
            global_experience: None,
            memory_flags: None,
            transition: None,
            arena_tables: None,
        }
    }

    /// The world clock this save restores at `ticks_ms` of the game clock:
    /// the header's saved seconds and that tick as the base (the original's
    /// setter, run by the player-data reader).
    pub fn world_clock(&self, ticks_ms: u32) -> exa_combat::remedy::WorldClock {
        crate::world_clock::restored(self.clock, ticks_ms)
    }

    /// Stamps the current world seconds into the header at `ticks_ms`, as
    /// the player-data writer reads them at the save.
    pub fn stamp_world_clock(&mut self, clock: &exa_combat::remedy::WorldClock, ticks_ms: u32) {
        self.clock = crate::world_clock::stamped(clock, ticks_ms);
    }

    /// Fills the session's arena tables from the live company, staff and
    /// arsenal, keeping every chunk the save already holds that Exarust has
    /// no live model for. `stash` is the hub stash's item ids.
    pub fn capture_arena(
        &mut self,
        career: &crate::hub_windows::ArenaCareer,
        staff: &crate::staff::Staff,
        arsenal: &crate::arsenal::Arsenal,
        stash: &[u32],
    ) {
        let previous = self.arena_tables.take();
        self.arena_tables = Some(crate::arena_tables::from_live(
            career,
            staff,
            arsenal,
            stash,
            previous.as_ref(),
        ));
    }

    /// Puts a loaded save's arena tables into the live models. `false` when
    /// the session carries none (a save written before the tables were
    /// filled, or one made from nothing).
    pub fn apply_arena_tables(
        &self,
        career: &mut crate::hub_windows::ArenaCareer,
        staff: &mut crate::staff::Staff,
        arsenal: &mut crate::arsenal::Arsenal,
        stash: &mut Vec<u32>,
    ) -> bool {
        match &self.arena_tables {
            Some(tables) => {
                crate::arena_tables::apply(career, staff, arsenal, stash, tables);
                true
            }
            None => false,
        }
    }
}

/// What a save slot's player data says without restoring the session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SlotInfo {
    pub flavour: Flavour,
    /// The saved locale.
    pub locale: [u8; 16],
    /// The game clock in seconds.
    pub clock: u32,
    /// The arena's copper; zero in the story.
    pub money: u32,
    /// The arena's day; zero in the story.
    pub day: u32,
    /// How many journal entries the save holds.
    pub journal: usize,
    /// The player's character record id, when a placement chunk is stored.
    pub player_record: Option<u32>,
    /// The player's name from the save's world character database, when
    /// the placement's reference names a record there. The original shows
    /// this name in the slot list.
    pub player_name: Option<String>,
}

/// Why a file is not a session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SaveError {
    /// The bytes are not an Exarust save, or they cut a field short.
    Malformed,
}

/// The journal records of a story's pages; the original journals carry an
/// id and Exarust's pages have none, so the page index is written.
fn journal_records(story: &Story) -> Vec<JournalRecord> {
    story
        .journal()
        .entries()
        .iter()
        .enumerate()
        .map(|(index, page)| JournalRecord {
            id: index as i32,
            title: page.title.clone(),
            text: page.text.clone(),
        })
        .collect()
}

/// The player data a session writes.
pub fn player_data(session: &Session) -> PlayerData {
    let mut data = match session.mode {
        Mode::Story => {
            let mut data = PlayerData::story(save_package::name(session.story.level()));
            if let PlayerHeader::Story(header) = &mut data.header {
                header.seed = session.seed;
                header.flags = (session.start_background & 0xFF) | save_package::STORY_STARTED_FLAG;
                header.clock = session.clock;
            }
            if let Some(discovery) = &session.discovery {
                data.set_chunk(save_package::DISCOVERY_CHUNK, discovery.clone());
            }
            if let Some(worth) = session.global_experience {
                data.set_chunk(save_package::GLOBAL_XP_CHUNK, worth.to_le_bytes().to_vec());
            }
            if let Some(flags) = session.memory_flags {
                if flags == 0 {
                    data.remove_chunk(save_package::MEMORY_FLAGS_CHUNK);
                } else {
                    data.set_chunk(
                        save_package::MEMORY_FLAGS_CHUNK,
                        flags.to_le_bytes().to_vec(),
                    );
                }
            }
            if let Some(state) = session.transition {
                if state.iter().all(|&byte| byte == 0) {
                    data.remove_chunk(save_package::TRANSITION_CHUNK);
                } else {
                    data.set_chunk(save_package::TRANSITION_CHUNK, state.to_vec());
                }
            }
            data
        }
        Mode::Arena => {
            let mut data = PlayerData::arena(save_package::name(ARENA_LOCALE));
            if let PlayerHeader::Arena(header) = &mut data.header {
                header.money = session.company.copper;
                header.data_version = ARENA_DATA_VERSION;
                header.clock = session.clock;
                header.day = session.company.day;
            }
            if let Some(tables) = &session.arena_tables {
                let mut encoded = Vec::new();
                tables.write(&mut encoded);
                if let Ok(chunks) = save_package::parse_chunks(&encoded) {
                    let present: Vec<u32> = chunks.iter().map(|chunk| chunk.id).collect();
                    for chunk in chunks {
                        data.set_chunk(chunk.id, chunk.payload.to_vec());
                    }
                    // The writer omits an empty stash, shop or staff list,
                    // and the merchant of a staff without one; remove the previous save's chunk so it does
                    // not survive the merge.
                    for (id, absent) in [
                        (save_package::HUB_STASH_CHUNK, tables.stash.is_empty()),
                        (save_package::SHOP_STOCK_CHUNK, tables.shop_stock.is_empty()),
                        (
                            save_package::EMPLOYEES_CHUNK,
                            tables.employees.employees.is_empty(),
                        ),
                        (
                            save_package::MERCHANT_CHUNK,
                            !arena_save::merchant_chunk_written(&tables.employees),
                        ),
                    ] {
                        if absent && !present.contains(&id) {
                            data.remove_chunk(id);
                        }
                    }
                }
            }
            data
        }
    };
    data.set_journal(&journal_records(&session.story));
    data
}

/// The `#playerdata` section a session writes: the mode's header, the
/// journal and the Exarust chunk with everything else. Over a previous
/// section of the same mode, the character placement is carried over when
/// the session has none (the original's arena writer does this for a save
/// made with no scene loaded), and so are the chunks Exarust does not
/// produce (discovery, global experience, the arena tables), in their
/// order, so a retail save overwritten by Exarust keeps what Exarust cannot
/// regenerate yet.
pub fn player_section_after(session: &Session, previous: Option<&[u8]>) -> Vec<u8> {
    let mut data = player_data(session);
    let flavour = data.header.flavour();
    if let Some(bytes) = previous
        && flavour_of(bytes) == flavour
        && let Ok(old) = PlayerData::parse(bytes, flavour)
    {
        if data.character.is_none() {
            data.character = old.character;
        }
        let ours = std::mem::take(&mut data.chunks);
        data.chunks = old
            .chunks
            .into_iter()
            .filter(|chunk| chunk.id != save_package::JOURNAL_CHUNK && chunk.id != EXARUST_CHUNK)
            .collect();
        for chunk in ours {
            data.set_chunk(chunk.id, chunk.data);
        }
    }
    // The story writer always emits the discovery chunk and the global
    // experience word; a session with neither writes the defaults.
    if flavour == Flavour::Story {
        // The memory flags and the transition state are written only when
        // non-zero. A session with a state (Some) decides: a zero state
        // removes an older chunk; None keeps what the save had.
        if let Some(flags) = session.memory_flags {
            if flags == 0 {
                data.remove_chunk(save_package::MEMORY_FLAGS_CHUNK);
            } else {
                data.set_chunk(
                    save_package::MEMORY_FLAGS_CHUNK,
                    flags.to_le_bytes().to_vec(),
                );
            }
        }
        if let Some(state) = session.transition {
            if state.iter().all(|&byte| byte == 0) {
                data.remove_chunk(save_package::TRANSITION_CHUNK);
            } else {
                data.set_chunk(save_package::TRANSITION_CHUNK, state.to_vec());
            }
        }
        if data.chunk(save_package::DISCOVERY_CHUNK).is_none() {
            data.set_chunk(save_package::DISCOVERY_CHUNK, vec![0; 8]);
        }
        if data.chunk(save_package::GLOBAL_XP_CHUNK).is_none() {
            data.set_chunk(save_package::GLOBAL_XP_CHUNK, 0u32.to_le_bytes().to_vec());
        }
    }
    data.set_chunk(EXARUST_CHUNK, write_body(session));
    data.write_ordered()
}

/// The `#playerdata` section a session writes into an empty save.
pub fn player_section(session: &Session) -> Vec<u8> {
    player_section_after(session, None)
}

/// The package a session writes into an empty file.
pub fn package(session: &Session) -> Vec<u8> {
    let mut package = PackageImage::new();
    package.put_section(PLAYER_DATA, &player_section(session));
    package.to_bytes()
}

/// Read a player data section's flavour: the arena signature is exact and
/// every other accepted signature is read as the story's.
fn flavour_of(bytes: &[u8]) -> Flavour {
    match bytes.get(0..4) {
        Some(word)
            if u32::from_le_bytes(word.try_into().unwrap()) == save_package::ARENA_SIGNATURE =>
        {
            Flavour::Arena
        }
        _ => Flavour::Story,
    }
}

/// What a save slot's player data says, for listing.
pub fn slot_info(bytes: &[u8]) -> Result<SlotInfo, SaveError> {
    let package = PackageImage::read(bytes).map_err(|_| SaveError::Malformed)?;
    let section = package.section(PLAYER_DATA).ok_or(SaveError::Malformed)?;
    let data = PlayerData::parse(section, flavour_of(section)).map_err(|_| SaveError::Malformed)?;
    Ok(slot_info_of(&data))
}

/// What a decoded player data says, for listing.
pub fn slot_info_of(data: &PlayerData) -> SlotInfo {
    let (money, day) = match &data.header {
        PlayerHeader::Story(_) => (0, 0),
        PlayerHeader::Arena(header) => (header.money, header.day),
    };
    SlotInfo {
        flavour: data.header.flavour(),
        locale: data.header.locale(),
        clock: data.header.clock(),
        money,
        day,
        journal: data
            .journal()
            .ok()
            .flatten()
            .map(|records| records.len())
            .unwrap_or(0),
        player_record: data.character_record_id(),
        player_name: None,
    }
}

/// The player's name from a package's `#worldchrdb` section: the record the
/// placement's reference finds in the world database. The original's slot
/// scan imports that database and reads the name through the same lookup.
/// None when the section is missing, the record is not there, or the
/// reference points at the other character database.
pub fn player_name(package: &PackageImage, data: &PlayerData) -> Option<String> {
    if !data.character_in_world_db()? {
        return None;
    }
    let id = data.character_record_id()?;
    let section = package.section(save_package::WORLD_CHARACTER_DB)?;
    // The original's import takes the section's first chunk whatever its
    // id.
    let payload = save_package::first_chunk_payload(section)?;
    let mut db = exa_formats::character_db::CharacterDb::from_rdb(
        exa_formats::character_db::WORLD_TAG,
        payload,
    )
    .ok()?;
    let record = db.get(id)?;
    Some(record.record().name.clone())
}

/// Write the session: a package with its `#playerdata` section.
pub fn write(session: &Session) -> Vec<u8> {
    package(session)
}

/// Read a session. Malformed input is an error.
pub fn read(bytes: &[u8]) -> Result<Session, SaveError> {
    let package = PackageImage::read(bytes).map_err(|_| SaveError::Malformed)?;
    read_package(&package)
}

/// Read a session from an already parsed package, as the slot load has it
/// after its version conversion.
pub fn read_package(package: &PackageImage) -> Result<Session, SaveError> {
    let section = package.section(PLAYER_DATA).ok_or(SaveError::Malformed)?;
    let data = PlayerData::parse(section, flavour_of(section)).map_err(|_| SaveError::Malformed)?;
    let body = data.chunk(EXARUST_CHUNK).ok_or(SaveError::Malformed)?;
    let mut session = read_body(body)?;
    let matches = matches!(
        (session.mode, data.header.flavour()),
        (Mode::Story, Flavour::Story) | (Mode::Arena, Flavour::Arena)
    );
    if !matches {
        return Err(SaveError::Malformed);
    }
    // The header is what the original restores; the Exarust payload's copy
    // is only a fallback for a file whose header a tool changed.
    match &data.header {
        PlayerHeader::Story(header) => {
            session.clock = header.clock;
            session.seed = header.seed;
            session.start_background = header.flags & 0xFF;
        }
        PlayerHeader::Arena(header) => {
            session.clock = header.clock;
            session.company.copper = header.money;
            session.company.day = header.day;
        }
    }
    // The story chunks carry the experience state; when present they are
    // what the original restores.
    if data.header.flavour() == Flavour::Story {
        if let Some(discovery) = data.chunk(save_package::DISCOVERY_CHUNK) {
            session.discovery = Some(discovery.to_vec());
        }
        if data.chunk(save_package::GLOBAL_XP_CHUNK).is_some() {
            session.global_experience = Some(data.global_experience());
        }
        if data.chunk(save_package::MEMORY_FLAGS_CHUNK).is_some() {
            session.memory_flags = Some(data.memory_flags());
        }
        if data.chunk(save_package::TRANSITION_CHUNK).is_some() {
            session.transition = Some(data.transition_state());
        }
        // The story reader resets the journal to one "New Entry" page and
        // then reads the chunk; an absent chunk is that default page. The
        // script model cannot hold an empty journal, so a chunk with no
        // entries falls back to the default page.
        let records = data
            .journal_or_default()
            .map_err(|_| SaveError::Malformed)?;
        let entries: Vec<Entry> = records
            .into_iter()
            .map(|record| Entry {
                title: record.title,
                text: record.text,
            })
            .collect();
        let journal = Journal::from_entries(entries).unwrap_or_default();
        let mut index = session.story.index();
        // The level comes from the header's locale (or `exanima05` when a
        // transition was cut short); the Exarust payload's index is only a
        // fallback.
        let locale = if let Some(forced) = data.story_transition_locale() {
            Some(forced)
        } else {
            Some(data.header.locale())
        };
        if let Some(locale) = locale {
            let name = save_package::name_text(&locale);
            if let Some(found) = exa_script::story::CAMPAIGN
                .iter()
                .position(|level| *level == name)
            {
                index = found;
            }
        }
        session.story = Story::restore(index, journal, session.story.topics_used().to_vec())
            .ok_or(SaveError::Malformed)?;
    }
    // The arena's own chunks are the live tables a load restores. They are
    // read whatever their order, as the original's reader dispatches on the
    // chunk ids; an absent always-written chunk keeps its default.
    if data.header.flavour() == Flavour::Arena {
        let chunks: Vec<save_package::Chunk<'_>> = data
            .chunks
            .iter()
            .map(|chunk| save_package::Chunk {
                id: chunk.id,
                payload: &chunk.data,
            })
            .collect();
        // A save without any arena table (one written before the tables
        // were filled) carries none, rather than the reader's defaults.
        if chunks
            .iter()
            .any(|chunk| arena_save::is_table_chunk(chunk.id))
            && let Ok(tables) = arena_save::read_tables(&chunks)
        {
            session.arena_tables = Some(tables);
        }
    }
    Ok(session)
}

/// Write a session's own payload: the mode, the story level, the journal,
/// the topics already chosen, the company and its roster, the day, the
/// weeks, the reward ids, the clock, the seed and the caster's focus.
fn write_body(session: &Session) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(MAGIC);
    out.extend_from_slice(&VERSION.to_le_bytes());
    out.push(match session.mode {
        Mode::Story => 0,
        Mode::Arena => 1,
    });
    out.extend_from_slice(&(session.story.index() as u16).to_le_bytes());
    out.extend_from_slice(&session.clock.to_le_bytes());
    out.extend_from_slice(&session.seed.to_le_bytes());
    out.extend_from_slice(&session.start_background.to_le_bytes());
    put_str(&mut out, &session.company.name);
    out.extend_from_slice(&session.company.copper.to_le_bytes());
    out.extend_from_slice(&session.company.won.to_le_bytes());
    out.extend_from_slice(&session.company.lost.to_le_bytes());
    out.extend_from_slice(&session.company.spent.to_le_bytes());
    out.extend_from_slice(&session.company.earned.to_le_bytes());
    out.extend_from_slice(&session.company.day.to_le_bytes());
    for won in session.company.weeks {
        out.push(u8::from(won));
    }
    out.extend_from_slice(&session.company.seasons_won.to_le_bytes());
    out.extend_from_slice(&session.company.season_rewards.to_le_bytes());
    out.extend_from_slice(&session.company.tournaments_won.to_le_bytes());
    put_count(&mut out, session.company.reward_ids.len());
    for id in &session.company.reward_ids {
        out.extend_from_slice(&id.to_le_bytes());
    }
    put_count(&mut out, session.company.roster.len());
    for member in &session.company.roster {
        out.push(profession_byte(member.profession));
        out.extend_from_slice(&member.xp.to_le_bytes());
        put_str(&mut out, &member.name);
    }
    let pages = session.story.journal().entries();
    put_count(&mut out, pages.len());
    for page in pages {
        put_str(&mut out, &page.title);
        put_str(&mut out, &page.text);
    }
    put_count(&mut out, session.story.topics_used().len());
    for topic in session.story.topics_used() {
        put_str(&mut out, topic);
    }
    out.extend_from_slice(&session.focus.to_le_bytes());
    match &session.discovery {
        Some(bytes) => {
            out.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
            out.extend_from_slice(bytes);
        }
        None => out.extend_from_slice(&u32::MAX.to_le_bytes()),
    }
    match session.global_experience {
        Some(worth) => {
            out.push(1);
            out.extend_from_slice(&worth.to_le_bytes());
        }
        None => out.push(0),
    }
    match session.memory_flags {
        Some(flags) => {
            out.push(1);
            out.extend_from_slice(&flags.to_le_bytes());
        }
        None => out.push(0),
    }
    match &session.transition {
        Some(state) => {
            out.push(1);
            out.extend_from_slice(state);
        }
        None => out.push(0),
    }
    out
}

/// Read a session's own payload. Malformed input is an error.
fn read_body(bytes: &[u8]) -> Result<Session, SaveError> {
    let mut rest = bytes;
    let magic = take(&mut rest, 4)?;
    if magic != MAGIC {
        return Err(SaveError::Malformed);
    }
    let version = u16::from_le_bytes(take(&mut rest, 2)?.try_into().unwrap());
    if version != VERSION {
        return Err(SaveError::Malformed);
    }
    let mode = match take(&mut rest, 1)?[0] {
        0 => Mode::Story,
        1 => Mode::Arena,
        _ => return Err(SaveError::Malformed),
    };
    let index = u16::from_le_bytes(take(&mut rest, 2)?.try_into().unwrap()) as usize;
    let clock = u32::from_le_bytes(take(&mut rest, 4)?.try_into().unwrap());
    let seed = u32::from_le_bytes(take(&mut rest, 4)?.try_into().unwrap());
    let start_background = u32::from_le_bytes(take(&mut rest, 4)?.try_into().unwrap());
    let company_name = take_str(&mut rest)?.to_string();
    let copper = u32::from_le_bytes(take(&mut rest, 4)?.try_into().unwrap());
    let won = u32::from_le_bytes(take(&mut rest, 4)?.try_into().unwrap());
    let lost = u32::from_le_bytes(take(&mut rest, 4)?.try_into().unwrap());
    let spent = u32::from_le_bytes(take(&mut rest, 4)?.try_into().unwrap());
    let earned = u32::from_le_bytes(take(&mut rest, 4)?.try_into().unwrap());
    let day = u32::from_le_bytes(take(&mut rest, 4)?.try_into().unwrap());
    let mut weeks = [false; 4];
    for won in &mut weeks {
        *won = match take(&mut rest, 1)?[0] {
            0 => false,
            1 => true,
            _ => return Err(SaveError::Malformed),
        };
    }
    let seasons_won = u32::from_le_bytes(take(&mut rest, 4)?.try_into().unwrap());
    let season_rewards = u32::from_le_bytes(take(&mut rest, 4)?.try_into().unwrap());
    let tournaments_won = u32::from_le_bytes(take(&mut rest, 4)?.try_into().unwrap());
    let reward_len = take_count(&mut rest)?;
    let mut reward_ids = Vec::with_capacity(reward_len);
    for _ in 0..reward_len {
        reward_ids.push(u32::from_le_bytes(take(&mut rest, 4)?.try_into().unwrap()));
    }
    let roster_len = take_count(&mut rest)?;
    let mut roster = Vec::with_capacity(roster_len);
    for _ in 0..roster_len {
        let profession = profession_of(take(&mut rest, 1)?[0])?;
        let xp = u32::from_le_bytes(take(&mut rest, 4)?.try_into().unwrap());
        roster.push(Member {
            name: take_str(&mut rest)?.to_string(),
            profession,
            xp,
            skills: [0; 6],
            skill_xp: [0; 6],
            techniques: [[0; 6]; 6],
            loadouts: Default::default(),
            health: u16::MAX,
            in_entry: false,
        });
    }
    let pages = take_count(&mut rest)?;
    let mut entries = Vec::with_capacity(pages);
    for _ in 0..pages {
        entries.push(Entry {
            title: take_str(&mut rest)?.to_string(),
            text: take_str(&mut rest)?.to_string(),
        });
    }
    let journal = Journal::from_entries(entries).ok_or(SaveError::Malformed)?;
    let used_len = take_count(&mut rest)?;
    let mut used = Vec::with_capacity(used_len);
    for _ in 0..used_len {
        used.push(take_str(&mut rest)?.to_string());
    }
    let focus = f32::from_le_bytes(take(&mut rest, 4)?.try_into().unwrap());
    let discovery_len = u32::from_le_bytes(take(&mut rest, 4)?.try_into().unwrap());
    let discovery = if discovery_len == u32::MAX {
        None
    } else {
        Some(take(&mut rest, discovery_len as usize)?.to_vec())
    };
    let global_experience = match take(&mut rest, 1)?[0] {
        0 => None,
        1 => Some(u32::from_le_bytes(take(&mut rest, 4)?.try_into().unwrap())),
        _ => return Err(SaveError::Malformed),
    };
    let memory_flags = match take(&mut rest, 1)?[0] {
        0 => None,
        1 => Some(u32::from_le_bytes(take(&mut rest, 4)?.try_into().unwrap())),
        _ => return Err(SaveError::Malformed),
    };
    let transition = match take(&mut rest, 1)?[0] {
        0 => None,
        1 => Some(<[u8; 16]>::try_from(take(&mut rest, 16)?).unwrap()),
        _ => return Err(SaveError::Malformed),
    };
    if !focus.is_finite() || !rest.is_empty() {
        return Err(SaveError::Malformed);
    }
    let story = Story::restore(index, journal, used).ok_or(SaveError::Malformed)?;
    Ok(Session {
        mode,
        story,
        company: Company {
            name: company_name,
            copper,
            roster,
            won,
            lost,
            spent,
            earned,
            day,
            weeks,
            seasons_won,
            season_rewards,
            reward_ids,
            tournaments_won,
            deaths: 0,
            stock: Vec::new(),
            emblem: exa_ui::EmblemRecord::default(),
        },
        focus,
        clock,
        seed,
        start_background,
        discovery,
        global_experience,
        memory_flags,
        transition,
        arena_tables: None,
    })
}

fn profession_byte(profession: Profession) -> u8 {
    match profession {
        Profession::Combatant => 0,
        Profession::Merchant => 1,
        Profession::Physician => 2,
        Profession::Trainer => 3,
    }
}

fn profession_of(byte: u8) -> Result<Profession, SaveError> {
    match byte {
        0 => Ok(Profession::Combatant),
        1 => Ok(Profession::Merchant),
        2 => Ok(Profession::Physician),
        3 => Ok(Profession::Trainer),
        _ => Err(SaveError::Malformed),
    }
}

fn put_count(out: &mut Vec<u8>, count: usize) {
    let count = u16::try_from(count).unwrap_or(u16::MAX);
    out.extend_from_slice(&count.to_le_bytes());
}

fn put_str(out: &mut Vec<u8>, text: &str) {
    let bytes = text.as_bytes();
    let len = u16::try_from(bytes.len()).unwrap_or(u16::MAX);
    let bytes = &bytes[..usize::from(len)];
    out.extend_from_slice(&len.to_le_bytes());
    out.extend_from_slice(bytes);
}

fn take<'a>(rest: &mut &'a [u8], n: usize) -> Result<&'a [u8], SaveError> {
    if rest.len() < n {
        return Err(SaveError::Malformed);
    }
    let (head, tail) = rest.split_at(n);
    *rest = tail;
    Ok(head)
}

fn take_count(rest: &mut &[u8]) -> Result<usize, SaveError> {
    Ok(usize::from(u16::from_le_bytes(
        take(rest, 2)?.try_into().unwrap(),
    )))
}

fn take_str<'a>(rest: &mut &'a [u8]) -> Result<&'a str, SaveError> {
    let len = take_count(rest)?;
    let bytes = take(rest, len)?;
    std::str::from_utf8(bytes).map_err(|_| SaveError::Malformed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::career::Profession;
    use crate::thaum::{self, Domain, Effect, Request};
    use exa_script::story::Topic;

    const GATE: Topic<'static> = Topic {
        title: "The gate",
        choices: &["I go on.", "I turn back."],
    };

    #[test]
    fn a_save_round_trips_the_session() {
        let mut session = Session::career("Ash");
        session.story.advance();
        session.story.journal_mut().write("The pit is open.");
        session.story.pick(GATE, 0).unwrap();
        session.company.hire("Bram", Profession::Combatant).unwrap();
        let ongoing = session
            .company
            .enter(crate::career::standard_board()[0])
            .unwrap();
        session.company.finish(ongoing, true, 1.0);
        session.focus = 0.5;
        session.clock = 4321;
        session.discovery = Some(vec![1, 0, 0, 0, 0, 0, 0, 0]);
        session.global_experience = Some(250);
        session.memory_flags = Some(3);
        session.transition = Some([1, 0, 0, 0, 2, 0, 0, 0, 3, 0, 0, 0, 4, 0, 0, 0]);
        let bytes = write(&session);
        let mut loaded = read(&bytes).unwrap();
        assert_eq!(loaded, session);
        assert_eq!(
            loaded.discovery.as_deref(),
            Some(&[1, 0, 0, 0, 0, 0, 0, 0][..])
        );
        assert_eq!(loaded.global_experience, Some(250));
        assert_eq!(loaded.memory_flags, Some(3));
        assert_eq!(loaded.transition, session.transition);
        assert_eq!(loaded.story.level(), "exanima02");
        assert_eq!(loaded.story.journal().entries()[0].text, "The pit is open.");
        assert_eq!(
            loaded.story.pick(GATE, 0),
            Err(exa_script::story::TalkError::Used)
        );
        assert_eq!(loaded.company.copper, 343 - 12 - 3 + 12);
        assert_eq!(loaded.company.roster[0].xp, 1000);
        assert_eq!(loaded.focus, 0.5);
        assert_eq!(loaded.clock, 4321);

        let mut caster = thaum::Caster::new(loaded.focus);
        let request = Request {
            arc: 0.05,
            yellow: 0.25,
            now_ms: 0,
            ..Request::default()
        };
        match thaum::cast(Domain::Energy, &mut caster, &request).unwrap() {
            Effect::Energy { yellow } => assert!((yellow - 0.20).abs() < 1e-5),
            other => panic!("{other:?}"),
        }

        assert_eq!(read(b""), Err(SaveError::Malformed));
        assert_eq!(read(b"EXRS"), Err(SaveError::Malformed));
        let mut retail = 0xDA7AEA02u32.to_le_bytes().to_vec();
        retail.extend_from_slice(&[0, 1, 0, 0]);
        assert_eq!(read(&retail), Err(SaveError::Malformed));
        let mut bad = bytes.clone();
        bad.pop();
        assert_eq!(read(&bad), Err(SaveError::Malformed));
    }

    #[test]
    fn the_package_carries_the_retail_fields_and_the_journal() {
        let mut session = Session::career("Ash");
        for _ in 0..5 {
            session.company.end_day([crate::career::OUR_COMPANY; 6]);
        }
        session.clock = 90;
        session.company.copper = 45;
        session.story.journal_mut().write("A page.");
        let bytes = package(&session);
        let package = exa_formats::save_package::PackageImage::read(&bytes).unwrap();
        let section = package.section(PLAYER_DATA).unwrap();
        let data = PlayerData::parse(section, Flavour::Arena).expect("the arena header parses");
        let PlayerHeader::Arena(header) = &data.header else {
            panic!("arena")
        };
        assert_eq!(header.locale, save_package::name("arenahub"));
        assert_eq!(header.money, 45);
        assert_eq!(header.data_version, ARENA_DATA_VERSION);
        assert_eq!(header.clock, 90);
        assert_eq!(header.day, 5);
        assert_eq!(
            data.journal().unwrap().unwrap(),
            vec![JournalRecord {
                id: 0,
                title: "New Entry".to_string(),
                text: "A page.".to_string(),
            }]
        );
        // The Exarust payload carries the rest.
        assert!(data.chunk(EXARUST_CHUNK).is_some());
        let info = slot_info(&bytes).unwrap();
        assert_eq!(info.flavour, Flavour::Arena);
        assert_eq!(info.clock, 90);
        assert_eq!(info.money, 45);
        assert_eq!(info.day, 5);
        assert_eq!(info.journal, 1);
    }

    #[test]
    fn a_slots_name_comes_from_the_world_character_database() {
        use exa_formats::characters::{CharacterRecord, ID_MASK, WORLD_DB};
        let mut record = CharacterRecord::new();
        record.id = 5;
        record.name = "Ash".to_string();
        let record = record.to_bytes().unwrap();
        let mut db = Vec::new();
        db.extend_from_slice(&0xDBAC_CD00u32.to_le_bytes());
        db.extend_from_slice(&16u32.to_le_bytes());
        db.extend_from_slice(&5u32.to_le_bytes());
        db.extend_from_slice(&0u32.to_le_bytes());
        db.extend_from_slice(&0u32.to_le_bytes());
        db.extend_from_slice(&(record.len() as u32).to_le_bytes());
        db.extend_from_slice(&record);

        let mut data = PlayerData::story(save_package::name("exanima01"));
        data.character = Some(
            save_package::CharacterPlacement {
                reference: WORLD_DB | 5,
                position: [1.0, 2.0, 3.0],
                heading: 0.0,
                condition: 0,
                chunks: Vec::new(),
            }
            .write(),
        );
        let mut package = PackageImage::new();
        package.put_section(
            save_package::WORLD_CHARACTER_DB,
            &save_package::world_character_db(&db),
        );
        package.put_section(PLAYER_DATA, &data.write());
        assert_eq!(player_name(&package, &data).as_deref(), Some("Ash"));
        // The importer takes the section's first chunk whatever its id.
        let mut other = Vec::new();
        save_package::write_chunk(&mut other, 0x1234, &db);
        package.put_section(save_package::WORLD_CHARACTER_DB, &other);
        assert_eq!(player_name(&package, &data).as_deref(), Some("Ash"));
        // A reference without the world bit does not look in this database.
        data.character = Some(
            save_package::CharacterPlacement {
                reference: 5,
                position: [1.0, 2.0, 3.0],
                heading: 0.0,
                condition: 0,
                chunks: Vec::new(),
            }
            .write(),
        );
        assert_eq!(player_name(&package, &data), None);
        // A record id that is not in the database gives no name.
        data.character = Some(
            save_package::CharacterPlacement {
                reference: WORLD_DB | (ID_MASK & 6),
                position: [1.0, 2.0, 3.0],
                heading: 0.0,
                condition: 0,
                chunks: Vec::new(),
            }
            .write(),
        );
        assert_eq!(player_name(&package, &data), None);
    }

    #[test]
    fn a_fresh_story_save_writes_the_always_chunks_in_the_writers_order() {
        let session = Session::story();
        let data = PlayerData::parse(&player_section(&session), Flavour::Story).unwrap();
        let ids: Vec<u32> = data.chunks.iter().map(|chunk| chunk.id).collect();
        assert_eq!(
            ids,
            [
                save_package::DISCOVERY_CHUNK,
                save_package::JOURNAL_CHUNK,
                save_package::GLOBAL_XP_CHUNK,
                save_package::EXARUST_CHUNK
            ]
        );
        // The defaults: an empty discovery list and a worth of zero.
        assert_eq!(data.chunk(save_package::DISCOVERY_CHUNK), Some(&[0; 8][..]));
        assert_eq!(data.global_experience(), 0);
        // The session restores the chunk's default state.
        assert_eq!(
            read(&package(&session)).unwrap().discovery,
            Some(vec![0; 8])
        );
        assert_eq!(read(&package(&session)).unwrap().global_experience, Some(0));
        // A session with state writes it, and the non-zero memory and
        // transition chunks follow the experience.
        let mut with = session;
        with.discovery = Some(vec![4, 0, 0, 0, 0, 0, 0, 0]);
        with.global_experience = Some(9);
        with.memory_flags = Some(2);
        with.transition = Some([2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        let data = PlayerData::parse(&player_section(&with), Flavour::Story).unwrap();
        assert_eq!(
            data.chunks.iter().map(|chunk| chunk.id).collect::<Vec<_>>(),
            [
                save_package::DISCOVERY_CHUNK,
                save_package::JOURNAL_CHUNK,
                save_package::GLOBAL_XP_CHUNK,
                save_package::MEMORY_FLAGS_CHUNK,
                save_package::TRANSITION_CHUNK,
                save_package::EXARUST_CHUNK
            ]
        );
        assert_eq!(data.global_experience(), 9);
        assert_eq!(data.memory_flags(), 2);
        assert_eq!(data.transition_state()[0], 2);
    }

    #[test]
    fn the_player_header_is_what_a_load_restores() {
        let mut session = Session::story();
        session.clock = 1;
        session.seed = 2;
        session.start_background = 3;
        let bytes = write(&session);
        let mut package = PackageImage::read(&bytes).unwrap();
        let mut data =
            PlayerData::parse(package.section(PLAYER_DATA).unwrap(), Flavour::Story).unwrap();
        let PlayerHeader::Story(header) = &mut data.header else {
            unreachable!()
        };
        header.clock = 50;
        header.seed = 60;
        header.flags = 2 | save_package::STORY_STARTED_FLAG;
        header.locale = save_package::name("exanima05");
        package.put_section(PLAYER_DATA, &data.write_ordered());
        let loaded = read(&package.to_bytes()).unwrap();
        assert_eq!(loaded.clock, 50);
        assert_eq!(loaded.seed, 60);
        assert_eq!(loaded.start_background, 2);
        assert_eq!(loaded.story.level(), "exanima05");

        let mut session = Session::career("Ash");
        session.company.copper = 4;
        session.company.day = 5;
        let bytes = write(&session);
        let mut package = PackageImage::read(&bytes).unwrap();
        let mut data =
            PlayerData::parse(package.section(PLAYER_DATA).unwrap(), Flavour::Arena).unwrap();
        let PlayerHeader::Arena(header) = &mut data.header else {
            unreachable!()
        };
        header.clock = 70;
        header.money = 80;
        header.day = 90;
        package.put_section(PLAYER_DATA, &data.write_ordered());
        let loaded = read(&package.to_bytes()).unwrap();
        assert_eq!(loaded.clock, 70);
        assert_eq!(loaded.company.copper, 80);
        assert_eq!(loaded.company.day, 90);
    }

    #[test]
    fn a_zero_memory_state_removes_an_older_chunk_and_none_keeps_it() {
        let mut old = PlayerData::story(save_package::name("exanima01"));
        old.set_chunk(
            save_package::MEMORY_FLAGS_CHUNK,
            9u32.to_le_bytes().to_vec(),
        );
        old.set_chunk(
            save_package::TRANSITION_CHUNK,
            [2u32, 0, 0, 0].map(u32::to_le_bytes).concat(),
        );
        let bytes = old.write();
        let session = Session::story();
        let kept = PlayerData::parse(
            &player_section_after(&session, Some(&bytes)),
            Flavour::Story,
        )
        .unwrap();
        assert_eq!(kept.memory_flags(), 9);
        assert_eq!(kept.transition_state()[0], 2);
        let mut cleared = session;
        cleared.memory_flags = Some(0);
        cleared.transition = Some([0; 16]);
        let cut = PlayerData::parse(
            &player_section_after(&cleared, Some(&bytes)),
            Flavour::Story,
        )
        .unwrap();
        assert_eq!(cut.memory_flags(), 0);
        assert_eq!(cut.chunk(save_package::MEMORY_FLAGS_CHUNK), None);
        assert_eq!(cut.chunk(save_package::TRANSITION_CHUNK), None);
    }

    #[test]
    fn a_story_save_stores_its_locale_seed_and_start() {
        let mut session = Session::story();
        session.story.advance();
        session.story.advance();
        session.seed = 0x1234;
        session.start_background = 2;
        session.clock = 10;
        let bytes = write(&session);
        let package = exa_formats::save_package::PackageImage::read(&bytes).unwrap();
        let data =
            PlayerData::parse(package.section(PLAYER_DATA).unwrap(), Flavour::Story).unwrap();
        let PlayerHeader::Story(header) = &data.header else {
            panic!("story")
        };
        assert_eq!(header.locale, save_package::name("exanima03"));
        assert_eq!(header.seed, 0x1234);
        assert_eq!(header.flags, 0x10002);
        assert_eq!(header.clock, 10);
        assert_eq!(read(&bytes).unwrap().story.level(), "exanima03");
    }

    #[test]
    fn a_save_keeps_the_season_and_its_rewards() {
        let mut session = Session::career("Ash");
        let sweep = [crate::career::OUR_COMPANY; 6];
        for _ in 0..8 {
            session.company.end_day(sweep);
        }
        let loaded = read(&write(&session)).unwrap();
        assert_eq!(loaded.company.day, 8);
        assert_eq!(loaded.company.weeks, [true, true, false, false]);
        assert!(loaded.company.reward_ids.is_empty());
        assert_eq!(loaded.company.seasons_won, 0);
        assert_eq!(loaded.company.season_rewards, 0);
        assert_eq!(loaded.company.tournaments_won, 2);

        let mut session = loaded;
        for _ in 0..14 {
            session.company.end_day(sweep);
        }
        let loaded = read(&write(&session)).unwrap();
        assert_eq!(loaded.company.day, 22);
        assert_eq!(loaded.company.weeks, [false; 4]);
        assert_eq!(loaded.company.seasons_won, 1);
        assert_eq!(loaded.company.season_rewards, 4);
        assert_eq!(loaded.company.tournaments_won, 4);
        assert_eq!(
            loaded.company.reward_ids.as_slice(),
            &[1175, 1174, 1177, 1176]
        );
    }

    #[test]
    fn an_arena_session_writes_its_table_chunks_in_the_original_order() {
        use exa_formats::arena_save::{ArenaTables, ItemEntry};

        let mut session = Session::career("Ash");
        let tables = ArenaTables {
            shop_stock: vec![7, 8],
            stash: vec![ItemEntry {
                id: 3,
                flags: 0,
                extra: 0,
            }],
            ..ArenaTables::default()
        };
        session.arena_tables = Some(tables);
        let data = player_data(&session);
        let bytes = data.write();
        // The section starts with the 52-byte arena header.
        let chunks = save_package::parse_chunks(&bytes[52..]).unwrap();
        let ids: Vec<u32> = chunks.iter().map(|chunk| chunk.id).collect();
        // The header's placement is absent in an Exarust session, so the
        // arena chunks start at the arsenal; the optional lists sit before
        // the company and the always-written hireables and tournament.
        assert!(ids.contains(&save_package::ARSENAL_CHUNK));
        assert!(ids.contains(&save_package::HUB_STASH_CHUNK));
        assert!(ids.contains(&save_package::SHOP_STOCK_CHUNK));
        assert!(ids.contains(&save_package::COMPANY_CHUNK));
        assert!(ids.contains(&save_package::ARENA_EXTRA_CHUNK));
        assert!(ids.contains(&save_package::HIREABLES_CHUNK));
        assert!(ids.contains(&save_package::TOURNAMENT_CHUNK));
        assert!(!ids.contains(&save_package::EMPLOYEES_CHUNK));
        // No staff at all still writes the merchant, as the original does.
        assert!(ids.contains(&save_package::MERCHANT_CHUNK));
        let arsenal = chunks
            .iter()
            .find(|chunk| chunk.id == save_package::ARSENAL_CHUNK)
            .unwrap();
        // 25 empty lists.
        assert_eq!(arsenal.payload.len(), 25 * 4);
        let shop = chunks
            .iter()
            .find(|chunk| chunk.id == save_package::SHOP_STOCK_CHUNK)
            .unwrap();
        assert_eq!(
            exa_formats::arena_save::parse_shop_stock(shop.payload).unwrap(),
            vec![7, 8]
        );
    }

    #[test]
    fn the_session_clock_restores_and_stamps_the_world_clock() {
        let mut session = Session::story();
        session.clock = 8684;
        // The reader's setter: the saved seconds plus the seconds since the
        // load tick.
        let clock = session.world_clock(1_000);
        assert_eq!(crate::world_clock::stamped(&clock, 1_000), 8684);
        assert_eq!(crate::world_clock::stamped(&clock, 61_000), 8744);
        // The writer's stamp: the header carries the current world seconds.
        session.stamp_world_clock(&clock, 61_000);
        assert_eq!(session.clock, 8744);
        // Through a package the stamp survives, and the next load restores
        // it at its own tick.
        let back = read(&write(&session)).unwrap();
        assert_eq!(back.clock, 8744);
        let reloaded = back.world_clock(0);
        assert_eq!(crate::world_clock::stamped(&reloaded, 0), 8744);
        assert_eq!(crate::world_clock::stamped(&reloaded, 3_000), 8747);
        // The arena header carries the same clock.
        let mut arena = Session::career("Rival");
        arena.clock = 100;
        let clock = arena.world_clock(0);
        arena.stamp_world_clock(&clock, 5_000);
        assert_eq!(arena.clock, 105);
        assert_eq!(read(&write(&arena)).unwrap().clock, 105);
    }

    #[test]
    fn an_arena_save_carries_the_live_tables_and_reads_them_back() {
        use crate::arsenal::{Arsenal, ItemKind};
        use crate::hub_windows::ArenaCareer;
        use crate::staff::{COMBATANT, Staff};
        let mut career = ArenaCareer::default();
        career.company.name = "Rivals".to_string();
        career.company.copper = 700;
        career.company.stock = vec![11, 12];
        career.company.won = 4;
        career.matches_today = 1;
        career.merchant_stock = vec![13];
        career.log.append_value(2, 3);
        let mut staff = Staff::default();
        staff.employees.push(crate::staff::Employee {
            class: COMBATANT,
            kit: vec![21],
            world: Some(0x55),
            ..Default::default()
        });
        let mut arsenal = Arsenal::default();
        assert!(arsenal.stow(
            21,
            ItemKind {
                known: true,
                weapon: true,
                ..Default::default()
            },
            0x600
        ));
        let mut session = Session::career("Rivals");
        session.capture_arena(&career, &staff, &arsenal, &[31]);
        // The session's own fields still decide the header.
        session.company.copper = 700;
        let back = read(&write(&session)).unwrap();
        let tables = back.arena_tables.clone().expect("the arena chunks");
        assert_eq!(tables.company.name, b"Rivals");
        assert_eq!(tables.company.records[crate::arena_tables::RECORD_WON], 4);
        assert_eq!(tables.arsenal.lists[0][0][0].id, 21);
        assert_eq!(tables.stash[0].id, 31);
        assert_eq!(tables.shop_stock, vec![11, 12]);
        // A staff without a merchant skips the merchant chunk, as the
        // original's writer does, so the merchant's stock is not saved.
        assert!(tables.merchant.ids.is_empty());
        assert_eq!(tables.day.matches_today, 1);
        assert_eq!(tables.employees.employees.len(), 1);
        assert_eq!(tables.employees.employees[0].character, 0x55);
        // The writer's empty hireables is a zero count.
        assert_eq!(tables.hireables, 0u32.to_le_bytes());

        let mut loaded_career = ArenaCareer::default();
        let mut loaded_staff = Staff::default();
        let mut loaded_arsenal = Arsenal::default();
        let mut stash = Vec::new();
        assert!(back.apply_arena_tables(
            &mut loaded_career,
            &mut loaded_staff,
            &mut loaded_arsenal,
            &mut stash
        ));
        assert_eq!(stash, vec![31]);
        assert_eq!(loaded_arsenal, arsenal);
        assert_eq!(loaded_staff, staff);
        assert_eq!(loaded_career.company.won, 4);
        assert_eq!(loaded_career.log, career.log);
        // With no staff the merchant chunk is written, and its stock comes
        // back.
        let mut empty_staff = Session::career("Rivals");
        empty_staff.capture_arena(&career, &Staff::default(), &Arsenal::default(), &[]);
        let merchant = read(&write(&empty_staff)).unwrap();
        let tables = merchant.arena_tables.expect("the arena chunks");
        assert_eq!(tables.merchant.ids, vec![13]);
        assert!(tables.employees.employees.is_empty());
        // A session without tables applies none.
        assert!(!Session::career("Rival").apply_arena_tables(
            &mut loaded_career,
            &mut loaded_staff,
            &mut loaded_arsenal,
            &mut stash
        ));
    }
}
