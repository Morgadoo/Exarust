//! The arena career's own save chunks: the company, the day, and the
//! tournament.
//!
//! Each is framed as `u32 id, u32 length, payload` (`save_package`). A
//! string is a `u32` byte count and the bytes (no terminator); a count of 0
//! is the empty string. Every other field is a little-endian word or a
//! fixed run of bytes:
//!
//! * company (`0xA0DC0000`): the name, a second string (empty in a new
//!   career), the 28-byte emblem record, three words, one unused word,
//!   then the sixteen record words (the thirteen the records window lists
//!   and three never written).
//! * day (`0xA0D10000`): today's matches, copper spent and earned, the
//!   special matches (a count, then per match its template id, rank, flags
//!   and an unused word), a word kept beside the log, then the log (a
//!   count and that many twenty-byte records).
//! * tournament (`0xA0600000`): the week's prize word, the six event rows
//!   (0x120 bytes), the sixteen rival companies (64 bytes), each event's
//!   template id, rank and two more words, then 16 and 32 bytes of the
//!   season's weeks and the prize history.
//!
//! Evidence: the arena save writers and loaders read from `Exanima.exe`
//! 0.9.5; no shipped file holds these chunks (they are written into the
//! player's own saves), so the layout is checked against synthetic bytes.

use std::fmt;

/// The company chunk's id.
pub const COMPANY_CHUNK: u32 = 0xA0DC_0000;
/// The day chunk's id.
pub const DAY_CHUNK: u32 = 0xA0D1_0000;
/// The tournament chunk's id.
pub const TOURNAMENT_CHUNK: u32 = 0xA060_0000;
/// The emblem record's size.
pub const EMBLEM_LEN: usize = 0x1c;
/// A log record's size.
pub const LOG_RECORD_LEN: usize = 0x14;
/// The six event rows' size.
pub const EVENT_ROWS_LEN: usize = 0x120;
/// Rival companies in a tournament field.
pub const FIELD_LEN: usize = 16;
/// Tournament events in a week.
pub const EVENTS: usize = 6;

/// Why an arena chunk does not read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArenaSaveError {
    /// The payload ends inside a field.
    Short,
}

impl fmt::Display for ArenaSaveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "arena chunk ends inside a field")
    }
}

impl std::error::Error for ArenaSaveError {}

struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl<'a> Reader<'a> {
    fn take(&mut self, len: usize) -> Result<&'a [u8], ArenaSaveError> {
        let end = self.at.checked_add(len).ok_or(ArenaSaveError::Short)?;
        let bytes = self.bytes.get(self.at..end).ok_or(ArenaSaveError::Short)?;
        self.at = end;
        Ok(bytes)
    }

    fn word(&mut self) -> Result<u32, ArenaSaveError> {
        let bytes = self.take(4)?;
        Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    fn string(&mut self) -> Result<Vec<u8>, ArenaSaveError> {
        let len = self.word()? as usize;
        Ok(self.take(len)?.to_vec())
    }

    fn array<const N: usize>(&mut self) -> Result<[u8; N], ArenaSaveError> {
        let mut out = [0u8; N];
        out.copy_from_slice(self.take(N)?);
        Ok(out)
    }

    fn words<const N: usize>(&mut self) -> Result<[u32; N], ArenaSaveError> {
        let mut out = [0u32; N];
        for word in &mut out {
            *word = self.word()?;
        }
        Ok(out)
    }
}

fn put(out: &mut Vec<u8>, word: u32) {
    out.extend_from_slice(&word.to_le_bytes());
}

fn put_string(out: &mut Vec<u8>, text: &[u8]) {
    put(out, text.len() as u32);
    out.extend_from_slice(text);
}

/// The company chunk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompanyChunk {
    pub name: Vec<u8>,
    /// A second string, empty in a new career (what it names is not traced).
    pub second: Vec<u8>,
    pub emblem: [u8; EMBLEM_LEN],
    /// Three words a new career clears (what they hold is not traced).
    pub words: [u32; 3],
    /// The sixteen record words: matches won, lost, forfeited, tournaments
    /// won, lost, seasons won, beasts fought, defeated, hires, deaths,
    /// fired, copper earned, spent, and three unused.
    pub records: [u32; 16],
}

impl Default for CompanyChunk {
    fn default() -> Self {
        Self {
            name: Vec::new(),
            second: Vec::new(),
            emblem: [0; EMBLEM_LEN],
            words: [0; 3],
            records: [0; 16],
        }
    }
}

