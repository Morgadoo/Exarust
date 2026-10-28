//! The loader's upgrade chain for databases of an older layout: each older
//! magic runs its converter and those of every newer layout, each
//! rewriting one section in every record. Layout and evidence:
//! `docs/formats/OTHER.md` (Object database, Older layouts).
//!
//! `0xDBAC0D08` (most of the shipped arena actors' databases) converts the
//! operative section; `0xDBAC0D07` the weapon section, then the operative
//! one; `0xDBAC0D06` the body first, then the weapon and the operative
//! section (the original runs its table's converters from the matched
//! entry down to the newest).

use super::packed::FractionWord;
use super::record::{section_range, set_u16, u16_at, u32_at};
use super::{BODY, OPERATIVE, RANDOM_TABLE, Store, WEAPON};
use crate::Result;

/// Brings a store read with `magic` up to the current layout. `paint_seeds`
/// feeds the paint seed each converted body takes; the original draws it
/// from its process RNG.
pub(super) fn upgrade(
    store: &mut Store,
    magic: u32,
    paint_seeds: &mut dyn FnMut() -> u64,
) -> Result<()> {
    match magic {
        super::MAGIC => {}
        0xDBAC_0D08 => store.rewrite_sections(OPERATIVE, 0x3C, convert_operative),
        0xDBAC_0D07 => {
            store.rewrite_sections(WEAPON, 0x60, convert_weapon);
            store.rewrite_sections(OPERATIVE, 0x3C, convert_operative);
        }
        0xDBAC_0D06 => {
            convert_bodies(store, paint_seeds);
            store.rewrite_sections(WEAPON, 0x60, convert_weapon);
            store.rewrite_sections(OPERATIVE, 0x3C, convert_operative);
        }
        found => {
            return Err(crate::Error::BadMagic {
                found,
                expected: super::MAGIC,
            });
        }
    }
    Ok(())
}

/// The old operative classes, by old class byte (the original's table; it
/// has twelve entries, and a higher byte reads unrelated data, 0 here).
const OLD_CLASSES: [u8; 12] = [0, 1, 4, 3, 5, 6, 7, 8, 2, 0, 0, 0];
/// The old operative kinds, by old kind byte (sixteen entries; the
/// original's lookup of a higher byte runs on into the class table, which
/// this keeps).
const OLD_KINDS: [u8; 16] = [0, 8, 11, 7, 1, 12, 2, 13, 6, 5, 14, 9, 10, 3, 4, 0];

fn old_kind(kind: u8) -> u8 {
    let k = kind as usize;
    if k < OLD_KINDS.len() {
        OLD_KINDS[k]
    } else {
        OLD_CLASSES.get(k - OLD_KINDS.len()).copied().unwrap_or(0)
    }
}

fn byte(old: &[u8], at: usize) -> u8 {
    old.get(at).copied().unwrap_or(0)
}

/// Copies `len` bytes of the old section body (`old` holds the 8-byte
/// header first) at `from` to `to` in the new body.
fn copy(old: &[u8], from: usize, new: &mut [u8], to: usize, len: usize) {
    for i in 0..len {
        if let Some(b) = new.get_mut(to + i) {
            *b = byte(old, 8 + from + i);
        }
    }
}

/// The old operative section into the 60-byte one: the class and (unless
/// the new class is a relay, 7) the kind through the original's tables, the
/// three words after them and the 24 bytes from `8` kept, the rest zero.
fn convert_operative(old: &[u8], new: &mut [u8]) {
    let class = OLD_CLASSES.get(byte(old, 8) as usize).copied().unwrap_or(0);
    new[0] = class;
    new[1] = if class == 7 {
        byte(old, 9)
    } else {
        old_kind(byte(old, 9))
    };
    copy(old, 2, new, 2, 0x1E);
}

