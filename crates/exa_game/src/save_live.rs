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
//! Every save writes the level section after the world databases
//! ([`crate::level_save`]: the section the level was read from with the
//! live local tier and placed objects written over it) and a story load
//! hands the save's section to the level load ([`StoryRestore::level_section`]).
//! The arena's save runs its promotion pass first (the manager, the stash,
//! the arsenal, the staff, the scene's entity ids). A story save first
//! flushes every scene character into its store (the world character
//! database or the level's) and the live player over the world store's
//! player slot, and its level section carries the scene's characters
//! ([`crate::level_save`]).
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

/// The view's camera place and zoom a save stamps into its header. The
/// original's writers copy twelve bytes of the viewport's camera node (the
/// translation row of its matrix: where the eye stands, centimetres) and
/// the camera controller's current zoom word into both headers; both
/// readers read the pair into scratch and drop it, so a load never moves
/// the view. The running game keeps it from the chase camera every frame
/// ([`follow_the_view`]); without a camera (a headless run) a save writes
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

    /// The pair of a chase camera whose eye stands at `eye` (m): the eye in
    /// the engine's centimetres (Exarust's world is the original's axes in
    /// metres) and the camera's shown zoom.
    pub fn of_view(eye: Vec3, camera: &crate::player::ChaseCamera) -> Self {
        Self {
            target_cm: (eye * 100.0).to_array(),
            zoom: camera.shown_zoom(),
        }
    }
}

/// Keeps [`SaveCamera`] on the chase camera, as the original's writers read
/// the live view when they run: the eye of the first chase camera and its
/// shown zoom. A run without a chase camera keeps what it had.
pub fn follow_the_view(
    cameras: Query<(&crate::player::ChaseCamera, &Transform)>,
    mut camera: ResMut<SaveCamera>,
) {
    let Some((chase, transform)) = cameras.iter().next() else {
        return;
    };
    let now = SaveCamera::of_view(transform.translation, chase);
    if *camera != now {
        *camera = now;
    }
}