/// Read a company chunk's payload. The unused word is skipped.
pub fn parse_company(bytes: &[u8]) -> Result<CompanyChunk, ArenaSaveError> {
    let mut r = Reader { bytes, at: 0 };
    let name = r.string()?;
    let second = r.string()?;
    let emblem = r.array::<EMBLEM_LEN>()?;
    let words = r.words::<3>()?;
    r.take(4)?;
    let records = r.words::<16>()?;
    Ok(CompanyChunk {
        name,
        second,
        emblem,
        words,
        records,
    })
}

/// Write a company chunk's payload; the unused word is 0.
pub fn write_company(chunk: &CompanyChunk) -> Vec<u8> {
    let mut out = Vec::new();
    put_string(&mut out, &chunk.name);
    put_string(&mut out, &chunk.second);
    out.extend_from_slice(&chunk.emblem);
    for &word in &chunk.words {
        put(&mut out, word);
    }
    put(&mut out, 0);
    for &word in &chunk.records {
        put(&mut out, word);
    }
    out
}

/// One special match the day keeps.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SpecialEntry {
    pub template: u32,
    pub rank: u32,
    pub flags: u32,
}

/// The day chunk.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DayChunk {
    pub matches_today: u32,
    pub spent_today: u32,
    pub earned_today: u32,
    pub specials: Vec<SpecialEntry>,
    /// The word written beside the log (what it counts is not traced).
    pub log_word: u32,
    pub log: Vec<[u8; LOG_RECORD_LEN]>,
}

/// Read a day chunk's payload. Each special's last word is skipped.
pub fn parse_day(bytes: &[u8]) -> Result<DayChunk, ArenaSaveError> {
    let mut r = Reader { bytes, at: 0 };
    let matches_today = r.word()?;
    let spent_today = r.word()?;
    let earned_today = r.word()?;
    let count = r.word()? as i32;
    let mut specials = Vec::new();
    for _ in 0..count.max(0) {
        let template = r.word()?;
        let rank = r.word()?;
        let flags = r.word()?;
        r.take(4)?;
        specials.push(SpecialEntry {
            template,
            rank,
            flags,
        });
    }
    let log_word = r.word()?;
    let records = r.word()? as i32;
    let mut log = Vec::new();
    for _ in 0..records.max(0) {
        log.push(r.array::<LOG_RECORD_LEN>()?);
    }
    Ok(DayChunk {
        matches_today,
        spent_today,
        earned_today,
        specials,
        log_word,
        log,
    })
}

/// Write a day chunk's payload.
pub fn write_day(chunk: &DayChunk) -> Vec<u8> {
    let mut out = Vec::new();
    put(&mut out, chunk.matches_today);
    put(&mut out, chunk.spent_today);
    put(&mut out, chunk.earned_today);
    put(&mut out, chunk.specials.len() as u32);
    for special in &chunk.specials {
        put(&mut out, special.template);
        put(&mut out, special.rank);
        put(&mut out, special.flags);
        put(&mut out, 0);
    }
    put(&mut out, chunk.log_word);
    put(&mut out, chunk.log.len() as u32);
    for record in &chunk.log {
        out.extend_from_slice(record);
    }
    out
}

/// One tournament event's saved words.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct EventEntry {
    pub template: u32,
    pub rank: u32,
    pub third: u32,
    pub fourth: u32,
}

/// The tournament chunk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TournamentChunk {
    pub prize: u32,
    pub rows: [u8; EVENT_ROWS_LEN],
    pub field: [u32; FIELD_LEN],
    pub events: [EventEntry; EVENTS],
    pub weeks: [u32; 4],
    pub history: [u32; 8],
}

impl Default for TournamentChunk {
    fn default() -> Self {
        Self {
            prize: 0,
            rows: [0; EVENT_ROWS_LEN],
            field: [0; FIELD_LEN],
            events: [EventEntry::default(); EVENTS],
            weeks: [0; 4],
            history: [0; 8],
        }
    }
}

impl TournamentChunk {
    /// A loader that does not keep the week's prize clears it; one whose
    /// field's first company is 0 draws a new field.
    pub fn needs_field(&self) -> bool {
        self.field[0] == 0
    }
}

/// Read a tournament chunk's payload.
pub fn parse_tournament(bytes: &[u8]) -> Result<TournamentChunk, ArenaSaveError> {
    let mut r = Reader { bytes, at: 0 };
    let prize = r.word()?;
    let rows = r.array::<EVENT_ROWS_LEN>()?;
    let field = r.words::<FIELD_LEN>()?;
    let mut events = [EventEntry::default(); EVENTS];
    for event in &mut events {
        *event = EventEntry {
            template: r.word()?,
            rank: r.word()?,
            third: r.word()?,
            fourth: r.word()?,
        };
    }
    let weeks = r.words::<4>()?;
    let history = r.words::<8>()?;
    Ok(TournamentChunk {
        prize,
        rows,
        field,
        events,
        weeks,
        history,
    })
}

