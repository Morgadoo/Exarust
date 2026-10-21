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
//! A short, mistyped or otherwise malformed file is an error.

use exa_formats::save_package::{
    self, ARENA_DATA_VERSION, EXARUST_CHUNK, Flavour, JournalRecord, PLAYER_DATA, PackageImage,
    PlayerData, PlayerHeader,
};
use exa_script::story::{Entry, Journal, Story};

use crate::career::{Company, Member, Profession};

const MAGIC: &[u8; 4] = b"EXRS";
const VERSION: u16 = 1;

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
                header.flags = (session.start_background & 0xFF) | 0x1_0000;
                header.clock = session.clock;
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
            data
        }
    };
    data.set_journal(&journal_records(&session.story));
    data
}

/// The `#playerdata` section a session writes: the mode's header, the
/// journal and the Exarust chunk with everything else. A previous section's
/// character placement is carried over when the session has none (the
/// original's arena writer does this for a save made with no scene loaded).
pub fn player_section_after(session: &Session, previous: Option<&[u8]>) -> Vec<u8> {
    let mut data = player_data(session);
    if data.character.is_none()
        && let Some(bytes) = previous
        && let Ok(old) = PlayerData::parse(bytes, flavour_of(bytes))
    {
        data.character = old.character;
    }
    data.set_chunk(EXARUST_CHUNK, write_body(session));
    data.write()
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
    Ok(info_of(&data))
}

fn info_of(data: &PlayerData) -> SlotInfo {
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
    }
}

/// Write the session: a package with its `#playerdata` section.
pub fn write(session: &Session) -> Vec<u8> {
    package(session)
}

/// Read a session. Malformed input is an error.
pub fn read(bytes: &[u8]) -> Result<Session, SaveError> {
    let package = PackageImage::read(bytes).map_err(|_| SaveError::Malformed)?;
    let section = package.section(PLAYER_DATA).ok_or(SaveError::Malformed)?;
    let data = PlayerData::parse(section, flavour_of(section)).map_err(|_| SaveError::Malformed)?;
    let body = data.chunk(EXARUST_CHUNK).ok_or(SaveError::Malformed)?;
    let session = read_body(body)?;
    let matches = matches!(
        (session.mode, data.header.flavour()),
        (Mode::Story, Flavour::Story) | (Mode::Arena, Flavour::Arena)
    );
    if !matches {
        return Err(SaveError::Malformed);
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
        },
        focus,
        clock,
        seed,
        start_background,
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
        let bytes = write(&session);
        let mut loaded = read(&bytes).unwrap();
        assert_eq!(loaded, session);
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
}