/// The level exit's availability check: the current story slot's save holds
/// the locale's section, or the game archive holds `<locale>.rfc`. The
/// original asks it before an exit changes anything and does nothing for a
/// level it does not find. Without the archive (a headless run) every level
/// counts as there.
pub fn level_available(
    lists: Option<&SlotLists>,
    game: Option<&exa_assets::Game>,
    locale: &str,
) -> bool {
    let Some(game) = game else {
        return true;
    };
    let archive_has = |locale: &str| {
        let entry = crate::save_scene::archive_level_name(locale);
        game.0
            .pack(exa_assets::PackKind::Resource)
            .and_then(|pack| pack.get(&entry))
            .is_some()
    };
    match lists.map(|lists| &lists.story) {
        Some(list) if list.current() > 0 => {
            list.slots()
                .level_available(list.current() as u32, locale, archive_has)
        }
        _ => archive_has(locale),
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

/// The flow's save work on the current slot of the mode's manager: a
/// save (with the death rule or without), the death's drop of the save
/// file, the copy of the save over the checkpoint, a new game's slot and
/// the arena forfeit's player-data save.
#[derive(Message, Clone, Copy, Debug, PartialEq, Eq)]
pub enum SaveStep {
    Save {
        drop_on_low_health: bool,
    },
    Drop,
    Checkpoint,
    /// A new story's or career's empty slot, which becomes current.
    NewSlot,
    /// The arena match quit's forfeit save: the player data alone.
    ForfeitSave,
    /// A save whose save is then copied over the checkpoint whether or not
    /// one exists (the adapter journey's save at its end): the save keeps
    /// the writer's disable check, the copy does not.
    SaveAndCheckpoint,
    /// The level exit's transition save: when the current slot's save holds
    /// neither an `exanima04` section nor one for `locale` (the exit's
    /// destination, entered for the first time), the save is written and
    /// copied over the checkpoint as [`SaveStep::SaveAndCheckpoint`] does,
    /// and with `level_time` the engine clock is kept ([`LevelTime`]);
    /// otherwise it is a plain save without the death rule.
    TransitionSave {
        locale: &'static str,
        level_time: bool,
    },
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

/// The load's request after the identity reset. Once the record's race
/// byte is cleared, the original stores the request word `0x1000` and the
/// world scene on the loaded character and posts it to the content
/// stream's character ring, so the character is built anew with neither
/// its AI started nor a saved AI pose restored
/// ([`exa_assets::content_stream::NO_AI_START`]). Exarust posts it with
/// the player's body ([`LoadedPlayerStream`], [`post_loaded_player`]): the
/// stream thread builds the body off the main thread, and the next level's
/// player spawn fetches it, waiting while it is built, as the original's
/// import spins on the fetch (the player that survived the last session is
/// despawned, [`SavedPlayer`]). The owner key is [`PLAYER_RECORD_ID`] and
/// the class word 0 [Exarust's: the original posts its own character
/// object, whose class word was not traced].
pub fn loaded_player_request(scene: u64) -> exa_assets::content_stream::CharacterRequest {
    exa_assets::content_stream::CharacterRequest {
        owner: u64::from(PLAYER_RECORD_ID),
        scene,
        class: 0,
        flags: exa_assets::content_stream::NO_AI_START,
    }
}

/// The loaded player's request to the content stream, from the story load
/// until the level's player spawn fetches it.
#[derive(Resource, Clone, Debug)]
pub struct LoadedPlayerStream {
    pub request: exa_assets::content_stream::CharacterRequest,
    /// The record the body is built for (its physique morphs the body).
    pub record: CharacterRecord,
    /// When the request went to the stream; `None` until it is posted.
    pub posted: Option<std::time::Instant>,
}

/// The body morph the loaded player's spawn gives its body: the record's
/// physique (the command line's `--physique` overrides it at the spawn,
/// which then builds its own).
pub fn loaded_player_morph(record: &CharacterRecord) -> exa_assets::BodyMorph {
    exa_assets::BodyMorph::of_model(
        exa_assets::model::BODY_MODEL,
        exa_formats::morph::Physique::of_record(record),
    )
}

/// Posts the loaded player's request with its body's job (the morphed body
/// model, parsed here; the stream thread builds it).
fn post_loaded_player(
    mut commands: Commands,
    stream: Option<ResMut<LoadedPlayerStream>>,
    requests: Option<Res<crate::content_requests::ContentRequests>>,
    mut assets: crate::gui_drop::DropAssets,
) {
    let (Some(mut stream), Some(requests)) = (stream, requests) else {
        return;
    };
    if stream.posted.is_some() {
        return;
    }
    let morph = loaded_player_morph(&stream.record);
    let Some(mut loader) = assets.loader() else {
        return;
    };
    let Some(job) = loader.morphed_job(
        exa_assets::PackKind::Resource,
        exa_assets::model::BODY_MODEL,
        Some(&morph),
    ) else {
        // No body to build: the spawn builds the player as it would.
        commands.remove_resource::<LoadedPlayerStream>();
        return;
    };
    info!(
        "content stream: the loaded player {:?} is posted to the character ring (body {})",
        stream.record.name,
        job.key()
    );
    requests.request_character(stream.request, job);
    stream.posted = Some(std::time::Instant::now());
}

/// The story respawn's player rebuild (the death screen's CONTINUE): the
/// original's respawn saves, clears the scene (the player's body goes with
/// it) and spawns the player anew from the current record once the level is
/// in. Exarust's player survives a level change, so the rebuild does that
/// clear for the player after the respawn's save ([`rebuild_player`]): the
/// record the save writes as the player's (the stripped record after the
/// reconstruction device's strip) becomes the next spawn's
/// ([`SavedPlayer`]) and the player root is despawned. The request to the
/// content stream was posted at the CONTINUE (`Step::PlayerData`).
#[derive(Message, Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RebuildPlayer;

/// Runs [`RebuildPlayer`] after the respawn's save has read the old body
/// (the flow's steps, then the save steps, in the same frame) and before
/// the level change's teardown, while the strip's record is still kept.
fn rebuild_player(
    mut commands: Commands,
    mut rebuilds: MessageReader<RebuildPlayer>,
    mut live: ResMut<exa_assets::live_objects::LiveObjects>,
    mut players: PlayerSaveQuery,
    roots: Query<Entity, With<crate::player::Player>>,
    created: Option<Res<crate::creator_screen::CreatedCharacter>>,
    left: Option<Res<crate::left_behind::LeftBody>>,
) {
    if rebuilds.read().count() == 0 {
        return;
    }
    sync_players(&mut live, &mut players);
    let Some(player) =
        live_story_player_left(&players, created.as_deref(), &|_| None, left.as_deref())
    else {
        warn!("respawn: no player to rebuild");
        return;
    };
    let items = world_items(&live.db().world);
    info!(
        "respawn: the player {:?} is rebuilt from its record ({} hands held, {} worn, {} carried, pools {}/{}); the old body goes",
        player.record.name,
        player.record.held_ids().count(),
        player.record.worn.len(),
        player.record.inventory.len(),
        player.record.pools.yellow,
        player.record.pools.red,
    );
    commands.insert_resource(SavedPlayer {
        record: player.record,
        items,
    });
    commands.insert_resource(RebuiltPlayer);
    for root in &roots {
        commands.entity(root).despawn();
    }
}

/// The next player spawn is the respawn's rebuild ([`rebuild_player`]):
/// the original spawns the player from the stripped record alone, so the
/// body holds what the record's hands hold (nothing after the strip) and
/// stands at the respawn portal from its first frame. The spawn takes the
/// marker (Exarust's carried torch is then not given, and the body starts
/// at the portal's placement instead of the level's own start) and removes
/// it.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RebuiltPlayer;

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

/// The save unit's system sets: the frame's save steps, and the quit's
/// autosave in `Last` (its gate, the arena match's forfeit, then the
/// writes).
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SaveLiveSystems {
    Steps,
    /// The story's level load takes the save's own section of the level
    /// ([`prefer_saved_section`]), after the teardown and before the host
    /// starts the load.
    SavedSection,
    QuitGate,
    QuitForfeit,
    QuitWrite,
}

/// The application is closing and the quit's autosave ran (the original's
/// quit hook). Once set, the frame's save steps run in `Last` instead of
/// `Update`, after the gate, so the quit's saves are written before the
/// app exits.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct QuitAutosave(pub bool);

/// Whether the quit's autosave has run.
pub fn quitting(quit: Option<Res<QuitAutosave>>) -> bool {
    quit.is_some_and(|quit| quit.0)
}

/// The quit hook: when the app is asked to close (the title's or pause
/// page's QUIT, the window's close) while a session plays, the session's
/// quit runs as the original's quit hook does ([`crate::flow::Flow::quit_session`]
/// with the menu's page): the story saves with the death rule; the arena
/// saves from the in-game page and forfeits from the arena's pause page
/// (the forfeit then saves the player data, [`SaveStep::ForfeitSave`]).
/// The closing app needs none of the quit's other steps.
fn quit_autosave(
    mut exits: MessageReader<AppExit>,
    mut quit: ResMut<QuitAutosave>,
    flow: Option<ResMut<crate::flow::Flow>>,
    menu: Option<Res<crate::menu_screen::MenuScreen>>,
    mut steps: MessageWriter<SaveStep>,
    mut forfeits: MessageWriter<crate::flow::ForfeitMatch>,
) {
    if exits.read().count() == 0 || quit.0 {
        return;
    }
    quit.0 = true;
    let Some(mut flow) = flow else {
        return;
    };
    // Without a menu form the page matches no arena row [the original
    // reads its menu form's page, and saves nothing without the form].
    let page = menu.map_or(crate::menu_pages::Page::Title, |menu| menu.page);
    let quit_steps = flow.quit_session(page);
    info!(
        "save: the app closes during {:?} ({:?}, page {page:?}): {quit_steps:?}",
        flow.mode, flow.phase
    );
    for step in quit_steps {
        match step {
            crate::flow::Step::Save { drop_on_low_health } => {
                steps.write(SaveStep::Save { drop_on_low_health });
            }
            crate::flow::Step::Forfeit => {
                forfeits.write(crate::flow::ForfeitMatch);
            }
            _ => {}
        }
    }
}

/// The running game's save and load.
pub struct SaveLivePlugin;

impl Plugin for SaveLivePlugin {
    fn build(&self, app: &mut App) {
        // The transition's migration pass runs just before a door's
        // teardown, while the followers' bodies still exist.
        let migrate = app.register_system(crate::follower_travel::collect_followers);
        app.init_resource::<exa_world::level_lifecycle::BeforeDoorTeardown>();
        app.world_mut()
            .resource_mut::<exa_world::level_lifecycle::BeforeDoorTeardown>()
            .0
            .push(migrate);
        app.add_message::<SaveToNewSlot>()
            .add_message::<StorySlotLoad>()
            .add_message::<SaveStep>()
            .add_message::<crate::destination::JourneySave>()
            .add_message::<exa_script::ScriptEffect>()
            .add_message::<crate::world_clock::RestoreWorldClock>()
            .init_resource::<ArenaStores>()
            .init_resource::<SaveCamera>()
            .init_resource::<SaveReport>()
            .init_resource::<QuitAutosave>()
            .add_message::<AppExit>()
            .add_message::<crate::flow::ForfeitMatch>()
            .init_resource::<crate::world_characters::WorldCharacters>()
            .init_resource::<crate::world_characters::LocalCharacters>()
            .init_resource::<exa_assets::live_objects::LiveObjects>()
            .add_message::<exa_world::level_lifecycle::LevelChangeRequest>()
            .add_systems(
                Update,
                (
                    make_slot_lists,
                    follow_the_view,
                    report_saves,
                    scripted_saves,
                    run_checkpoint_scripts,
                    journey_saves,
                    // Every save first saves the open screens.
                    crate::gui_screens::save_open_screens_for_saves::<false>,
                    save_to_new_slot,
                    run_save_steps::<false>.in_set(SaveLiveSystems::Steps),
                    load_arena_slot,
                    load_story_slot,
                    take_loaded_journal,
                )
                    .chain()
                    .run_if(resource_exists::<SaveFolder>),
            )
            // The steps a frame asks for are written before a level change
            // asked in the same frame tears the level down, and in the frame
            // the flow applies them (a respawn's save, then its rebuild, see
            // the old player and the strip's record).
            .configure_sets(
                Update,
                SaveLiveSystems::Steps
                    .after(crate::flow::FlowSystems::ApplySteps)
                    .before(exa_world::level_lifecycle::LevelChangeSystems::Teardown),
            )
            .configure_sets(
                Last,
                (
                    SaveLiveSystems::QuitGate,
                    SaveLiveSystems::QuitForfeit,
                    SaveLiveSystems::QuitWrite,
                )
                    .chain(),
            )
            .add_systems(
                Last,
                (
                    quit_autosave.in_set(SaveLiveSystems::QuitGate),
                    (
                        crate::gui_screens::save_open_screens_for_saves::<true>,
                        run_save_steps::<true>,
                    )
                        .chain()
                        .in_set(SaveLiveSystems::QuitWrite),
                )
                    .run_if(resource_exists::<SaveFolder>),
            )
            .add_systems(
                Update,
                post_loaded_player
                    .after(load_story_slot)
                    .run_if(resource_exists::<LoadedPlayerStream>)
                    .run_if(resource_exists::<crate::content_requests::ContentRequests>),
            )
            .add_systems(
                Update,
                sync_before_level_change
                    .before(exa_world::level_lifecycle::LevelChangeSystems::Teardown),
            )
            .add_message::<RebuildPlayer>()
            .add_systems(
                Update,
                rebuild_player
                    .after(SaveLiveSystems::Steps)
                    .before(exa_world::level_lifecycle::LevelChangeSystems::Teardown),
            )
            .init_resource::<crate::left_behind::LeftBody>()
            .configure_sets(
                Update,
                SaveLiveSystems::SavedSection
                    .after(exa_world::level_lifecycle::LevelChangeSystems::Teardown),
            )
            .add_systems(
                Update,
                (prefer_saved_section, crate::left_behind::forget_on_level_change)
                    .chain()
                    .in_set(SaveLiveSystems::SavedSection),
            )
            .add_systems(Update, crate::follower_travel::restore_arrivals)
            .add_systems(Update, crate::level_save::restore_saved_state)
            .init_resource::<crate::save_level::SceneContentLists>()
            .add_systems(Update, crate::save_level::track_scene_content)
            .add_systems(
                Update,
                crate::world_characters::track_local_characters
                    .before(crate::follower_travel::restore_arrivals),
            )
            .add_systems(
                Update,
                bench_the_manager.before(crate::arena_live::stand_first_round),
            );
    }
}

/// A spectated arena match's start: when no participant is the player's
/// to control (only hired employees fight), the original's match start
/// tears the scene's player body down with the old scene, spawns no new
/// one (the player body word stays 0), points the camera from the spawn
/// spot raised by [`crate::spectate::SPECTATE_CAMERA_OFFSET_CM`] at the
/// spot and starts the Spectate game mode; the hub's arrival builds the
/// manager again from his record.
///
/// Exarust's player survives the level change, so the bench does the
/// teardown once the arena is in: the manager's record is kept with his
/// possessions and health (his local items promoted first, as a save
/// does) as the next player spawn's record ([`SavedPlayer`]), and the
/// player is despawned. The session starts with the camera looking at the
/// spawn spot. [Exarust's: the chase camera keeps its own orbit around that
/// point instead of the original's fixed position.]
/// The controls' state the bench changes: the spectate session, the
/// controls' switch and the scene's game mode.
type BenchControls<'a> = (
    Option<ResMut<'a, crate::spectate::SpectateSession>>,
    Option<ResMut<'a, crate::player_controls::PlayerControl>>,
    Option<ResMut<'a, crate::player_controls::GameModes>>,
);

#[allow(clippy::too_many_arguments)]
fn bench_the_manager(
    mut commands: Commands,
    scene: Option<Res<crate::arena_live::ArenaScene>>,
    loading: Option<Res<exa_ui::LoadingStatus>>,
    levels: Query<&exa_world::LevelRoot>,
    building: Query<(), With<exa_physics::StaticGeometry>>,
    mut live: ResMut<exa_assets::live_objects::LiveObjects>,
    mut players: PlayerSaveQuery,
    roots: Query<Entity, With<crate::player::Player>>,
    created: Option<Res<crate::creator_screen::CreatedCharacter>>,
    (session, control, modes): BenchControls,
) {
    let (Some(scene), Some(mut session)) = (scene, session) else {
        return;
    };
    if scene.player_entry.is_some()
        || scene.placed
        || loading.is_some()
        || !building.is_empty()
        || roots.is_empty()
        || !levels
            .iter()
            .any(|level| crate::arena_match::arena_index(&level.name).is_some())
    {
        return;
    }
    sync_players(&mut live, &mut players);
    let Some(manager) = live_story_player(&players, created.as_deref(), &|_| None) else {
        return;
    };
    let items = world_items(&live.db().world);
    info!(
        "arena match: no entrant is the player's ({:?}); the manager {:?} is benched ({} world items kept) and the match is spectated",
        scene.entrants,
        manager.record.name,
        items.len()
    );
    commands.insert_resource(SavedPlayer {
        record: manager.record,
        items,
    });
    for root in &roots {
        commands.entity(root).despawn();
    }
    // The Spectate start's camera switch leaves the Character mode (its end
    // of play) and enters Spectate.
    if let Some(mut modes) = modes {
        crate::player_controls::enter_mode(
            &mut modes,
            Some(crate::player_controls::GameMode::Spectate),
            "the arena match is spectated",
        );
    }
    session.start_arena(Vec3::from_array(scene.spawn_at) / 100.0);
    // The Character mode's end, then the Spectate mode's start: the
    // controls end on.
    if let Some(mut control) = control {
        crate::controls_live::switch_controls(&mut control, true, "end of play");
        crate::controls_live::switch_controls(&mut control, false, "spectate starts");
    }
}

/// The level change's world sync: before the level goes, the surviving
/// player's local items are promoted to the world tier, as the original's
/// door transition does before it loads the destination, so they keep
/// their records when the next level's database replaces the local tier.
/// A change that carries the player's kit through the kit's temporary
/// database (the next practice fight, [`crate::save_kit::KitScratch`]) does
/// not promote: the equip after the load copies the items into the new
/// local tier, as the original's next fight does. [Exarust's: the other
/// changes after a world reset (an arena entry, a scripted jump) promote.]
fn sync_before_level_change(
    mut requests: MessageReader<exa_world::level_lifecycle::LevelChangeRequest>,
    mut live: ResMut<exa_assets::live_objects::LiveObjects>,
    mut players: PlayerSaveQuery,
    scratch: Option<Res<crate::save_kit::KitScratch>>,
) {
    if requests.read().filter(|r| r.level.is_some()).count() == 0 {
        return;
    }
    if scratch.as_ref().is_some_and(|scratch| scratch.db.is_some()) {
        info!("world sync: the player's kit travels in the kit's temporary database; no promotion");
        return;
    }
    sync_players(&mut live, &mut players);
}

/// The consumer of the scripts' `CheckpointSave` calls (the story's
/// checkpoint zone in `exanima01` runs one): a call made for the player
/// while the settings' checkpoints are on becomes the checkpoint step
/// ([`SaveStep::Checkpoint`]: the save, then its copy over the checkpoint
/// when the slot has none); any other call does nothing
/// ([`exa_formats::save_chunk::checkpoint_script_runs`]). Without the
/// settings resource checkpoints count as on, the options' default.
pub fn run_checkpoint_scripts(
    mut effects: MessageReader<exa_script::ScriptEffect>,
    config: Option<Res<crate::hw_config::HardwareConfig>>,
    mut steps: MessageWriter<SaveStep>,
) {
    let enabled = config.as_ref().is_none_or(|c| c.live.checkpoints);
    for effect in effects.read() {
        let exa_script::dialogue::Effect::CheckpointSave { player } = effect.0 else {
            continue;
        };
        if exa_formats::save_chunk::checkpoint_script_runs(player, enabled) {
            info!("save: the CheckpointSave script runs the checkpoint step");
            steps.write(SaveStep::Checkpoint);
        } else {
            info!(
                "save: CheckpointSave ignored (for the player: {player}, checkpoints on: {enabled})"
            );
        }
    }
}

/// The managers are made once the folder is known; they scan lazily. As
/// the game's start-up makes each manager it runs the legacy pass over the
/// manager's saves ([`remove_legacy_saves`]): the story's, then the arena's.
fn make_slot_lists(mut commands: Commands, folder: Res<SaveFolder>, lists: Option<Res<SlotLists>>) {
    if lists.is_none() {
        let lists = SlotLists::new(&folder.0);
        remove_legacy_saves(&lists);
        commands.insert_resource(lists);
    }
}

/// The start-up legacy pass of both managers, story first: every save whose
/// player data signature is below its mode's current one is deleted
/// ([`Slots::remove_legacy`]). A pass that fails is logged and the start-up
/// goes on, as the original's handler reports "Error removing legacy
/// saves." and continues. Returns how many saves each pass removed.
pub fn remove_legacy_saves(lists: &SlotLists) -> [usize; 2] {
    let mut removed = [0; 2];
    for (i, list) in [&lists.story, &lists.arena].into_iter().enumerate() {
        match list.slots().remove_legacy() {
            Ok(count) => {
                removed[i] = count;
                info!(
                    "save: legacy pass over the {} saves removed {count}",
                    list.slots().prefix()
                );
            }
            Err(error) => warn!("Error removing legacy saves. ({error})"),
        }
    }
    removed
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
    /// The running powers the placement saves with the player
    /// ([`crate::power_save::saved_powers`]).
    pub powers: Vec<exa_formats::characters::RunningPower>,
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

/// A record with the character's live roles written back, as the
/// original's role list lives in the record itself: a role added while
/// the level ran (the follow a recruit takes) is saved with the record,
/// and the spawn reads it back ([`exa_ai::roles::Roles::of_record`]).
/// Only a version 7 record keeps its list as held; an older one is left
/// alone (its roles are rebuilt from the list at every spawn).
pub fn record_with_roles(
    mut record: CharacterRecord,
    roles: &exa_ai::roles::Roles,
) -> CharacterRecord {
    if record.version >= exa_formats::characters::RECORD_VERSION {
        record.roles = roles
            .0
            .iter()
            .map(|r| exa_formats::characters::Role {
                id: r.id,
                weight: r.weight,
                explicit: r.explicit,
                inherited: r.inherited,
                variables: r.values.clone(),
            })
            .collect();
    }
    record
}

/// What a fighting body's health is read from: its fighter, state flags,
/// condition and caster.
pub type FightingBody<'a> = (
    &'a exa_combat::Fighter,
    Option<exa_combat::StateFlags>,
    Option<exa_combat::Condition>,
    Option<&'a crate::thaum::cast::PowerCaster>,
);

/// A scene character's record as the save's flush stores it: the body's
/// record with its live possessions written back and, for a body that
/// fights, its health ([`body_health`], [`record_with_health`]), and its
/// live roles ([`record_with_roles`]); the door's follower flush stores the
/// same.
pub fn scene_character_record(
    data: &exa_character::record::CharacterData,
    possessions: Option<&exa_items::possession::Possessions>,
    body: Option<FightingBody<'_>>,
    roles: Option<&exa_ai::roles::Roles>,
) -> CharacterRecord {
    let mut record = data.record.clone();
    if let Some(possessions) = possessions {
        record = record_with_possessions(record, possessions);
    }
    if let Some(roles) = roles {
        record = record_with_roles(record, roles);
    }
    if let Some((fighter, flags, condition, caster)) = body {
        let focus = caster.map_or(record_focus(&record), |caster| caster.body.focus);
        let health = body_health(
            fighter,
            flags.unwrap_or_default(),
            condition.unwrap_or_default(),
            focus,
        );
        record = record_with_health(record, health);
    }
    record
}

/// A level-placed character's pools loaded from its record, as the
/// original's spawn builder for imported characters writes them after the
/// body is built: yellow and red as the record's shares of the fighter's
/// maximum ([`exa_formats::characters::Pools::restore`], in double
/// precision). The focus, the heal pool and a potion are not loaded there.
pub fn restore_placed_pools(record: &CharacterRecord, fighter: &mut exa_combat::Fighter) {
    let max = fighter.health.max;
    fighter.health.red = exa_formats::characters::Pools::restore(record.pools.red, max);
    fighter.health.yellow = exa_formats::characters::Pools::restore(record.pools.yellow, max);
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
/// save's `#worldchrdb`, the live object database's world tier is exported
/// as its `#worldobjdb` (none when the tier is empty), and the session's
/// placement becomes the player's
/// (the record reference with the world bit, the position with its height
/// as the floor, the heading, a living condition and an AI that saves no
/// home or pose [guess: the player's controller writes neither]).
pub fn story_player_world(
    player: &StoryPlayer,
    session: &mut Session,
    objects: &exa_formats::objdb::ObjectDb,
) -> exa_formats::Result<crate::slots::WorldExport> {
    story_player_world_with(player, session, objects, &[])
}

/// [`story_player_world`] with the live world character database's other
/// records (`others`, each under its world id: the followers the doors
/// registered, [`crate::world_characters::WorldCharacters::others`]) kept
/// in the exported store beside the player, as the original's save writes
/// its whole world database.
pub fn story_player_world_with(
    player: &StoryPlayer,
    session: &mut Session,
    objects: &exa_formats::objdb::ObjectDb,
    others: &[CharacterRecord],
) -> exa_formats::Result<crate::slots::WorldExport> {
    let mut db = exa_formats::character_db::CharacterDb::new(exa_formats::character_db::WORLD_TAG);
    for other in others {
        let id = other.id & exa_formats::characters::ID_MASK;
        if id != PLAYER_RECORD_ID {
            let _ = db.add(id, other.clone());
        }
    }
    let mut record = player.record.clone();
    // The save's reader finds the player by the low id the placement names.
    record.id = PLAYER_RECORD_ID;
    let _ = db.add(PLAYER_RECORD_ID, record);
    let export = crate::slots::export_world_databases(&mut db, objects)?;
    let body = exa_formats::rfc_write::ExportCharacter {
        record: exa_formats::characters::WORLD_DB | PLAYER_RECORD_ID,
        body_position: player.position_cm,
        floor_height: player.position_cm[1],
        heading: player.heading,
        condition: 0,
        ai: Some(exa_formats::rfc_write::ExportAi::default()),
        // Each running power's own record, after the AI's chunks.
        attachments: player
            .powers
            .iter()
            .map(|power| power.bytes().map(Some))
            .collect::<exa_formats::Result<_>>()?,
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
        Option<&'static mut exa_character::record::CharacterData>,
        Option<&'static mut exa_items::possession::Possessions>,
        PlayerHealthParts,
    ),
    With<crate::player::PlayerBody>,
>;

/// The story's world sync on the player's possessions, as the original
/// runs it before a save and before a level change: the link remap table
/// on, the carried items promoted to the world tier
/// ([`exa_formats::objdb::ObjectDb::promote_carried`]: worn list, out
/// hands, stowed hands, then the pack with the hands' repeats following),
/// then the local doors' links rewritten through the table and the table
/// off. The possessions and the body's record take the new ids without
/// counting as a change (the items are the same; nothing is re-dressed).
/// Returns how many ids changed. The travelling followers' items follow
/// ([`promote_followers`]).
pub fn promote_player_items(
    db: &mut exa_formats::objdb::ObjectDb,
    possessions: &mut exa_items::possession::Possessions,
    data: Option<&mut exa_character::record::CharacterData>,
) -> usize {
    let mut record = record_with_possessions(CharacterRecord::default(), possessions);
    let before = record.clone();
    db.set_link_remaps(true);
    db.promote_carried(&mut record);
    let doors = db.apply_link_remaps();
    let changed = before
        .hands
        .iter()
        .zip(&record.hands)
        .filter(|(a, b)| a.id != b.id)
        .count()
        + before
            .worn
            .iter()
            .zip(&record.worn)
            .filter(|(a, b)| a.id != b.id)
            .count()
        + before
            .inventory
            .iter()
            .zip(&record.inventory)
            .filter(|(a, b)| a.item != b.item)
            .count();
    if changed == 0 {
        return 0;
    }
    crate::save_kit::mirror_kit(&record, possessions);
    if let Some(data) = data {
        data.record = record_with_possessions(data.record.clone(), possessions);
    }
    info!("world sync: {changed} carried item ids promoted to the world tier ({doors} door links)");
    for (old, new) in before
        .inventory
        .iter()
        .zip(&record.inventory)
        .map(|(a, b)| (a.item, b.item))
        .chain(
            before
                .worn
                .iter()
                .zip(&record.worn)
                .map(|(a, b)| (a.id, b.id)),
        )
        .chain(
            before
                .hands
                .iter()
                .zip(&record.hands)
                .map(|(a, b)| (a.id, b.id)),
        )
        .filter(|(old, new)| old != new)
    {
        let stored = db.stored_condition(new).unwrap_or_default();
        info!(
            "world sync: {old:#x} -> {new:#x}: apparel dirt {:?} wear {:?}, weapon dirt {:?} wear {:?}",
            stored.apparel_dirt, stored.apparel_wear, stored.weapon_dirt, stored.weapon_wear
        );
    }
    changed
}

/// The travelling followers' share of the story's world sync, which the
/// original runs right after the player's inside the same link remap
/// window: every character the migration list takes along (controller word
/// 4 without flag `0x800`, each once, [`crate::save_level::migrations`]) has
/// its carried items promoted as the player's are
/// ([`exa_formats::objdb::ObjectDb::promote_carried`]). `records` holds the
/// scene characters' records in the motiles' order. Returns how many
/// followers were promoted. [Exarust's: the window is the followers' own,
/// since the player's promotion closes its window itself.] A door's level
/// change calls it from the migration pass
/// ([`crate::follower_travel::collect_followers`]).
pub fn promote_followers(
    db: &mut exa_formats::objdb::ObjectDb,
    motiles: &[crate::save_level::SceneMotile<'_>],
    records: &mut [CharacterRecord],
) -> usize {
    let travelling = crate::save_level::migrations(motiles);
    db.set_link_remaps(true);
    let mut promoted = 0;
    for migration in &travelling.travelling {
        let Some(at) = motiles
            .iter()
            .position(|motile| motile.address == migration.address)
        else {
            continue;
        };
        if let Some(record) = records.get_mut(at) {
            db.promote_carried(record);
            promoted += 1;
        }
    }
    db.apply_link_remaps();
    promoted
}

/// [`promote_player_items`] on the player bodies of a save query.
fn sync_players(live: &mut exa_assets::live_objects::LiveObjects, players: &mut PlayerSaveQuery) {
    for (_, mut data, possessions, _) in players.iter_mut() {
        let Some(mut possessions) = possessions else {
            continue;
        };
        if !record_has_local(&possessions) {
            continue;
        }
        let data = data.as_mut().map(|data| data.bypass_change_detection());
        promote_player_items(live.db_mut(), possessions.bypass_change_detection(), data);
    }
}

/// The arena save's world sync, as its save runs the promotion pass: the id
/// remap table on; the manager's record promoted whole
/// ([`exa_formats::objdb::ObjectDb::promote_record`]: the hands, worn list,
/// the six per-rank loadouts and the pack), the body's possessions and the
/// career's loadouts taking the new ids; then the stash, the arsenal's
/// boxes and every employee's record and list ([`promote_arena_stores`]);
/// then the scene's entity ids rewritten through the table; the table off.
fn promote_arena_items(
    db: &mut exa_formats::objdb::ObjectDb,
    players: &mut PlayerSaveQuery,
    career: &mut crate::hub_windows::ArenaCareer,
    stores: &mut ArenaStores,
    scene: &mut crate::level_save::LevelSaveScene,
) {
    db.set_remaps(true);
    promote_manager_items(db, players, career);
    let stored = promote_arena_stores(db, stores);
    let entities = scene.rewrite_entity_ids(db);
    db.set_remaps(false);
    let arsenal: Vec<u32> = stores
        .arsenal
        .shelves
        .iter()
        .flatten()
        .flatten()
        .map(|entry| entry.item)
        .collect();
    info!(
        "world sync: the arena's stash, arsenal and staff promoted ({stored} ids changed), {entities} scene entity ids rewritten; stash {:#x?}, arsenal {:#x?}",
        stores.stash, arsenal
    );
}

/// The arena save's promotion of the stores beside the manager, with the
/// id remap table already on: every stash id, every entry of the arsenal's
/// five shelves of five boxes, then each employee's record (its two hands,
/// worn items and own items, promoted as a record is) and its list (the
/// kit). Returns how many ids changed.
pub fn promote_arena_stores(
    db: &mut exa_formats::objdb::ObjectDb,
    stores: &mut ArenaStores,
) -> usize {
    let mut changed = 0;
    let mut promote = |db: &mut exa_formats::objdb::ObjectDb, id: &mut u32| {
        let new = db.promote(*id);
        if new != *id {
            changed += 1;
            *id = new;
        }
    };
    for id in &mut stores.stash {
        promote(db, id);
    }
    for shelf in &mut stores.arsenal.shelves {
        for entries in shelf {
            for entry in entries {
                promote(db, &mut entry.item);
            }
        }
    }
    for employee in &mut stores.staff.employees {
        // The record: the hands, the worn list, then the pack, as the
        // record promotion walks it.
        for hand in &mut employee.hands {
            promote(db, hand);
        }
        for (worn, _) in &mut employee.worn {
            promote(db, worn);
        }
        for carried in &mut employee.carried {
            promote(db, carried);
        }
        // The list.
        for item in &mut employee.kit {
            promote(db, item);
        }
    }
    changed
}

/// The manager's share of [`promote_arena_items`], with the id remap
/// table already on.
fn promote_manager_items(
    db: &mut exa_formats::objdb::ObjectDb,
    players: &mut PlayerSaveQuery,
    career: &mut crate::hub_windows::ArenaCareer,
) {
    for (_, mut data, possessions, _) in players.iter_mut() {
        let Some(mut possessions) = possessions else {
            continue;
        };
        let mut record = record_with_possessions(CharacterRecord::default(), &possessions);
        record.loadouts = career.player.loadouts.to_vec();
        let before = record.clone();
        db.promote_record(&mut record);
        if record == before {
            continue;
        }
        crate::save_kit::mirror_kit(&record, possessions.bypass_change_detection());
        for (slot, loadout) in career.player.loadouts.iter_mut().zip(&record.loadouts) {
            *slot = loadout.clone();
        }
        if let Some(data) = data.as_mut() {
            let data = data.bypass_change_detection();
            data.record = record_with_possessions(data.record.clone(), &possessions);
        }
        info!("world sync: the manager's items promoted to the world tier");
    }
}

/// The save's world object export, for the log.
fn log_world_export(export: &crate::slots::WorldExport) {
    if let Some(bytes) = &export.characters {
        let names: Vec<String> = exa_formats::character_db::CharacterDb::load(
            exa_formats::character_db::WORLD_TAG,
            bytes,
        )
        .map(|mut db| {
            let ids: Vec<u32> = db.ids().collect();
            ids.into_iter()
                .filter_map(|id| {
                    let shared = db.get(id)?;
                    let record = shared.record();
                    Some(format!(
                        "{:#x} {} (level {})",
                        record.id, record.name, record.locale
                    ))
                })
                .collect()
        })
        .unwrap_or_default();
        info!("save: #worldchrdb exported: {names:?}");
    }
    match &export.objects {
        Some((objects, strings)) => info!(
            "save: #worldobjdb exported ({} record bytes, {} string bytes)",
            objects.len(),
            strings.len()
        ),
        None => info!("save: the world tier is empty; no #worldobjdb"),
    }
}

/// Whether any of the possessions names a local-tier object.
fn record_has_local(possessions: &exa_items::possession::Possessions) -> bool {
    let local = |id: u32| id & exa_formats::objdb::TAG_MASK == exa_formats::objdb::LOCAL;
    possessions
        .out
        .iter()
        .chain(&possessions.stowed)
        .any(|h| local(h.id))
        || possessions.worn.iter().any(|w| local(w.id))
        || possessions.carried.iter().any(|c| local(c.id))
}

/// What the save reads a player body's health from.
type PlayerHealthParts = (
    Option<&'static exa_combat::Fighter>,
    Option<&'static exa_combat::StateFlags>,
    Option<&'static exa_combat::Condition>,
    Option<&'static crate::thaum::cast::PowerCaster>,
    Option<&'static crate::mind::casts::MindBody>,
);

/// The focus a record holds, for a body without a caster.
pub(crate) fn record_focus(record: &CharacterRecord) -> f32 {
    exa_formats::characters::Pools::restore_focus(record.pools.focus)
}

/// `record_of` names the character a body id carries, for the running
/// Loyalty and Command Dead the save writes with the player.
fn live_story_player(
    players: &PlayerSaveQuery,
    created: Option<&crate::creator_screen::CreatedCharacter>,
    record_of: &dyn Fn(crate::mind::casts::BodyId) -> Option<u32>,
) -> Option<StoryPlayer> {
    live_story_player_left(players, created, record_of, None)
}

/// [`live_story_player`] after the reconstruction device's strip: the
/// player is the stripped record the flow kept ([`crate::left_behind`]),
/// written as it is (the dead body's possessions and health are the
/// left-behind copy's), at the body's place.
fn live_story_player_left(
    players: &PlayerSaveQuery,
    created: Option<&crate::creator_screen::CreatedCharacter>,
    record_of: &dyn Fn(crate::mind::casts::BodyId) -> Option<u32>,
    left: Option<&crate::left_behind::LeftBody>,
) -> Option<StoryPlayer> {
    let (transform, data, possessions, (fighter, flags, condition, caster, mind)) =
        players.iter().next()?;
    let powers = crate::power_save::saved_powers(mind, caster, record_of);
    if !powers.is_empty() {
        info!(
            "save: the player's running powers {:x?} are saved with the placement",
            powers.iter().map(|power| power.id).collect::<Vec<_>>()
        );
    }
    if let Some(stripped) = left.and_then(|left| left.player_record()) {
        info!(
            "save: the stripped player record is written; the dead body keeps record {:#x}",
            data.map_or(0, |data| data.record.id)
        );
        let (yaw, _, _) = transform.rotation().to_euler(EulerRot::YXZ);
        return Some(StoryPlayer {
            record: stripped.clone(),
            position_cm: (transform.translation() * 100.0).to_array(),
            heading: yaw,
            powers,
        });
    }
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
        powers,
    })
}

/// The story's session words a save carries from the flow: the start
/// background (the header's low byte) and the memory fragments seen (the
/// memory-flags chunk, left out when zero).
pub fn stamp_story(session: &mut Session, flow: Option<&crate::flow::Flow>) {
    let Some(flow) = flow else {
        return;
    };
    session.start_background = flow.story.background.index();
    session.memory_flags = (flow.story.memory != 0).then_some(flow.story.memory);
}

/// The story's discovery chunk and its global-experience snapshot, from the
/// live experience once the level-start hook has run. A story that has not
/// started leaves the session's copies alone.
pub fn stamp_discovery(
    session: &mut Session,
    experience: Option<&crate::experience_live::StoryExperience>,
) {
    let Some(experience) = experience.filter(|experience| experience.started) else {
        return;
    };
    session.discovery = Some(experience.discovery());
    session.global_experience = Some(experience.state.global);
}

/// Puts a loaded story's discovery chunk and global-experience snapshot onto
/// the live experience. A chunk that does not parse is left unused. The
/// account value from the settings is replaced when the save carried one.
pub fn apply_loaded_discovery(
    live: &mut crate::experience_live::StoryExperience,
    session: &Session,
) {
    if let Some(bytes) = &session.discovery {
        match crate::experience::read_discovery(bytes) {
            Ok(found) => live.state.restore(found),
            Err(error) => warn!("save: discovery data does not restore: {error:?}"),
        }
    }
    if let Some(global) = session.global_experience {
        live.state.global = global;
    }
}

/// The story save's character flush, ahead of its sections: every scene
/// character's record takes its live possessions and health in its store
/// ([`crate::level_save::LevelSaveScene::flush_characters`]) and the live
/// player's record becomes the world store's player slot
/// ([`crate::world_characters::WorldCharacters::refresh_player`]). Returns
/// the world store's other records (the save's `#worldchrdb` beside the
/// player) and where the scene's characters stand.
fn flush_story_characters(
    mut world: Option<&mut crate::world_characters::WorldCharacters>,
    player: Option<&StoryPlayer>,
    scene: &mut crate::level_save::LevelSaveScene,
    left_body: Option<(CharacterRecord, bool)>,
) -> (Vec<CharacterRecord>, Vec<crate::level_save::LiveCharacter>) {
    let mut characters = scene.flush_characters(world.as_deref_mut());
    // The dead player's body, its owned mark cleared, is flushed as one of
    // the scene's characters: its record (the left-behind copy) into the
    // world store, its body into the level section.
    if let Some((record, dead)) = left_body
        && let Some(body) = scene.left_character(record.id, dead)
    {
        if let Some(world) = world.as_deref_mut()
            && record.id & exa_formats::characters::WORLD_DB != 0
        {
            world.store(&record);
        }
        info!(
            "save: the dead player's body (record {:#x}, dead {dead}) is written into the level at {:?}",
            record.id, body.position
        );
        characters.push(body);
    }
    let Some(world) = world else {
        return (Vec::new(), characters);
    };
    if let Some(player) = player {
        world.refresh_player(&player.record);
    }
    (world.others(), characters)
}

/// The dead player's body as the story save's flush stores it once its
/// owned mark is cleared ([`crate::left_behind::LeftBody::exported`]): the
/// body's record (the left-behind copy) with its live possessions written
/// back, and whether it is dead. Its pools and roles are the copy's.
fn left_body_record(
    players: &PlayerSaveQuery,
    left: Option<&crate::left_behind::LeftBody>,
) -> Option<(CharacterRecord, bool)> {
    left.filter(|left| left.exported)?;
    let (_, data, possessions, (_, _, condition, _, _)) = players.iter().next()?;
    let mut record = data?.record.clone();
    if let Some(possessions) = possessions {
        record = record_with_possessions(record, possessions);
    }
    Some((record, condition.is_some_and(|c| c.is_dead())))
}

/// The live session with the story player's record and placement: the
/// sections it writes besides the player data.
fn story_save_world(
    session: &mut Session,
    player: Option<&StoryPlayer>,
    objects: &exa_formats::objdb::ObjectDb,
    others: &[CharacterRecord],
) -> crate::slots::WorldExport {
    let Some(player) = player else {
        return crate::slots::WorldExport::default();
    };
    match story_player_world_with(player, session, objects, others) {
        Ok(export) => export,
        Err(error) => {
            warn!("save: the player's record does not export: {error}");
            crate::slots::WorldExport::default()
        }
    }
}

/// The hub's manager as the arena save writes him: the live body's
/// record with the career's six per-rank loadouts, each the rank's hands
/// and worn items.
pub fn arena_manager(
    mut player: StoryPlayer,
    career: &crate::hub_windows::ArenaCareer,
) -> StoryPlayer {
    player.record.loadouts = career.player.loadouts.to_vec();
    player
}

/// A loaded arena save's manager record gives the career its six per-rank
/// loadouts back; a record with fewer leaves the missing ranks as they
/// are. Returns how many were taken.
pub fn restore_manager_loadouts(
    record: &CharacterRecord,
    career: &mut crate::hub_windows::ArenaCareer,
) -> usize {
    let mut taken = 0;
    for (slot, loadout) in career.player.loadouts.iter_mut().zip(&record.loadouts) {
        *slot = loadout.clone();
        taken += 1;
    }
    taken
}

#[allow(clippy::too_many_arguments)]
fn save_to_new_slot(
    mut requests: MessageReader<SaveToNewSlot>,
    lists: Option<ResMut<SlotLists>>,
    levels: Query<&exa_world::LevelRoot>,
    clock: Option<Res<exa_combat::remedy::WorldClock>>,
    ticks: Option<Res<exa_physics::body::BodyTicks>>,
    mut career: Option<ResMut<crate::hub_windows::ArenaCareer>>,
    mut stores: ResMut<ArenaStores>,
    camera: Option<Res<SaveCamera>>,
    transition: Option<Res<crate::level_transition::LiveTransition>>,
    mut players: PlayerSaveQuery,
    created: Option<Res<crate::creator_screen::CreatedCharacter>>,
    mut report: ResMut<SaveReport>,
    flow: Option<Res<crate::flow::Flow>>,
    experience: Option<Res<crate::experience_live::StoryExperience>>,
    (mut live, mut world, screens): (
        ResMut<exa_assets::live_objects::LiveObjects>,
        Option<ResMut<crate::world_characters::WorldCharacters>>,
        Option<Res<crate::gui_screens::GameScreens>>,
    ),
    mut scene: crate::level_save::LevelSaveScene,
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
    let arena = level.is_some_and(crate::arena_hub_live::is_hub_level);
    // The save's world sync: the arena promotes the manager's whole record
    // with the id remap table on, the story the player's carried items.
    if arena {
        if let Some(career) = career.as_mut() {
            promote_arena_items(live.db_mut(), &mut players, career, &mut stores, &mut scene);
        }
    } else {
        sync_players(&mut live, &mut players);
    }
    let fallback = crate::hub_windows::ArenaCareer::default();
    let career = career.as_deref().unwrap_or(&fallback);
    let mut session = live_session(level, seconds, career, &stores);
    camera
        .as_deref()
        .copied()
        .unwrap_or_default()
        .stamp(&mut session);
    stamp_journal(&mut session, screens.as_deref());
    let mut characters: Option<Vec<crate::level_save::LiveCharacter>> = None;
    let (name, export) = match session.mode {
        Mode::Arena => {
            // The manager's record goes into the world character database
            // with his six per-rank kits, as the arena's save writes every
            // fighter's record.
            let player = live_story_player(&players, created.as_deref(), &|_| None)
                .map(|player| arena_manager(player, career));
            let export = story_save_world(&mut session, player.as_ref(), live.db(), &[]);
            (Some(career.player.name.clone()), export)
        }
        Mode::Story => {
            stamp_story(&mut session, flow.as_deref());
            stamp_discovery(&mut session, experience.as_deref());
            let records = scene.record_ids();
            let left = scene.left().cloned();
            let player = live_story_player_left(
                &players,
                created.as_deref(),
                &|id| {
                    records
                        .iter()
                        .find(|(body, _)| *body == id)
                        .map(|(_, record)| *record)
                },
                left.as_ref(),
            );
            let (others, flushed) = flush_story_characters(
                world.as_deref_mut(),
                player.as_ref(),
                &mut scene,
                left_body_record(&players, left.as_ref()),
            );
            characters = Some(flushed);
            let export = story_save_world(&mut session, player.as_ref(), live.db(), &others);
            (player.map(|p| p.record.name), export)
        }
    };
    log_world_export(&export);
    // The current level under its locale name, after the world databases.
    let named = world.as_deref_mut().map(|w| w.others()).unwrap_or_default();
    let section = scene.section(
        live.db(),
        session.mode == Mode::Story,
        characters.as_deref().map(|live| (live, named.as_slice())),
    );
    let world = export.save_world(
        section
            .as_ref()
            .map(|(name, bytes)| (name.as_str(), bytes.as_slice())),
        false,
    );
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

/// The manager a save step writes through: the original's save unit
/// writes the current manager, which the mode switch sets (the story's in
/// story mode, the arena's in arena mode). Without a flow the story's
/// [Exarust's: a test or debug app]; a hub level outside the story counts
/// as the arena [Exarust's: a hub launched straight from the command line
/// keeps the title's mode]. Practice and the scripted modes have none.
fn save_mode(flow: Option<&crate::flow::Flow>, level: Option<&str>) -> Option<Mode> {
    let hub = level.is_some_and(crate::arena_hub_live::is_hub_level);
    match flow.map(|flow| flow.mode) {
        None | Some(crate::flow::Mode::Story) => Some(Mode::Story),
        Some(crate::flow::Mode::Arena) => Some(Mode::Arena),
        Some(_) if hub => Some(Mode::Arena),
        Some(_) => None,
    }
}

/// The live journal a save writes as its journal chunk: the in-game
/// journal's pages (the screens' copy, which the open journal stored into
/// first, [`crate::gui_screens::save_open_screens_for_saves`]). Without the
/// screens the session's own journal stays.
pub fn stamp_journal(session: &mut Session, screens: Option<&crate::gui_screens::GameScreens>) {
    let Some(screens) = screens else {
        return;
    };
    let entries: Vec<exa_script::story::Entry> = screens
        .journal
        .entries
        .iter()
        .map(|entry| exa_script::story::Entry {
            id: entry.id,
            title: entry.title.clone(),
            text: entry.text.clone(),
        })
        .collect();
    if let Some(journal) = exa_script::story::Journal::from_entries(entries) {
        *session.story.journal_mut() = journal;
    }
}

/// A loaded save's journal as the in-game journal holds it: the reader's
/// pages (one "New Entry" page when the save had none), the first page
/// selected [guess: the reader's selection is not traced]; the window's
/// place is kept.
pub fn apply_loaded_journal(journal: &mut exa_ui::game_gui::Journal, session: &Session) {
    journal.entries = session
        .story
        .journal()
        .entries()
        .iter()
        .map(|entry| exa_ui::game_gui::JournalEntry {
            id: entry.id,
            title: entry.title.clone(),
            text: entry.text.clone(),
        })
        .collect();
    journal.ensure_entry();
    journal.current = 0;
}

/// The story load hands its journal to the in-game journal once the load
/// put the loaded session up ([`LoadedStory`]).
fn take_loaded_journal(
    loaded: Option<Res<LoadedStory>>,
    screens: Option<ResMut<crate::gui_screens::GameScreens>>,
) {
    let (Some(loaded), Some(mut screens)) = (loaded, screens) else {
        return;
    };
    if !loaded.is_changed() {
        return;
    }
    apply_loaded_journal(&mut screens.journal, &loaded.0);
    info!(
        "save: the loaded journal's {} pages go into the in-game journal",
        screens.journal.entries.len()
    );
}

/// The arena's live session with the manager, as the arena's save writes
/// it: the company and its tables and the manager's record (his six
/// per-rank kits) with his placement, and the world databases. The world
/// sync runs first when `promote` is set (the full save's promotion pass;
/// the forfeit's player-data save runs none).
#[allow(clippy::too_many_arguments)]
fn arena_save_session(
    seconds: u32,
    camera: SaveCamera,
    career: Option<&mut crate::hub_windows::ArenaCareer>,
    stores: &mut ArenaStores,
    live: &mut exa_assets::live_objects::LiveObjects,
    players: &mut PlayerSaveQuery,
    created: Option<&crate::creator_screen::CreatedCharacter>,
    scene: &mut crate::level_save::LevelSaveScene,
    promote: bool,
) -> (Session, crate::slots::WorldExport) {
    let fallback = crate::hub_windows::ArenaCareer::default();
    let career: &crate::hub_windows::ArenaCareer = match career {
        Some(career) => {
            if promote {
                promote_arena_items(live.db_mut(), players, career, stores, scene);
            }
            career
        }
        None => &fallback,
    };
    let mut session = Session::career(career.company.name.clone());
    session.company = career.company.clone();
    session.capture_arena(career, &stores.staff, &stores.arsenal, &stores.stash);
    session.clock = seconds;
    camera.stamp(&mut session);
    let player =
        live_story_player(players, created, &|_| None).map(|player| arena_manager(player, career));
    let export = story_save_world(&mut session, player.as_ref(), live.db(), &[]);
    (session, export)
}

/// The clocks a save step stamps and the quit's state.
type SaveClocks<'w> = (
    Option<Res<'w, exa_combat::remedy::WorldClock>>,
    Option<Res<'w, exa_physics::body::BodyTicks>>,
    Option<Res<'w, QuitAutosave>>,
);

/// The flow's save steps on the current slot of the mode's manager
/// ([`save_mode`]).
///
/// * A new slot ([`SaveStep::NewSlot`], a new story's or career's start)
///   is made in the manager ([`SlotList::create`]) under the created
///   character's name and becomes the current slot.
/// * A save writes the live session: the story's with the player's record
///   and placement, the arena's with the company's tables and the manager
///   (its promotion pass first). With the death rule a dead player's save
///   is dropped instead.
/// * A drop deletes the save file; without a checkpoint the slot leaves
///   the list.
/// * A checkpoint (the story's) copies the save over it.
/// * A save and checkpoint ([`SaveStep::SaveAndCheckpoint`], the adapter
///   journey's end) saves, then copies the save over the story's
///   checkpoint whether one exists or not.
/// * A transition save ([`SaveStep::TransitionSave`], a story level exit
///   that keeps a checkpoint) does the same when the save holds neither
///   `exanima04` nor the destination, else saves plainly.
/// * The forfeit's save ([`SaveStep::ForfeitSave`], the arena match quit)
///   writes only the arena's player data, under the hub's locale.
///
/// Nothing but the new slot runs without a current slot.
///
/// Two instances run: the frame's (`AT_QUIT` false, in `Update`) and the
/// quit's (`AT_QUIT` true, in `Last` after the quit's gate). Each reads
/// every frame, so neither sees a step twice, and only the one for the
/// app's state writes.
#[allow(clippy::too_many_arguments)]
fn run_save_steps<const AT_QUIT: bool>(
    mut commands: Commands,
    mut steps: MessageReader<SaveStep>,
    lists: Option<ResMut<SlotLists>>,
    flow: Option<Res<crate::flow::Flow>>,
    levels: Query<&exa_world::LevelRoot>,
    (clock, ticks, quit): SaveClocks,
    camera: Option<Res<SaveCamera>>,
    transition: Option<Res<crate::level_transition::LiveTransition>>,
    mut players: PlayerSaveQuery,
    fighters: Query<&exa_combat::Fighter, With<crate::player::PlayerBody>>,
    created: Option<Res<crate::creator_screen::CreatedCharacter>>,
    experience: Option<Res<crate::experience_live::StoryExperience>>,
    mut report: ResMut<SaveReport>,
    (mut live, mut world, screens): (
        ResMut<exa_assets::live_objects::LiveObjects>,
        Option<ResMut<crate::world_characters::WorldCharacters>>,
        Option<Res<crate::gui_screens::GameScreens>>,
    ),
    (mut career, mut stores): (
        Option<ResMut<crate::hub_windows::ArenaCareer>>,
        ResMut<ArenaStores>,
    ),
    mut scene: crate::level_save::LevelSaveScene,
) {
    let steps: Vec<SaveStep> = steps.read().copied().collect();
    if steps.is_empty() || quitting(quit) != AT_QUIT {
        return;
    }
    let Some(mut lists) = lists else {
        return;
    };
    let level = levels.iter().next().map(|root| root.name.clone());
    let Some(mode) = save_mode(flow.as_deref(), level.as_deref()) else {
        debug!("save: no save manager in this mode for {steps:?}");
        return;
    };
    let story = mode == Mode::Story;
    let list = lists.of_mode(mode);
    let disabled = transition.as_ref().is_some_and(|t| t.saves_disabled());
    let health = fighters
        .iter()
        .next()
        .map_or(1.0, |fighter| fighter.health.red);
    let now = ticks.as_ref().map_or(0, |ticks| ticks.millis() as u32);
    let seconds = clock
        .as_ref()
        .map_or(0, |clock| crate::world_clock::stamped(clock, now));
    let camera = camera.as_deref().copied().unwrap_or_default();
    // The story's session at this tick, before its world sync.
    let story_session = || {
        let mut session = Session::story();
        session.story = story_at(level.as_deref().unwrap_or(""));
        session.clock = seconds;
        camera.stamp(&mut session);
        stamp_story(&mut session, flow.as_deref());
        stamp_discovery(&mut session, experience.as_deref());
        stamp_journal(&mut session, screens.as_deref());
        session
    };
    for step in steps {
        if step == SaveStep::NewSlot {
            // The original names the slot after the current player record:
            // the created character at a new game's start.
            let name = created
                .as_deref()
                .and_then(|created| created.0.as_ref())
                .map(|record| record.name.clone())
                .or_else(|| {
                    live_story_player(&players, created.as_deref(), &|_| None)
                        .map(|p| p.record.name)
                });
            match list.create(name.as_deref()) {
                Ok(number) => info!(
                    "save: new {mode:?} game: slot {number} created for {name:?} and made current"
                ),
                Err(error) => warn!("save: cannot create the new game's slot: {error}"),
            }
            continue;
        }
        if list.current() <= 0 {
            debug!("save: no current {mode:?} slot for {step:?}");
            continue;
        }
        let number = list.current() as u32;
        match step {
            SaveStep::NewSlot => {}
            SaveStep::Save { .. }
            | SaveStep::SaveAndCheckpoint
            | SaveStep::TransitionSave { .. } => {
                let drop_on_low_health = matches!(
                    step,
                    SaveStep::Save {
                        drop_on_low_health: true
                    }
                );
                // The transition save keeps a checkpoint only for a save
                // that holds neither `exanima04` nor the destination (the
                // section lookup on the slot's package, read before this
                // save writes it).
                let fresh = match step {
                    SaveStep::TransitionSave { locale, level_time } if story => {
                        let slots = list.slots();
                        let held = slots.has_saved_level(number, "exanima04")
                            || slots.has_saved_level(number, locale);
                        info!(
                            "save: the exit to {locale} {} checkpoint (the save {} exanima04 or {locale})",
                            if held { "keeps no" } else { "keeps a" },
                            if held { "holds" } else { "holds neither" }
                        );
                        if !held && level_time {
                            commands.insert_resource(LevelTime(now));
                        }
                        !held
                    }
                    _ => false,
                };
                // The journey's and a fresh transition's save copy the
                // slot's save over the checkpoint after it, the disabled
                // save included.
                let copy =
                    ((step == SaveStep::SaveAndCheckpoint || fresh) && story).then_some(number);
                let copy_over = |list: &SlotList| {
                    if let Some(number) = copy {
                        match list.slots().copy_save_over_checkpoint(number) {
                            Ok(()) => {
                                info!("save: story slot {number}'s save copied over its checkpoint")
                            }
                            Err(error) => {
                                warn!(
                                    "save: cannot copy slot {number}'s save over its checkpoint: {error}"
                                )
                            }
                        }
                    }
                };
                if disabled {
                    warn!("save: saving is disabled after the last level change; nothing written");
                    copy_over(list);
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
                let (session, export, section) = if story {
                    let mut session = story_session();
                    sync_players(&mut live, &mut players);
                    let records = scene.record_ids();
                    let left = scene.left().cloned();
                    let player = live_story_player_left(
                        &players,
                        created.as_deref(),
                        &|id| {
                            records
                                .iter()
                                .find(|(body, _)| *body == id)
                                .map(|(_, record)| *record)
                        },
                        left.as_ref(),
                    );
                    let (others, characters) = flush_story_characters(
                        world.as_deref_mut(),
                        player.as_ref(),
                        &mut scene,
                        left_body_record(&players, left.as_ref()),
                    );
                    let export =
                        story_save_world(&mut session, player.as_ref(), live.db(), &others);
                    let section = scene.section(live.db(), true, Some((&characters, &others)));
                    (session, export, section)
                } else {
                    let (mut session, export) = arena_save_session(
                        seconds,
                        camera,
                        career.as_deref_mut(),
                        &mut stores,
                        &mut live,
                        &mut players,
                        created.as_deref(),
                        &mut scene,
                        true,
                    );
                    stamp_journal(&mut session, screens.as_deref());
                    let section = scene.section(live.db(), false, None);
                    (session, export, section)
                };
                log_world_export(&export);
                let level_section = section
                    .as_ref()
                    .map(|(name, bytes)| (name.as_str(), bytes.as_slice()));
                match list.save_current(&session, &export.save_world(level_section, false)) {
                    Ok(_) => {
                        info!(
                            "save: {mode:?} slot {number} saved (level {level:?}, clock {seconds} s)"
                        );
                        report.saved = Some(number);
                    }
                    Err(error) => warn!("save: cannot write {mode:?} slot {number}: {error}"),
                }
                copy_over(list);
            }
            SaveStep::Drop => match list.drop_after_death(0.0) {
                Ok(dropped) => info!(
                    "save: {mode:?} slot {number}'s save is dropped (slot left the list: {dropped})"
                ),
                Err(error) => warn!("save: cannot drop slot {number}: {error}"),
            },
            SaveStep::Checkpoint if !story => {
                debug!("save: the arena keeps no checkpoint");
            }
            SaveStep::Checkpoint => {
                // The original's checkpoint routine saves first (with the
                // writer's disable check) and only then copies the save over
                // the checkpoint; an existing checkpoint is left alone.
                let mut session = story_session();
                // The checkpoint's pre-save world sync, as for a save.
                if !disabled {
                    sync_players(&mut live, &mut players);
                }
                let records = scene.record_ids();
                let left = scene.left().cloned();
                let player = live_story_player_left(
                    &players,
                    created.as_deref(),
                    &|id| {
                        records
                            .iter()
                            .find(|(body, _)| *body == id)
                            .map(|(_, record)| *record)
                    },
                    left.as_ref(),
                );
                let (others, characters) = flush_story_characters(
                    world.as_deref_mut(),
                    player.as_ref(),
                    &mut scene,
                    left_body_record(&players, left.as_ref()),
                );
                let export = story_save_world(&mut session, player.as_ref(), live.db(), &others);
                // The disable flag skips the save but not the copy: the
                // old save becomes the checkpoint, as in the original.
                let section = (!disabled)
                    .then(|| scene.section(live.db(), true, Some((&characters, &others))))
                    .flatten();
                let world = export.save_world(
                    section
                        .as_ref()
                        .map(|(name, bytes)| (name.as_str(), bytes.as_slice())),
                    disabled,
                );
                match list.slots().checkpoint_full(number, &session, &world) {
                    Ok(true) => info!("save: story slot {number}'s save kept as its checkpoint"),
                    Ok(false) => {
                        info!("save: story slot {number} already has a checkpoint; it stays")
                    }
                    Err(error) => warn!("save: cannot write slot {number}'s checkpoint: {error}"),
                }
            }
            SaveStep::ForfeitSave if story => {
                debug!("save: the forfeit's save is the arena's");
            }
            SaveStep::ForfeitSave => {
                // The forfeit's save writes the player data alone, with the
                // hub's locale (the arena writer's only locale), and no
                // world sync, world databases or level section.
                let (mut session, _) = arena_save_session(
                    seconds,
                    camera,
                    career.as_deref_mut(),
                    &mut stores,
                    &mut live,
                    &mut players,
                    created.as_deref(),
                    &mut scene,
                    false,
                );
                stamp_journal(&mut session, screens.as_deref());
                match list.save_current(&session, &SaveWorld::default()) {
                    Ok(_) => {
                        info!("save: arena slot {number}'s player data saved after the forfeit");
                        report.saved = Some(number);
                    }
                    Err(error) => warn!("save: cannot write arena slot {number}: {error}"),
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
    /// The content-stream request the reset posts for the loaded player
    /// ([`loaded_player_request`]); `None` without a player record.
    pub request: Option<exa_assets::content_stream::CharacterRequest>,
}

/// The classification of the save's items: the shipped object database's
/// records and the save's world tier, by id.
pub fn save_item_kinds(
    base: &[exa_formats::objdb::Item],
    loaded: &LoadedSave,
) -> HashMap<u32, ItemKind> {
    let mut kinds = kinds_from_items(base);
    if let Some(world) = loaded.world_objects() {
        kinds.extend(kinds_from_items(&world_items(&world.world)));
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
    let kits = player
        .as_ref()
        .map_or(0, |record| restore_manager_loadouts(record, career));
    if kits > 0 {
        info!(
            "save: the manager's {kits} per-rank kits are restored: {:?}",
            career
                .player
                .loadouts
                .iter()
                .map(|l| (
                    l.hands.map(|h| h.id),
                    l.worn.iter().map(|w| w.id).collect::<Vec<_>>()
                ))
                .collect::<Vec<_>>()
        );
    }
    // The identity reset, then its request to the content stream.
    let request = player.as_mut().map(|record| {
        record.reset_loaded_identity();
        // The hub's scene is not built from the save: no scene key yet.
        loaded_player_request(0)
    });
    ArenaRestore {
        applied,
        shelved,
        clock,
        player,
        request,
    }
}

/// The arena load's id pass over the live stores, as the original runs it
/// after the arena chunk and before the continue fixups: every arsenal
/// entry's id goes through `map_item` (the load instances an untagged base
/// record with the instance bit into the local tier,
/// [`exa_formats::objdb::ObjectDb::instance_of`]); an entry whose id changed
/// has its holders remapped by its flags
/// ([`crate::arena_ids::remap_arsenal_ids`]): `0x400` the stash and the
/// manager's per-rank loadouts, else the first employee whose list (his
/// kit) holds it and that employee's loadouts; `0x200` the manager's four
/// hands and carried items, then each employee's two hands and own items.
/// The manager's hands and pack are the hub body's possessions (the out
/// set, then the stowed one); employee `i`'s loadouts are the roster's
/// member `i`. Returns how many arsenal entries took a new id.
pub fn remap_loaded_items(
    stores: &mut ArenaStores,
    career: &mut crate::hub_windows::ArenaCareer,
    body: Option<&mut exa_items::possession::Possessions>,
    mut map_item: impl FnMut(u32) -> u32,
) -> usize {
    use exa_items::possession::Hand;
    let hand_ref = |hand: &Hand| ItemRef {
        id: hand.id,
        flags: hand.flags,
    };
    let carried = |ids: &[u32]| -> Vec<Carried> {
        ids.iter()
            .map(|&item| Carried {
                item,
                flags: 0,
                extra: 0,
            })
            .collect()
    };
    let mut manager = CharacterRecord::new();
    manager.loadouts = career.player.loadouts.to_vec();
    if let Some(body) = body.as_deref() {
        manager.hands = [
            hand_ref(&body.out[0]),
            hand_ref(&body.out[1]),
            hand_ref(&body.stowed[0]),
            hand_ref(&body.stowed[1]),
        ];
        let ids: Vec<u32> = body.carried.iter().map(|c| c.id).collect();
        manager.inventory = carried(&ids);
    }
    let mut employees: Vec<crate::arena_ids::EmployeeRefs> = stores
        .staff
        .employees
        .iter()
        .enumerate()
        .map(|(i, employee)| {
            let mut record = CharacterRecord::new();
            if let Some(member) = career.company.roster.get(i) {
                record.loadouts = member.loadouts.to_vec();
            }
            record.hands[0].id = employee.hands[0];
            record.hands[1].id = employee.hands[1];
            record.inventory = carried(&employee.carried);
            crate::arena_ids::EmployeeRefs {
                record,
                items: employee
                    .kit
                    .iter()
                    .map(|&id| ItemEntry {
                        id,
                        flags: 0,
                        extra: crate::arena_ids::LIST_ENTRY_EXTRA,
                    })
                    .collect(),
            }
        })
        .collect();
    let mut changed = 0;
    crate::arena_ids::remap_arsenal_ids(
        &mut stores.arsenal,
        &mut stores.stash,
        &mut manager,
        &mut employees,
        |id| {
            let new = map_item(id);
            if new != id {
                changed += 1;
            }
            new
        },
    );
    if changed == 0 {
        return 0;
    }
    for (loadout, remapped) in career.player.loadouts.iter_mut().zip(manager.loadouts) {
        *loadout = remapped;
    }
    if let Some(body) = body {
        for (hand, remapped) in body
            .out
            .iter_mut()
            .chain(body.stowed.iter_mut())
            .zip(manager.hands)
        {
            hand.id = remapped.id;
        }
        for (item, remapped) in body.carried.iter_mut().zip(&manager.inventory) {
            item.id = remapped.item;
        }
    }
    for (i, (employee, refs)) in stores.staff.employees.iter_mut().zip(employees).enumerate() {
        employee.hands = [refs.record.hands[0].id, refs.record.hands[1].id];
        for (item, remapped) in employee.carried.iter_mut().zip(&refs.record.inventory) {
            *item = remapped.item;
        }
        for (item, remapped) in employee.kit.iter_mut().zip(&refs.items) {
            *item = remapped.id;
        }
        if let Some(member) = career.company.roster.get_mut(i) {
            for (loadout, remapped) in member.loadouts.iter_mut().zip(refs.record.loadouts) {
                *loadout = remapped;
            }
        }
    }
    changed
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
    mut live: ResMut<exa_assets::live_objects::LiveObjects>,
    mut players: Query<&mut exa_items::possession::Possessions, With<crate::player::PlayerBody>>,
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
    import_world_tier(
        &mut live,
        game.as_deref(),
        crate::slots::world_object_tier(&loaded.package),
    );
    let restored = restore_arena(&loaded, &kinds, career, &mut stores);
    // The arena's id pass: base records the arsenal names become local
    // instances, their holders following.
    let remapped = {
        let db = live.db_mut();
        remap_loaded_items(
            &mut stores,
            career,
            players.iter_mut().next().map(|p| p.into_inner()),
            |id| db.instance_of(id, exa_formats::objdb::Tier::Local),
        )
    };
    info!("save: the arena's id pass gave {remapped} arsenal entries new instances");
    // The continued career's fixups follow the read.
    let fixups = crate::arena_session::continue_career(career);
    info!(
        "save: the continued career's fixups ran (tournament generated {}, unfit dropped {})",
        fixups.generate_tournament, fixups.drop_unfit
    );
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
    let arsenal: Vec<u32> = stores
        .arsenal
        .shelves
        .iter()
        .flatten()
        .flatten()
        .map(|entry| entry.item)
        .collect();
    info!(
        "save: the loaded stash holds {:#x?}; the arsenal's boxes hold {:#x?}",
        stores.stash, arsenal
    );
    if let Some(stream) = restored.request {
        info!(
            "save: the loaded player is requested anew (request word {:#x}: no AI start, no saved AI pose)",
            stream.flags
        );
    }
}

/// The story's level loads read the level from the save when the save
/// holds it, as the original's transition (a door, a journey) and its
/// respawn load do: the current story slot's save is opened, and a section
/// named by the destination's locale is imported in place of the archive's
/// `<locale>.rfc` ([`exa_world::SavedLevelSection`], used once by the host's
/// load). A save without the section (a level not visited yet, a new
/// story's fresh slot) leaves the archive's; a section a save load already
/// handed is left alone. In the arena only the settled match's return to
/// the hub does the same, from the current arena slot's `arenahub` section
/// (the arena's saves export the scene they leave under its locale); its
/// other loads are the arena unit's.
///
/// Evidence (REA, Exanima.exe, the assembly): the transition opens the save
/// package when none is open, looks the destination's section up by name
/// and imports it, the archive's file only when that failed; the respawn's
/// level load (the save-or-archive loader) the same, and the match return
/// loads the hub through that loader with its save flag set.
pub fn prefer_saved_section(
    mut commands: Commands,
    ready: Option<Res<exa_world::level_lifecycle::LevelChangeReady>>,
    mut seen: Local<u64>,
    flow: Option<Res<crate::flow::Flow>>,
    lists: Option<Res<SlotLists>>,
    saved: Option<Res<exa_world::SavedLevelSection>>,
    hub: Option<Res<crate::arena_hub_live::ArenaHub>>,
) {
    let Some(ready) = ready else {
        return;
    };
    if ready.changes == *seen {
        return;
    }
    *seen = ready.changes;
    let Some(level) = ready.level.as_deref() else {
        return;
    };
    let arena = flow
        .as_ref()
        .is_some_and(|flow| flow.mode == crate::flow::Mode::Arena);
    if arena {
        // The settled match's return: the hub, with the return placement
        // the settle wrote.
        let returning = hub.as_ref().is_some_and(|hub| hub.returned.is_some());
        if !(returning && crate::arena_hub_live::is_hub_level(level)) {
            return;
        }
    } else if flow.is_some_and(|flow| flow.mode != crate::flow::Mode::Story) {
        return;
    }
    if saved.is_some_and(|saved| saved.is_for(level)) {
        return;
    }
    let Some(lists) = lists else {
        return;
    };
    let list = if arena { &lists.arena } else { &lists.story };
    let Some(section) = saved_level_section(list, level) else {
        debug!("level change: the save holds no {level} section; the archive's is read");
        return;
    };
    info!(
        "level change: {} comes from the save's own section ({} bytes)",
        section.locale,
        section.bytes.len()
    );
    commands.insert_resource(section);
}

/// The current slot's save section for `level` (with or without `.rfc`),
/// named by its locale; `None` without a current slot, a save file or the
/// section. Nothing is created or written.
pub fn saved_level_section(list: &SlotList, level: &str) -> Option<exa_world::SavedLevelSection> {
    if list.current() <= 0 {
        return None;
    }
    let file = list
        .slots()
        .file(list.current() as u32, save_package::SlotKind::Save);
    if !file.is_file() {
        return None;
    }
    let locale = level
        .strip_suffix(".rfc")
        .unwrap_or(level)
        .to_ascii_lowercase();
    let package = Slots::open(&file).ok()?;
    let bytes = package.section(&save_package::name(&locale))?;
    Some(exa_world::SavedLevelSection {
        locale,
        bytes: bytes.to_vec().into(),
    })
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
    /// The save's world object records, under their world-tagged ids.
    pub items: Vec<exa_formats::objdb::Item>,
    /// The save's world tier and its own library, for the live database's
    /// import; `None` when the save has no `#worldobjdb` (the load then
    /// clears the live tier).
    pub world: Option<(exa_formats::objdb::Store, exa_formats::objstrings::Library)>,
    /// Which file was read.
    pub kind: save_package::SlotKind,
    /// The content-stream request the identity reset posts for the loaded
    /// player ([`loaded_player_request`]); `None` without a player record.
    pub request: Option<exa_assets::content_stream::CharacterRequest>,
    /// The save's own section for the level, named by the load's locale,
    /// which the level load imports instead of the archive's file; `None`
    /// when the package holds none.
    pub level_section: Option<exa_world::SavedLevelSection>,
    /// The save's world character database in its save form, which the
    /// load imports as the live store
    /// ([`crate::world_characters::WorldCharacters::from_save`]); `None`
    /// when the package holds none.
    pub world_characters: Option<Vec<u8>>,
    /// The running powers the player's placement saved, which the new
    /// player body rebuilds ([`crate::power_save`]).
    pub powers: Vec<exa_formats::characters::RunningPower>,
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
    // The identity reset, then its request to the content stream: the
    // character goes into the world scene.
    let request = player.as_mut().map(|record| {
        record.reset_loaded_identity();
        loaded_player_request(crate::content_requests::WORLD_SCENE)
    });
    let saved_placement = loaded.data.placement().and_then(Result::ok);
    let placement = saved_placement.as_ref().map(placement_of);
    // The record build hands the placement's running powers to the body; a
    // chunk that does not read rebuilds none.
    let powers = match saved_placement.as_ref().map(|p| p.running_powers()) {
        Some(Ok(powers)) => powers,
        Some(Err(error)) => {
            warn!("save: the player's running powers do not read ({error}); none are rebuilt");
            Vec::new()
        }
        None => Vec::new(),
    };
    let world = crate::slots::world_object_tier(&loaded.package);
    let items = world
        .as_ref()
        .map(|(tier, _)| world_items(tier))
        .unwrap_or_default();
    let level_section =
        loaded
            .package
            .section(&loaded.level_locale)
            .map(|bytes| exa_world::SavedLevelSection {
                locale: locale.clone(),
                bytes: std::sync::Arc::from(bytes),
            });
    let world_characters = loaded.world_characters().map(|db| db.save());
    Ok(StoryRestore {
        session,
        level,
        player,
        placement,
        items,
        world,
        kind: loaded.kind,
        request,
        level_section,
        world_characters,
        powers,
    })
}

/// A world tier's records decoded as items under their world-tagged ids.
pub fn world_items(tier: &exa_formats::objdb::Store) -> Vec<exa_formats::objdb::Item> {
    tier.records()
        .filter_map(|record| {
            let mut item = record.item().ok()?;
            item.id = record.id | exa_formats::objdb::WORLD;
            Some(item)
        })
        .collect()
}

/// The save load's object database import into the live database: the
/// world tier and its library replaced by the save's (cleared without
/// one), then migrated from their templates, as the original's load does.
pub fn import_world_tier(
    live: &mut exa_assets::live_objects::LiveObjects,
    game: Option<&exa_assets::Game>,
    world: Option<(exa_formats::objdb::Store, exa_formats::objstrings::Library)>,
) {
    if let Some(game) = game {
        live.ensure_base(&game.0);
    }
    let records = world.as_ref().map_or(0, |(tier, _)| tier.len());
    let migrated = live.db_mut().import_world(world);
    info!(
        "save: world tier imported ({records} records, {migrated} visited by the template migration)"
    );
    for record in live.db().world.records() {
        let id = record.id | exa_formats::objdb::WORLD;
        if let Some(stored) = live.db().stored_condition(id) {
            info!(
                "save: world record {id:#x} {:?}: apparel dirt {:?} wear {:?}, weapon dirt {:?} wear {:?}",
                record.header().map(|h| h.name).unwrap_or_default(),
                stored.apparel_dirt,
                stored.apparel_wear,
                stored.weapon_dirt,
                stored.weapon_wear
            );
        }
    }
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
    (mut live, game): (
        ResMut<exa_assets::live_objects::LiveObjects>,
        Option<Res<exa_assets::Game>>,
    ),
) {
    let Some(request) = requests.read().last().copied() else {
        return;
    };
    let Some(mut lists) = lists else {
        return;
    };
    let restored = restore_story(&mut lists.story, request.number, request.prefer_checkpoint);
    let mut respawn_pending = false;
    let (level, placement) = match restored {
        Ok(mut restored) => {
            // A save written while a respawn was pending: the original's
            // load turns the state to 1 and, once the level is in, puts the
            // player at the level's portal (15 cm up) to settle there.
            respawn_pending = take_pending_respawn(&mut restored.session);
            import_world_tier(&mut live, game.as_deref(), restored.world.take());
            // The save's world character database becomes the live store.
            let characters = restored.world_characters.take().and_then(|bytes| {
                exa_formats::character_db::CharacterDb::load(
                    exa_formats::character_db::WORLD_TAG,
                    &bytes,
                )
                .ok()
            });
            let mut store = crate::world_characters::WorldCharacters::from_save(characters);
            info!(
                "save: world character database imported ({} records)",
                store.records().len()
            );
            commands.insert_resource(store);
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
            // The level is read from the save's own section when it has one.
            match restored.level_section.take() {
                Some(section) => {
                    info!(
                        "save: the level {} comes from the save's own section ({} bytes)",
                        section.locale,
                        section.bytes.len()
                    );
                    commands.insert_resource(section);
                }
                None => {
                    info!("save: the save has no section for its level; the archive's is read");
                    commands.remove_resource::<exa_world::SavedLevelSection>();
                }
            }
            if let (Some(stream), Some(record)) = (restored.request, restored.player.as_ref()) {
                info!(
                    "save: the loaded player is requested anew (request word {:#x}: no AI start, no saved AI pose)",
                    stream.flags
                );
                // The request goes to the content stream's character ring
                // once its body's job is prepared; the level's player
                // spawn fetches it.
                commands.insert_resource(LoadedPlayerStream {
                    request: stream,
                    record: record.clone(),
                    posted: None,
                });
            } else {
                commands.remove_resource::<LoadedPlayerStream>();
            }
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
            // The story's session words come back: the start background
            // and the fragments seen.
            if let Some(flow) = flow.as_mut() {
                flow.story.background =
                    crate::flow::Background::from_index(restored.session.start_background);
                flow.story.memory = restored.session.memory_flags.unwrap_or(0);
            }
            // The new player body rebuilds the powers that ran when it was
            // saved.
            if restored.powers.is_empty() {
                commands.remove_resource::<crate::power_save::PendingPlayerPowers>();
            } else {
                info!(
                    "save: the player's running powers {:x?} come back with the body",
                    restored.powers.iter().map(|p| p.id).collect::<Vec<_>>()
                );
                commands.insert_resource(crate::power_save::PendingPlayerPowers(
                    restored.powers.clone(),
                ));
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
    if respawn_pending {
        info!(
            "save: the save holds a pending respawn: the level loads with the session-start word; \
             a portal it names places the player 15 cm up and settles it {} ticks, else the saved position stays",
            exa_physics::body::PLACED_SETTLE
        );
    }
    let mut resumed = flow.continue_story(level, at, viewport);
    if respawn_pending {
        portal_after_placement(&mut resumed);
    }
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
        // the player) starts at the level's own start, which no exit
        // holds; the nudge goes with the placement it moves.
        resumed.retain(|step| {
            !matches!(
                step,
                crate::flow::Step::PlacePlayer(_)
                    | crate::flow::Step::SettlePlayer(_)
                    | crate::flow::Step::ExitNudge
                    | crate::flow::Step::Camera(_)
            )
        });
    }
    steps.steps.extend(flow.continue_session(resumed));
}

/// The adapter journey's save: a journey that ended with its traveller
/// alive asks for a save copied over the checkpoint, as the selection
/// screen's travel step calls the save unit's save-and-checkpoint. Only the
/// player's journey saves [Exarust's: the original's journey is always the
/// player's].
fn journey_saves(
    mut journeys: MessageReader<crate::destination::JourneySave>,
    players: Query<(), With<crate::player::PlayerBody>>,
    mut steps: MessageWriter<SaveStep>,
) {
    for journey in journeys.read() {
        if players.contains(journey.user) {
            info!("save: the journey ended; the game saves and refreshes its checkpoint");
            steps.write(SaveStep::SaveAndCheckpoint);
        }
    }
}

/// Takes a pending respawn off a loaded session: when its transition word
/// is [`crate::session::RESPAWN_PENDING`] it becomes 1, as the original's
/// load writes it, and the load re-places the player at the portal.
pub fn take_pending_respawn(session: &mut Session) -> bool {
    if session.transition_word() != crate::session::RESPAWN_PENDING {
        return false;
    }
    if let Some(state) = session.transition.as_mut() {
        state[..4].copy_from_slice(&1u32.to_le_bytes());
    }
    true
}

/// Puts a pending respawn's portal placement after the load's placement
/// step (or after its level load when it has none): once the level is in,
/// a respawn portal the load names replaces the saved placement and gives
/// the player 16 ticks of settling (`exa_physics::body::PLACED_SETTLE`),
/// as the original's load does; without a portal the saved placement
/// stays and nothing settles.
pub fn portal_after_placement(steps: &mut Vec<crate::flow::Step>) {
    let after = steps
        .iter()
        .position(|step| matches!(step, crate::flow::Step::PlacePlayer(_)))
        .or_else(|| {
            steps
                .iter()
                .position(|step| matches!(step, crate::flow::Step::LoadLevel { .. }))
        });
    let at = after.map_or(steps.len(), |at| at + 1);
    steps.insert(
        at,
        crate::flow::Step::PlaceAtPortal(crate::level_change::PortalArrival {
            without: None,
            settle: exa_physics::body::PLACED_SETTLE,
        }),
    );
}

/// The session the last story load restored (its story position, journal,
/// clock and experience state), for the units that take it over.
#[derive(Resource, Clone, Debug, PartialEq)]
pub struct LoadedStory(pub Session);

#[cfg(test)]
mod tests {
    use super::*;

    /// F564: the respawn's rebuild hands the next spawn the record and
    /// marks it as the rebuild, so the spawn gives no carried torch and
    /// starts the body at the respawn placement; the old root goes.
    #[test]
    fn the_rebuild_marks_the_next_spawn() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<exa_assets::live_objects::LiveObjects>()
            .add_message::<RebuildPlayer>()
            .add_systems(Update, rebuild_player);
        let mut record = CharacterRecord::reset();
        record.name = "Ash".into();
        let root = app.world_mut().spawn(crate::player::Player).id();
        app.world_mut().spawn((
            GlobalTransform::default(),
            exa_character::record::CharacterData::of_record(record),
            crate::player::PlayerBody::new(0.0),
        ));
        app.update();
        assert!(app.world().get_resource::<RebuiltPlayer>().is_none());
        app.world_mut().write_message(RebuildPlayer);
        app.update();
        assert!(app.world().get_resource::<RebuiltPlayer>().is_some());
        let saved = app.world().resource::<SavedPlayer>();
        assert_eq!(saved.record.name, "Ash");
        assert!(saved.record.held_ids().next().is_none());
        assert!(app.world().get_entity(root).is_err());
    }

    /// The live roles replace a version 7 record's list; an older record
    /// keeps its own.
    #[test]
    fn the_flush_writes_the_live_roles_into_the_record() {
        let roles = exa_ai::roles::Roles(vec![exa_formats::roles::HeldRole {
            id: 31,
            weight: 0.5,
            explicit: true,
            inherited: 0,
            values: vec![7],
        }]);
        let record = record_with_roles(CharacterRecord::reset(), &roles);
        assert_eq!(record.roles.len(), 1);
        assert_eq!(
            (
                record.roles[0].id,
                record.roles[0].weight,
                record.roles[0].explicit
            ),
            (31, 0.5, true)
        );
        assert_eq!(record.roles[0].variables, [7]);
        let mut old = CharacterRecord::reset();
        old.version = 6;
        assert!(record_with_roles(old, &roles).roles.is_empty());
    }
    use crate::arsenal::{SHIELDS, WEAPONS};
    use exa_formats::save_package::SlotKind;

    fn folder(name: &str) -> PathBuf {
        let path =
            std::env::temp_dir().join(format!("exarust-save-live-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).unwrap();
        path
    }

    /// A story save carries the flow's start background and a non-zero
    /// fragment word; a zero word is left out.
    #[test]
    fn a_story_save_carries_the_background_and_the_fragments() {
        let mut flow = crate::flow::Flow::default();
        flow.story.background = crate::flow::Background::Proctor;
        flow.story.memory = 0x0003;
        let mut session = Session::story();
        stamp_story(&mut session, Some(&flow));
        assert_eq!(session.start_background, 2);
        assert_eq!(session.memory_flags, Some(3));
        flow.story.memory = 0;
        stamp_story(&mut session, Some(&flow));
        assert_eq!(session.memory_flags, None);
    }

    /// A started story's save carries the item kinds it has seen and the
    /// account experience; a load puts both back. A story that has not
    /// started does not replace them.
    #[test]
    fn a_story_save_carries_discovery_and_a_load_restores_it() {
        use crate::experience_live::StoryExperience;

        let mut live = StoryExperience {
            started: true,
            ..Default::default()
        };
        live.state.seen = vec![77];
        live.state.global = 40;
        let mut session = Session::story();
        stamp_discovery(&mut session, Some(&live));
        assert_eq!(session.global_experience, Some(40));
        let mut restored = StoryExperience::default();
        restored.state.global = 1;
        apply_loaded_discovery(&mut restored, &session);
        assert_eq!(restored.state.seen, [77]);
        assert_eq!(restored.state.global, 40);
        live.started = false;
        session.discovery = None;
        session.global_experience = None;
        stamp_discovery(&mut session, Some(&live));
        assert!(session.discovery.is_none());
        assert_eq!(session.global_experience, None);
    }

    #[test]
    fn the_story_saves_at_the_loaded_campaign_level() {
        assert_eq!(story_at("exanima01").level(), "exanima01");
        let third = exa_script::story::Story::begin().advance().to_string();
        assert_eq!(story_at(&format!("{third}.rfc")).level(), third);
        assert_eq!(story_at("arena1").index(), 0);
    }

    /// The start-up pass deletes each manager's saves older than its mode's
    /// signature, story and arena, and leaves current ones.
    #[test]
    fn the_start_up_legacy_pass_runs_for_both_managers() {
        use exa_formats::save_package::PackageImage;
        let dir = folder("legacy");
        let story = Slots::story(&dir);
        let arena = Slots::arena(&dir);
        let current = story.create(&Session::story()).unwrap();
        let old = |slots: &Slots, signature: u32| {
            let number = slots.create(&Session::story()).unwrap();
            let mut package = PackageImage::new();
            package.put_section(
                exa_formats::save_package::PLAYER_DATA,
                &signature.to_le_bytes(),
            );
            std::fs::write(slots.file(number, SlotKind::Save), package.to_bytes()).unwrap();
            number
        };
        let old_story = old(&story, story.signature() - 1);
        let old_arena = old(&arena, arena.signature() - 1);
        let lists = SlotLists::new(&dir);
        assert_eq!(remove_legacy_saves(&lists), [1, 1]);
        assert!(story.file(current, SlotKind::Save).exists());
        assert!(!story.file(old_story, SlotKind::Save).exists());
        assert!(!arena.file(old_arena, SlotKind::Save).exists());
        let _ = std::fs::remove_dir_all(&dir);
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
    fn the_hub_save_keeps_the_managers_six_kits_and_its_load_restores_them() {
        let dir = folder("arena-kits");
        let mut lists = SlotLists::new(&dir);
        let mut career = crate::hub_windows::ArenaCareer::default();
        career.player.loadouts[0].hands[0] = ItemRef::new(7, 0);
        career.player.loadouts[2].hands[1] = ItemRef::new(8, 0);
        career.player.loadouts[2].worn = vec![ItemRef::new(9, 0)];
        career.player.loadouts[5].word = 3;
        let stores = ArenaStores::default();
        let mut session = live_session(Some("arenahub.rfc"), 5, &career, &stores);
        let player = arena_manager(
            StoryPlayer {
                record: CharacterRecord::reset(),
                position_cm: [0.0; 3],
                heading: 0.0,
                powers: Vec::new(),
            },
            &career,
        );
        assert_eq!(player.record.loadouts.len(), 6);
        let export = story_player_world(
            &player,
            &mut session,
            &exa_formats::objdb::ObjectDb::default(),
        )
        .unwrap();
        save_new_slot(
            &mut lists.arena,
            &session,
            Some("Manager"),
            &export.save_world(None, false),
        )
        .unwrap()
        .unwrap();
        let mut list = SlotList::new(Slots::arena(&dir));
        assert!(list.any());
        list.select(0);
        let loaded = list.load_current(false).unwrap();
        let mut back = crate::hub_windows::ArenaCareer::default();
        let mut live = ArenaStores::default();
        let restored = restore_arena(&loaded, &HashMap::new(), &mut back, &mut live);
        assert!(restored.player.is_some());
        assert_eq!(back.player.loadouts, career.player.loadouts);
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
            powers: Vec::new(),
        };
        let career = crate::hub_windows::ArenaCareer::default();
        let mut session = live_session(Some("exanima02"), 4321, &career, &ArenaStores::default());
        let export = story_player_world(
            &player,
            &mut session,
            &exa_formats::objdb::ObjectDb::default(),
        )
        .unwrap();
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
        // Only the save (kind 1) is there.
        assert_eq!(found.availability, 1);
        assert_eq!(list.current(), number as i32);
        assert_eq!(found.loaded.unwrap().player_record().unwrap().name, "Ash");
        let restored = restore_story(&mut list, number, true).unwrap();
        // The identity reset posts the loaded player's request: no AI
        // start, no saved AI pose.
        assert_eq!(
            restored.request.map(|r| r.flags),
            restored
                .player
                .as_ref()
                .map(|_| exa_assets::content_stream::NO_AI_START)
        );
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

    /// A section's bytes: tag, size (body + 8), body.
    fn section(tag: u32, body: &[u8]) -> Vec<u8> {
        let mut out = tag.to_le_bytes().to_vec();
        out.extend((body.len() as u32 + 8).to_le_bytes());
        out.extend_from_slice(body);
        out
    }

    /// A record with a header (name, flags) and an apparel body of the
    /// given dirt and wear.
    fn apparel_record(name: &str, flags: u32, dirt: u8, wear: u8) -> Vec<u8> {
        let mut header = name.as_bytes().to_vec();
        header.resize(16, 0);
        header.extend(flags.to_le_bytes());
        header.extend(0u32.to_le_bytes());
        let mut body = vec![0u8; 0x224];
        body[0x1B1] = wear;
        body[0x1B2] = dirt;
        let mut out = section(0x1000_0000, &header);
        out.extend(section(0x1000, &body));
        out
    }

    /// The live database end to end: a kit's copy of a marked template
    /// takes the first id the level's own records leave free, its bytes are
    /// written there, the save's world sync moves it to the world tier and
    /// the pack names it there, the save exports the tier, and the load
    /// brings the record back with its dirt and wear.
    /// The arena save's pass promotes the stash, every arsenal box and each
    /// employee's record and list, with the id remap table on: an item
    /// named twice takes the same world id.
    #[test]
    fn the_arena_save_promotes_the_stash_the_arsenal_and_the_staff() {
        use exa_formats::objdb::{LOCAL, ObjectDb, Store, WORLD};
        let records: Vec<Vec<u8>> = (1..=5)
            .map(|id| apparel_record(&format!("Item{id}"), 0, 0, 0))
            .collect();
        let mut db = ObjectDb::default();
        db.load_local(
            Store::from_records(
                records
                    .iter()
                    .enumerate()
                    .map(|(i, p)| (i as u32 + 1, 0x1000_1000, p.as_slice())),
            ),
            Default::default(),
        );
        let mut stores = ArenaStores {
            stash: vec![LOCAL | 1, 0x20],
            ..Default::default()
        };
        stores.arsenal.shelves[2][4].push(crate::arsenal::ArsenalEntry {
            item: LOCAL | 2,
            flags: 0,
            first: 0x28,
            second: 4,
        });
        stores.staff.employees.push(crate::staff::Employee {
            class: 4,
            name: [0; 16],
            kit: vec![LOCAL | 3, LOCAL | 1],
            carried: vec![LOCAL | 4],
            hands: [LOCAL | 5, 0],
            worn: vec![],
            world: None,
        });
        db.set_remaps(true);
        let changed = promote_arena_stores(&mut db, &mut stores);
        db.set_remaps(false);
        assert_eq!(changed, 6);
        assert_eq!(stores.stash, [WORLD | 1, 0x20]);
        assert_eq!(stores.arsenal.shelves[2][4][0].item, WORLD | 2);
        let employee = &stores.staff.employees[0];
        // The record first (hands, then the pack), then the list; the list's
        // second entry is the stash's item again.
        assert_eq!(employee.hands, [WORLD | 3, 0]);
        assert_eq!(employee.carried, [WORLD | 4]);
        assert_eq!(employee.kit, [WORLD | 5, WORLD | 1]);
        assert!(db.local.is_empty());
        assert_eq!(db.world.len(), 5);
    }

    /// Only the characters the migration list takes along are promoted,
    /// each once.
    #[test]
    fn the_travelling_followers_items_are_promoted() {
        use exa_formats::objdb::{LOCAL, ObjectDb, Store, WORLD};
        let records: Vec<Vec<u8>> = (1..=2)
            .map(|id| apparel_record(&format!("Item{id}"), 0, 0, 0))
            .collect();
        let mut db = ObjectDb::default();
        db.load_local(
            Store::from_records(
                records
                    .iter()
                    .enumerate()
                    .map(|(i, p)| (i as u32 + 1, 0x1000_1000, p.as_slice())),
            ),
            Default::default(),
        );
        let motiles = [
            crate::save_level::SceneMotile {
                address: 1,
                name: "Follower",
                controller: 4,
                flags: 0,
            },
            crate::save_level::SceneMotile {
                address: 2,
                name: "Stays",
                controller: 4,
                flags: 0x800,
            },
        ];
        let mut follower = CharacterRecord::reset();
        follower.worn.push(ItemRef {
            id: LOCAL | 1,
            flags: 0,
        });
        let mut stays = CharacterRecord::reset();
        stays.worn.push(ItemRef {
            id: LOCAL | 2,
            flags: 0,
        });
        let mut list = [follower, stays];
        assert_eq!(promote_followers(&mut db, &motiles, &mut list), 1);
        assert_eq!(list[0].worn.last().unwrap().id, WORLD | 1);
        assert_eq!(list[1].worn.last().unwrap().id, LOCAL | 2);
    }

    #[test]
    fn a_kit_copy_keeps_its_condition_through_a_save_and_a_load() {
        use exa_assets::live_objects::LiveObjects;
        use exa_formats::objdb::{LOCAL, ObjectDb, Store, WORLD};
        let base = || {
            let mut db = ObjectDb::default();
            db.base = Store::from_records([(
                0x20,
                0x1000_1000,
                apparel_record("Tunic", exa_formats::objdb::INSTANCE_ON_USE, 0xC0, 0xA0).as_slice(),
            )]);
            db
        };
        let mut live = LiveObjects::of(base());
        let level: Vec<Vec<u8>> = (1..=3)
            .map(|id| apparel_record(&format!("Level{id}"), 0, 0, 0))
            .collect();
        live.load_level(
            "test",
            Some(Store::from_records(
                level
                    .iter()
                    .enumerate()
                    .map(|(i, p)| (i as u32 + 1, 0x1000_1000, p.as_slice())),
            )),
            None,
        );
        let copy = live.db_mut().instantiate(0x20);
        assert_eq!(copy, LOCAL | 4, "the copy follows the level's own records");
        assert!(live.write_condition(copy, true, true, 0x10));
        assert!(live.write_condition(copy, true, false, 0x08));
        let mut record = CharacterRecord::reset();
        record.name = "Ash".to_string();
        let mut possessions = exa_items::possession::Possessions::of_record(&record);
        possessions.give(copy, 0);
        possessions.give(0x20, 0);
        let changed = promote_player_items(live.db_mut(), &mut possessions, None);
        assert_eq!(changed, 1);
        let ids: Vec<u32> = possessions.carried.iter().map(|c| c.id).collect();
        assert!(ids.contains(&(WORLD | 1)) && ids.contains(&0x20));
        assert!(live.db().find(LOCAL | 4).is_none());
        // The level's own records stay local.
        assert!(live.db().find(LOCAL | 3).is_some());
        let dir = folder("kit-copy");
        let mut lists = SlotLists::new(&dir);
        let player = StoryPlayer {
            record: record_with_possessions(record, &possessions),
            position_cm: [0.0; 3],
            heading: 0.0,
            powers: Vec::new(),
        };
        let career = crate::hub_windows::ArenaCareer::default();
        let mut session = live_session(Some("exanima01"), 1, &career, &ArenaStores::default());
        let export = story_player_world(&player, &mut session, live.db()).unwrap();
        assert!(export.objects.is_some());
        let number = save_new_slot(
            &mut lists.story,
            &session,
            Some("Ash"),
            &export.save_world(None, false),
        )
        .unwrap()
        .unwrap();
        let mut list = SlotList::new(Slots::story(&dir));
        assert!(list.any());
        let mut restored = restore_story(&mut list, number, false).unwrap();
        assert_eq!(restored.items[0].id, WORLD | 1);
        let back = restored.player.clone().unwrap();
        assert!(back.inventory.iter().any(|c| c.item == WORLD | 1));
        let mut fresh = LiveObjects::of(base());
        import_world_tier(&mut fresh, None, restored.world.take());
        let stored = fresh.db().stored_condition(WORLD | 1).unwrap();
        assert_eq!(
            (stored.apparel_dirt, stored.apparel_wear),
            (Some(0x10), Some(0x08))
        );
        // A save without a world tier clears the live one.
        import_world_tier(&mut fresh, None, None);
        assert!(fresh.db().world.is_empty());
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

    /// A level-placed character takes its record's yellow and red shares
    /// of its maximum, and nothing else of the saved health: the focus,
    /// heal pool and potion stay as the fighter was built.
    #[test]
    fn a_placed_character_takes_its_records_red_and_yellow() {
        let mut record = CharacterRecord::reset();
        record.pools = exa_formats::characters::Pools {
            yellow: 26214,
            red: 47185,
            focus: 100,
        };
        record.saved.push(exa_formats::characters::SavedState {
            tag: exa_formats::characters::SAVED_HEAL_POOL,
            value: 0.03f32.to_bits(),
        });
        let mut fighter = exa_combat::Fighter::default();
        assert_eq!(fighter.health.max, 0.25);
        restore_placed_pools(&record, &mut fighter);
        let share = |stored: u16| (0.25f64 * (f64::from(stored) * (1.0 / 65535.0))) as f32;
        assert_eq!(fighter.health.yellow, share(26214));
        assert_eq!(fighter.health.red, share(47185));
        assert_eq!(
            fighter.health.heal_pool,
            exa_combat::Fighter::default().health.heal_pool
        );
        // A full record gives full pools.
        let mut fresh = exa_combat::Fighter::default();
        restore_placed_pools(&CharacterRecord::reset(), &mut fresh);
        assert_eq!(fresh.health.yellow, 0.25);
        assert_eq!(fresh.health.red, 0.25);
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
            powers: Vec::new(),
        };
        let career = crate::hub_windows::ArenaCareer::default();
        let mut session = live_session(Some("exanima01"), 60, &career, &ArenaStores::default());
        let export = story_player_world(
            &player,
            &mut session,
            &exa_formats::objdb::ObjectDb::default(),
        )
        .unwrap();
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
    fn a_live_save_stamps_the_chase_cameras_eye_and_shown_zoom() {
        // The writers copy the viewport camera node's translation (the eye,
        // centimetres) and the controller's current zoom word into the
        // header: the running game takes them from the chase camera.
        let dir = folder("live-camera");
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_plugins(SaveLivePlugin)
            .insert_resource(SaveFolder(dir.clone()));
        let chase = crate::player::ChaseCamera::default();
        let zoom = chase.shown_zoom();
        assert_eq!(zoom, 1300.0);
        app.world_mut()
            .spawn((chase, Transform::from_xyz(69.8125, 15.75, 32.5)));
        app.update();
        assert_eq!(
            *app.world().resource::<SaveCamera>(),
            SaveCamera {
                target_cm: [6981.25, 1575.0, 3250.0],
                zoom,
            }
        );
        app.world_mut().write_message(SaveToNewSlot);
        app.update();
        let number = app.world().resource::<SaveReport>().saved.unwrap();
        let bytes = std::fs::read(Slots::story(&dir).file(number, SlotKind::Save)).unwrap();
        let package = save_package::PackageImage::read(&bytes).unwrap();
        let data = save_package::PlayerData::parse(
            package.section(save_package::PLAYER_DATA).unwrap(),
            save_package::Flavour::Story,
        )
        .unwrap();
        let save_package::PlayerHeader::Story(header) = data.header else {
            panic!("story")
        };
        assert_eq!(header.camera, [6981.25, 1575.0, 3250.0]);
        assert_eq!(header.zoom, 1300.0);
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

    /// The level exit's transition save: a save holding neither
    /// `exanima04` nor the destination is written and copied over the
    /// checkpoint, an existing one overwritten (the original's copy does not
    /// fail on an existing file), with the sixth level's clock kept; a save
    /// that already holds the destination's section is written and keeps
    /// the checkpoint it had.
    #[test]
    fn the_transition_save_keeps_a_checkpoint_only_for_a_new_level() {
        let dir = folder("transition-save");
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_plugins(SaveLivePlugin)
            .insert_resource(SaveFolder(dir.clone()));
        let level = exa_formats::rfc_write::export_save_level(
            &exa_formats::rfc_write::ExportScene {
                whole_scene: true,
                ..Default::default()
            },
            &exa_formats::rfc_write::ExportOptions::default(),
            None,
        )
        .unwrap();
        app.insert_resource(exa_world::LevelSource {
            entry: "exanima02.rfc".to_string(),
            bytes: level.into(),
            from_save: false,
            spawned: Vec::new(),
        });
        app.world_mut().spawn(exa_world::LevelRoot {
            name: "exanima02".to_string(),
        });
        app.update();
        app.world_mut().write_message(SaveToNewSlot);
        app.update();
        let number = app.world().resource::<SaveReport>().saved.unwrap();
        let slots = Slots::story(&dir);
        let checkpoint = slots.file(number, SlotKind::Checkpoint);
        std::fs::write(&checkpoint, b"old").unwrap();
        // The save holds exanima02 (its level) but not exanima03.
        app.world_mut().write_message(SaveStep::TransitionSave {
            locale: "exanima03",
            level_time: true,
        });
        app.update();
        assert!(slots.has_saved_level(number, "exanima02"));
        let save = std::fs::read(slots.file(number, SlotKind::Save)).unwrap();
        assert_eq!(std::fs::read(&checkpoint).unwrap(), save, "overwritten");
        assert!(app.world().contains_resource::<LevelTime>());
        // An exit to a level the save holds: a plain save, the checkpoint
        // kept.
        app.world_mut().remove_resource::<LevelTime>();
        std::fs::write(&checkpoint, b"old").unwrap();
        app.world_mut().write_message(SaveStep::TransitionSave {
            locale: "exanima02",
            level_time: true,
        });
        app.update();
        assert_eq!(std::fs::read(&checkpoint).unwrap(), b"old");
        assert!(!app.world().contains_resource::<LevelTime>());
        assert_eq!(app.world().resource::<SaveReport>().saved, Some(number));
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The exit's availability check: the archive's shipped levels are
    /// there, a name it lacks is not unless the current slot's save holds
    /// its section; with no archive every level counts as there.
    #[test]
    fn a_level_is_available_from_the_save_or_the_archive() {
        assert!(level_available(None, None, "nowhere"));
        let Some(install) = exa_assets::locate_install(None) else {
            return;
        };
        let game = exa_assets::Game(std::sync::Arc::new(
            exa_assets::GameData::open(&install).unwrap(),
        ));
        assert!(level_available(None, Some(&game), "exanima03"));
        assert!(!level_available(None, Some(&game), "exanima99"));
        let dir = folder("level-available");
        let mut lists = SlotLists::new(&dir);
        let number = lists.story.create(Some("Ash")).unwrap();
        let session = Session::story();
        let world = crate::slots::SaveWorld {
            level: Some(("exanima99", b"level bytes")),
            ..Default::default()
        };
        lists
            .story
            .slots()
            .save_full(number, &session, &world)
            .unwrap();
        assert!(level_available(Some(&lists), Some(&game), "exanima99"));
        assert!(level_available(Some(&lists), Some(&game), "exanima03"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A `CheckpointSave` call for the player runs the checkpoint step; a
    /// call for another character, or with the settings' checkpoints off,
    /// writes nothing.
    #[test]
    fn the_checkpoint_script_writes_the_checkpoint_only_for_the_player() {
        use exa_script::dialogue::Effect;
        let dir = folder("checkpoint-script");
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_plugins(SaveLivePlugin)
            .insert_resource(SaveFolder(dir.clone()));
        app.update();
        app.world_mut().write_message(SaveToNewSlot);
        app.update();
        let number = app.world().resource::<SaveReport>().saved.unwrap();
        let checkpoint = Slots::story(&dir).file(number, SlotKind::Checkpoint);
        // Another character's call: nothing.
        app.world_mut()
            .write_message(exa_script::ScriptEffect(Effect::CheckpointSave {
                player: false,
            }));
        app.update();
        assert!(!checkpoint.exists());
        // Checkpoints off in the settings: nothing.
        let live = exa_formats::hw_config::Options {
            checkpoints: false,
            ..Default::default()
        };
        app.insert_resource(crate::hw_config::HardwareConfig::new(live));
        app.world_mut()
            .write_message(exa_script::ScriptEffect(Effect::CheckpointSave {
                player: true,
            }));
        app.update();
        assert!(!checkpoint.exists());
        // On, for the player: the save, then its copy.
        app.insert_resource(crate::hw_config::HardwareConfig::new(
            exa_formats::hw_config::Options::default(),
        ));
        app.world_mut()
            .write_message(exa_script::ScriptEffect(Effect::CheckpointSave {
                player: true,
            }));
        app.update();
        assert!(checkpoint.is_file());
        assert_eq!(
            std::fs::read(Slots::story(&dir).file(number, SlotKind::Save)).unwrap(),
            std::fs::read(&checkpoint).unwrap()
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
            0,
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

    /// The level exit's save: the original writes it before the
    /// destination loads, so it records the level being left (its locale
    /// and its own section), not the destination. In one frame the exit's
    /// save step and its change request run; the save holds `exanima03`,
    /// the level the player stands in, and not `exanima04`.
    #[test]
    fn the_exit_save_records_the_level_being_left() {
        let dir = folder("exit-save");
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_plugins(SaveLivePlugin)
            .insert_resource(SaveFolder(dir.clone()));
        let level = app
            .world_mut()
            .spawn(exa_world::LevelRoot {
                name: "exanima02.rfc".to_string(),
            })
            .id();
        app.update();
        app.world_mut().write_message(SaveToNewSlot);
        app.update();
        let number = app.world().resource::<SaveReport>().saved.unwrap();
        // The player walked on to the next level and now takes its exit.
        app.world_mut().despawn(level);
        app.world_mut().spawn(exa_world::LevelRoot {
            name: "exanima03.rfc".to_string(),
        });
        app.world_mut().write_message(SaveStep::Save {
            drop_on_low_health: false,
        });
        app.world_mut()
            .write_message(exa_world::level_lifecycle::LevelChangeRequest::change(
                "exanima04",
                0,
            ));
        app.update();
        let mut list = SlotList::new(Slots::story(&dir));
        assert!(list.any());
        let restored = restore_story(&mut list, number, false).unwrap();
        assert_eq!(restored.kind, SlotKind::Save);
        assert_eq!(restored.level, "exanima03");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The exit save keeps the player where it stood, inside the exit zone
    /// of the level it left. Its continue loads that level at the saved
    /// position (the original's save load names no level it came from, so
    /// no arrival marker applies) and the continue's nudge moves the player
    /// out of the exit before it stands on the floor.
    #[test]
    fn a_continue_after_an_exit_save_stands_in_the_level_left_out_of_its_exit() {
        let dir = folder("exit-continue");
        let mut lists = SlotLists::new(&dir);
        // The player in `exanima01`'s exit to `exanima02` (centimetres).
        let player = StoryPlayer {
            record: CharacterRecord::reset(),
            position_cm: [-901.0, 20.0, -9980.0],
            heading: 0.5,
            powers: Vec::new(),
        };
        let career = crate::hub_windows::ArenaCareer::default();
        let mut session = live_session(Some("exanima01.rfc"), 60, &career, &ArenaStores::default());
        let export = story_player_world(
            &player,
            &mut session,
            &exa_formats::objdb::ObjectDb::default(),
        )
        .unwrap();
        let number = save_new_slot(
            &mut lists.story,
            &session,
            Some("Ash"),
            &export.save_world(None, false),
        )
        .unwrap()
        .unwrap();
        let mut list = SlotList::new(Slots::story(&dir));
        let restored = restore_story(&mut list, number, false).unwrap();
        assert_eq!(restored.level, "exanima01");
        let at = crate::level_change::PlayerPlacement::of_flow(&restored.placement.unwrap());
        assert!((at.position - Vec3::new(-9.01, 0.2, -99.8)).length() < 1e-4);
        let exit = exa_world::ExitVolume {
            bounds: exa_physics::intersect::Obb::axis_aligned(
                Vec3::new(-901.01166, 100.42999, -9980.086),
                Vec3::new(225.0, 150.0, 225.0),
            ),
            radius: 350.0,
            exit: exa_formats::objdb::LevelExit {
                level: 3,
                entrance: 0,
                flagged: false,
            },
            destination: Some("exanima02".into()),
            name: ">exanima02".into(),
        };
        let (moved, _, nudge) =
            crate::level_change::nudge_out_of_exits(at.position, std::slice::from_ref(&exit))
                .unwrap();
        // Out past the box's nearer side by 75 cm.
        assert!(nudge.distance > 75.0);
        assert!(crate::level_change::nudge_out_of_exits(moved, &[exit]).is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The adapter journey's end saves the player's story slot and copies
    /// the save over its checkpoint, an existing checkpoint included;
    /// another body's journey saves nothing.
    #[test]
    fn a_journeys_end_saves_and_refreshes_the_checkpoint() {
        let dir = folder("journey");
        let mut app = new_game_app(&dir, crate::flow::Mode::Story, "Traveller");
        app.world_mut().spawn(exa_world::LevelRoot {
            name: "exanima02.rfc".to_string(),
        });
        app.world_mut().write_message(SaveStep::NewSlot);
        app.update();
        let player = app
            .world_mut()
            .spawn((
                GlobalTransform::default(),
                crate::player::PlayerBody::new(0.0),
            ))
            .id();
        let other = app.world_mut().spawn_empty().id();
        let slots = Slots::story(&dir);
        let number = app.world().resource::<SlotLists>().story.current() as u32;
        // An old checkpoint the journey's copy replaces.
        std::fs::write(slots.file(number, SlotKind::Checkpoint), b"old").unwrap();
        app.world_mut()
            .write_message(crate::destination::JourneySave { user: other });
        app.update();
        assert_eq!(app.world().resource::<SaveReport>().saved, None);
        app.world_mut()
            .write_message(crate::destination::JourneySave { user: player });
        app.update();
        assert_eq!(app.world().resource::<SaveReport>().saved, Some(number));
        let save = std::fs::read(slots.file(number, SlotKind::Save)).unwrap();
        let checkpoint = std::fs::read(slots.file(number, SlotKind::Checkpoint)).unwrap();
        assert_eq!(save, checkpoint);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A save written while a respawn was pending (transition word 2):
    /// the load turns the word to 1 and the continue places the player at
    /// the saved position, then asks for the portal placement the loaded
    /// level names (16 ticks of settling with it, none without); a portal
    /// an earlier load found does not decide it. Any other save asks for
    /// no portal and settles nothing.
    #[test]
    fn a_pending_respawn_load_asks_for_the_loaded_levels_portal() {
        for pending in [false, true] {
            let dir = folder(if pending {
                "pending-respawn"
            } else {
                "no-respawn"
            });
            let mut lists = SlotLists::new(&dir);
            let player = StoryPlayer {
                record: CharacterRecord::reset(),
                position_cm: [100.0, 20.0, 300.0],
                heading: 0.0,
                powers: Vec::new(),
            };
            let career = crate::hub_windows::ArenaCareer::default();
            let mut session =
                live_session(Some("exanima05.rfc"), 60, &career, &ArenaStores::default());
            if pending {
                let mut state = [0u8; 16];
                state[..4].copy_from_slice(&crate::session::RESPAWN_PENDING.to_le_bytes());
                session.transition = Some(state);
            }
            let export = story_player_world(
                &player,
                &mut session,
                &exa_formats::objdb::ObjectDb::default(),
            )
            .unwrap();
            let number = save_new_slot(
                &mut lists.story,
                &session,
                Some("Ash"),
                &export.save_world(None, false),
            )
            .unwrap()
            .unwrap();
            // The plugin's load system over the slot lists that hold it.
            let mut app = App::new();
            app.add_plugins(MinimalPlugins)
                .add_message::<StorySlotLoad>()
                .add_message::<crate::world_clock::RestoreWorldClock>()
                .init_resource::<SaveReport>()
                .init_resource::<exa_assets::live_objects::LiveObjects>()
                .insert_resource(lists)
                .add_systems(Update, load_story_slot)
                .insert_resource({
                    let mut flow = crate::flow::Flow::default();
                    flow.respawn_portal = Some(crate::flow::Placement {
                        rotation: crate::flow::NO_ROTATION,
                        position: Vec3::new(10.0, 0.0, 20.0),
                    });
                    flow
                })
                .init_resource::<crate::flow::FlowSteps>();
            app.update();
            app.world_mut().write_message(StorySlotLoad {
                number,
                prefer_checkpoint: false,
            });
            app.update();
            let steps = &app.world().resource::<crate::flow::FlowSteps>().steps;
            let place = steps
                .iter()
                .position(|step| matches!(step, crate::flow::Step::PlacePlayer(_)))
                .expect("the continue places the player");
            let crate::flow::Step::PlacePlayer(at) = steps[place] else {
                unreachable!()
            };
            let loaded = app.world().resource::<LoadedStory>();
            assert!((at.position - Vec3::new(100.0, 20.0, 300.0)).length() < 1e-3);
            assert!(
                !steps
                    .iter()
                    .any(|step| matches!(step, crate::flow::Step::SettlePlayer(_)))
            );
            if pending {
                assert_eq!(
                    steps.get(place + 1),
                    Some(&crate::flow::Step::PlaceAtPortal(
                        crate::level_change::PortalArrival {
                            without: None,
                            settle: 16,
                        }
                    ))
                );
                assert_eq!(loaded.0.transition_word(), 1);
            } else {
                assert!(
                    !steps
                        .iter()
                        .any(|step| matches!(step, crate::flow::Step::PlaceAtPortal(_)))
                );
                assert_eq!(loaded.0.transition_word(), 0);
            }
            let _ = std::fs::remove_dir_all(&dir);
        }
    }

    /// A section: tag, size (body + 8) and body.
    fn db_section(tag: u32, body: &[u8]) -> Vec<u8> {
        let mut v = tag.to_le_bytes().to_vec();
        v.extend((body.len() as u32 + 8).to_le_bytes());
        v.extend_from_slice(body);
        v
    }

    /// A header body: the name (16 bytes), the flag word and the parent.
    fn db_header(name: &str, flags: u32, parent: u32) -> Vec<u8> {
        let mut b = name.as_bytes().to_vec();
        b.resize(16, 0);
        b.extend(flags.to_le_bytes());
        b.extend(parent.to_le_bytes());
        b
    }

    /// An object database file of `magic` holding `(id, flags, payload)`.
    fn db_file(magic: u32, records: &[(u32, u32, Vec<u8>)]) -> Vec<u8> {
        let mut b = magic.to_le_bytes().to_vec();
        b.extend((((records.len() + 1) * 16) as u32).to_le_bytes());
        let mut offset = 0u32;
        for (id, flags, payload) in records {
            for w in [*id, *flags, offset, payload.len() as u32] {
                b.extend(w.to_le_bytes());
            }
            offset += payload.len() as u32;
        }
        b.extend([0; 16]);
        for (_, _, payload) in records {
            b.extend_from_slice(payload);
        }
        b
    }

    /// Writes a story slot whose `#worldobjdb` holds `objects`, loads it
    /// through the plugin's load system over a live database that holds
    /// `base`, and returns the live database after the load.
    fn load_world_objects(
        name: &str,
        objects: Vec<u8>,
        base: Option<exa_formats::objdb::Store>,
    ) -> exa_formats::objdb::ObjectDb {
        let dir = folder(name);
        let mut lists = SlotLists::new(&dir);
        let player = StoryPlayer {
            record: CharacterRecord::reset(),
            position_cm: [0.0; 3],
            heading: 0.0,
            powers: Vec::new(),
        };
        let career = crate::hub_windows::ArenaCareer::default();
        let mut session = live_session(Some("exanima05.rfc"), 60, &career, &ArenaStores::default());
        let mut export = story_player_world(
            &player,
            &mut session,
            &exa_formats::objdb::ObjectDb::default(),
        )
        .unwrap();
        export.objects = Some((objects, exa_formats::objstrings::Library::default().write()));
        let number = save_new_slot(
            &mut lists.story,
            &session,
            Some("Ash"),
            &export.save_world(None, false),
        )
        .unwrap()
        .unwrap();
        let mut live = exa_assets::live_objects::LiveObjects::default();
        if let Some(base) = base {
            live.db_mut().base = base;
        }
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_message::<StorySlotLoad>()
            .add_message::<crate::world_clock::RestoreWorldClock>()
            .init_resource::<SaveReport>()
            .insert_resource(live)
            .insert_resource(lists)
            .add_systems(Update, load_story_slot)
            .init_resource::<crate::flow::Flow>()
            .init_resource::<crate::flow::FlowSteps>();
        app.update();
        app.world_mut().write_message(StorySlotLoad {
            number,
            prefer_checkpoint: false,
        });
        app.update();
        let db = app
            .world()
            .resource::<exa_assets::live_objects::LiveObjects>()
            .db()
            .clone();
        let _ = std::fs::remove_dir_all(&dir);
        db
    }

    /// An older save's `#worldobjdb` goes through the loader's upgrade
    /// chain on the live load: `0xDBAC0D08` converts the operative section,
    /// `0xDBAC0D07` the weapon and then the operative section, and
    /// `0xDBAC0D06` the old body (its count byte through the count setter,
    /// its paint seed from the run-time library's generator) and the
    /// random tables' condition words first. The live world tier then
    /// holds the current layout.
    #[test]
    fn an_older_saves_world_objects_are_upgraded_on_the_load() {
        use exa_formats::objdb::RANDOM_TABLE;
        const NAME: u32 = 0x1000_0000;
        const BODY: u32 = 0x1000;
        const WEAPON: u32 = 0x4000;
        const OPERATIVE: u32 = 0x1_0000;
        let mut old_body = vec![0u8; 0x224];
        old_body[0x1A8] = 3;
        old_body[0x1A9] = 0x82;
        // Part 0: kind 0, finish 0: set 3, finish 1, material 1.
        old_body[0x1B8 + 2] = 0x12;
        let mut weapon = vec![0u8; 0x2C];
        weapon[0x1F] = 4; // kind 0, class 4: flags 0x14
        let mut operative = vec![0u8; 0x20];
        operative[0] = 2; // old class 2: 4
        operative[1] = 3; // old kind 3: 7
        let mut table = vec![0u8; 0x20];
        table[0xC..0x10].copy_from_slice(&1u32.to_le_bytes());
        let payload = [
            db_section(NAME, &db_header("Old", 0, 0)),
            db_section(BODY, &old_body),
            db_section(WEAPON, &weapon),
            db_section(OPERATIVE, &operative),
            db_section(RANDOM_TABLE, &table),
        ]
        .concat();
        let flags = NAME | BODY | WEAPON | OPERATIVE | RANDOM_TABLE;
        for magic in [0xDBAC_0D08u32, 0xDBAC_0D07, 0xDBAC_0D06] {
            let file = db_file(magic, &[(1, flags, payload.clone())]);
            let db = load_world_objects(&format!("upgrade-{magic:x}"), file, None);
            let record = db.world.find(1).expect("the world tier holds the record");
            let operative = record.section(OPERATIVE).unwrap();
            assert_eq!((operative.len(), operative[0], operative[1]), (0x3C, 4, 7));
            let weapon = record.section(WEAPON).unwrap();
            let body = record.section(BODY).unwrap();
            let condition = u16::from_le_bytes(
                record.section(RANDOM_TABLE).unwrap()[0x1A..0x1C]
                    .try_into()
                    .unwrap(),
            );
            match magic {
                0xDBAC_0D08 => assert_eq!(weapon.len(), 0x2C),
                0xDBAC_0D07 => {
                    assert_eq!((weapon.len(), weapon[0]), (0x60, 0x14));
                    assert_eq!(body, &old_body[..]);
                    assert_eq!(condition, 0);
                }
                _ => {
                    assert_eq!((weapon.len(), weapon[0]), (0x60, 0x14));
                    assert_eq!(body.len(), 0x224);
                    // The count byte, the armour type and the first layer.
                    assert_eq!(body[0x1A9] & 3, 1);
                    assert_eq!(body[0x1A8], 3);
                    assert_eq!(&body[0x1B4..0x1B9], &[1, 1, 3, 1, 1]);
                    assert_eq!(body[0x1B4 + 8], 0x12);
                    // A zero condition word takes the defaults.
                    assert_eq!(condition, 0x5284);
                }
            }
        }
    }

    /// A saved world instance whose header's material class (bits 16–23)
    /// differs from its template's takes the template's appearance layers on
    /// the load's template migration; one of the same class keeps its own.
    #[test]
    fn a_class_change_refreshes_a_saved_instances_appearance() {
        const NAME: u32 = 0x1000_0000;
        const BODY: u32 = 0x1000;
        let mut template_body = vec![0u8; 0x224];
        // Mask 9, set 2 (materials 5, 6, 10), byte 3, colour 0x40, byte 0xB.
        template_body[0x1B4..0x1C0].copy_from_slice(&[9, 2, 2, 3, 0, 0, 0, 0, 0x40, 0, 0, 1]);
        let mut instance_body = vec![0u8; 0x224];
        // Material 7, not in the template's set: it becomes the set's first.
        instance_body[0x1B4..0x1C0].copy_from_slice(&[0, 0, 0, 0, 7, 0, 0, 0, 0x11, 0, 0, 0]);
        let template = [
            db_section(NAME, &db_header("Tunic", 0x12_0000, 0)),
            db_section(BODY, &template_body),
        ]
        .concat();
        let base_file = db_file(exa_formats::objdb::MAGIC, &[(10, NAME | BODY, template)]);
        let base = exa_formats::objdb::Store::parse(&base_file).unwrap();
        let instance = |class: u32| {
            [
                db_section(NAME, &db_header("Worn", class, 10)),
                db_section(BODY, &instance_body),
            ]
            .concat()
        };
        let world = db_file(
            exa_formats::objdb::MAGIC,
            &[
                (1, NAME | BODY, instance(0x11_0000)),
                (2, NAME | BODY, instance(0x12_0000)),
            ],
        );
        let db = load_world_objects("class-change", world, Some(base));
        let layer =
            |id: u32| db.world.find(id).unwrap().section(BODY).unwrap()[0x1B4..0x1C0].to_vec();
        assert_eq!(layer(1), [9, 2, 2, 3, 5, 0, 0, 0, 0x11, 0, 0, 1]);
        assert_eq!(layer(2), [0, 0, 0, 0, 7, 0, 0, 0, 0x11, 0, 0, 0]);
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

    /// An app over a scratch save folder with the flow in `mode` and a
    /// created character named `name`, as a new game's commit leaves it.
    fn new_game_app(dir: &std::path::Path, mode: crate::flow::Mode, name: &str) -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_plugins(SaveLivePlugin)
            .insert_resource(SaveFolder(dir.to_path_buf()))
            .insert_resource({
                let mut flow = crate::flow::Flow::default();
                flow.mode = mode;
                flow.phase = crate::flow::Phase::Playing;
                flow
            })
            .insert_resource(crate::creator_screen::CreatedCharacter(Some(
                CharacterRecord {
                    name: name.to_string(),
                    ..CharacterRecord::reset()
                },
            )));
        app.update();
        app
    }

    /// The settled match's return to the hub reads the hub from the current
    /// arena slot's saved `arenahub` section, as the original's match
    /// return loads it through the save-or-archive loader with its save
    /// flag set; any other arena hub load leaves the archive's.
    #[test]
    fn the_hub_return_reads_the_arena_slots_saved_hub() {
        let dir = folder("hub-return");
        let mut app = new_game_app(&dir, crate::flow::Mode::Arena, "Boss");
        app.world_mut().write_message(SaveStep::NewSlot);
        app.update();
        let current = app.world().resource::<SlotLists>().arena.current();
        assert!(current > 0);
        // The arena's save at the match's begin exported the hub it left.
        let file = Slots::arena(&dir).file(current as u32, SlotKind::Save);
        let mut package = std::fs::read(&file)
            .ok()
            .and_then(|bytes| save_package::PackageImage::read(&bytes).ok())
            .unwrap_or_default();
        package.put_section(&save_package::name("arenahub"), b"the saved hub");
        std::fs::write(&file, package.to_bytes()).unwrap();
        // A hub load that is not the match's return: the archive's.
        app.insert_resource(crate::arena_hub_live::ArenaHub::default());
        app.insert_resource(exa_world::level_lifecycle::LevelChangeReady {
            level: Some("arenahub".into()),
            spawn: 0,
            changes: 1,
            door: false,
        });
        app.update();
        assert!(
            !app.world()
                .contains_resource::<exa_world::SavedLevelSection>()
        );
        // The return: the slot's own section.
        app.world_mut()
            .resource_mut::<crate::arena_hub_live::ArenaHub>()
            .returned = Some(crate::arena_session::RETURN_PLACEMENT);
        app.insert_resource(exa_world::level_lifecycle::LevelChangeReady {
            level: Some("arenahub".into()),
            spawn: 0,
            changes: 2,
            door: false,
        });
        app.update();
        let section = app.world().resource::<exa_world::SavedLevelSection>();
        assert_eq!(section.locale, "arenahub");
        assert_eq!(&section.bytes[..], b"the saved hub");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A new story makes its slot at the start (named after the created
    /// character, current at once), so the story's first save lands in it
    /// without a continue from the saves screen.
    #[test]
    fn a_new_story_creates_its_slot_and_its_first_save_lands_there() {
        let dir = folder("new-story");
        // Two older saves: the new slot takes the lowest free number and
        // goes to the front.
        let slots = Slots::story(&dir);
        slots.create(&Session::story()).unwrap();
        slots.create(&Session::story()).unwrap();
        let mut app = new_game_app(&dir, crate::flow::Mode::Story, "Wanderer");
        // Before the slot the flow's save writes nothing.
        app.world_mut().write_message(SaveStep::Save {
            drop_on_low_health: true,
        });
        app.update();
        assert_eq!(app.world().resource::<SaveReport>().saved, None);
        app.world_mut().write_message(SaveStep::NewSlot);
        app.update();
        let lists = app.world().resource::<SlotLists>();
        assert_eq!(lists.story.current(), 3);
        assert_eq!(lists.story.numbers(), &[3, 2, 1]);
        assert_eq!(lists.story.names()[0].as_deref(), Some("Wanderer"));
        assert_eq!(lists.arena.current(), -1);
        // The slot's package is there, empty until the first save.
        let file = slots.file(3, SlotKind::Save);
        assert!(file.is_file());
        app.world_mut().write_message(SaveStep::Save {
            drop_on_low_health: true,
        });
        app.update();
        assert_eq!(app.world().resource::<SaveReport>().saved, Some(3));
        let mut list = SlotList::new(Slots::story(&dir));
        assert!(list.any());
        assert_eq!(list.numbers()[0], 3);
        list.select(0);
        let loaded = list.load_current(false).unwrap();
        assert!(matches!(loaded.data.header, PlayerHeader::Story(_)));
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The flow's start of a story or a career begins with the new slot;
    /// practice makes none.
    #[test]
    fn the_session_begin_asks_for_the_new_slot_in_story_and_arena_only() {
        use crate::flow::{Flow, Mode as FlowMode, Step};
        for (mode, slot) in [
            (FlowMode::Story, true),
            (FlowMode::Arena, true),
            (FlowMode::Practice, false),
        ] {
            let mut flow = Flow::default();
            flow.mode = mode;
            let steps = flow.begin_session(vec![Step::LoadLevel {
                locale: "exanima01",
                spawn: 0,
                teardown: true,
            }]);
            assert_eq!(steps.first() == Some(&Step::NewSlot), slot, "{mode:?}");
        }
    }

    /// The arena's saves go to the arena manager's current slot: the new
    /// career's slot, a save with the company's tables, and the forfeit's
    /// player-data save.
    #[test]
    fn an_arena_career_saves_into_its_own_slot_and_the_forfeit_saves_its_player_data() {
        let dir = folder("new-career");
        let mut app = new_game_app(&dir, crate::flow::Mode::Arena, "Manager");
        let mut career = crate::hub_windows::ArenaCareer::default();
        career.company.copper = 750;
        career.company.day = 3;
        app.insert_resource(career);
        app.world_mut().write_message(SaveStep::NewSlot);
        app.update();
        let lists = app.world().resource::<SlotLists>();
        assert_eq!(lists.arena.current(), 1);
        assert_eq!(lists.arena.names()[0].as_deref(), Some("Manager"));
        assert_eq!(lists.story.current(), -1);
        app.world_mut().write_message(SaveStep::Save {
            drop_on_low_health: false,
        });
        app.update();
        let arena = Slots::arena(&dir);
        assert!(!Slots::story(&dir).file(1, SlotKind::Save).exists());
        let money = |dir: &std::path::Path| {
            let mut list = SlotList::new(Slots::arena(dir));
            assert!(list.any());
            list.select(0);
            match list.load_current(false).unwrap().data.header {
                PlayerHeader::Arena(header) => (header.money, header.day),
                _ => panic!("not an arena save"),
            }
        };
        assert_eq!(money(&dir), (750, 3));
        // The forfeit's save rewrites the player data with the settled
        // career; a checkpoint step does nothing in the arena.
        app.world_mut()
            .resource_mut::<crate::hub_windows::ArenaCareer>()
            .company
            .copper = 600;
        app.world_mut().write_message(SaveStep::ForfeitSave);
        app.world_mut().write_message(SaveStep::Checkpoint);
        app.update();
        assert_eq!(money(&dir), (600, 3));
        assert!(!arena.file(1, SlotKind::Checkpoint).exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A continued arena career, through the live slot load: the read is
    /// followed by the continue fixups, so an unfit entrant leaves its
    /// pending event.
    #[test]
    fn a_continued_arena_career_runs_its_fixups_after_the_load() {
        let dir = folder("continue-fixups");
        let mut app = new_game_app(&dir, crate::flow::Mode::Arena, "Manager");
        let mut career = crate::hub_windows::ArenaCareer::default();
        // The manager is entered in the first event at a health under the
        // fit line.
        career.player.health = 100;
        career.entries[0].count = 1;
        career.entries[0].entries[0].actor_id = 0;
        app.insert_resource(career);
        app.world_mut().write_message(SaveStep::NewSlot);
        app.update();
        app.world_mut().write_message(SaveStep::Save {
            drop_on_low_health: false,
        });
        app.update();
        let load = |app: &mut App| {
            app.insert_resource(crate::arena_hub_live::ArenaHub {
                entered: true,
                ..Default::default()
            });
            app.insert_resource(ArenaSlotLoad { number: 1 });
            app.update();
            assert_eq!(
                app.world().resource::<SaveReport>().loaded.map(|l| l.0),
                Some(1)
            );
        };
        load(&mut app);
        let career = app.world().resource::<crate::hub_windows::ArenaCareer>();
        // The week's events came back; the unfit entrant left his event.
        assert!(career.tournament.is_some());
        assert_eq!(career.entries[0].count, 0);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The arena load's id pass, through the live slot save and load: an
    /// arsenal entry flagged `0x400` naming a base record with the instance
    /// bit comes back as a fresh local instance, and the stash item that
    /// held the same id follows it; an unflagged base id the database lacks
    /// stays as it was.
    #[test]
    fn an_arena_load_instances_the_arsenals_base_items_and_their_holders() {
        use exa_formats::objdb::{INSTANCE_ON_USE, LOCAL, ObjectDb, Store};
        let dir = folder("load-remap");
        let mut app = new_game_app(&dir, crate::flow::Mode::Arena, "Manager");
        app.insert_resource(crate::hub_windows::ArenaCareer::default());
        let kind = ItemKind {
            known: true,
            apparel_layer: Some(1),
            ..Default::default()
        };
        {
            let mut stores = app.world_mut().resource_mut::<ArenaStores>();
            stores
                .arsenal
                .stow(1175, kind, crate::arena_ids::EMPLOYEE_REF);
            stores.arsenal.stow(4321, kind, 0);
            stores.stash.push(1175);
        }
        app.world_mut().write_message(SaveStep::NewSlot);
        app.update();
        app.world_mut().write_message(SaveStep::Save {
            drop_on_low_health: false,
        });
        app.update();
        // The load's database knows 1175 as an instanced record.
        let mut name = vec![0u8; 0x20];
        name[0x10..0x14].copy_from_slice(&INSTANCE_ON_USE.to_le_bytes());
        let mut payload = Vec::new();
        payload.extend_from_slice(&0x1000_0000u32.to_le_bytes());
        payload.extend_from_slice(&((name.len() + 8) as u32).to_le_bytes());
        payload.extend_from_slice(&name);
        let mut db = ObjectDb::default();
        db.base = Store::from_records([(1175, 0x1000_0000, payload.as_slice())]);
        app.insert_resource(exa_assets::live_objects::LiveObjects::of(db));
        app.insert_resource(crate::arena_hub_live::ArenaHub {
            entered: true,
            ..Default::default()
        });
        app.insert_resource(ArenaSlotLoad { number: 1 });
        app.update();
        assert_eq!(
            app.world().resource::<SaveReport>().loaded.map(|l| l.0),
            Some(1)
        );
        let stores = app.world().resource::<ArenaStores>();
        let entries: Vec<(u32, u32)> = stores
            .arsenal
            .shelves
            .iter()
            .flatten()
            .flatten()
            .map(|entry| (entry.item, entry.flags))
            .collect();
        let made = entries
            .iter()
            .find(|(_, flags)| flags & crate::arena_ids::EMPLOYEE_REF != 0)
            .map(|e| e.0)
            .expect("the flagged entry");
        assert_ne!(made & LOCAL, 0, "{entries:#x?}");
        assert_eq!(
            app.world()
                .resource::<exa_assets::live_objects::LiveObjects>()
                .db()
                .template_of(made),
            1175
        );
        assert_eq!(stores.stash, vec![made]);
        assert!(entries.iter().any(|&(item, _)| item == 4321));
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The app's close while a session plays runs the quit's autosave in
    /// the same frame: the story saves, the arena hub saves from its
    /// in-game page, practice writes nothing.
    #[test]
    fn closing_the_app_saves_the_playing_session() {
        for (mode, page, saved) in [
            (crate::flow::Mode::Story, None, true),
            (
                crate::flow::Mode::Arena,
                Some(crate::menu_pages::Page::InGame),
                true,
            ),
            (crate::flow::Mode::Arena, None, false),
            (crate::flow::Mode::Practice, None, false),
        ] {
            let dir = folder(&format!("quit-{mode:?}-{page:?}"));
            let mut app = new_game_app(&dir, mode, "Quitter");
            if let Some(page) = page {
                let mut menu = crate::menu_screen::MenuScreen::default();
                menu.page = page;
                app.insert_resource(menu);
            }
            // The slot comes from the start (practice has no manager).
            app.world_mut().write_message(SaveStep::NewSlot);
            app.update();
            assert_eq!(app.world().resource::<SaveReport>().saved, None);
            app.world_mut().write_message(AppExit::Success);
            app.update();
            assert!(app.world().resource::<QuitAutosave>().0);
            assert_eq!(
                app.world().resource::<SaveReport>().saved,
                saved.then_some(1),
                "{mode:?} {page:?}"
            );
            // The next frame (a test app keeps running) writes nothing more.
            let before = app.world().resource::<SaveReport>().clone();
            app.update();
            assert_eq!(*app.world().resource::<SaveReport>(), before);
            let _ = std::fs::remove_dir_all(&dir);
        }
    }

    /// A save writes the live journal's pages and a story load hands the
    /// saved pages to the in-game journal.
    #[test]
    fn the_live_journal_is_saved_and_a_load_puts_it_back() {
        let dir = folder("journal");
        let mut app = new_game_app(&dir, crate::flow::Mode::Story, "Scribe");
        let mut screens = crate::gui_screens::GameScreens::default();
        screens.journal.entries = vec![
            exa_ui::game_gui::JournalEntry {
                id: 0,
                title: "Gate".to_string(),
                text: "The gate is shut.".to_string(),
            },
            exa_ui::game_gui::JournalEntry {
                id: 0x51,
                title: "Key".to_string(),
                text: "A key in the well.".to_string(),
            },
        ];
        app.insert_resource(screens);
        app.world_mut().write_message(SaveStep::NewSlot);
        app.update();
        app.world_mut().write_message(SaveStep::Save {
            drop_on_low_health: false,
        });
        app.update();
        let mut list = SlotList::new(Slots::story(&dir));
        assert!(list.any());
        list.select(0);
        let loaded = list.load_current(false).unwrap();
        // Each page's id word is written as the page carries it (the
        // original's writer writes the word its reader put back; the game's
        // own pages carry 0).
        let pages: Vec<(i32, String, String)> = loaded
            .data
            .journal_or_default()
            .unwrap()
            .into_iter()
            .map(|record| (record.id, record.title, record.text))
            .collect();
        assert_eq!(
            pages,
            vec![
                (0, "Gate".to_string(), "The gate is shut.".to_string()),
                (0x51, "Key".to_string(), "A key in the well.".to_string()),
            ]
        );
        // The load: the in-game journal (here back to its first page)
        // takes the loaded session's pages with their id words.
        let number = app.world().resource::<SaveReport>().saved.unwrap();
        let mut reread = SlotList::new(Slots::story(&dir));
        let session = restore_story(&mut reread, number, false).unwrap().session;
        let ids: Vec<i32> = session
            .story
            .journal()
            .entries()
            .iter()
            .map(|page| page.id)
            .collect();
        assert_eq!(ids, [0, 0x51]);
        app.world_mut()
            .resource_mut::<crate::gui_screens::GameScreens>()
            .journal = exa_ui::game_gui::Journal::default();
        app.insert_resource(LoadedStory(session));
        app.update();
        let journal = &app
            .world()
            .resource::<crate::gui_screens::GameScreens>()
            .journal;
        assert_eq!(journal.entries.len(), 2);
        assert_eq!(journal.entries[1].title, "Key");
        assert_eq!(journal.entries[1].text, "A key in the well.");
        assert_eq!(journal.entries[1].id, 0x51);
        assert_eq!(journal.current, 0);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A story save's level section carries the armed golem alarm in its
    /// settings chunk, and the section read back restores it.
    #[test]
    fn the_golem_alarm_is_saved_with_the_level_and_restored_from_it() {
        let dir = folder("alarm");
        let mut app = new_game_app(&dir, crate::flow::Mode::Story, "Thief");
        let level = exa_formats::rfc_write::export_save_level(
            &exa_formats::rfc_write::ExportScene {
                whole_scene: true,
                ..Default::default()
            },
            &exa_formats::rfc_write::ExportOptions::default(),
            None,
        )
        .unwrap();
        app.insert_resource(exa_world::LevelSource {
            entry: "exanima02.rfc".to_string(),
            bytes: level.into(),
            from_save: false,
            spawned: Vec::new(),
        });
        let alarm = crate::golem_alarm::GolemAlarm {
            armed: exa_ai::golem::AlarmZone {
                armed: true,
                at: Vec3::new(120.0, 35.5, -4410.0),
                pass: 474,
            },
            rung: 2,
        };
        app.insert_resource(alarm);
        app.world_mut().write_message(SaveStep::NewSlot);
        app.update();
        app.world_mut().write_message(SaveStep::Save {
            drop_on_low_health: false,
        });
        app.update();
        let package = Slots::open(&Slots::story(&dir).file(1, SlotKind::Save)).unwrap();
        let section = package
            .section(&save_package::name("exanima02"))
            .expect("the level section")
            .to_vec();
        assert_eq!(
            crate::golem_alarm::GolemAlarm::from_saved_section(&section),
            Some(alarm)
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