/// Write a tournament chunk's payload.
pub fn write_tournament(chunk: &TournamentChunk) -> Vec<u8> {
    let mut out = Vec::new();
    put(&mut out, chunk.prize);
    out.extend_from_slice(&chunk.rows);
    for &word in &chunk.field {
        put(&mut out, word);
    }
    for event in &chunk.events {
        put(&mut out, event.template);
        put(&mut out, event.rank);
        put(&mut out, event.third);
        put(&mut out, event.fourth);
    }
    for &word in chunk.weeks.iter().chain(&chunk.history) {
        put(&mut out, word);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::save_package::{parse_chunks, write_chunk};

    #[test]
    fn a_company_round_trips_through_its_chunk() {
        let mut chunk = CompanyChunk {
            name: b"Ash Company".to_vec(),
            ..CompanyChunk::default()
        };
        chunk.emblem[0] = 52;
        chunk.emblem[27] = 81;
        chunk.records[0] = 3;
        chunk.records[12] = 343;
        let payload = write_company(&chunk);
        // 4 + 11, 4 + 0, 28, 3 × 4, 4 unused, 16 × 4.
        assert_eq!(payload.len(), 15 + 4 + 28 + 12 + 4 + 64);
        assert_eq!(&payload[..4], &11u32.to_le_bytes());
        assert_eq!(&payload[15..19], &[0, 0, 0, 0]);
        let mut framed = Vec::new();
        write_chunk(&mut framed, COMPANY_CHUNK, &payload);
        assert_eq!(&framed[..4], &[0x00, 0x00, 0xdc, 0xa0]);
        let chunks = parse_chunks(&framed).unwrap();
        assert_eq!(parse_company(chunks[0].payload).unwrap(), chunk);
        // The unused word is skipped on reading.
        let mut odd = payload.clone();
        odd[15 + 4 + 28 + 12] = 9;
        assert_eq!(parse_company(&odd).unwrap(), chunk);
        assert_eq!(
            parse_company(&payload[..payload.len() - 1]),
            Err(ArenaSaveError::Short)
        );
        assert_eq!(
            parse_company(&[0xff, 0xff, 0xff, 0xff]),
            Err(ArenaSaveError::Short)
        );
    }

    #[test]
    fn a_day_round_trips_with_its_specials_and_log() {
        let chunk = DayChunk {
            matches_today: 2,
            spent_today: 12,
            earned_today: 36,
            specials: vec![
                SpecialEntry {
                    template: 8,
                    rank: 3,
                    flags: 2,
                },
                SpecialEntry {
                    template: 11,
                    rank: 5,
                    flags: 0,
                },
            ],
            log_word: 1,
            log: vec![[7; LOG_RECORD_LEN]],
        };
        let payload = write_day(&chunk);
        assert_eq!(payload.len(), 16 + 2 * 16 + 8 + 20);
        assert_eq!(parse_day(&payload).unwrap(), chunk);
        assert_eq!(parse_day(&payload[..20]), Err(ArenaSaveError::Short));
        // A negative count reads no specials.
        let mut empty = vec![0u8; 12];
        empty.extend_from_slice(&(-1i32).to_le_bytes());
        empty.extend_from_slice(&[0; 8]);
        assert!(parse_day(&empty).unwrap().specials.is_empty());
    }

    #[test]
    fn a_tournament_round_trips() {
        let mut chunk = TournamentChunk {
            prize: 4,
            ..TournamentChunk::default()
        };
        assert!(chunk.needs_field());
        chunk.field[0] = 17;
        chunk.rows[0x2c] = 3;
        chunk.events[5] = EventEntry {
            template: 7,
            rank: 5,
            third: 1,
            fourth: 2,
        };
        chunk.history[7] = 1172;
        let payload = write_tournament(&chunk);
        assert_eq!(payload.len(), 4 + 0x120 + 64 + 6 * 16 + 16 + 32);
        let back = parse_tournament(&payload).unwrap();
        assert_eq!(back, chunk);
        assert!(!back.needs_field());
        assert_eq!(
            parse_tournament(&payload[..100]),
            Err(ArenaSaveError::Short)
        );
    }
}
