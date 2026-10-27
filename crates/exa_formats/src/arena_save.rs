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
//! * employees (`0xA0400000`): written only while someone is employed; a
//!   count, then each employee's two unused words, profession word, a
//!   custom-fighter flag (a flagged entry carries no character id), the
//!   character id otherwise, an item list (a count and twelve-byte entries)
//!   and four words. When it loads a character, the original also resets
//!   the profession word from the character's class (16, 4 or 8 for the
//!   upper nibbles 3, 4 and 5); that step needs the character database and
//!   is the caller's.
//! * hireables (`0x00010000`): a count and, per candidate, an unused word
//!   then the candidate's character export, written whenever the writer
//!   runs (an empty list is just a zero count). The export's own codec
//!   delimits it.
//! * merchant (`0xA0500000`): written only while a merchant is employed:
//!   the stock type, the specialisation word, two unused words, and the
//!   stock's count and item ids. A loader that skips stock reads only the
//!   header.
//!
//! Evidence: the arena save writers and loaders read from `Exanima.exe`
//! 0.9.5; no shipped file holds these chunks (they are written into the
//! player's own saves), so the layout is checked against synthetic bytes.

use std::fmt;

use crate::save_package::{Chunk, write_chunk};

/// The company chunk's id.
pub const COMPANY_CHUNK: u32 = 0xA0DC_0000;
/// The day chunk's id.
pub const DAY_CHUNK: u32 = 0xA0D1_0000;
/// The tournament chunk's id.
pub const TOURNAMENT_CHUNK: u32 = 0xA060_0000;
/// The employees chunk's id.
pub const EMPLOYEES_CHUNK: u32 = 0xA040_0000;
/// The merchant chunk's id.
pub const MERCHANT_CHUNK: u32 = 0xA050_0000;
/// The shop stock chunk's id.
pub const SHOP_STOCK_CHUNK: u32 = 0xA030_0000;
/// The hireable candidates' chunk id.
pub const HIREABLES_CHUNK: u32 = 0x0001_0000;
/// One item reference in an employee's own list.
pub const ITEM_ENTRY_LEN: usize = 12;
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

/// One twelve-byte item reference of an employee's own list: the item, its
/// flags and a third word the original carries without reading.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ItemEntry {
    pub id: u32,
    pub flags: u32,
    pub extra: u32,
}

impl ItemEntry {
    fn parse(bytes: &[u8]) -> Self {
        Self {
            id: u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
            flags: u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]),
            extra: u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]),
        }
    }

    fn write(&self, out: &mut Vec<u8>) {
        put(out, self.id);
        put(out, self.flags);
        put(out, self.extra);
    }
}

/// An employed specialist or fighter as the employees chunk keeps one.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct EmployeeRecord {
    /// The profession word: the class is its low byte.
    pub class: u32,
    /// A custom fighter (a recruit): the entry carries no character id.
    pub custom: bool,
    /// The world character id of a candidate hired from the list.
    pub character: u32,
    /// The employee's own items.
    pub items: Vec<ItemEntry>,
    /// Four words kept after the item list.
    pub words: [u32; 4],
}

/// The employees chunk. An empty staff writes no chunk at all.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct EmployeesChunk {
    pub employees: Vec<EmployeeRecord>,
}

/// Read an employees chunk's payload. A negative count reads no employees.
pub fn parse_employees(bytes: &[u8]) -> Result<EmployeesChunk, ArenaSaveError> {
    let mut r = Reader { bytes, at: 0 };
    let count = r.word()? as i32;
    let mut employees = Vec::new();
    for _ in 0..count.max(0) {
        r.take(4)?;
        r.take(4)?;
        let class = r.word()?;
        let custom = r.word()? & 1 != 0;
        let character = if custom { 0 } else { r.word()? };
        let entries = r.word()? as i32;
        let mut items = Vec::new();
        for _ in 0..entries.max(0) {
            items.push(ItemEntry::parse(r.take(ITEM_ENTRY_LEN)?));
        }
        let words = r.words::<4>()?;
        employees.push(EmployeeRecord {
            class,
            custom,
            character,
            items,
            words,
        });
    }
    Ok(EmployeesChunk { employees })
}

