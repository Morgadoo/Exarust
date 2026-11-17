//! The discovery and experience system in the running story.
//!
//! [`crate::experience`] holds the original's rules as plain functions: the
//! account pool, the player's and the companions' conversions, the explored
//! levels and the technique word. This module runs them: it reads the
//! account-wide experience from the launch's settings store, seeds the pool
//! from the player's carried experience when play starts (the original's
//! level-start hook), converts at each of the flow's per-frame
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
//! the same way. A rise in the armour skill also runs the armour pass again
//! over the worn outfit ([`ArmourRefresh`], [`refresh_armour`]). A
//! companion's rise refreshes its own body the same way (its technique word,
//! and its armour after an armour rise); the two AI factors its rise writes
//! are not applied (their fields in the AI are not named yet).
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

/// On the player's body after a rise in the armour skill: the armour pass
/// runs again over the outfit it wears ([`refresh_armour`]), as the
/// original's conversion calls it on the player's body when the trained
/// skill that rose is the armour skill.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ArmourRefresh;

/// The flow's discovery pass (every story frame) ran at `now`, on the game clock
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
    /// The companions' bodies, in the party's order (the original's
    /// companion list holds the bodies).
    pub party_bodies: Vec<Entity>,
    /// The player's body technique word after its last rise.
    pub technique_word: u32,
    /// The last pass raised the player's armour skill
    /// ([`crate::experience::ARMOUR_SKILL`]): the body's armour is redone.
    pub armour_rose: bool,
    /// The companions whose skill rose in the last pass (their index in
    /// the party) with their new technique word and whether the skill was
    /// the armour skill: the rise refreshes the body (its technique word)
    /// and, for the armour skill, its armour, as the player's does.
    pub companion_rises: Vec<(usize, u32, bool)>,
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
            party_bodies: Vec::new(),
            technique_word: 0,
            armour_rose: false,
            companion_rises: Vec::new(),
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

    /// One discovery pass (the flow runs it every story frame): the account
    /// pool, then the player, then each companion in list order. Returns
    /// whether the body's technique word changed. [`Self::explore`] runs
    /// before it.
    pub fn advance(&mut self, now: u32) -> bool {
        if !self.started {
            return false;
        }
        self.state.convert_global(now);
        let Conversion {
            techniques, raised, ..
        } = self.state.convert_player(now, &mut self.player);
        self.armour_rose = raised == Some(crate::experience::ARMOUR_SKILL);
        self.companion_rises.clear();
        for (index, companion) in self.party.iter_mut().enumerate() {
            let Conversion {
                raised, techniques, ..
            } = convert_companion(now, companion);
            if let (Some(skill), Some(word)) = (raised, techniques) {
                self.companion_rises
                    .push((index, word, skill == crate::experience::ARMOUR_SKILL));
            }
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
        write_learner(&self.player, record);
    }

    /// A companion joins the party (a follower's apply of its roles): its
    /// body is added once, its learner read from its record, and a learner
    /// that never converted starts its clock at `now`. Returns whether it
    /// was added.
    pub fn join(&mut self, body: Entity, record: &CharacterRecord, now: u32) -> bool {
        let mut learner = Learner::from_record(record);
        if !crate::experience::join(&mut self.party_bodies, body, &mut learner, now) {
            return false;
        }
        self.party.push(learner);
        true
    }

    /// A companion leaves the party: the last one takes its place. Returns
    /// whether it was listed.
    pub fn leave(&mut self, body: Entity) -> bool {
        let Some(index) = self.party_bodies.iter().position(|b| *b == body) else {
            return false;
        };
        crate::experience::leave(&mut self.party_bodies, &body);
        self.party.swap_remove(index);
        true
    }

    /// The story's discovery chunk, the format a save carries.
    pub fn discovery(&self) -> Vec<u8> {
        self.state.write_discovery()
    }
}

/// A learner's converted state written to its record: the experience
/// still to convert, the skills and the powers' words (the original
/// converts the record itself).
pub fn write_learner(learner: &Learner, record: &mut CharacterRecord) {
    record.experience = learner.experience as i32;
    record.skills = learner.skills;
    if let Some(powers) = record.powers.as_mut() {
        powers.known.clone_from(&learner.powers);
    }
}

