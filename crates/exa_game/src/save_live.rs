//! The save unit in the running game: a save to a new slot and an arena
//! slot's load at the hub, over the slot manager's cached list.
//!
//! Both are idle unless the host gives a save folder ([`SaveFolder`]). A
//! [`SaveToNewSlot`] message (or a frame from [`ScriptedSaves`]) creates a
//! slot the way the original's new-slot entry does ([`SlotList::create`])
//! and writes the session into it: the story's level and world clock, or
//! the arena's company, staff, arsenal and stash as the arena tables. A
//! number whose save file already holds bytes is refused, so a folder of
//! real saves is never written over [Exarust's guard: the original's new
//! slot takes the lowest number its list does not hold and would reuse a
//! file it left out of the list].
//!
//! An [`ArenaSlotLoad`] makes the hub, once entered, load that arena slot
//! as the original's full load does ([`restore_arena`]): the slot becomes
//! current and moves to the front, the world object and character
//! databases are read from the save, the arena reader's tables go into the
//! live company, staff, arsenal and stash, the reader's closing list
//! regeneration runs with the object records' classification, the world
//! clock takes the saved seconds, and the player record's race byte is
//! cleared before the import.
//!
//! A story save writes the player's record into the save's world character
//! database (under [`PLAYER_RECORD_ID`]) with the player's placement in the
//! player data, so the slot list names the player and a load restores the
//! character. A [`StorySlotLoad`] (the saves' character screen's CONTINUE,
//! [`crate::save_screen`]) runs the story's full load: the slot becomes
//! current and moves to the front, the player's record (with the load's
//! reset) becomes the character the next player spawn is made from, dressed
//! from its own worn and held items ([`SavedPlayer`]), the world clock takes
//! the saved seconds, and the story continues at the save's level at the
//! saved placement. The flow's save steps ([`SaveStep`]) write, drop and
//! checkpoint the current story slot.
//!
//! Not written yet: the save's level section (Exarust's loaded world has no
//! converter back into the exporter's scene, and a load takes the level
//! from the archive), the world object database and every character but
//! the player (the running game keeps no live object or character
//! database).
//!
//! Evidence: the save unit's new-slot, save, full load, arena reader and
//! list regeneration routines (`docs/formats/SAVES.md`).

use std::collections::HashMap;
use std::path::PathBuf;

use bevy::prelude::*;
use exa_formats::arena_save::{self, ArenaTables, ItemEntry};
use exa_formats::characters::CharacterRecord;
use exa_formats::save_package::{self, PlayerHeader};

use exa_formats::characters::{Carried, ItemRef};

use crate::arena_ids::{StaffLists, kinds_from_items, regenerate_lists};
use crate::arsenal::{Arsenal, ItemKind};
use crate::session::{Mode, Session};
use crate::slots::{LoadedSave, SaveWorld, SlotList, Slots};
use crate::staff::Staff;

/// The folder the running game saves into and loads from. Inserted by the
/// host (`--save-dir`); without it nothing here runs.
#[derive(Resource, Clone, Debug, PartialEq, Eq)]
pub struct SaveFolder(pub PathBuf);

/// The view's camera target and zoom a save stamps into its header. The
/// original's writers copy the scene view's target (centimetres) and its
/// zoom word into both headers; both readers read the pair into scratch and
/// drop it, so a load never moves the view. The host may set this from its
/// camera; nothing in the running game does yet (Exarust's chase camera
/// has no traced mapping onto the original's view fields), so a save writes
/// the header's defaults (the origin, zoom 1).
#[derive(Resource, Clone, Copy, Debug, PartialEq)]
pub struct SaveCamera {
    /// The camera's target, centimetres.
    pub target_cm: [f32; 3],
    /// The camera's zoom word.
    pub zoom: f32,
}

impl Default for SaveCamera {
    fn default() -> Self {
        Self {
            target_cm: [0.0; 3],
            zoom: 1.0,
        }
    }
}

impl SaveCamera {
    /// Stamps the pair into a session's header.
    pub fn stamp(&self, session: &mut Session) {
        session.camera = self.target_cm;
        session.zoom = self.zoom;
    }
}

/// The two slot managers' cached lists over the save folder.
#[derive(Resource, Clone, Debug, PartialEq, Eq)]
pub struct SlotLists {
    pub story: SlotList,
    pub arena: SlotList,
}

impl SlotLists {
    /// Unscanned managers over `dir`, as the game makes them at start-up.
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        let dir = dir.into();
        Self {
            story: SlotList::new(Slots::story(&dir)),
            arena: SlotList::new(Slots::arena(&dir)),
        }
    }

    /// The manager of a mode.
    pub fn of_mode(&mut self, mode: Mode) -> &mut SlotList {
        match mode {
            Mode::Story => &mut self.story,
            Mode::Arena => &mut self.arena,
        }
    }
}

/// The arena's live stores that the save's tables hold beside the career:
/// the staff, the arsenal's shelves and the hub stash's ids.
#[derive(Resource, Clone, Debug, Default, PartialEq, Eq)]
pub struct ArenaStores {
    pub staff: Staff,
    pub arsenal: Arsenal,
    pub stash: Vec<u32>,
}

impl ArenaStores {
    /// The single reset's share of the stores: the hub stash, every
    /// arsenal shelf and box, and the staff list with its manage order
    /// ([`crate::hub_windows::ArenaCareer::reset`]).
    pub fn reset(&mut self) {
        self.stash.clear();
        self.arsenal.clear();
        self.staff.employees.clear();
        self.staff.order.clear();
    }
}

/// Asks for a save of the running session into a new slot.
#[derive(Message, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SaveToNewSlot;

/// Frames after the level is up (loading frames do not count) at which a
/// save to a new slot is asked for (`--save-at`).
#[derive(Resource, Clone, Debug, Default, PartialEq, Eq)]
pub struct ScriptedSaves(pub Vec<u32>);

/// An arena slot the hub loads once it is entered (`--arena-slot`).
#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
pub struct ArenaSlotLoad {
    pub number: u32,
}

/// A story slot to load into the running game, as the saves' character
/// screen's CONTINUE asks for it: its number and whether the checkpoint is
/// read first (the screen's kind 1) or the save (kind 2).
#[derive(Message, Clone, Copy, Debug, PartialEq, Eq)]
pub struct StorySlotLoad {
    pub number: u32,
    pub prefer_checkpoint: bool,
}

/// The flow's save work on the current story slot: a save (with the death
/// rule or without), the death's drop of the save file, and the copy of the
/// save over the checkpoint.
#[derive(Message, Clone, Copy, Debug, PartialEq, Eq)]
pub enum SaveStep {
    Save { drop_on_low_health: bool },
    Drop,
    Checkpoint,
}

/// The engine clock (game milliseconds) the level-6 exit's checkpoint
/// stamped (`Step::LevelTime`): the original's level exit stores the clock
/// after its checkpoint callback succeeds. Which save field reads it back
/// is not traced; it is kept here for that reader.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LevelTime(pub u32);

/// The player record a story load restored: the next player spawn takes
/// its body, skin, hair and name (through
/// [`crate::creator_screen::CreatedCharacter`]) and is dressed from its own
/// worn and held items, with its stats derived from it.
#[derive(Resource, Clone, Debug, PartialEq)]
pub struct SavedPlayer {
    pub record: CharacterRecord,
    /// The save's world object records, for items the shipped database
    /// does not hold.
    pub items: Vec<exa_formats::objdb::Item>,
}

/// The id the player's record takes in a story save's world character
/// database [Exarust's choice: the original saves the record under the id
/// its world database gave it].
pub const PLAYER_RECORD_ID: u32 = 1;

/// What the last live save or load did, for the report and the tests.
#[derive(Resource, Clone, Debug, Default, PartialEq, Eq)]
pub struct SaveReport {
    /// The slot the last save wrote.
    pub saved: Option<u32>,
    /// The arena slot the hub loaded and how many items the regeneration
    /// shelved.
    pub loaded: Option<(u32, usize)>,
    /// The story slot the last story load read, and from which file.
    pub story_loaded: Option<(u32, save_package::SlotKind)>,
}

