//! The scene side of the save unit: which copy of a level a load opens, the
//! import of its chunk tree and the constants the game scene is created and
//! the player put into it with.
//!
//! The original's save unit owns the load's scene calls even though the
//! scene itself belongs to the world: [`scene_load_plan`] reproduces the
//! three entry points that choose between a save's level section and the
//! game archive's `<locale>.rfc`, [`import_level`] reproduces the import
//! routine's order (the section, the scene-content registration, the
//! terrain from `<locale>.rft`, the node and sector passes) and reports
//! what it would have done, [`game_scene_defaults`] the values the game
//! scene is created with, [`menu_scene_load`] the menu background scene's
//! load, [`player_node_setup`] and [`player_spawn`] the player's node and
//! spawn, and [`reset_game_gui`] the GUI reset every load ends with.
//!
//! The world, renderer, audio and GUI calls these rules drive are the
//! host's; nothing here touches Bevy or the archive. Exarust's live level
//! loader (`exa_world`) keeps its own scene model, so no running game calls
//! these yet: they are the tested rules the save path will call, pinned to
//! the original's numbers and branch order.
//!
//! Evidence: the load, import, scene-creation and spawn routines of the
//! save unit, read from the instructions (`docs/formats/SAVES.md`, Loading a
//! level).

use exa_formats::rfc::Scene;
use exa_formats::terrain::Terrain;
use exa_formats::text::strip_extension_append;

/// The extension the archive's level files carry. A save's level section is
/// named by the bare locale instead.
pub const LEVEL_SUFFIX: &str = ".rfc";

/// The extension of a level's terrain file.
pub const TERRAIN_SUFFIX: &str = ".rft";

/// The archive name of a locale's level: the locale with its extension
/// replaced by `.rfc`, as the original's name helper does it (the engine's
/// helper cuts the result at sixteen bytes; locales never come near that).
pub fn archive_level_name(locale: &str) -> String {
    strip_extension_append(locale, LEVEL_SUFFIX)
}

/// Which copy of a level a load opens.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LevelOrigin {
    /// The save package's section, named by the bare locale.
    SaveSection,
    /// The game archive's `<locale>.rfc`.
    Archive,
}

/// The three load entry points that open a level.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SceneLoadKind {
    /// A save load (or the scene load after it): the save's section when
    /// there is one and the load asks for it, else the archive.
    SaveOrArchive,
    /// A level loaded by name for a new game or a practice fight: always
    /// the archive, with the save-disable flag cleared first, the load's
    /// global reset, and the scene environment's outdoor word cleared on
    /// success.
    NamedLevel,
    /// An arena level: always the archive, without the load's global reset.
    ArenaLevel,
}

/// What the chosen entry point does before and after the import.
#[derive(Debug, Clone, PartialEq)]
pub struct SceneLoadPlan {
    pub origin: LevelOrigin,
    /// The engine's current level id for the locale (`locales.rdb`).
    pub level_id: u32,
    /// The previous level id these entry points always store as zero.
    pub previous_level: u32,
    /// The portal node is cleared (a null pointer: the level's own start).
    pub portal_cleared: bool,
    /// The engine's portal id word, stored as −1 (none).
    pub portal_word: i32,
    /// The saved spawn matrix is reset to its default (the two zero
    /// globals the entry points store).
    pub matrix_reset: bool,
    /// The GUI's task and item-panel reset (the named-level path only).
    pub gui_reset: bool,
    /// The settings tag, saved word and sun-height reset (the named-level
    /// path only; the save path resets them earlier, before the world
    /// databases are read).
    pub settings_reset: bool,
    /// The local item database is imported by the save's importer when the
    /// section came from the save; the archive's own comes with the file.
    pub save_local_database: bool,
    /// Clears the scene environment's word that the outdoor preset sets to
    /// 1 and the indoor preset to 0, once the import succeeds (the
    /// named-level path only). It is not the sun mode, which has its own
    /// setter [guess at the word's meaning; the store is exact].
    pub outdoor_word_cleared: bool,
    /// Clears the save-disable flag before anything else (the named-level
    /// path only: a new game or practice fight may save again).
    pub saves_reenabled: bool,
}

impl SceneLoadPlan {
    /// Whether the store of the current level's name happens after a
    /// successful import: all three entry points do it.
    pub const STORE_NAME_ON_SUCCESS: bool = true;
}