/// The old weapon section into the 96-byte one, as the original's
/// converter maps it: the flags from the old kind bits (`0x10` word bits
/// 16–23) and class byte `0x1F`, with `0x400` and `0x100` giving
/// `0x1000000` and `0x2000000`; the seeds from `0` and `4`; the stat bytes
/// from `0x14`–`0x1F`, with byte `0x34` set to 1 and `0x35`/`0x36` derived
/// from byte `0x1C`; and the words from `0x20`.
fn convert_weapon(old: &[u8], new: &mut [u8]) {
    let word = |at: usize| {
        u32::from_le_bytes([
            byte(old, 8 + at),
            byte(old, 9 + at),
            byte(old, 10 + at),
            byte(old, 11 + at),
        ])
    };
    let kind = word(0x10) & 0xFF_0000;
    let class = byte(old, 8 + 0x1F);
    let in_pair = class.wrapping_sub(5) <= 1;
    let mut flags: u32 = match kind {
        0 if class == 0 => 0x11,
        0 if class == 4 => 0x14,
        0 if in_pair => 0x13,
        0 => 0x12,
        0x1_0000 if class == 0 => 0x421,
        0x1_0000 if class >= 5 && in_pair => 0x23,
        0x2_0000 => 0x32,
        0x41_0000 => 0x521,
        _ => 0x22,
    };
    if word(0x10) & 0x400 != 0 {
        flags |= 0x100_0000;
    }
    if word(0x10) & 0x100 != 0 {
        flags |= 0x200_0000;
    }
    new[0..4].copy_from_slice(&flags.to_le_bytes());
    copy(old, 0, new, 4, 4);
    copy(old, 4, new, 0xC, 4);
    copy(old, 0x14, new, 0x30, 3);
    copy(old, 0x1B, new, 0x33, 1);
    copy(old, 0x17, new, 0x40, 1);
    copy(old, 0x1F, new, 0x41, 1);
    copy(old, 0x1E, new, 0x42, 1);
    new[0x34] = 1;
    let v = byte(old, 8 + 0x1C);
    let (low, high) = if v < 0x47 {
        (0x14, 200)
    } else if v < 0x5A {
        (
            v.wrapping_sub(10).wrapping_mul(2),
            v.wrapping_add(10).wrapping_mul(2),
        )
    } else {
        (v.wrapping_mul(2).wrapping_sub(100).wrapping_mul(2), 200)
    };
    new[0x35] = low;
    new[0x36] = high;
    copy(old, 0x18, new, 0x37, 3);
    copy(old, 0x20, new, 0x44, 4);
    copy(old, 0x24, new, 0x48, 8);
}

/// Converts every record's body to the current layout and then repairs the
/// random tables' condition words, as the oldest layout's converter does.
fn convert_bodies(store: &mut Store, paint_seeds: &mut dyn FnMut() -> u64) {
    store.rewrite_sections(BODY, 0x224, |old, new| convert_body(old, new, paint_seeds));
    for at in 0..store.len() {
        let Some(record) = store.view(at) else {
            continue;
        };
        let Some(range) = section_range(record.payload(), RANDOM_TABLE) else {
            continue;
        };
        let mut payload = record.payload().to_vec();
        // The row count at body 0xC, 16-byte rows from body 0x10, the
        // condition word at row 0xA.
        let count = u32_at(&payload[range.clone()], 0xC) as i32;
        for i in 0..count.max(0) as usize {
            let condition = range.start + 0x10 + i * 0x10 + 0xA;
            if condition + 2 > range.end {
                break;
            }
            let word = u16_at(&payload, condition);
            set_u16(&mut payload, condition, convert_table_condition(word));
        }
        store.store(at, &payload);
    }
}

