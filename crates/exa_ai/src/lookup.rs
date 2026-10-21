//! The two small lookups the character physics shares: a simulation's
//! character by its record id, and a status effect by a masked word.
//!
//! Evidence: the original's lookup routines in the character physics unit.
//! Both walk their list in order and return the first match; an empty list
//! finds nothing.

/// The index of the character whose record id is `record` (the first one,
/// searching the list in order).
pub fn character_with_record(records: impl IntoIterator<Item = u32>, record: u32) -> Option<usize> {
    records.into_iter().position(|id| id == record)
}

/// The index of the first status effect whose word, masked, is `wanted`:
/// `(word & mask) == wanted`.
pub fn status_with_mask(
    words: impl IntoIterator<Item = u32>,
    wanted: u32,
    mask: u32,
) -> Option<usize> {
    words.into_iter().position(|word| word & mask == wanted)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_lookups_take_the_first_match() {
        assert_eq!(character_with_record([3, 7, 9, 7], 7), Some(1));
        assert_eq!(character_with_record([3, 7, 9], 4), None);
        assert_eq!(character_with_record([], 1), None);
        // Status words: a mask of zero takes the first wanted word of zero.
        assert_eq!(status_with_mask([0x10, 0x21, 0x31], 0x20, 0xF0), Some(1));
        assert_eq!(status_with_mask([0x10, 0x21], 0x40, 0xF0), None);
        assert_eq!(status_with_mask([0x10, 0x21], 0x11, 0xFF), None);
        assert_eq!(status_with_mask([0x10, 0x21], 0x21, 0xFF), Some(1));
        assert_eq!(status_with_mask([], 0, 0), None);
    }
}
