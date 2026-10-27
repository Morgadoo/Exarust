//! The running arena hub: its entry, its click hook and its session hook.
//!
//! The rules are [`crate::arena_hub`]'s; this module calls them from the
//! game. When a play level named `arenahub` is loaded the hub is entered
//! once: [`enter_hub`] draws its music from the session's roll and the
//! alternative-music setting, the track goes through the audio crate's level
//! music resources, the click and session hooks are installed, and the
//! manager is placed by [`manager_plan`] on the hub's known start. Every
//! frame [`hub_fall`] checks the manager while the session plays; an answer
//! ends the session and, when its health is under the death line, wipes the
//! slot through [`Slots::drops_after_death`] when the host supplied the
//! slot.
//!
//! A left click that is not a drag sends [`WorldClick`] from
//! [`crate::hand`]. The hub turns it into a [`HubTarget`] from the object's
//! name and side or the character's condition and merchant profession, asks
//! [`hub_click`] and opens the answered window on the hub's forms.
//!
//! Evidence: the retail hub entry, hub session hook and hub click hook
//! (`context/re/arena-hub.md`, `context/re/arena-match.md`).

use bevy::prelude::*;
use exa_combat::{Condition, Fighter};
use exa_physics::body::Ragdoll;
use exa_world::LevelRoot;
use exa_world::start::player_start;

use crate::arena_hub::{ARENA_MODE, HubAction, HubHook, HubTarget, enter_hub, hub_click, hub_fall};
use crate::arena_spawn::manager_plan;
use crate::flow::{Flow, Hook, Phase};
use crate::hand::WorldClick;
use crate::hub_windows::{HubWindowKind, HubWindows};
use crate::player::{Player, PlayerBody};
use crate::slots::Slots;

/// The level the hub lives in.
pub const HUB_LEVEL: &str = "arenahub";

/// What the hub knows after it was entered.
#[derive(Resource, Clone, Debug, PartialEq)]
pub struct ArenaHub {
    /// The entry ran for the level now loaded.
    pub entered: bool,
    /// The busy flag the click hook reads.
    pub busy: bool,
    /// The click and session hooks the entry installed.
    pub click_hook: HubHook,
    pub session_hook: HubHook,
    /// The manager's plan from its profile place and the hub's floor.
    pub manager: Option<crate::arena_spawn::ManagerPlan>,
    /// The session hook's answer, once the manager fell.
    pub fall: Option<HubFall>,
    /// The roll the entry used.
    pub roll: u32,
}

impl Default for ArenaHub {
    fn default() -> Self {
        Self {
            entered: false,
            busy: false,
            click_hook: HubHook::None,
            session_hook: HubHook::None,
            manager: None,
            fall: None,
            roll: 0,
        }
    }
}

/// The hub session hook's answer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HubFall {
    /// The manager's health is under the death line: the slot is wiped.
    pub wipe: bool,
}

/// The host's arena slot for the hub's permadeath, when it has one.
///
/// The play-mode hub has no live session slot; a host that loads a career
/// inserts this so [`hub_fall`]'s wipe reaches the existing slot code.
#[derive(Resource, Clone, Debug, PartialEq, Eq)]
pub struct HubSlots {
    pub slots: Slots,
    pub number: u32,
}

/// The hub's own frame roll, a stand-in for the engine's shared generator
/// when no session roll is supplied. The play-mode hub begins from the
/// performance counter, as the match draws do.
#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
pub struct HubRoll(pub u32);

impl Default for HubRoll {
    fn default() -> Self {
        Self(crate::arena_match::counter_now() as u32)
    }
}

/// The manager's body as the hub's hooks read it.
#[derive(Clone, Copy, Debug, PartialEq)]
struct ManagerReading {
    condition: u32,
    health: f32,
    pelvis_cm: f32,
}

/// Runs the hub entry, the click hook and the session hook in the game.
pub struct ArenaHubPlugin;

impl Plugin for ArenaHubPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ArenaHub>()
            .init_resource::<HubRoll>()
            .add_message::<WorldClick>()
            .add_message::<ArenaMatchLaunch>()
            .init_resource::<HubWindows>()
            .init_resource::<crate::hub_windows::ArenaCareer>()
            .init_resource::<crate::hub_windows::HubItemNames>()
            .init_resource::<LastHubClick>()
            .init_resource::<exa_ui::gui_draw::GuiFrames>()
            .init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<ButtonInput<MouseButton>>()
            .add_systems(
                Update,
                (
                    crate::hub_windows::sync_hub_screen,
                    enter_loaded_hub,
                    hub_clicks,
                    hub_session_fall,
                    close_hub_windows,
                    crate::hub_windows::draw_hub_windows,
                )
                    .chain(),
            );
    }
}

