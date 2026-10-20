//! Character creation's record movement.
//!
//! A new record takes the appearance: four words, a count and a parts list,
//! then a tail and its list. The source keeps its words and loses both
//! counts and lists. Evidence: the 228-byte `0x10012bd20`.

/// A character creation record's appearance.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CharacterAppearance {
    pub words: [u64; 4],
    pub count: i32,
    pub parts: Vec<u32>,
    pub tail: i32,
    pub tail_parts: Vec<u32>,
}

/// Moves the appearance out of `source` into a new record.
pub fn move_appearance(source: &mut CharacterAppearance) -> CharacterAppearance {
    let moved = CharacterAppearance {
        words: source.words,
        count: source.count,
        parts: std::mem::take(&mut source.parts),
        tail: source.tail,
        tail_parts: std::mem::take(&mut source.tail_parts),
    };
    source.count = 0;
    source.tail = 0;
    moved
}

/// The creation screen's appearance labels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CreationLabel {
    None,
    HumanFemale,
    HumanMale,
    Ham,
    Om,
    Ml,
}

impl CreationLabel {
    /// The literal the original stores, or none.
    pub fn code(self) -> Option<&'static str> {
        match self {
            Self::None => None,
            Self::HumanFemale => Some("HF"),
            Self::HumanMale => Some("HM"),
            Self::Ham => Some("HAM"),
            Self::Om => Some("OM"),
            Self::Ml => Some("ML"),
        }
    }
}

/// Picks the appearance label from the record's word.
pub fn creation_label(word: i32) -> CreationLabel {
    match word & 0xff00 {
        0 => {
            if word & 1 != 0 {
                CreationLabel::HumanFemale
            } else {
                CreationLabel::HumanMale
            }
        }
        0x200 => CreationLabel::Ham,
        0x300 => CreationLabel::Om,
        0x400 => CreationLabel::Ml,
        _ => CreationLabel::None,
    }
}

/// Clears the first used creation slot and the slots after it.
pub fn clear_creation_slots(slots: &mut [[u8; 8]; 6]) -> Option<usize> {
    let first = slots
        .iter()
        .position(|slot| u16::from_le_bytes([slot[0], slot[1]]) != 0)?;
    let slot = &mut slots[first];
    slot[0] = 0;
    slot[1] = 0;
    for byte in &mut slot[3..] {
        *byte = 0;
    }
    if first < 5 {
        for later in &mut slots[first + 1..5] {
            *later = [0; 8];
        }
    }
    Some(first)
}

/// What settling the creation appearance did.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CreationSettle {
    pub stored: Option<f32>,
    pub pending: bool,
    pub last: i32,
    pub callback: bool,
}

/// Settles an appearance value toward its target.
pub fn settle_appearance(source: f32, target: f32, last: i32, clock: i32) -> CreationSettle {
    const EPSILON: f32 = f32::from_bits(0x33ff_ffd7);
    let value = (0.5 - source) * 2.0;
    if (value - target).abs() < EPSILON {
        return CreationSettle {
            stored: None,
            pending: false,
            last,
            callback: false,
        };
    }
    if last + 2 > clock {
        return CreationSettle {
            stored: Some(value),
            pending: true,
            last,
            callback: false,
        };
    }
    CreationSettle {
        stored: Some(value),
        pending: false,
        last: clock,
        callback: true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_character_record_moves_its_appearance() {
        let mut source = CharacterAppearance {
            words: [1, 2, 3, 4],
            count: 5,
            parts: vec![7, 8],
            tail: 9,
            tail_parts: vec![10],
        };
        let moved = move_appearance(&mut source);
        assert_eq!(
            moved,
            CharacterAppearance {
                words: [1, 2, 3, 4],
                count: 5,
                parts: vec![7, 8],
                tail: 9,
                tail_parts: vec![10],
            }
        );
        assert_eq!(
            source,
            CharacterAppearance {
                words: [1, 2, 3, 4],
                count: 0,
                parts: Vec::new(),
                tail: 0,
                tail_parts: Vec::new(),
            }
        );
    }

    #[test]
    fn the_creation_appearance_labels_its_bands() {
        assert_eq!(creation_label(0x001), CreationLabel::HumanFemale);
        assert_eq!(creation_label(0x001).code(), Some("HF"));
        assert_eq!(creation_label(0x000), CreationLabel::HumanMale);
        assert_eq!(creation_label(0x000).code(), Some("HM"));
        assert_eq!(creation_label(0x201), CreationLabel::Ham);
        assert_eq!(creation_label(0x201).code(), Some("HAM"));
        assert_eq!(creation_label(0x300), CreationLabel::Om);
        assert_eq!(creation_label(0x4ff), CreationLabel::Ml);
        assert_eq!(creation_label(0x4ff).code(), Some("ML"));
        assert_eq!(creation_label(0x500), CreationLabel::None);
        assert_eq!(creation_label(0x500).code(), None);
    }

    #[test]
    fn the_creation_slots_clear_from_the_first_used() {
        let mut slots = [[0u8; 8]; 6];
        slots[1] = [1, 2, 3, 4, 5, 6, 7, 8];
        slots[5] = [7; 8];
        assert_eq!(clear_creation_slots(&mut slots), Some(1));
        assert_eq!(slots[0], [0; 8]);
        assert_eq!(slots[1], [0, 0, 3, 0, 0, 0, 0, 0]);
        assert_eq!(slots[2], [0; 8]);
        assert_eq!(slots[3], [0; 8]);
        assert_eq!(slots[4], [0; 8]);
        assert_eq!(slots[5], [7; 8]);

        let mut empty = [[0u8; 8]; 6];
        assert_eq!(clear_creation_slots(&mut empty), None);
        assert!(empty.iter().all(|slot| slot == &[0u8; 8]));

        let mut last = [[0u8; 8]; 6];
        last[5] = [7; 8];
        assert_eq!(clear_creation_slots(&mut last), Some(5));
        assert_eq!(last[5], [0, 0, 7, 0, 0, 0, 0, 0]);
    }

    #[test]
    fn the_creation_appearance_settles_toward_its_target() {
        let settled = settle_appearance(0.25, 0.5, 0, 0);
        assert_eq!(settled.stored, None);
        assert!(!settled.pending);
        assert!(!settled.callback);

        let pending = settle_appearance(0.25, 0.0, 100, 101);
        assert_eq!(pending.stored, Some(0.5));
        assert!(pending.pending);
        assert_eq!(pending.last, 100);
        assert!(!pending.callback);

        let fired = settle_appearance(0.25, 0.0, 100, 102);
        assert_eq!(fired.stored, Some(0.5));
        assert!(!fired.pending);
        assert_eq!(fired.last, 102);
        assert!(fired.callback);
    }
}
