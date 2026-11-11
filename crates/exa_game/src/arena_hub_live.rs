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
    /// The specialists whose bodies stand in this hub visit.
    pub specialists_placed: Vec<crate::career::Profession>,
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
            specialists_placed: Vec::new(),
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
            .add_message::<crate::arsenal_view::OpenArsenal>()
            .add_message::<OpenHubShop>()
            .add_message::<bevy::input::keyboard::KeyboardInput>()
            .add_message::<crate::item_card::WorldCard>()
            .init_resource::<ShopBuyScript>()
            .add_message::<crate::scene_bodies::SceneCommand>()
            .init_resource::<HubWindows>()
            .init_resource::<crate::hub_windows::ArenaCareer>()
            .init_resource::<crate::hub_windows::HubItemNames>()
            .init_resource::<crate::shop_stock::LiveShopStock>()
            .init_resource::<crate::shop_stock::ItemPalette>()
            .init_resource::<crate::shop_stock::ShopFactories>()
            .init_resource::<exa_assets::live_objects::LiveObjects>()
            .init_resource::<LastHubClick>()
            .init_resource::<crate::hub_forms::OpenHubWindow>()
            .init_resource::<crate::arsenal::ShelfKinds>()
            .init_resource::<crate::loadout_kit::KitPanels>()
            .init_resource::<crate::loadout_kit::KitPreview>()
            .init_resource::<crate::doll_view::DollItems>()
            .init_resource::<ArenaRematch>()
            .init_resource::<crate::arena_live::ArenaAutoContinue>()
            .init_resource::<exa_ui::gui_draw::GuiFrames>()
            .init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<ButtonInput<MouseButton>>()
            .add_systems(
                Update,
                (
                    crate::hub_windows::sync_hub_screen,
                    crate::arsenal::load_shelf_kinds,
                    enter_loaded_hub,
                    auto_rematch,
                    crate::hub_forms::open_hub_window_on_load,
                    crate::hub_forms::hub_employee_kit_on_load,
                    crate::hub_forms::hub_arsenal_on_load,
                    crate::hub_forms::hub_stash_on_load,
                    (
                        crate::hub_forms::hub_form_input,
                        crate::hub_forms::manage_table_script,
                    )
                        .chain(),
                    crate::loadout_kit::kit_input,
                    crate::loadout_kit::sync_kit,
                    hub_clicks,
                    scripted_shop,
                    hub_restock,
                    hub_session_fall,
                    shop_drag,
                    sync_shop_preview,
                    shop_item_cards,
                    close_hub_windows,
                    release_shop_icons,
                )
                    .chain()
                    .in_set(HubWindowSystems),
            )
            .add_systems(
                Update,
                hub_enter_on_load
                    .after(crate::hub_forms::hub_employee_kit_on_load)
                    .before(hub_clicks),
            )
            .add_systems(
                Update,
                (
                    crate::option_forms::hub_form_typing,
                    crate::option_forms::load_emblem_textures,
                )
                    .chain()
                    .after(crate::hub_forms::hub_form_input)
                    .in_set(HubWindowSystems),
            )
            .add_systems(Update, read_hire_facts.before(enter_loaded_hub))
            .add_systems(
                Update,
                place_hired_specialists
                    .after(enter_loaded_hub)
                    .after(crate::hub_forms::hub_form_input),
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
        hub.specialists_placed.clear();
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
                hub.specialists_placed.push(member.profession);
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

/// A specialist hired during the hub visit: the original's hire places a
/// merchant's or physician's body at its spot at once, as the hub's entry
/// places those already employed.
fn place_hired_specialists(
    mut hub: ResMut<ArenaHub>,
    career: Res<crate::hub_windows::ArenaCareer>,
    mut scene_commands: MessageWriter<crate::scene_bodies::SceneCommand>,
) {
    if !hub.npcs_spawned {
        return;
    }
    for member in &career.company.roster {
        if hub.specialists_placed.contains(&member.profession) {
            continue;
        }
        let Some(spot) = crate::arena_spawn::specialist_spot(member.profession) else {
            continue;
        };
        let mut plan = crate::scene_bodies::hub_body_plan(&spot, false);
        plan.name = Some(member.name.clone());
        scene_commands.write(crate::scene_bodies::SceneCommand::Spawn(Box::new(plan)));
        hub.specialists_placed.push(member.profession);
        info!(
            "arena hub: the hired {:?} {} stands at its spot",
            member.profession, member.name
        );
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
    /// The company entry that is the player's own, if any. None when no
    /// entrant is the player's to control (only hired employees fight):
    /// the match is spectated (`crate::spectate`).
    pub player: Option<usize>,
    /// The board's entrants in entry order, as company pool slots (0 is the
    /// manager, roster member `i` is slot `i + 1`): the participant list.
    pub entrants: Vec<usize>,
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
    /// A weapon event's hands for the company copies: the two weapon words
    /// of the started event's first entrant.
    pub event_hands: Option<[u64; 2]>,
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
    // The match's start counts a Beast match entered.
    career.company.record_match_started(template);
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
    // A tournament event's start gives each earlier event that has no
    // winner and no entrant a rival winner (2 to 7), stopping at the first
    // entered event.
    if tournament {
        crate::tournament::settle_unentered(&mut career.entries, &mut career.roll);
    }
    // A weapon event's company copies hold the started event's (the first
    // undecided one with entrants) first entrant's two weapon words.
    let weapon_event = board.selected.flags & exa_ui::match_board::FLAG_WEAPON != 0;
    let event_hands = if tournament && weapon_event {
        career
            .entries
            .iter()
            .find(|row| row.team_id == 0 && 0 < row.count as i32)
            .map(|row| row.entries[0].weapon)
    } else {
        None
    };
    Some(ArenaMatchLaunch {
        template,
        arena: start.arena,
        track: start.track,
        spawn_at,
        level,
        tier,
        player,
        entrants: board.team.clone(),
        tournament,
        prize: board.selected.prize,
        weapon_variants,
        rivals,
        event_hands,
        fighters,
    })
}

/// The hire list's object-database facts ([`crate::hub_windows::HireFacts`]):
/// the outfit tables, each body item's apparel rank byte (the body
/// section's rank word's low byte) and each item's value word (the model
/// section's, as the catalogue expands and the shop's price compacts it).
#[must_use]
pub fn hire_facts_of(game: &exa_assets::Game) -> Option<crate::hub_windows::HireFacts> {
    let bytes = game
        .0
        .pack(exa_assets::PackKind::Resource)?
        .get("objdb.rdb")?;
    let database = exa_formats::rdb::Database::parse(bytes).ok()?;
    let items = exa_formats::objdb::items(&database);
    let catalogue = exa_items::catalogue::ItemCatalogue::of_database(&database);
    let facts: std::collections::BTreeMap<u32, crate::arena_outfit::ItemFacts> = items
        .iter()
        .map(|item| (item.id, crate::arena_outfit::ItemFacts::of_item(item)))
        .collect();
    let tables = crate::arena_outfit::OutfitTables::build(|id| facts.get(&id.0).cloned());
    let apparel_rank = items
        .iter()
        .filter(|item| item.has_body)
        .map(|item| (item.id, item.body_rank as u8))
        .collect();
    let values = items
        .iter()
        .filter_map(|item| {
            let card = catalogue.card_facts(item.id)?;
            Some((item.id, exa_formats::containers::price_of(card.coins)))
        })
        .collect();
    Some(crate::hub_windows::HireFacts {
        tables,
        apparel_rank,
        values,
    })
}

/// Reads the hire list's facts once the hub's level is in, so the
/// candidates are dressed and priced with their outfits.
fn read_hire_facts(
    game: Option<Res<exa_assets::Game>>,
    levels: Query<&LevelRoot>,
    mut career: ResMut<crate::hub_windows::ArenaCareer>,
    mut tried: Local<bool>,
) {
    if *tried || career.hire_facts.is_some() || !levels.iter().any(|root| is_hub_level(&root.name))
    {
        return;
    }
    let Some(game) = game else {
        return;
    };
    *tried = true;
    match hire_facts_of(&game) {
        Some(facts) => {
            career.set_hire_facts(
                std::sync::Arc::new(facts),
                crate::arena_match::counter_now(),
            );
            let prices: Vec<u32> = (0..career.hire_pool.len())
                .map(|index| career.hire_price(index))
                .collect();
            info!("hire list: dressed from the object database; prices {prices:?}");
        }
        None => warn!("hire list: the object database does not read; candidates stay undressed"),
    }
}

/// `--hub-enter slot,...`: once the manager stands in the hub (and a
/// `--hub-employee-kit` employee has joined), the board's selected match
/// begins with these company pool slots as its lineup (0 is the manager,
/// roster member `i` is slot `i + 1`), through the board's own launch
/// ([`plan_launch`]) without the clicks. A debug tool for entering
/// employees without the manager (the spectated match).
#[derive(Resource, Clone, Debug, Default, PartialEq, Eq)]
pub struct HubEnter {
    pub slots: Vec<usize>,
    pub done: bool,
}

/// Runs `--hub-enter` once, when the lineup's members are on the roster.
fn hub_enter_on_load(
    ask: Option<ResMut<HubEnter>>,
    mut hub: ResMut<ArenaHub>,
    mut career: ResMut<crate::hub_windows::ArenaCareer>,
    flow: Option<Res<Flow>>,
    mut launches: MessageWriter<ArenaMatchLaunch>,
    mut rematch: ResMut<ArenaRematch>,
) {
    let Some(mut ask) = ask else {
        return;
    };
    let roster = career.company.roster.len();
    if ask.done || !hub.manager_placed || ask.slots.iter().any(|&slot| slot > roster) {
        return;
    }
    ask.done = true;
    let mut board = career.board();
    board.team = ask.slots.clone();
    let alternative = flow.is_some_and(|flow| flow.alternative_music);
    match plan_launch(&mut career, &board, alternative) {
        Some(launch) => {
            info!(
                "--hub-enter: template {} begins with slots {:?} (the player's entry {:?})",
                launch.template, launch.entrants, launch.player
            );
            launches.write(launch);
            rematch.board = Some(board);
            hub.busy = true;
        }
        None => warn!("--hub-enter: the board refuses the match"),
    }
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
struct HubShop<'w, 's> {
    game: Option<Res<'w, exa_assets::Game>>,
    catalogue: Option<ResMut<'w, exa_items::catalogue::ItemCatalogue>>,
    live: ResMut<'w, crate::shop_stock::LiveShopStock>,
    objects: ResMut<'w, exa_assets::live_objects::LiveObjects>,
    palette: ResMut<'w, crate::shop_stock::ItemPalette>,
    factories: ResMut<'w, crate::shop_stock::ShopFactories>,
    names: ResMut<'w, crate::hub_windows::HubItemNames>,
    icons: Option<ResMut<'w, crate::item_panel_draw::PanelIcons>>,
    players: Query<
        'w,
        's,
        &'static mut exa_items::possession::Possessions,
        With<crate::player::PlayerBody>,
    >,
    /// The company's arsenal and holders a confirmed trade reaches.
    stores: Option<ResMut<'w, crate::save_live::ArenaStores>>,
    kinds: Option<Res<'w, crate::arsenal::ShelfKinds>>,
}

/// The shop line for an item's value: a copy's own model value word in the
/// live database (its condition already scaled it), else the catalogue's,
/// else its source record's. No value is a zero line, and the row stays on
/// the shelf.
fn shop_line_price(
    catalogue: &exa_items::catalogue::ItemCatalogue,
    live: &crate::shop_stock::LiveShopStock,
    objects: &exa_assets::live_objects::LiveObjects,
    id: u32,
) -> u16 {
    if id & exa_formats::objdb::TAG_MASK != 0
        && let Some(word) = objects
            .db()
            .section(id, 0x0010_0000)
            .and_then(|model| model.get(0x1E..0x20))
    {
        return u16::from_le_bytes([word[0], word[1]]);
    }
    let facts = catalogue.card_facts(id).or_else(|| {
        live.parent_of_instance(id)
            .and_then(|base| catalogue.card_facts(base))
    });
    facts
        .map(|facts| exa_formats::containers::price_of(facts.coins))
        .unwrap_or(0)
}

impl HubShop<'_, '_> {
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
        let report = if stock.is_empty() {
            let rolls = rolls();
            self.with_world(|world| {
                crate::trade::ensure_first_stock_of(stock, kinds, rank, rolls, world)
            })
        } else {
            None
        };
        if let Some(report) = report {
            bevy::log::info!(
                "hub shop: first stock of {} items ({} invalid)",
                stock.len(),
                report.invalid
            );
            self.log_copies(stock);
        }
        // The copies the stock update made are in the live database now;
        // the catalogue takes their tiers at once, so the panels' icons can
        // render them (the per-frame registration would come a frame late).
        if let Some(catalogue) = self.catalogue.as_deref_mut()
            && !catalogue.is_empty()
        {
            let mut tiers = Vec::new();
            for &id in stock.iter() {
                let tier = exa_formats::objdb::Tier::of(id);
                if id & exa_formats::objdb::TAG_MASK != 0 && !tiers.contains(&tier) {
                    tiers.push(tier);
                }
            }
            for tier in tiers {
                crate::gui_screens::register_tier(&self.objects, catalogue, tier);
            }
        }
        if let Some(catalogue) = self.catalogue.as_deref() {
            self.live.fill_names(catalogue, &mut self.names);
        }
        if career.goods.is_empty()
            && let Some(possessions) = self.players.iter().next()
        {
            career.seed_goods(possessions.carried.iter().map(|carried| carried.id));
        }
        let carried = self
            .players
            .iter()
            .next()
            .map(|possessions| possessions.carried.clone())
            .unwrap_or_default();
        let pack = crate::hub_windows::pack_list(&career.goods, &carried);
        let catalogue = self.catalogue.as_deref();
        let mut icons = self.icons.as_deref_mut();
        let unit = if windows.unit > 0 { windows.unit } else { 4 };
        let icon = exa_items::icon_cache::icon_size(unit);
        windows.open_shop(merchant, career, &pack, |id, flags| {
            match icons.as_deref_mut() {
                Some(icons) => icons.reserve(id, flags, catalogue),
                None => (0, (icon / 2, icon / 2)),
            }
        });
        if let Some(catalogue) = catalogue {
            let live = &self.live;
            let objects = &self.objects;
            windows.price_shop_items(|id| shop_line_price(catalogue, live, objects, id));
        }
        if let Some(shop) = windows.shop.as_ref() {
            for panel in crate::shop_panels::SHOP_PANELS {
                let centres: Vec<(u32, (i32, i32))> = (0..shop.panels.get(panel).entries.len())
                    .filter_map(|i| {
                        let id = shop.panels.get(panel).entries[i].id;
                        let local = windows.shop_entry_centre(panel, i)?;
                        let bounds = windows.bounds()?;
                        Some((id, (bounds.left + local.0, bounds.top + local.1)))
                    })
                    .collect();
                bevy::log::info!("hub shop: {panel:?} entry centres (id, screen) {centres:?}");
            }
        }
    }

    /// Runs a stock update over the live object database, where the copies
    /// are made (its base tier read first).
    fn with_world<R>(
        &mut self,
        run: impl FnOnce(&mut crate::shop_stock::LiveShopWorld<'_>) -> R,
    ) -> R {
        if let Some(game) = self.game.as_deref() {
            self.objects.ensure_base(&game.0);
        }
        let game = self.game.as_deref();
        let factories = &mut self.factories;
        let mut factory = |name: &str, body: &mut [u8]| factories.pass(game, name, body);
        let mut entropy = || crate::arena_match::counter_now() as u32;
        let mut world = crate::shop_stock::LiveShopWorld {
            stock: &mut self.live,
            db: self.objects.db_mut(),
            palette: &mut self.palette.0,
            entropy: &mut entropy,
            factory: &mut factory,
        };
        run(&mut world)
    }

    /// A confirmed trade reaches the company, as the original's two arena
    /// hooks on the trade window do: the pack loses the sold items (the
    /// player's carried list is rewritten), each sold item leaves the
    /// arsenal and its holders, and each bought item is made the world's
    /// own copy and stowed on its shelf ([`crate::arsenal::Arsenal::trade`]).
    fn arena_trade(
        &mut self,
        trade: &crate::hub_windows::ShopTrade,
        career: &mut crate::hub_windows::ArenaCareer,
    ) -> Vec<(u32, Option<usize>)> {
        if career.goods_changed
            && let Some(mut possessions) = self.players.iter_mut().next()
        {
            possessions.carried = crate::hub_windows::ArenaCareer::carried_after_trade(
                &possessions.carried,
                &career.goods,
            );
            career.goods_changed = false;
        }
        let Some(stores) = self.stores.as_deref_mut() else {
            return Vec::new();
        };
        let kinds = self.kinds.as_deref();
        let objects = &mut *self.objects;
        // A level's or the world's own record is shelved by the template it
        // was made from (the shelf facts are the templates').
        let template = |db: &exa_formats::objdb::ObjectDb, item: u32| {
            if item & exa_formats::objdb::TAG_MASK != 0 {
                db.template_of(item)
            } else {
                item
            }
        };
        let kinds_of: Vec<(u32, crate::arsenal::ItemKind)> = trade
            .sold
            .iter()
            .map(|&item| {
                let kind =
                    kinds.map_or_else(Default::default, |k| k.kind(template(objects.db(), item)));
                (item, kind)
            })
            .collect();
        let promoted: Vec<(u32, crate::arsenal::ItemKind)> = trade
            .bought
            .iter()
            .map(|&item| {
                let copy = objects.db_mut().promote(item);
                let kind =
                    kinds.map_or_else(Default::default, |k| k.kind(template(objects.db(), copy)));
                (copy, kind)
            })
            .collect();
        let kind = |item: u32| {
            kinds_of
                .iter()
                .chain(&promoted)
                .find(|(id, _)| *id == item)
                .map_or_else(Default::default, |(_, kind)| *kind)
        };
        let crate::save_live::ArenaStores {
            staff,
            arsenal,
            stash,
        } = stores;
        let mut holders = vec![crate::arsenal::Holder {
            items: stash,
            loadouts: &mut career.player.loadouts,
            own: None,
        }];
        for (employee, member) in staff
            .employees
            .iter_mut()
            .zip(career.company.roster.iter_mut())
        {
            holders.push(crate::arsenal::Holder {
                items: &mut employee.kit,
                loadouts: &mut member.loadouts,
                own: Some(crate::arsenal::OwnItems {
                    carried: &mut employee.carried,
                    hands: &mut employee.hands,
                    worn: &mut employee.worn,
                }),
            });
        }
        let promoted_ids: Vec<u32> = promoted.iter().map(|(id, _)| *id).collect();
        let went = arsenal.trade(&trade.sold, &promoted_ids, kind, |id| id, &mut holders);
        drop(holders);
        let shelf_counts: Vec<usize> = arsenal
            .shelves
            .iter()
            .map(|shelf| shelf.iter().map(Vec::len).sum())
            .collect();
        let kits: Vec<(&str, &Vec<u32>, Vec<usize>)> = staff
            .employees
            .iter()
            .zip(&career.company.roster)
            .map(|(employee, member)| {
                let worn = member.loadouts.iter().map(|l| l.worn.len()).collect();
                (member.name.as_str(), &employee.kit, worn)
            })
            .collect();
        bevy::log::info!(
            "arena trade: sold {:x?} left the arsenal and its holders; bought (copy, shelf) {went:x?} stowed; shelves hold {shelf_counts:?}; stash {:x?}; staff (name, kit, worn per loadout) {kits:x?}",
            trade.sold,
            stash
        );
        went
    }

    /// Logs each copy the shelf holds with its own condition: an apparel
    /// body's condition, damage, dirt and variant bytes, a weapon's quality,
    /// damage and dirt, and the value word.
    fn log_copies(&self, stock: &[u32]) {
        let db = self.objects.db();
        for &id in stock {
            if id & exa_formats::objdb::TAG_MASK == 0 {
                continue;
            }
            let name = db.name(id).unwrap_or_default();
            let value = db
                .section(id, 0x0010_0000)
                .and_then(|m| m.get(0x1E..0x20))
                .map_or(0, |w| u16::from_le_bytes([w[0], w[1]]));
            if let Some(body) = db.section(id, 0x1000).filter(|b| b.len() > 0x1B2) {
                bevy::log::info!(
                    "hub shop: copy {id:#x} {name:?} apparel condition {:#04x} variant {:#04x} quality {} damage {} dirt {} value {value}",
                    body[0x1A9],
                    body[0x1AA],
                    body[0x1B0],
                    body[0x1B1],
                    body[0x1B2],
                );
            } else if let Some(weapon) = db.section(id, 0x4000).filter(|b| b.len() > 0x0E) {
                bevy::log::info!(
                    "hub shop: copy {id:#x} {name:?} weapon quality {} damage {} dirt {} seeds {:#x} {:#x} value {value}",
                    weapon[0x0C],
                    weapon[0x0D],
                    weapon[0x0E],
                    u32::from_le_bytes(weapon[4..8].try_into().expect("4 bytes")),
                    u32::from_le_bytes(weapon[8..12].try_into().expect("4 bytes")),
                );
            }
        }
    }

    /// The night's restock: the shop's shelf and, with a merchant employed,
    /// the merchant's own, using the live stock world.
    fn restock(&mut self, career: &mut crate::hub_windows::ArenaCareer) {
        if let Some(game) = self.game.as_deref() {
            self.live.load_from_game(game);
        }
        let rolls = [
            crate::arena_match::counter_now() as u32,
            crate::arena_match::counter_now() as u32,
        ];
        self.with_world(|world| career.company.restock(0, -1, rolls, world));
        if career
            .company
            .roster
            .iter()
            .any(|member| member.profession == crate::career::Profession::Merchant)
        {
            let (stock, kinds, rank) = (
                &mut career.merchant_stock,
                career.merchant_kinds,
                career.merchant_rank,
            );
            self.with_world(|world| crate::trade::update_stock(stock, kinds, rank, rolls, world));
        }
        if let Some(catalogue) = self.catalogue.as_deref() {
            self.live.fill_names(catalogue, &mut self.names);
        }
    }
}

/// Opens the shop as the hub's click on it does (`--open-hub-window shop`),
/// or the merchant's trade with its options (`--open-hub-window merchant`).
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub struct OpenHubShop {
    pub merchant: bool,
}

/// `--shop-buy ROW` (debug): once the scripted shop opens, its stock row
/// `ROW` is clicked into the buy basket and the trade confirmed through the
/// window's own clicks; the window then closes and the bought copy's card
/// opens at the pointer. `--shop-sell ID` (debug) puts item `ID` in the
/// manager's pack before the open, and the same step drags it into the
/// sell basket before the trade.
#[derive(Resource, Debug, Clone, Default, PartialEq, Eq)]
pub struct ShopBuyScript {
    pub row: Option<usize>,
    pub sell: Option<u32>,
    stage: u8,
    wait: u32,
    bought: u32,
}

impl ShopBuyScript {
    /// The purchase of stock row `row` (if any) and the sale of item `sell`
    /// (if any), in one trade.
    pub fn of_trade(row: Option<usize>, sell: Option<u32>) -> Self {
        Self {
            row,
            sell,
            ..Self::default()
        }
    }
}

/// Frames between the scripted shop's steps.
const SHOP_SCRIPT_WAIT: u32 = 40;

/// The scripted shop: the open, and the scripted purchase.
fn scripted_shop(
    mut opens: MessageReader<OpenHubShop>,
    career: Option<ResMut<crate::hub_windows::ArenaCareer>>,
    mut windows: ResMut<HubWindows>,
    mut shop: HubShop,
    mut buy: ResMut<ShopBuyScript>,
    mut cards: MessageWriter<crate::item_card::WorldCard>,
) {
    let Some(mut career) = career else {
        return;
    };
    for open in opens.read() {
        if let Some(id) = buy.sell {
            if career.goods.is_empty()
                && let Some(possessions) = shop.players.iter().next()
            {
                career.seed_goods(possessions.carried.iter().map(|carried| carried.id));
            }
            if !career.goods.contains(&id) {
                career.goods.push(id);
            }
        }
        shop.open(open.merchant, &mut career, &mut windows);
        info!(
            "hub shop: opened by script (merchant {}), {} on the shelf, {} with the merchant",
            open.merchant,
            career.company.stock.len(),
            career.merchant_stock.len()
        );
        if buy.row.is_some() || buy.sell.is_some() {
            buy.stage = 1;
            buy.wait = SHOP_SCRIPT_WAIT;
        }
    }
    if buy.stage == 0 {
        return;
    }
    if buy.wait > 0 {
        buy.wait -= 1;
        return;
    }
    match buy.stage {
        1 => {
            // The stock entry is dragged into the buy basket and the pack's
            // sold item into the sell basket through the window's own press
            // and release, then the trade confirmed.
            use crate::shop_panels::{ShopDrop, ShopPanel};
            let mut took = false;
            let mut id = 0;
            if let Some(row) = buy.row {
                id = windows
                    .shop
                    .as_ref()
                    .and_then(|shop| shop.panels.stock.entries.get(row))
                    .map_or(0, |entry| entry.id);
                let pressed = windows.shop_press_entry(ShopPanel::Stock, row).is_some();
                let to = HubWindows::shop_panel_middle(ShopPanel::Buy);
                let dropped = windows.shop_release(to, (0, 0), true);
                took |= matches!(dropped, Some(ShopDrop::Moved { .. }));
                info!(
                    "shop buy script: stock entry {row} item {id:#x} pressed {pressed}, dropped {dropped:?}"
                );
            }
            if let Some(sold) = buy.sell {
                let row = windows.shop.as_ref().and_then(|shop| {
                    shop.panels
                        .pack
                        .entries
                        .iter()
                        .position(|entry| entry.id == sold)
                });
                let pressed =
                    row.is_some_and(|row| windows.shop_press_entry(ShopPanel::Pack, row).is_some());
                let dropped = if pressed {
                    let to = HubWindows::shop_panel_middle(ShopPanel::Sell);
                    windows.shop_release(to, (0, 0), true)
                } else {
                    None
                };
                took |= matches!(dropped, Some(ShopDrop::Moved { .. }));
                info!(
                    "shop sell script: pack entry {row:?} item {sold:#x} pressed {pressed}, dropped {dropped:?}"
                );
            }
            let trade = (10, crate::hub_windows::SHOP_SIZE.1 - 20);
            let traded = if took {
                windows.shop_click(trade, &mut career)
            } else {
                None
            };
            let done = traded.is_some();
            // The card is the arsenal's copy (a level's item is promoted).
            let copies = match &traded {
                Some(traded) => shop.arena_trade(traded, &mut career),
                None => Vec::new(),
            };
            info!("shop script: trade {done} ({traded:x?})");
            let id = copies.first().map_or(id, |&(copy, _)| copy);
            buy.bought = if done { id } else { 0 };
            buy.stage = if done && id != 0 { 2 } else { 0 };
            buy.wait = SHOP_SCRIPT_WAIT;
            if done && id != 0 {
                shop.log_copies(&[id]);
            }
        }
        2 => {
            // The card opens over the trade window (an upper-layer form).
            info!("shop buy script: the card of {:#x}", buy.bought);
            cards.write(crate::item_card::WorldCard {
                item: buy.bought,
                flags: 0,
                delayed: false,
            });
            buy.stage = 0;
        }
        _ => buy.stage = 0,
    }
}

/// The night's restock, asked by the day-end button: the career's flag is
/// cleared and the live shop world rotates both shelves.
fn hub_restock(mut career: Option<ResMut<crate::hub_windows::ArenaCareer>>, mut shop: HubShop) {
    let Some(career) = career.as_mut() else {
        return;
    };
    if !career.restock_pending {
        return;
    }
    career.restock_pending = false;
    shop.restock(career);
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
        // The table-built forms take their own pointer, with or without a
        // hub window under them (the company dialog stands alone).
        if windows.forms.contains(click.pointer.x, click.pointer.y) {
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
                            } else if action == exa_ui::match_board::BoardAction::Forfeit {
                                // The tournament's Forfeit asks first; its
                                // Yes forfeits every undecided event.
                                windows.open_yes_no(
                                    crate::tournament::FORFEIT_TOURNEY,
                                    crate::hub_forms::YesHook::ForfeitTourney,
                                );
                            }
                        }
                        HubWindowKind::Shop => {
                            if let Some(trade) = windows.shop_click(local, &mut career) {
                                let _ = shop.arena_trade(&trade, &mut career);
                            }
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

/// The pointer in physical pixels: the scripted one, else the mouse's.
fn hub_pointer(window: &Window, scripted: Option<&exa_ui::ScriptedPointer>) -> Option<(i32, i32)> {
    let at = match scripted {
        Some(scripted) => scripted.position(window),
        None => window.cursor_position(),
    }?;
    let at = (at * window.scale_factor()).as_ivec2();
    Some((at.x, at.y))
}

/// The trade window's character preview rect on the screen: the page's
/// client inside its border, at the constructor's place.
fn shop_preview_rect(bounds: exa_ui::gui_core::GuiRect) -> [i32; 4] {
    let border = crate::shop_window::BORDER;
    let [x, y, w, h] = crate::shop_window::CHARACTER_VIEW;
    [bounds.left + border + x, bounds.top + border + y, w, h]
}

fn inside4(rect: [i32; 4], at: (i32, i32)) -> bool {
    at.0 >= rect[0] && at.1 >= rect[1] && at.0 < rect[0] + rect[2] && at.1 < rect[1] + rect[3]
}

/// The race word of a record's race (bit 4: it wears apparel), from the
/// races table; a human's 4 when the table is not read.
fn race_word(game: Option<&exa_assets::Game>, race: u8) -> u32 {
    game.and_then(|game| {
        crate::scene_bodies::game_races(game)
            .iter()
            .find(|r| r.id == u32::from(race))
            .map(|r| r.flags)
    })
    .unwrap_or(4)
}

/// The trade window's drag, as its item panels and its character preview
/// take the mouse: a left press on an item starts the item's drag, a press
/// on the preview's figure starts a drag of the item its pick finds there;
/// the left release ends it on the panel under the pointer (the panel's
/// accept test, its drop and the source's drag end), on the preview (the
/// view's drop, [`crate::shop_preview::dress`]), or cancels it anywhere
/// else ([`crate::shop_panels`]). A drag off the preview takes the item
/// off as the view's release does ([`crate::shop_preview::take_off`]). A
/// move into or out of a basket redoes its total.
#[allow(clippy::too_many_arguments)]
fn shop_drag(
    buttons: Res<ButtonInput<MouseButton>>,
    screen: Query<&Window, With<bevy::window::PrimaryWindow>>,
    scripted: Option<Res<exa_ui::ScriptedPointer>>,
    mut windows: ResMut<HubWindows>,
    mut icons: Option<ResMut<crate::item_panel_draw::PanelIcons>>,
    catalogue: Option<Res<exa_items::catalogue::ItemCatalogue>>,
    game: Option<Res<exa_assets::Game>>,
    mut items: ResMut<crate::doll_view::DollItems>,
    pick: crate::loadout_kit::PreviewPick,
) {
    let pressed = buttons.just_pressed(MouseButton::Left);
    let released = buttons.just_released(MouseButton::Left);
    if !(pressed || released) || !windows.is_kind(HubWindowKind::Shop) {
        return;
    }
    let Some(pointer) = screen
        .single()
        .ok()
        .and_then(|window| hub_pointer(window, scripted.as_deref()))
    else {
        return;
    };
    let Some(bounds) = windows.bounds() else {
        return;
    };
    let local = (pointer.0 - bounds.left, pointer.1 - bounds.top);
    let preview_rect = shop_preview_rect(bounds);
    let catalogue = catalogue.as_deref();
    if pressed
        && !windows.forms.contains(pointer.0, pointer.1)
        && windows
            .shop
            .as_ref()
            .is_some_and(|shop| shop.preview_drag.is_none())
    {
        if let Some(drag) =
            windows.shop_press(local, |id| crate::panel_live::bulk_of(id, catalogue))
        {
            info!(
                "hub shop: drag of {:#x} (flags {:#x}) from the {:?} at {pointer:?}",
                drag.id, drag.flags, drag.source
            );
        } else if inside4(preview_rect, pointer)
            && let Some(shop) = windows.shop.as_mut()
            && let Some(record) = shop.preview.as_ref()
        {
            items.ensure(game.as_deref());
            let p = crate::shop_preview::possessions(record);
            if let Some(item) = pick.item_at(preview_rect, pointer, &p, &items) {
                let (icon, _) = icons.as_deref_mut().map_or((0, (0, 0)), |icons| {
                    icons.reserve(item.id, item.flags, catalogue)
                });
                shop.preview_drag = Some(crate::shop_preview::PreviewDrag {
                    item,
                    icon,
                    grab: crate::item_container::drag_grab(shop.panels.pack.icon_size),
                });
                info!("hub shop: drag of {item:?} off the preview at {pointer:?}");
            }
        }
    }
    if !released {
        return;
    }
    items.ensure(game.as_deref());
    let find = |id: u32| items.get(id);
    let kind_of = |id: u32| catalogue.and_then(|c| c.kind(id));
    // A drag off the preview: the panel under the pointer may take it,
    // then the view's release.
    if let Some(shop) = windows.shop.as_mut()
        && let Some(drag) = shop.preview_drag.take()
    {
        let item = drag.item;
        let client = (
            local.0 - crate::shop_window::BORDER,
            local.1 - crate::shop_window::BORDER,
        );
        let bulk = crate::panel_live::bulk_of(item.id, catalogue);
        let half = icons
            .as_deref()
            .and_then(|icons| icons.half(drag.icon))
            .unwrap_or((0, 0));
        let mut landed = None;
        if let Some((panel, cell)) = shop.panels.panel_at(client) {
            let accepted = shop.panels.get(panel).accept(
                &crate::item_container::Drag {
                    src: Some(crate::shop_preview::PREVIEW_PANEL),
                    dst: Some(panel.number()),
                    objid: item.id,
                    objflags: item.flags,
                    icon: drag.icon as u32,
                    bulk: bulk.clamp(0, 255) as u8,
                    model_found: crate::panel_live::model_found(item.id, catalogue),
                },
                panel.number(),
            );
            if accepted
                && let Some(done) = shop.panels.get_mut(panel).drop_entry(
                    crate::item_container::DropDrag {
                        same_panel: false,
                        linked: false,
                        id: item.id,
                        icon: drag.icon,
                        flags: item.flags,
                        bulk,
                    },
                    drag.grab,
                    cell,
                    half,
                )
            {
                landed = Some((panel, done.added.is_some()));
                if panel.is_basket() {
                    shop.basket_changed(panel);
                }
            }
        }
        // The new entry keeps the drag's icon reference; otherwise it goes.
        if !landed.is_some_and(|(_, added)| added)
            && let Some(icons) = icons.as_deref_mut()
        {
            icons.release(drag.icon);
        }
        let off = crate::shop_preview::release_takes_off(item.flags, landed.is_some())
            && shop
                .preview
                .as_mut()
                .is_some_and(|record| crate::shop_preview::take_off(record, item, find, kind_of));
        info!(
            "hub shop: the drag of {item:?} off the preview ends at {pointer:?}: into {:?}, taken off {off}; worn now {:?}, hands {:?}",
            landed.map(|(panel, _)| panel),
            shop.preview
                .as_ref()
                .map(|r| r.worn.iter().map(|w| w.id).collect::<Vec<_>>()),
            shop.preview.as_ref().map(|r| r.hands.map(|h| h.id))
        );
        return;
    }
    let Some(drag) = windows.shop.as_ref().and_then(|shop| shop.panels.drag) else {
        return;
    };
    // A panel's drag released over the preview: the view's drop.
    if inside4(preview_rect, pointer)
        && let Some(shop) = windows.shop.as_mut()
        && let Some(record) = shop.preview.as_mut()
    {
        let race = race_word(game.as_deref(), record.race);
        let dressed = crate::shop_preview::dress(record, drag.id, drag.flags, race, find, kind_of);
        match &dressed {
            Some(done) => {
                shop.panels.drag = None;
                let source = shop.panels.get_mut(drag.source);
                source.finish_drag(
                    crate::shop_preview::PREVIEW_PANEL,
                    false,
                    true,
                    drag.flags,
                    drag.bulk,
                );
                if drag.source.is_basket() {
                    shop.basket_changed(drag.source);
                }
                for &(id, flags) in &done.to_pack {
                    let (icon, _) = icons
                        .as_deref_mut()
                        .map_or((0, (0, 0)), |icons| icons.reserve(id, flags, catalogue));
                    let bulk = crate::panel_live::bulk_of(id, catalogue);
                    shop.panels.pack.add_at_centre(id, icon, flags, bulk);
                }
            }
            None => shop.panels.cancel(),
        }
        info!(
            "hub shop: the drag of {:#x} from the {:?} ends on the preview at {pointer:?}: {dressed:?}; worn now {:?}, hands {:?}",
            drag.id,
            drag.source,
            shop.preview
                .as_ref()
                .map(|r| r.worn.iter().map(|w| w.id).collect::<Vec<_>>()),
            shop.preview.as_ref().map(|r| r.hands.map(|h| h.id))
        );
        return;
    }
    let half = icons
        .as_deref()
        .and_then(|icons| icons.half(drag.icon))
        .unwrap_or((0, 0));
    let found = crate::panel_live::model_found(drag.id, catalogue);
    let dropped = windows.shop_release(local, half, found);
    let totals = windows.shop.as_ref().map(|shop| shop.window.shown_balance);
    info!(
        "hub shop: the drag of {:#x} ends at {pointer:?}: {dropped:?}; bottom row {totals:?}",
        drag.id
    );
}

/// The trade window's preview: the opener's copy of the player's record
/// (its worn list and hands the body's live ones), shown through the hub
/// preview's view at the window's character view place.
fn sync_shop_preview(
    mut windows: ResMut<HubWindows>,
    players: Query<
        (
            &exa_character::record::CharacterData,
            &exa_items::possession::Possessions,
        ),
        With<crate::player::PlayerBody>,
    >,
    mut preview: ResMut<crate::loadout_kit::KitPreview>,
    mut panels: ResMut<crate::loadout_kit::KitPanels>,
) {
    if !windows.is_kind(HubWindowKind::Shop) {
        return;
    }
    let Some(bounds) = windows.bounds() else {
        return;
    };
    let Some(shop) = windows.shop.as_mut() else {
        return;
    };
    if shop.preview.is_none() {
        let mut record = players
            .iter()
            .next()
            .map(|(data, _)| data.record.clone())
            .unwrap_or_else(exa_formats::characters::CharacterRecord::reset);
        if let Some((_, p)) = players.iter().next() {
            record.worn = p.worn.clone();
            record.hands = [p.out[0], p.out[1], p.stowed[0], p.stowed[1]]
                .map(|h| exa_formats::characters::ItemRef::new(h.id, h.flags));
        }
        info!(
            "hub shop: the preview copies the player's record: worn {:?}, hands {:?}",
            record.worn.iter().map(|w| w.id).collect::<Vec<_>>(),
            record.hands.map(|h| h.id)
        );
        shop.preview = Some(record);
    }
    let [x, y, w, h] = shop_preview_rect(bounds);
    preview.rect = Some(exa_ui::gui_core::GuiRect::new(x, y, w, h));
    if preview.record != shop.preview {
        panels.revision += 1;
        preview.revision = panels.revision;
        preview.record.clone_from(&shop.preview);
    }
}

/// Gives the icon manager back the icons a closed trade window's panels
/// held.
fn release_shop_icons(
    mut windows: ResMut<HubWindows>,
    icons: Option<ResMut<crate::item_panel_draw::PanelIcons>>,
) {
    if windows.released_icons.is_empty() {
        return;
    }
    let handles = std::mem::take(&mut windows.released_icons);
    if let Some(mut icons) = icons {
        for handle in handles {
            icons.release(handle);
        }
    }
}

/// A right press over an item of the trade window asks for its card, as
/// the window's item panels do: the item and the flags its panel holds it
/// with ([`crate::hub_windows::SHOP_ITEM_FLAGS`] for the stock), delayed
/// on the card timer for use kinds 0 to 2, at once for anything else. A
/// press over the shown card is the card's own. Either way the release
/// that follows does not close the window.
#[allow(clippy::too_many_arguments)]
fn shop_item_cards(
    buttons: Res<ButtonInput<MouseButton>>,
    screen: Query<&Window, With<bevy::window::PrimaryWindow>>,
    scripted: Option<Res<exa_ui::ScriptedPointer>>,
    mut windows: ResMut<HubWindows>,
    view: Option<Res<crate::item_card::CardView>>,
    screens: Option<Res<crate::gui_screens::GameScreens>>,
    catalogue: Option<Res<exa_items::catalogue::ItemCatalogue>>,
    mut cards: MessageWriter<crate::item_card::WorldCard>,
) {
    if !buttons.just_pressed(MouseButton::Right) || !windows.is_kind(HubWindowKind::Shop) {
        return;
    }
    let Ok(window) = screen.single() else {
        return;
    };
    let pointer = match scripted.as_deref() {
        Some(scripted) => scripted.position(window),
        None => window.cursor_position(),
    };
    let Some(pointer) = pointer.map(|p| (p * window.scale_factor()).as_ivec2()) else {
        return;
    };
    let unit = exa_ui::scale::gui_scale(window.physical_height(), 0) as i32;
    if view
        .as_deref()
        .and_then(|view| view.shown.as_ref())
        .is_some_and(|card| card.hit((pointer.x, pointer.y), unit.max(1)))
    {
        windows.card_press = true;
        return;
    }
    let Some(bounds) = windows.bounds() else {
        return;
    };
    let local = (pointer.x - bounds.left, pointer.y - bounds.top);
    let Some((item, flags)) = windows.shop_item_at(local) else {
        return;
    };
    windows.card_press = true;
    let kind = catalogue.as_deref().and_then(|c| c.card_kind(item));
    let pending = screens
        .as_deref()
        .is_some_and(|screens| screens.card.timer.due.is_some());
    let delayed = match crate::item_container::card_action(true, pending, kind) {
        crate::item_container::CardAction::None => return,
        crate::item_container::CardAction::Delayed { .. } => true,
        crate::item_container::CardAction::Immediate => false,
    };
    info!(
        "hub shop: the card of {item:#x} (flags {flags:#x}) asked at {:?}{}",
        (pointer.x, pointer.y),
        if delayed { ", delayed" } else { "" }
    );
    cards.write(crate::item_card::WorldCard {
        item,
        flags,
        delayed,
    });
}

/// Escape and the windows' own close end the open hub window. A right
/// release after an item's card request is the item panel's, not the
/// window's.
fn close_hub_windows(
    keys: Res<ButtonInput<KeyCode>>,
    buttons: Res<ButtonInput<MouseButton>>,
    mut windows: ResMut<HubWindows>,
    mut hub: ResMut<ArenaHub>,
) {
    // A message box on top takes the keys (Enter and Space close it,
    // Escape closes it with -1) and nothing else closes under it.
    if windows.forms.message_on_top() {
        let code = if keys.any_just_pressed([KeyCode::Enter, KeyCode::NumpadEnter]) {
            Some(exa_ui::modal_screens::KEY_ENTER)
        } else if keys.just_pressed(KeyCode::Space) {
            Some(exa_ui::modal_screens::KEY_SPACE)
        } else if keys.just_pressed(KeyCode::Escape) {
            Some(exa_ui::modal_screens::KEY_ESCAPE)
        } else {
            None
        };
        if let Some(code) = code {
            windows.forms.message_key(code);
        }
        return;
    }
    let right = buttons.just_released(MouseButton::Right);
    if right && std::mem::take(&mut windows.card_press) {
        return;
    }
    if windows.is_open() && (keys.just_pressed(KeyCode::Escape) || right) && windows.close_top() {
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

        // The day's end rotates the shelf once the live world supplies it.
        app.world_mut()
            .resource_mut::<crate::hub_windows::ArenaCareer>()
            .restock_pending = true;
        app.update();
        assert_ne!(
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
            app.world_mut()
                .resource_mut::<HubWindows>()
                .open_shop(false, &career, &[], |_, _| (0, (80, 80)));
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
            outfit: Default::default(),
        });
        let board = career.board();
        // A duel takes the combatant (pool slot 1), not the manager.
        assert_eq!(board.team, vec![1]);
        let launch = plan_launch_at(&mut career, &board, false, 0x1234_5678).unwrap();
        assert_eq!(launch.player, None);
        // The participant list is the lineup's pool slots.
        assert_eq!(launch.entrants, vec![1]);
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
    fn a_weapon_event_start_settles_earlier_events_and_carries_the_hands() {
        let mut career = crate::hub_windows::ArenaCareer::default();
        career.company.day = 0;
        // Event 0 has no entrant, event 1 is the company's with its first
        // entrant's two weapon words, event 2 has no entrant either.
        career.entries[1].count = 1;
        career.entries[1].entries[0].weapon = [0x0000_0001_0000_0042, 0x0000_0002_0000_0043];
        let mut board = career.board();
        board.tab = exa_ui::match_board::BoardTab::Tournament;
        board.selected = exa_ui::match_board::STANDARD_MATCHES[0].record();
        // Champion: a weapon event.
        board.selected.kind = 17;
        board.selected.flags = exa_ui::match_board::FLAG_WEAPON;
        board.selected.team_size = 1;
        board.rebuild_lineups();
        let launch = plan_launch_at(&mut career, &board, false, 0x1000_0000).unwrap();
        assert_eq!(
            launch.event_hands,
            Some([0x0000_0001_0000_0042, 0x0000_0002_0000_0043])
        );
        // The earlier unentered event takes a rival winner of 2 to 7; the
        // pass stops at the entered one.
        assert!((2..=7).contains(&career.entries[0].team_id));
        assert_eq!(career.entries[1].team_id, 0);
        assert_eq!(career.entries[2].team_id, 0);
        // A standard match carries no event hands and settles nothing.
        let mut career = crate::hub_windows::ArenaCareer::default();
        let board = career.board();
        let launch = plan_launch_at(&mut career, &board, false, 0x1000_0000).unwrap();
        assert_eq!(launch.event_hands, None);
        assert_eq!(career.entries[0].team_id, 0);
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
            outfit: Default::default(),
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
    fn a_specialist_hired_in_the_hub_stands_at_once() {
        let mut app = hub_app();
        let mut flow = Flow::default();
        flow.mode = crate::flow::Mode::Arena;
        flow.phase = Phase::Playing;
        app.insert_resource(flow);
        spawn_hub_level(&mut app);
        spawn_player(&mut app);
        app.update();
        let drained = app
            .world_mut()
            .resource_mut::<Messages<crate::scene_bodies::SceneCommand>>()
            .drain()
            .count();
        assert_eq!(drained, 1, "the shopkeeper");
        app.world_mut()
            .resource_mut::<crate::hub_windows::ArenaCareer>()
            .company
            .roster
            .push(member(crate::career::Profession::Physician));
        app.update();
        app.update();
        let commands: Vec<crate::scene_bodies::SceneCommand> = app
            .world_mut()
            .resource_mut::<Messages<crate::scene_bodies::SceneCommand>>()
            .drain()
            .collect();
        assert_eq!(commands.len(), 1, "the hired physician, once");
        assert_eq!(
            app.world().resource::<ArenaHub>().specialists_placed,
            vec![crate::career::Profession::Physician]
        );
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
