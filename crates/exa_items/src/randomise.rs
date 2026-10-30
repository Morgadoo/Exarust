//! Item variants and their condition, rolled when a world loads and when a
//! factory item is made to a grade.
//!
//! A grade sets a made weapon's condition before its roll: dirt `0x35`
//! with the widest dirt spread, then grade −1 pristine (quality `0xFF`, no
//! damage, no dirt, no spreads), 0 damage `0x20` within a half, 1 `0x40`
//! and 2 `0x80` within a whole.
//!
//! Containers whose children name a random table are filled from it. Each
//! table first hands out its rows' quotas to random empty slots that fit
//! them, then its count draws, then a weighted draw for every slot still
//! empty. A row weighs `w·1000`, `(1 − decay·0.005)·w·1000` after one
//! pick, and `(1 − decay·0.005)^n·w·1000` after `n` (in the engine's fast
//! logarithm and exponential), and nothing once it reaches its limit. A
//! row naming another table weighs that table's total less its empty row's,
//! times the row's weight in hundredths, and a pick inside it continues
//! into that table. A slot takes no row bigger than it allows. Evidence:
//! the item-variant notes (`context/re/item-gui-lane.md`).
//!
//! The world-load walk goes through the database in record order: an item
//! body whose armour type byte is not zero runs the apparel palette
//! generator on its palette byte's wealth plus signed variance and is
//! refreshed; else a weapon record is refreshed; else a variant list is
//! appended to the tables the fill will use.

use crate::instance::{MadeCondition, Spreads};
use crate::refresh::{made_condition, write_condition};
use exa_formats::containers::{self, RandomTable, RowCondition, TableEntry, entry};
use exa_formats::objdb::Item;

/// Section tags the grade, the row application and the walk read.
const NAME: u32 = 0x1000_0000;
const BODY: u32 = 0x1000;
const WEAPON: u32 = 0x4000;
/// The appearance block's offset inside the item body.
const APPEARANCE: usize = 0x1B0;
/// The item body's palette byte, which a row's wealth rewrites.
const PALETTE: usize = 0x1A9;

/// The dirt every graded weapon starts with.
pub const GRADE_DIRT: u8 = 0x35;
/// Draws per quota or count pick before a pick gives up.
pub const PICK_TRIES: u32 = 20;

/// The engine's generator step.
fn step(state: &mut u32) -> u32 {
    *state = state.wrapping_mul(0x0808_8405).wrapping_add(1);
    *state
}

/// One step, then the high half of the state times `n`.
fn below(state: &mut u32, n: u32) -> u32 {
    ((u64::from(step(state)) * u64::from(n)) >> 32) as u32
}

/// The engine's fast natural logarithm.
pub fn fast_ln(x: f32) -> f32 {
    let bits = x.to_bits();
    let exponent = (bits >> 23) as i32 as f32;
    let floor = if x > 0.0 || x.is_nan() {
        f32::from_bits(0xC2B3_DE55)
    } else {
        f32::NEG_INFINITY
    };
    let m = f32::from_bits(bits & 0x007F_FFFF | 0x3F80_0000);
    let m2 = m * m;
    let mut t = m * f32::from_bits(0x3CCC_A7FC) - f32::from_bits(0x3E79_8F62);
    t = t * m2 + (m - f32::from_bits(0x4011_9122));
    t = t * m2 + exponent * f32::from_bits(0x3F31_7218);
    (m * f32::from_bits(0x4059_7523) + floor) + t
}

/// The engine's fast exponential.
pub fn fast_exp(x: f32) -> f32 {
    let top = f32::from_bits(0x4EFF_0000);
    let v = x * f32::from_bits(0x4B38_AA3B) + f32::from_bits(0x4E7E_0000);
    let v = if v < top { v } else { top };
    let mut i = v.round_ties_even() as i32;
    if i <= 0 {
        i = 0;
    }
    let bits = i as u32;
    let m = f32::from_bits(bits & 0x007F_FFFF | 0x3F80_0000);
    let mut p = m * f32::from_bits(0x3C5D_BE77) + f32::from_bits(0xBB0B_4858);
    p = p * m + f32::from_bits(0x3E2A_9CBE);
    p = p * m + f32::from_bits(0x3E9F_C08B);
    p = p * m + f32::from_bits(0x3F02_8D05);
    f32::from_bits(bits & 0x7F80_0000) * p
}

/// Sets a made weapon's condition for `grade` and marks it for its roll.
/// Grades other than −1, 0, 1 and 2 keep the stored damage.
pub fn grade(c: &mut MadeCondition, grade: i32) {
    c.dirt = GRADE_DIRT;
    c.spreads.set_dirt(1.0);
    match grade {
        -1 => {
            c.quality = 0xFF;
            c.spreads.set_quality(0.0);
            c.damage = 0;
            c.spreads.set_damage(0.0);
            c.dirt = 0;
            c.spreads.set_dirt(0.0);
        }
        0 => {
            c.damage = 0x20;
            c.spreads.set_damage(0.5);
        }
        1 => {
            c.damage = 0x40;
            c.spreads.set_damage(1.0);
        }
        2 => {
            c.damage = 0x80;
            c.spreads.set_damage(1.0);
        }
        _ => {}
    }
}

/// A row's weight after `picks` picks.
pub fn row_weight(row: &TableEntry, picks: u32) -> i32 {
    if row.limit != 0 && picks >= u32::from(row.limit) {
        return 0;
    }
    let base = u32::from(row.weight) * 1000;
    match picks {
        0 => base as i32,
        1 => ((1.0 - f64::from(row.decay) * 0.005) * f64::from(base)).round_ties_even() as i32,
        n => {
            let keep = (1.0 - f64::from(row.decay) * 0.005) as f32;
            (fast_exp(fast_ln(keep) * n as f32) * base as f32).round_ties_even() as i32
        }
    }
}