/// The plan for one of the load entry points, or `None` when the level
/// cannot be opened at all: a save section was not offered and the archive
/// does not hold `<locale>.rfc` (the original returns without importing).
///
/// The original's own order: the arena path resets the item GUI first and
/// sets the level ids; the named-level path additionally resets the GUI, the
/// character databases and the load globals; the save path resets the world
/// databases earlier in the load. Only the save path can take the section.
pub fn scene_load_plan(
    kind: SceneLoadKind,
    locale_id: u32,
    save_section_offered: bool,
    archive_found: bool,
) -> Option<SceneLoadPlan> {
    let origin = if kind == SceneLoadKind::SaveOrArchive && save_section_offered {
        LevelOrigin::SaveSection
    } else if archive_found {
        LevelOrigin::Archive
    } else {
        return None;
    };
    Some(SceneLoadPlan {
        origin,
        level_id: locale_id,
        previous_level: 0,
        portal_cleared: true,
        portal_word: -1,
        matrix_reset: true,
        gui_reset: kind == SceneLoadKind::NamedLevel,
        settings_reset: kind == SceneLoadKind::NamedLevel,
        save_local_database: origin == LevelOrigin::SaveSection,
        outdoor_word_cleared: kind == SceneLoadKind::NamedLevel,
        saves_reenabled: kind == SceneLoadKind::NamedLevel,
    })
}

/// One step of the import, in the original's order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImportStep {
    /// The kd-tree build is reserved.
    ReserveTrees,
    /// The section is handed to the chunk-tree importer.
    ImportChunks,
    /// The section's stream is released (the original frees the buffer it
    /// was given, whoever owned it).
    FreeSection,
    /// The reservation is released.
    ReleaseReservation,
    /// The scene-content helpers begin their pass.
    ContentBegin,
    /// The kd-tree and texture passes are prepared for the nodes.
    PreparePasses,
    /// The imported nodes are classified into the scene's lists (the save
    /// unit's own classifier, [`crate::save_level::scene_content`]).
    ClassifyContent,
    /// The node count is fixed.
    NumberNodes,
    /// The renderer's GI manager, when there is one, is reset.
    ResetGi,
    /// The terrain `<locale>.rft` is read from the archive.
    Terrain,
    /// The arena task is told the terrain is missing (only with an arena
    /// task present).
    ArenaTerrainMissing,
    /// The kd-tree and texture passes run again once the terrain is in.
    CompletePasses,
    /// The nodes are added to the scene.
    RegisterNodes,
    /// The scene's sector and bounds passes rebuild.
    RebuildScene,
    /// The scene's final node pass runs.
    FinishScene,
    /// The grid environments are updated, when the scene has a grid.
    GridEnvironments,
    /// The scene-ready callback runs.
    SceneReady,
}

/// What an import did.
#[derive(Debug, Clone)]
pub struct LevelImport {
    /// The steps that ran, in order.
    pub steps: Vec<ImportStep>,
    /// Whether the chunk tree imported at least one node.
    pub imported: bool,
    /// The nodes the section holds, when it parsed.
    pub nodes: usize,
    /// The terrain's archive name.
    pub terrain_name: String,
    /// Whether the archive had the terrain file and it was read: the
    /// original's test, which does not depend on the terrain's contents.
    pub terrain_loaded: bool,
    /// Whether the arena task was told the terrain is missing.
    pub arena_terrain_missing: bool,
    /// The parsed chunk tree when it imported.
    pub scene: Option<Scene>,
    /// The parsed terrain, when it loaded and parsed.
    pub terrain: Option<Terrain>,
    /// The level's settings chunk, when the save's importer read one (the
    /// story's exporter writes it after the scene).
    pub settings: Option<exa_formats::save_chunk::SaveSettingsRead>,
}