/// The manager as the player's body reads: its condition, red health and
/// the pelvis height in centimetres.
fn manager_reading(reading: Option<(&Condition, &Fighter, Option<&Ragdoll>)>) -> ManagerReading {
    let (condition, health, pelvis_cm) = reading.map_or(
        (Condition::NORMAL, 0.25, 0.0),
        |(condition, fighter, ragdoll)| {
            let pelvis = ragdoll
                .and_then(|ragdoll| ragdoll.parts.iter().find(|part| part.parent.is_none()))
                .map(|part| part.transform().translation.y * 100.0)
                .unwrap_or(0.0);
            (*condition, fighter.health.red, pelvis)
        },
    );
    ManagerReading {
        condition: condition.0,
        health,
        pelvis_cm,
    }
}

/// Enters the hub once when a play level named `arenahub` appears.
///
/// The entry draws the music from [`HubRoll`] and the flow's alternative
/// setting, writes the track through the audio crate's level-music
/// resources, installs the hooks and places the manager at the hub's known
/// start. The player root is moved to the manager's plan when it is there:
/// the plan's floor lift is the original's +5 cm.
#[allow(clippy::too_many_arguments)]
fn enter_loaded_hub(
    mut commands: Commands,
    levels: Query<&LevelRoot>,
    mut hub: ResMut<ArenaHub>,
    mut roll: ResMut<HubRoll>,
    flow: Option<Res<Flow>>,
    players: Query<&mut Transform, With<Player>>,
    bodies: Query<(&Condition, &Fighter, Option<&Ragdoll>), With<PlayerBody>>,
    mut career: ResMut<crate::hub_windows::ArenaCareer>,
) {
    let loaded = levels
        .iter()
        .any(|root| root.name.eq_ignore_ascii_case(HUB_LEVEL));
    if !loaded {
        hub.entered = false;
        return;
    }
    if hub.entered {
        return;
    }
    if flow.as_deref().is_some_and(|flow| {
        !matches!(
            flow.mode,
            crate::flow::Mode::Arena | crate::flow::Mode::Menu
        )
    }) {
        return;
    }
    let alternative = flow.is_some_and(|flow| flow.alternative_music);
    let reading = manager_reading(bodies.iter().next());
    let entry = enter_hub(&mut roll.0, alternative, reading.pelvis_cm);
    // The hub's own start is the known spot the entry places the manager on;
    // its floor is y 0 (the level's start puts the player at 0, 0, 3 m).
    let start = player_start(HUB_LEVEL).map(|start| start.position);
    let floor = start.map(|at| [at.x * 100.0, at.y * 100.0]);
    let plan = manager_plan(entry.player, floor);
    if let Some(at) = start {
        let manager = Vec3::new(at.x, plan.position[1] / 100.0, at.z);
        for mut transform in players {
            transform.translation = manager;
        }
    }
    hub.entered = true;
    hub.busy = entry.hub_busy;
    hub.click_hook = entry.click_hook;
    hub.session_hook = entry.session_hook;
    hub.manager = Some(plan);
    hub.fall = None;
    hub.roll = entry.roll;
    if let Some(index) = clear_physician_weapon(&mut career) {
        info!("arena hub: cleared the physician's weapon object {index}");
    }
    commands.insert_resource(exa_audio::LevelMusic(entry.music.name.to_ascii_lowercase()));
    commands.insert_resource(exa_audio::LevelMusicVolume(entry.music.volume));
    match entry.start {
        Some(at) => commands.insert_resource(exa_audio::LevelMusicStart(at)),
        None => commands.remove_resource::<exa_audio::LevelMusicStart>(),
    }
    info!(
        "arena hub entered: {} at {} from {:?}, roll {:#x}",
        entry.music.name, entry.music.volume, entry.start, entry.roll
    );
}

/// The click the hub last asked about, for the report and the tests.
#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct LastHubClick {
    pub action: Option<HubAction>,
}

#[derive(Message, Clone, Debug, PartialEq)]
pub struct ArenaMatchLaunch {
    pub template: i32,
    pub arena: usize,
    pub track: crate::arena_match::Track,
    /// The arena's spawn spot, centimetres.
    pub spawn_at: [f32; 3],
    /// The company entry that is the player's own, if any.
    pub player: Option<usize>,
    pub fighters: Vec<LaunchedFighter>,
}