/// The old body into the current 548-byte one. The old program sat on the
/// same table for the occupancy, slot, coverage and protection blocks, but
/// its appearance block was four 16-byte parts at body `0x1B8` rather than
/// eight 12-byte layers at `0x1B4`, and its item category sat at `0x1A8`
/// where the current layout keeps the armour type. The paint seed at
/// `0x1AC`, the condition bytes and the packed spreads are new. Every
/// offset is body-relative; `old` starts at the section header.
fn convert_body(old: &[u8], new: &mut [u8], paint_seeds: &mut dyn FnMut() -> u64) {
    let byte = |at: usize| old.get(8 + at).copied().unwrap_or(0);
    copy(old, 0x00, new, 0x00, 0x10);
    copy(old, 0x10, new, 0x10, 0xA0);
    copy(old, 0xB0, new, 0xB0, 0xA0);
    copy(old, 0x150, new, 0x150, 0x50);
    for at in 0x1A0..0x1A7 {
        set_byte(new, at, byte(at));
    }
    set_byte(new, 0x214, byte(0x1A7));
    copy(old, 0x1AC, new, 0x218, 8);
    set_byte(new, 0x1A8, byte(0x1A8));
    // The old condition word's low two bits become the current count's 1.
    set_byte(new, 0x1A9, 1);
    set_byte(new, 0x1AA, if byte(0x1A9) & 0x80 != 0 { 0xFF } else { 0 });
    set_byte(new, 0x1B2, byte(0x1F9));
    // Three fractions of 0.25 in byte 0x1B3: the encoder's 2 is a quarter.
    set_byte(new, 0x1B3, 0x02 | 0x02 << 3 | 0x02 << 6);
    set_byte(new, 0x1A7, byte(0x1AA));
    set_byte(new, 0x1AB, byte(0x1AB));
    copy(old, 0x1B4, new, 0x220, 4);
    set_byte(new, 0x1B0, 0xA0);
    set_byte(new, 0x1B1, 0x40);

    let (armour_type, changes) = old_body_kind(byte(0x1A8), byte(0x1A9));
    set_byte(new, 0x1A8, armour_type);
    for (at, value) in changes {
        if at != 0 {
            set_byte(new, at, value);
        }
    }

    // The paint seed: a 63-bit draw modulo 0xFFFFFFFF, as the original's
    // conversion takes it from the run-time library's generator.
    let seed = (paint_seeds() & 0x7FFF_FFFF_FFFF_FFFF) % 0xFFFF_FFFF;
    super::record::set_u32(new, 0x1AC, seed as u32);

    for i in 0..4 {
        let part = |at: usize| byte(0x1B8 + i * 0x10 + at);
        let layer = 0x1B4 + i * 0xC;
        if i == 0 {
            set_byte(new, layer, 1);
        }
        set_byte(new, layer + 3, 1);
        set_byte(new, layer + 1, i as u8 + 1);
        set_byte(new, layer + 8, part(2));
        match part(0) {
            0 => {
                match part(5) {
                    0 | 1 => {
                        set_byte(new, layer + 2, 3);
                        set_byte(new, layer + 0xB, 1);
                    }
                    2 => {
                        set_byte(new, layer + 2, 6);
                        set_byte(new, layer + 0xB, 2);
                    }
                    _ => {}
                }
                match part(1) {
                    0 => set_byte(new, layer + 4, 1),
                    1 => {
                        let material = if part(3) <= 0x80 { 2 } else { 3 };
                        set_byte(new, layer + 4, material);
                    }
                    2 => set_byte(new, layer + 4, 4),
                    3 => {
                        let set = if armour_type == 3 { 9 } else { 2 };
                        set_byte(new, layer + 2, set);
                        set_byte(new, layer + 4, 5);
                    }
                    4 => {
                        set_byte(new, layer + 2, 3);
                        set_byte(new, layer + 4, 6);
                    }
                    _ => {}
                }
            }
            1 => {
                let set = if matches!(byte(0x1A8), 4 | 5) { 4 } else { 10 };
                match part(1) {
                    0 => {
                        set_byte(new, layer + 2, set);
                        set_byte(new, layer + 4, 8);
                    }
                    1 => {
                        set_byte(new, layer + 2, set);
                        set_byte(new, layer + 4, 7);
                    }
                    2 => {
                        let set = if byte(0x1A8) == 3 { 4 } else { 2 };
                        set_byte(new, layer + 2, set);
                        set_byte(new, layer + 4, 9);
                    }
                    _ => {}
                }
            }
            2 => match part(1) {
                0 => {
                    if part(5) == 1 {
                        set_byte(new, layer + 2, 7);
                        if part(2) <= 0x10 {
                            set_byte(new, layer + 4, 0xD);
                            set_byte(new, layer + 8, 3);
                        } else {
                            set_byte(new, layer + 4, 0x14);
                            set_byte(new, layer + 8, 0x4B);
                        }
                    } else {
                        if part(2) == 7 || part(2) > 0x10 {
                            set_byte(new, layer + 4, 0xE);
                            set_byte(new, layer + 3, 2);
                        } else {
                            set_byte(new, layer + 4, 0xD);
                            set_byte(new, layer + 8, 3);
                            set_byte(new, layer + 3, 0);
                        }
                        if part(5) == 2 {
                            set_byte(new, layer + 2, 0xD);
                            set_byte(new, layer + 3, 2);
                        } else {
                            set_byte(new, layer + 2, 0xC);
                        }
                        match part(2) {
                            0x9B => set_byte(new, layer + 8, 0x94),
                            0x24 => set_byte(new, layer + 8, 0x24),
                            0x6C => set_byte(new, layer + 8, 0x5C),
                            0x07 => set_byte(new, layer + 8, 6),
                            0xCC => set_byte(new, layer + 8, 0xCC),
                            0xE4 => set_byte(new, layer + 8, 0xE5),
                            0xFD => set_byte(new, layer + 8, 0xFD),
                            _ => {
                                if part(5) == 2 && part(2) > 0x10 {
                                    set_byte(new, layer + 4, 0x14);
                                    set_byte(new, layer + 8, 0x53);
                                }
                            }
                        }
                    }
                }
                1 | 2 => {
                    set_byte(new, layer + 2, 0xB);
                    set_byte(new, layer + 4, 0xB);
                }
                _ => {}
            },
            _ => {}
        }
    }
}