/// Write an employees chunk's payload.
pub fn write_employees(chunk: &EmployeesChunk) -> Vec<u8> {
    let mut out = Vec::new();
    put(&mut out, chunk.employees.len() as u32);
    for employee in &chunk.employees {
        put(&mut out, 0);
        put(&mut out, 0);
        put(&mut out, employee.class);
        put(&mut out, u32::from(employee.custom));
        if !employee.custom {
            put(&mut out, employee.character);
        }
        put(&mut out, employee.items.len() as u32);
        for item in &employee.items {
            item.write(&mut out);
        }
        for &word in &employee.words {
            put(&mut out, word);
        }
    }
    out
}

/// The merchant's stock chunk.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MerchantChunk {
    /// What kind of stock the merchant carries.
    pub stock_type: u32,
    /// The specialisation word (kept raw: the original starts it at −1).
    pub specialisation: u32,
    /// The two unused words between the specialisation and the count.
    pub unused: [u32; 2],
    /// The stock's item ids.
    pub ids: Vec<u32>,
}

/// Read a merchant chunk's payload. `keep_stock` is false on the load path
/// that regenerates the stock: only the header is read then.
pub fn parse_merchant(bytes: &[u8], keep_stock: bool) -> Result<MerchantChunk, ArenaSaveError> {
    let mut r = Reader { bytes, at: 0 };
    let stock_type = r.word()?;
    let specialisation = r.word()?;
    let unused = r.words::<2>()?;
    let count = if keep_stock { r.word()? as i32 } else { 0 };
    let mut ids = Vec::new();
    for _ in 0..count.max(0) {
        ids.push(r.word()?);
    }
    Ok(MerchantChunk {
        stock_type,
        specialisation,
        unused,
        ids,
    })
}

/// Write a merchant chunk's payload.
pub fn write_merchant(chunk: &MerchantChunk) -> Vec<u8> {
    let mut out = Vec::new();
    put(&mut out, chunk.stock_type);
    put(&mut out, chunk.specialisation);
    for &word in &chunk.unused {
        put(&mut out, word);
    }
    put(&mut out, chunk.ids.len() as u32);
    for &id in &chunk.ids {
        put(&mut out, id);
    }
    out
}

/// Read the shop's stock chunk: a signed count and that many item ids. A
/// negative count reads no ids.
pub fn parse_shop_stock(bytes: &[u8]) -> Result<Vec<u32>, ArenaSaveError> {
    let mut r = Reader { bytes, at: 0 };
    let count = r.word()? as i32;
    let mut ids = Vec::new();
    for _ in 0..count.max(0) {
        ids.push(r.word()?);
    }
    Ok(ids)
}

/// Write the shop's stock chunk: the count and the ids.
pub fn write_shop_stock(ids: &[u32]) -> Vec<u8> {
    let mut out = Vec::new();
    put(&mut out, ids.len() as u32);
    for &id in ids {
        put(&mut out, id);
    }
    out
}

/// One hire-list candidate: the unused word the chunk writes and the
/// candidate's encoded character.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Hireable<T> {
    pub unused: u32,
    pub character: T,
}

/// Read a hireables chunk's payload. `parse_character` returns the
/// character and how many bytes it consumed, or `None` when the export is
/// broken. The count's low word is read as written.
pub fn parse_hireables<T>(
    bytes: &[u8],
    mut parse_character: impl FnMut(&[u8]) -> Option<(T, usize)>,
) -> Result<Vec<Hireable<T>>, ArenaSaveError> {
    let mut r = Reader { bytes, at: 0 };
    let count = r.word()? as i32;
    let mut entries = Vec::new();
    for _ in 0..count.max(0) {
        let unused = r.word()?;
        let rest = bytes.get(r.at..).ok_or(ArenaSaveError::Short)?;
        let (character, used) = parse_character(rest).ok_or(ArenaSaveError::Short)?;
        if used == 0 || used > rest.len() {
            return Err(ArenaSaveError::Short);
        }
        r.at += used;
        entries.push(Hireable { unused, character });
    }
    Ok(entries)
}

