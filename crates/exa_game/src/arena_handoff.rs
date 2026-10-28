//! The career the hub hands to a launched match and back.
//!
//! The original keeps one process: the hub opens the arena level, fights the
//! match and returns, all on the same career. Exarust changes level by
//! restarting the process ([`crate::level_transition`] records what blocks a
//! live change), so the career would be lost at the launch. This module is
//! Exarust's own device for that one gap: the hub writes the career and the
//! match's setup into a small hand-off file before the restart, the match's
//! process reads it back, and the settled career is written again for the
//! hub's process to restore. It is not a retail format and carries no game
//! data of its own.
//!
//! The file is little-endian: the magic `XAHC`, a version, a flag byte, then
//! the flag's parts. [`read`] is total: a short, malformed or newer file
//! gives `None`, never a panic. The default path is the system temporary
//! directory's `exarust-arena-handoff`; the tests take explicit paths.

use std::io::Write;
use std::path::{Path, PathBuf};

use exa_formats::characters::ItemRef;

use crate::arena::Entrant;
use crate::arena_log::{ArenaLog, ArenaRecord};
use crate::arena_match::ArenaPicks;
use crate::arena_session::ReturnPlacement;
use crate::arena_spawn::PlacedItem;
use crate::career::{Company, Member, Profession};
use crate::hub_windows::ArenaCareer;

/// The file's magic.
pub const MAGIC: u32 = 0x4348_4158;
/// The format version this module writes and reads.
pub const VERSION: u32 = 1;
/// The default file name in the temporary directory.
pub const FILE_NAME: &str = "exarust-arena-handoff";
/// Bit 0 of the flag byte: a career follows.
const HAS_CAREER: u8 = 1;
/// Bit 1: a launched match's setup follows.
const HAS_LAUNCH: u8 = 2;
/// Bit 2: the match returned, and its placement follows.
const HAS_RETURN: u8 = 4;

/// A launched match's setup, as the hub's board chose it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MatchSetup {
    pub template: i32,
    pub level: i32,
    pub tier: i32,
    pub tournament: bool,
    /// The company entry that is the player's own, as the launch planned it.
    pub player: Option<usize>,
    /// The match's prize when won, copper.
    pub prize: u32,
}

/// The career, the match and the return the two processes pass.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Handoff {
    pub career: Option<ArenaCareer>,
    pub launch: Option<MatchSetup>,
    /// The placement the settled match returned the company to.
    pub returned: Option<ReturnPlacement>,
}

/// The default hand-off file, in the system temporary directory.
#[must_use]
pub fn default_path() -> PathBuf {
    std::env::temp_dir().join(FILE_NAME)
}

/// Writes a hand-off to `path`, replacing any older one.
pub fn write(path: &Path, handoff: &Handoff) -> std::io::Result<()> {
    let bytes = encode(handoff);
    let mut file = std::fs::File::create(path)?;
    file.write_all(&bytes)
}

/// Reads a hand-off. `None` when the file is missing, short, malformed or of
/// another version.
#[must_use]
pub fn read(path: &Path) -> Option<Handoff> {
    let bytes = std::fs::read(path).ok()?;
    decode(&bytes)
}

/// Reads the default hand-off and removes it: a launch is used once.
#[must_use]
pub fn take() -> Option<Handoff> {
    take_at(&default_path())
}

/// [`take`] from `path`.
#[must_use]
pub fn take_at(path: &Path) -> Option<Handoff> {
    let handoff = read(path);
    let _ = std::fs::remove_file(path);
    handoff
}

/// Removes the default hand-off, as a dead manager's session does.
pub fn discard() {
    let _ = std::fs::remove_file(default_path());
}

/// Encodes a hand-off.
#[must_use]
pub fn encode(handoff: &Handoff) -> Vec<u8> {
    let mut out = Writer::default();
    out.u32(MAGIC);
    out.u32(VERSION);
    let flags = u8::from(handoff.career.is_some())
        | u8::from(handoff.launch.is_some()) << 1
        | u8::from(handoff.returned.is_some()) << 2;
    out.u8(flags);
    if let Some(career) = &handoff.career {
        career_bytes(&mut out, career);
    }
    if let Some(launch) = &handoff.launch {
        out.i32(launch.template);
        out.i32(launch.level);
        out.i32(launch.tier);
        out.u8(u8::from(launch.tournament));
        match launch.player {
            Some(player) => {
                out.u8(1);
                out.u32(u32::try_from(player).unwrap_or(u32::MAX));
            }
            None => out.u8(0),
        }
        out.u32(launch.prize);
    }
    if let Some(returned) = &handoff.returned {
        out.f32(returned.distance);
        out.f32(returned.heading);
        out.u32(returned.flags);
        out.u8(u8::from(returned.saves));
    }
    out.0
}

