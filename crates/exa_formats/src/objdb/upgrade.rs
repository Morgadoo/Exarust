//! The loader's upgrade chain for databases of an older layout: each older
//! magic runs its converter and those of every newer layout, each
//! rewriting one section in every record. Layout and evidence:
//! `docs/formats/OTHER.md` (Object database, Older layouts).
//!
//! `0xDBAC0D08` (most of the shipped arena actors' databases) converts the
//! operative section; `0xDBAC0D07` converts the weapon section first. The
//! oldest layout, `0xDBAC0D06`, also needs the body converter, which is not
//! reproduced.

use super::{OPERATIVE, Store, WEAPON};
use crate::{Error, Result};

/// Brings a store read with `magic` up to the current layout.
pub(super) fn upgrade(store: &mut Store, magic: u32) -> Result<()> {
    match magic {
        super::MAGIC => {}
        0xDBAC_0D08 => store.rewrite_sections(OPERATIVE, 0x3C, convert_operative),
        0xDBAC_0D07 => {
            store.rewrite_sections(WEAPON, 0x60, convert_weapon);
            store.rewrite_sections(OPERATIVE, 0x3C, convert_operative);
        }
        0xDBAC_0D06 => {
            return Err(Error::Unsupported(
                "object database of the 0xdbac0d06 layout (its body converter is not reproduced)"
                    .into(),
            ));
        }
        found => {
            return Err(Error::BadMagic {
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

#[cfg(test)]
mod tests {
    use super::super::record::test_util::*;
    use super::*;
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
        assert!(matches!(
            Store::parse(&file(0xDBAC_0D06)),
            Err(Error::Unsupported(_))
        ));
        assert!(matches!(
            Store::parse(&file(0xDBAC_0D05)),
            Err(Error::BadMagic { .. })
        ));
    }
}
