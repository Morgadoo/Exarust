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
use exa_world::start::player_start;
use exa_world::{LevelRoot, PartBounds};

use crate::arena_hub::{ARENA_MODE, HubAction, HubHook, HubTarget, enter_hub, hub_click, hub_fall};
use crate::arena_spawn::manager_plan;
use crate::flow::{Flow, Hook, Phase};
use crate::hand::WorldClick;
use crate::hub_windows::{HubWindowKind, HubWindows};
use crate::player::{Player, PlayerBody};
use crate::slots::Slots;

/// The level the hub lives in.
pub const HUB_LEVEL: &str = "arenahub";

/// Whether a `LevelRoot` name is the hub's: the loader keeps the `.rfc`
/// suffix.
pub fn is_hub_level(name: &str) -> bool {
    name.strip_suffix(".rfc")
        .unwrap_or(name)
        .eq_ignore_ascii_case(HUB_LEVEL)
}

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
    /// The player root took the manager's plan once it spawned.
    pub manager_placed: bool,
    /// The session hook's answer, once the manager fell.
    pub fall: Option<HubFall>,
    /// The placement a settled match returned the company to: the hub puts
    /// the manager there instead of its own start.
    pub returned: Option<crate::arena_session::ReturnPlacement>,
    /// The shopkeeper and the specialists' bodies were asked for.
    pub npcs_spawned: bool,
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
            manager_placed: false,
            fall: None,
            returned: None,
            npcs_spawned: false,
            roll: 0,
        }
    }
}