/// Decodes a hand-off. `None` for anything this version does not write.
#[must_use]
pub fn decode(bytes: &[u8]) -> Option<Handoff> {
    let mut r = Reader { bytes, at: 0 };
    if r.u32()? != MAGIC || r.u32()? != VERSION {
        return None;
    }
    let flags = r.u8()?;
    if flags & !(HAS_CAREER | HAS_LAUNCH | HAS_RETURN) != 0 {
        return None;
    }
    let career = if flags & HAS_CAREER != 0 {
        Some(career_from(&mut r)?)
    } else {
        None
    };
    let launch = if flags & HAS_LAUNCH != 0 {
        Some(MatchSetup {
            template: r.i32()?,
            level: r.i32()?,
            tier: r.i32()?,
            tournament: r.u8()? != 0,
            player: if r.u8()? != 0 {
                Some(r.u32()? as usize)
            } else {
                None
            },
            prize: r.u32()?,
        })
    } else {
        None
    };
    let returned = if flags & HAS_RETURN != 0 {
        Some(ReturnPlacement {
            distance: r.f32()?,
            heading: r.f32()?,
            flags: r.u32()?,
            saves: r.u8()? != 0,
        })
    } else {
        None
    };
    if r.at != bytes.len() {
        return None;
    }
    Some(Handoff {
        career,
        launch,
        returned,
    })
}

/// A little-endian byte writer.
#[derive(Default)]
struct Writer(Vec<u8>);

impl Writer {
    fn u8(&mut self, value: u8) {
        self.0.push(value);
    }
    fn u16(&mut self, value: u16) {
        self.0.extend_from_slice(&value.to_le_bytes());
    }
    fn u32(&mut self, value: u32) {
        self.0.extend_from_slice(&value.to_le_bytes());
    }
    fn i32(&mut self, value: i32) {
        self.0.extend_from_slice(&value.to_le_bytes());
    }
    fn u64(&mut self, value: u64) {
        self.0.extend_from_slice(&value.to_le_bytes());
    }
    fn f32(&mut self, value: f32) {
        self.0.extend_from_slice(&value.to_le_bytes());
    }
    fn bytes(&mut self, value: &[u8]) {
        self.u32(u32::try_from(value.len()).unwrap_or(u32::MAX));
        self.0.extend_from_slice(value);
    }
    fn text(&mut self, value: &str) {
        self.bytes(value.as_bytes());
    }
}

/// A bounds-checked little-endian reader.
struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl Reader<'_> {
    fn take(&mut self, len: usize) -> Option<&[u8]> {
        let end = self.at.checked_add(len)?;
        let slice = self.bytes.get(self.at..end)?;
        self.at = end;
        Some(slice)
    }
    fn u8(&mut self) -> Option<u8> {
        self.take(1).map(|b| b[0])
    }
    fn u16(&mut self) -> Option<u16> {
        Some(u16::from_le_bytes(self.take(2)?.try_into().ok()?))
    }
    fn u32(&mut self) -> Option<u32> {
        Some(u32::from_le_bytes(self.take(4)?.try_into().ok()?))
    }
    fn i32(&mut self) -> Option<i32> {
        Some(i32::from_le_bytes(self.take(4)?.try_into().ok()?))
    }
    fn u64(&mut self) -> Option<u64> {
        Some(u64::from_le_bytes(self.take(8)?.try_into().ok()?))
    }
    fn f32(&mut self) -> Option<f32> {
        Some(f32::from_le_bytes(self.take(4)?.try_into().ok()?))
    }
    /// A length-prefixed run; a length past the buffer is an error, so a
    /// corrupt count cannot make a huge allocation.
    fn bytes(&mut self) -> Option<Vec<u8>> {
        let len = usize::try_from(self.u32()?).ok()?;
        Some(self.take(len)?.to_vec())
    }
    fn text(&mut self) -> Option<String> {
        String::from_utf8(self.bytes()?).ok()
    }
    /// A count of entries; a count larger than the remaining bytes is an
    /// error.
    fn count(&mut self) -> Option<usize> {
        let count = usize::try_from(self.u32()?).ok()?;
        if count > self.bytes.len().saturating_sub(self.at) {
            return None;
        }
        Some(count)
    }
}

