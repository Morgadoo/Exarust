//! Copying one item's look and weapon stats onto another, as the controls'
//! material and weapon copy helpers do. Rules: the source and the target
//! must both have the section; the model section's 16-bit word at `0x26`
//! always comes along; the appearance (body section) is copied whole; a
//! weapon copies its condition/quality bytes at `0x38`–`0x3b`, the qword at
//! `0x3c` and the dword at `0x44`, then the caller rebuilds its mesh and
//! phrases.
//!
//! These are byte-level copies so the original's field widths and order are
//! pinned by tests; the semantic rebuild is the item factory's or the card's
//! job (`exa_items::instance`, `exa_items::card`).

use exa_formats::objdb::Appearance;

/// The model section's word the copy takes along (section offset `0x26`).
pub const MODEL_WORD: usize = 0x26;
/// The weapon bytes copied in order: `0x38`, `0x39`, `0x3a`, `0x3b`, then
/// the qword at `0x3c` and the dword at `0x44`.
pub const WEAPON_BYTES: usize = 0x38;
pub const WEAPON_QWORD: usize = 0x3c;
pub const WEAPON_DWORD: usize = 0x44;

/// The weapon section fields the copy moves, kept as the original reads
/// them: four bytes, a qword and a dword.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct WeaponFields {
    pub bytes: [u8; 4],
    pub qword: u64,
    pub dword: u32,
}

impl WeaponFields {
    /// Reads the fields from a weapon section's bytes; `None` when the
    /// section is shorter than `0x48`.
    pub fn parse(section: &[u8]) -> Option<Self> {
        let bytes = section.get(WEAPON_BYTES..WEAPON_BYTES + 4)?;
        let qword = section.get(WEAPON_QWORD..WEAPON_QWORD + 8)?;
        let dword = section.get(WEAPON_DWORD..WEAPON_DWORD + 4)?;
        Some(Self {
            bytes: bytes.try_into().ok()?,
            qword: u64::from_le_bytes(qword.try_into().ok()?),
            dword: u32::from_le_bytes(dword.try_into().ok()?),
        })
    }

    /// Writes the fields into a weapon section's bytes; `false` when the
    /// section is too short (nothing is written).
    pub fn write(&self, section: &mut [u8]) -> bool {
        if section.len() < WEAPON_DWORD + 4 {
            return false;
        }
        section[WEAPON_BYTES..WEAPON_BYTES + 4].copy_from_slice(&self.bytes);
        section[WEAPON_QWORD..WEAPON_QWORD + 8].copy_from_slice(&self.qword.to_le_bytes());
        section[WEAPON_DWORD..WEAPON_DWORD + 4].copy_from_slice(&self.dword.to_le_bytes());
        true
    }
}

/// Copies the appearance and model word of `source` onto `target`, as the
/// material copy does. `None` for either side copies nothing. Returns
/// whether the target changed and so needs its phrases rebuilt.
pub fn copy_appearance(
    target: Option<(&mut u16, &mut Appearance)>,
    source: Option<(u16, &Appearance)>,
) -> bool {
    let (Some((target_word, target_appearance)), Some((source_word, source_appearance))) =
        (target, source)
    else {
        return false;
    };
    *target_word = source_word;
    *target_appearance = *source_appearance;
    true
}

/// Copies a weapon's fields and model word onto `target`, as the weapon copy
/// does. Both sides must have the weapon section (and the target the model
/// section); returns whether the target changed, so its mesh and phrases
/// need rebuilding.
pub fn copy_weapon(
    target: Option<(&mut u16, &mut WeaponFields)>,
    source: Option<(u16, &WeaponFields)>,
) -> bool {
    let (Some((target_word, target_fields)), Some((source_word, source_fields))) = (target, source)
    else {
        return false;
    };
    *target_word = source_word;
    *target_fields = *source_fields;
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn appearance(quality: u8, dirt: u8) -> Appearance {
        Appearance {
            quality,
            damage: 3,
            dirt,
            layers: [Default::default(); 8],
        }
    }

    /// The appearance and the model word move together; a missing side does
    /// nothing.
    #[test]
    fn the_appearance_copy_takes_the_model_word() {
        let mut target_word = 7u16;
        let mut target = appearance(1, 2);
        let source = appearance(9, 8);
        assert!(copy_appearance(
            Some((&mut target_word, &mut target)),
            Some((0x1234, &source))
        ));
        assert_eq!(target_word, 0x1234);
        assert_eq!(target, source);
        assert!(!copy_appearance(None, Some((1, &source))));
        assert!(!copy_appearance(
            Some((&mut target_word, &mut target)),
            None
        ));
    }

    /// The weapon's bytes, qword and dword are read, copied and written at
    /// the original's offsets; a short section copies nothing.
    #[test]
    fn the_weapon_copy_moves_its_raw_fields() {
        let mut section = [0u8; 0x50];
        for (i, b) in section.iter_mut().enumerate().skip(WEAPON_BYTES).take(4) {
            *b = i as u8;
        }
        section[WEAPON_QWORD..WEAPON_QWORD + 8]
            .copy_from_slice(&0x1122_3344_5566_7788u64.to_le_bytes());
        section[WEAPON_DWORD..WEAPON_DWORD + 4].copy_from_slice(&0xAABB_CCDDu32.to_le_bytes());
        let fields = WeaponFields::parse(&section).unwrap();
        assert_eq!(fields.bytes, [0x38, 0x39, 0x3a, 0x3b]);
        assert_eq!(fields.qword, 0x1122_3344_5566_7788);
        assert_eq!(fields.dword, 0xAABB_CCDD);
        let mut target_word = 0u16;
        let mut target = WeaponFields::default();
        assert!(copy_weapon(
            Some((&mut target_word, &mut target)),
            Some((0x99, &fields))
        ));
        assert_eq!((target_word, target), (0x99, fields));
        let mut out = [0u8; 0x50];
        assert!(fields.write(&mut out));
        assert_eq!(&out[WEAPON_BYTES..WEAPON_BYTES + 4], &fields.bytes);
        assert_eq!(
            &out[WEAPON_QWORD..WEAPON_QWORD + 8],
            &fields.qword.to_le_bytes()
        );
        assert_eq!(
            &out[WEAPON_DWORD..WEAPON_DWORD + 4],
            &fields.dword.to_le_bytes()
        );
        assert!(!fields.write(&mut [0u8; 0x40]));
        assert_eq!(WeaponFields::parse(&[0u8; 0x40]), None);
    }
}