/// One fighter a launch places.
#[derive(Clone, Debug, PartialEq)]
pub struct LaunchedFighter {
    pub place: crate::match_layout::Place,
    pub team: u32,
    pub body: LaunchedBody,
    /// The event weapon variant a hire-list opponent drew, if any.
    pub weapon: Option<u32>,
}

/// What a launched fighter's scene body is generated from.
#[derive(Clone, Debug, PartialEq)]
pub enum LaunchedBody {
    /// The player's own body, already in the scene.
    Player,
    /// Another company entry.
    Company { role: u32 },
    /// A generated opponent.
    Opponent(crate::match_layout::OpponentPlan),
    /// The Beast and its tier plan.
    Beast(crate::arena_spawn::BeastPlan),
}

/// The manager as the match's participant list sees it.
fn manager_roles(count: usize) -> Vec<u32> {
    (0..count)
        .map(|index| {
            if index == 0 {
                crate::arena_spawn::MANAGER_ROLE | 0x1000_0000
            } else {
                crate::arena_spawn::MANAGER_ROLE
            }
        })
        .collect()
}

/// Builds a match's launch from a board pick: the arena and music draw, the
/// layout, and each fighter's spawn plan.
///
/// A standard match's opponents are [`generic_plan`] at the template's level
/// and the event's gear tier; a tournament event's are [`hire_plan`]s with
/// the event's weapon row drawn by [`pick_weapon_slot`]; the Beast uses
/// [`beast_plan`]. The manager is [`player_participant`] among the company
/// entries.
pub fn plan_launch(
    career: &mut crate::hub_windows::ArenaCareer,
    board: &exa_ui::match_board::Board,
    alternative: bool,
) -> Option<ArenaMatchLaunch> {
    plan_launch_at(
        career,
        board,
        alternative,
        crate::arena_match::counter_now(),
    )
}

/// [`plan_launch`] with the performance counter given, for tests.
pub fn plan_launch_at(
    career: &mut crate::hub_windows::ArenaCareer,
    board: &exa_ui::match_board::Board,
    alternative: bool,
    counter: u64,
) -> Option<ArenaMatchLaunch> {
    if career.matches_today >= crate::arena_match::MATCHES_PER_DAY {
        return None;
    }
    let template = board.match_id() as i32;
    let start = crate::arena_match::start_match(&mut career.picks, template, counter, alternative);
    let spawn_at = crate::arena_match::ARENA_MAPS
        .get(start.arena)
        .map_or([0.0; 3], |arena| arena.spawn);
    let level = board.selected.rank.max(0);
    let tier = board.selected.tier;
    let slots = crate::match_layout::layout(template, level, tier, spawn_at);
    let roles = manager_roles(board.team.len());
    let player =
        (0..roles.len()).find_map(|index| crate::arena_spawn::player_participant(&roles, index));
    let tournament = board.tab == exa_ui::match_board::BoardTab::Tournament;
    let mut fighters = Vec::with_capacity(slots.len());
    for slot in slots {
        let (team, body, weapon) = match slot.who {
            crate::match_layout::Fighter::Ours(index) => {
                if index >= roles.len() {
                    continue;
                }
                let body = if player == Some(index) {
                    LaunchedBody::Player
                } else {
                    LaunchedBody::Company { role: roles[index] }
                };
                (crate::arena_match::PLAYER_TEAM as u32, body, None)
            }
            crate::match_layout::Fighter::Opponent {
                level, tier, team, ..
            } => {
                if tournament {
                    let event_weapon = board.selected.weapon_row;
                    let plan = crate::arena_spawn::hire_plan(team as u8, 0, event_weapon);
                    let weapon = if plan.draw_weapon {
                        board.weapon_row(tier.max(0) as usize).map(|row| {
                            career.roll = crate::trade::step(career.roll);
                            crate::arena_spawn::pick_weapon_slot(row.variants, career.roll)
                        })
                    } else {
                        None
                    };
                    (
                        team,
                        LaunchedBody::Opponent(crate::match_layout::OpponentPlan::Hire(plan)),
                        weapon,
                    )
                } else {
                    let plan = crate::arena_spawn::generic_plan(level, tier);
                    (
                        team,
                        LaunchedBody::Opponent(crate::match_layout::OpponentPlan::Generic(plan)),
                        None,
                    )
                }
            }
            crate::match_layout::Fighter::Beast { tier, .. } => (
                crate::arena_spawn::OPPONENT_TEAM,
                LaunchedBody::Beast(crate::arena_spawn::beast_plan(
                    u32::try_from(tier).unwrap_or(1).max(1),
                    0.0,
                )),
                None,
            ),
        };
        fighters.push(LaunchedFighter {
            place: slot.place,
            team,
            body,
            weapon,
        });
    }
    Some(ArenaMatchLaunch {
        template,
        arena: start.arena,
        track: start.track,
        spawn_at,
        player,
        fighters,
    })
}