fn member_bytes(out: &mut Writer, member: &Member) {
    out.text(&member.name);
    out.u8(match member.profession {
        Profession::Combatant => 0,
        Profession::Merchant => 1,
        Profession::Physician => 2,
        Profession::Trainer => 3,
    });
    out.u32(member.xp);
    for skill in member.skills {
        out.u32(skill);
    }
    for xp in member.skill_xp {
        out.u16(xp);
    }
    for row in member.techniques {
        out.bytes(&row);
    }
    for loadout in &member.loadouts {
        out.u32(loadout.word);
        for hand in loadout.hands {
            item_ref(out, hand);
        }
        out.u32(u32::try_from(loadout.worn.len()).unwrap_or(u32::MAX));
        for worn in &loadout.worn {
            item_ref(out, *worn);
        }
    }
    out.u16(member.health);
    out.u8(u8::from(member.in_entry));
}

fn item_ref(out: &mut Writer, item: ItemRef) {
    out.u32(item.id);
    out.u32(item.flags);
}

fn member_from(r: &mut Reader<'_>) -> Option<Member> {
    let name = r.text()?;
    let profession = match r.u8()? {
        0 => Profession::Combatant,
        1 => Profession::Merchant,
        2 => Profession::Physician,
        3 => Profession::Trainer,
        _ => return None,
    };
    let xp = r.u32()?;
    let mut skills = [0u32; 6];
    for skill in &mut skills {
        *skill = r.u32()?;
    }
    let mut skill_xp = [0u16; 6];
    for xp in &mut skill_xp {
        *xp = r.u16()?;
    }
    let mut techniques = [[0u8; 6]; 6];
    for row in &mut techniques {
        let bytes = r.bytes()?;
        if bytes.len() != row.len() {
            return None;
        }
        row.copy_from_slice(&bytes);
    }
    let mut loadouts: [crate::loadout::CharacterLoadout; crate::loadout::RANKS] =
        Default::default();
    for loadout in &mut loadouts {
        loadout.word = r.u32()?;
        for hand in &mut loadout.hands {
            *hand = item_ref_from(r)?;
        }
        let count = r.count()?;
        let mut worn = Vec::with_capacity(count);
        for _ in 0..count {
            worn.push(item_ref_from(r)?);
        }
        loadout.worn = worn;
    }
    let health = r.u16()?;
    let in_entry = r.u8()? != 0;
    Some(Member {
        name,
        profession,
        xp,
        skills,
        skill_xp,
        techniques,
        loadouts,
        health,
        in_entry,
    })
}

fn item_ref_from(r: &mut Reader<'_>) -> Option<ItemRef> {
    Some(ItemRef {
        id: r.u32()?,
        flags: r.u32()?,
    })
}

fn career_bytes(out: &mut Writer, career: &ArenaCareer) {
    let company = &career.company;
    out.text(&company.name);
    out.u32(company.copper);
    out.u32(u32::try_from(company.roster.len()).unwrap_or(u32::MAX));
    for member in &company.roster {
        member_bytes(out, member);
    }
    for value in [
        company.won,
        company.lost,
        company.spent,
        company.earned,
        company.day,
        company.seasons_won,
        company.season_rewards,
        company.tournaments_won,
        company.deaths,
    ] {
        out.u32(value);
    }
    for week in company.weeks {
        out.u8(u8::from(week));
    }
    out.u32(u32::try_from(company.reward_ids.len()).unwrap_or(u32::MAX));
    for id in &company.reward_ids {
        out.u32(*id);
    }
    out.u32(u32::try_from(company.stock.len()).unwrap_or(u32::MAX));
    for id in &company.stock {
        out.u32(*id);
    }
    let emblem = company.emblem;
    for word in [
        emblem.layers,
        emblem.background,
        emblem.design,
        emblem.colours[0],
        emblem.colours[1],
        emblem.colours[2],
        emblem.colours[3],
    ] {
        out.u32(word);
    }
    member_bytes(out, &career.player);
    for row in &career.entries {
        out.u32(row.count);
        for entrant in &row.entries {
            out.u32(entrant.actor_id);
            out.u64(entrant.weapon[0]);
            out.u64(entrant.weapon[1]);
        }
        out.u32(row.team_id);
    }
    out.u32(u32::try_from(career.log.records.len()).unwrap_or(u32::MAX));
    for record in &career.log.records {
        out.i32(record.id);
        for word in record.words {
            out.i32(word);
        }
    }
    out.i32(career.picks.variety);
    for value in career.picks.indoor {
        out.i32(value);
    }
    for value in career.picks.outdoor {
        out.i32(value);
    }
    for value in career.picks.tracks {
        out.i32(value);
    }
    out.u32(u32::try_from(career.merchant_stock.len()).unwrap_or(u32::MAX));
    for id in &career.merchant_stock {
        out.u32(*id);
    }
    out.u32(u32::try_from(career.venue_items.len()).unwrap_or(u32::MAX));
    for item in &career.venue_items {
        out.f32(item.position[0]);
        out.f32(item.position[1]);
        out.u32(item.type_bits);
        out.u32(item.template_kind);
    }
    out.u32(career.roll);
    out.u32(career.matches_today);
    out.u8(u8::from(career.warn_manager));
}

