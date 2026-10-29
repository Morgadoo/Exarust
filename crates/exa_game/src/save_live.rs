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
//! Not written yet: the save's level section (Exarust's loaded world has no
//! converter back into the exporter's scene) and the world databases (the
//! running game keeps no live object or character database).
//!
//! Evidence: the save unit's new-slot, save, full load, arena reader and
//! list regeneration routines (`docs/formats/SAVES.md`).

use std::collections::HashMap;
use std::path::PathBuf;

use bevy::prelude::*;
use exa_formats::arena_save::{self, ArenaTables, ItemEntry};
use exa_formats::characters::CharacterRecord;
use exa_formats::save_package::{self, PlayerHeader};

use crate::arena_ids::{StaffLists, kinds_from_items, regenerate_lists};
use crate::arsenal::{Arsenal, ItemKind};
use crate::session::{Mode, Session};
use crate::slots::{LoadedSave, SaveWorld, SlotList, Slots};
use crate::staff::Staff;

/// The folder the running game saves into and loads from. Inserted by the
/// host (`--save-dir`); without it nothing here runs.
#[derive(Resource, Clone, Debug, PartialEq, Eq)]
pub struct SaveFolder(pub PathBuf);

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

/// What the last live save or load did, for the report and the tests.
#[derive(Resource, Clone, Debug, Default, PartialEq, Eq)]
pub struct SaveReport {
    /// The slot the last save wrote.
    pub saved: Option<u32>,
    /// The arena slot the hub loaded and how many items the regeneration
    /// shelved.
    pub loaded: Option<(u32, usize)>,
}

/// The running game's save and load.
pub struct SaveLivePlugin;

impl Plugin for SaveLivePlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<SaveToNewSlot>()
            .add_message::<crate::world_clock::RestoreWorldClock>()
            .init_resource::<ArenaStores>()
            .init_resource::<SaveReport>()
            .add_systems(
                Update,
                (
                    make_slot_lists,
                    scripted_saves,
                    save_to_new_slot,
                    load_arena_slot,
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
) -> std::io::Result<Option<u32>> {
    let next = list.next_free_number()?;
    let file = list.slots().file(next, save_package::SlotKind::Save);
    if std::fs::metadata(&file).is_ok_and(|meta| meta.len() > 0) {
        return Ok(None);
    }
    let number = list.create(player_name)?;
    list.save_current(session, &SaveWorld::default())?;
    Ok(Some(number))
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
    mut report: ResMut<SaveReport>,
) {
    if requests.read().count() == 0 {
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
    let session = live_session(level, seconds, career, &stores);
    let name = match session.mode {
        Mode::Arena => Some(career.player.name.as_str()),
        Mode::Story => None,
    };
    match save_new_slot(lists.of_mode(session.mode), &session, name) {
        Ok(Some(number)) => {
            info!(
                "save: {:?} session saved to new slot {number} (level {level:?}, clock {seconds} s)",
                session.mode
            );
            report.saved = Some(number);
        }
        Ok(None) => warn!("save: the next slot's file already holds a save; nothing written"),
        Err(error) => warn!("save: cannot write a new slot: {error}"),
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
            save_new_slot(&mut lists.story, &session, None).unwrap(),
            Some(1)
        );
        let (back, kind) = Slots::story(&dir).load(1, false).unwrap();
        assert_eq!((back.clock, kind), (8684, SlotKind::Save));
        // A file at the next number that the list left out (it does not
        // read) is not written over.
        std::fs::write(Slots::story(&dir).file(2, SlotKind::Save), b"not a save").unwrap();
        assert_eq!(
            save_new_slot(&mut lists.story, &session, None).unwrap(),
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
        let number = save_new_slot(&mut lists.arena, &session, Some("Manager"))
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
