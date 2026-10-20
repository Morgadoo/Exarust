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

/// The distance under which nothing is written.
const SETTLE_EPSILON: f32 = f32::from_bits(0x33ff_ffd7);

fn unchanged(last: i32) -> CreationSettle {
    CreationSettle {
        stored: None,
        pending: false,
        last,
        callback: false,
    }
}

fn debounce(value: f32, last: i32, clock: i32) -> CreationSettle {
    if last + 2 > clock {
        CreationSettle {
            stored: Some(value),
            pending: true,
            last,
            callback: false,
        }
    } else {
        CreationSettle {
            stored: Some(value),
            pending: false,
            last: clock,
            callback: true,
        }
    }
}

/// Settles an appearance value toward its target.
pub fn settle_appearance(source: f32, target: f32, last: i32, clock: i32) -> CreationSettle {
    let value = (0.5 - source) * 2.0;
    if (value - target).abs() < SETTLE_EPSILON {
        return unchanged(last);
    }
    debounce(value, last, clock)
}

/// Copies an appearance value to its target.
pub fn copy_appearance(source: f32, target: f32, last: i32, clock: i32) -> CreationSettle {
    if (source - target).abs() < SETTLE_EPSILON {
        return unchanged(last);
    }
    debounce(source, last, clock)
}

/// The creation change's inputs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CreationChangeInput {
    pub enabled: bool,
    pub female: bool,
    pub word: i32,
    pub has_child_object: bool,
    pub global_flag: bool,
}

/// What the creation change did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CreationChange {
    pub word: i32,
    pub label: CreationLabel,
    pub text_set: bool,
    pub child_call: bool,
    pub global_call: bool,
    pub first_pass: bool,
    pub second_pass: bool,
    pub callback: bool,
    pub finalized: bool,
}

/// Applies the sex option to the record word and refreshes the screen.
pub fn apply_creation_change(input: CreationChangeInput) -> CreationChange {
    if !input.enabled {
        return CreationChange {
            word: input.word,
            label: CreationLabel::None,
            text_set: false,
            child_call: false,
            global_call: false,
            first_pass: false,
            second_pass: false,
            callback: false,
            finalized: true,
        };
    }
    let word = if input.female {
        input.word | 1
    } else {
        input.word & !1
    };
    CreationChange {
        word,
        label: creation_label(word),
        text_set: true,
        child_call: input.has_child_object,
        global_call: !input.global_flag,
        first_pass: true,
        second_pass: true,
        callback: true,
        finalized: true,
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

    #[test]
    fn the_creation_appearance_copies_with_the_same_debounce() {
        assert_eq!(copy_appearance(0.5, 0.5, 0, 0).stored, None);

        let pending = copy_appearance(0.5, 0.0, 100, 101);
        assert_eq!(pending.stored, Some(0.5));
        assert!(pending.pending);
        assert_eq!(pending.last, 100);
        assert!(!pending.callback);

        let fired = copy_appearance(0.5, 0.0, 100, 102);
        assert_eq!(fired.stored, Some(0.5));
        assert!(!fired.pending);
        assert_eq!(fired.last, 102);
        assert!(fired.callback);
    }

    #[test]
    fn the_creation_change_sets_the_sex_bit_and_refreshes() {
        let female = apply_creation_change(CreationChangeInput {
            enabled: true,
            female: true,
            word: 0,
            has_child_object: true,
            global_flag: false,
        });
        assert_eq!(female.word, 1);
        assert_eq!(female.label, CreationLabel::HumanFemale);
        assert!(female.text_set);
        assert!(female.child_call);
        assert!(female.global_call);
        assert!(female.first_pass);
        assert!(female.second_pass);
        assert!(female.callback);
        assert!(female.finalized);

        let male = apply_creation_change(CreationChangeInput {
            enabled: true,
            female: false,
            word: 1,
            has_child_object: false,
            global_flag: true,
        });
        assert_eq!(male.word, 0);
        assert_eq!(male.label, CreationLabel::HumanMale);
        assert!(!male.child_call);
        assert!(!male.global_call);

        let disabled = apply_creation_change(CreationChangeInput {
            enabled: false,
            female: true,
            word: 0x201,
            has_child_object: true,
            global_flag: false,
        });
        assert_eq!(disabled.word, 0x201);
        assert_eq!(disabled.label, CreationLabel::None);
        assert!(!disabled.text_set);
        assert!(!disabled.child_call);
        assert!(!disabled.global_call);
        assert!(!disabled.first_pass);
        assert!(!disabled.second_pass);
        assert!(!disabled.callback);
        assert!(disabled.finalized);
    }
}