/// The running game's save and load.
pub struct SaveLivePlugin;

impl Plugin for SaveLivePlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<SaveToNewSlot>()
            .add_message::<StorySlotLoad>()
            .add_message::<SaveStep>()
            .add_message::<crate::world_clock::RestoreWorldClock>()
            .init_resource::<ArenaStores>()
            .init_resource::<SaveReport>()
            .add_systems(
                Update,
                (
                    make_slot_lists,
                    report_saves,
                    scripted_saves,
                    save_to_new_slot,
                    run_save_steps,
                    load_arena_slot,
                    load_story_slot,
                )
                    .chain()
                    .run_if(resource_exists::<SaveFolder>),
            );
    }
}

/// The managers are made once the folder is known; they scan lazily.
fn make_slot_lists(mut commands: Commands, folder: Res<SaveFolder>, lists: Option<Res<SlotLists>>) {
    if lists.is_none() {
        commands.insert_resource(SlotLists::new(&folder.0));
    }
}

/// The menu's CONTINUE rows read whether each manager holds a slot: the
/// flow's two save words follow the managers' cached lists (the first
/// question scans the folder), and an open menu's page state with them.
fn report_saves(
    lists: Option<ResMut<SlotLists>>,
    flow: Option<ResMut<crate::flow::Flow>>,
    menu: Option<ResMut<crate::menu_screen::MenuScreen>>,
) {
    let (Some(mut lists), Some(mut flow)) = (lists, flow) else {
        return;
    };
    let has = [lists.story.any(), lists.arena.any()];
    if flow.has_save != has {
        flow.has_save = has;
        if let Some(mut menu) = menu {
            menu.state = flow.page_state();
        }
    }
}

fn scripted_saves(
    scripted: Option<Res<ScriptedSaves>>,
    loading: Option<Res<exa_ui::LoadingStatus>>,
    levels: Query<(), With<exa_world::LevelRoot>>,
    mut frames: Local<u32>,
    mut saves: MessageWriter<SaveToNewSlot>,
) {
    let Some(scripted) = scripted else {
        return;
    };
    if loading.is_some() || levels.is_empty() {
        return;
    }
    *frames += 1;
    if scripted.0.contains(&*frames) {
        info!("save: scripted save at frame {}", *frames);
        saves.write(SaveToNewSlot);
    }
}

/// The story's position for a loaded level: the campaign index naming it,
/// else the story's start [guess: a level outside the campaign, as a
/// directly launched arena or test level, saves as the first level].
pub fn story_at(level: &str) -> exa_script::story::Story {
    let wanted = level.strip_suffix(".rfc").unwrap_or(level);
    let mut story = exa_script::story::Story::begin();
    loop {
        if story.level().eq_ignore_ascii_case(wanted) {
            return story;
        }
        let index = story.index();
        story.advance();
        if story.index() == index {
            return exa_script::story::Story::begin();
        }
    }
}

/// The session the running game saves: the arena's company and tables when
/// the hub is loaded, else the story at the loaded level; the world clock's
/// seconds at this tick go into the header.
pub fn live_session(
    level: Option<&str>,
    clock_seconds: u32,
    career: &crate::hub_windows::ArenaCareer,
    stores: &ArenaStores,
) -> Session {
    let arena = level.is_some_and(crate::arena_hub_live::is_hub_level);
    let mut session = if arena {
        let mut session = Session::career(career.company.name.clone());
        session.company = career.company.clone();
        session.capture_arena(career, &stores.staff, &stores.arsenal, &stores.stash);
        session
    } else {
        let mut session = Session::story();
        session.story = story_at(level.unwrap_or(""));
        session
    };
    session.clock = clock_seconds;
    session
}

/// Saves a session into a new slot of its mode's manager. Returns the
/// number, or `None` when the next number's save file already holds bytes
/// (nothing is created then).
pub fn save_new_slot(
    list: &mut SlotList,
    session: &Session,
    player_name: Option<&str>,
    world: &SaveWorld<'_>,
) -> std::io::Result<Option<u32>> {
    let next = list.next_free_number()?;
    let file = list.slots().file(next, save_package::SlotKind::Save);
    if std::fs::metadata(&file).is_ok_and(|meta| meta.len() > 0) {
        return Ok(None);
    }
    let number = list.create(player_name)?;
    list.save_current(session, world)?;
    Ok(Some(number))
}

/// The story player as a save writes it: its record and where it stands.
#[derive(Clone, Debug, PartialEq)]
pub struct StoryPlayer {
    pub record: CharacterRecord,
    /// The body's position, centimetres.
    pub position_cm: [f32; 3],
    /// The body's heading, radians (Exarust's yaw).
    pub heading: f32,
}

/// A record with a body's live possessions written back: the four hands
/// (the out set first), the worn list and the carried list with each
/// entry's pack cell.
pub fn record_with_possessions(
    mut record: CharacterRecord,
    possessions: &exa_items::possession::Possessions,
) -> CharacterRecord {
    let p = possessions;
    record.hands = [p.out[0], p.out[1], p.stowed[0], p.stowed[1]]
        .map(|hand| ItemRef::new(hand.id, hand.flags));
    record.worn = p.worn.clone();
    record.inventory = p
        .carried
        .iter()
        .map(|c| Carried {
            item: c.id,
            flags: c.flags,
            extra: u32::from(c.x as u16) | (u32::from(c.y as u16) << 16),
        })
        .collect();
    record
}

/// A body's health as the story save reads it: the fighter's yellow, red,
/// maximum and heal pool, the caster's focus, the potion's expiry while the
/// potion flag is set, and the condition word.
pub fn body_health(
    fighter: &exa_combat::Fighter,
    flags: exa_combat::StateFlags,
    condition: exa_combat::Condition,
    focus: f32,
) -> exa_character::record::Health {
    exa_character::record::Health {
        yellow: fighter.health.yellow,
        red: fighter.health.red,
        max: fighter.health.max,
        focus,
        heal_pool: fighter.health.heal_pool,
        potion: flags
            .has(exa_combat::StateFlags::POTION)
            .then_some(fighter.potion_until),
        condition: condition.0,
    }
}

/// A record with a body's health stored into it, as the story save stores
/// every scene character's before it writes ([`CharacterData::save_health`]:
/// the three pools as shares, the seed, the heal-pool and potion entries).
///
/// [`CharacterData::save_health`]: exa_character::record::CharacterData::save_health
pub fn record_with_health(
    record: CharacterRecord,
    health: exa_character::record::Health,
) -> CharacterRecord {
    let mut data = exa_character::record::CharacterData::of_record(record);
    data.save_health(health);
    data.into_record()
}

/// A spawned body's health loaded from its record, as the player spawn's
/// last write does ([`exa_character::record::restore_record_health`]): red
/// and yellow as the record's shares of the fighter's maximum, the focus,
/// and the saved heal pool and potion (the potion sets the potion flag and
/// its expiry). What the record does not name keeps the body's value.
pub fn restore_body_health(
    record: &CharacterRecord,
    fighter: &mut exa_combat::Fighter,
    flags: &mut exa_combat::StateFlags,
    focus: &mut f32,
) {
    let mut health = body_health(fighter, *flags, exa_combat::Condition::NORMAL, *focus);
    exa_character::record::restore_record_health(record, &mut health);
    fighter.health.yellow = health.yellow;
    fighter.health.red = health.red;
    fighter.health.heal_pool = health.heal_pool;
    if let Some(expiry) = health.potion {
        flags.0 |= exa_combat::StateFlags::POTION;
        fighter.potion_until = expiry;
    }
    *focus = health.focus;
}