fn career_from(r: &mut Reader<'_>) -> Option<ArenaCareer> {
    let name = r.text()?;
    let copper = r.u32()?;
    let roster_len = r.count()?;
    let mut roster = Vec::with_capacity(roster_len.min(256));
    for _ in 0..roster_len {
        roster.push(member_from(r)?);
    }
    let mut words = [0u32; 9];
    for word in &mut words {
        *word = r.u32()?;
    }
    let mut weeks = [false; 4];
    for week in &mut weeks {
        *week = r.u8()? != 0;
    }
    let rewards = r.count()?;
    let mut reward_ids = Vec::with_capacity(rewards.min(256));
    for _ in 0..rewards {
        reward_ids.push(r.u32()?);
    }
    let stock_len = r.count()?;
    let mut stock = Vec::with_capacity(stock_len.min(256));
    for _ in 0..stock_len {
        stock.push(r.u32()?);
    }
    let mut emblem_words = [0u32; 7];
    for word in &mut emblem_words {
        *word = r.u32()?;
    }
    let company = Company {
        name,
        copper,
        roster,
        won: words[0],
        lost: words[1],
        spent: words[2],
        earned: words[3],
        day: words[4],
        weeks,
        seasons_won: words[5],
        season_rewards: words[6],
        reward_ids,
        tournaments_won: words[7],
        deaths: words[8],
        stock,
        emblem: exa_ui::EmblemRecord {
            layers: emblem_words[0],
            background: emblem_words[1],
            design: emblem_words[2],
            colours: [
                emblem_words[3],
                emblem_words[4],
                emblem_words[5],
                emblem_words[6],
            ],
        },
    };
    let player = member_from(r)?;
    let mut entries: crate::arena::Roster = Default::default();
    for row in &mut entries {
        row.count = r.u32()?;
        for entrant in &mut row.entries {
            *entrant = Entrant {
                actor_id: r.u32()?,
                weapon: [r.u64()?, r.u64()?],
            };
        }
        row.team_id = r.u32()?;
    }
    let log_len = r.count()?;
    let mut log = ArenaLog::default();
    for _ in 0..log_len {
        let mut record = ArenaRecord {
            id: r.i32()?,
            words: [0; 4],
        };
        for word in &mut record.words {
            *word = r.i32()?;
        }
        log.records.push(record);
    }
    let mut picks = ArenaPicks {
        variety: r.i32()?,
        ..Default::default()
    };
    for value in &mut picks.indoor {
        *value = r.i32()?;
    }
    for value in &mut picks.outdoor {
        *value = r.i32()?;
    }
    for value in &mut picks.tracks {
        *value = r.i32()?;
    }
    let stock_len = r.count()?;
    let mut merchant_stock = Vec::with_capacity(stock_len.min(256));
    for _ in 0..stock_len {
        merchant_stock.push(r.u32()?);
    }
    let venue_len = r.count()?;
    let mut venue_items = Vec::with_capacity(venue_len.min(256));
    for _ in 0..venue_len {
        venue_items.push(PlacedItem {
            position: [r.f32()?, r.f32()?],
            type_bits: r.u32()?,
            template_kind: r.u32()?,
        });
    }
    let roll = r.u32()?;
    let matches_today = r.u32()?;
    let warn_manager = r.u8()? != 0;
    Some(ArenaCareer {
        company,
        player,
        entries,
        log,
        matches_today,
        picks,
        merchant_stock,
        venue_items,
        roll,
        warn_manager,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn career() -> ArenaCareer {
        let mut career = ArenaCareer::default();
        career.company.name = "The Company".into();
        career.company.copper = 1234;
        career.company.roster.push(Member {
            name: "Ash".into(),
            profession: Profession::Physician,
            xp: 7,
            skills: [1, 2, 3, 4, 5, 6],
            skill_xp: [10, 20, 30, 40, 50, 60],
            techniques: [[1; 6]; 6],
            loadouts: Default::default(),
            health: 65_000,
            in_entry: true,
        });
        career.company.roster[0].loadouts[2].hands[0] = ItemRef::new(0x8000_0001, 4);
        career.company.roster[0].loadouts[2].worn = vec![ItemRef::new(0x1234, 0)];
        career.player.name = "Manager".into();
        career.player.profession = Profession::Trainer;
        career.entries[0].count = 1;
        career.entries[0].team_id = 1;
        career.entries[0].entries[1] = Entrant {
            actor_id: 3,
            weapon: [5, 6],
        };
        career.log.append_value(4, 9);
        career.picks.variety = 2;
        career.picks.indoor[1] = 6;
        career.picks.outdoor[2] = 4;
        career.picks.tracks[0] = 2;
        career.merchant_stock = vec![1, 2];
        career.venue_items.push(PlacedItem {
            position: [-2100.0, 3.5],
            type_bits: 0x4000,
            template_kind: 0x40000,
        });
        career.roll = 0x1234_5678;
        career.matches_today = 2;
        career.warn_manager = false;
        career
    }

    #[test]
    fn a_career_round_trips_with_the_match_and_the_return() {
        let handoff = Handoff {
            career: Some(career()),
            launch: Some(MatchSetup {
                template: 9,
                level: 3,
                tier: 2,
                tournament: true,
                player: Some(0),
                prize: 12,
            }),
            returned: Some(crate::arena_session::RETURN_PLACEMENT),
        };
        let bytes = encode(&handoff);
        assert_eq!(decode(&bytes), Some(handoff));
    }

    #[test]
    fn an_empty_handoff_round_trips_and_keeps_its_flags() {
        let handoff = Handoff::default();
        let bytes = encode(&handoff);
        assert_eq!(bytes.len(), 9);
        assert_eq!(decode(&bytes), Some(handoff));
    }

    #[test]
    fn a_truncated_or_corrupt_file_decodes_to_nothing() {
        let bytes = encode(&Handoff {
            career: Some(career()),
            ..Default::default()
        });
        for len in 0..bytes.len() {
            assert_eq!(decode(&bytes[..len]), None, "{len}");
        }
        let mut wrong_magic = bytes.clone();
        wrong_magic[0] ^= 0xFF;
        assert_eq!(decode(&wrong_magic), None);
        let mut wrong_version = bytes.clone();
        wrong_version[4] = 2;
        assert_eq!(decode(&wrong_version), None);
        let mut extra = bytes.clone();
        extra.push(0);
        assert_eq!(decode(&extra), None);
        // A length prefix past the buffer is refused before it allocates.
        let mut corrupt = encode(&Handoff {
            career: Some(career()),
            ..Default::default()
        });
        corrupt[9..13].copy_from_slice(&u32::MAX.to_le_bytes());
        assert_eq!(decode(&corrupt), None);
    }

    #[test]
    fn a_match_setup_without_a_player_keeps_none() {
        let handoff = Handoff {
            launch: Some(MatchSetup {
                template: 1,
                level: 0,
                tier: 0,
                tournament: false,
                player: None,
                prize: 0,
            }),
            ..Default::default()
        };
        assert_eq!(decode(&encode(&handoff)), Some(handoff));
    }

    #[test]
    fn the_file_is_written_read_and_taken_once() {
        let dir = std::env::temp_dir().join(format!("exarust-handoff-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("handoff");
        let handoff = Handoff {
            career: Some(career()),
            ..Default::default()
        };
        write(&path, &handoff).unwrap();
        assert_eq!(read(&path), Some(handoff));
        std::fs::remove_file(&path).unwrap();
        assert_eq!(read(&path), None);
        let _ = std::fs::remove_dir(&dir);
    }
}