/// The old item category and finish into the current armour type and the
/// one or two appearance bytes the finish changes. A finish of 0 or with
/// bit 7 set keeps the defaults (quality 0xA0, damage 0x40, a variant that
/// says it is unrolled).
fn old_body_kind(kind: u8, finish: u8) -> (u8, [(usize, u8); 2]) {
    // Category 0 and anything past 8 keep the copied byte.
    let armour_type = match kind {
        1 | 4 => 1,
        2 | 5 => 2,
        3 => 3,
        6 => 4,
        7 => 5,
        8 => 6,
        other => other,
    };
    if finish == 0 || finish & 0x80 != 0 {
        return (armour_type, [(0, 0); 2]);
    }
    let pair = match (kind, finish) {
        (1 | 2, 1) => [(0x1B1, 0xC4), (0, 0)],
        (1 | 2, 2) => [(0x1AA, 1), (0, 0)],
        (1 | 2, 3 | 4) => [(0x1AA, 3), (0, 0)],
        (3, 1) => [(0x1B1, 0xC4), (0, 0)],
        (3, 2) => [(0x1B0, 0x40), (0, 0)],
        (3, 3) => [(0x1AA, 1), (0, 0)],
        (3, 4) => [(0x1AA, 2), (0, 0)],
        (3, 5) => [(0x1B1, 0x40), (0x1B0, 0xBE)],
        (3, 6) => [(0x1B1, 0x10), (0x1B0, 0xF0)],
        (4, 1) => [(0x1B1, 0xC4), (0, 0)],
        (4, 2) => [(0x1B0, 0xBE), (0, 0)],
        (4, 3) => [(0x1AA, 4), (0, 0)],
        (4, 4) => [(0x1AA, 3), (0, 0)],
        (5, 1) => [(0x1B1, 0xC4), (0, 0)],
        (5, 2) => [(0x1B0, 0x40), (0, 0)],
        (5, 3) => [(0x1B0, 0xBE), (0, 0)],
        (5, 4) => [(0x1AA, 4), (0, 0)],
        (5, 5) => [(0x1AA, 3), (0, 0)],
        (5, 6) => [(0x1B1, 0x40), (0x1B0, 0xBE)],
        (6, 1) => [(0x1B1, 0xC4), (0, 0)],
        (6, 2) => [(0x1B0, 0x40), (0, 0)],
        (6, 3) => [(0x1B0, 0xBE), (0, 0)],
        (6, 4) => [(0x1AA, 2), (0, 0)],
        (6, 5) => [(0x1AA, 5), (0, 0)],
        (6, 6) => [(0x1B1, 0x40), (0x1B0, 0xBE)],
        (6, 7) => [(0x1B1, 0x10), (0x1B0, 0xF0)],
        (7 | 8, 1) => [(0x1B1, 0xC4), (0, 0)],
        (7 | 8, 2) => [(0x1B1, 0xA0), (0, 0)],
        (7 | 8, 3) => [(0x1B0, 0x40), (0, 0)],
        (7 | 8, 4) => [(0x1AA, 1), (0, 0)],
        (7 | 8, 5) => [(0x1AA, 2), (0, 0)],
        (7 | 8, 6) => [(0x1B1, 0x40), (0x1B0, 0xBE)],
        (7 | 8, 7) => [(0x1B1, 0x10), (0x1B0, 0xF0)],
        _ => [(0, 0), (0, 0)],
    };
    (armour_type, pair)
}

