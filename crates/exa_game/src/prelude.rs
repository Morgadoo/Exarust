//! The story and practice helpers the original's prelude unit holds: the
//! rotating game-tips popup, the actor templates loaded from `actors.rpk`,
//! and the practice opponent's fixed traits, team and reaction delay.
//!
//! The tips popup shows the explicit tip it is given, or, without one and
//! with the `ShowGameTips` setting on, the least-shown tip. Ties are broken
//! by the engine's generator: the count of least-shown tips is one draw,
//! and the draw picks that many entries in from the start. Shown counts
//! start at one; a tip shown again goes to two and then counts up. The tip
//! strings themselves are retail text, so the selection lives here and the
//! caller supplies the texts.
//!
//! Actor templates are read once per name from the archive and cached when
//! asked for; the uncached reader (shopkeepers, summoned demons) parses the
//! record every time. Evidence: the prelude unit's notes in the component
//! map (SG_Prelude_Main).

use std::collections::HashMap;

use exa_assets::{Archive, Game, PackKind};
use exa_formats::characters::CharacterRecord;

/// How many tips the popup rotates through.
pub const TIPS: usize = 18;
/// The actor library inside the resource pack.
pub const ACTOR_PACK: &str = "actors.rpk";
/// The role the practice opponent is put on (the arena's Unaligned role).
pub const PRACTICE_TEAM: u32 = 0x10;
/// The practice opponent's reaction delay after the fight starts, in
/// milliseconds.
pub const PRACTICE_REACTION_MS: u32 = 3000;

/// The original's generator step.
fn step(state: &mut u32) -> u32 {
    *state = state.wrapping_mul(0x0808_8405).wrapping_add(1);
    *state
}

/// The tips popup's state: the window exists once shown, the setting, the
/// shown counts and the generator.
#[derive(bevy::prelude::Resource, Debug, Clone, PartialEq, Eq)]
pub struct Tips {
    /// `ShowGameTips`; automatic tips need it.
    pub show_game_tips: bool,
    /// Whether the popup has been created (its counts were initialised).
    pub created: bool,
    /// One count per tip, all one when the popup is created.
    pub shown: [u32; TIPS],
    /// The generator state.
    pub roll: u32,
    /// The tip index on screen, if any.
    pub current: Option<usize>,
}

impl Default for Tips {
    fn default() -> Self {
        Self {
            show_game_tips: true,
            created: false,
            shown: [0; TIPS],
            roll: 0,
            current: None,
        }
    }
}

impl Tips {
    /// Creates the popup: its counts all start at one.
    fn create(&mut self) {
        self.shown = [1; TIPS];
        self.created = true;
    }

    /// Shows a tip as the original's show routine does: index 0 is the
    /// automatic least-shown tip, gated by the setting; any other index is
    /// the tip at that zero-based table position, whatever the setting.
    pub fn show(&mut self, index: usize) -> Option<usize> {
        if index == 0 && !self.show_game_tips {
            return None;
        }
        if !self.created {
            self.create();
        }
        if index != 0 {
            if index >= TIPS {
                return None;
            }
            self.current = Some(index);
            return Some(index);
        }
        let least = *self.shown.iter().min()?;
        let ties = self.shown.iter().filter(|&&count| count == least).count() as u32;
        let draw = ((u64::from(step(&mut self.roll)) * u64::from(ties)) >> 32) as usize;
        let chosen = self
            .shown
            .iter()
            .enumerate()
            .filter(|(_, count)| **count == least)
            .nth(draw)
            .map(|(at, _)| at)?;
        if self.shown[chosen] == 0 {
            self.shown[chosen] = 2;
        } else {
            self.shown[chosen] += 1;
        }
        self.current = Some(chosen);
        Some(chosen)
    }

    /// Shows the least-shown tip: [`Tips::show`] with index 0.
    pub fn show_automatic(&mut self) -> Option<usize> {
        self.show(0)
    }

    /// Hides the popup, leaving the counts as they are.
    pub fn hide(&mut self) {
        self.current = None;
    }
}

/// Every named actor template in an archive.
#[derive(Debug, Clone, Default)]
pub struct ActorTemplates {
    entries: Vec<(String, CharacterRecord)>,
}