/// Removes the first weapon object within ten centimetres of the physician's
/// hub spot, as arriving at the hub does. The host supplies the level's
/// items; the play-mode hub has none to give yet.
pub fn clear_physician_weapon(career: &mut crate::hub_windows::ArenaCareer) -> Option<usize> {
    let at = crate::arena_spawn::find_nearby_weapon(
        &career.venue_items,
        crate::arena_spawn::PHYSICIAN_ITEM_POINT,
    )?;
    career.venue_items.remove(at);
    Some(at)
}

/// The mode word the click hook compares: the flow's mode, else the arena's
/// while a hub level is loaded (play mode has no loaded session yet).
fn hub_mode(flow: Option<&Flow>, hub_loaded: bool) -> u32 {
    match flow {
        Some(flow) if flow.mode == crate::flow::Mode::Menu && hub_loaded => ARENA_MODE,
        Some(flow) => flow.mode.word(),
        None if hub_loaded => ARENA_MODE,
        None => 0,
    }
}

/// Whether a window that blocks the click hook is open: the trade window's
/// form tag (the original's 0xce1c2023), the board or the manage window.
fn confirming(windows: &HubWindows) -> bool {
    windows.is_open()
}

/// The click hook: the object or character under the click, the action it
/// answers and the window it opens.
#[allow(clippy::too_many_arguments)]
fn hub_clicks(
    mut clicks: MessageReader<WorldClick>,
    mut hub: ResMut<ArenaHub>,
    flow: Option<Res<Flow>>,
    levels: Query<&LevelRoot>,
    names: Query<&Name>,
    transforms: Query<&GlobalTransform>,
    characters: Query<(&Condition, &exa_character::record::CharacterData)>,
    mut windows: ResMut<HubWindows>,
    mut career: ResMut<crate::hub_windows::ArenaCareer>,
    item_names: Res<crate::hub_windows::HubItemNames>,
    mut launches: MessageWriter<ArenaMatchLaunch>,
    mut last: ResMut<LastHubClick>,
) {
    let hub_loaded = levels
        .iter()
        .any(|root| root.name.eq_ignore_ascii_case(HUB_LEVEL));
    if !hub.entered && !hub_loaded {
        clicks.clear();
        return;
    }
    let mode = hub_mode(flow.as_deref(), hub_loaded);
    for click in clicks.read() {
        // A window takes the click while it is open: the board's own hit
        // test, the trade button, or a press outside it (which closes the
        // modal form, as the window manager says).
        if windows.is_open() {
            let inside = windows
                .bounds()
                .is_some_and(|bounds| bounds.contains(click.pointer.x, click.pointer.y));
            if inside {
                if let Some(bounds) = windows.bounds() {
                    let local = (click.pointer.x - bounds.left, click.pointer.y - bounds.top);
                    match windows.kind {
                        HubWindowKind::Board => {
                            let action = windows.board_click(local);
                            if matches!(
                                action,
                                exa_ui::match_board::BoardAction::EnterMatch
                                    | exa_ui::match_board::BoardAction::BeginTournament
                                    | exa_ui::match_board::BoardAction::ContinueTournament
                            ) {
                                let alternative =
                                    flow.as_deref().is_some_and(|flow| flow.alternative_music);
                                let board = windows.board.clone();
                                if let Some(board) = board
                                    && let Some(launch) =
                                        plan_launch(&mut career, &board, alternative)
                                {
                                    launches.write(launch);
                                    windows.close_window();
                                    hub.busy = true;
                                }
                            } else if action == exa_ui::match_board::BoardAction::Close {
                                windows.close_window();
                                hub.busy = false;
                            }
                        }
                        HubWindowKind::Shop => {
                            let _ = windows.shop_click(local, &mut career);
                        }
                        HubWindowKind::Manage | HubWindowKind::None => {}
                    }
                }
            } else {
                windows.close_window();
                hub.busy = false;
            }
            continue;
        }
        let action = match click.hit {
            crate::hand::Hit::Prop { body, .. } => {
                let name = names.get(body).ok().map(|name| hub_name(name.as_bytes()));
                let side = transforms
                    .get(body)
                    .map(|transform| transform.translation().x)
                    .unwrap_or(0.0);
                match name.as_ref() {
                    Some(name) => hub_click(
                        hub.busy,
                        mode,
                        confirming(&windows),
                        HubTarget::Object { name, side },
                    ),
                    None => HubAction::Nothing,
                }
            }
            crate::hand::Hit::Character { body, .. } => match characters.get(body) {
                Ok((condition, data)) => hub_click(
                    hub.busy,
                    mode,
                    confirming(&windows),
                    HubTarget::Character {
                        condition: condition.0,
                        // The record's profession nibble 4 is the merchant.
                        merchant: data.record.profession() >> 28 == 4,
                    },
                ),
                Err(_) => HubAction::Nothing,
            },
            crate::hand::Hit::Level => HubAction::Nothing,
        };
        last.action = Some(action);
        match action {
            HubAction::Nothing => {}
            HubAction::Shop => {
                windows.open_shop(false, &career, &item_names);
            }
            HubAction::MerchantStock => {
                windows.open_shop(true, &career, &item_names);
            }
            HubAction::MatchBoard => {
                windows.open_board(&career);
            }
            HubAction::ManageCompany => {
                windows.open_manage(&career);
                hub.busy = true;
            }
        }
    }
}