/// Imports a level section exactly as the save unit's import does.
///
/// The order: reserve the trees; import the chunk tree (a section that does
/// not parse imports nothing); free the section; release the reservation;
/// when nodes came back, run the content pass (classify, number nodes),
/// reset the GI manager, read the terrain from the archive under the level's
/// locale (the original appends `.rft` to the **locale**, not to the save's
/// section name), tell the arena task when it is missing, complete the
/// tree/texture passes, register the nodes, rebuild the scene, update the
/// grid environments when the scene has a grid and report the scene ready.
/// A section with no nodes or a malformed one returns `imported: false`
/// with only the first four steps.
pub fn import_level(
    section: &[u8],
    locale: &str,
    terrain_bytes: Option<&[u8]>,
    arena_present: bool,
) -> LevelImport {
    let terrain_name = strip_extension_append(locale, TERRAIN_SUFFIX);
    let mut out = LevelImport {
        steps: vec![
            ImportStep::ReserveTrees,
            ImportStep::ImportChunks,
            ImportStep::FreeSection,
            ImportStep::ReleaseReservation,
        ],
        imported: false,
        nodes: 0,
        terrain_name,
        terrain_loaded: false,
        arena_terrain_missing: false,
        scene: None,
        terrain: None,
        settings: read_settings(section),
    };
    let Ok(scene) = Scene::parse(section) else {
        return out;
    };
    if scene.nodes.is_empty() {
        return out;
    }
    out.nodes = scene.nodes.len();
    out.steps.extend([
        ImportStep::ContentBegin,
        ImportStep::PreparePasses,
        ImportStep::ClassifyContent,
        ImportStep::NumberNodes,
        ImportStep::ResetGi,
        ImportStep::Terrain,
    ]);
    // The original reports the terrain loaded once the archive entry is
    // found and read; the terrain importer's own result does not feed the
    // arena hook.
    if let Some(bytes) = terrain_bytes {
        out.terrain_loaded = true;
        out.terrain = Terrain::parse(bytes).ok();
    }
    if !out.terrain_loaded && arena_present {
        out.arena_terrain_missing = true;
        out.steps.push(ImportStep::ArenaTerrainMissing);
    }
    out.steps.extend([
        ImportStep::CompletePasses,
        ImportStep::RegisterNodes,
        ImportStep::RebuildScene,
        ImportStep::FinishScene,
    ]);
    if scene.grid.is_some() {
        out.steps.push(ImportStep::GridEnvironments);
    }
    out.steps.push(ImportStep::SceneReady);
    out.imported = true;
    out.scene = Some(scene);
    out
}

/// The settings chunk a save's level section carries, read as the load's
/// settings hook does: the root chunk with the header id, its twelve-byte
/// tag (its version float decides the old flag) and the two words. Chunk
/// tags other than the settings header are skipped by the hook, so only the
/// first matching chunk counts, as the importer dispatches each once.
fn read_settings(section: &[u8]) -> Option<exa_formats::save_chunk::SaveSettingsRead> {
    let body = section.get(4..)?;
    let chunks = exa_formats::save_package::parse_chunks(body).ok()?;
    let chunk = chunks
        .iter()
        .find(|chunk| chunk.id == exa_formats::save_chunk::SAVE_SETTINGS_ID)?;
    let payload = chunk.payload;
    if payload.len() < 20 {
        return None;
    }
    let tag = f32::from_le_bytes(payload[4..8].try_into().ok()?);
    let first = i32::from_le_bytes(payload[12..16].try_into().ok()?);
    let second = i32::from_le_bytes(payload[16..20].try_into().ok()?);
    Some(exa_formats::save_chunk::read_save_settings(
        chunk.id, tag, first, second,
    ))
}

/// A level that an entry point opened and imported.
#[derive(Debug, Clone)]
pub struct LoadedLevel {
    pub plan: SceneLoadPlan,
    pub import: LevelImport,
}

/// Opens and imports a level for one of the three entry points: the plan
/// chooses the save's section or the archive's file, the import runs on the
/// chosen bytes with the locale for the terrain probe. The caller supplies
/// the bytes it read (the save's section and the archive's `<locale>.rfc`);
/// `None` when the chosen copy is not there or the plan refuses the load.
pub fn load_level(
    kind: SceneLoadKind,
    locale: &str,
    locale_id: u32,
    save_section: Option<&[u8]>,
    archive: Option<&[u8]>,
    terrain: Option<&[u8]>,
    arena_present: bool,
) -> Option<LoadedLevel> {
    let plan = scene_load_plan(kind, locale_id, save_section.is_some(), archive.is_some())?;
    let bytes = match plan.origin {
        LevelOrigin::SaveSection => save_section?,
        LevelOrigin::Archive => archive?,
    };
    let import = import_level(bytes, locale, terrain, arena_present);
    Some(LoadedLevel { plan, import })
}

/// The values the game scene is created with, before any level is loaded:
/// the scene's own flag word, the sound-slot count the original stores, the
/// editor's (empty) name, the two node-manager flags, the scene
/// environment's sun mode and fog distance, the view's flag and mode and the
/// renderer option word. The objects created on the way (the object
/// manager, the audio environment, the GI manager, blood, the AI hooks) are
/// the host's; these are the values it must see. The scene environment's
/// sun mode is set to off here through the same setter the presets use
/// (argument 1; the indoor preset sets it again, the outdoor preset to on).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GameSceneDefaults {
    /// The flag word the scene object starts with (`0x10000`).
    pub scene_flags: u32,
    /// The word the original stores at the scene's sound-slot field (115;
    /// the field's meaning is inferred from its neighbours) [guess].
    pub sound_slots: u32,
    /// The node-manager flags the original sets (`1` and `1`).
    pub node_flags: [u8; 2],
    /// The scene environment's sun mode, written through its setter
    /// (argument 1: off).
    pub sun_mode: exa_formats::environment::SunMode,
    /// The scene environment's first fog distance: `0x3F333333` (0.7).
    pub fog_distance: f32,
    /// The view's own byte the original sets to one.
    pub view_flag: bool,
    /// The mode the view's own setter is called with (1).
    pub view_mode: u32,
    /// The renderer's option word the original stores (3).
    pub renderer_option: u32,
}

