//! The discovery and experience system in the running story.
//!
//! [`crate::experience`] holds the original's rules as plain functions: the
//! account pool, the player's and the companions' conversions, the explored
//! levels and the technique word. This module runs them: it reads the
//! account-wide experience from the launch's settings store, seeds the pool
//! from the player's carried experience when play starts (the original's
//! level-start hook), converts once for every whole second of play, puts the
//! technique word on the player's body, and writes the account experience
//! back to the settings when play ends.
//!
//! The explored-cell half of the tick needs the level's sector grid; the
//! running game does not hand one to the experience system yet, so the live
//! tick only converts. [`crate::experience::Experience::explore`] and its
//! [`crate::experience::ExploreGrid`] stay the tested rule for it.

use bevy::prelude::*;
use exa_formats::characters::CharacterRecord;

use crate::experience::{
    Conversion, Experience, GLOBAL_XP_KEY, Learner, convert_companion, global_xp_from_setting,
    global_xp_to_setting, technique_bits,
};
use crate::flow::SettingsEntry;
use crate::player::Player;
use crate::player_controls::PlaySession;

/// The account-wide experience read from the launch's settings store, when
/// it held the entry. The binary fills this before the first frame.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AccountExperience(pub Option<[u8; 4]>);

impl AccountExperience {
    /// The stored account experience under [`GLOBAL_XP_KEY`], if present.
    #[must_use]
    pub fn from_store(store: &exa_formats::settings_store::SettingsStore) -> Self {
        Self(
            store
                .get(&GLOBAL_XP_KEY)
                .and_then(|value| value.get(0..4))
                .map(|four| [four[0], four[1], four[2], four[3]]),
        )
    }
}

/// The technique word the body holds, read again when a skill reaches a new
/// level. The original keeps it on the body; the experience system writes it
/// here for the body's systems to read.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TechniqueWord(pub u32);

/// The story's live experience: the account state, the player's learner, the
/// companions in list order and the last whole second converted.
#[derive(Resource, Debug)]
pub struct StoryExperience {
    pub state: Experience,
    pub player: Learner,
    pub party: Vec<Learner>,
    /// The player's body technique word after its last rise.
    pub technique_word: u32,
    /// Play started and the level-start hook ran.
    pub started: bool,
    /// The last whole second the tick converted at.
    pub last_second: u32,
    /// The account experience last written to the settings, so an unchanged
    /// value is not written twice.
    pub written_global: Option<u32>,
}

impl Default for StoryExperience {
    fn default() -> Self {
        Self {
            state: Experience::default(),
            player: Learner::default(),
            party: Vec::new(),
            technique_word: 0,
            started: false,
            last_second: 0,
            written_global: Some(0),
        }
    }
}

impl StoryExperience {
    /// The level start: the player's carried experience becomes the account
    /// pool, both clocks restart at `now`, and the body takes the technique
    /// word its skills name.
    pub fn begin(&mut self, record: &CharacterRecord, now: u32) {
        self.player = Learner::from_record(record);
        self.state.begin();
        self.state.start_level(now, &mut self.player);
        self.technique_word = technique_bits(&self.player.skills);
        self.started = true;
        self.last_second = now / 1000;
    }

    /// One whole second of play, if the second has changed: the account
    /// pool then the player, then each companion in list order. Returns the
    /// second's conversions (the account's has no [`Conversion`] of its own)
    /// and whether the body's technique word changed.
    pub fn advance(&mut self, now: u32) -> bool {
        if !self.started {
            return false;
        }
        let second = now / 1000;
        if second <= self.last_second {
            return false;
        }
        self.last_second = second;
        self.state.convert_global(now);
        let Conversion { techniques, .. } = self.state.convert_player(now, &mut self.player);
        for companion in &mut self.party {
            convert_companion(now, companion);
        }
        if let Some(word) = techniques {
            let changed = word != self.technique_word;
            self.technique_word = word;
            return changed;
        }
        false
    }

    /// The account experience changed since it was last written.
    pub fn unsaved_global(&self) -> Option<u32> {
        (self.written_global != Some(self.state.global)).then_some(self.state.global)
    }

    /// The story's discovery chunk, the format a save carries.
    pub fn discovery(&self) -> Vec<u8> {
        self.state.write_discovery()
    }
}

/// Reads the launch's account experience into the live state once.
fn load_account_experience(
    settings: Res<AccountExperience>,
    mut live: ResMut<StoryExperience>,
    mut done: Local<bool>,
) {
    if *done {
        return;
    }
    *done = true;
    if let Some(bytes) = settings.0
        && let Some(xp) = global_xp_from_setting(&bytes)
    {
        live.state.global = xp;
        live.written_global = Some(xp);
    }
}

/// The level-start hook: the first frame of play takes the player's record.
fn begin_story_experience(
    time: Res<Time>,
    play: Option<Res<PlaySession>>,
    players: Query<&exa_character::record::CharacterData, With<Player>>,
    mut live: ResMut<StoryExperience>,
) {
    if live.started || !play.is_some_and(|play| play.playing) {
        return;
    }
    let Some(data) = players.iter().next() else {
        return;
    };
    live.begin(&data.record, now_ms(&time));
}