/// The original's sixteen-byte scene name: the executable's copies begin
/// with 0xAA. Exarust's level objects carry the object-database name, so the
/// byte is put back for the comparison. A name that already begins with it
/// is left alone.
fn hub_name(name: &[u8]) -> [u8; 16] {
    let mut out = [0u8; 16];
    let (first, rest) = if name.first() == Some(&0xAA) {
        (0xAA, &name[1..])
    } else {
        (0xAA, name)
    };
    out[0] = first;
    let len = rest.len().min(15);
    out[1..1 + len].copy_from_slice(&rest[..len]);
    out
}

/// The session hook each frame: a dead manager ends the hub session, and
/// the wipe reaches [`Slots::drops_after_death`] when the host supplied the
/// slot.
fn hub_session_fall(
    mut hub: ResMut<ArenaHub>,
    flow: Option<ResMut<Flow>>,
    slots: Option<Res<HubSlots>>,
    bodies: Query<(&Condition, &Fighter), With<PlayerBody>>,
) {
    if !hub.entered || hub.fall.is_some() {
        return;
    }
    let Some(mut flow) = flow else {
        return;
    };
    if !matches!(
        flow.mode,
        crate::flow::Mode::Arena | crate::flow::Mode::Menu
    ) {
        return;
    }
    let Some((condition, fighter)) = bodies.iter().next() else {
        return;
    };
    let answer = hub_fall(flow.phase.word(), condition.0, fighter.health.red);
    let Some(wipe) = answer else {
        return;
    };
    hub.fall = Some(HubFall { wipe });
    if wipe && let Some(slots) = slots {
        match slots
            .slots
            .drops_after_death(slots.number, fighter.health.red)
        {
            Ok(dropped) => info!("arena hub: the manager died, slot dropped {dropped}"),
            Err(error) => warn!("arena hub: cannot wipe the slot: {error}"),
        }
    }
    flow.phase = Phase::Ended;
    flow.hook = Hook::None;
    info!("arena hub: the manager fell; the session ends (wipe {wipe})");
}

/// Escape and the windows' own close end the open hub window.
fn close_hub_windows(
    keys: Res<ButtonInput<KeyCode>>,
    buttons: Res<ButtonInput<MouseButton>>,
    mut windows: ResMut<HubWindows>,
    mut hub: ResMut<ArenaHub>,
) {
    if windows.is_open()
        && (keys.just_pressed(KeyCode::Escape) || buttons.just_released(MouseButton::Right))
    {
        windows.close_window();
        hub.busy = false;
    }
}