/// One step of the game scene's creation, in the original's order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameSceneStep {
    /// The renderer's scene object and the node manager are created.
    CreateRendererScene,
    /// The scene's flag word gains `0x10000`.
    FlagScene,
    /// The object manager is created and attached to the scene.
    CreateObjectManager,
    /// The OpenAL loader runs when the audio environment is not up yet, and
    /// the audio environment is attached and switched on.
    CreateAudio,
    /// The scene's sound-slot word is stored.
    StoreSoundSlots,
    /// The scene node is added with 0x20, 0x20.
    AddSceneNode,
    /// The scene editor is created with the empty name.
    CreateEditor,
    /// The node-manager flags are set.
    SetNodeFlags,
    /// The scene environment's sun mode and fog distance are set.
    SetSunAndFog,
    /// The view's byte is set and its setter called with 1.
    SetView,
    /// The renderer's option word is stored and its create call runs.
    SetRenderer,
    /// The GI manager is created and attached to the renderer.
    CreateGi,
    /// The blood sprays and the weapon blood are created.
    CreateBlood,
    /// The AI's scene hook is initialised.
    CreateAi,
    /// The save unit's scene-content classifier is registered.
    RegisterClassifier,
}

/// The scene-creation steps in the original's order.
pub fn game_scene_steps() -> Vec<GameSceneStep> {
    vec![
        GameSceneStep::CreateRendererScene,
        GameSceneStep::FlagScene,
        GameSceneStep::CreateObjectManager,
        GameSceneStep::CreateAudio,
        GameSceneStep::StoreSoundSlots,
        GameSceneStep::AddSceneNode,
        GameSceneStep::CreateEditor,
        GameSceneStep::SetNodeFlags,
        GameSceneStep::SetSunAndFog,
        GameSceneStep::SetView,
        GameSceneStep::SetRenderer,
        GameSceneStep::CreateGi,
        GameSceneStep::CreateBlood,
        GameSceneStep::CreateAi,
        GameSceneStep::RegisterClassifier,
    ]
}

/// The scene-creation constants.
pub fn game_scene_defaults() -> GameSceneDefaults {
    GameSceneDefaults {
        scene_flags: 0x1_0000,
        sound_slots: 0x73,
        node_flags: [1, 1],
        sun_mode: exa_formats::environment::SunMode::Off,
        fog_distance: 0.7,
        view_flag: true,
        view_mode: 1,
        renderer_option: 3,
    }
}

/// The steps the menu background scene's load performs, in order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuStep {
    /// Create the game scene when there is none.
    CreateScene,
    /// Reset the arena task when one exists (`0x108`, the same hook the
    /// terrain failure uses).
    ResetArena,
    /// The portal word goes to −1.
    ClearPortal,
    /// The indoor preset with fog 1.8.
    IndoorPreset,
    /// The GUI reset every load ends with.
    GuiReset,
    /// The dialogue engine's reset.
    DialogueReset,
    /// The GUI's open form is closed (argument zero).
    CloseForm,
    /// The game scene becomes the current scene the other units read
    /// [guess at the global's role; the store itself is exact].
    SetCurrentScene,
    /// The archive's `menuscene.rfc` is opened.
    OpenArchive,
    /// It is imported with its own name for the terrain probe.
    Import,
    /// A word of the scene's grid object goes to zero (its meaning is not
    /// traced) [guess at the field's role; the store is exact].
    ClearGridWord,
    /// The scene editor is put in mode zero.
    EditorMode,
    /// The camera's field, view and target are set.
    Camera,
    /// The scene's node pass updates the camera's matrices.
    UpdateNodes,
}

/// The menu background scene's load plan: the values and the steps.
#[derive(Debug, Clone, PartialEq)]
pub struct MenuSceneLoad {
    pub steps: Vec<MenuStep>,
    /// The archive entry.
    pub file: &'static str,
    /// The terrain the import probes, from the same name.
    pub terrain: String,
    /// The indoor preset's fog distance.
    pub fog_distance: f32,
    /// The scene grid's word after a successful import.
    pub grid_word: u32,
    /// The scene editor's mode.
    pub editor_mode: u32,
    /// The camera's field of view (degrees; the field is only stored) [guess].
    pub camera_field: f32,
    /// The camera's view point.
    pub view: [f32; 3],
    /// The camera's target.
    pub target: [f32; 3],
}

