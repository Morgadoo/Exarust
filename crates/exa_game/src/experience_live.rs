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
//! The player's record lives on the player's body (the entity with
//! [`crate::player::PlayerBody`]), as its other body state does. A rise that
//! changes the technique word puts it on the body's caster (its technique
//! bits) and adds the record's perks to the body's fighter
//! ([`apply_technique_word`]); the level start puts the starting word there
//! the same way.
//!
//! Each pass first explores, as the original's tick does: in a grid level
//! the 5 × 5 cells around the player's cell are walked
//! ([`crate::experience::Experience::explore`]) over the level's rooms
//! ([`RoomCells`]): a cell counts once it is open (plane A's open flag) and
//! its room is active in the room manager's tracker (the room's working
//! state low bit, [`exa_world::rooms::RoomTracker::is_active`]), so a room
//! is explored only once it has faded in. The level's map of explored
//! cells is found by its locale name, or added empty at the grid's size,
//! before the first walk in it (the original's level-enter hook). The
//! item-kind and kill hooks have no caller yet (see the module README).

use bevy::prelude::*;
use exa_formats::characters::CharacterRecord;

use exa_world::rooms::{RoomTracker, Rooms};

use crate::experience::{
    Conversion, Experience, ExploreGrid, GLOBAL_XP_KEY, Learner, convert_companion,
    global_xp_from_setting, global_xp_to_setting, technique_bits,
};
use crate::flow::SettingsEntry;
use crate::player::PlayerBody;
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
#[derive(Resource, Debug, Clone)]
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

    /// The exploring half of a discovery pass, before [`Self::advance`]: in
    /// the level named `locale` (its map entered first when it is not the
    /// current one), the cells around `x`, `z` (centimetres) that `grid`
    /// shows open and active earn their points for the account pool, the
    /// player and each companion. Returns the cells newly found.
    pub fn explore(&mut self, grid: &impl ExploreGrid, locale: &str, x: f32, z: f32) -> u32 {
        if !self.started {
            return 0;
        }
        let name = locale_name(locale);
        let entered = self
            .state
            .current
            .and_then(|i| self.state.maps.get(i))
            .is_some_and(|map| map.name == name);
        if !entered {
            let (width, height) = grid.size();
            self.state.enter_level(name, width, height);
        }
        self.state
            .explore(grid, x, z, &mut self.player, &mut self.party)
    }

    /// One discovery pass (the flow gates it to once a second): the account
    /// pool, then the player, then each companion in list order. Returns
    /// whether the body's technique word changed. [`Self::explore`] runs
    /// before it.
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

/// The level-start hook: the first frame of play takes the player's record,
/// and the body takes the technique word its skills name.
fn begin_story_experience(
    ticks: Option<Res<exa_physics::body::BodyTicks>>,
    play: Option<Res<PlaySession>>,
    players: Query<(Entity, &exa_character::record::CharacterData), With<PlayerBody>>,
    loaded: Option<Res<crate::save_live::LoadedStory>>,
    mut commands: Commands,
    mut live: ResMut<StoryExperience>,
) {
    if live.started || !play.is_some_and(|play| play.playing) {
        return;
    }
    let Some((player, data)) = players.iter().next() else {
        return;
    };
    let now = now_ms(ticks.as_deref());
    live.begin(&data.record, now);
    if let Some(loaded) = loaded.as_deref() {
        crate::save_live::apply_loaded_discovery(&mut live, &loaded.0);
    }
    info!(
        "experience: level start at {now} ms: pool {}, account {}, technique word {:#010x}",
        live.state.pool, live.state.global, live.technique_word
    );
    commands
        .entity(player)
        .insert(TechniqueWord(live.technique_word));
}

/// A level's name as the explored-cell maps key it: the locale's bytes,
/// zero padded (or cut) to 16.
pub fn locale_name(locale: &str) -> [u8; 16] {
    let mut name = [0; 16];
    for (to, from) in name.iter_mut().zip(locale.bytes()) {
        *to = from;
    }
    name
}