impl ActorTemplates {
    /// Reads every entry's record; an entry that does not parse is left
    /// out. No actors, or no archive, is an empty library.
    pub fn load(game: &Game) -> Self {
        let Some(resource) = game.0.pack(PackKind::Resource) else {
            return Self::default();
        };
        let Some(bytes) = resource.get(ACTOR_PACK) else {
            return Self::default();
        };
        let Ok(archive) = Archive::parse(bytes) else {
            return Self::default();
        };
        let entries = archive
            .entries()
            .iter()
            .filter_map(|entry| {
                let record = CharacterRecord::parse(0, archive.bytes(entry)).ok()?;
                Some((entry.name.clone(), record))
            })
            .collect();
        Self { entries }
    }

    /// An empty library.
    pub fn new() -> Self {
        Self::default()
    }

    /// Builds a library from records, for callers that already hold them.
    pub fn from_records(entries: impl IntoIterator<Item = (String, CharacterRecord)>) -> Self {
        Self {
            entries: entries.into_iter().collect(),
        }
    }

    /// The template named `name`, exactly.
    pub fn get(&self, name: &str) -> Option<&CharacterRecord> {
        self.entries
            .iter()
            .find(|(entry, _)| entry == name)
            .map(|(_, record)| record)
    }

    /// How many templates were read.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the library is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// The template loader with the original's two modes: a cached lookup keeps
/// the record it read, the uncached one parses a fresh copy each time.
#[derive(Debug, Clone, Default)]
pub struct ActorCache {
    /// The library, read on the first lookup.
    library: Option<ActorTemplates>,
    /// Records the cached loader has handed out, by name.
    cached: HashMap<String, CharacterRecord>,
}

impl ActorCache {
    /// An empty cache.
    pub fn new() -> Self {
        Self::default()
    }

    fn library<'a>(&'a mut self, game: &Game) -> &'a ActorTemplates {
        self.library
            .get_or_insert_with(|| ActorTemplates::load(game))
    }

    /// A template by name: `cached` keeps the first record read and returns
    /// it again, as the story's cached loader does; either mode reads the
    /// library once.
    pub fn template(&mut self, game: &Game, name: &str, cached: bool) -> Option<CharacterRecord> {
        if cached && let Some(record) = self.cached.get(name) {
            return Some(record.clone());
        }
        let mut record = self.library(game).get(name).cloned()?;
        // A template whose own name is empty takes the name it was asked
        // for, as the original copies the requested asset name into it.
        if record.name.is_empty() {
            record.name = name.to_string();
        }
        if cached {
            self.cached.insert(name.to_string(), record.clone());
        }
        Some(record)
    }
}

/// What the practice spawn changes in a template and gives the motile.
#[derive(Debug, Clone, PartialEq)]
pub struct PracticeOpponent {
    /// The template with the practice traits set.
    pub record: CharacterRecord,
    /// Its role id: the arena's Unaligned, which is team `0x10`.
    pub team: u32,
    /// Its reaction delay after the fight starts, in milliseconds.
    pub reaction_ms: u32,
}