/// Write a hireables chunk's payload. `write_character` appends one
/// candidate's encoded character.
pub fn write_hireables<T>(
    entries: &[Hireable<T>],
    mut write_character: impl FnMut(&T, &mut Vec<u8>),
) -> Vec<u8> {
    let mut out = Vec::new();
    put(&mut out, entries.len() as u32);
    for entry in entries {
        put(&mut out, entry.unused);
        write_character(&entry.character, &mut out);
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

/// The arsenal's chunk id.
pub const ARSENAL_CHUNK: u32 = 0xA010_0000;
/// The hub stash's chunk id.
pub const HUB_STASH_CHUNK: u32 = 0xA020_0000;
/// Boxes on a shelf and shelves in the arsenal.
pub const BOXES: usize = 5;
/// Shelves in the arsenal.
pub const SHELVES: usize = 5;

/// Read one item list: a signed count and that many twelve-byte entries.
/// A negative count reads no entries.
pub fn parse_item_list(bytes: &[u8]) -> Result<Vec<ItemEntry>, ArenaSaveError> {
    let mut r = Reader { bytes, at: 0 };
    let count = r.word()? as i32;
    let mut items = Vec::new();
    for _ in 0..count.max(0) {
        items.push(ItemEntry::parse(r.take(ITEM_ENTRY_LEN)?));
    }
    Ok(items)
}

/// Write one item list: the count and the entries.
pub fn write_item_list(items: &[ItemEntry]) -> Vec<u8> {
    let mut out = Vec::new();
    put(&mut out, items.len() as u32);
    for item in items {
        item.write(&mut out);
    }
    out
}

/// The arsenal: five boxes on each of five shelves. The original's writer
/// walks the boxes first and the shelves inside each box, so the wire order
/// is `lists[box][shelf]`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArsenalChunk {
    pub lists: [[Vec<ItemEntry>; SHELVES]; BOXES],
}

impl Default for ArsenalChunk {
    fn default() -> Self {
        Self {
            lists: std::array::from_fn(|_| std::array::from_fn(|_| Vec::new())),
        }
    }
}

impl ArsenalChunk {
    /// Read the 25 lists in the writer's order (box first, then shelf).
    pub fn parse(bytes: &[u8]) -> Result<Self, ArenaSaveError> {
        let mut r = Reader { bytes, at: 0 };
        let mut lists: [[Vec<ItemEntry>; SHELVES]; BOXES] =
            std::array::from_fn(|_| std::array::from_fn(|_| Vec::new()));
        for box_list in &mut lists {
            for list in box_list {
                let count = r.word()? as i32;
                for _ in 0..count.max(0) {
                    list.push(ItemEntry::parse(r.take(ITEM_ENTRY_LEN)?));
                }
            }
        }
        Ok(Self { lists })
    }

    /// Write the 25 lists in the writer's order.
    pub fn write(&self) -> Vec<u8> {
        let mut out = Vec::new();
        for box_list in &self.lists {
            for list in box_list {
                put(&mut out, list.len() as u32);
                for item in list {
                    item.write(&mut out);
                }
            }
        }
        out
    }
}

/// The arena's own chunks a save writes after the placement and journal.
/// The arsenal, company, day, hireables and tournament are always written;
/// the stash and shop stock only when non-empty, the employees only while
/// someone is employed, and the merchant only while one is employed.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ArenaTables {
    pub arsenal: ArsenalChunk,
    pub stash: Vec<ItemEntry>,
    pub shop_stock: Vec<u32>,
    pub company: CompanyChunk,
    pub day: DayChunk,
    pub employees: EmployeesChunk,
    pub merchant: Option<MerchantChunk>,
    /// The hireables payload, count and candidates included.
    pub hireables: Vec<u8>,
    pub tournament: TournamentChunk,
}