/// The story player's share of a save: its record goes into a world
/// character database under [`PLAYER_RECORD_ID`], which is exported as the
/// save's `#worldchrdb`, and the session's placement becomes the player's
/// (the record reference with the world bit, the position with its height
/// as the floor, the heading, a living condition and an AI that saves no
/// home or pose [guess: the player's controller writes neither]).
pub fn story_player_world(
    player: &StoryPlayer,
    session: &mut Session,
) -> exa_formats::Result<crate::slots::WorldExport> {
    let mut db = exa_formats::character_db::CharacterDb::new(exa_formats::character_db::WORLD_TAG);
    let mut record = player.record.clone();
    // The save's reader finds the player by the low id the placement names.
    record.id = PLAYER_RECORD_ID;
    let _ = db.add(PLAYER_RECORD_ID, record);
    let export =
        crate::slots::export_world_databases(&mut db, &exa_formats::objdb::ObjectDb::default())?;
    let body = exa_formats::rfc_write::ExportCharacter {
        record: exa_formats::characters::WORLD_DB | PLAYER_RECORD_ID,
        body_position: player.position_cm,
        floor_height: player.position_cm[1],
        heading: player.heading,
        condition: 0,
        ai: Some(exa_formats::rfc_write::ExportAi::default()),
        ..Default::default()
    };
    let mut saved = None;
    let placement =
        crate::session::export_player_placement(&body, &mut saved, session.transition_word())?;
    session.placement = Some(placement.payload);
    Ok(export)
}

/// The placement a story load puts the player at: the saved position (cm)
/// and heading turned into the flow's spawn frame.
pub fn placement_of(placement: &save_package::CharacterPlacement) -> crate::flow::Placement {
    let (sin, cos) = placement.heading.sin_cos();
    crate::flow::Placement {
        rotation: [[cos, 0.0, -sin], [0.0, 1.0, 0.0], [sin, 0.0, cos]],
        position: Vec3::from(placement.position),
    }
}

/// The live story player for a save: the player body's own record when it
/// has one (a loaded save's player), else the created character's, else
/// the creator's defaults; its possessions written back; its place and
/// heading.
type PlayerSaveQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static GlobalTransform,
        Option<&'static exa_character::record::CharacterData>,
        Option<&'static exa_items::possession::Possessions>,
        PlayerHealthParts,
    ),
    With<crate::player::PlayerBody>,
>;

/// What the save reads a player body's health from.
type PlayerHealthParts = (
    Option<&'static exa_combat::Fighter>,
    Option<&'static exa_combat::StateFlags>,
    Option<&'static exa_combat::Condition>,
    Option<&'static crate::thaum::cast::PowerCaster>,
);

/// The focus a record holds, for a body without a caster.
fn record_focus(record: &CharacterRecord) -> f32 {
    exa_formats::characters::Pools::restore_focus(record.pools.focus)
}

fn live_story_player(
    players: &PlayerSaveQuery,
    created: Option<&crate::creator_screen::CreatedCharacter>,
) -> Option<StoryPlayer> {
    let (transform, data, possessions, (fighter, flags, condition, caster)) =
        players.iter().next()?;
    let mut record = data
        .map(|data| data.record.clone())
        .or_else(|| created.and_then(|c| c.0.clone()))
        .unwrap_or_else(CharacterRecord::reset);
    if let Some(possessions) = possessions {
        record = record_with_possessions(record, possessions);
    }
    // The save stores the body's health into its record first (the
    // original does it for every scene character; only the player's record
    // is saved here).
    if let Some(fighter) = fighter {
        let focus = caster.map_or(record_focus(&record), |caster| caster.body.focus);
        let health = body_health(
            fighter,
            flags.copied().unwrap_or_default(),
            condition.copied().unwrap_or_default(),
            focus,
        );
        record = record_with_health(record, health);
        info!(
            "save: the player's health stored: yellow {:.5} red {:.5} of {:.5}, focus {:.5}, heal pool {:.5}, potion {:?} (pools {}/{}/{}, {} saved entries)",
            health.yellow,
            health.red,
            health.max,
            health.focus,
            health.heal_pool,
            health.potion,
            record.pools.yellow,
            record.pools.red,
            record.pools.focus,
            record.saved.len()
        );
    }
    let (yaw, _, _) = transform.rotation().to_euler(EulerRot::YXZ);
    let at = transform.translation() * 100.0;
    Some(StoryPlayer {
        record,
        position_cm: at.to_array(),
        heading: yaw,
    })
}