/// One container child that draws from a table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Slot {
    /// The container child it fills (the caller's index).
    pub child: usize,
    /// The largest bulk the container takes.
    pub max_item: i32,
    /// The table a weighted pick landed in, when not the slot's own.
    pub table: Option<usize>,
    /// The 1-based row it was given; 0 while empty.
    pub row: u32,
}

/// One table being filled: its rows, their weights and picks, and its
/// slots.
#[derive(Debug, Clone, PartialEq)]
pub struct Draw {
    pub id: u32,
    pub table: RandomTable,
    pub weights: Vec<i32>,
    pub picks: Vec<u32>,
    /// For each nested row, the index of its table in the fill.
    pub nested: Vec<Option<usize>>,
    pub total: i32,
    pub slots: Vec<Slot>,
    /// Slots not yet given a row by a quota or a count draw.
    pub remaining: i32,
}

/// What the fill reads from the object database: a table by id, and an
/// item's bulk.
pub trait TableSource {
    fn table(&self, id: u32) -> Option<RandomTable>;
    fn bulk(&self, id: u32) -> Option<u8>;
}

impl Draw {
    /// A table as the builder prepares it: rows naming a table are marked
    /// nested, and each row takes its item's bulk.
    fn new(id: u32, mut table: RandomTable, source: &dyn TableSource) -> Self {
        for row in &mut table.entries {
            if source.table(row.id).is_some() {
                row.flags |= entry::NESTED;
            }
            if let Some(b) = source.bulk(row.id) {
                row.bulk = b;
            }
        }
        let n = table.entries.len();
        Self {
            id,
            table,
            weights: vec![0; n],
            picks: vec![0; n],
            nested: vec![None; n],
            total: 0,
            slots: Vec::new(),
            remaining: 0,
        }
    }

    fn set_weight(&mut self, r: usize, w: i32) {
        self.total = self.total - self.weights[r] + w;
        self.weights[r] = w;
    }
}

/// Every table a world fills, in the order the containers met them.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Fill {
    pub draws: Vec<Draw>,
}

impl Fill {
    /// The index of table `id`, building it the first time.
    pub fn draw_of(&mut self, id: u32, source: &dyn TableSource) -> Option<usize> {
        if let Some(i) = self.draws.iter().position(|d| d.id == id) {
            return Some(i);
        }
        let t = source.table(id)?;
        self.draws.push(Draw::new(id, t, source));
        Some(self.draws.len() - 1)
    }

    /// Adds a container child drawing from table `id`. False when `id` has
    /// no table.
    pub fn add_slot(
        &mut self,
        id: u32,
        child: usize,
        max_item: i32,
        source: &dyn TableSource,
    ) -> bool {
        let Some(i) = self.draw_of(id, source) else {
            return false;
        };
        self.draws[i].slots.push(Slot {
            child,
            max_item,
            table: None,
            row: 0,
        });
        true
    }

    /// Fills every slot, as the world load does once its containers are
    /// listed: remaining counts and nested tables (tables found here are
    /// filled too), quotas, count draws, the weights, then a weighted draw
    /// for every empty slot.
    pub fn run(&mut self, source: &dyn TableSource, state: &mut u32) {
        let mut d = 0;
        while d < self.draws.len() {
            self.draws[d].remaining = self.draws[d].slots.len() as i32;
            for r in 0..self.draws[d].table.entries.len() {
                let row = self.draws[d].table.entries[r];
                if row.flags & entry::NESTED != 0 {
                    self.draws[d].nested[r] = self.draw_of(row.id, source);
                }
            }
            d += 1;
        }
        for d in 0..self.draws.len() {
            self.quotas(d, state);
        }
        for d in 0..self.draws.len() {
            self.counts(d, state);
        }
        for draw in &mut self.draws {
            draw.total = 0;
            for r in 0..draw.table.entries.len() {
                let row = draw.table.entries[r];
                draw.weights[r] = if row.flags & (entry::COUNT_ONLY | entry::NESTED) != 0 {
                    0
                } else {
                    row_weight(&row, draw.picks[r])
                };
                draw.total += draw.weights[r];
            }
        }
        for d in 0..self.draws.len() {
            self.nested_weights(d, 0);
            for s in 0..self.draws[d].slots.len() {
                if self.draws[d].slots[s].row >= 1 {
                    continue;
                }
                let roll = below(state, self.draws[d].total as u32) as i32;
                let max = self.draws[d].slots[s].max_item;
                self.pick(d, d, s, roll, max, 0);
                if self.draws[d].slots[s].table != Some(d) {
                    self.nested_weights(d, 0);
                }
            }
        }
    }

    /// Each row hands out its quota to random empty slots its bulk fits,
    /// drawing a slot at most [`PICK_TRIES`] times per row.
    fn quotas(&mut self, d: usize, state: &mut u32) {
        let draw = &mut self.draws[d];
        let slots = draw.slots.len() as u32;
        for r in 0..draw.table.entries.len() {
            let row = draw.table.entries[r];
            let mut n = i32::from(row.quota).min(draw.remaining);
            let mut tries = 0;
            while n > 0 {
                tries += 1;
                if tries > PICK_TRIES {
                    break;
                }
                let s = below(state, slots) as usize;
                let slot = &mut draw.slots[s];
                if slot.row < 1 && i32::from(row.bulk) <= slot.max_item {
                    draw.picks[r] += 1;
                    slot.row = r as u32 + 1;
                    n -= 1;
                    draw.remaining -= 1;
                }
            }
        }
    }