/// A random table row's old condition word into the current packing, the
/// setters applied one after another to the same word as the original
/// does. A word whose bit 7 is clear is cleared and takes wealth 1, damage
/// 0.5 and spread 0.25, dirt 0.5 and spread 0.25 (`0x5284`). A word with
/// bit 7 keeps its other bits; wealth 1 is set first, and the damage is
/// then read from the updated word (the sign-magnitude value of bits 0–2,
/// bit 3 the sign, plus 2, in quarters: always 1.5 once wealth holds 4),
/// and the damage spread from bits 4–6 of the word after that (bit 6 now
/// the damage's low bit, 0).
fn convert_table_condition(word: u16) -> u16 {
    let mut out = FractionWord(word);
    if word & 0x80 == 0 {
        out.0 = 0;
        out.set_wealth(1.0);
        out.set_damage(0.5);
        out.set_damage_spread(0.25);
    } else {
        out.set_wealth(1.0);
        let magnitude = (out.0 & 7) as i16;
        let signed = if out.0 & 8 != 0 {
            magnitude.wrapping_neg()
        } else {
            magnitude
        };
        out.set_damage(f32::from(signed + 2) * 0.25);
        out.set_damage_spread(f32::from((out.0 & 0x70) >> 4) * 0.25);
    }
    out.set_dirt(0.5);
    out.set_dirt_spread(0.25);
    out.0
}

fn set_byte(b: &mut [u8], at: usize, value: u8) {
    if let Some(slot) = b.get_mut(at) {
        *slot = value;
    }
}

#[cfg(test)]
mod tests {
    use super::super::record::test_util::*;
    use super::*;
    use crate::Error;
    use crate::objdb::{NAME, TableEntry};

    fn old_section(tag: u32, body: &[u8]) -> Vec<u8> {
        section(tag, body)
    }

    #[test]
    fn old_operative_sections() {
        let mut body = vec![0u8; 0x20];
        body[0] = 2; // old class 2: new 4
        body[1] = 3; // old kind 3: new 7
        for (i, b) in body.iter_mut().enumerate().skip(2) {
            *b = i as u8;
        }
        let mut new = vec![0u8; 0x3C];
        convert_operative(&old_section(OPERATIVE, &body), &mut new);
        assert_eq!((new[0], new[1]), (4, 7));
        assert_eq!(&new[2..0x20], &body[2..0x20]);
        assert!(new[0x20..].iter().all(|&b| b == 0));
        // A relay (old class 6, new 7) keeps its kind byte.
        body[0] = 6;
        body[1] = 3;
        convert_operative(&old_section(OPERATIVE, &body), &mut new);
        assert_eq!((new[0], new[1]), (7, 3));
        // Kinds past the kind table run on into the class table.
        assert_eq!((old_kind(16), old_kind(18), old_kind(40)), (0, 4, 0));
    }

