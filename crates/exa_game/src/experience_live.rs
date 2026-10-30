//! The discovery and experience system in the running story.
//!
//! [`crate::experience`] holds the original's rules as plain functions: the
//! account pool, the player's and the companions' conversions, the explored
//! levels and the technique word. This module runs them: it reads the
//! account-wide experience from the launch's settings store, seeds the pool
//! from the player's carried experience when play starts (the original's
//! level-start hook), converts at each of the flow's once-a-second
//! [`crate::flow::Step::Discovery`] passes ([`DiscoveryTick`], on the game
//! clock), writes the player's converted experience, skills and powers back
//! to the body's record (the original converts the body's own record), puts
//! the technique word on the player's body, and writes the account
//! experience back to the settings when play ends.
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

/// The flow's once-a-second discovery pass ran at `now`, on the game clock
/// (ms). [`crate::flow`]'s step application sends it.
#[derive(Message, Clone, Copy, Debug, PartialEq, Eq)]
pub struct DiscoveryTick {
    pub now: u32,
}

/// The story's live experience: the account state, the player's learner and
/// the companions in list order.
#[derive(Resource, Debug)]
pub struct StoryExperience {
    pub state: Experience,
    pub player: Learner,
    pub party: Vec<Learner>,
    /// The player's body technique word after its last rise.
    pub technique_word: u32,
    /// Play started and the level-start hook ran.
    pub started: bool,
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
    }

    /// One discovery pass (the flow gates it to once a second): the account
    /// pool, then the player, then each companion in list order. Returns
    /// whether the body's technique word changed. The explored cells are
    /// not walked: no level grid is handed to the system yet.
    pub fn advance(&mut self, now: u32) -> bool {
        if !self.started {
            return false;
        }
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

    /// Writes the player's converted state back to its record: the
    /// experience still to convert, the skills and the powers' words.
    pub fn write_player(&self, record: &mut CharacterRecord) {
        record.experience = self.player.experience as i32;
        record.skills = self.player.skills;
        if let Some(powers) = record.powers.as_mut() {
            powers.known.clone_from(&self.player.powers);
        }
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
    ticks: Option<Res<exa_physics::body::BodyTicks>>,
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
    live.begin(&data.record, now_ms(ticks.as_deref()));
}

/// The flow's discovery pass: convert, write the player's record back and
/// put the technique word on the body when it changed.
fn tick_story_experience(
    mut passes: MessageReader<DiscoveryTick>,
    mut players: Query<(Entity, &mut exa_character::record::CharacterData), With<Player>>,
    mut commands: Commands,
    mut live: ResMut<StoryExperience>,
) {
    for pass in passes.read() {
        if !live.started {
            continue;
        }
        let changed = live.advance(pass.now);
        if let Ok((player, mut data)) = players.single_mut() {
            live.write_player(&mut data.record);
            if changed {
                commands
                    .entity(player)
                    .insert(TechniqueWord(live.technique_word));
            }
        }
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

/// The game clock in milliseconds (the body ticks' clock the flow runs on).
fn now_ms(ticks: Option<&exa_physics::body::BodyTicks>) -> u32 {
    ticks.map_or(0, |ticks| ticks.millis() as u32)
}

pub struct ExperienceLivePlugin;

impl Plugin for ExperienceLivePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<AccountExperience>()
            .init_resource::<StoryExperience>()
            .add_message::<DiscoveryTick>()
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
        assert!(live.started);
        // The technique word is the skills' at the start.
        assert_eq!(live.technique_word, technique_bits(&live.player.skills));
    }

    #[test]
    fn a_pass_converts_and_writes_the_record_back() {
        let mut live = StoryExperience::default();
        // Nothing converts before the level-start hook.
        assert!(!live.advance(1000));
        let mut player = record(600);
        // The first skill trains toward its second level.
        player.skills[0].techniques[1] = 2;
        live.begin(&player, 0);
        // At one second the player's cost (4 + 1 level) is 5, and the pool
        // owes 1000 / (50 · 5) = 4 points.
        live.advance(1000);
        assert_eq!(live.player.experience, 600 - 4 * 5);
        assert_eq!(live.state.player_clock, 1000);
        live.write_player(&mut player);
        assert_eq!(player.experience, 580);
        // The record takes the learner's skills: the trained one gained
        // two skill points per converted point.
        assert_eq!(player.skills, live.player.skills);
        let total = |skills: &[exa_formats::characters::Skill]| {
            skills.iter().map(|s| u32::from(s.points)).sum::<u32>()
        };
        assert_eq!(player.skills[0].points, 5000 + 2 * 4);
        assert_eq!(total(&player.skills), total(&record(600).skills) + 2 * 4);
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
