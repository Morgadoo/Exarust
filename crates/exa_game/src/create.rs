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
}