    #[test]
    fn old_weapon_sections() {
        let mut body = vec![0u8; 0x2C];
        body[0..4].copy_from_slice(&0x1111_1111u32.to_le_bytes());
        body[4..8].copy_from_slice(&0x2222_2222u32.to_le_bytes());
        // Kind 0x10000, class 5: 0x23; bit 0x400: 0x1000000.
        body[0x10..0x14].copy_from_slice(&0x0001_0400u32.to_le_bytes());
        body[0x1F] = 5;
        body[0x14..0x1C].copy_from_slice(&[1, 2, 3, 4, 5, 6, 7, 8]);
        body[0x1C] = 0x50; // between 0x47 and 0x5A
        body[0x1E] = 9;
        body[0x20..0x24].copy_from_slice(&0x3333_3333u32.to_le_bytes());
        body[0x24..0x2C].copy_from_slice(&[0xA; 8]);
        let mut new = vec![0u8; 0x60];
        convert_weapon(&old_section(WEAPON, &body), &mut new);
        fn word_at(n: &[u8], at: usize) -> u32 {
            u32::from_le_bytes(n[at..at + 4].try_into().unwrap())
        }
        assert_eq!(word_at(&new, 0), 0x23 | 0x100_0000);
        assert_eq!(
            (word_at(&new, 4), word_at(&new, 0xC)),
            (0x1111_1111, 0x2222_2222)
        );
        assert_eq!(&new[0x30..0x34], &[1, 2, 3, 8]);
        // 0x50: (0x50 − 10)·2 = 0x8C and (0x50 + 10)·2 = 0xB4.
        assert_eq!(&new[0x34..0x3A], &[1, 0x8C, 0xB4, 5, 6, 7]);
        assert_eq!(&new[0x40..0x43], &[4, 5, 9]);
        assert_eq!(word_at(&new, 0x44), 0x3333_3333);
        assert_eq!(&new[0x48..0x50], &[0xA; 8]);
        for (kind, class, flags) in [
            (0, 0, 0x11),
            (0, 4, 0x14),
            (0, 6, 0x13),
            (0, 7, 0x12),
            (0x1_0000, 0, 0x421),
            (0x1_0000, 3, 0x22),
            (0x2_0000, 0, 0x32),
            (0x41_0000, 0, 0x521),
            (0x3_0000, 0, 0x22),
        ] {
            body[0x10..0x14].copy_from_slice(&(kind as u32).to_le_bytes());
            body[0x1F] = class;
            convert_weapon(&old_section(WEAPON, &body), &mut new);
            assert_eq!(word_at(&new, 0), flags, "{kind:#x} {class}");
        }
        // Byte 0x1C below 0x47 and from 0x5A.
        body[0x1C] = 0x10;
        convert_weapon(&old_section(WEAPON, &body), &mut new);
        assert_eq!((new[0x35], new[0x36]), (0x14, 200));
        body[0x1C] = 0x60; // (0xC0 − 100)·2 = 0xB8
        convert_weapon(&old_section(WEAPON, &body), &mut new);
        assert_eq!((new[0x35], new[0x36]), (0xB8, 200));
    }

    #[test]
    fn the_loader_runs_the_chain() {
        let mut op = vec![0u8; 0x20];
        op[0] = 1;
        let mut weapon = vec![0u8; 0x2C];
        weapon[0x1F] = 4;
        let p = payload(&[
            (NAME, header("Old", 0, 0)),
            (WEAPON, weapon),
            (OPERATIVE, op),
        ]);
        let file = |magic: u32| {
            let mut b = magic.to_le_bytes().to_vec();
            b.extend(32u32.to_le_bytes());
            let e = TableEntry {
                id: 1,
                flags: NAME | WEAPON | OPERATIVE,
                offset: 0,
                size: p.len() as u32,
            };
            for w in [e.id, e.flags, e.offset, e.size] {
                b.extend(w.to_le_bytes());
            }
            b.extend([0; 16]);
            b.extend(&p);
            b
        };
        // 0D08: only the operative section is converted (and moves to the
        // end of the payload).
        let s = Store::parse(&file(0xDBAC_0D08)).unwrap();
        let r = s.find(1).unwrap();
        assert_eq!(r.section(WEAPON).unwrap().len(), 0x2C);
        assert_eq!(r.section(OPERATIVE).unwrap().len(), 0x3C);
        assert_eq!(r.sections().last().unwrap().tag, OPERATIVE);
        // 0D07: the weapon first, then the operative section.
        let s = Store::parse(&file(0xDBAC_0D07)).unwrap();
        let r = s.find(1).unwrap();
        let w = r.section(WEAPON).unwrap();
        assert_eq!((w.len(), w[0]), (0x60, 0x14));
        assert_eq!(r.section(OPERATIVE).unwrap().len(), 0x3C);
        // The converted database is written in the current layout.
        assert_eq!(&s.write()[0..4], &super::super::MAGIC.to_le_bytes());
        // 0D06 runs the same three converters; no body is present here.
        let s = Store::parse(&file(0xDBAC_0D06)).unwrap();
        assert_eq!(s.find(1).unwrap().section(WEAPON).unwrap().len(), 0x60);
        assert!(matches!(
            Store::parse(&file(0xDBAC_0D05)),
            Err(Error::BadMagic { .. })
        ));
    }