/// The menu scene's load: `menuscene.rfc` with the 1.8 fog preset, the editor
/// in mode zero, the camera at (0, 500, 700) looking at (0, 0, −200), and a
/// 60 written into its field beforehand. Reuses [`crate::save_level::menu_scene`]
/// for the values the load sets.
pub fn menu_scene_load() -> MenuSceneLoad {
    let scene = crate::save_level::menu_scene();
    MenuSceneLoad {
        steps: vec![
            MenuStep::CreateScene,
            MenuStep::ResetArena,
            MenuStep::ClearPortal,
            MenuStep::IndoorPreset,
            MenuStep::GuiReset,
            MenuStep::DialogueReset,
            MenuStep::CloseForm,
            MenuStep::SetCurrentScene,
            MenuStep::OpenArchive,
            MenuStep::Import,
            MenuStep::ClearGridWord,
            MenuStep::EditorMode,
            MenuStep::Camera,
            MenuStep::UpdateNodes,
        ],
        file: scene.file,
        terrain: strip_extension_append(scene.file, TERRAIN_SUFFIX),
        fog_distance: scene.fog_distance,
        grid_word: 0,
        editor_mode: 0,
        camera_field: scene.camera_field,
        view: scene.view,
        target: scene.target,
    }
}

/// The GUI state a load resets. The original calls four reset routines and
/// clears a global word; the routines' internals belong to the GUI units.
/// What each resets is inferred from the units they belong to [guess]; the
/// order and the cleared word are exact.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GameGuiReset {
    /// The game GUI's task list and the form it has open.
    pub task_list: bool,
    /// The tips/hover state; the original passes argument 1 (the selector
    /// is not traced) [guess].
    pub hints: bool,
    /// The shared game functions' per-load state.
    pub game_functions: bool,
    /// The item GUI's panels, cursor and hover.
    pub item_gui: bool,
    /// The global word the original zeroes last.
    pub word: u32,
}

/// The GUI reset every level load, practice/arena preparation and the menu
/// scene runs after the item GUI's own reset.
pub fn reset_game_gui() -> GameGuiReset {
    GameGuiReset {
        task_list: true,
        hints: true,
        game_functions: true,
        item_gui: true,
        word: 0,
    }
}

/// What the player's node set-up does: the body part's controller mode (8),
/// the node's visibility, the two words cleared on the body and the node's
/// registration with the scene. The node's own owner (the player object) is
/// installed by the caller.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerNodeSetup {
    pub part_mode: u32,
    pub visible: bool,
    pub cleared: [u32; 2],
    pub registered: bool,
}

/// The player node's set-up constants.
pub fn player_node_setup() -> PlayerNodeSetup {
    PlayerNodeSetup {
        part_mode: 8,
        visible: true,
        cleared: [0, 0],
        registered: true,
    }
}

/// What the player's spawn does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerSpawn {
    /// The spawn goes through the installed custom generator, which may
    /// answer "not yet"; the original then sleeps zero and asks again.
    pub async_generator: bool,
    /// The root node's flag word gains `0x1000`.
    pub root_flag: u32,
    /// The attached nodes' part pass runs before the flag is set.
    pub animate_parts: bool,
    /// The body's ready call, made before the controls and inventory.
    pub ready: bool,
    /// The player behaviour object is created for the body.
    pub behaviour: bool,
    /// The inventory is attached.
    pub inventory: bool,
    /// The player controls are attached.
    pub controls: bool,
    /// The node is added to the scene.
    pub registered: bool,
    /// The generator's finishing pass runs last.
    pub finish: bool,
}