/// The slot kind a hub wipe deletes: the save file (the checkpoint stays).
pub const HUB_SLOT_KIND: exa_formats::save_package::SlotKind =
    exa_formats::save_package::SlotKind::Save;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::arena_hub::same_name;
    use crate::hand::Hit;
    use crate::hub_windows::HubWindowKind;

    fn hub_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins).add_plugins(ArenaHubPlugin);
        app
    }

    fn spawn_hub_level(app: &mut App) {
        app.world_mut().spawn(LevelRoot {
            name: HUB_LEVEL.into(),
        });
    }

    fn spawn_manager(app: &mut App, condition: u32, health: f32) {
        let mut fighter = Fighter::default();
        fighter.health.red = health;
        app.world_mut()
            .spawn((PlayerBody::new(0.0), Condition(condition), fighter));
    }

    #[test]
    fn the_plugin_runs_three_frames_in_arena_and_story_modes() {
        for mode in [crate::flow::Mode::Arena, crate::flow::Mode::Story] {
            let mut app = hub_app();
            let mut flow = Flow::default();
            flow.mode = mode;
            flow.phase = Phase::Playing;
            app.insert_resource(flow);
            spawn_hub_level(&mut app);
            for _ in 0..3 {
                app.update();
            }
            let hub = app.world().resource::<ArenaHub>();
            assert_eq!(hub.entered, mode == crate::flow::Mode::Arena);
        }
    }

    #[test]
    fn the_entry_draws_the_hub_music_once_and_installs_the_hooks() {
        let mut app = hub_app();
        let mut flow = Flow::default();
        flow.mode = crate::flow::Mode::Arena;
        flow.phase = Phase::Playing;
        flow.alternative_music = true;
        app.insert_resource(flow);
        app.insert_resource(HubRoll(0));
        spawn_hub_level(&mut app);
        app.update();
        let hub = app.world().resource::<ArenaHub>();
        assert!(hub.entered);
        assert_eq!(hub.click_hook, HubHook::Click);
        assert_eq!(hub.session_hook, HubHook::Session);
        assert_eq!(hub.roll, 0x0808_8406);
        assert_eq!(
            app.world().resource::<exa_audio::LevelMusic>().0,
            "msc_hub01_dnb"
        );
        assert_eq!(app.world().resource::<exa_audio::LevelMusicVolume>().0, 0.1);
        // A second frame does not enter again.
        let before = app.world().resource::<ArenaHub>().clone();
        app.world_mut().resource_mut::<HubRoll>().0 = 0xF000_0000;
        app.update();
        assert_eq!(app.world().resource::<ArenaHub>(), &before);
    }

    #[test]
    fn the_manager_is_placed_five_centimetres_over_the_hub_floor() {
        let mut app = hub_app();
        let mut flow = Flow::default();
        flow.mode = crate::flow::Mode::Arena;
        flow.phase = Phase::Playing;
        app.insert_resource(flow);
        spawn_hub_level(&mut app);
        app.update();
        let plan = app.world().resource::<ArenaHub>().manager.unwrap();
        assert_eq!(plan.position, [0.0, 5.0, 300.0]);
        assert_eq!(plan.team, 1);
        assert_eq!(plan.role, 0x11);
    }

    #[test]
    fn a_dead_manager_ends_the_session_and_asks_for_the_wipe() {
        let mut app = hub_app();
        let mut flow = Flow::default();
        flow.mode = crate::flow::Mode::Arena;
        flow.phase = Phase::Playing;
        app.insert_resource(flow);
        spawn_hub_level(&mut app);
        spawn_manager(&mut app, 0xA00, 0.0);
        app.update();
        assert_eq!(
            app.world().resource::<ArenaHub>().fall,
            Some(HubFall { wipe: true })
        );
        assert_eq!(app.world().resource::<Flow>().phase, Phase::Ended);
    }

    #[test]
    fn a_living_manager_leaves_the_session_alone() {
        let mut app = hub_app();
        let mut flow = Flow::default();
        flow.mode = crate::flow::Mode::Arena;
        flow.phase = Phase::Playing;
        app.insert_resource(flow);
        spawn_hub_level(&mut app);
        spawn_manager(&mut app, 0, 0.2);
        app.update();
        assert_eq!(app.world().resource::<ArenaHub>().fall, None);
        assert_eq!(app.world().resource::<Flow>().phase, Phase::Playing);
    }

    #[test]
    fn a_shop_click_opens_the_shop_a_machine_the_board_or_manage() {
        for (side, want) in [
            (1.0, HubAction::MatchBoard),
            (-1.0, HubAction::ManageCompany),
        ] {
            let mut app = hub_app();
            let mut flow = Flow::default();
            flow.mode = crate::flow::Mode::Arena;
            flow.phase = Phase::Playing;
            app.insert_resource(flow);
            spawn_hub_level(&mut app);
            app.update();
            let shop = app
                .world_mut()
                .spawn((Name::new("ArenaShop"), GlobalTransform::default()))
                .id();
            app.world_mut().write_message(WorldClick::picked(Hit::Prop {
                body: shop,
                part: 0,
            }));
            app.update();
            assert_eq!(
                app.world().resource::<LastHubClick>().action,
                Some(HubAction::Shop)
            );
            assert!(
                app.world()
                    .resource::<HubWindows>()
                    .is_kind(HubWindowKind::Shop)
            );

            let machine = app
                .world_mut()
                .spawn((
                    Name::new("ArenaMach"),
                    GlobalTransform::from_xyz(side, 0.0, 0.0),
                ))
                .id();
            app.world_mut().write_message(WorldClick::picked(Hit::Prop {
                body: machine,
                part: 0,
            }));
            // The shop is still open (it is the confirming form), so close it.
            app.world_mut().resource_mut::<HubWindows>().close();
            app.update();
            app.world_mut().write_message(WorldClick::picked(Hit::Prop {
                body: machine,
                part: 0,
            }));
            app.update();
            assert_eq!(app.world().resource::<LastHubClick>().action, Some(want));
        }
    }

    #[test]
    fn a_busy_or_confirming_hub_opens_nothing() {
        let mut app = hub_app();
        let mut flow = Flow::default();
        flow.mode = crate::flow::Mode::Arena;
        flow.phase = Phase::Playing;
        app.insert_resource(flow);
        spawn_hub_level(&mut app);
        app.update();
        let shop = app
            .world_mut()
            .spawn((Name::new("ArenaShop"), GlobalTransform::default()))
            .id();
        app.world_mut().resource_mut::<ArenaHub>().busy = true;
        assert_eq!(
            hub_click(
                true,
                ARENA_MODE,
                false,
                HubTarget::Object {
                    name: &crate::arena_hub::SHOP_NAME,
                    side: 0.0
                }
            ),
            HubAction::Nothing
        );
        app.world_mut().write_message(WorldClick::picked(Hit::Prop {
            body: shop,
            part: 0,
        }));
        app.update();
        assert_eq!(
            app.world().resource::<LastHubClick>().action,
            Some(HubAction::Nothing)
        );
        assert!(
            app.world()
                .resource::<HubWindows>()
                .is_kind(HubWindowKind::None)
        );
    }

    #[test]
    fn an_open_confirmation_takes_the_click_and_opens_no_second_window() {
        let mut app = hub_app();
        let mut flow = Flow::default();
        flow.mode = crate::flow::Mode::Arena;
        flow.phase = Phase::Playing;
        app.insert_resource(flow);
        spawn_hub_level(&mut app);
        app.update();
        {
            let career = app
                .world()
                .resource::<crate::hub_windows::ArenaCareer>()
                .clone();
            let names = app
                .world()
                .resource::<crate::hub_windows::HubItemNames>()
                .clone();
            app.world_mut()
                .resource_mut::<HubWindows>()
                .open_shop(false, &career, &names);
        }
        let bounds = app.world().resource::<HubWindows>().bounds().unwrap();
        let shop = app
            .world_mut()
            .spawn((Name::new("ArenaShop"), GlobalTransform::default()))
            .id();
        app.world_mut().write_message(WorldClick {
            hit: Hit::Prop {
                body: shop,
                part: 0,
            },
            pointer: IVec2::new(
                (bounds.left + bounds.right) / 2,
                (bounds.top + bounds.bottom) / 2,
            ),
        });
        app.update();
        assert!(
            app.world()
                .resource::<HubWindows>()
                .is_kind(HubWindowKind::Shop)
        );
        assert_eq!(app.world().resource::<LastHubClick>().action, None);
    }

    #[test]
    fn a_merchant_character_opens_his_stock_and_a_helpless_one_does_not() {
        for (condition, want) in [(0, HubAction::MerchantStock), (0xAC0, HubAction::Nothing)] {
            let mut app = hub_app();
            let mut flow = Flow::default();
            flow.mode = crate::flow::Mode::Arena;
            flow.phase = Phase::Playing;
            app.insert_resource(flow);
            spawn_hub_level(&mut app);
            let mut data = exa_character::record::CharacterData::new();
            data.record.set_profession(0x4000_0000);
            let merchant = app.world_mut().spawn((Condition(condition), data)).id();
            app.update();
            app.world_mut()
                .write_message(WorldClick::picked(Hit::Character {
                    body: merchant,
                    part: 0,
                    dead: false,
                }));
            app.update();
            assert_eq!(app.world().resource::<LastHubClick>().action, Some(want));
        }
    }

    #[test]
    fn escape_closes_the_window_and_clears_the_busy_flag() {
        let mut app = hub_app();
        app.init_resource::<ButtonInput<KeyCode>>();
        let mut flow = Flow::default();
        flow.mode = crate::flow::Mode::Arena;
        flow.phase = Phase::Playing;
        app.insert_resource(flow);
        spawn_hub_level(&mut app);
        app.update();
        app.world_mut().resource_mut::<ArenaHub>().busy = true;
        app.world_mut()
            .resource_mut::<HubWindows>()
            .open_manage(&crate::hub_windows::ArenaCareer::default());
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Escape);
        app.update();
        assert!(
            app.world()
                .resource::<HubWindows>()
                .is_kind(HubWindowKind::None)
        );
        assert!(!app.world().resource::<ArenaHub>().busy);
    }

    #[test]
    fn the_scene_name_puts_the_executables_byte_back() {
        assert!(same_name(
            &hub_name(b"ArenaShop"),
            &crate::arena_hub::SHOP_NAME
        ));
        assert!(same_name(
            &hub_name(&[0xAA, b'A', b'r', b'e', b'n', b'a', b'S', b'h', b'o', b'p']),
            &crate::arena_hub::SHOP_NAME
        ));
        assert!(!same_name(
            &hub_name(b"ArenaMach"),
            &crate::arena_hub::SHOP_NAME
        ));
    }

    #[test]
    fn a_board_pick_draws_the_arena_track_and_generic_plans() {
        let mut career = crate::hub_windows::ArenaCareer::default();
        let board = career.board();
        let launch = plan_launch_at(&mut career, &board, false, 0x1234_5678).unwrap();
        assert_eq!(launch.template, 1);
        assert_eq!(launch.arena, 4);
        assert_eq!(launch.track.name, "Msc_Arena3");
        assert_eq!(launch.spawn_at, crate::arena_match::ARENA_MAPS[4].spawn);
        // The manager is the player's own entry and the company is one.
        assert_eq!(launch.player, Some(0));
        assert_eq!(launch.fighters.len(), 2);
        // The layout stands the opponent first, then the company's entry.
        let LaunchedBody::Opponent(crate::match_layout::OpponentPlan::Generic(plan)) =
            launch.fighters[0].body
        else {
            panic!("an opponent");
        };
        // The board's Duel has rank floor 0 and tier 0 at a new career.
        assert_eq!((plan.level, plan.outfit_level), (0, 0));
        assert_eq!(launch.fighters[0].team, 2);
        assert_eq!(launch.fighters[1].body, LaunchedBody::Player);
    }

    #[test]
    fn a_tournament_launch_uses_hire_plans_and_the_weapon_row() {
        let mut career = crate::hub_windows::ArenaCareer::default();
        // A tournament day with the tournament tab open and an event whose
        // weapon row is set.
        career.company.day = 0;
        let mut board = career.board();
        board.tab = exa_ui::match_board::BoardTab::Tournament;
        board.selected = exa_ui::match_board::STANDARD_MATCHES[0].record();
        board.selected.kind = 9;
        board.selected.team_size = 3;
        board.selected.rank = 0;
        board.selected.tier = 0;
        board.selected.weapon_row = 1;
        board.rebuild_lineups();
        let launch = plan_launch_at(&mut career, &board, true, 0x1000_0000).unwrap();
        let hires: Vec<_> = launch
            .fighters
            .iter()
            .filter(|fighter| {
                matches!(
                    fighter.body,
                    LaunchedBody::Opponent(crate::match_layout::OpponentPlan::Hire(_))
                )
            })
            .collect();
        assert!(!hires.is_empty());
        for fighter in hires {
            assert!(matches!(
                fighter.body,
                LaunchedBody::Opponent(crate::match_layout::OpponentPlan::Hire(_))
            ));
            assert!(fighter.weapon.is_some());
        }
    }

    #[test]
    fn the_board_runs_three_matches_a_day() {
        let mut career = crate::hub_windows::ArenaCareer {
            matches_today: crate::arena_match::MATCHES_PER_DAY,
            ..Default::default()
        };
        let board = career.board();
        assert_eq!(plan_launch_at(&mut career, &board, false, 0), None);
    }

    #[test]
    fn the_physician_weapon_is_cleared_from_the_hosts_items() {
        let mut career = crate::hub_windows::ArenaCareer {
            venue_items: vec![
                crate::arena_spawn::PlacedItem {
                    position: [-2100.0, 0.0],
                    type_bits: 0x4000,
                    template_kind: 0x40000,
                },
                crate::arena_spawn::PlacedItem {
                    position: [1.0, 2.0],
                    type_bits: 0x4000,
                    template_kind: 0x40000,
                },
            ],
            ..Default::default()
        };
        assert_eq!(clear_physician_weapon(&mut career), Some(0));
        assert_eq!(career.venue_items.len(), 1);
        assert_eq!(clear_physician_weapon(&mut career), None);
    }
}