    /// A 0D06 database whose record has an old body and a random table:
    /// the body is converted first (with the seed from `parse_with_seeds`)
    /// and then each table row's condition word.
    #[test]
    fn the_old_body_and_random_tables_are_converted() {
        let old_body = old_body();
        // A 16-byte header with the row count at 0xC, then 16-byte rows.
        let mut table = vec![0u8; 0x10 + 2 * 0x10];
        table[0xC..0x10].copy_from_slice(&2u32.to_le_bytes());
        table[0x10 + 0xA..0x10 + 0xC].copy_from_slice(&0u16.to_le_bytes());
        table[0x20 + 0xA..0x20 + 0xC].copy_from_slice(&0x00F0u16.to_le_bytes());
        let p = payload(&[
            (NAME, header("Old", 0, 0)),
            (BODY, old_body),
            (RANDOM_TABLE, table),
        ]);
        let mut b = 0xDBAC_0D06u32.to_le_bytes().to_vec();
        b.extend(32u32.to_le_bytes());
        for w in [1, NAME | BODY | RANDOM_TABLE, 0, p.len() as u32] {
            b.extend(w.to_le_bytes());
        }
        b.extend([0; 16]);
        b.extend(&p);
        let mut seeds = || 0x1234_5678_9ABC_DEF0u64;
        let s = Store::parse_with_seeds(&b, &mut seeds).unwrap();
        let r = s.find(1).unwrap();
        assert_eq!(r.section(BODY).unwrap().len(), 0x224);
        assert_eq!(u32_at(r.section(BODY).unwrap(), 0x1AC), 0xACF1_3568);
        let t = r.section(RANDOM_TABLE).unwrap();
        assert_eq!(u16_at(t, 0x10 + 0xA), 0x5284);
        assert_eq!(u16_at(t, 0x20 + 0xA), 0x57B4);
        // A 0D07 database runs only the weapon and operative converters:
        // the body and the table rows stay as they were.
        b[0..4].copy_from_slice(&0xDBAC_0D07u32.to_le_bytes());
        let s = Store::parse(&b).unwrap();
        let r = s.find(1).unwrap();
        assert_eq!(r.section(BODY).unwrap(), &self::old_body()[..]);
        assert_eq!(u16_at(r.section(RANDOM_TABLE).unwrap(), 0x20 + 0xA), 0x00F0);
    }