impl ArenaTables {
    /// Append the arena chunks in the writer's order. Each chunk is its id,
    /// its size and its payload (`save_package::write_chunk`).
    pub fn write(&self, out: &mut Vec<u8>) {
        write_chunk(out, ARSENAL_CHUNK, &self.arsenal.write());
        if !self.stash.is_empty() {
            write_chunk(out, HUB_STASH_CHUNK, &write_item_list(&self.stash));
        }
        if !self.shop_stock.is_empty() {
            write_chunk(out, SHOP_STOCK_CHUNK, &write_shop_stock(&self.shop_stock));
        }
        write_chunk(out, COMPANY_CHUNK, &write_company(&self.company));
        write_chunk(out, DAY_CHUNK, &write_day(&self.day));
        if !self.employees.employees.is_empty() {
            write_chunk(out, EMPLOYEES_CHUNK, &write_employees(&self.employees));
        }
        if let Some(merchant) = &self.merchant {
            write_chunk(out, MERCHANT_CHUNK, &write_merchant(merchant));
        }
        if self.hireables.is_empty() {
            write_chunk(out, HIREABLES_CHUNK, &[0; 4]);
        } else {
            write_chunk(out, HIREABLES_CHUNK, &self.hireables);
        }
        write_chunk(out, TOURNAMENT_CHUNK, &write_tournament(&self.tournament));
    }
}

/// Decode the arena chunks a save carries. Unknown ids are ignored; a
/// missing always-written chunk leaves its default.
pub fn read_tables(chunks: &[Chunk<'_>]) -> Result<ArenaTables, ArenaSaveError> {
    let mut tables = ArenaTables::default();
    for chunk in chunks {
        match chunk.id {
            ARSENAL_CHUNK => tables.arsenal = ArsenalChunk::parse(chunk.payload)?,
            HUB_STASH_CHUNK => tables.stash = parse_item_list(chunk.payload)?,
            SHOP_STOCK_CHUNK => tables.shop_stock = parse_shop_stock(chunk.payload)?,
            COMPANY_CHUNK => tables.company = parse_company(chunk.payload)?,
            DAY_CHUNK => tables.day = parse_day(chunk.payload)?,
            EMPLOYEES_CHUNK => tables.employees = parse_employees(chunk.payload)?,
            MERCHANT_CHUNK => tables.merchant = Some(parse_merchant(chunk.payload, true)?),
            HIREABLES_CHUNK => tables.hireables = chunk.payload.to_vec(),
            TOURNAMENT_CHUNK => tables.tournament = parse_tournament(chunk.payload)?,
            _ => {}
        }
    }
    Ok(tables)
}

/// An item-list entry's top bit: such an entry is a marker, not an item.
pub const ITEM_MARKER: u32 = 0x8000_0000;

/// The high-bit pair the arena's dedupe pass treats as special.
pub const ENTRY_SPECIAL: u32 = 0xC000_0000;

/// Keeps only plain entries of an eight-byte item list: the arena's load
/// compaction drops every entry whose first word has the top bit set and
/// closes the gaps in place.
pub fn retain_plain_items(items: &mut Vec<[u8; 8]>) {
    items.retain(|entry| {
        u32::from_le_bytes([entry[0], entry[1], entry[2], entry[3]]) & ITEM_MARKER == 0
    });
}

/// Keeps only plain entries of a twelve-byte record list: the arena's load
/// compaction drops every entry whose first word has the top bit set.
pub fn retain_plain_records(records: &mut Vec<[u8; 12]>) {
    records.retain(|entry| {
        u32::from_le_bytes([entry[0], entry[1], entry[2], entry[3]]) & ITEM_MARKER == 0
    });
}

/// The arena's duplicate pass over a twelve-byte record list. A plain entry
/// (both top bits clear) is only stepped over. A special entry (either top
/// bit set) is compared with the **later** entries: each later entry with
/// the same first word is removed, scanning from the end of the list back
/// to the special entry, and the list closes the gap after each removal. The special entry itself stays.
pub fn remove_duplicate_item_ids(records: &mut Vec<[u8; 12]>) {
    let word = |entry: &[u8; 12]| u32::from_le_bytes([entry[0], entry[1], entry[2], entry[3]]);
    let mut index = 0;
    while index < records.len() {
        if word(&records[index]) & ENTRY_SPECIAL == 0 {
            index += 1;
            continue;
        }
        let mut later = records.len();
        while later > index + 1 {
            later -= 1;
            if word(&records[later]) == word(&records[index]) {
                records.remove(later);
            }
        }
        index += 1;
    }
}

/// A list of eight-byte item entries and a list of twelve-byte records as
/// the arena's record repair sees them. The save unit compacts one
/// eight-byte list, then every employee's eight-byte list, then the
/// twelve-byte list, in that order.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ArenaLists {
    /// The eight-byte lists in the order the repair walks them: the stash
    /// list first, then one per employee.
    pub lists8: Vec<Vec<[u8; 8]>>,
    /// The twelve-byte record list.
    pub records12: Vec<[u8; 12]>,
}