/// The practice opponent from `record`: its second to fourth personality
/// traits are set to −100, 100 and 100, it goes on team `0x10` and it waits
/// three seconds after the fight starts.
pub fn practice_opponent(mut record: CharacterRecord) -> PracticeOpponent {
    record.traits[1] = -100.0;
    record.traits[2] = 100.0;
    record.traits[3] = 100.0;
    PracticeOpponent {
        record,
        team: PRACTICE_TEAM,
        reaction_ms: PRACTICE_REACTION_MS,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_tip_counts_up_and_hiding_keeps_the_counts() {
        let mut tips = Tips::default();
        assert_eq!(tips.shown, [0; TIPS]);
        assert_eq!(tips.show(3), Some(3));
        assert!(tips.created);
        assert_eq!(tips.current, Some(3));
        assert_eq!(tips.shown, [1; TIPS], "indexed tips do not count");
        tips.hide();
        assert_eq!(tips.current, None);
        assert_eq!(tips.shown, [1; TIPS]);
    }

    #[test]
    fn the_least_shown_tip_is_drawn_and_counted() {
        let mut tips = Tips::default();
        let first = tips.show_automatic().unwrap();
        assert!(first < TIPS);
        assert_eq!(tips.shown[first], 2);
        let mut again = Tips {
            created: true,
            roll: tips.roll,
            shown: tips.shown,
            ..Tips::default()
        };
        let second = again.show_automatic().unwrap();
        assert_ne!(second, first, "the first tip now has a higher count");
        assert_eq!(again.shown[second], 2);
        // The draws are the generator's high word times the tie count.
        let mut state = 0u32;
        let draw = ((u64::from(step(&mut state)) * 17) >> 32) as usize;
        assert!(draw < 17);
    }

    #[test]
    fn the_setting_gates_only_automatic_tips() {
        let mut tips = Tips {
            show_game_tips: false,
            ..Tips::default()
        };
        assert_eq!(tips.show_automatic(), None);
        assert!(!tips.created, "a refused automatic tip creates nothing");
        assert_eq!(tips.show(1), Some(1), "an explicit tip still shows");
    }

    #[test]
    fn an_out_of_range_tip_shows_nothing() {
        let mut tips = Tips::default();
        assert_eq!(tips.show(TIPS), None);
        assert_eq!(tips.show(TIPS + 1), None);
        assert_eq!(tips.current, None);
    }

    #[test]
    fn the_draw_can_pick_a_later_tie_and_counts_only_that_one() {
        let mut tips = Tips {
            created: true,
            shown: [3; TIPS],
            roll: 0,
            ..Tips::default()
        };
        tips.shown[5] = 1;
        tips.shown[9] = 1;
        // Two ties; the draw is 0 or 1.
        let mut state = 0u32;
        let draw = ((u64::from(step(&mut state)) * 2) >> 32) as usize;
        let chosen = if draw == 0 { 5 } else { 9 };
        let got = tips.show_automatic().unwrap();
        assert_eq!(got, chosen);
        assert_eq!(tips.shown[chosen], 2);
        assert_eq!(tips.shown[if chosen == 5 { 9 } else { 5 }], 1);
    }

    #[test]
    fn an_actor_cache_reads_named_records_and_caches_them() {
        let mut record = CharacterRecord::reset();
        record.name = "Opponent".into();
        let library = ActorTemplates::from_records([("opponent01".to_string(), record.clone())]);
        assert_eq!(library.len(), 1);
        assert_eq!(
            library.get("opponent01").map(|r| r.name.as_str()),
            Some("Opponent")
        );
        assert!(library.get("Opponent").is_none(), "the entry name is exact");
        assert!(library.get("missing").is_none());
        // The cache's bookkeeping without a game: an empty library.
        let mut cache = ActorCache::new();
        assert_eq!(cache.cached.len(), 0);
        cache.cached.insert("opponent01".into(), record.clone());
        assert_eq!(
            cache.cached.get("opponent01").map(|r| r.name.clone()),
            Some("Opponent".to_string())
        );
    }

    #[test]
    fn the_practice_opponent_gets_its_traits_team_and_delay() {
        let record = CharacterRecord::reset();
        let opponent = practice_opponent(record);
        assert_eq!(opponent.record.traits[1], -100.0);
        assert_eq!(opponent.record.traits[2], 100.0);
        assert_eq!(opponent.record.traits[3], 100.0);
        assert_eq!(opponent.team, 0x10);
        assert_eq!(opponent.reaction_ms, 3000);
    }

    #[test]
    fn the_shipped_actor_library_has_opponents() {
        let Some(dir) = exa_rpk::locate_install(None) else {
            return;
        };
        let Ok(data) = exa_rpk::GameData::open(&dir) else {
            return;
        };
        let game = Game(std::sync::Arc::new(data));
        let library = ActorTemplates::load(&game);
        assert_eq!(library.len(), 21);
        assert!(library.get("opponent01").is_some());
        let mut cache = ActorCache::new();
        let first = cache.template(&game, "opponent01", true).unwrap();
        let again = cache.template(&game, "opponent01", true).unwrap();
        assert_eq!(first, again);
        assert!(cache.template(&game, "missing", true).is_none());
    }
}