/// The player's spawn constants: with the default generator the character
/// record builds the body directly; with a custom one the original calls it
/// until it returns a body. The root node then takes `0x1000`, the body is
/// readied, the behaviour, inventory and controls are attached, and the node
/// joins the scene before the generator finishes.
pub fn player_spawn(async_generator: bool) -> PlayerSpawn {
    PlayerSpawn {
        async_generator,
        root_flag: 0x1000,
        animate_parts: true,
        ready: true,
        behaviour: true,
        inventory: true,
        controls: true,
        registered: true,
        finish: true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A minimal chunk tree with one node, written by the exporter so it has
    /// the shipped shape the reader accepts.
    fn one_node_section() -> Vec<u8> {
        let mut name = [0u8; 16];
        name[..4].copy_from_slice(b"node");
        let scene = exa_formats::rfc_write::ExportScene {
            nodes: vec![exa_formats::rfc_write::SceneNode {
                name,
                flags: exa_formats::rfc_write::class::PLAIN,
                local: exa_formats::rfc_write::IDENTITY,
                world: exa_formats::rfc_write::IDENTITY,
                ..Default::default()
            }],
            whole_scene: true,
            ..Default::default()
        };
        exa_formats::rfc_write::export_save_level(
            &scene,
            &exa_formats::rfc_write::ExportOptions::default(),
            None,
        )
        .unwrap()
    }

    #[test]
    fn the_archive_names_replace_the_extension() {
        assert_eq!(archive_level_name("exanima04"), "exanima04.rfc");
        assert_eq!(archive_level_name("arenao1"), "arenao1.rfc");
        assert_eq!(archive_level_name("level.rfc"), "level.rfc");
        assert_eq!(
            strip_extension_append("exanima04", TERRAIN_SUFFIX),
            "exanima04.rft"
        );
    }

    #[test]
    fn a_save_section_is_preferred_only_when_it_is_offered() {
        let save = scene_load_plan(SceneLoadKind::SaveOrArchive, 27, true, true).unwrap();
        assert_eq!(save.origin, LevelOrigin::SaveSection);
        assert!(save.save_local_database);
        assert_eq!(save.level_id, 27);
        assert_eq!(save.previous_level, 0);
        assert!(save.portal_cleared);
        assert_eq!(save.portal_word, -1);
        assert!(save.matrix_reset);
        assert!(
            !save.gui_reset
                && !save.settings_reset
                && !save.outdoor_word_cleared
                && !save.saves_reenabled
        );

        // A save without the section falls back to the archive.
        let fallback = scene_load_plan(SceneLoadKind::SaveOrArchive, 27, false, true).unwrap();
        assert_eq!(fallback.origin, LevelOrigin::Archive);
        assert!(!fallback.save_local_database);
        // Neither copy there: nothing is imported.
        assert!(scene_load_plan(SceneLoadKind::SaveOrArchive, 27, false, false).is_none());

        // A named level always uses the archive, resets and sets the sun
        // automatic on success.
        let named = scene_load_plan(SceneLoadKind::NamedLevel, 5, true, true).unwrap();
        assert_eq!(named.origin, LevelOrigin::Archive);
        assert!(
            named.gui_reset
                && named.settings_reset
                && named.outdoor_word_cleared
                && named.saves_reenabled
        );
        assert!(!named.save_local_database);
        assert!(scene_load_plan(SceneLoadKind::NamedLevel, 5, false, false).is_none());

        // An arena level uses the archive and does not reset the load's
        // globals.
        let arena = scene_load_plan(SceneLoadKind::ArenaLevel, 9, true, true).unwrap();
        assert_eq!(arena.origin, LevelOrigin::Archive);
        assert!(
            !arena.gui_reset
                && !arena.settings_reset
                && !arena.outdoor_word_cleared
                && !arena.saves_reenabled
        );
    }

    #[test]
    fn an_import_runs_the_content_terrain_and_node_passes_in_order() {
        let section = one_node_section();
        let import = import_level(&section, "exanima04", None, false);
        assert!(import.imported);
        assert_eq!(import.nodes, 1);
        assert_eq!(import.terrain_name, "exanima04.rft");
        assert!(!import.terrain_loaded && !import.arena_terrain_missing);
        assert_eq!(
            import.steps,
            vec![
                ImportStep::ReserveTrees,
                ImportStep::ImportChunks,
                ImportStep::FreeSection,
                ImportStep::ReleaseReservation,
                ImportStep::ContentBegin,
                ImportStep::PreparePasses,
                ImportStep::ClassifyContent,
                ImportStep::NumberNodes,
                ImportStep::ResetGi,
                ImportStep::Terrain,
                ImportStep::CompletePasses,
                ImportStep::RegisterNodes,
                ImportStep::RebuildScene,
                ImportStep::FinishScene,
                ImportStep::SceneReady,
            ]
        );
        // A grid section updates the grid environments before the ready
        // callback; the minimal section has no grid.
        assert!(!import.steps.contains(&ImportStep::GridEnvironments));
        assert!(import.scene.is_some());
    }

    #[test]
    fn a_missing_terrain_tells_an_arena_but_not_a_story_load() {
        let section = one_node_section();
        let arena = import_level(&section, "arenao1", None, true);
        assert!(!arena.terrain_loaded && arena.arena_terrain_missing);
        assert!(arena.steps.contains(&ImportStep::ArenaTerrainMissing));
        let story = import_level(&section, "exanima04", None, false);
        assert!(!story.arena_terrain_missing);
        assert!(!story.steps.contains(&ImportStep::ArenaTerrainMissing));
    }

    #[test]
    fn a_terrain_file_that_parses_is_kept() {
        // The minimal shipped shape: magic, a word, the palette name, the
        // spacing and the tile grid's size, with no tiles.
        let mut terrain = Vec::new();
        terrain.extend_from_slice(&exa_formats::terrain::TERRAIN_MAGIC.to_le_bytes());
        terrain.extend_from_slice(&0u32.to_le_bytes());
        let mut palette = [0u8; 16];
        palette[..4].copy_from_slice(b"pal0");
        terrain.extend_from_slice(&palette);
        terrain.extend_from_slice(&300.0f32.to_le_bytes());
        terrain.extend_from_slice(&1u32.to_le_bytes());
        terrain.extend_from_slice(&1u32.to_le_bytes());
        terrain.extend_from_slice(&0u32.to_le_bytes());
        let import = import_level(&one_node_section(), "arenao1", Some(&terrain), true);
        assert!(import.terrain_loaded);
        assert_eq!(import.terrain.as_ref().unwrap().palette, "pal0");
        assert!(!import.arena_terrain_missing);
        assert!(!import.steps.contains(&ImportStep::ArenaTerrainMissing));
        // A terrain entry that was read but does not parse still counts as
        // loaded: the arena hook only hears of a missing file.
        let broken = import_level(&one_node_section(), "arenao1", Some(&[1, 2, 3]), true);
        assert!(broken.terrain_loaded && broken.terrain.is_none());
        assert!(!broken.arena_terrain_missing);
    }

    #[test]
    fn the_entry_points_open_the_chosen_copy_and_import_it() {
        let section = one_node_section();
        let archive = one_node_section();
        // A save load with the section offered: the save's copy is
        // imported, the archive's is not consulted.
        let loaded = load_level(
            SceneLoadKind::SaveOrArchive,
            "exanima04",
            27,
            Some(&section),
            Some(&archive),
            None,
            false,
        )
        .unwrap();
        assert_eq!(loaded.plan.origin, LevelOrigin::SaveSection);
        assert!(loaded.import.imported);
        // Without the section the archive is imported.
        let loaded = load_level(
            SceneLoadKind::SaveOrArchive,
            "exanima04",
            27,
            None,
            Some(&archive),
            None,
            false,
        )
        .unwrap();
        assert_eq!(loaded.plan.origin, LevelOrigin::Archive);
        assert!(loaded.import.imported);
        // A named level ignores an offered save section.
        let loaded = load_level(
            SceneLoadKind::NamedLevel,
            "arenao1",
            9,
            Some(&section),
            Some(&archive),
            None,
            true,
        )
        .unwrap();
        assert_eq!(loaded.plan.origin, LevelOrigin::Archive);
        assert!(loaded.plan.gui_reset && loaded.plan.outdoor_word_cleared);
        assert_eq!(loaded.import.terrain_name, "arenao1.rft");
        assert!(loaded.import.arena_terrain_missing);
        // Nothing to open at all.
        assert!(
            load_level(
                SceneLoadKind::SaveOrArchive,
                "exanima04",
                27,
                None,
                None,
                None,
                false
            )
            .is_none()
        );
        // A copy that does not parse is reported, not panicked on.
        let broken = load_level(
            SceneLoadKind::NamedLevel,
            "exanima04",
            27,
            None,
            Some(b"not a level"),
            None,
            false,
        )
        .unwrap();
        assert!(!broken.import.imported);
    }

    #[test]
    fn a_level_sections_settings_chunk_is_read_by_the_import() {
        let mut name = [0u8; 16];
        name[..4].copy_from_slice(b"node");
        let scene = exa_formats::rfc_write::ExportScene {
            nodes: vec![exa_formats::rfc_write::SceneNode {
                name,
                flags: exa_formats::rfc_write::class::PLAIN,
                local: exa_formats::rfc_write::IDENTITY,
                world: exa_formats::rfc_write::IDENTITY,
                ..Default::default()
            }],
            whole_scene: true,
            ..Default::default()
        };
        let settings = exa_formats::save_chunk::SettingsChunk {
            tag: exa_formats::save_chunk::save_tag_bytes(false, [0; 12]),
            first: 3,
            saved: 4,
        };
        let section = exa_formats::rfc_write::export_save_level(
            &scene,
            &exa_formats::rfc_write::ExportOptions::default(),
            Some(settings),
        )
        .unwrap();
        let import = import_level(&section, "exanima04", None, false);
        let read = import.settings.unwrap();
        assert!(read.accepted);
        assert!(!read.old);
        assert_eq!((read.first, read.second), (3, 4));
        // An old tag (version under a million) sets the flag.
        let mut old_tag = [0u8; 12];
        old_tag[4..8].copy_from_slice(&900_000.0f32.to_le_bytes());
        let section = exa_formats::rfc_write::export_save_level(
            &scene,
            &exa_formats::rfc_write::ExportOptions::default(),
            Some(exa_formats::save_chunk::SettingsChunk {
                tag: old_tag,
                first: 0,
                saved: 1,
            }),
        )
        .unwrap();
        assert!(
            import_level(&section, "exanima04", None, false)
                .settings
                .unwrap()
                .old
        );
        // The plain section has none.
        assert!(
            import_level(&one_node_section(), "exanima04", None, false)
                .settings
                .is_none()
        );
    }

    #[test]
    fn a_section_that_does_not_parse_imports_nothing_but_frees_itself() {
        let import = import_level(b"not a chunk tree", "exanima04", None, false);
        assert!(!import.imported);
        assert_eq!(import.nodes, 0);
        assert!(
            import.steps
                == vec![
                    ImportStep::ReserveTrees,
                    ImportStep::ImportChunks,
                    ImportStep::FreeSection,
                    ImportStep::ReleaseReservation,
                ]
        );
        assert!(import.scene.is_none());
    }

    #[test]
    fn the_game_scene_starts_with_the_originals_values() {
        let scene = game_scene_defaults();
        assert_eq!(scene.scene_flags, 0x1_0000);
        assert_eq!(scene.sound_slots, 0x73);
        assert_eq!(scene.node_flags, [1, 1]);
        assert_eq!(scene.sun_mode, exa_formats::environment::SunMode::Off);
        assert_eq!(scene.fog_distance.to_bits(), 0x3f33_3333);
        assert!(scene.view_flag);
        assert_eq!(scene.view_mode, 1);
        assert_eq!(scene.renderer_option, 3);
        let steps = game_scene_steps();
        assert_eq!(steps.len(), 15);
        assert_eq!(steps.first(), Some(&GameSceneStep::CreateRendererScene));
        assert_eq!(steps.last(), Some(&GameSceneStep::RegisterClassifier));
        // The content classifier is registered last, after every object
        // the scene needs.
        let ai = steps
            .iter()
            .position(|step| *step == GameSceneStep::CreateAi)
            .unwrap();
        let classifier = steps
            .iter()
            .position(|step| *step == GameSceneStep::RegisterClassifier)
            .unwrap();
        assert!(ai < classifier);
    }

    #[test]
    fn the_menu_scene_load_sets_the_editor_and_camera() {
        let load = menu_scene_load();
        assert_eq!(load.file, "menuscene.rfc");
        assert_eq!(load.terrain, "menuscene.rft");
        assert_eq!(load.fog_distance, 1.8);
        assert_eq!(load.grid_word, 0);
        assert_eq!(load.editor_mode, 0);
        assert_eq!(load.camera_field, 60.0);
        assert_eq!(load.view, [0.0, 500.0, 700.0]);
        assert_eq!(load.target, [0.0, 0.0, -200.0]);
        assert_eq!(
            load.steps,
            vec![
                MenuStep::CreateScene,
                MenuStep::ResetArena,
                MenuStep::ClearPortal,
                MenuStep::IndoorPreset,
                MenuStep::GuiReset,
                MenuStep::DialogueReset,
                MenuStep::CloseForm,
                MenuStep::SetCurrentScene,
                MenuStep::OpenArchive,
                MenuStep::Import,
                MenuStep::ClearGridWord,
                MenuStep::EditorMode,
                MenuStep::Camera,
                MenuStep::UpdateNodes,
            ]
        );
        assert_eq!(load.steps.len(), 14);
    }

    #[test]
    fn the_gui_reset_clears_the_word_and_runs_four_resets() {
        let reset = reset_game_gui();
        assert!(reset.task_list && reset.hints && reset.game_functions && reset.item_gui);
        assert_eq!(reset.word, 0);
    }

    #[test]
    fn the_player_node_takes_mode_eight_and_is_shown() {
        let node = player_node_setup();
        assert_eq!(node.part_mode, 8);
        assert!(node.visible);
        assert_eq!(node.cleared, [0, 0]);
        assert!(node.registered);
    }

    #[test]
    fn the_player_spawn_flags_and_order() {
        let spawn = player_spawn(false);
        assert!(!spawn.async_generator);
        assert_eq!(spawn.root_flag, 0x1000);
        assert!(spawn.animate_parts && spawn.ready && spawn.behaviour);
        assert!(spawn.inventory && spawn.controls && spawn.registered);
        assert!(spawn.finish);
        assert!(player_spawn(true).async_generator);
    }
}