impl ArenaLists {
    /// The load's record repair: every eight-byte list keeps only plain
    /// entries, then the twelve-byte list does. The character-record fix
    /// that follows in the original needs the character database and is
    /// the caller's.
    pub fn compact(&mut self) {
        for list in &mut self.lists8 {
            retain_plain_items(list);
        }
        retain_plain_records(&mut self.records12);
    }
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
    fn employees_round_trip_with_both_entry_kinds() {
        let chunk = EmployeesChunk {
            employees: vec![
                EmployeeRecord {
                    class: 0x10,
                    custom: false,
                    character: 0x1234,
                    items: vec![
                        ItemEntry {
                            id: 7,
                            flags: 0x200,
                            extra: 9,
                        },
                        ItemEntry {
                            id: 8,
                            flags: 0,
                            extra: 0,
                        },
                    ],
                    words: [1, 2, 3, 4],
                },
                EmployeeRecord {
                    class: 2,
                    custom: true,
                    character: 0,
                    items: Vec::new(),
                    words: [5, 6, 7, 8],
                },
            ],
        };
        let payload = write_employees(&chunk);
        // count; the first entry has a character id, the second not.
        assert_eq!(payload.len(), 4 + 64 + 36);
        let back = parse_employees(&payload).unwrap();
        assert_eq!(back, chunk);
        // The two leading words are skipped whatever they hold.
        let mut odd = payload.clone();
        odd[4] = 0xff;
        odd[8] = 0xee;
        assert_eq!(parse_employees(&odd).unwrap(), chunk);
        assert_eq!(
            parse_employees(&payload[..payload.len() - 1]),
            Err(ArenaSaveError::Short)
        );
        assert_eq!(parse_employees(&[]), Err(ArenaSaveError::Short));
    }

    #[test]
    fn an_empty_staff_writes_and_reads_an_empty_chunk() {
        let payload = write_employees(&EmployeesChunk::default());
        assert_eq!(payload, 0u32.to_le_bytes());
        assert_eq!(
            parse_employees(&payload).unwrap(),
            EmployeesChunk::default()
        );
        let mut negative = (-1i32).to_le_bytes().to_vec();
        negative.extend_from_slice(&[0; 8]);
        assert!(parse_employees(&negative).unwrap().employees.is_empty());
    }