    /// The table's count draws: between its minimum and maximum (at most
    /// the slots left), each a weighted pick among the count-only rows put
    /// in a random empty slot it fits.
    fn counts(&mut self, d: usize, state: &mut u32) {
        let draw = &mut self.draws[d];
        // The span multiplies sign-extended, so a table whose maximum is
        // below its minimum wraps as the original's 64-bit product does.
        let span = i64::from(draw.table.max) - i64::from(draw.table.min) + 1;
        let product = u64::from(step(state)).wrapping_mul(span as u64);
        let mut n = ((product >> 32) as u32 as i32).wrapping_add(i32::from(draw.table.min));
        if draw.remaining < n {
            n = draw.remaining;
        }
        if n < 1 {
            return;
        }
        draw.total = 0;
        for r in 0..draw.table.entries.len() {
            let row = draw.table.entries[r];
            if row.flags & entry::COUNT_ONLY != 0 {
                draw.weights[r] = row_weight(&row, draw.picks[r]);
                draw.total += draw.weights[r];
            }
        }
        let slots = draw.slots.len() as u32;
        while n > 0 {
            let roll = below(state, draw.total as u32) as i32;
            let mut acc = 0;
            let chosen = (0..draw.table.entries.len()).find(|&r| {
                acc += draw.weights[r];
                roll < acc
            });
            if let Some(r) = chosen {
                let bulk = i32::from(draw.table.entries[r].bulk);
                let mut tries = 0;
                let found = loop {
                    let s = below(state, slots) as usize;
                    tries += 1;
                    if tries > PICK_TRIES {
                        break None;
                    }
                    let slot = draw.slots[s];
                    if slot.row == 0 && slot.max_item >= bulk {
                        break Some(s);
                    }
                };
                if let Some(s) = found {
                    draw.slots[s].row = r as u32 + 1;
                    draw.remaining -= 1;
                    draw.picks[r] += 1;
                    let w = row_weight(&draw.table.entries[r], draw.picks[r]);
                    draw.set_weight(r, w);
                }
            }
            n -= 1;
        }
    }

    /// A nested row weighs its table's total less that table's first row,
    /// times its own weight in hundredths. `depth` stops a table that names
    /// itself.
    fn nested_weights(&mut self, d: usize, depth: usize) {
        if depth > self.draws.len() {
            return;
        }
        for r in 0..self.draws[d].table.entries.len() {
            let row = self.draws[d].table.entries[r];
            let Some(child) = self.draws[d].nested[r].filter(|_| row.flags & entry::NESTED != 0)
            else {
                continue;
            };
            self.nested_weights(child, depth + 1);
            let c = &self.draws[child];
            let inner = c.total - c.weights.first().copied().unwrap_or(0);
            let w = (f64::from(row.weight) * 0.01 * f64::from(inner)).round_ties_even() as i32;
            self.draws[d].set_weight(r, w);
        }
    }

    /// A weighted pick of `roll` in table `d` for slot `s` of table `home`.
    /// Count-only rows and rows bigger than `max` are passed over.
    fn pick(&mut self, d: usize, home: usize, s: usize, roll: i32, max: i32, depth: usize) {
        self.draws[home].slots[s].table = Some(d);
        if depth > self.draws.len() {
            return;
        }
        let mut acc = 0;
        for r in 0..self.draws[d].table.entries.len() {
            let row = self.draws[d].table.entries[r];
            if row.flags & entry::COUNT_ONLY != 0 || i32::from(row.bulk) > max {
                continue;
            }
            let before = acc;
            acc += self.draws[d].weights[r];
            if roll >= acc {
                continue;
            }
            let child = self.draws[d].nested[r].filter(|_| row.flags & entry::NESTED != 0);
            let w = match child {
                Some(child) => {
                    let first = self.draws[child].weights.first().copied().unwrap_or(0);
                    let inner = (100.0 / f64::from(row.weight) * f64::from(roll - before))
                        .round_ties_even() as i32;
                    self.pick(child, home, s, first + inner, max, depth + 1);
                    let c = &self.draws[child];
                    c.total - c.weights.first().copied().unwrap_or(0)
                }
                None => {
                    self.draws[d].picks[r] += 1;
                    self.draws[home].slots[s].row = r as u32 + 1;
                    row_weight(&row, self.draws[d].picks[r])
                }
            };
            self.draws[d].set_weight(r, w);
            return;
        }
    }

    /// What each slot of table `d` was given: its container child and the
    /// row, or `None` for an empty slot (which the original stores as id 0
    /// with flags `0x100`).
    pub fn given(&self, d: usize) -> Vec<(usize, Option<TableEntry>)> {
        self.draws[d]
            .slots
            .iter()
            .map(|slot| {
                let from = &self.draws[slot.table.unwrap_or(d)];
                let row = slot
                    .row
                    .checked_sub(1)
                    .and_then(|r| from.table.entries.get(r as usize))
                    .filter(|row| row.id != 0)
                    .copied();
                (slot.child, row)
            })
            .collect()
    }
}

/// Applies a table row's condition to a made weapon: damage and dirt as
/// rounded shares with their spreads. The copy then rolls.
pub fn row_condition(c: &mut MadeCondition, word: u32) {
    let rc = RowCondition::of(word);
    c.damage = (rc.damage * 255.0).round_ties_even() as u8;
    c.spreads.set_damage(rc.damage_spread);
    c.dirt = (rc.dirt * 255.0).round_ties_even() as u8;
    c.spreads.set_dirt(rc.dirt_spread);
}