/// The live session with the story player's record and placement: the
/// sections it writes besides the player data.
fn story_save_world(
    session: &mut Session,
    player: Option<&StoryPlayer>,
) -> crate::slots::WorldExport {
    let Some(player) = player else {
        return crate::slots::WorldExport::default();
    };
    match story_player_world(player, session) {
        Ok(export) => export,
        Err(error) => {
            warn!("save: the player's record does not export: {error}");
            crate::slots::WorldExport::default()
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn save_to_new_slot(
    mut requests: MessageReader<SaveToNewSlot>,
    lists: Option<ResMut<SlotLists>>,
    levels: Query<&exa_world::LevelRoot>,
    clock: Option<Res<exa_combat::remedy::WorldClock>>,
    ticks: Option<Res<exa_physics::body::BodyTicks>>,
    career: Option<Res<crate::hub_windows::ArenaCareer>>,
    stores: Res<ArenaStores>,
    camera: Option<Res<SaveCamera>>,
    transition: Option<Res<crate::level_transition::LiveTransition>>,
    players: PlayerSaveQuery,
    created: Option<Res<crate::creator_screen::CreatedCharacter>>,
    mut report: ResMut<SaveReport>,
) {
    if requests.read().count() == 0 {
        return;
    }
    // The save writer's first check: a level change whose archive copy did
    // not import leaves saving disabled until a level loads properly, and
    // nothing is written.
    if transition.as_ref().is_some_and(|t| t.saves_disabled()) {
        warn!("save: saving is disabled after the last level change; nothing written");
        return;
    }
    let Some(mut lists) = lists else {
        return;
    };
    let level = levels.iter().next().map(|root| root.name.as_str());
    let now = ticks.as_ref().map_or(0, |ticks| ticks.millis() as u32);
    let seconds = clock
        .as_ref()
        .map_or(0, |clock| crate::world_clock::stamped(clock, now));
    let fallback = crate::hub_windows::ArenaCareer::default();
    let career = career.as_deref().unwrap_or(&fallback);
    let mut session = live_session(level, seconds, career, &stores);
    camera
        .as_deref()
        .copied()
        .unwrap_or_default()
        .stamp(&mut session);
    let (name, export) = match session.mode {
        Mode::Arena => (
            Some(career.player.name.clone()),
            crate::slots::WorldExport::default(),
        ),
        Mode::Story => {
            let player = live_story_player(&players, created.as_deref());
            let export = story_save_world(&mut session, player.as_ref());
            (player.map(|p| p.record.name), export)
        }
    };
    let world = export.save_world(None, false);
    match save_new_slot(
        lists.of_mode(session.mode),
        &session,
        name.as_deref(),
        &world,
    ) {
        Ok(Some(number)) => {
            info!(
                "save: {:?} session saved to new slot {number} (level {level:?}, clock {seconds} s, player {name:?})",
                session.mode
            );
            report.saved = Some(number);
        }
        Ok(None) => warn!("save: the next slot's file already holds a save; nothing written"),
        Err(error) => warn!("save: cannot write a new slot: {error}"),
    }
}

/// The flow's save steps on the current story slot. A save writes the
/// live story session with the player's record and placement (the death
/// rule first, when asked: a dead player's save is dropped instead); a drop
/// deletes the save file, and without a checkpoint the slot leaves the
/// list; a checkpoint copies the save over it. Nothing runs without a
/// current story slot, or outside the story.
#[allow(clippy::too_many_arguments)]
fn run_save_steps(
    mut steps: MessageReader<SaveStep>,
    lists: Option<ResMut<SlotLists>>,
    flow: Option<Res<crate::flow::Flow>>,
    levels: Query<&exa_world::LevelRoot>,
    clock: Option<Res<exa_combat::remedy::WorldClock>>,
    ticks: Option<Res<exa_physics::body::BodyTicks>>,
    camera: Option<Res<SaveCamera>>,
    transition: Option<Res<crate::level_transition::LiveTransition>>,
    players: PlayerSaveQuery,
    fighters: Query<&exa_combat::Fighter, With<crate::player::PlayerBody>>,
    created: Option<Res<crate::creator_screen::CreatedCharacter>>,
    mut report: ResMut<SaveReport>,
) {
    let steps: Vec<SaveStep> = steps.read().copied().collect();
    if steps.is_empty() {
        return;
    }
    let Some(mut lists) = lists else {
        return;
    };
    if flow
        .as_ref()
        .is_some_and(|flow| flow.mode != crate::flow::Mode::Story)
    {
        return;
    }
    let list = &mut lists.story;
    if list.current() <= 0 {
        debug!("save: no current story slot for {steps:?}");
        return;
    }
    let number = list.current() as u32;
    let disabled = transition.as_ref().is_some_and(|t| t.saves_disabled());
    let health = fighters
        .iter()
        .next()
        .map_or(1.0, |fighter| fighter.health.red);
    for step in steps {
        match step {
            SaveStep::Save { drop_on_low_health } => {
                if disabled {
                    warn!("save: saving is disabled after the last level change; nothing written");
                    continue;
                }
                if drop_on_low_health && !exa_formats::save_chunk::save_allowed(health, false) {
                    match list.drop_after_death(health) {
                        Ok(dropped) => info!(
                            "save: the dead player's save {number} is dropped (slot left the list: {dropped})"
                        ),
                        Err(error) => warn!("save: cannot drop slot {number}: {error}"),
                    }
                    continue;
                }
                let level = levels.iter().next().map(|root| root.name.as_str());
                let now = ticks.as_ref().map_or(0, |ticks| ticks.millis() as u32);
                let seconds = clock
                    .as_ref()
                    .map_or(0, |clock| crate::world_clock::stamped(clock, now));
                let mut session = Session::story();
                session.story = story_at(level.unwrap_or(""));
                session.clock = seconds;
                camera
                    .as_deref()
                    .copied()
                    .unwrap_or_default()
                    .stamp(&mut session);
                let player = live_story_player(&players, created.as_deref());
                let export = story_save_world(&mut session, player.as_ref());
                match list.save_current(&session, &export.save_world(None, false)) {
                    Ok(_) => {
                        info!(
                            "save: story slot {number} saved (level {level:?}, clock {seconds} s)"
                        );
                        report.saved = Some(number);
                    }
                    Err(error) => warn!("save: cannot write story slot {number}: {error}"),
                }
            }
            SaveStep::Drop => match list.drop_after_death(0.0) {
                Ok(dropped) => info!(
                    "save: story slot {number}'s save is dropped (slot left the list: {dropped})"
                ),
                Err(error) => warn!("save: cannot drop slot {number}: {error}"),
            },
            SaveStep::Checkpoint => {
                // The original's checkpoint routine saves first (with the
                // writer's disable check) and only then copies the save over
                // the checkpoint; an existing checkpoint is left alone.
                let level = levels.iter().next().map(|root| root.name.as_str());
                let now = ticks.as_ref().map_or(0, |ticks| ticks.millis() as u32);
                let seconds = clock
                    .as_ref()
                    .map_or(0, |clock| crate::world_clock::stamped(clock, now));
                let mut session = Session::story();
                session.story = story_at(level.unwrap_or(""));
                session.clock = seconds;
                camera
                    .as_deref()
                    .copied()
                    .unwrap_or_default()
                    .stamp(&mut session);
                let player = live_story_player(&players, created.as_deref());
                let export = story_save_world(&mut session, player.as_ref());
                // The disable flag skips the save but not the copy: the
                // old save becomes the checkpoint, as in the original.
                let world = export.save_world(None, disabled);
                match list.slots().checkpoint_full(number, &session, &world) {
                    Ok(true) => info!("save: story slot {number}'s save kept as its checkpoint"),
                    Ok(false) => {
                        info!("save: story slot {number} already has a checkpoint; it stays")
                    }
                    Err(error) => warn!("save: cannot write slot {number}'s checkpoint: {error}"),
                }
            }
        }
    }
}

/// What restoring an arena save did.
#[derive(Clone, Debug, PartialEq)]
pub struct ArenaRestore {
    /// The save's tables went into the live state.
    pub applied: bool,
    /// Items the list regeneration put on the arsenal's shelves.
    pub shelved: usize,
    /// The world clock's saved seconds.
    pub clock: u32,
    /// The player's record with the load's reset applied (race byte
    /// cleared), ready for the import.
    pub player: Option<CharacterRecord>,
}

/// The classification of the save's items: the shipped object database's
/// records and the save's world tier, by id.
pub fn save_item_kinds(
    base: &[exa_formats::objdb::Item],
    loaded: &LoadedSave,
) -> HashMap<u32, ItemKind> {
    let mut kinds = kinds_from_items(base);
    if let Some(world) = loaded.world_objects() {
        let items: Vec<_> = world
            .world
            .records()
            .filter_map(|record| record.item().ok())
            .collect();
        kinds.extend(kinds_from_items(&items));
    }
    kinds
}

/// The arena reader's restore over a loaded slot, in the original's order:
/// the header's money, day and clock, the table chunks into the live
/// career, staff, arsenal and stash, then (on a full load, whose mask holds
/// every bit of the arsenal chunk's id) the list regeneration: each
/// employee's list against its world character record, the stash against
/// the player's record, every arsenal box deduplicated, each entry shelved
/// by `kinds`. Last, the load resets the player record before the import.
pub fn restore_arena(
    loaded: &LoadedSave,
    kinds: &HashMap<u32, ItemKind>,
    career: &mut crate::hub_windows::ArenaCareer,
    stores: &mut ArenaStores,
) -> ArenaRestore {
    // The load resets every arena store before it reads the header.
    career.reset();
    stores.reset();
    let mut clock = 0;
    if let PlayerHeader::Arena(header) = &loaded.data.header {
        career.company.copper = header.money;
        career.company.day = header.day;
        clock = header.clock;
    }
    let chunks: Vec<save_package::Chunk<'_>> = loaded
        .data
        .chunks
        .iter()
        .map(|chunk| save_package::Chunk {
            id: chunk.id,
            payload: &chunk.data,
        })
        .collect();
    let tables = chunks
        .iter()
        .any(|chunk| arena_save::is_table_chunk(chunk.id))
        .then(|| arena_save::read_tables(&chunks).ok())
        .flatten();
    let mut player = loaded.player_record();
    // An old save's player record is repaired before the arena's id pass:
    // the worn list, each loadout's worn list, then the carried items, each
    // dropping the ids the arena marked with the top bit, and the record is
    // re-derived afterwards. Hands are left alone.
    if loaded.converted
        && let Some(record) = player.as_mut()
    {
        arena_save::repair_converted_record(record);
    }
    let mut shelved = 0;
    let applied = tables.is_some();
    if let Some(mut tables) = tables {
        crate::arena_tables::apply(
            career,
            &mut stores.staff,
            &mut stores.arsenal,
            &mut stores.stash,
            &tables,
        );
        shelved = regenerate_saved_lists(
            &mut tables,
            loaded,
            player.as_ref(),
            &mut stores.arsenal,
            kinds,
        );
        stores.staff.employees = crate::arena_tables::employees_from_tables(&tables);
        stores.stash = crate::arena_tables::stash_from_tables(&tables);
    }
    if let Some(record) = player.as_mut() {
        record.reset_loaded_identity();
    }
    ArenaRestore {
        applied,
        shelved,
        clock,
        player,
    }
}

/// The reader's closing regeneration over the tables it read: runs only
/// when the load's mask holds the arsenal chunk's id, and writes the merged
/// employee lists and stash back into `tables`.
fn regenerate_saved_lists(
    tables: &mut ArenaTables,
    loaded: &LoadedSave,
    player: Option<&CharacterRecord>,
    arsenal: &mut Arsenal,
    kinds: &HashMap<u32, ItemKind>,
) -> usize {
    if loaded.mask & arena_save::ARSENAL_CHUNK != arena_save::ARSENAL_CHUNK {
        return 0;
    }
    let mut characters = loaded.world_characters();
    let mut staff: Vec<StaffLists> = tables
        .employees
        .employees
        .iter()
        .map(|employee| {
            // A recruit has no world record in the chunk [guess: its own
            // record is not kept by Exarust's tables, so no loadout is
            // merged for it].
            let record = (!employee.custom)
                .then(|| {
                    characters
                        .as_mut()
                        .and_then(|db| db.get(employee.character))
                        .map(|shared| shared.record().clone())
                })
                .flatten()
                .unwrap_or_else(CharacterRecord::new);
            StaffLists {
                class: employee.class,
                refs: crate::arena_ids::EmployeeRefs {
                    record,
                    items: employee.items.clone(),
                },
            }
        })
        .collect();
    let manager = player.cloned().unwrap_or_else(CharacterRecord::new);
    let mut stash: Vec<ItemEntry> = tables.stash.clone();
    let shelved = regenerate_lists(&manager, &mut stash, &mut staff, arsenal, |id| {
        kinds.get(&id).copied().unwrap_or_default()
    });
    for (employee, lists) in tables.employees.employees.iter_mut().zip(staff) {
        employee.items = lists.refs.items;
    }
    tables.stash = stash;
    shelved
}

#[allow(clippy::too_many_arguments)]
fn load_arena_slot(
    mut commands: Commands,
    request: Option<Res<ArenaSlotLoad>>,
    hub: Option<Res<crate::arena_hub_live::ArenaHub>>,
    lists: Option<ResMut<SlotLists>>,
    game: Option<Res<exa_assets::Game>>,
    mut career: Option<ResMut<crate::hub_windows::ArenaCareer>>,
    mut stores: ResMut<ArenaStores>,
    mut clock: MessageWriter<crate::world_clock::RestoreWorldClock>,
    mut report: ResMut<SaveReport>,
) {
    let (Some(request), Some(mut lists)) = (request, lists) else {
        return;
    };
    if !hub.is_some_and(|hub| hub.entered) {
        return;
    }
    commands.remove_resource::<ArenaSlotLoad>();
    let list = &mut lists.arena;
    let _ = list.any();
    let Some(index) = list.numbers().iter().position(|&n| n == request.number) else {
        warn!("save: arena slot {} is not listed", request.number);
        return;
    };
    list.select(index);
    let loaded = match list.load_current(false) {
        Ok(loaded) => loaded,
        Err(error) => {
            warn!(
                "save: arena slot {} does not load: {error:?}",
                request.number
            );
            return;
        }
    };
    let base = game
        .as_ref()
        .and_then(|game| game.0.pack(exa_assets::PackKind::Resource))
        .and_then(|pack| pack.get("objdb.rdb"))
        .and_then(|bytes| exa_formats::rdb::Database::parse(bytes).ok())
        .map(|db| exa_formats::objdb::items(&db))
        .unwrap_or_default();
    let kinds = save_item_kinds(&base, &loaded);
    let Some(career) = career.as_mut() else {
        return;
    };
    let restored = restore_arena(&loaded, &kinds, career, &mut stores);
    clock.write(crate::world_clock::RestoreWorldClock {
        saved_seconds: restored.clock,
    });
    commands.insert_resource(crate::arena_hub_live::HubSlots {
        slots: list.slots().clone(),
        number: request.number,
    });
    report.loaded = Some((request.number, restored.shelved));
    info!(
        "save: arena slot {} loaded from {:?}: tables {}, {} items shelved, clock {} s, player record {}",
        request.number,
        loaded.kind,
        restored.applied,
        restored.shelved,
        restored.clock,
        restored.player.is_some()
    );
}

/// What a story load read from a slot, before the running game takes it.
#[derive(Clone, Debug, PartialEq)]
pub struct StoryRestore {
    /// The session the player data restores (the Exarust chunk's, else the
    /// story at the saved level with the header's clock and seed).
    pub session: Session,
    /// The campaign level the load continues at.
    pub level: &'static str,
    /// The player's record with the load's reset applied.
    pub player: Option<CharacterRecord>,
    /// Where the save placed the player.
    pub placement: Option<crate::flow::Placement>,
    /// The save's world object records.
    pub items: Vec<exa_formats::objdb::Item>,
    /// Which file was read.
    pub kind: save_package::SlotKind,
}

/// The story's full load of a slot, as the saves' character screen's
/// CONTINUE runs it: the slot becomes current, then its files are tried
/// (the checkpoint first when asked; the slot moves to the front and a
/// checkpoint read is copied over the save, [`SlotList::load_current`]).
/// The level is the save's locale (the transition's forced one included)
/// when the campaign names it, else the first level; the player's record
/// is the one the placement names, reset as the load resets it.
pub fn restore_story(
    list: &mut SlotList,
    number: u32,
    prefer_checkpoint: bool,
) -> Result<StoryRestore, crate::session::SaveError> {
    let _ = list.any();
    let index = list
        .numbers()
        .iter()
        .position(|&n| n == number)
        .ok_or(crate::session::SaveError::Malformed)?;
    list.select(index);
    let loaded = list.load_current(prefer_checkpoint)?;
    let locale = loaded.level_locale_name();
    // Exarust's own payload when the save has one; otherwise the retail
    // player data's globals (its journal, clock, experience and transition
    // state), which the original's reader restores without any payload of
    // ours.
    let session = crate::session::read_package(&loaded.package)
        .unwrap_or_else(|_| crate::session::session_from_player_data(&loaded.data));
    let level = story_at(&locale).level();
    let mut player = loaded.player_record();
    if let Some(record) = player.as_mut() {
        record.reset_loaded_identity();
    }
    let placement = loaded
        .data
        .placement()
        .and_then(Result::ok)
        .map(|placement| placement_of(&placement));
    let items = loaded
        .world_objects()
        .map(|db| {
            db.world
                .records()
                .filter_map(|record| record.item().ok())
                .collect()
        })
        .unwrap_or_default();
    Ok(StoryRestore {
        session,
        level,
        player,
        placement,
        items,
        kind: loaded.kind,
    })
}

/// Runs a story load into the running game: the record becomes the next
/// player's ([`SavedPlayer`], [`crate::creator_screen::CreatedCharacter`]),
/// the old player goes, the world clock takes the saved seconds and the
/// flow continues the story at the saved level and placement. A load that
/// fails starts the first level, as the original's continue does.
#[allow(clippy::too_many_arguments)]
fn load_story_slot(
    mut commands: Commands,
    mut requests: MessageReader<StorySlotLoad>,
    lists: Option<ResMut<SlotLists>>,
    mut flow: Option<ResMut<crate::flow::Flow>>,
    mut steps: Option<ResMut<crate::flow::FlowSteps>>,
    mut created: Option<ResMut<crate::creator_screen::CreatedCharacter>>,
    players: Query<Entity, With<crate::player::Player>>,
    levels: Query<(), With<exa_world::level_lifecycle::LevelEntity>>,
    mut clock: MessageWriter<crate::world_clock::RestoreWorldClock>,
    mut report: ResMut<SaveReport>,
) {
    let Some(request) = requests.read().last().copied() else {
        return;
    };
    let Some(mut lists) = lists else {
        return;
    };
    let restored = restore_story(&mut lists.story, request.number, request.prefer_checkpoint);
    let (level, placement) = match restored {
        Ok(restored) => {
            info!(
                "save: story slot {} loaded from {:?}: level {}, clock {} s, player {:?}, {} world object records",
                request.number,
                restored.kind,
                restored.level,
                restored.session.clock,
                restored.player.as_ref().map(|r| r.name.clone()),
                restored.items.len()
            );
            report.story_loaded = Some((request.number, restored.kind));
            clock.write(crate::world_clock::RestoreWorldClock {
                saved_seconds: restored.session.clock,
            });
            if let Some(record) = restored.player.clone() {
                if let Some(created) = created.as_mut() {
                    created.0 = Some(record.clone());
                }
                commands.insert_resource(SavedPlayer {
                    record,
                    items: restored.items.clone(),
                });
            } else {
                commands.remove_resource::<SavedPlayer>();
            }
            commands.insert_resource(LoadedStory(restored.session.clone()));
            (Some(restored.level), restored.placement)
        }
        Err(error) => {
            warn!(
                "save: story slot {} does not load ({error:?}); the first level starts",
                request.number
            );
            (None, None)
        }
    };
    // The loaded character replaces the player that survived the last
    // session: the level load spawns it from the record.
    for player in &players {
        commands.entity(player).despawn();
    }
    let (Some(flow), Some(steps)) = (flow.as_mut(), steps.as_mut()) else {
        return;
    };
    let viewport = flow.viewport;
    let at = placement.unwrap_or(crate::flow::Placement {
        rotation: crate::flow::NO_ROTATION,
        position: Vec3::ZERO,
    });
    let mut resumed = flow.continue_story(level, at, viewport);
    if levels.is_empty() {
        // The saves' screen already took the world down: the load does not
        // tear it down a second time.
        for step in &mut resumed {
            if let crate::flow::Step::LoadLevel { teardown, .. } = step {
                *teardown = false;
            }
        }
    }
    if placement.is_none() {
        // A save without a placement (one Exarust wrote before it saved
        // the player) starts at the level's own start.
        resumed.retain(|step| {
            !matches!(
                step,
                crate::flow::Step::PlacePlayer(_) | crate::flow::Step::Camera(_)
            )
        });
    }
    steps.steps.extend(flow.continue_session(resumed));
}

/// The session the last story load restored (its story position, journal,
/// clock and experience state), for the units that take it over.
#[derive(Resource, Clone, Debug, PartialEq)]
pub struct LoadedStory(pub Session);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::arsenal::{SHIELDS, WEAPONS};
    use exa_formats::save_package::SlotKind;

    fn folder(name: &str) -> PathBuf {
        let path =
            std::env::temp_dir().join(format!("exarust-save-live-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).unwrap();
        path
    }

    #[test]
    fn the_story_saves_at_the_loaded_campaign_level() {
        assert_eq!(story_at("exanima01").level(), "exanima01");
        let third = exa_script::story::Story::begin().advance().to_string();
        assert_eq!(story_at(&format!("{third}.rfc")).level(), third);
        assert_eq!(story_at("arena1").index(), 0);
    }

    #[test]
    fn a_new_slot_takes_the_free_number_and_never_writes_over_a_save() {
        let dir = folder("new");
        let mut lists = SlotLists::new(&dir);
        let career = crate::hub_windows::ArenaCareer::default();
        let session = live_session(Some("exanima01"), 8684, &career, &ArenaStores::default());
        assert_eq!(session.mode, Mode::Story);
        assert_eq!(
            save_new_slot(&mut lists.story, &session, None, &SaveWorld::default()).unwrap(),
            Some(1)
        );
        let (back, kind) = Slots::story(&dir).load(1, false).unwrap();
        assert_eq!((back.clock, kind), (8684, SlotKind::Save));
        // A file at the next number that the list left out (it does not
        // read) is not written over.
        std::fs::write(Slots::story(&dir).file(2, SlotKind::Save), b"not a save").unwrap();
        assert_eq!(
            save_new_slot(&mut lists.story, &session, None, &SaveWorld::default()).unwrap(),
            None
        );
        assert_eq!(
            std::fs::read(Slots::story(&dir).file(2, SlotKind::Save)).unwrap(),
            b"not a save"
        );
        assert_eq!(lists.story.numbers(), &[1]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_hub_saves_the_company_and_its_load_regenerates_the_shelves() {
        let dir = folder("arena");
        let mut lists = SlotLists::new(&dir);
        let mut career = crate::hub_windows::ArenaCareer::default();
        career.company.copper = 750;
        career.company.day = 3;
        let mut stores = ArenaStores {
            stash: vec![5],
            ..Default::default()
        };
        stores.staff.employees.push(crate::staff::Employee {
            class: crate::staff::COMBATANT,
            kit: vec![7],
            ..Default::default()
        });
        let session = live_session(Some("arenahub.rfc"), 99, &career, &stores);
        assert_eq!(session.mode, Mode::Arena);
        let number = save_new_slot(
            &mut lists.arena,
            &session,
            Some("Manager"),
            &SaveWorld::default(),
        )
        .unwrap()
        .unwrap();
        assert_eq!(number, 1);
        assert_eq!(lists.arena.names()[0].as_deref(), Some("Manager"));

        // A fresh manager loads it: the slot is listed, selected and read.
        let mut list = SlotList::new(Slots::arena(&dir));
        assert!(list.any());
        assert_eq!(list.select(0), Some(1));
        let loaded = list.load_current(false).unwrap();
        assert_eq!(loaded.mask, !0);
        let kinds: HashMap<u32, ItemKind> = [
            (
                5,
                ItemKind {
                    known: true,
                    weapon: true,
                    ..Default::default()
                },
            ),
            (
                7,
                ItemKind {
                    known: true,
                    shield: true,
                    ..Default::default()
                },
            ),
        ]
        .into_iter()
        .collect();
        let mut back = crate::hub_windows::ArenaCareer::default();
        let mut live = ArenaStores::default();
        let restored = restore_arena(&loaded, &kinds, &mut back, &mut live);
        assert!(restored.applied);
        assert_eq!(restored.clock, 99);
        assert_eq!((back.company.copper, back.company.day), (750, 3));
        // The stash and the fighter's list are shelved with the saved-item
        // flag; the lists themselves come back unchanged.
        assert_eq!(restored.shelved, 2);
        assert_eq!(live.arsenal.shelves[WEAPONS][0][0].item, 5);
        assert_eq!(
            live.arsenal.shelves[WEAPONS][0][0].flags & crate::arena_ids::SAVED_ITEM,
            crate::arena_ids::SAVED_ITEM
        );
        assert_eq!(live.arsenal.shelves[SHIELDS][0][0].item, 7);
        assert_eq!(live.stash, vec![5]);
        assert_eq!(live.staff.employees[0].kit, vec![7]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_story_save_keeps_the_player_and_its_load_restores_it() {
        let dir = folder("story-player");
        let mut lists = SlotLists::new(&dir);
        let mut record = CharacterRecord::reset();
        record.name = "Ash".to_string();
        record.worn.push(ItemRef::new(0x77, 0));
        let mut possessions = exa_items::possession::Possessions::of_record(&record);
        possessions.give(0x55, 0);
        let player = StoryPlayer {
            record: record_with_possessions(record, &possessions),
            position_cm: [120.0, 30.0, -450.0],
            heading: 1.25,
        };
        let career = crate::hub_windows::ArenaCareer::default();
        let mut session = live_session(Some("exanima02"), 4321, &career, &ArenaStores::default());
        let export = story_player_world(&player, &mut session).unwrap();
        let number = save_new_slot(
            &mut lists.story,
            &session,
            Some("Ash"),
            &export.save_world(None, false),
        )
        .unwrap()
        .unwrap();
        // A fresh manager lists the slot under the player's name.
        let mut list = SlotList::new(Slots::story(&dir));
        assert!(list.any());
        assert_eq!(list.names()[0].as_deref(), Some("Ash"));
        // The character screen's select reads it with its mask.
        let found = list.character_screen_select(0, 1);
        assert_eq!(found.availability, 2);
        assert_eq!(list.current(), number as i32);
        assert_eq!(found.loaded.unwrap().player_record().unwrap().name, "Ash");
        let restored = restore_story(&mut list, number, true).unwrap();
        assert_eq!(restored.level, "exanima02");
        assert_eq!(restored.session.clock, 4321);
        let back = restored.player.unwrap();
        assert_eq!(back.name, "Ash");
        assert!(back.worn.iter().any(|w| w.id == 0x77));
        assert!(back.inventory.iter().any(|c| c.item == 0x55));
        let at = restored.placement.unwrap();
        assert_eq!(at.position, Vec3::new(120.0, 30.0, -450.0));
        let placed = crate::level_change::PlayerPlacement::of_flow(&at);
        assert!((placed.yaw - 1.25).abs() < 1e-5);
        assert_eq!(restored.kind, save_package::SlotKind::Save);
        // An index outside the list answers nothing.
        assert_eq!(list.character_screen_select(5, 1).availability, 0);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A human body (maximum health 0.25) as the spawn makes it from a
    /// record, with the record's health loaded.
    fn spawned_from(
        record: &CharacterRecord,
    ) -> (exa_combat::Fighter, exa_combat::StateFlags, f32) {
        let stats = exa_formats::character_stats::CharacterStats::derive(
            record,
            None,
            exa_formats::character_stats::Spawn {
                story: true,
                dead: false,
            },
        );
        let mut fighter = exa_combat::Fighter::of_stats(None, stats);
        let mut flags = exa_combat::StateFlags::default();
        let mut focus = 1.0;
        restore_body_health(record, &mut fighter, &mut flags, &mut focus);
        (fighter, flags, focus)
    }

    #[test]
    fn a_story_save_keeps_the_players_wounds_and_its_load_restores_them() {
        let dir = folder("story-health");
        let mut lists = SlotLists::new(&dir);
        // A wounded human: yellow 0.1 and red 0.18 of 0.25, focus 0.4, a
        // bandage's 0.03 still to heal, in combat idle.
        let mut fighter = exa_combat::Fighter::default();
        assert_eq!(fighter.health.max, 0.25);
        fighter.health.yellow = 0.1;
        fighter.health.red = 0.18;
        fighter.health.heal_pool = 0.03;
        let health = body_health(
            &fighter,
            exa_combat::StateFlags::default(),
            exa_combat::Condition::COMBAT_IDLE,
            0.4,
        );
        assert_eq!(health.condition, exa_combat::Condition::COMBAT_IDLE.0);
        let mut record = record_with_health(CharacterRecord::reset(), health);
        record.name = "Ash".to_string();
        // 0.1 / 0.25 × 65535 = 26214; 0.18 / 0.25 × 65535 = 47185.2 → 47185;
        // 0.4 × 65535 = 26214; the seed 0xFFFF; one heal-pool entry.
        assert_eq!(
            record.pools,
            exa_formats::characters::Pools {
                yellow: 26214,
                red: 47185,
                focus: 26214
            }
        );
        assert_eq!(record.seed, 0xFFFF);
        assert_eq!(record.heal_pool(), Some(0.03));
        assert_eq!(record.potion_expiry(), None);
        let player = StoryPlayer {
            record,
            position_cm: [0.0, 0.0, 0.0],
            heading: 0.0,
        };
        let career = crate::hub_windows::ArenaCareer::default();
        let mut session = live_session(Some("exanima01"), 60, &career, &ArenaStores::default());
        let export = story_player_world(&player, &mut session).unwrap();
        let number = save_new_slot(
            &mut lists.story,
            &session,
            Some("Ash"),
            &export.save_world(None, false),
        )
        .unwrap()
        .unwrap();
        let mut list = SlotList::new(Slots::story(&dir));
        let back = restore_story(&mut list, number, false)
            .unwrap()
            .player
            .unwrap();
        assert_eq!(back.pools, player.record.pools);
        assert_eq!(back.saved, player.record.saved);
        // The spawn loads them back in double precision: 0.25 × 47185 /
        // 65535 = 0.17999923, 0.25 × 26214 / 65535 = 0.1, 26214 / 65535 =
        // 0.4; the heal pool as saved; no potion.
        let (body, flags, focus) = spawned_from(&back);
        assert_eq!(body.health.red, 0.179_999_23);
        assert_eq!(body.health.yellow, 0.1);
        assert_eq!(focus, 0.4);
        assert_eq!(body.health.heal_pool, 0.03);
        assert!(!flags.has(exa_combat::StateFlags::POTION));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_saved_potion_comes_back_with_its_flag_and_the_last_entry_quirk() {
        // A potion drunk until 5400 s, no heal pool: the flag and the
        // expiry come back; yellow 0.0625 → 16384 → 0.06250095, red 0.15 →
        // 39321 → 0.15, focus 0 stays 0.
        let mut fighter = exa_combat::Fighter::default();
        fighter.health.yellow = 0.0625;
        fighter.health.red = 0.15;
        fighter.potion_until = 5400;
        let flags = exa_combat::StateFlags(exa_combat::StateFlags::POTION);
        let health = body_health(&fighter, flags, exa_combat::Condition::NORMAL, 0.0);
        let record = record_with_health(CharacterRecord::reset(), health);
        assert_eq!(
            (record.pools.yellow, record.pools.red, record.pools.focus),
            (16384, 39321, 0)
        );
        let (body, flags, focus) = spawned_from(&record);
        assert_eq!(body.health.yellow, 0.062_500_95);
        assert_eq!(body.health.red, 0.15);
        assert_eq!(focus, 0.0);
        assert!(flags.has(exa_combat::StateFlags::POTION));
        assert_eq!(body.potion_until, 5400);
        assert_eq!(body.health.heal_pool, 0.0);
        // With a heal pool too, the original's loop gives the heal pool the
        // last entry's value: the potion's expiry read as a float.
        fighter.health.heal_pool = 0.03;
        let both = record_with_health(
            CharacterRecord::reset(),
            body_health(&fighter, flags, exa_combat::Condition::NORMAL, 0.0),
        );
        let (body, _, _) = spawned_from(&both);
        assert_eq!(body.health.heal_pool, f32::from_bits(5400));
        assert_eq!(body.potion_until, 5400);
        // A body that drank nothing keeps no potion flag.
        let plain = exa_combat::Fighter {
            potion_until: 99,
            ..Default::default()
        };
        let none = body_health(
            &plain,
            exa_combat::StateFlags::default(),
            exa_combat::Condition::NORMAL,
            1.0,
        );
        assert_eq!(none.potion, None);
    }

    #[test]
    fn a_story_save_stamps_the_views_camera_and_its_load_leaves_it() {
        let dir = folder("camera");
        let mut lists = SlotLists::new(&dir);
        let career = crate::hub_windows::ArenaCareer::default();
        let mut session = live_session(Some("exanima02"), 77, &career, &ArenaStores::default());
        let camera = SaveCamera {
            target_cm: [7034.0, 1532.5, 4748.5],
            zoom: 1.25,
        };
        camera.stamp(&mut session);
        let number = save_new_slot(
            &mut lists.story,
            &session,
            Some("Ash"),
            &SaveWorld::default(),
        )
        .unwrap()
        .unwrap();
        // The file's header carries the pair.
        let (_, kind) = Slots::story(&dir).load(number, false).unwrap();
        assert_eq!(kind, SlotKind::Save);
        let mut list = SlotList::new(Slots::story(&dir));
        let restored = restore_story(&mut list, number, false).unwrap();
        // The readers read the pair and drop it: the load keeps the
        // session's defaults.
        assert_eq!(restored.session.camera, [0.0; 3]);
        assert_eq!(restored.session.zoom, 1.0);
        // The header itself holds the values, not only the Exarust payload.
        let bytes = std::fs::read(Slots::story(&dir).file(number, SlotKind::Save)).unwrap();
        let package = save_package::PackageImage::read(&bytes).unwrap();
        let data = save_package::PlayerData::parse(
            package.section(save_package::PLAYER_DATA).unwrap(),
            save_package::Flavour::Story,
        )
        .unwrap();
        assert_eq!(data.header.locale(), save_package::name("exanima02"));
        let save_package::PlayerHeader::Story(header) = data.header else {
            panic!("story")
        };
        assert_eq!(header.camera, [7034.0, 1532.5, 4748.5]);
        assert_eq!(header.zoom, 1.25);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_checkpoint_step_saves_first_then_copies_and_leaves_an_existing_one() {
        let dir = folder("checkpoint-step");
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_plugins(SaveLivePlugin)
            .insert_resource(SaveFolder(dir.clone()));
        app.update();
        app.world_mut().write_message(SaveToNewSlot);
        app.update();
        let number = app.world().resource::<SaveReport>().saved.unwrap();
        // Creating a slot makes it current, which the flow's save steps need.
        assert_eq!(
            app.world().resource::<SlotLists>().story.current(),
            number as i32
        );
        // No checkpoint yet: the step writes the save and copies it.
        assert!(
            !Slots::story(&dir)
                .file(number, SlotKind::Checkpoint)
                .exists()
        );
        app.world_mut().write_message(SaveStep::Checkpoint);
        app.update();
        assert!(
            Slots::story(&dir)
                .file(number, SlotKind::Checkpoint)
                .is_file()
        );
        let save = std::fs::read(Slots::story(&dir).file(number, SlotKind::Save)).unwrap();
        let checkpoint =
            std::fs::read(Slots::story(&dir).file(number, SlotKind::Checkpoint)).unwrap();
        assert_eq!(save, checkpoint);
        // An existing checkpoint is left alone: its bytes do not change.
        std::fs::write(
            Slots::story(&dir).file(number, SlotKind::Checkpoint),
            b"old",
        )
        .unwrap();
        app.world_mut().write_message(SaveStep::Checkpoint);
        app.update();
        assert_eq!(
            std::fs::read(Slots::story(&dir).file(number, SlotKind::Checkpoint)).unwrap(),
            b"old"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_checkpoint_step_with_saving_disabled_copies_the_old_save() {
        // The checkpoint routine's save returns on the disable flag, but the
        // copy still runs: the save as it was becomes the checkpoint.
        let dir = folder("checkpoint-disabled");
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_plugins(SaveLivePlugin)
            .insert_resource(SaveFolder(dir.clone()));
        app.update();
        app.world_mut().write_message(SaveToNewSlot);
        app.update();
        let number = app.world().resource::<SaveReport>().saved.unwrap();
        let save_path = Slots::story(&dir).file(number, SlotKind::Save);
        std::fs::write(&save_path, b"stale save").unwrap();
        let flow =
            crate::level_transition::transition_flow(false, true, false, false, false, false, 0);
        app.insert_resource(crate::level_transition::LiveTransition {
            level: "nowhere".to_string(),
            spawn: 0,
            flow,
        });
        app.world_mut().write_message(SaveStep::Checkpoint);
        app.update();
        assert_eq!(std::fs::read(&save_path).unwrap(), b"stale save");
        assert_eq!(
            std::fs::read(Slots::story(&dir).file(number, SlotKind::Checkpoint)).unwrap(),
            b"stale save"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_level_change_that_disabled_saving_refuses_a_new_slot() {
        let dir = folder("disabled");
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_plugins(SaveLivePlugin)
            .insert_resource(SaveFolder(dir.clone()));
        // An archive destination whose import failed leaves saving disabled.
        let flow =
            crate::level_transition::transition_flow(false, true, false, false, false, false, 0);
        assert!(flow.saves_disabled);
        app.insert_resource(crate::level_transition::LiveTransition {
            level: "nowhere".to_string(),
            spawn: 0,
            flow,
        });
        app.update();
        app.world_mut().write_message(SaveToNewSlot);
        app.update();
        app.update();
        assert_eq!(app.world().resource::<SaveReport>().saved, None);
        assert!(!Slots::story(&dir).file(1, SlotKind::Save).exists());
        // A level change that did not disable saving saves as usual.
        app.insert_resource(crate::level_transition::live_transition(
            "exanima01",
            0,
            true,
        ));
        app.world_mut().write_message(SaveToNewSlot);
        app.update();
        assert_eq!(app.world().resource::<SaveReport>().saved, Some(1));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn an_old_arena_load_repairs_the_players_record_before_the_id_pass() {
        // A converted old save: the player's worn, loadout and carried lists
        // hold the arena's top-bit markers, which the repair drops before
        // the identity reset.
        let mut record = CharacterRecord::new();
        record.name = "Ash".to_string();
        record.id = PLAYER_RECORD_ID;
        record.worn = vec![
            ItemRef::new(1, 0),
            ItemRef::new(0x8000_0002, 0),
            ItemRef::new(3, 0),
        ];
        record.loadouts.push(Default::default());
        record.loadouts[0].worn = vec![ItemRef::new(1, 0), ItemRef::new(0xC000_0002, 0)];
        record.inventory = vec![
            Carried {
                item: 0x8000_0001,
                flags: 0,
                extra: 0,
            },
            Carried {
                item: 6,
                flags: 0,
                extra: 0,
            },
        ];
        let mut db =
            exa_formats::character_db::CharacterDb::new(exa_formats::character_db::WORLD_TAG);
        db.add(PLAYER_RECORD_ID, record).unwrap();
        let export =
            crate::slots::export_world_databases(&mut db, &exa_formats::objdb::ObjectDb::default())
                .unwrap();
        let mut data = save_package::PlayerData::arena(save_package::name("arenahub"));
        data.character = Some(
            save_package::CharacterPlacement {
                reference: exa_formats::characters::WORLD_DB | PLAYER_RECORD_ID,
                position: [0.0, 0.0, 300.0],
                heading: 0.0,
                condition: 0,
                chunks: Vec::new(),
            }
            .write(),
        );
        let PlayerHeader::Arena(header) = &mut data.header else {
            unreachable!()
        };
        header.data_version = 8;
        let mut package = save_package::PackageImage::new();
        package.put_section(
            save_package::WORLD_CHARACTER_DB,
            &save_package::world_character_db(export.characters.as_deref().unwrap()),
        );
        package.put_section(save_package::PLAYER_DATA, &data.write_ordered());
        let loaded = LoadedSave {
            data,
            package,
            kind: save_package::SlotKind::Save,
            forced_locale: None,
            level_locale: save_package::name("arenahub"),
            has_level_section: false,
            has_world_objects: false,
            has_world_characters: true,
            converted: true,
            mask: !0,
        };
        let mut career = crate::hub_windows::ArenaCareer::default();
        let mut stores = ArenaStores::default();
        let restored = restore_arena(&loaded, &HashMap::new(), &mut career, &mut stores);
        let back = restored.player.expect("the player's record");
        assert_eq!(
            back.worn.iter().map(|w| w.id).collect::<Vec<_>>(),
            vec![1, 3]
        );
        assert_eq!(
            back.loadouts[0]
                .worn
                .iter()
                .map(|w| w.id)
                .collect::<Vec<_>>(),
            vec![1]
        );
        assert_eq!(
            back.inventory.iter().map(|c| c.item).collect::<Vec<_>>(),
            vec![6]
        );
        // The identity reset follows the repair (the race byte is cleared).
        assert_eq!(back.profession() & 0x0000_FF00, 0);
    }

    #[test]
    fn the_plugin_saves_on_request_only_with_a_folder() {
        let dir = folder("plugin");
        // Without a folder a request does nothing and no manager is made.
        let mut idle = App::new();
        idle.add_plugins(MinimalPlugins).add_plugins(SaveLivePlugin);
        idle.update();
        idle.world_mut().write_message(SaveToNewSlot);
        idle.update();
        idle.update();
        assert_eq!(idle.world().resource::<SaveReport>().saved, None);
        assert!(!idle.world().contains_resource::<SlotLists>());
        // With one, a request writes the first free slot.
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_plugins(SaveLivePlugin)
            .insert_resource(SaveFolder(dir.clone()));
        app.update();
        app.world_mut().write_message(SaveToNewSlot);
        app.update();
        assert_eq!(app.world().resource::<SaveReport>().saved, Some(1));
        assert!(Slots::story(&dir).file(1, SlotKind::Save).is_file());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