    #[test]
    fn a_merchant_keeps_or_skips_its_stock() {
        let chunk = MerchantChunk {
            stock_type: 0x2c,
            specialisation: u32::MAX,
            unused: [0, 0],
            ids: vec![17, 18, 19],
        };
        let payload = write_merchant(&chunk);
        assert_eq!(payload.len(), 20 + 12);
        assert_eq!(parse_merchant(&payload, true).unwrap(), chunk);
        let skipped = parse_merchant(&payload, false).unwrap();
        assert_eq!(skipped.ids, Vec::new());
        assert_eq!(skipped.stock_type, 0x2c);
        assert_eq!(skipped.specialisation, u32::MAX);
        assert_eq!(
            parse_merchant(&payload[..15], true),
            Err(ArenaSaveError::Short)
        );
    }

    #[test]
    fn hireables_use_the_character_codec_for_their_boundaries() {
        // A synthetic export: a 16-bit byte count and that many bytes.
        #[allow(clippy::ptr_arg)]
        fn write_character(character: &Vec<u8>, out: &mut Vec<u8>) {
            out.extend_from_slice(&(character.len() as u16).to_le_bytes());
            out.extend_from_slice(character);
        }
        fn parse_character(bytes: &[u8]) -> Option<(Vec<u8>, usize)> {
            let len = u16::from_le_bytes([*bytes.first()?, *bytes.get(1)?]) as usize;
            let body = bytes.get(2..2 + len)?;
            Some((body.to_vec(), 2 + len))
        }
        let entries = vec![
            Hireable {
                unused: 0,
                character: vec![1, 2, 3],
            },
            Hireable {
                unused: 7,
                character: vec![],
            },
        ];
        let payload = write_hireables(&entries, write_character);
        assert_eq!(payload.len(), 4 + 4 + 5 + 4 + 2);
        assert_eq!(parse_hireables(&payload, parse_character).unwrap(), entries);
        // A negative count reads no entries.
        let mut negative = (-1i32).to_le_bytes().to_vec();
        negative.extend_from_slice(&[0; 8]);
        assert!(
            parse_hireables(&negative, parse_character)
                .unwrap()
                .is_empty()
        );
        // A character export that consumes nothing is refused.
        assert_eq!(
            parse_hireables(&payload, |_| Some(((), 0))),
            Err(ArenaSaveError::Short)
        );
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

    fn item(id: u32) -> [u8; 8] {
        let mut entry = [0u8; 8];
        entry[..4].copy_from_slice(&id.to_le_bytes());
        entry
    }

    fn record(id: u32) -> [u8; 12] {
        let mut entry = [0u8; 12];
        entry[..4].copy_from_slice(&id.to_le_bytes());
        entry
    }

    #[test]
    fn the_load_compaction_drops_top_bit_entries() {
        let mut items = vec![item(1), item(0x8000_0002), item(3)];
        retain_plain_items(&mut items);
        assert_eq!(items, vec![item(1), item(3)]);
        let mut records = vec![record(1), record(0x8000_0002), record(3)];
        retain_plain_records(&mut records);
        assert_eq!(records, vec![record(1), record(3)]);
    }

    #[test]
    fn the_duplicate_pass_removes_later_matches_of_a_special_entry() {
        // A plain entry is never deduplicated, even when it repeats.
        let mut records = vec![record(1), record(1), record(2)];
        remove_duplicate_item_ids(&mut records);
        assert_eq!(records, vec![record(1), record(1), record(2)]);
        // A special entry removes later entries with the same first word,
        // keeping the ones before it.
        let mut records = vec![
            record(1),
            record(0x8000_0005),
            record(1),
            record(0x8000_0005),
            record(5),
        ];
        remove_duplicate_item_ids(&mut records);
        assert_eq!(
            records,
            vec![record(1), record(0x8000_0005), record(1), record(5)]
        );
        // A special entry does not compare with positions before it.
        let mut records = vec![record(7), record(0xC000_0007)];
        remove_duplicate_item_ids(&mut records);
        assert_eq!(records, vec![record(7), record(0xC000_0007)]);
    }

    #[test]
    fn the_record_repair_compacts_every_eight_byte_list_then_the_records() {
        let mut lists = ArenaLists {
            lists8: vec![
                vec![item(1), item(0x8000_0002), item(3)],
                vec![item(0x8000_0004), item(5)],
            ],
            records12: vec![record(6), record(0xC000_0007), record(8)],
        };
        lists.compact();
        assert_eq!(lists.lists8[0], vec![item(1), item(3)]);
        assert_eq!(lists.lists8[1], vec![item(5)]);
        assert_eq!(lists.records12, vec![record(6), record(8)]);
    }

    #[test]
    fn the_shop_stock_is_a_count_and_item_ids() {
        let payload = write_shop_stock(&[7, 8, 9]);
        assert_eq!(&payload[..4], &3u32.to_le_bytes());
        assert_eq!(parse_shop_stock(&payload).unwrap(), vec![7, 8, 9]);
        // The count is signed; a negative one is empty.
        assert_eq!(parse_shop_stock(&(-1i32).to_le_bytes()).unwrap(), vec![]);
        assert_eq!(parse_shop_stock(&payload[..6]), Err(ArenaSaveError::Short));
    }

    #[test]
    fn the_arsenal_writes_the_boxes_before_the_shelves() {
        let mut arsenal = ArsenalChunk::default();
        arsenal.lists[1][3].push(ItemEntry {
            id: 40,
            flags: 0x400,
            extra: 0,
        });
        let payload = arsenal.write();
        // 25 counts and one entry.
        assert_eq!(payload.len(), 25 * 4 + 12);
        let back = ArsenalChunk::parse(&payload).unwrap();
        assert_eq!(back, arsenal);
        assert_eq!(back.lists[1][3][0].id, 40);
        assert_eq!(
            ArsenalChunk::parse(&payload[..100]),
            Err(ArenaSaveError::Short)
        );
    }

    #[test]
    fn the_arena_tables_write_in_the_writers_order_and_read_back() {
        let mut tables = ArenaTables::default();
        tables.stash.push(ItemEntry {
            id: 1,
            flags: 0,
            extra: 0,
        });
        tables.shop_stock.push(2);
        tables.hireables = 1u32.to_le_bytes().to_vec();
        let mut out = Vec::new();
        tables.write(&mut out);
        let chunks = crate::save_package::parse_chunks(&out).unwrap();
        let ids: Vec<u32> = chunks.iter().map(|chunk| chunk.id).collect();
        // The arsenal always; the stash and shop only when non-empty; the
        // company and day always; no employees or merchant; hireables and
        // tournament always.
        assert_eq!(
            ids,
            vec![
                ARSENAL_CHUNK,
                HUB_STASH_CHUNK,
                SHOP_STOCK_CHUNK,
                COMPANY_CHUNK,
                DAY_CHUNK,
                HIREABLES_CHUNK,
                TOURNAMENT_CHUNK
            ]
        );
        let back = read_tables(&chunks).unwrap();
        assert_eq!(back.arsenal, tables.arsenal);
        assert_eq!(back.stash, tables.stash);
        assert_eq!(back.shop_stock, vec![2]);
        assert_eq!(back.hireables, tables.hireables);
        assert_eq!(back.company, tables.company);
        assert_eq!(back.merchant, None);
    }

    #[test]
    fn a_new_arena_table_set_writes_empty_arsenal_and_hireable_lists() {
        let tables = ArenaTables::default();
        let mut out = Vec::new();
        tables.write(&mut out);
        let chunks = crate::save_package::parse_chunks(&out).unwrap();
        assert_eq!(
            chunks.iter().map(|chunk| chunk.id).collect::<Vec<_>>(),
            vec![
                ARSENAL_CHUNK,
                COMPANY_CHUNK,
                DAY_CHUNK,
                HIREABLES_CHUNK,
                TOURNAMENT_CHUNK
            ]
        );
        assert_eq!(chunks[0].payload.len(), 25 * 4);
        assert_eq!(chunks[3].payload, vec![0; 4]);
    }
}