/// The party follows the brains: a follower whose roles were applied joins
/// it, one that stopped following or died leaves it, and a companion whose
/// body is gone leaves it (the original's AI destructor leaves the party
/// when it was following).
fn keep_party(
    ticks: Option<Res<exa_physics::body::BodyTicks>>,
    mut changes: MessageReader<exa_ai::brain::PartyChange>,
    records: Query<&exa_character::record::CharacterData>,
    alive: Query<()>,
    mut live: ResMut<StoryExperience>,
) {
    let now = now_ms(ticks.as_deref());
    for change in changes.read() {
        if change.joins {
            let Ok(data) = records.get(change.body) else {
                continue;
            };
            if live.join(change.body, &data.record, now) {
                info!(
                    "experience: {:?} joins the party at {now} ms ({} companions)",
                    change.body,
                    live.party.len()
                );
            }
        } else if live.leave(change.body) {
            info!(
                "experience: {:?} leaves the party ({} companions)",
                change.body,
                live.party.len()
            );
        }
    }
    let gone: Vec<Entity> = live
        .party_bodies
        .iter()
        .copied()
        .filter(|b| alive.get(*b).is_err())
        .collect();
    for body in gone {
        live.leave(body);
        info!("experience: {body:?} is gone and leaves the party");
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
pub(crate) fn begin_story_experience(
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
pub(crate) fn tick_story_experience(
    mut passes: MessageReader<DiscoveryTick>,
    mut players: Query<
        (
            Entity,
            &GlobalTransform,
            &mut exa_character::record::CharacterData,
        ),
        With<PlayerBody>,
    >,
    mut companions: Query<&mut exa_character::record::CharacterData, Without<PlayerBody>>,
    (level, source): (
        Option<Res<exa_world::room_view::LevelRooms>>,
        Option<Res<exa_world::LevelSource>>,
    ),
    mut commands: Commands,
    mut live: ResMut<StoryExperience>,
    mut last_cell: Local<Option<(i64, i64)>>,
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
            let cell = level
                .rooms
                .cell_of(bevy::math::Vec2::new(p.x / 100.0, p.z / 100.0));
            if *last_cell != Some(cell) {
                *last_cell = Some(cell);
                let explored: u32 = live
                    .state
                    .current
                    .and_then(|i| live.state.maps.get(i))
                    .map_or(0, |map| map.bits.iter().map(|w| w.count_ones()).sum());
                debug!(
                    "experience: the player enters cell {cell:?} of {} at {} ms ({explored} cells explored)",
                    source.locale(),
                    pass.now
                );
            }
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
        for (body, learner) in live.party_bodies.iter().zip(&live.party) {
            if let Ok(mut data) = companions.get_mut(*body) {
                write_learner(learner, &mut data.record);
            }
        }
        // A companion's rise refreshes its body: the technique word, and
        // the armour after a rise in the armour skill.
        for &(index, word, armour) in &live.companion_rises {
            let Some(&body) = live.party_bodies.get(index) else {
                continue;
            };
            info!(
                "experience: companion {body:?} rose at {} ms: technique word {word:#010x}{}",
                pass.now,
                if armour { ", armour redone" } else { "" }
            );
            let mut entity = commands.entity(body);
            entity.insert(TechniqueWord(word));
            if armour {
                entity.insert(ArmourRefresh);
            }
        }
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
            if live.armour_rose {
                info!(
                    "experience: the armour skill rose at {} ms: the armour pass runs again",
                    pass.now
                );
                commands.entity(player).insert(ArmourRefresh);
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

/// The bodies whose armour skill rose, with what the armour pass reads.
type ArmourReaders<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static exa_character::record::CharacterData,
        &'static mut exa_combat::Fighter,
        Option<&'static exa_character::WornArmour>,
    ),
    With<ArmourRefresh>,
>;

/// The armour refresh after a rise in the armour skill: the fighter's
/// Deflection is read again from its record (a rise can teach it) and the
/// armour pass runs over the worn items' protection
/// ([`exa_combat::Fighter::wear`]). The fighter is the simulation's: the
/// write lands on its copy at the next step.
pub fn refresh_armour(
    mut bodies: ArmourReaders,
    mut fixed: Option<ResMut<exa_physics::sim_app::FixedCommands>>,
    mut commands: Commands,
) {
    for (body, data, mut fighter, worn) in &mut bodies {
        let deflection = data.record.deflection();
        let worn = worn.map(|w| w.0.clone()).unwrap_or_default();
        let refresh = move |f: &mut exa_combat::Fighter| {
            f.stats.deflection = deflection;
            f.wear(&worn);
        };
        match fixed.as_deref_mut() {
            Some(fixed) => fixed.write(body, &mut fighter, refresh),
            None => refresh(&mut fighter),
        }
        commands.entity(body).remove::<ArmourRefresh>();
    }
}

/// Play ended: the account experience goes to the settings store when it
/// changed.
pub(crate) fn store_account_experience(
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
            .add_message::<exa_ai::brain::PartyChange>()
            .add_systems(
                Update,
                (
                    load_account_experience,
                    keep_party.after(crate::save_live::sync_before_level_change),
                    begin_story_experience,
                    tick_story_experience.after(crate::golem_alarm::return_from_alarm),
                    apply_technique_word
                        .after(crate::flow::apply_steps)
                        .after(crate::thaum::cast::broadcast_level_change)
                        .after(crate::save_live::save_to_new_slot),
                    // After the flow's level-state reset, which the
                    // player's placement (also a simulation write) runs
                    // before.
                    refresh_armour.after(crate::flow::reset_level_state),
                    store_account_experience,
                )
                    .chain()
                    .in_set(exa_physics::FramePhase::Session),
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

    /// A brain that follows (a Loyalty marker over its roles) joins the
    /// party through the live apply of its roles, with its record's learner
    /// and its clock started; a second apply does not add it again; its
    /// death leaves the party, and a companion whose body is gone leaves.
    #[test]
    fn a_follower_joins_the_party_and_its_death_leaves_it() {
        let mut app = App::new();
        app.add_plugins(ExperienceLivePlugin)
            .add_message::<SettingsEntry>()
            .insert_resource(exa_physics::body::BodyTicks(100))
            .insert_resource(exa_ai::brain::RoleDatabase {
                book: exa_formats::roles::RoleDb::default(),
                locale: 0,
                registry: exa_ai::activity::IdleRegistry::shipped(),
            })
            .add_systems(
                Update,
                (exa_ai::brain::apply_new_roles, exa_ai::brain::death_hook).before(keep_party),
            );
        let marker = exa_ai::combatant::MindPowers {
            effects: vec![exa_ai::combatant::Effect {
                kind: 0x10_010A,
                source: None,
                target: None,
                state: 1,
            }],
            resists_control: false,
            mask: None,
        };
        let body = |app: &mut App| {
            app.world_mut()
                .spawn((
                    exa_ai::roles::Roles::default(),
                    exa_ai::brain::Npc::new([0.0; 4]),
                    marker.clone(),
                    exa_combat::Condition::NORMAL,
                    exa_character::record::CharacterData::of_record(record(40)),
                ))
                .id()
        };
        let follower = body(&mut app);
        app.update();
        let live = app.world().resource::<StoryExperience>();
        assert_eq!(live.party_bodies, vec![follower]);
        assert_eq!(live.party.len(), 1);
        assert_eq!(live.party[0].experience, 40);
        let clock = live.party[0].clock;
        assert_eq!(clock, exa_physics::body::BodyTicks(100).millis() as u32);
        app.world_mut()
            .get_mut::<exa_ai::brain::Npc>(follower)
            .unwrap()
            .reapply = true;
        app.update();
        assert_eq!(app.world().resource::<StoryExperience>().party.len(), 1);
        app.world_mut()
            .entity_mut(follower)
            .insert(exa_combat::Condition::DEAD);
        app.update();
        assert!(app.world().resource::<StoryExperience>().party.is_empty());
        let other = body(&mut app);
        app.update();
        assert_eq!(
            app.world().resource::<StoryExperience>().party_bodies,
            vec![other]
        );
        app.world_mut().despawn(other);
        app.update();
        assert!(
            app.world()
                .resource::<StoryExperience>()
                .party_bodies
                .is_empty()
        );
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

    /// A pass that raises the player's armour skill to the level whose
    /// chosen technique is Deflection runs the armour pass again over the
    /// worn items: the fighter's layers become the ones Deflection gives.
    /// A rise in another skill leaves the armour alone.
    #[test]
    fn an_armour_skill_rise_redoes_the_players_armour() {
        let worn = exa_formats::objdb::Armour {
            coverage: [50; 18],
            padding: 60,
            slash: 140,
            crush: 60,
            pierce: 80,
            material: 4,
            ..Default::default()
        };
        let run = |skill: usize| {
            let mut app = App::new();
            app.add_plugins(MinimalPlugins)
                .add_plugins(ExperienceLivePlugin)
                .add_message::<SettingsEntry>();
            let mut record = record(2_000);
            // A point short of the first level, Deflection chosen there.
            record.skills[skill].points = 4_999;
            record.skills[skill].techniques[0] = 4;
            let mut fighter = exa_combat::Fighter::default();
            fighter.wear(std::slice::from_ref(&worn));
            let before = fighter.armour;
            let body = app
                .world_mut()
                .spawn((
                    PlayerBody::new(0.0),
                    GlobalTransform::default(),
                    exa_character::record::CharacterData::of_record(record.clone()),
                    fighter,
                    exa_character::WornArmour(vec![worn]),
                ))
                .id();
            app.world_mut()
                .resource_mut::<StoryExperience>()
                .begin(&record, 0);
            app.world_mut().write_message(DiscoveryTick { now: 1000 });
            app.update();
            app.update();
            let data = app
                .world()
                .get::<exa_character::record::CharacterData>(body)
                .unwrap();
            let fighter = app.world().get::<exa_combat::Fighter>(body).unwrap();
            assert!(app.world().get::<ArmourRefresh>(body).is_none());
            (data.record.deflection(), fighter.clone(), before)
        };
        let (deflection, fighter, before) = run(crate::experience::ARMOUR_SKILL);
        assert!(deflection);
        assert!(fighter.stats.deflection);
        let expected = exa_combat::damage::armour_layers(None, &[worn], true);
        assert_eq!(fighter.armour, expected);
        assert_ne!(fighter.armour, before);
        let (_, fighter, before) = run(0);
        assert!(!fighter.stats.deflection);
        assert_eq!(fighter.armour, before);
    }

    /// A companion's rise in the armour skill refreshes its body as the
    /// player's does: the technique word and the armour pass.
    #[test]
    fn a_companions_armour_rise_refreshes_its_body() {
        let worn = exa_formats::objdb::Armour {
            coverage: [50; 18],
            slash: 140,
            ..Default::default()
        };
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_plugins(ExperienceLivePlugin)
            .add_message::<SettingsEntry>()
            .add_systems(Update, apply_technique_word.after(refresh_armour));
        let mut companion = record(2_000);
        companion.skills[crate::experience::ARMOUR_SKILL].points = 4_999;
        companion.skills[crate::experience::ARMOUR_SKILL].techniques[0] = 4;
        let mut fighter = exa_combat::Fighter::default();
        fighter.wear(std::slice::from_ref(&worn));
        let body = app
            .world_mut()
            .spawn((
                exa_character::record::CharacterData::of_record(companion.clone()),
                fighter,
                exa_character::WornArmour(vec![worn]),
            ))
            .id();
        {
            let mut live = app.world_mut().resource_mut::<StoryExperience>();
            live.begin(&record(0), 0);
            assert!(live.join(body, &companion, 0));
        }
        app.world_mut().write_message(DiscoveryTick { now: 1000 });
        app.update();
        app.update();
        let word = app.world().get::<TechniqueWord>(body).copied();
        assert_eq!(
            word.map(|w| w.0),
            Some(technique_bits(
                &app.world()
                    .get::<exa_character::record::CharacterData>(body)
                    .unwrap()
                    .record
                    .skills
            ))
        );
        let fighter = app.world().get::<exa_combat::Fighter>(body).unwrap();
        assert!(fighter.stats.deflection);
        assert_eq!(
            fighter.armour,
            exa_combat::damage::armour_layers(None, &[worn], true)
        );
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