/// The level's rooms as the exploration reads the grid: the grid's size,
/// each cell's room word (room + 1, 0 for none) from the room manager's
/// cell table, a room's active flag from its tracker and each cell's plane A
/// word.
pub struct RoomCells<'a> {
    pub rooms: &'a Rooms,
    pub tracker: &'a RoomTracker,
}

impl ExploreGrid for RoomCells<'_> {
    fn tracks(&self) -> bool {
        true
    }

    fn size(&self) -> (i32, i32) {
        (self.rooms.width() as i32, self.rooms.height() as i32)
    }

    fn sector_word(&self, cell: u32) -> Option<i32> {
        Some(
            self.rooms
                .room_of_index(cell)
                .map_or(0, |room| i32::from(room) + 1),
        )
    }

    fn sector_open(&self, sector: i32) -> bool {
        u16::try_from(sector).is_ok_and(|room| self.tracker.is_active(room))
    }

    fn cell_width(&self) -> i32 {
        self.rooms.width() as i32
    }

    fn cell_flags(&self, cell: u32) -> u32 {
        self.rooms.cell_word(cell)
    }
}

/// The flow's discovery pass: explore around the player in a grid level,
/// convert, write the player's record back and put the technique word on
/// the body when it changed.
fn tick_story_experience(
    mut passes: MessageReader<DiscoveryTick>,
    mut players: Query<
        (
            Entity,
            &GlobalTransform,
            &mut exa_character::record::CharacterData,
        ),
        With<PlayerBody>,
    >,
    level: Option<Res<exa_world::room_view::LevelRooms>>,
    source: Option<Res<exa_world::LevelSource>>,
    mut commands: Commands,
    mut live: ResMut<StoryExperience>,
) {
    for pass in passes.read() {
        if !live.started {
            continue;
        }
        if let (Some(level), Some(source), Ok((_, at, _))) =
            (level.as_deref(), source.as_deref(), players.single())
        {
            let p = at.translation() * 100.0;
            let grid = RoomCells {
                rooms: &level.rooms,
                tracker: &level.tracker,
            };
            let found = live.explore(&grid, source.locale(), p.x, p.z);
            if found > 0 {
                info!(
                    "experience: explored {found} cells of {} at {} ms (pool {})",
                    source.locale(),
                    pass.now,
                    live.state.pool
                );
            }
        }
        let changed = live.advance(pass.now);
        if let Ok((player, _, mut data)) = players.single_mut() {
            live.write_player(&mut data.record);
            if changed {
                info!(
                    "experience: a skill rose at {} ms: technique word {:#010x}",
                    pass.now, live.technique_word
                );
                commands
                    .entity(player)
                    .insert(TechniqueWord(live.technique_word));
            }
        }
    }
}

/// The bodies whose technique word changed, with what reads it.
type TechniqueReaders<'w, 's> = Query<
    'w,
    's,
    (
        &'static TechniqueWord,
        Option<&'static exa_character::record::CharacterData>,
        Option<&'static mut crate::thaum::cast::PowerCaster>,
        Option<&'static mut exa_combat::Fighter>,
    ),
    Changed<TechniqueWord>,
>;