/// The byte range of the first section tagged `tag` in a payload (its body,
/// after the 8-byte header).
pub fn section_range(payload: &[u8], tag: u32) -> Option<std::ops::Range<usize>> {
    let mut at = 0usize;
    while at.checked_add(8)? <= payload.len() {
        let id = u32::from_le_bytes(payload[at..at + 4].try_into().ok()?);
        let size = u32::from_le_bytes(payload[at + 4..at + 8].try_into().ok()?) as usize;
        if size < 8 {
            return None;
        }
        let end = at.checked_add(size)?;
        if end > payload.len() {
            return None;
        }
        if id == tag {
            return Some(at + 8..end);
        }
        at = end;
    }
    None
}

/// The palette byte a picked apparel copy takes from a row's `wealth`, as
/// the original writes it: the difference `wealth − (byte & 3)` (the low
/// two bits are the byte's own wealth), its magnitude times 31 rounded to
/// nearest even into bits 2 and up over the kept low bits, and the top bit
/// set when the difference is negative.
pub fn adjust_paint_seed(byte: u8, wealth: f32) -> u8 {
    let difference = wealth - f32::from(byte & 3);
    let magnitude = (difference * 31.0).abs().round_ties_even();
    // `CVTSS2SI` to 64 bits: NaN or out of range is the integer indefinite.
    let magnitude = if magnitude.is_nan() || magnitude >= 9.223_372e18 {
        i64::MIN
    } else {
        magnitude as i64
    };
    let mut out = (i64::from(byte & 3) | magnitude.wrapping_shl(2)) as u8;
    if difference < 0.0 {
        out |= 0x80;
    }
    out
}

/// Applies a table row's condition to a picked copy's payload, as the
/// original's row application does: an item body (checked first) gets the
/// rounded damage and dirt bytes and their spread codes in its appearance
/// block and its palette byte rewritten from the row's wealth; otherwise a
/// weapon section gets the damage, dirt and spreads in its condition
/// block. Returns whether a section was written. (The original runs the
/// weapon branch only for a copy in the world or local tier, which every
/// pick is.)
pub fn apply_row(payload: &mut [u8], word: u32) -> bool {
    let row = RowCondition::of(word);
    let byte = |share: f32| (share * 255.0).round_ties_even() as i64 as u8;
    if let Some(range) = section_range(payload, BODY) {
        let body = &mut payload[range];
        if body.len() <= APPEARANCE + 3 {
            return false;
        }
        body[PALETTE] = adjust_paint_seed(body[PALETTE], row.wealth);
        let appearance = &mut body[APPEARANCE..];
        appearance[1] = byte(row.damage);
        let mut spreads = Spreads(appearance[3]);
        spreads.set_damage(row.damage_spread);
        appearance[3] = spreads.0;
        appearance[2] = byte(row.dirt);
        let mut spreads = Spreads(appearance[3]);
        spreads.set_dirt(row.dirt_spread);
        appearance[3] = spreads.0;
        return true;
    }
    let Some(range) = section_range(payload, WEAPON) else {
        return false;
    };
    let body = &mut payload[range];
    if body.len() < 0x10 {
        return false;
    }
    body[0xD] = byte(row.damage);
    let mut spreads = Spreads(body[0xF]);
    spreads.set_damage(row.damage_spread);
    body[0xF] = spreads.0;
    body[0xE] = byte(row.dirt);
    let mut spreads = Spreads(body[0xF]);
    spreads.set_dirt(row.dirt_spread);
    body[0xF] = spreads.0;
    true
}

/// A picked item copied into a local record: its id, or `None` when the
/// store cannot copy it.
pub trait PickStore {
    /// Instantiates `row`'s item and applies its condition word; returns
    /// the copy's id.
    fn pick(&mut self, row: u32, condition: u32) -> Option<u32>;
}

/// Writes a container's children from the rows a fill gave them, as the
/// original's world load does: a picked child keeps its place and takes
/// the copy's id with zero flags; an empty slot, or a row of id 0, gets id
/// 0 and flags `0x100`. `given` pairs the child's index with its row.
pub fn fill_children(
    body: &mut [u8],
    given: &[(usize, Option<TableEntry>)],
    store: &mut dyn PickStore,
) -> bool {
    if body.len() < 0x18 {
        return false;
    }
    for &(child, row) in given {
        let at = 0x18 + child * containers::CHILD_LEN;
        if at + containers::CHILD_LEN > body.len() {
            return false;
        }
        match row.filter(|row| row.id != 0) {
            Some(row) => {
                let Some(id) = store.pick(row.id, row.condition) else {
                    continue;
                };
                body[at..at + 4].copy_from_slice(&id.to_le_bytes());
                body[at + 4..at + 8].copy_from_slice(&0u32.to_le_bytes());
            }
            None => {
                body[at..at + 4].copy_from_slice(&0u32.to_le_bytes());
                body[at + 4..at + 8].copy_from_slice(&0x100u32.to_le_bytes());
            }
        }
    }
    true
}

/// The original's grade routine over an object database: an object whose
/// name section's flag word has bit 0 set and whose model is a factory
/// weapon (`@…`) is copied into the local tier and its copy takes the
/// grade's condition. Returns the copy's id, the id itself when the object
/// is not graded, or 0 when it cannot be read or has no name section.
pub trait GradeSource {
    /// The record and its payload.
    fn read(&mut self, id: u32) -> Option<(Item, Vec<u8>)>;
    /// Instantiates the record, returning the copy's id.
    fn instantiate(&mut self, id: u32) -> u32;
    /// Stores the copy's payload.
    fn write(&mut self, id: u32, payload: &[u8]);
    /// Refreshes the copy's description and stats.
    fn refresh(&mut self, id: u32);
}

