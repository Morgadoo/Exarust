//! The arena's record log.
//!
//! Two appenders grow a list of twenty-byte records: one id and a value,
//! or one id and four words. Evidence: the 113-byte `0x100184080` and the
//! 127-byte `0x100184100`.

/// One record in the arena log.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArenaRecord {
    pub id: i32,
    pub words: [i32; 4],
}

/// The arena's record list.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ArenaLog {
    pub records: Vec<ArenaRecord>,
}

impl ArenaLog {
    /// Appends an id and one value, returning the new slot.
    pub fn append_value(&mut self, id: i32, value: i32) -> usize {
        let slot = self.records.len();
        self.records.push(ArenaRecord {
            id,
            words: [value, 0, 0, 0],
        });
        slot
    }

    /// Appends an id and four words, returning the new slot.
    pub fn append_words(&mut self, id: i32, words: [i32; 4]) -> usize {
        let slot = self.records.len();
        self.records.push(ArenaRecord { id, words });
        slot
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_arena_log_appends_values_and_words() {
        let mut log = ArenaLog::default();
        assert_eq!(log.append_value(3, 7), 0);
        assert_eq!(
            log.records[0],
            ArenaRecord {
                id: 3,
                words: [7, 0, 0, 0],
            }
        );

        assert_eq!(log.append_words(5, [1, 2, 3, 4]), 1);
        assert_eq!(
            log.records[1],
            ArenaRecord {
                id: 5,
                words: [1, 2, 3, 4],
            }
        );
        assert_eq!(log.records.len(), 2);
    }
}