impl ArenaHub {
    /// The level changed: the entry must run again for the destination. The
    /// settled return placement stays, since the hub it belongs to is the
    /// destination of that change.
    pub fn level_changed(&mut self) {
        let returned = self.returned;
        *self = Self {
            returned,
            ..Self::default()
        };
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
/// byte-reversed performance counter, as the match start's draws do.
#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
pub struct HubRoll(pub u32);

impl Default for HubRoll {
    fn default() -> Self {
        Self((crate::arena_match::counter_now() as u32).swap_bytes())
    }
}

/// The manager's body as the hub's hooks read it.
#[derive(Clone, Copy, Debug, PartialEq)]
struct ManagerReading {
    condition: u32,
    health: f32,
    pelvis_cm: f32,
}

/// The hub's entry, click, window and session systems, which the windows'
/// draw follows.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct HubWindowSystems;

/// Runs the hub entry, the click hook and the session hook in the game.
pub struct ArenaHubPlugin;

impl Plugin for ArenaHubPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ArenaHub>()
            .init_resource::<HubRoll>()
            .add_message::<WorldClick>()
            .add_message::<ArenaMatchLaunch>()
            .add_message::<crate::scene_bodies::SceneCommand>()
            .init_resource::<HubWindows>()
            .init_resource::<crate::hub_windows::ArenaCareer>()
            .init_resource::<crate::hub_windows::HubItemNames>()
            .init_resource::<crate::shop_stock::LiveShopStock>()
            .init_resource::<LastHubClick>()
            .init_resource::<crate::hub_forms::OpenHubWindow>()
            .init_resource::<ArenaRematch>()
            .init_resource::<crate::arena_live::ArenaAutoContinue>()
            .init_resource::<exa_ui::gui_draw::GuiFrames>()
            .init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<ButtonInput<MouseButton>>()
            .add_systems(
                Update,
                (
                    crate::hub_windows::sync_hub_screen,
                    enter_loaded_hub,
                    auto_rematch,
                    crate::hub_forms::open_hub_window_on_load,
                    crate::hub_forms::hub_form_input,
                    hub_clicks,
                    hub_session_fall,
                    close_hub_windows,
                )
                    .chain()
                    .in_set(HubWindowSystems),
            );
        crate::hub_windows::add_hub_draw(app);
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
    mut scene_commands: MessageWriter<crate::scene_bodies::SceneCommand>,
) {
    let loaded = levels.iter().any(|root| is_hub_level(&root.name));
    if !loaded {
        hub.entered = false;
        hub.manager_placed = false;
        return;
    }
    // The play view's practice stand-in counts as the arena while the hub
    // is loaded, as the click hook reads it; story mode does not.
    if hub_mode(flow.as_deref(), true) != ARENA_MODE {
        return;
    }
    if !hub.entered {
        let alternative = flow.is_some_and(|flow| flow.alternative_music);
        let reading = manager_reading(bodies.iter().next());
        let entry = enter_hub(&mut roll.0, alternative, reading.pelvis_cm);
        // The hub's own start is the known spot the entry places the manager
        // on; its floor is y 0 (the level's start puts the player at 0, 0,
        // 3 m).
        let start = player_start(HUB_LEVEL).map(|start| start.position);
        let floor = start.map(|at| [at.x * 100.0, at.y * 100.0]);
        let plan = manager_plan(entry.player, floor);
        hub.entered = true;
        hub.busy = entry.hub_busy;
        hub.click_hook = entry.click_hook;
        hub.session_hook = entry.session_hook;
        hub.manager = Some(plan);
        hub.manager_placed = false;
        hub.npcs_spawned = false;
        hub.fall = None;
        hub.roll = entry.roll;
        // Only a physician clears the weapon object at the hub's spot.
        if career
            .company
            .roster
            .iter()
            .any(|member| member.profession == crate::career::Profession::Physician)
            && let Some(index) = clear_physician_weapon(&mut career)
        {
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
    // The player spawns after the level's collision is built; place it when
    // it first appears rather than only on the entry frame.
    if !hub.manager_placed
        && let (Some(plan), Some(at)) = (hub.manager, player_start(HUB_LEVEL).map(|s| s.position))
    {
        // A settled match's return puts the company back at the saved
        // placement: 300 cm out along the hub's entry direction (the hub's
        // own start), a half turn round. Both are the same spot, which is
        // why the entry's start and the return agree. The half turn is the
        // hub start's own facing (towards -z), which the player's body
        // already carries; the root, which holds only the position, is not
        // turned again.
        let distance = hub
            .returned
            .map_or(at.z, |placement| placement.distance / 100.0);
        let manager = Vec3::new(at.x, plan.position[1] / 100.0, distance);
        let mut placed = false;
        for mut transform in players {
            transform.translation = manager;
            placed = true;
        }
        if placed {
            info!("arena hub: the manager stands at {manager}");
        }
        hub.manager_placed = placed;
    }
    // The shopkeeper and one body per employed specialist stand at their
    // planned spots once the manager is there. The original finds the
    // shopkeeper's actor record by name and creates the specialists' bodies
    // from the employees' records.
    if hub.manager_placed && !hub.npcs_spawned {
        let mut plans = vec![crate::scene_bodies::hub_body_plan(
            &crate::arena_spawn::shopkeeper_plan(),
            true,
        )];
        for member in &career.company.roster {
            if let Some(spot) = crate::arena_spawn::specialist_spot(member.profession) {
                let mut plan = crate::scene_bodies::hub_body_plan(&spot, false);
                plan.name = Some(member.name.clone());
                plans.push(plan);
            }
        }
        let count = plans.len();
        for plan in plans {
            scene_commands.write(crate::scene_bodies::SceneCommand::Spawn(Box::new(plan)));
        }
        hub.npcs_spawned = true;
        info!("arena hub: {count} bodies asked for (shopkeeper and specialists)");
    }
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
    /// The event's rank: the generic spawn plans' character level.
    pub level: i32,
    /// The event's gear tier.
    pub tier: i32,
    /// The company entry that is the player's own, if any.
    pub player: Option<usize>,
    /// A tournament event's match.
    pub tournament: bool,
    /// The prize a win pays, copper.
    pub prize: u32,
    /// The event weapon row's three variants, when the event arms its
    /// fighters (the bracket's later fights draw from it).
    pub weapon_variants: [u32; 3],
    /// The week's tournament field: the rival company each team from 2 on
    /// wears.
    pub rivals: [i32; crate::career::RIVAL_FIELD],
    pub fighters: Vec<LaunchedFighter>,
}

/// One fighter a launch places.
#[derive(Clone, Debug, PartialEq)]
pub struct LaunchedFighter {
    pub place: crate::match_layout::Place,
    pub team: u32,
    pub body: LaunchedBody,
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
/// and the event's gear tier; a tournament event's are [`hire_plan`]s at the
/// event's tier wearing the team's rival company, whose weapon the record's
/// generation draws from the event row; the Beast uses
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
    // The match start takes the event's entry fee from the purse (a short
    // purse stops at 0; nothing refuses the match) and adds the whole fee
    // to the day's spending.
    let fee = board.selected.fee;
    career.company.copper = career.company.copper.saturating_sub(fee);
    career.company.spent = career.company.spent.saturating_add(fee);
    plan_match(career, board, alternative, counter)
}

/// The match start after the board's checks and fee: the arena and track
/// draw, the layout and each fighter's spawn plan. The auto rematch starts
/// here, with no fee and no day limit.
fn plan_match(
    career: &mut crate::hub_windows::ArenaCareer,
    board: &exa_ui::match_board::Board,
    alternative: bool,
    counter: u64,
) -> Option<ArenaMatchLaunch> {
    let template = board.match_id() as i32;
    let start = crate::arena_match::start_match(&mut career.picks, template, counter, alternative);
    let spawn_at = crate::arena_match::ARENA_MAPS
        .get(start.arena)
        .map_or([0.0; 3], |arena| arena.spawn);
    let level = board.selected.rank.max(0);
    let tier = board.selected.tier;
    let slots = crate::match_layout::layout(template, level, tier, spawn_at);
    let roles = manager_roles(board.team.len());
    // The player's own body is the team member whose pool slot is 0 (the
    // manager), not merely the team's first member.
    let player = board.team.iter().position(|&slot| slot == 0);
    let tournament = board.tab == exa_ui::match_board::BoardTab::Tournament;
    let rivals = if tournament {
        career.rival_field()
    } else {
        career.rivals
    };
    let mut fighters = Vec::with_capacity(slots.len());
    for slot in slots {
        let (team, body) = match slot.who {
            crate::match_layout::Fighter::Ours(index) => {
                if index >= roles.len() {
                    continue;
                }
                let body = if player == Some(index) {
                    LaunchedBody::Player
                } else {
                    LaunchedBody::Company { role: roles[index] }
                };
                (crate::arena_match::PLAYER_TEAM as u32, body)
            }
            crate::match_layout::Fighter::Opponent {
                level, tier, team, ..
            } => {
                if tournament {
                    // The profile's level byte is the event's tier and its
                    // forced outfit the team's rival company.
                    let plan = crate::arena_spawn::hire_plan(
                        tier as u8,
                        crate::arena_spawn::rival_company(&rivals, team as i32),
                        board.selected.weapon_row,
                    );
                    (
                        team,
                        LaunchedBody::Opponent(crate::match_layout::OpponentPlan::Hire(plan)),
                    )
                } else {
                    let plan = crate::arena_spawn::generic_plan(level, tier);
                    (
                        team,
                        LaunchedBody::Opponent(crate::match_layout::OpponentPlan::Generic(plan)),
                    )
                }
            }
            crate::match_layout::Fighter::Beast { tier, .. } => (
                crate::arena_spawn::OPPONENT_TEAM,
                LaunchedBody::Beast(crate::arena_spawn::beast_plan(
                    u32::try_from(tier).unwrap_or(1).max(1),
                    0.0,
                )),
            ),
        };
        fighters.push(LaunchedFighter {
            place: slot.place,
            team,
            body,
        });
    }
    let weapon_variants = if tournament && board.selected.weapon_row > 0 {
        board
            .weapon_row(tier.max(0) as usize)
            .map(|row| row.variants)
            .unwrap_or([0; 3])
    } else {
        [0; 3]
    };
    Some(ArenaMatchLaunch {
        template,
        arena: start.arena,
        track: start.track,
        spawn_at,
        level,
        tier,
        player,
        tournament,
        prize: board.selected.prize,
        weapon_variants,
        rivals,
        fighters,
    })
}

/// Timer ticks the auto rematch waits after the return to the hub.
pub const REMATCH_TICKS: u64 = 40;
/// Real-time milliseconds per timer tick.
pub const TIMER_TICK_MS: u64 = 100;

/// The original's dormant auto-match switch ([`crate::arena_live::ArenaAutoContinue`],
/// `--arena-auto`): a round's end continues by itself and, 40 timer ticks
/// (4 s of real time) after the return to the hub, the last match starts
/// again with the company's combatants. The retail switch is never set by
/// the game's own code; Exarust exposes it for unattended runs.
#[derive(Resource, Clone, Debug, Default)]
pub struct ArenaRematch {
    /// The board the last launch came from: its selected match is replayed.
    pub board: Option<exa_ui::match_board::Board>,
    /// The real-time millisecond the rematch is due.
    pub due_ms: Option<u64>,
    /// This hub entry already scheduled (or skipped) its rematch.
    pub scheduled: bool,
}

/// The rematch's lineup: `per_side` participants taken in staff order after
/// the manager, every recruit or combatant (class below 4,
/// [`crate::arena_session::meet_participants`]), as board pool slots (the
/// manager is slot 0, roster member `i` slot `i + 1`). A short staff leaves
/// the rest of the original's list empty, so no slot stands for it.
pub fn rematch_team(
    per_side: usize,
    manager: crate::career::Profession,
    roster: &[crate::career::Profession],
) -> Vec<usize> {
    let staff: Vec<(i32, u8)> = std::iter::once(manager)
        .chain(roster.iter().copied())
        .enumerate()
        .map(|(slot, profession)| {
            let class = match profession {
                crate::career::Profession::Combatant => crate::staff::COMBATANT,
                crate::career::Profession::Merchant => crate::staff::MERCHANT,
                crate::career::Profession::Physician => crate::staff::PHYSICIAN,
                crate::career::Profession::Trainer => crate::staff::TRAINER,
            };
            (slot as i32, class)
        })
        .collect();
    crate::arena_session::meet_participants(per_side, &staff)
        .into_iter()
        .filter(|&slot| slot > 0)
        .map(|slot| slot as usize)
        .collect()
}

/// The auto switch's rematch: once per hub entry it is scheduled
/// [`REMATCH_TICKS`] timer ticks of real time ahead when a match was
/// launched before; when due, the last board's match starts again with
/// [`rematch_team`]'s lineup.
#[allow(clippy::too_many_arguments)]
fn auto_rematch(
    auto: Option<Res<crate::arena_live::ArenaAutoContinue>>,
    mut hub: ResMut<ArenaHub>,
    time: Option<Res<Time<Real>>>,
    mut rematch: ResMut<ArenaRematch>,
    mut career: ResMut<crate::hub_windows::ArenaCareer>,
    flow: Option<Res<Flow>>,
    mut launches: MessageWriter<ArenaMatchLaunch>,
) {
    if !auto.is_some_and(|auto| auto.0) {
        return;
    }
    if !hub.entered {
        rematch.due_ms = None;
        rematch.scheduled = false;
        return;
    }
    let now = time.map_or(0, |time| time.elapsed().as_millis() as u64);
    if !rematch.scheduled {
        rematch.scheduled = true;
        if rematch.board.is_some() {
            rematch.due_ms = Some(now + REMATCH_TICKS * TIMER_TICK_MS);
            info!(
                "arena auto: rematch in {} ms",
                REMATCH_TICKS * TIMER_TICK_MS
            );
        }
        return;
    }
    if rematch.due_ms.is_none_or(|due| now < due) {
        return;
    }
    rematch.due_ms = None;
    let Some(mut board) = rematch.board.clone() else {
        return;
    };
    let roster: Vec<crate::career::Profession> = career
        .company
        .roster
        .iter()
        .map(|member| member.profession)
        .collect();
    let per_side = usize::try_from(board.selected.team_size).unwrap_or(0);
    board.team = rematch_team(per_side, career.player.profession, &roster);
    let alternative = flow.is_some_and(|flow| flow.alternative_music);
    if let Some(launch) = plan_match(
        &mut career,
        &board,
        alternative,
        crate::arena_match::counter_now(),
    ) {
        info!(
            "arena auto: rematch of template {} with slots {:?}",
            launch.template, board.team
        );
        launches.write(launch);
        hub.busy = true;
    }
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
/// while a hub level is loaded (play mode has no loaded session yet). The
/// play view switches the flow to Exarust's practice stand-in once the
/// player spawns; with the hub loaded that is the hub's play mode too.
fn hub_mode(flow: Option<&Flow>, hub_loaded: bool) -> u32 {
    use crate::flow::Mode;
    match flow {
        Some(flow) if matches!(flow.mode, Mode::Menu | Mode::Practice) && hub_loaded => ARENA_MODE,
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

/// The shop's live data, bundled so the click hook keeps a system-param
/// count Bevy can satisfy: the game (to read the object database once), the
/// item catalogue (for the shelf's names), the live stock world and the
/// name table the trade window shows.
#[derive(bevy::ecs::system::SystemParam)]
struct HubShop<'w> {
    game: Option<Res<'w, exa_assets::Game>>,
    catalogue: Option<Res<'w, exa_items::catalogue::ItemCatalogue>>,
    live: ResMut<'w, crate::shop_stock::LiveShopStock>,
    names: ResMut<'w, crate::hub_windows::HubItemNames>,
}

impl HubShop<'_> {
    /// Build the first stock when the shelf is empty, as the hub click does,
    /// and open the trade window on it.
    fn open(
        &mut self,
        merchant: bool,
        career: &mut crate::hub_windows::ArenaCareer,
        windows: &mut HubWindows,
    ) {
        if let Some(game) = self.game.as_deref() {
            self.live.load_from_game(game);
        }
        // The original reads the performance counter twice: the draw's roll
        // and the variant seed.
        let rolls = || {
            [
                crate::arena_match::counter_now() as u32,
                crate::arena_match::counter_now() as u32,
            ]
        };
        // The shop's shelf asks the default kinds and any rank; the
        // merchant's asks its stock type and specialisation. Either is built
        // only when empty.
        let (stock, kinds, rank) = if merchant {
            (
                &mut career.merchant_stock,
                career.merchant_kinds,
                career.merchant_rank,
            )
        } else {
            (&mut career.company.stock, 0, -1)
        };
        if let Some(report) =
            crate::trade::ensure_first_stock_of(stock, kinds, rank, rolls(), &mut *self.live)
        {
            bevy::log::info!(
                "hub shop: first stock of {} items ({} invalid)",
                stock.len(),
                report.invalid
            );
        }
        if let Some(catalogue) = self.catalogue.as_deref() {
            self.live.fill_names(catalogue, &mut self.names);
        }
        windows.open_shop(merchant, career, &self.names);
    }
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
    bounds: Query<&PartBounds>,
    characters: Query<(&Condition, &exa_character::record::CharacterData)>,
    mut windows: ResMut<HubWindows>,
    mut career: ResMut<crate::hub_windows::ArenaCareer>,
    mut shop: HubShop,
    mut launches: MessageWriter<ArenaMatchLaunch>,
    (mut last, mut rematch): (ResMut<LastHubClick>, ResMut<ArenaRematch>),
) {
    let hub_loaded = levels.iter().any(|root| is_hub_level(&root.name));
    if !hub.entered && !hub_loaded {
        clicks.clear();
        return;
    }
    let mode = hub_mode(flow.as_deref(), hub_loaded);
    let took = std::mem::take(&mut windows.took_pointer);
    for click in clicks.read() {
        if took {
            continue;
        }
        // A window takes the click while it is open: the board's own hit
        // test, the trade button, or a press outside it (which closes the
        // modal form, as the window manager says).
        if windows.is_open() {
            // The table-built forms take their own pointer
            // ([`crate::hub_forms::hub_form_input`]).
            if windows.forms.contains(click.pointer.x, click.pointer.y) {
                continue;
            }
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
                                // The first board Begin that enters the
                                // manager shows the tip and waits for a
                                // second press; a tournament never warns.
                                let manager_entered = board.as_ref().is_some_and(|board| {
                                    board.tab != exa_ui::match_board::BoardTab::Tournament
                                        && board.team.contains(&0)
                                });
                                if crate::arena_match::warn_manager(
                                    &mut career.warn_manager,
                                    manager_entered,
                                ) {
                                    info!("arena begin: the manager is entered; warning shown");
                                    continue;
                                }
                                if let Some(board) = board
                                    && let Some(launch) =
                                        plan_launch(&mut career, &board, alternative)
                                {
                                    // Before a match the first unhealthy
                                    // specialist leaves the company.
                                    if let Some(departure) = career.company.leave_before_match() {
                                        info!("arena begin: specialist left ({departure:?})");
                                    }
                                    launches.write(launch);
                                    // The match the auto switch replays.
                                    rematch.board = Some(board);
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
        // A static object's mesh names the scene node the original's pick
        // returns; the name is on that node, not on the prop model's root.
        let object = |node: Entity,
                      names: &Query<&Name>,
                      transforms: &Query<&GlobalTransform>,
                      bounds: &Query<&PartBounds>| {
            let name = names.get(node).ok().map(|name| hub_name(name.as_str()));
            // The side is the x of the node's world bounding-box centre: the
            // collider centre the scene keeps, transformed by the node's
            // frame. A node without bounds reads its own origin.
            let side = match (transforms.get(node), bounds.get(node)) {
                (Ok(transform), Ok(bounds)) => transform.transform_point(bounds.centre).x,
                (Ok(transform), Err(_)) => transform.translation().x,
                _ => 0.0,
            };
            match name {
                Some(name) => hub_click(
                    hub.busy,
                    mode,
                    confirming(&windows),
                    HubTarget::Object { name: &name, side },
                ),
                None => HubAction::Nothing,
            }
        };
        let action = match click.hit {
            crate::hand::Hit::SceneNode { node } => object(node, &names, &transforms, &bounds),
            crate::hand::Hit::Prop { body, .. } => object(body, &names, &transforms, &bounds),
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
        let picked = match click.hit {
            crate::hand::Hit::SceneNode { node: e } | crate::hand::Hit::Prop { body: e, .. } => {
                names.get(e).ok()
            }
            _ => None,
        };
        debug!(
            "hub click on {:?} {:?} (busy {}, mode {:#x}): {:?}",
            click.hit, picked, hub.busy, mode, action
        );
        last.action = Some(action);
        match action {
            HubAction::Nothing => {}
            HubAction::Shop => {
                shop.open(false, &mut career, &mut windows);
            }
            HubAction::MerchantStock => {
                shop.open(true, &mut career, &mut windows);
            }
            HubAction::MatchBoard => {
                // Opening the board makes the hub busy; the board's close
                // clears it again.
                windows.open_board(&career);
                hub.busy = true;
            }
            HubAction::ManageCompany => {
                windows.open_manage(&career);
                hub.busy = true;
            }
        }
    }
}

/// A node's sixteen-byte scene name, as the click hook compares it. The
/// object library stores the hub's names with a leading 0xAA byte
/// ("\xaaArenaShop" in `shop_arena`, "\xaaArenaMach" in the notice boards);
/// the scene reader keeps each stored byte as one character, so the name is
/// turned back into those bytes. A character past one byte cannot be stored
/// and makes the name match nothing.
fn hub_name(name: &str) -> [u8; 16] {
    let mut out = [0u8; 16];
    for (slot, c) in out.iter_mut().zip(name.chars()) {
        *slot = u8::try_from(u32::from(c)).unwrap_or(0xFF);
    }
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
        && windows.close_top()
    {
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

    #[test]
    fn the_rematch_takes_the_combatants_after_the_manager() {
        use crate::career::Profession::{Combatant, Merchant, Physician};
        // Slots 1 and 3 are combatants; the merchant is skipped.
        assert_eq!(
            rematch_team(2, Combatant, &[Combatant, Merchant, Combatant]),
            vec![1, 3]
        );
        // A short staff fills what it can; the manager never stands.
        assert_eq!(rematch_team(3, Combatant, &[Physician, Combatant]), vec![2]);
        assert_eq!(REMATCH_TICKS * TIMER_TICK_MS, 4_000);
    }

    fn hub_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins).add_plugins(ArenaHubPlugin);
        app
    }

    fn spawn_hub_level(app: &mut App) {
        app.world_mut().spawn(LevelRoot {
            name: format!("{HUB_LEVEL}.rfc"),
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
        // The player spawns after the level's collision: it is placed when
        // it appears, not only on the entry frame.
        let player = app.world_mut().spawn((Player, Transform::default())).id();
        app.update();
        let at = app.world().get::<Transform>(player).unwrap().translation;
        assert_eq!(at, Vec3::new(0.0, 0.05, 3.0));
        assert!(app.world().resource::<ArenaHub>().manager_placed);
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
                .spawn((Name::new("\u{aa}ArenaShop"), GlobalTransform::default()))
                .id();
            app.world_mut()
                .write_message(WorldClick::picked(Hit::SceneNode { node: shop }));
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
                    Name::new("\u{aa}ArenaMach"),
                    // The side field is the bounding centre, not the node's
                    // own origin: an origin at zero on each board.
                    GlobalTransform::default(),
                    PartBounds {
                        centre: Vec3::new(side, 0.0, 0.0),
                        radius: 1.0,
                        half_extents: Vec3::splat(1.0),
                    },
                ))
                .id();
            app.world_mut()
                .write_message(WorldClick::picked(Hit::SceneNode { node: machine }));
            // The shop is still open (it is the confirming form), so close it.
            app.world_mut().resource_mut::<HubWindows>().close();
            app.update();
            assert_eq!(app.world().resource::<LastHubClick>().action, Some(want));
            let kind = if want == HubAction::MatchBoard {
                HubWindowKind::Board
            } else {
                HubWindowKind::Manage
            };
            assert!(app.world().resource::<HubWindows>().is_kind(kind));
            // The board and Manage Company both make the hub busy.
            assert!(app.world().resource::<ArenaHub>().busy);
            // A press outside the modal form closes it and clears the flag.
            app.world_mut()
                .write_message(WorldClick::picked(Hit::SceneNode { node: machine }));
            app.update();
            assert!(
                app.world()
                    .resource::<HubWindows>()
                    .is_kind(HubWindowKind::None)
            );
            assert!(!app.world().resource::<ArenaHub>().busy);
        }
    }

    /// A shop click on an empty shelf builds the first stock from the live
    /// stock world (18 to 20 copies of its one listed apparel record) and a
    /// second click leaves the shelf as it is. The merchant's shelf asks its
    /// own kinds: a weapons merchant finds nothing among apparel.
    #[test]
    fn a_shop_click_builds_the_first_stock_once() {
        use exa_formats::trade_record::{TradeModel, TradeRecord};
        let mut app = hub_app();
        let mut flow = Flow::default();
        flow.mode = crate::flow::Mode::Arena;
        flow.phase = Phase::Playing;
        app.insert_resource(flow);
        app.insert_resource(crate::shop_stock::LiveShopStock::from_entries(vec![
            crate::trade::ShopEntry {
                id: 10,
                flags: crate::trade::APPAREL,
                record: Some(TradeRecord {
                    listing: Some(0),
                    parent: 0,
                    model: Some(TradeModel {
                        named: true,
                        flags: 0,
                        rarity: 0,
                        size: [1, 1],
                    }),
                    rank: 0,
                }),
            },
        ]));
        spawn_hub_level(&mut app);
        app.update();
        let shop = app
            .world_mut()
            .spawn((Name::new("\u{aa}ArenaShop"), GlobalTransform::default()))
            .id();
        app.world_mut()
            .write_message(WorldClick::picked(Hit::SceneNode { node: shop }));
        app.update();
        let first = app
            .world()
            .resource::<crate::hub_windows::ArenaCareer>()
            .company
            .stock
            .clone();
        assert!((18..=20).contains(&first.len()), "{}", first.len());
        let live = app.world().resource::<crate::shop_stock::LiveShopStock>();
        assert!(
            first
                .iter()
                .all(|&id| live.parent_of_instance(id) == Some(10))
        );
        app.world_mut().resource_mut::<HubWindows>().close();
        app.world_mut()
            .write_message(WorldClick::picked(Hit::SceneNode { node: shop }));
        app.update();
        assert_eq!(
            app.world()
                .resource::<crate::hub_windows::ArenaCareer>()
                .company
                .stock,
            first
        );

        // A weapons merchant's first stock finds no candidate among the
        // apparel and stays empty.
        let mut live = app
            .world()
            .resource::<crate::shop_stock::LiveShopStock>()
            .clone();
        let mut merchant = Vec::new();
        let report = crate::trade::ensure_first_stock_of(
            &mut merchant,
            crate::trade::ARMS,
            -1,
            [1, 2],
            &mut live,
        )
        .unwrap();
        assert_eq!(report.invalid, 0);
        assert!(merchant.is_empty());
    }

    /// The ArenaMach side is the bounding centre's x, transformed by the
    /// node, not the node's own origin.
    #[test]
    fn the_machine_side_is_the_bounding_centre() {
        let mut app = hub_app();
        let mut flow = Flow::default();
        flow.mode = crate::flow::Mode::Arena;
        flow.phase = Phase::Playing;
        app.insert_resource(flow);
        spawn_hub_level(&mut app);
        app.update();
        for (origin, centre, want) in [
            (5.0, -10.0, HubAction::ManageCompany),
            (5.0, 10.0, HubAction::MatchBoard),
        ] {
            let machine = app
                .world_mut()
                .spawn((
                    Name::new("\u{aa}ArenaMach"),
                    GlobalTransform::from_xyz(origin, 0.0, 0.0),
                    PartBounds {
                        centre: Vec3::new(centre, 0.0, 0.0),
                        radius: 1.0,
                        half_extents: Vec3::splat(1.0),
                    },
                ))
                .id();
            app.world_mut().resource_mut::<HubWindows>().close();
            app.world_mut().resource_mut::<ArenaHub>().busy = false;
            app.world_mut()
                .write_message(WorldClick::picked(Hit::SceneNode { node: machine }));
            app.update();
            assert_eq!(
                app.world().resource::<LastHubClick>().action,
                Some(want),
                "{origin} {centre}"
            );
        }
    }

    /// The play view's practice stand-in and the menu count as the arena
    /// while the hub is loaded; story mode and a practice without the hub
    /// do not.
    #[test]
    fn the_play_view_hub_counts_as_the_arena() {
        use crate::flow::Mode;
        let mut flow = Flow::default();
        for (mode, loaded, word) in [
            (Mode::Practice, true, ARENA_MODE),
            (Mode::Menu, true, ARENA_MODE),
            (Mode::Practice, false, crate::flow::PRACTICE_MODE),
            (Mode::Story, true, crate::flow::STORY_MODE),
        ] {
            flow.mode = mode;
            assert_eq!(hub_mode(Some(&flow), loaded), word, "{mode:?} {loaded}");
        }
        assert_eq!(hub_mode(None, true), ARENA_MODE);
        assert_eq!(hub_mode(None, false), 0);
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
            .spawn((Name::new("\u{aa}ArenaShop"), GlobalTransform::default()))
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
        app.world_mut()
            .write_message(WorldClick::picked(Hit::SceneNode { node: shop }));
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
            .spawn((Name::new("\u{aa}ArenaShop"), GlobalTransform::default()))
            .id();
        app.world_mut().write_message(WorldClick {
            hit: Hit::SceneNode { node: shop },
            pointer: IVec2::new(
                (bounds.left + bounds.right) / 2,
                (bounds.top + bounds.bottom) / 2,
            ),
            double: false,
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
    fn the_hub_level_name_takes_the_loaders_suffix() {
        assert!(is_hub_level("arenahub"));
        assert!(is_hub_level("ArenaHub.rfc"));
        assert!(!is_hub_level("arena1"));
        assert!(!is_hub_level("arenahub2"));
    }

    #[test]
    fn the_scene_name_is_the_stored_bytes() {
        // The scene reader keeps byte 0xAA as the character U+00AA.
        assert!(same_name(
            &hub_name("\u{aa}ArenaShop"),
            &crate::arena_hub::SHOP_NAME
        ));
        assert!(same_name(
            &hub_name("\u{aa}ARENAMACH"),
            &crate::arena_hub::MACHINE_NAME
        ));
        // Without the leading byte the stored name is a different one.
        assert!(!same_name(
            &hub_name("ArenaShop"),
            &crate::arena_hub::SHOP_NAME
        ));
        assert!(!same_name(
            &hub_name("\u{aa}ArenaMach"),
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
        // The match start took the Duel's 3 copper entry fee.
        assert_eq!(career.company.copper, 340);
        assert_eq!(career.company.spent, 3);
    }

    #[test]
    fn the_player_body_is_the_manager_not_the_first_team_member() {
        let mut career = crate::hub_windows::ArenaCareer::default();
        career.company.roster.push(crate::career::Member {
            name: "Ash".into(),
            profession: crate::career::Profession::Combatant,
            xp: 0,
            skills: [6, 0, 0, 0, 0, 0],
            skill_xp: [0; 6],
            techniques: [[0; 6]; 6],
            loadouts: Default::default(),
            health: 65_000,
            in_entry: false,
        });
        let board = career.board();
        // A duel takes the combatant (pool slot 1), not the manager.
        assert_eq!(board.team, vec![1]);
        let launch = plan_launch_at(&mut career, &board, false, 0x1234_5678).unwrap();
        assert_eq!(launch.player, None);
        assert!(
            launch
                .fighters
                .iter()
                .all(|fighter| { !matches!(fighter.body, LaunchedBody::Player) })
        );
        assert!(matches!(
            launch.fighters.last().unwrap().body,
            LaunchedBody::Company { .. }
        ));
    }

    #[test]
    fn a_short_purse_pays_down_to_nothing_and_still_starts() {
        let mut career = crate::hub_windows::ArenaCareer::default();
        career.company.copper = 2;
        let board = career.board();
        assert!(plan_launch_at(&mut career, &board, false, 0x1234_5678).is_some());
        assert_eq!(career.company.copper, 0);
        assert_eq!(career.company.spent, 3);
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
        // The week's field was drawn for the tournament and travels with
        // the launch; each rival side wears its team's company at the
        // event's tier, armed from the row.
        assert_ne!(launch.rivals[0], 0);
        assert_eq!(launch.rivals, career.rivals);
        assert_ne!(launch.weapon_variants, [0; 3]);
        for fighter in hires {
            let LaunchedBody::Opponent(crate::match_layout::OpponentPlan::Hire(plan)) =
                fighter.body
            else {
                panic!("a tournament side takes a hire plan");
            };
            let company = crate::arena_spawn::rival_company(&launch.rivals, fighter.team as i32);
            assert_eq!(
                plan,
                crate::arena_spawn::hire_plan(plan.outfit.level(), company, 1)
            );
            assert!(plan.draw_weapon);
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

    fn spawn_player(app: &mut App) -> Entity {
        app.world_mut().spawn((Player, Transform::default())).id()
    }

    fn member(profession: crate::career::Profession) -> crate::career::Member {
        crate::career::Member {
            name: "Ash".into(),
            profession,
            xp: 0,
            skills: [0; 6],
            skill_xp: [0; 6],
            techniques: [[0; 6]; 6],
            loadouts: Default::default(),
            health: u16::MAX,
            in_entry: false,
        }
    }

    /// The play view runs the hub under its practice stand-in mode: the
    /// manager is still placed and the hub's bodies asked for; a story-mode
    /// flow leaves the hub alone.
    #[test]
    fn the_play_views_practice_mode_still_enters_the_hub() {
        for (mode, entered) in [
            (crate::flow::Mode::Practice, true),
            (crate::flow::Mode::Story, false),
        ] {
            let mut app = hub_app();
            let mut flow = Flow::default();
            flow.mode = mode;
            flow.phase = Phase::Playing;
            app.insert_resource(flow);
            spawn_hub_level(&mut app);
            spawn_player(&mut app);
            app.update();
            let hub = app.world().resource::<ArenaHub>();
            assert_eq!(hub.manager_placed, entered, "{mode:?}");
            assert_eq!(hub.npcs_spawned, entered, "{mode:?}");
        }
    }

    #[test]
    fn the_hub_asks_for_its_shopkeeper_and_one_body_per_specialist() {
        let mut app = hub_app();
        let mut flow = Flow::default();
        flow.mode = crate::flow::Mode::Arena;
        flow.phase = Phase::Playing;
        app.insert_resource(flow);
        {
            let mut career = app
                .world_mut()
                .resource_mut::<crate::hub_windows::ArenaCareer>();
            career
                .company
                .roster
                .push(member(crate::career::Profession::Merchant));
            career
                .company
                .roster
                .push(member(crate::career::Profession::Combatant));
            career
                .company
                .roster
                .push(member(crate::career::Profession::Physician));
        }
        spawn_hub_level(&mut app);
        spawn_player(&mut app);
        app.update();
        assert!(app.world().resource::<ArenaHub>().npcs_spawned);
        let commands: Vec<crate::scene_bodies::SceneCommand> = app
            .world_mut()
            .resource_mut::<Messages<crate::scene_bodies::SceneCommand>>()
            .drain()
            .collect();
        assert_eq!(commands.len(), 3, "shopkeeper, merchant and physician");
        let plans: Vec<&crate::scene_bodies::SceneBodyPlan> = commands
            .iter()
            .filter_map(|command| match command {
                crate::scene_bodies::SceneCommand::Spawn(plan) => Some(plan.as_ref()),
                _ => None,
            })
            .collect();
        assert_eq!(plans[0].at, Vec3::new(-3.2, 0.0, -1.1));
        assert_eq!(
            plans[0].yaw.to_bits(),
            (15.0f32 * crate::arena_spawn::DEGREE).to_bits()
        );
        assert_eq!(plans[0].roles, vec![0x11, 0x21]);
        assert_eq!(plans[0].actor(), Some("arenashpkp"));
        assert_eq!(plans[1].at, Vec3::new(4.4, 0.0, 0.1));
        assert_eq!(
            plans[1].actor(),
            Some(crate::scene_bodies::FIGHTER_TEMPLATES[0])
        );
        assert_eq!(plans[2].at, Vec3::new(-27.0, 0.0, 12.5));
        assert_eq!(
            plans[2].yaw.to_bits(),
            (140.0f32 * crate::arena_spawn::DEGREE).to_bits()
        );
        // The bodies are asked for once.
        app.update();
        assert!(
            app.world_mut()
                .resource_mut::<Messages<crate::scene_bodies::SceneCommand>>()
                .drain()
                .next()
                .is_none()
        );
    }

    #[test]
    fn a_settled_match_puts_the_manager_back_at_the_return_placement() {
        let mut app = hub_app();
        let mut flow = Flow::default();
        flow.mode = crate::flow::Mode::Arena;
        flow.phase = Phase::Playing;
        app.insert_resource(flow);
        app.world_mut().resource_mut::<ArenaHub>().returned =
            Some(crate::arena_session::ReturnPlacement {
                distance: 250.0,
                heading: 0.5,
                flags: 0x100,
                saves: true,
            });
        spawn_hub_level(&mut app);
        let player = spawn_player(&mut app);
        app.update();
        let transform = app.world().get::<Transform>(player).unwrap();
        // The saved distance is along z, in metres, over the floor lift;
        // the facing stays the body's own.
        assert_eq!(transform.translation, Vec3::new(0.0, 0.05, 2.5));
        assert_eq!(transform.rotation, Quat::IDENTITY);
    }
}
