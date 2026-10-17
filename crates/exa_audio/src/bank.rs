//! Sounds decoded from `Sound.rpk` by name, and the one picker every sound
//! event uses to choose a variant of a numbered sound set.

use std::collections::HashMap;
use std::sync::Arc;

use bevy::prelude::*;
use exa_assets::{Game, PackKind};

/// A set of numbered sound variants, named as the original names them: its
/// prefix and a two-digit number from 01 to `count` (`wh_sworda01` to
/// `wh_sworda06`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SoundSet<'a> {
    pub prefix: &'a str,
    pub count: usize,
}

impl<'a> SoundSet<'a> {
    pub const fn new(prefix: &'a str, count: usize) -> Self {
        Self { prefix, count }
    }

    /// The name of variant `index` (0 is the set's 01).
    pub fn name(&self, index: usize) -> String {
        format!("{}{:02}", self.prefix, index + 1)
    }
}

/// Decoded sounds by name, and the picker's state.
#[derive(Resource, Default)]
pub struct SoundBank {
    loaded: HashMap<String, Option<Handle<AudioSource>>>,
    seed: u64,
}

impl SoundBank {
    pub fn get(
        &mut self,
        game: &Game,
        sources: &mut Assets<AudioSource>,
        name: &str,
    ) -> Option<Handle<AudioSource>> {
        let key = name.to_ascii_lowercase();
        if let Some(h) = self.loaded.get(&key) {
            return h.clone();
        }
        let handle = game
            .0
            .pack(PackKind::Sound)
            .and_then(|p| p.get(&key))
            .map(|bytes| {
                sources.add(AudioSource {
                    bytes: Arc::from(bytes),
                })
            });
        if handle.is_none() {
            warn!("sound {name:?} not found");
        }
        self.loaded.insert(key, handle.clone());
        handle
    }

    /// A pseudo-random index in `0..n` (xorshift; audio variety only).
    pub fn pick(&mut self, n: usize) -> usize {
        self.seed ^= self.seed << 13;
        self.seed ^= self.seed >> 7;
        self.seed ^= self.seed << 17;
        if self.seed == 0 {
            self.seed = 0x9E37_79B9_7F4A_7C15;
        }
        (self.seed % n.max(1) as u64) as usize
    }

    /// The name of a variant of `set`, picked at random.
    pub fn pick_from(&mut self, set: SoundSet) -> String {
        let index = self.pick(set.count);
        set.name(index)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Variants are numbered from 01 with two digits.
    #[test]
    fn sound_sets_name_their_variants_from_01() {
        let swords = SoundSet::new("wh_sworda", 6);
        assert_eq!(swords.name(0), "wh_sworda01");
        assert_eq!(swords.name(5), "wh_sworda06");
        assert_eq!(SoundSet::new("vhm09_grunts", 6).name(2), "vhm09_grunts03");
    }

    /// The picker stays within a set, and its first pick from a fresh bank
    /// is the same as before the picker was shared: the seed restarts at
    /// 0x9E3779B97F4A7C15, so variant 2 of six and voice 8 of nine.
    #[test]
    fn the_picker_stays_within_the_set() {
        let mut bank = SoundBank::default();
        assert_eq!(bank.pick_from(SoundSet::new("wh_sworda", 6)), "wh_sworda02");
        let mut fresh = SoundBank::default();
        assert_eq!(fresh.pick(9), 7);
        for _ in 0..500 {
            let name = bank.pick_from(SoundSet::new("stpmed", 6));
            let n: usize = name["stpmed".len()..].parse().unwrap();
            assert!((1..=6).contains(&n), "{name}");
        }
    }
}