    fn old_body() -> Vec<u8> {
        let mut b = vec![0u8; 0x224];
        b[0..8].copy_from_slice(b"WornMod\0");
        b[0x1AC..0x1B4].copy_from_slice(&[0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18]);
        b[0x1A0..0x1A7].copy_from_slice(&[1, 2, 3, 4, 5, 6, 7]);
        b[0x1A7] = 7;
        b[0x1A8] = 3;
        b[0x1A9] = 0x82;
        b[0x1AA] = 0xAA;
        b[0x1AB] = 0xBB;
        b[0x1F9] = 0xCC;
        b[0x1B4..0x1B8].copy_from_slice(&[1, 2, 3, 4]);
        // Part 0: kind 0, finish 0, colour 0x12, finish byte 3 = 0x80,
        // sub-kind 0: set 3, finish 1, material 1.
        b[0x1B8..0x1C8].copy_from_slice(&[0, 0, 0x12, 0x80, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        // Part 1: kind 1, sub-kind 2, kind byte 3: set 4, material 9.
        b[0x1C8..0x1D8].copy_from_slice(&[1, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        // Part 2: kind 2, sub-kind 0, colour 0x9B, finish 2.
        b[0x1D8..0x1E8].copy_from_slice(&[2, 0, 0x9B, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        b
    }

    #[test]
    fn old_body_fields_move_to_the_current_offsets() {
        let mut new = vec![0u8; 0x224];
        let mut seeds = || 0x1234_5678_9ABC_DEF0u64;
        convert_body(&old_section(BODY, &old_body()), &mut new, &mut seeds);
        assert_eq!(&new[0..8], b"WornMod\0");
        assert_eq!(&new[0x1A0..0x1A7], &[1, 2, 3, 4, 5, 6, 7]);
        assert_eq!(new[0x214], 7);
        // The power word takes the old bytes 0x1AC–0x1B3.
        assert_eq!(
            &new[0x218..0x220],
            &[0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18]
        );
        // The old category 3 is the armour type; the finish's bit 7 says
        // its appearance changes do not apply (the variant stays 0xFF).
        assert_eq!(new[0x1A8], 3);
        assert_eq!(new[0x1A9], 1);
        assert_eq!(new[0x1AA], 0xFF);
        assert_eq!(new[0x1A7], 0xAA);
        assert_eq!(new[0x1AB], 0xBB);
        assert_eq!(new[0x1B2], 0xCC);
        assert_eq!(&new[0x220..0x224], &[1, 2, 3, 4]);
        assert_eq!(new[0x1B0], 0xA0);
        assert_eq!(new[0x1B1], 0x40);
        assert_eq!(new[0x1B3], 0x92);
        assert_eq!(u32_at(&new, 0x1AC), 0xACF1_3568);
        // Part 0: its own mask 1, set 3, finish 1, material 1, colour 0x12.
        assert_eq!(&new[0x1B4..0x1C0], &[1, 1, 3, 1, 1, 0, 0, 0, 0x12, 0, 0, 1]);
        // Part 1 (old kind 1, sub-kind 2): set 4 for the old kind 3,
        // material 9.
        assert_eq!(&new[0x1C0..0x1CC], &[0, 2, 4, 1, 9, 0, 0, 0, 0, 0, 0, 0]);
        // Part 2 (old kind 2, sub-kind 0, colour 0x9B, finish 2): set 0xD,
        // byte 3 = 2, material 0xE, colour 0x94.
        assert_eq!(
            &new[0x1CC..0x1D8],
            &[0, 3, 0xD, 2, 0xE, 0, 0, 0, 0x94, 0, 0, 0]
        );
        // Part 3 is unused: the default set 3, finish 1, material 1.
        assert_eq!(&new[0x1D8..0x1E4], &[0, 4, 3, 1, 1, 0, 0, 0, 0, 0, 0, 1]);
        // The remaining four layers stay empty.
        assert!(new[0x1E4..0x214].iter().all(|&b| b == 0));
    }

    #[test]
    fn old_categories_and_finishes() {
        let mut b = vec![0u8; 0x224];
        for (kind, finish, armour_type, at, value) in [
            (1, 1, 1, 0x1B1, 0xC4),
            (2, 2, 2, 0x1AA, 1),
            (3, 3, 3, 0x1AA, 1),
            (4, 2, 1, 0x1B0, 0xBE),
            (5, 5, 2, 0x1AA, 3),
            (6, 6, 4, 0x1B1, 0x40),
            (7, 2, 5, 0x1B1, 0xA0),
            (8, 5, 6, 0x1AA, 2),
            // Past 8 the copied byte stays.
            (9, 1, 9, 0, 0),
        ] {
            b[0x1A8] = kind;
            b[0x1A9] = finish;
            b[0x1B0] = 0xA0;
            b[0x1B1] = 0x40;
            let mut new = vec![0u8; 0x224];
            let mut seeds = || 0u64;
            convert_body(&old_section(BODY, &b), &mut new, &mut seeds);
            assert_eq!(new[0x1A8], armour_type, "{kind}");
            if at != 0 {
                assert_eq!(new[at], value, "{kind} {finish}");
            }
        }
        // A finish of 0 keeps the defaults.
        b[0x1A8] = 3;
        b[0x1A9] = 0;
        let mut new = vec![0u8; 0x224];
        let mut seeds = || 0u64;
        convert_body(&old_section(BODY, &b), &mut new, &mut seeds);
        assert_eq!(new[0x1AA], 0);
        assert_eq!((new[0x1B0], new[0x1B1]), (0xA0, 0x40));
    }

    #[test]
    fn old_random_table_condition_words() {
        for (old, new) in [
            // Bit 7 clear: cleared, then the defaults.
            (0x0000, 0x5284),
            (0x001B, 0x5284),
            (0x0010, 0x5284),
            // Bit 7 set: wealth 4 is written before the damage is read, so
            // the damage is (4 + 2) quarters = 6; the spread then reads
            // bits 4–5 (bit 6 is the damage's, 0): 3 quarters → 3.
            (0x00F0, 0x57B4),
            // Bits 4–5 = 2: spread 0.5 → 2.
            (0x00A0, 0x55A4),
            // Bits 4–5 = 0: the spread is cleared.
            (0x0085, 0x5184),
            // Higher bits are rewritten by the setters' masks.
            (0x8E80, 0x5184),
        ] {
            assert_eq!(convert_table_condition(old), new, "{old:#06x}");
        }
    }
}