/// Copies a factory object and grades it, as the arena, the trade system and
/// the character generator ask. The check is the name section's flag bit 0,
/// then the model's `@`; the copy must have a weapon section, whose
/// condition block takes the grade.
pub fn grade_object(source: &mut dyn GradeSource, id: u32, level: i32) -> u32 {
    let Some((item, payload)) = source.read(id) else {
        return 0;
    };
    let Some(name) = section_range(&payload, NAME) else {
        return 0;
    };
    let graded = payload
        .get(name.start + 16..name.start + 20)
        .is_some_and(|w| u32::from_le_bytes(w.try_into().expect("4 bytes")) & 1 != 0);
    if !graded || !item.model.starts_with('@') {
        return id;
    }
    let copy = source.instantiate(id);
    let Some((_, mut payload)) = source.read(copy) else {
        return copy;
    };
    let Some(range) = section_range(&payload, WEAPON) else {
        return copy;
    };
    let Some(mut condition) = made_condition(&payload[range.clone()]) else {
        return copy;
    };
    grade(&mut condition, level);
    write_condition(&mut payload[range], &condition);
    source.write(copy, &payload);
    source.refresh(copy);
    copy
}

/// The palette byte an item body carries at `0x1A9`: its low two bits the
/// wealth, bits 2 through 6 the variance magnitude (0..=31) and bit 7 the
/// variance's sign.
pub const PALETTE_BYTE: usize = 0x1A9;

/// The armour type byte at `0x1A8`; the world-load walk reads a body's
/// palette only when it is not zero.
pub const ARMOUR_TYPE: usize = 0x1A8;

/// The wealth the palette byte names (`byte & 3`), as a float.
pub fn palette_wealth(palette: u16) -> f32 {
    (palette & 3) as f32
}

/// The variance the palette byte names: bits 2 through 6 over 31, positive
/// in single precision and, with bit 7 set, negated in double precision,
/// as the original's two readers do.
pub fn palette_variance(palette: u16) -> f32 {
    let magnitude = i32::from((palette & 0x7C) >> 2);
    if palette & 0x80 != 0 {
        (-f64::from(magnitude) / 31.0) as f32
    } else {
        magnitude as f32 * (1.0 / 31.0)
    }
}

/// What the world-load pass reads of the object database: each record's
/// section tags, an item body's byte, and the refresh it runs.
pub trait WorldSource {
    /// Every record id, in the database's order.
    fn ids(&self) -> Vec<u32>;
    /// The section tags of a record's payload, in order.
    fn sections(&self, id: u32) -> Vec<u32>;
    /// The byte at `offset` of a record's item body.
    fn body_byte(&self, id: u32, offset: usize) -> Option<u8>;
    /// A record's whole payload, for the variant-list walk.
    fn payload(&self, id: u32) -> Option<Vec<u8>>;
    /// Refreshes a record's description and stats.
    fn refresh(&mut self, id: u32);
    /// The palette byte (`0x1A9`) of a record's item body, as the two
    /// bytes it spans. Defaults to reading [`WorldSource::body_byte`].
    fn palette_byte(&self, id: u32) -> Option<u16> {
        let low = self.body_byte(id, PALETTE_BYTE)?;
        let high = self.body_byte(id, PALETTE_BYTE + 1).unwrap_or(0);
        Some(u16::from_le_bytes([low, high]))
    }
    /// Starts the apparel palette generator on a wealth value with mode 1
    /// (the original's shared palette, consumed by the texture painter).
    /// Exarust's painter keeps its own palette, so nothing is generated;
    /// the call is kept so the walk's reads and order are pinned.
    fn randomise_palette(&mut self, _wealth: f32) {}
    /// Appends an object's variant list to the tables the walk builds:
    /// each listed variant gets a table row at `row.offset` with `bulk`.
    fn register_variants(&mut self, _object: u32, _table: &containers::VariantTable) {}
}

