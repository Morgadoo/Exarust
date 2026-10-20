//! The arena's record log.
//!
//! Two appenders grow a list of twenty-byte records: one id and a value,
//! or one id and four words. Evidence: the retail arena's two appenders
//! (`docs/formats/OTHER.md`, Arena tables).

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

/// One arena pick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArenaPick {
    pub value: i32,
    pub seed: u32,
    pub index: usize,
}

/// Picks one of three values with the arena's `0x08088405` generator.
pub fn pick_arena_value(values: [i32; 3], seed: u32) -> ArenaPick {
    let count = values.iter().filter(|&&value| value > 0).count();
    let seed = seed.wrapping_mul(0x0808_8405).wrapping_add(1);
    let index = ((u64::from(seed) * count as u64) >> 32) as usize;
    ArenaPick {
        value: values[index],
        seed,
        index,
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

    #[test]
    fn the_arena_pick_scales_the_seed_by_the_count() {
        let one = pick_arena_value([5, 0, 0], 0);
        assert_eq!(one.seed, 1);
        assert_eq!(one.index, 0);
        assert_eq!(one.value, 5);

        let three = pick_arena_value([3, 7, 9], 0);
        assert_eq!(three.index, 0);
        assert_eq!(three.value, 3);

        let leading = pick_arena_value([0, 3, 0], 0);
        assert_eq!(leading.index, 0);
        assert_eq!(leading.value, 0);
    }
}