/// The once-a-second tick, with the technique word put on the player's body.
fn tick_story_experience(
    time: Res<Time>,
    play: Option<Res<PlaySession>>,
    players: Query<Entity, With<Player>>,
    mut commands: Commands,
    mut live: ResMut<StoryExperience>,
) {
    if !play.is_some_and(|play| play.playing) {
        return;
    }
    let changed = live.advance(now_ms(&time));
    if changed && let Ok(player) = players.single() {
        commands
            .entity(player)
            .insert(TechniqueWord(live.technique_word));
    }
}

/// Play ended: the account experience goes to the settings store when it
/// changed.
fn store_account_experience(
    play: Option<Res<PlaySession>>,
    mut live: ResMut<StoryExperience>,
    mut entries: MessageWriter<SettingsEntry>,
) {
    if !live.started || play.is_some_and(|play| play.playing) {
        return;
    }
    live.started = false;
    if let Some(xp) = live.unsaved_global() {
        entries.write(SettingsEntry {
            section: GLOBAL_XP_KEY[0],
            key: GLOBAL_XP_KEY[1],
            bytes: global_xp_to_setting(xp),
        });
        live.written_global = Some(xp);
    }
}

/// The game clock in milliseconds, as the experience system's clock.
fn now_ms(time: &Time) -> u32 {
    time.elapsed().as_millis().min(u128::from(u32::MAX)) as u32
}

pub struct ExperienceLivePlugin;

impl Plugin for ExperienceLivePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<AccountExperience>()
            .init_resource::<StoryExperience>()
            .add_systems(
                Update,
                (
                    load_account_experience,
                    begin_story_experience,
                    tick_story_experience,
                    store_account_experience,
                )
                    .chain(),
            );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use exa_formats::characters::Skill;

    fn record(experience: i32) -> CharacterRecord {
        let mut record = CharacterRecord::new();
        record.experience = experience;
        record.skills[0].points = 5000;
        record.skills[0].techniques[0] = 1;
        record
    }
    #[test]
    fn the_level_start_seeds_the_pool_and_restarts_both_clocks() {
        let mut live = StoryExperience::default();
        live.state.global = 700;
        live.state.player_clock = 1;
        live.state.global_clock = 2;
        live.begin(&record(900), 12_345);
        assert_eq!(live.state.pool, 900);
        assert_eq!(live.state.global, 700);
        assert_eq!(
            (live.state.player_clock, live.state.global_clock),
            (12_345, 12_345)
        );
        assert_eq!(live.last_second, 12);
        assert!(live.started);
        // The technique word is the skills' at the start.
        assert_eq!(live.technique_word, technique_bits(&live.player.skills));
    }

    #[test]
    fn the_tick_runs_once_per_whole_second() {
        let mut live = StoryExperience::default();
        live.begin(&record(600), 0);
        // Under a second, the clock does not move and nothing converts.
        assert!(!live.advance(999));
        assert_eq!(live.player.experience, 600);
        // At one second the player's cost (4 + 1 level) is 5, and the pool
        // owes 1000 / (50 · 5) = 4 points.
        live.advance(1000);
        assert_eq!(live.player.experience, 600 - 4 * 5);
        assert_eq!(live.state.player_clock, 1000);
        // The same second does not run again.
        assert!(!live.advance(1500));
    }

    #[test]
    fn the_body_takes_the_technique_word_on_a_rise() {
        let mut live = StoryExperience::default();
        let mut record = record(2_000);
        // A point short of the second level, whose technique is chosen and
        // unfunded until the level rises.
        record.skills[0].points = 9_999;
        record.skills[0].techniques[1] = 2;
        live.begin(&record, 0);
        assert_eq!(live.technique_word, 0x1);
        live.advance(1000);
        assert_eq!(live.technique_word, 0x3);
    }

    #[test]
    fn the_account_experience_is_read_from_its_settings_value() {
        let mut store = exa_formats::settings_store::SettingsStore::new();
        store.set(&GLOBAL_XP_KEY, &1234u32.to_le_bytes()).unwrap();
        let settings = AccountExperience::from_store(&store);
        assert_eq!(settings.0, Some(1234u32.to_le_bytes()));
        assert_eq!(
            AccountExperience::from_store(&exa_formats::settings_store::SettingsStore::new()).0,
            None
        );
    }

    #[test]
    fn only_a_changed_account_experience_is_written_once() {
        let mut live = StoryExperience::default();
        assert_eq!(live.unsaved_global(), None);
        live.state.global = 10;
        assert_eq!(live.unsaved_global(), Some(10));
        live.written_global = Some(10);
        assert_eq!(live.unsaved_global(), None);
        live.state.global = 20;
        assert_eq!(live.unsaved_global(), Some(20));
    }

    #[test]
    fn the_technique_word_component_carries_the_word() {
        assert_eq!(TechniqueWord(0x102).0, 0x102);
        let _ = Skill::default();
    }
}