/// The world-load pass's walk over the database, in record order:
///
/// * a record with an item body whose armour type (`0x1A8`) is not zero
///   reads its palette byte's wealth and variance, runs the palette
///   generator on their sum and refreshes the record;
/// * else a weapon record is refreshed;
/// * else a record with a variant list (`0x20000`) hands it to
///   [`WorldSource::register_variants`].
///
/// Returns how many records were refreshed. The original runs the
/// container fill right after this walk, which `Fill::run` does.
pub fn world_load(source: &mut dyn WorldSource) -> usize {
    let mut refreshed = 0;
    for id in source.ids() {
        let tags = source.sections(id);
        if tags.contains(&BODY) {
            if source.body_byte(id, ARMOUR_TYPE).is_some_and(|b| b != 0) {
                let palette = source.palette_byte(id).unwrap_or(0);
                source.randomise_palette(palette_wealth(palette) + palette_variance(palette));
                source.refresh(id);
                refreshed += 1;
            }
        } else if tags.contains(&WEAPON) {
            source.refresh(id);
            refreshed += 1;
        } else if tags.contains(&containers::VARIANT_TABLE)
            && let Some(payload) = source.payload(id)
            && let Some(table) = containers::variant_table(&payload)
        {
            source.register_variants(id, &table);
        }
    }
    refreshed
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn the_fast_logarithm_and_exponential() {
        // Pinned to the engine's polynomials, not the true functions.
        assert_eq!(fast_ln(1.0).to_bits(), 0x3780_0000);
        assert_eq!(fast_ln(0.5).to_bits(), 0xBF31_7100);
        assert_eq!(fast_exp(0.0).to_bits(), 0x3F80_0016);
        assert_eq!(fast_exp(1.0).to_bits(), 0x402D_F826);
        assert_eq!(fast_ln(0.0), f32::NEG_INFINITY);
        assert_eq!(fast_exp(-1000.0), 0.0);
    }

    fn row(id: u32, weight: u8, decay: u8) -> TableEntry {
        TableEntry {
            id,
            weight,
            decay,
            quota: 0,
            limit: 0,
            flags: 0,
            condition: 0,
            bulk: 0,
        }
    }

    #[test]
    fn a_row_loses_weight_as_it_is_picked() {
        let r = row(1, 10, 100);
        assert_eq!(row_weight(&r, 0), 10_000);
        assert_eq!(row_weight(&r, 1), 5_000);
        assert_eq!(row_weight(&r, 2), 2_500);
        assert_eq!(row_weight(&r, 3), 1_250);
        let limited = TableEntry { limit: 2, ..r };
        assert_eq!(row_weight(&limited, 2), 0);
        assert_eq!(row_weight(&limited, 1), 5_000);
    }

    #[test]
    fn grades_set_the_condition_before_the_roll() {
        let mut c = MadeCondition::default();
        grade(&mut c, -1);
        assert_eq!((c.quality, c.damage, c.dirt, c.spreads.0), (0xFF, 0, 0, 0));
        let mut c = MadeCondition::default();
        grade(&mut c, 0);
        assert_eq!((c.damage, c.dirt), (0x20, GRADE_DIRT));
        assert_eq!((c.spreads.damage(), c.spreads.dirt()), (0.5, 0.5));
        grade(&mut c, 2);
        assert_eq!((c.damage, c.spreads.damage()), (0x80, 1.0));
        let mut c = MadeCondition {
            damage: 9,
            ..Default::default()
        };
        grade(&mut c, 7);
        assert_eq!(c.damage, 9);
    }

    struct Db(HashMap<u32, RandomTable>, HashMap<u32, u8>);

    impl TableSource for Db {
        fn table(&self, id: u32) -> Option<RandomTable> {
            self.0.get(&id).cloned()
        }
        fn bulk(&self, id: u32) -> Option<u8> {
            self.1.get(&id).copied()
        }
    }

    #[test]
    fn quotas_counts_and_weights_fill_every_slot() {
        let mut tables = HashMap::new();
        let mut quota = row(20, 1, 0);
        quota.quota = 1;
        quota.limit = 1;
        let mut count = row(21, 5, 0);
        count.flags = entry::COUNT_ONLY;
        tables.insert(
            100,
            RandomTable {
                min: 1,
                max: 1,
                entries: vec![row(0, 0, 0), quota, count, row(22, 10, 100), row(23, 10, 0)],
            },
        );
        let bulk = HashMap::from([(23, 60)]);
        let db = Db(tables, bulk);
        let mut fill = Fill::default();
        for child in 0..5 {
            assert!(fill.add_slot(100, child, 50, &db));
        }
        assert!(!fill.add_slot(7, 9, 50, &db));
        let mut state = 0x1234;
        fill.run(&db, &mut state);
        let given = fill.given(0);
        let ids: Vec<u32> = given.iter().map(|(_, r)| r.map_or(0, |r| r.id)).collect();
        // The quota row once (then at its limit), the count row once.
        assert_eq!(ids.iter().filter(|&&id| id == 20).count(), 1);
        assert_eq!(ids.iter().filter(|&&id| id == 21).count(), 1);
        // Row 23 is too big for every slot: a draw that lands on its share
        // leaves the slot empty. The rest went to row 22.
        assert!(!ids.contains(&23));
        let others = ids.iter().filter(|&&id| id == 22).count();
        assert_eq!(ids.iter().filter(|&&id| id == 0).count() + others, 3);
        assert_eq!(fill.draws[0].picks[3] as usize, others);
        assert_eq!(fill.draws[0].remaining, 3);
        // Same seed, same fill.
        let mut again = Fill::default();
        for child in 0..5 {
            again.add_slot(100, child, 50, &db);
        }
        let mut state = 0x1234;
        again.run(&db, &mut state);
        assert_eq!(again.given(0), given);
    }

    #[test]
    fn an_inverted_count_span_wraps() {
        // Minimum 3, maximum 1: the span of −1 multiplies sign-extended, so
        // the draw's high word is all ones and the table makes 3 − 1 = 2
        // count draws.
        let mut count = row(21, 5, 0);
        count.flags = entry::COUNT_ONLY;
        let tables = HashMap::from([(
            100,
            RandomTable {
                min: 3,
                max: 1,
                entries: vec![row(0, 0, 0), count],
            },
        )]);
        let db = Db(tables, HashMap::new());
        let mut fill = Fill::default();
        for child in 0..5 {
            fill.add_slot(100, child, 50, &db);
        }
        let mut state = 0x1234;
        fill.run(&db, &mut state);
        assert_eq!(fill.draws[0].picks[1], 2);
    }

    #[test]
    fn a_nested_row_picks_inside_its_table() {
        let mut tables = HashMap::new();
        tables.insert(
            200,
            RandomTable {
                min: 0,
                max: 0,
                entries: vec![row(0, 0, 0), row(300, 100, 0)],
            },
        );
        tables.insert(
            300,
            RandomTable {
                min: 0,
                max: 0,
                entries: vec![row(0, 0, 0), row(31, 10, 0)],
            },
        );
        let db = Db(tables, HashMap::new());
        let mut fill = Fill::default();
        fill.add_slot(200, 4, 50, &db);
        let mut state = 99;
        fill.run(&db, &mut state);
        assert_eq!(fill.draws.len(), 2);
        assert_eq!(
            fill.draws[0].table.entries[1].flags & entry::NESTED,
            entry::NESTED
        );
        let given = fill.given(0);
        assert_eq!(given[0].0, 4);
        assert_eq!(given[0].1.map(|r| r.id), Some(31));
        assert_eq!(fill.draws[0].slots[0].table, Some(1));
        // The nested row weighs its table's total at 100 hundredths.
        assert_eq!(fill.draws[1].weights[1], 10_000);
        assert_eq!(fill.draws[0].weights[1], 10_000);
    }

    fn section_bytes(tag: u32, body: &[u8]) -> Vec<u8> {
        let mut out = tag.to_le_bytes().to_vec();
        out.extend_from_slice(&(body.len() as u32 + 8).to_le_bytes());
        out.extend_from_slice(body);
        out
    }

    #[test]
    fn a_weapon_row_writes_damage_dirt_and_spreads() {
        // A body section wins over a weapon section.
        let mut both = section_bytes(0x4000, &[0u8; 0x20]);
        both.extend(section_bytes(0x1000, &[0u8; 0x1C0]));
        assert!(apply_row(&mut both, 0x5284));
        assert_eq!(both[8 + 0xD], 0, "the weapon section is left alone");
        let mut payload = section_bytes(0x4000, &[0u8; 0x20]);
        assert!(apply_row(&mut payload, 0x5284));
        let body = &payload[8..];
        assert_eq!((body[0xD], body[0xE]), (128, 128));
        assert_eq!(body[0xF], 0x90);
        // No weapon and no body: nothing is written.
        assert!(!apply_row(&mut Vec::new(), 0x5284));
        let mut short = section_bytes(0x4000, &[0u8; 4]);
        assert!(!apply_row(&mut short, 0x5284));
    }

    #[test]
    fn an_apparel_row_writes_the_appearance_and_the_paint_seed() {
        let mut body = vec![0u8; 0x1C0];
        body[0x1A9] = 1;
        body[0x1B0] = 200;
        let mut payload = section_bytes(0x1000, &body);
        assert!(apply_row(&mut payload, 0x5284));
        let body = &payload[8..];
        assert_eq!(body[0x1B0], 200, "quality is the copy's");
        assert_eq!((body[0x1B1], body[0x1B2]), (128, 128));
        assert_eq!(body[0x1B3], 0x90);
        // Wealth 1 (word 0x5284, low nibble 4 × 0.25) over own wealth 1:
        // no difference, the low bits kept.
        assert_eq!(body[0x1A9], 1);
        // Wealth 0 over 1: |−1 × 31| = 31 in bits 2–6, the sign bit set.
        assert_eq!(adjust_paint_seed(1, 0.0), 0x80 | (31 << 2) | 1);
        // Wealth 0.25 over 0: 7.75 rounds to 8.
        assert_eq!(adjust_paint_seed(0x7C, 0.25), 8 << 2);
        // Wealth 2 over 3: 31 again, negative.
        assert_eq!(adjust_paint_seed(3, 2.0), 0x80 | (31 << 2) | 3);
        assert_eq!(adjust_paint_seed(0, f32::NAN), 0);
    }

    struct Picks(Vec<u32>);

    impl PickStore for Picks {
        fn pick(&mut self, row: u32, _condition: u32) -> Option<u32> {
            let id = 0x8000_0000 | row;
            self.0.push(id);
            Some(id)
        }
    }

    #[test]
    fn a_fill_writes_the_container_children() {
        let mut body = vec![0u8; 0x18 + 2 * containers::CHILD_LEN];
        let row = row(50, 1, 0);
        let given = [(0usize, Some(row)), (1, None)];
        let mut store = Picks(Vec::new());
        assert!(fill_children(&mut body, &given, &mut store));
        assert_eq!(store.0, [0x8000_0032]);
        let child = |i: usize| {
            let at = 0x18 + i * containers::CHILD_LEN;
            (
                u32::from_le_bytes(body[at..at + 4].try_into().unwrap()),
                u32::from_le_bytes(body[at + 4..at + 8].try_into().unwrap()),
            )
        };
        assert_eq!(child(0), (0x8000_0032, 0));
        assert_eq!(child(1), (0, 0x100));
        assert!(!fill_children(&mut body[..0x17], &given, &mut store));
    }

    struct Graded(std::collections::HashMap<u32, (Item, Vec<u8>)>, Vec<u32>);

    impl GradeSource for Graded {
        fn read(&mut self, id: u32) -> Option<(Item, Vec<u8>)> {
            self.0.get(&id).cloned()
        }
        fn instantiate(&mut self, _id: u32) -> u32 {
            2
        }
        fn write(&mut self, id: u32, payload: &[u8]) {
            if let Some((_, body)) = self.0.get_mut(&id) {
                *body = payload.to_vec();
            }
        }
        fn refresh(&mut self, id: u32) {
            self.1.push(id);
        }
    }

    fn factory_item() -> Item {
        let mut item = Item::parse(1, &[]).unwrap();
        item.name = "sword".into();
        item.model = "@ub05ssworda".into();
        item
    }

    /// A name section with the given flag word after the 16-byte name.
    fn name_section(flags: u32) -> Vec<u8> {
        let mut body = b"sword".to_vec();
        body.resize(16, 0);
        body.extend_from_slice(&flags.to_le_bytes());
        section_bytes(0x1000_0000, &body)
    }

    #[test]
    fn a_grade_copies_the_factory_object_and_sets_its_condition() {
        let item = factory_item();
        let weapon = section_bytes(0x4000, &[0u8; 0x20]);
        let mut source = name_section(1);
        source.extend_from_slice(&weapon);
        let mut store = Graded(
            std::collections::HashMap::from([(1, (item.clone(), source))]),
            Vec::new(),
        );
        // The copy keeps the weapon section.
        store.0.insert(2, (item.clone(), weapon));
        assert_eq!(grade_object(&mut store, 1, 1), 2);
        assert_eq!(store.1, [2]);
        let (_, copy) = store.0.get(&2).unwrap();
        let body = &copy[8..];
        assert_eq!(body[0xD], 0x40, "damage");
        assert_eq!(body[0xE], GRADE_DIRT, "dirt");
        assert_eq!(body[0xF] & 7, 0, "quality spread cleared");
        assert_eq!((body[0xF] >> 3) & 7, 5, "damage spread a whole");
        assert_eq!((body[0xF] >> 6) & 3, 3, "dirt spread a whole");
        // A non-factory item, or one without the name flag, is returned
        // unchanged; one without a name section, or unreadable, gives 0.
        let mut plain = store;
        plain.0.get_mut(&1).unwrap().0.model = "chair".into();
        assert_eq!(grade_object(&mut plain, 1, 1), 1);
        plain.0.get_mut(&1).unwrap().0.model = "@ub05ssworda".into();
        plain.0.get_mut(&1).unwrap().1 = name_section(0);
        assert_eq!(grade_object(&mut plain, 1, 1), 1);
        plain.0.get_mut(&1).unwrap().1 = Vec::new();
        assert_eq!(grade_object(&mut plain, 1, 1), 0);
        assert_eq!(grade_object(&mut plain, 9, 1), 0);
    }

    struct Record {
        id: u32,
        tags: Vec<u32>,
        body: Vec<u8>,
        payload: Vec<u8>,
    }

    struct World {
        records: Vec<Record>,
        refreshed: Vec<u32>,
        palettes: Vec<f32>,
        variants: Vec<(u32, containers::VariantTable)>,
    }

    impl WorldSource for World {
        fn ids(&self) -> Vec<u32> {
            self.records.iter().map(|r| r.id).collect()
        }
        fn sections(&self, id: u32) -> Vec<u32> {
            self.records
                .iter()
                .find(|r| r.id == id)
                .map(|r| r.tags.clone())
                .unwrap_or_default()
        }
        fn body_byte(&self, id: u32, offset: usize) -> Option<u8> {
            self.records
                .iter()
                .find(|r| r.id == id)
                .and_then(|r| r.body.get(offset))
                .copied()
        }
        fn payload(&self, id: u32) -> Option<Vec<u8>> {
            self.records
                .iter()
                .find(|r| r.id == id)
                .map(|r| r.payload.clone())
        }
        fn refresh(&mut self, id: u32) {
            self.refreshed.push(id);
        }
        fn randomise_palette(&mut self, wealth: f32) {
            self.palettes.push(wealth);
        }
        fn register_variants(&mut self, object: u32, table: &containers::VariantTable) {
            self.variants.push((object, table.clone()));
        }
    }

    fn record(id: u32, tags: Vec<u32>, body: Vec<u8>, payload: Vec<u8>) -> Record {
        Record {
            id,
            tags,
            body,
            payload,
        }
    }

    #[test]
    fn the_world_load_walks_apparel_weapons_and_variant_lists() {
        // A body of armour type 1 with palette byte 0x82: wealth 2, a
        // negative variance of 0. 0x82 & 0x7c = 0, so variance 0.
        let mut armour = vec![0u8; 0x1AC];
        armour[0x1A8] = 1;
        armour[0x1A9] = 0x82;
        let mut plain = vec![0u8; 0x1AC];
        plain[0x1A9] = 0xFF;
        // A variant list of one row.
        let mut variant_body = vec![0u8; 0x18];
        variant_body[0x0A..0x0C].copy_from_slice(&42u16.to_le_bytes());
        variant_body[0x14..0x18].copy_from_slice(&1u32.to_le_bytes());
        variant_body.extend_from_slice(&0x77u32.to_le_bytes());
        variant_body.extend_from_slice(&[0; 8]);
        let mut variant_payload = containers::VARIANT_TABLE.to_le_bytes().to_vec();
        variant_payload.extend_from_slice(&((variant_body.len() + 8) as u32).to_le_bytes());
        variant_payload.extend_from_slice(&variant_body);

        let mut world = World {
            records: vec![
                record(1, vec![0x1000], armour, Vec::new()),
                record(2, vec![0x1000], plain, Vec::new()),
                record(3, vec![0x4000], Vec::new(), Vec::new()),
                record(4, vec![0x200], Vec::new(), Vec::new()),
                record(5, vec![0x1000], Vec::new(), Vec::new()),
                record(
                    6,
                    vec![containers::VARIANT_TABLE],
                    Vec::new(),
                    variant_payload,
                ),
            ],
            refreshed: Vec::new(),
            palettes: Vec::new(),
            variants: Vec::new(),
        };
        assert_eq!(world_load(&mut world), 2);
        assert_eq!(world.refreshed, [1, 3]);
        assert_eq!(world.palettes, [2.0]);
        assert_eq!(world.variants.len(), 1);
        let (object, table) = &world.variants[0];
        assert_eq!(*object, 6);
        assert_eq!(table.bulk, 42);
        assert_eq!(table.rows.len(), 1);
        assert_eq!(table.rows[0].id, 0x77);
        assert_eq!(table.rows[0].offset, 0x20);
    }

    #[test]
    fn the_palette_byte_splits_into_wealth_and_a_signed_variance() {
        assert_eq!(palette_wealth(0b10), 2.0);
        assert_eq!(palette_wealth(0xFF), 3.0);
        assert_eq!(palette_variance(0), 0.0);
        // Bits 2..6: 31 over 31 is 1.
        assert_eq!(palette_variance(31 << 2), 1.0);
        // Bit 7 negates in double precision.
        assert_eq!(palette_variance(0x80 | (5 << 2)), -(5.0 / 31.0) as f32);
    }

    #[test]
    fn a_row_condition_sets_damage_and_dirt() {
        let mut c = MadeCondition::default();
        row_condition(&mut c, 0x5284);
        assert_eq!((c.damage, c.dirt), (128, 128));
        assert_eq!((c.spreads.damage(), c.spreads.dirt()), (0.25, 0.25));
        assert!(c.needs_roll());
    }
}
