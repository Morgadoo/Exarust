//! An Exarust save of the session. The original's save packages are not read.
//!
//! The file starts with the four bytes `EXRS` and a version of 1. It stores
//! the mode, the story level, the journal, the topics already chosen, the
//! company and its roster, and the caster's focus. A short, mistyped or
//! otherwise malformed file is an error.

use exa_script::story::{Entry, Journal, Story};

use crate::career::{Company, Member, Profession};

const MAGIC: &[u8; 4] = b"EXRS";
const VERSION: u16 = 1;

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
}

impl Session {
    pub fn story() -> Self {
        Self {
            mode: Mode::Story,
            story: Story::begin(),
            company: Company::new(""),
            focus: 1.0,
        }
    }

    pub fn career(name: impl Into<String>) -> Self {
        Self {
            mode: Mode::Arena,
            story: Story::begin(),
            company: Company::new(name),
            focus: 1.0,
        }
    }
}

/// Why a file is not a session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SaveError {
    /// The bytes are not an Exarust save, or they cut a field short.
    Malformed,
}

/// Write the session.
pub fn write(session: &Session) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(MAGIC);
    out.extend_from_slice(&VERSION.to_le_bytes());
    out.push(match session.mode {
        Mode::Story => 0,
        Mode::Arena => 1,
    });
    out.extend_from_slice(&(session.story.index() as u16).to_le_bytes());
    put_str(&mut out, &session.company.name);
    out.extend_from_slice(&session.company.copper.to_le_bytes());
    out.extend_from_slice(&session.company.won.to_le_bytes());
    out.extend_from_slice(&session.company.lost.to_le_bytes());
    out.extend_from_slice(&session.company.spent.to_le_bytes());
    out.extend_from_slice(&session.company.earned.to_le_bytes());
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

/// Read a session. Malformed input is an error.
pub fn read(bytes: &[u8]) -> Result<Session, SaveError> {
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
    let company_name = take_str(&mut rest)?.to_string();
    let copper = u32::from_le_bytes(take(&mut rest, 4)?.try_into().unwrap());
    let won = u32::from_le_bytes(take(&mut rest, 4)?.try_into().unwrap());
    let lost = u32::from_le_bytes(take(&mut rest, 4)?.try_into().unwrap());
    let spent = u32::from_le_bytes(take(&mut rest, 4)?.try_into().unwrap());
    let earned = u32::from_le_bytes(take(&mut rest, 4)?.try_into().unwrap());
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
            day: 0,
            weeks: [false; 4],
            seasons_won: 0,
            season_rewards: 0,
            tournaments_won: 0,
        },
        focus,
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
        let bytes = write(&session);
        let mut loaded = read(&bytes).unwrap();
        assert_eq!(loaded, session);
        assert_eq!(loaded.story.level(), "exanima02");
        assert_eq!(loaded.story.journal().entries()[0].text, "The pit is open.");
        assert_eq!(
            loaded.story.pick(GATE, 0),
            Err(exa_script::story::TalkError::Used)
        );
        assert_eq!(loaded.company.copper, 343 - 12 - 3 + 256);
        assert_eq!(loaded.company.roster[0].xp, 1000);
        assert_eq!(loaded.focus, 0.5);

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
}