/// The body reads its technique word again: the caster's technique bits
/// become the word, and the fighter gains the perks the record's techniques
/// now give (the perks a launch option gave stay).
pub fn apply_technique_word(mut bodies: TechniqueReaders) {
    for (word, data, caster, fighter) in &mut bodies {
        if let Some(mut caster) = caster {
            caster.body.techniques = word.0;
        }
        if let (Some(mut fighter), Some(data)) = (fighter, data) {
            for perk in data.record.perks().iter() {
                fighter.perks.insert(perk);
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
                    apply_technique_word,
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

    /// The body's caster takes the new word, and its fighter the perks the
    /// record's techniques give; perks it already had stay.
    #[test]
    fn a_changed_technique_word_reaches_the_caster_and_the_fighter() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_systems(Update, apply_technique_word);
        let mut record = record(0);
        // Close combat's first technique, funded.
        record.skills[0].points = 5000;
        let caster = crate::thaum::cast::PowerCaster::new(&[]);
        let fighter = exa_combat::Fighter::default();
        let body = app
            .world_mut()
            .spawn((
                exa_character::record::CharacterData::of_record(record.clone()),
                caster,
                fighter,
                TechniqueWord(technique_bits(&record.skills)),
            ))
            .id();
        app.update();
        let caster = app
            .world()
            .get::<crate::thaum::cast::PowerCaster>(body)
            .unwrap();
        assert_eq!(caster.body.techniques, technique_bits(&record.skills));
        let fighter = app.world().get::<exa_combat::Fighter>(body).unwrap();
        for perk in record.perks().iter() {
            assert!(fighter.perks.has(perk));
        }
    }

    #[test]
    fn the_technique_word_component_carries_the_word() {
        assert_eq!(TechniqueWord(0x102).0, 0x102);
        let _ = Skill::default();
    }

    /// A 64 × 6 grid whose cells 1..=10 of rows 1..=4 are open and joined
    /// into one room.
    fn one_room() -> Rooms {
        let (w, h) = (64u32, 6u32);
        let mut grid = exa_formats::level::LevelGrid {
            wall_sets: vec!["GEWS_UWA01".into()],
            floor_sets: vec!["GEFS_UWA01".into()],
            width: w,
            height: h,
            edges: vec![0; (w * h) as usize],
            plane_b: vec![0; (w * h) as usize],
            plane_c: vec![0; (w * h) as usize],
        };
        for y in 1..=4 {
            for x in 1..=10 {
                grid.edges[(y * w + x) as usize] = exa_world::rooms::CELL_OPEN;
            }
        }
        Rooms::flood(&grid)
    }

    /// The pass explores the open cells of active rooms around the player
    /// once, in the level's own map; a room not yet faded in counts nothing.
    #[test]
    fn the_pass_explores_the_open_cells_of_active_rooms() {
        let rooms = one_room();
        // Cell (5, 2): x −159 m, z −3 m.
        let (x, z) = (-15_900.0, -300.0);
        let at = bevy::math::Vec2::new(x / 100.0, z / 100.0);
        let mut tracker = RoomTracker::new(rooms.len());
        let mut live = StoryExperience::default();
        live.begin(&record(0), 0);
        let pool = live.state.pool;
        // Entering: not active yet, nothing found, but the map is entered.
        tracker.update(&rooms, at, 0);
        let grid = RoomCells {
            rooms: &rooms,
            tracker: &tracker,
        };
        assert_eq!(live.explore(&grid, "exanima01", x, z), 0);
        assert_eq!(live.state.maps.len(), 1);
        assert_eq!(live.state.maps[0].name, locale_name("exanima01"));
        assert_eq!(
            (live.state.maps[0].width, live.state.maps[0].height),
            (64, 6)
        );
        tracker.update(&rooms, at, 2_000);
        let grid = RoomCells {
            rooms: &rooms,
            tracker: &tracker,
        };
        // Columns 3..=7, rows 1..=4 (row 0 is under the border clamp).
        assert_eq!(live.explore(&grid, "exanima01", x, z), 20);
        assert!(live.state.pool > pool);
        assert_eq!(live.explore(&grid, "exanima01", x, z), 0);
        // Another level gets its own map; coming back finds the first.
        assert_eq!(live.explore(&grid, "exanima02", x, z), 20);
        assert_eq!(live.state.maps.len(), 2);
        assert_eq!(live.explore(&grid, "exanima01", x, z), 0);
        assert_eq!(live.state.maps.len(), 2);
    }

    #[test]
    fn a_locale_name_is_zero_padded_to_sixteen_bytes() {
        assert_eq!(&locale_name("exanima01"), b"exanima01\0\0\0\0\0\0\0");
        assert_eq!(locale_name("abcdefghijklmnopqrs")[15], b'p');
    }
}
