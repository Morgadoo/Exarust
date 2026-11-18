//! The arena match's live scene: the round's bodies, the end screen's
//! Continue, the next round and the way back to the hub.
//!
//! The hub's board pick changes the level to the drawn arena in process
//! ([`exa_world::level_lifecycle`]) and keeps the career in its
//! `ArenaCareer` resource. This module stands the round's bodies from the
//! match's layout and the spawn plans — a generated opponent from
//! [`crate::arena_spawn::generic_plan`] or a tournament's hire-list profile
//! with its drawn weapon, the Beast, and the company's other entrants. The
//! player's own body is the one the play start already made.
//!
//! When the round result opens a transition screen, [`ArenaEndScreen`]
//! draws the original's end menu through the GUI path: `NEXT FIGHT` for
//! screen 3 and `EXIT;` for screen 2, the arena's single-row end menu. Its
//! first row confirms the Continue on Enter or on a pointer release over it.
//! A next round despawns the round's bodies and stands the winner and the
//! reserve or a fresh opponent after the original's 13 ms yield; a
//! tournament's next round first resolves its bracket (the won group keeps
//! team one, the other groups draw a winner) and stands the first surviving
//! teams, as many as the event has sides. Escape opens the arena's pause page and its FORFEIT row settles
//! the match as forfeited. The end of the match settles the result into the
//! career, drops the unfit on a tournament day and asks for the hub's level
//! in process, whose entry places the manager at the saved 300 cm, half
//! turn, flag `0x100` placement.
//!
//! Evidence: the arena's match tick, end screen, next-round routine and
//! return (`arena_session`, `arena_match`, `match_layout`).

use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use exa_combat::{Condition, Fighter};
use exa_ui::gui_draw::{GuiFrames, GuiTextures};

use crate::arena;
use crate::arena_match::{ArenaMatch, CompanyRecords, Finish, MatchState, any_entry};
use crate::arena_session::{
    BracketState, EndAction, ReturnFighter, RoundFight, Screen, bracket_for_template,
    bracket_next_fight, regenerate_round, settle_return, write_back_health,
};
use crate::arena_spawn::{MANAGER_ROLE, generic_plan, hire_plan};
use crate::match_layout::{
    Fighter as SlotFighter, FreshMatch, HOSTILE, MATCH_ACT_GRACE_MS, NextRound, RaisedBody, Slot,
    layout, opponent_place, our_place, raise_fresh_match,
};
use crate::scene_bodies::{
    BodyBrain, SceneBodyPlan, SceneCommand, SceneFighter, beast_scene_plan, company_plan,
    hire_opponent_plan, hire_row_plan, opponent_plan,
};

/// The Captain's template id: its entrants after the first take the
/// loadout two ranks under the tier.
const CAPTAIN_TEMPLATE: i32 = 9;

/// The running match's scene state.
#[derive(Resource, Clone, Debug, PartialEq)]
pub struct ArenaScene {
    /// The match's template id, as the layout reads it.
    pub template: i32,
    /// The arena's spawn spot, centimetres.
    pub spawn_at: [f32; 3],
    /// The match's rank: the generated opponents' level.
    pub level: i32,
    pub tier: i32,
    pub tournament: bool,
    /// The company entry that is the player's own, as the launch planned it.
    pub player_entry: Option<usize>,
    /// The drawn weapon variant's item id per opponent, in layout order.
    pub weapons: Vec<u32>,
    /// The prize the match pays when won, copper.
    pub prize: u32,
    /// The first round's bodies were asked for.
    pub placed: bool,
    /// The company's thirteen records, carried from the career for the
    /// settle.
    pub records: CompanyRecords,
    /// A tournament event's bracket, from its template's teams and rounds.
    pub bracket: Option<BracketState>,
    /// The event weapon row's three variants, when the event arms its
    /// fighters.
    pub weapon_variants: Option<[u32; 3]>,
    /// The week's tournament field: the rival company each team from 2 on
    /// wears.
    pub rivals: [i32; crate::career::RIVAL_FIELD],
    /// The roll a bracket's unwatched pairs draw from.
    pub roll: u32,
    /// The participants in entry order, as company pool slots (0 is the
    /// manager, roster member `i` is slot `i + 1`); empty for a match the
    /// command line laid out, whose entry `i` is slot `i`.
    pub entrants: Vec<usize>,
    /// A weapon event's hands: the two weapon words of the event's first
    /// entrant, which every company copy holds ([`crate::hirelings::match_copy`]).
    pub event_hands: Option<[u64; 2]>,
}

impl ArenaScene {
    /// A scene at the arena's spot with the career's records.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        template: i32,
        spawn_at: [f32; 3],
        level: i32,
        tier: i32,
        tournament: bool,
        player: Option<usize>,
        weapons: Vec<u32>,
        prize: u32,
        career: Option<&crate::hub_windows::ArenaCareer>,
    ) -> Self {
        Self {
            template,
            spawn_at,
            level,
            tier,
            tournament,
            player_entry: player,
            weapons,
            prize,
            placed: false,
            records: career.map_or_else(CompanyRecords::default, |career| career.records()),
            bracket: if tournament {
                bracket_for_template(template)
            } else {
                None
            },
            weapon_variants: None,
            rivals: [0; crate::career::RIVAL_FIELD],
            roll: crate::arena_match::counter_now() as u32,
            entrants: Vec::new(),
            event_hands: None,
        }
    }

    /// A weapon event's hands for the company copies.
    #[must_use]
    pub fn with_event_hands(mut self, hands: Option<[u64; 2]>) -> Self {
        self.event_hands = hands;
        self
    }

    /// Company entry `index`'s match copy: the roster member's skills,
    /// fighting skill and kit for the match's tier, a Captain's entrants
    /// after the first two ranks under it. The manager (slot 0) has none:
    /// the career does not keep his kit.
    #[must_use]
    pub fn match_copy(
        &self,
        career: Option<&crate::hub_windows::ArenaCareer>,
        index: usize,
    ) -> Option<crate::hirelings::MatchCopy> {
        let slot = self.entrant_slot(index);
        let member = career?.company.roster.get(slot.checked_sub(1)?)?;
        Some(crate::hirelings::match_copy(
            member,
            self.tier,
            self.template == CAPTAIN_TEMPLATE && index > 0,
            self.event_hands,
        ))
    }

    /// The board's participants, as company pool slots.
    #[must_use]
    pub fn with_entrants(mut self, entrants: Vec<usize>) -> Self {
        self.entrants = entrants;
        self
    }

    /// The company pool slot of the layout's company entry `index`: the
    /// participant list maps it (the original's participants are the
    /// board's entrants mapped by id, in entry order).
    #[must_use]
    pub fn entrant_slot(&self, index: usize) -> usize {
        self.entrants.get(index).copied().unwrap_or(index)
    }

    /// Company entry `index`'s member (name and health word) and the id its
    /// body carries (its pool slot, the record's participant id): slot 0 is
    /// the manager, slot `i` roster member `i - 1`.
    #[must_use]
    pub fn entrant<'a>(
        &self,
        career: Option<&'a crate::hub_windows::ArenaCareer>,
        index: usize,
    ) -> (Option<(&'a str, u16)>, u32) {
        let slot = self.entrant_slot(index);
        let member = career.and_then(|career| {
            let member = match slot {
                0 => Some(&career.player),
                slot => career.company.roster.get(slot - 1),
            };
            member.map(|member| (member.name.as_str(), member.health))
        });
        (member, u32::try_from(slot).unwrap_or(0))
    }

    /// The week's tournament field the rival sides wear.
    #[must_use]
    pub fn with_rivals(mut self, rivals: [i32; crate::career::RIVAL_FIELD]) -> Self {
        self.rivals = rivals;
        self
    }

    /// The event weapon row's variants, for the bracket's later fights.
    #[must_use]
    pub fn with_weapon_variants(mut self, variants: Option<[u32; 3]>) -> Self {
        self.weapon_variants = variants;
        self
    }
}

/// A launched match's setup, as the hub's board chose it. It stays in
/// process now that the level changes in place; the career itself is the
/// running `ArenaCareer` resource.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MatchSetup {
    /// The layout's template id.
    pub template: i32,
    /// The event's rank: the generated fighters' level.
    pub level: i32,
    /// The event's gear tier.
    pub tier: i32,
    /// A tournament event's match.
    pub tournament: bool,
    /// The company entry that is the player's own, as the launch planned it.
    pub player: Option<usize>,
    /// The match's prize when won, copper.
    pub prize: u32,
    /// A tournament event's weapon row: the three item variants later
    /// bracket fights draw from.
    pub weapon_variants: Option<[u32; 3]>,
}

/// The match's exit: where the process goes when the end screen's Continue
/// settles it.
#[derive(Message, Clone, Debug, PartialEq)]
pub enum ArenaExit {
    /// The settled match returns to the hub's level in process, with the
    /// career resource and the return placement the settle wrote.
    Hub,
    /// The manager is dead: the main menu.
    MainMenu,
}

/// The original's dormant auto-match switch: the end screen's Continue
/// runs without a key, and the hub replays the last match
/// ([`crate::arena_hub_live::ArenaRematch`]). The game's own code never sets
/// it; `--arena-auto` does, for unattended runs and tests.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ArenaAutoContinue(pub bool);

/// The arena's end screen: the original's end-session menu as a round's
/// result opens it.
///
/// Screen 3 (the next round) shows the single `NEXT FIGHT` row and screen 2
/// (the match's end) the single `EXIT;` row, both at the end menu's scale.
/// The rows come from [`crate::flow::end_menu`] in arena mode; the frame is
/// painted by [`crate::end_menu_screen::end_menu_frame`], the same GUI path
/// the practice and story end menus use. Enter runs the selected row and a
/// pointer release over it runs it too; Up and Down move the selection.
#[derive(Resource, Clone, Debug, Default, PartialEq)]
pub struct ArenaEndScreen {
    pub open: bool,
    /// The menu number: 3 for the next round, 2 for the match's end.
    pub kind: u32,
    /// The selected row, 0-based.
    pub selected: usize,
    /// The row a press is held on.
    pub pressed: Option<usize>,
}

impl ArenaEndScreen {
    /// Opens the screen a transition word names.
    pub fn show(&mut self, screen: Screen) {
        self.open = true;
        self.kind = screen.word();
        self.selected = 0;
        self.pressed = None;
    }

    /// Hides the screen.
    pub fn close(&mut self) {
        self.open = false;
        self.pressed = None;
    }

    /// The menu the screen paints, when it is open.
    #[must_use]
    pub fn menu(&self) -> Option<crate::flow::EndMenu> {
        self.open
            .then(|| crate::flow::end_menu(self.kind, crate::flow::Mode::Arena))
    }

    /// Moves the selection by `delta`, clamped to the rows.
    pub fn move_selection(&mut self, delta: i32) {
        let Some(menu) = self.menu() else {
            return;
        };
        let count = menu.rows.len() as i32;
        if count == 0 {
            self.selected = 0;
            return;
        }
        self.selected = (self.selected as i32 + delta).clamp(0, count - 1) as usize;
    }

    /// Runs the selected row, returning whether the arena's Continue should
    /// handle it. Only the first row continues; a later row is the lobby
    /// choice, which the original sends to the title and this screen does
    /// not raise.
    pub fn confirm(&mut self) -> bool {
        let continues = self.open && self.selected == 0;
        self.close();
        continues
    }
}

/// The end screen's Continue was pressed. The arena's continue system reads
/// it; the row's action is the same whichever screen opened the menu.
#[derive(Message, Clone, Copy, Debug, PartialEq, Eq)]
pub struct ArenaContinue;

/// The arena's pause page asked to forfeit the match (the flow's
/// [`crate::flow::Step::Forfeit`]).
#[derive(Message, Clone, Copy, Debug, PartialEq, Eq)]
pub struct ArenaForfeit;

/// Runs the live arena scene.
pub struct ArenaLivePlugin;

impl Plugin for ArenaLivePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ArenaAutoContinue>()
            .init_resource::<ArenaEndScreen>()
            .add_message::<ArenaExit>()
            .add_message::<ArenaContinue>()
            .add_message::<ArenaForfeit>()
            .add_message::<SceneCommand>()
            .add_systems(
                Update,
                (
                    mark_arena_session.after(crate::save_live::run_save_steps::<false>),
                    stand_first_round.after(crate::save_live::load_arena_slot),
                    arena_pause,
                    open_end_screen.after(crate::arena_fade::start_held_match),
                    update_end_screen,
                    continue_round
                        .before(exa_world::level_lifecycle::apply_requests)
                        .after(crate::equip::apply_pending_hands),
                    forfeit_match::<false>,
                    draw_end_screen,
                )
                    .chain()
                    .after(crate::gui_screens::draw_screens)
                    // A staging's callback fires in the flow's steps; the
                    // Continue's staging runs in the same frame after them.
                    .after(crate::flow::FlowSystems::ApplySteps)
                        .in_set(exa_physics::FramePhase::Session),
            )
            // The quit's forfeit (the arena pause page's QUIT) settles the
            // match before the quit's saves are written.
            .add_message::<crate::save_live::SaveStep>()
            .add_message::<crate::flow::ForfeitMatch>()
            .add_systems(
                Last,
                forfeit_match::<true>.in_set(crate::save_live::SaveLiveSystems::QuitForfeit),
            );
    }
}

/// The bodies a match's layout asks the generator for.
///
/// The player's own entry is not spawned again; every other slot gets its
/// plan — an opponent at the layout's level, tier, team and flags (a
/// tournament side's hire-list profile with the event row's drawn weapon,
/// the Beast's own plan), or a company entrant.
#[must_use]
pub fn round_plans(
    scene: &ArenaScene,
    career: Option<&crate::hub_windows::ArenaCareer>,
    slots: &[Slot],
    now_ms: u32,
    act_grace_ms: u32,
) -> Vec<SceneBodyPlan> {
    let mut plans = Vec::with_capacity(slots.len());
    let mut opponent = 0usize;
    for slot in slots {
        match slot.who {
            SlotFighter::Ours(index) => {
                if scene.player_entry == Some(index) {
                    continue;
                }
                // The participant id is the entrant's pool slot: 0 is the
                // manager, the first roster member is 1, as the numbering
                // pass writes it into each record's id word.
                let (member, id) = scene.entrant(career, index);
                plans.push(company_plan(
                    member,
                    scene.match_copy(career, index),
                    &slot.place,
                    MANAGER_ROLE,
                    id,
                    now_ms,
                    act_grace_ms,
                ));
            }
            SlotFighter::Opponent {
                level,
                tier,
                team,
                flags,
            } => {
                let mut body = if scene.tournament {
                    // Each rival side wears the company the week's field
                    // names for its team.
                    let company = crate::arena_spawn::rival_company(&scene.rivals, team as i32);
                    match scene.weapon_variants {
                        // The event's weapon row: the variant is drawn while
                        // the record is generated.
                        Some(row) => hire_row_plan(
                            hire_plan(tier as u8, company, 1),
                            &slot.place,
                            flags,
                            row,
                            now_ms,
                            act_grace_ms,
                        ),
                        // A command-line match names its weapons instead.
                        None => {
                            let weapon = scene.weapons.get(opponent).copied().filter(|&id| id != 0);
                            hire_opponent_plan(
                                hire_plan(tier as u8, company, i32::from(weapon.is_some())),
                                &slot.place,
                                flags,
                                weapon,
                                now_ms,
                                act_grace_ms,
                            )
                        }
                    }
                } else {
                    opponent_plan(
                        generic_plan(level, tier),
                        &slot.place,
                        flags,
                        now_ms,
                        act_grace_ms,
                    )
                };
                body.team = team;
                opponent += 1;
                plans.push(body);
            }
            SlotFighter::Beast { level, tier } => {
                let mut body = beast_scene_plan(
                    u32::try_from(tier).unwrap_or(1).max(1),
                    &slot.place,
                    now_ms,
                    act_grace_ms,
                );
                body.id = u32::try_from(level).unwrap_or(0);
                plans.push(body);
            }
        }
    }
    plans
}

type MatchBody<'a> = (
    Entity,
    &'a Fighter,
    &'a Condition,
    &'a SceneFighter,
    Has<crate::player::PlayerBody>,
);

/// A body whose kit the next round re-dresses: its record, the possessions
/// that mirror the record, and whether it is the player.
type KitBody<'a> = (
    Entity,
    &'a mut exa_character::record::CharacterData,
    Option<&'a mut exa_items::possession::Possessions>,
    Option<&'a mut crate::save_kit::OwnDatabase>,
    Has<crate::player::PlayerBody>,
);

/// Preparation then equip on one body that stays. An actor's own database
/// is used when the body carries one; otherwise the live database, whose
/// local tier is the loaded level's. With neither, own ids stay.
fn redress(
    kits: &mut Query<KitBody<'_>>,
    level: Option<&mut exa_assets::live_objects::LiveObjects>,
    entity: Entity,
) {
    let Ok((_, mut data, mut held, mut own, _)) = kits.get_mut(entity) else {
        return;
    };
    let database = match own.as_deref_mut() {
        Some(own) => Some(&mut own.0),
        None => level.map(|level| level.db_mut()),
    };
    crate::save_kit::dress_kit(&mut data.record, &mut [], None, database);
    if let Some(held) = held.as_mut() {
        crate::save_kit::mirror_kit(&data.record, held);
    }
}

/// A body the scene draw counts: every fighter the match stood, and the
/// player's own body, which carries no scene marker.
type DrawBody<'a> = (
    &'a Fighter,
    &'a Condition,
    Option<&'a SceneFighter>,
    Has<crate::player::PlayerBody>,
);

/// The scene draw's view of the match's bodies
/// ([`crate::career::draw_unwatched`]): each body's team word (the player's
/// and the company's are the player's team), its red health as its
/// strength, and a collapsing or dead body (the condition's dying bits) set
/// aside.
fn scene_sides(bodies: &Query<DrawBody<'_>>) -> Vec<crate::career::SideFighter> {
    bodies
        .iter()
        .filter(|(_, _, marker, player)| marker.is_some() || *player)
        .map(
            |(fighter, condition, marker, player)| crate::career::SideFighter {
                team: match marker {
                    Some(marker) if !player && !marker.company => marker.team as i32,
                    _ => crate::arena_match::PLAYER_TEAM,
                },
                strength: fighter.health.red,
                aside: condition.has(Condition::DYING_MASK),
            },
        )
        .collect()
}

/// The running match is an arena session: the arena's pause page and the
/// end-of-session dispatch read the arena mode, not the play view's practice
/// stand-in.
fn mark_arena_session(
    arena: Option<Res<ArenaMatch>>,
    mut flow: Option<ResMut<crate::flow::Flow>>,
    levels: Query<&exa_world::LevelRoot>,
) {
    if arena.is_none() {
        return;
    }
    if !levels
        .iter()
        .any(|level| crate::arena_match::arena_index(&level.name).is_some())
    {
        return;
    }
    if let Some(flow) = flow.as_mut()
        && flow.mode != crate::flow::Mode::Arena
    {
        flow.mode = crate::flow::Mode::Arena;
        flow.phase = crate::flow::Phase::Playing;
        flow.hook = crate::flow::Hook::None;
    }
}

/// Escape opens the arena's pause page; closing it (by Escape, by the page's
/// CONTINUE row or by the menu's own close) resumes the pause.
#[allow(clippy::too_many_arguments)]
fn arena_pause(
    arena: Option<Res<ArenaMatch>>,
    mut flow: Option<ResMut<crate::flow::Flow>>,
    mut queued: Option<ResMut<crate::flow::FlowSteps>>,
    menu: Option<Res<crate::menu_screen::MenuScreen>>,
    end: Option<Res<ArenaEndScreen>>,
    keys: Option<Res<ButtonInput<KeyCode>>>,
) {
    let (Some(arena), Some(flow), Some(queued)) = (arena, flow.as_mut(), queued.as_mut()) else {
        return;
    };
    if arena.state == MatchState::Fighting && arena.screen.is_none() {
        // A live round can be paused.
    } else if flow.phase != crate::flow::Phase::Paused {
        return;
    }
    if end.as_ref().is_some_and(|end| end.open) {
        return;
    }
    let menu_open = menu.as_ref().is_some_and(|menu| menu.open);
    if flow.phase == crate::flow::Phase::Paused {
        if !menu_open {
            queued.steps.extend(flow.resume());
        }
        return;
    }
    if menu_open {
        return;
    }
    if keys
        .as_ref()
        .is_some_and(|keys| keys.just_pressed(KeyCode::Escape))
    {
        queued.steps.extend(flow.pause());
    }
}

/// Opens the end screen the round's transition asked for, and closes it once
/// the match carries no transition.
fn open_end_screen(
    arena: Option<Res<ArenaMatch>>,
    mut screen: Option<ResMut<ArenaEndScreen>>,
    staging: Option<Res<crate::arena_fade::ArenaStaging>>,
) {
    let (Some(arena), Some(screen)) = (arena, screen.as_mut()) else {
        return;
    };
    // A Continue behind the transition screen keeps the menu closed.
    if staging.is_some_and(|staging| staging.round_pending()) {
        return;
    }
    match arena.screen {
        Some(transition) if !screen.open => screen.show(transition),
        None if screen.open => screen.close(),
        _ => {}
    }
}

/// The end screen's input: Enter and a release over a row confirm, Up and
/// Down move, and the pointer selects the row under it.
fn update_end_screen(
    mut screen: Option<ResMut<ArenaEndScreen>>,
    keys: Option<Res<ButtonInput<KeyCode>>>,
    mouse: Option<Res<ButtonInput<MouseButton>>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut confirmed: MessageWriter<ArenaContinue>,
) {
    let Some(screen) = screen.as_mut() else {
        return;
    };
    if !screen.open {
        return;
    }
    let Some(menu) = screen.menu() else {
        return;
    };
    let Some(keys) = keys else {
        return;
    };
    if keys.just_pressed(KeyCode::ArrowDown) {
        screen.move_selection(1);
    }
    if keys.just_pressed(KeyCode::ArrowUp) {
        screen.move_selection(-1);
    }
    if keys.just_pressed(KeyCode::Enter) {
        if screen.confirm() {
            confirmed.write(ArenaContinue);
        }
        return;
    }
    let Ok(window) = windows.single() else {
        return;
    };
    let width = window.physical_width() as i32;
    let height = window.physical_height() as i32;
    let layout = crate::end_menu_screen::end_layout(width, height, &menu, Some(screen.selected));
    let Some(cursor) = window.cursor_position() else {
        return;
    };
    let scale = window.scale_factor();
    let hit = layout.hit((cursor.x * scale) as i32, (cursor.y * scale) as i32);
    if let Some(row) = hit {
        screen.selected = row;
    }
    let Some(mouse) = mouse else {
        return;
    };
    if mouse.just_pressed(MouseButton::Left) {
        screen.pressed = hit;
    }
    if mouse.just_released(MouseButton::Left)
        && let Some(pressed) = screen.pressed.take()
        && hit == Some(pressed)
        && screen.confirm()
    {
        confirmed.write(ArenaContinue);
    }
}

/// Paints the end screen through the GUI draw path.
fn draw_end_screen(
    screen: Option<Res<ArenaEndScreen>>,
    textures: Option<Res<GuiTextures>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut frames: Option<ResMut<GuiFrames>>,
) {
    let (Some(screen), Some(frames)) = (screen, frames.as_mut()) else {
        return;
    };
    if !screen.open {
        return;
    }
    let Some(menu) = screen.menu() else {
        return;
    };
    let Ok(window) = windows.single() else {
        return;
    };
    let width = window.physical_width() as i32;
    let height = window.physical_height() as i32;
    if width <= 0 || height <= 0 {
        return;
    }
    let empty = GuiTextures::default();
    let textures = textures.as_deref().unwrap_or(&empty);
    frames.push(crate::end_menu_screen::end_menu_frame(
        width,
        height,
        &menu,
        Some(screen.selected),
        textures,
    ));
}

/// Stands the first round's bodies once the level is built and the player
/// is there; in a spectated match (no entrant is the player's) once the
/// loading screen is gone and the player with it (`crate::save_live`'s
/// bench). A match the player enters is stood behind the loading screen,
/// as the original builds a round's bodies within its load: the play
/// view's load then waits for them to be dressed and built, and the
/// match's first tick is the same from run to run (stood after the load,
/// the bodies were dressed on whichever frame the dressing budget and the
/// paint jobs allowed, a varying tick).
#[allow(clippy::too_many_arguments)]
pub(crate) fn stand_first_round(
    scene: Option<ResMut<ArenaScene>>,
    arena: Option<ResMut<ArenaMatch>>,
    career: Option<Res<crate::hub_windows::ArenaCareer>>,
    loading: Option<Res<exa_ui::LoadingStatus>>,
    levels: Query<&exa_world::LevelRoot>,
    building: Query<(), With<exa_physics::StaticGeometry>>,
    players: Query<(), With<crate::player::Player>>,
    clock: Option<Res<crate::effects::EffectClock>>,
    mut commands: MessageWriter<SceneCommand>,
    ((mut fixed, mut freeze), flow): (FreezeQueue<'_>, Option<Res<crate::flow::Flow>>),
    (env, steps): (
        Option<Res<exa_world::environment::SceneEnvRuntime>>,
        Option<ResMut<crate::flow::FlowSteps>>,
    ),
) {
    let Some(mut scene) = scene else {
        return;
    };
    // The player's own entry waits for the player's body; a spectated
    // match waits until the bench has taken it away.
    let player_ready = if scene.player_entry.is_some() {
        !players.is_empty()
    } else {
        players.is_empty()
    };
    let spectated = scene.player_entry.is_none();
    if scene.placed || (spectated && loading.is_some()) || !building.is_empty() || !player_ready {
        return;
    }
    if !levels
        .iter()
        .any(|level| crate::arena_match::arena_index(&level.name).is_some())
    {
        return;
    }
    let now = clock.as_ref().map_or(0, |clock| clock.now_ms);
    // The match start's map load, with the outdoor wind over 0.1, wakes the
    // level's cloths before anyone stands.
    if let Some(wind) = env.as_ref().map(|env| env.env.wind)
        && wakes_cloths(wind)
    {
        wake_cloths(fixed.as_deref_mut(), MATCH_WIND_WAKE);
        info!("arena match: wind {wind:.3} wakes the level's cloths to {MATCH_WIND_WAKE}");
    }
    if let Some(mut arena) = arena {
        // A physician on the company's books saves a fall that would have
        // killed (the fall rule's widened line).
        arena.physician = career.as_ref().is_some_and(|career| {
            career
                .company
                .roster
                .iter()
                .any(|member| member.profession == crate::career::Profession::Physician)
        });
    }
    let slots = layout(scene.template, scene.level, scene.tier, scene.spawn_at);
    if slots.is_empty() {
        warn!(
            "arena match: template {} lays out nobody here",
            scene.template
        );
    }
    let plans = round_plans(&scene, career.as_deref(), &slots, now, MATCH_ACT_GRACE_MS);
    for plan in plans {
        commands.write(SceneCommand::Spawn(Box::new(plan)));
    }
    scene.placed = true;
    // The player's own entrant's start is the same fast two-point shot as
    // the spectated start's: from the spawn spot raised by the near offset,
    // looking at the spot (the spectated one is the spectate session's).
    if !spectated && let Some(mut steps) = steps {
        let spot = Vec3::from_array(scene.spawn_at);
        let viewport = flow.as_ref().map_or(
            crate::flow::Viewport {
                width: 1600,
                height: 900,
            },
            |f| f.viewport,
        );
        let frame = crate::flow::camera::frame_two_points(
            spot,
            spot + crate::flow::camera::NEAR_OFFSET,
            true,
            viewport,
        );
        info!("arena match: the player's start shot frames the spawn spot {spot}");
        steps.steps.push(crate::flow::Step::Camera(frame));
    }
    // The match start sets the staging freeze as it stages the first round:
    // the AI states wait out the transition screen's hold and fade-in, until
    // the screen the hub's Begin opened closes (counted without one, as a
    // command-line match has none).
    if flow.is_some_and(|flow| flow.transition_running()) {
        exa_ai::freeze::stage_held(fixed.as_deref_mut(), freeze.as_deref_mut());
    } else {
        exa_ai::freeze::stage(fixed.as_deref_mut(), freeze.as_deref_mut());
    }
}

/// The wake the match start gives the level's cloths in the wind: the
/// sweep's argument word, 32 (the cloth's wake slot raises its countdown
/// to it).
pub const MATCH_WIND_WAKE: i32 = 0x20;

/// Whether the match start's wind wakes the cloths: the environment's wind
/// widened to double over 0.1 (a not-a-number does not).
#[must_use]
pub fn wakes_cloths(wind: f32) -> bool {
    f64::from(wind) > 0.1
}

/// The match start's sweep over the simulation's first sector: each
/// object's wake slot with `count`, posted to the next fixed step (cloths
/// are simulation data). A cloth raises its countdown to `count`; the base
/// slot of the other objects (clearing two state bits) has no Exarust
/// state, and every cloth is taken as the first sector's [guess: an arena
/// is one sector].
pub fn wake_cloths(fixed: Option<&mut exa_physics::sim_app::FixedCommands>, count: i32) {
    let wake = move |world: &mut World| {
        let mut query = world.query::<&mut exa_world::cloth::ClothInstance>();
        let mut woken = 0;
        for mut instance in query.iter_mut(world) {
            instance.cloth.wake_to(count);
            woken += 1;
        }
        debug!("arena match: {woken} cloths woken to {count}");
    };
    if let Some(fixed) = fixed {
        fixed.post(wake);
    }
}

/// The fixed queue and the staging freeze a staging posts to.
type FreezeQueue<'w> = (
    Option<ResMut<'w, exa_physics::sim_app::FixedCommands>>,
    Option<ResMut<'w, exa_ai::freeze::StagingFreeze>>,
);

/// The flow, its queued steps and the arena's held stagings, which a
/// Continue behind the transition screen uses.
type StagingFlow<'w> = (
    Option<ResMut<'w, crate::flow::Flow>>,
    Option<ResMut<'w, crate::flow::FlowSteps>>,
    Option<ResMut<'w, crate::arena_fade::ArenaStaging>>,
);

/// The end screen's Continue, the next round and the return.
#[allow(clippy::too_many_arguments)]
pub(crate) fn continue_round(
    mut arena: Option<ResMut<ArenaMatch>>,
    mut scene: Option<ResMut<ArenaScene>>,
    mut career: Option<ResMut<crate::hub_windows::ArenaCareer>>,
    mut confirms: MessageReader<ArenaContinue>,
    mut screen: Option<ResMut<ArenaEndScreen>>,
    auto: Res<ArenaAutoContinue>,
    clock: Option<Res<crate::effects::EffectClock>>,
    (fighters, drawn): (Query<MatchBody<'_>>, Query<DrawBody<'_>>),
    mut kits: Query<KitBody<'_>>,
    mut level: Option<ResMut<exa_assets::live_objects::LiveObjects>>,
    mut commands: MessageWriter<SceneCommand>,
    mut exits: MessageWriter<ArenaExit>,
    (mut fixed, mut freeze): FreezeQueue<'_>,
    (mut flow, mut queued, mut staging): StagingFlow<'_>,
) {
    let (Some(arena), Some(scene)) = (arena.as_mut(), scene.as_mut()) else {
        return;
    };
    let now = clock.as_ref().map_or(0, |clock| clock.now_ms);
    // The transition screen is black: the next round held at the Continue is
    // staged now, under the screen, whose close ends the staging freeze.
    let finish = if arena.state == MatchState::Won {
        Finish::Won
    } else {
        Finish::Lost
    };
    let (round, bracket, held) = match staging.as_mut().and_then(|s| s.take_round()) {
        Some(EndAction::NextRound { round, bracket }) => (
            round,
            bracket,
            flow.as_ref().is_some_and(|flow| flow.transition_running()),
        ),
        Some(EndAction::ReturnToHub { tournament }) => {
            // The return's callback at black: the match is settled and the
            // hub's level asked for (the bracket ended at the Continue).
            info!("arena return: settled at the transition screen's black");
            settle_and_exit(
                arena,
                scene,
                career.as_deref_mut(),
                finish,
                tournament,
                &fighters,
                &mut commands,
                &mut exits,
            );
            return;
        }
        Some(EndAction::MainMenu) => return,
        None => {
            if staging.as_ref().is_some_and(|s| s.round_pending()) {
                return;
            }
            if arena.screen.is_none() || arena.state == MatchState::Fighting {
                return;
            }
            // The end screen's row is the only input now; the auto-continue
            // switch runs the same handler without it.
            let pressed = confirms.read().next().is_some();
            if !pressed && !auto.0 {
                return;
            }
            if auto.0
                && let Some(screen) = screen.as_mut()
            {
                screen.close();
            }
            // Continue first resolves the company bodies that fell since the
            // last tick.
            sweep_fallen(arena, &fighters, career.as_deref_mut());
            let action = arena.end_action(scene.tournament);
            match action {
                EndAction::MainMenu => {
                    // The dead manager's leave to the title runs behind the
                    // transition screen with the music fade on; the world's
                    // teardown there takes the bodies.
                    if crate::arena_fade::open_staging(
                        flow.as_deref_mut(),
                        queued.as_deref_mut(),
                        crate::flow::Action::QuitToMenu,
                        true,
                    ) {
                        if let Some(screen) = screen.as_mut() {
                            screen.close();
                        }
                        info!("arena match: the manager is dead; the title waits for the screen");
                    } else {
                        for (entity, ..) in &fighters {
                            commands.write(SceneCommand::Despawn(entity));
                        }
                        exits.write(ArenaExit::MainMenu);
                    }
                    arena.screen = None;
                    return;
                }
                EndAction::ReturnToHub { tournament } => {
                    let sides = scene_sides(&drawn);
                    let mode = arena.state.word() as i32;
                    // The Continue ends the bracket, then opens the
                    // transition screen over the return (the music fade on);
                    // the match is settled at its black.
                    if let Some(staging) = staging.as_mut()
                        && crate::arena_fade::open_staging(
                            flow.as_deref_mut(),
                            queued.as_deref_mut(),
                            crate::flow::Action::ArenaReturn,
                            true,
                        )
                    {
                        end_bracket(scene, career.as_deref_mut(), &sides, mode);
                        if let Some(screen) = screen.as_mut() {
                            screen.close();
                        }
                        staging.hold_round(action);
                        info!("arena match: the return to the hub waits for the transition screen");
                        return;
                    }
                    finish_and_exit(
                        arena,
                        scene,
                        career.as_deref_mut(),
                        finish,
                        tournament,
                        (&fighters, &sides, mode),
                        &mut commands,
                        &mut exits,
                    );
                    return;
                }
                EndAction::NextRound { round, bracket } => {
                    // The Continue raises the round counter, then opens the
                    // transition screen over the next round (the music
                    // fade off); the round is staged at its black.
                    if let Some(staging) = staging.as_mut()
                        && crate::arena_fade::open_staging(
                            flow.as_deref_mut(),
                            queued.as_deref_mut(),
                            crate::flow::Action::ArenaRound,
                            false,
                        )
                    {
                        arena.rounds.round = round;
                        if let Some(screen) = screen.as_mut() {
                            screen.close();
                        }
                        staging.hold_round(action);
                        info!("arena match: round {round} waits for the transition screen");
                        return;
                    }
                    (round, bracket, false)
                }
            }
        }
    };
    // Both stagings first write the company bodies' health words back into
    // their records, as the return does.
    if let Some(career) = career.as_deref_mut() {
        let written = write_back_bodies(career, &fighters);
        debug!("arena match: round {round} wrote {written} health words back");
    }
    if bracket {
        // A tournament's next round raises the bracket's fight: the
        // won group keeps team one, the other groups draw their
        // winner, and the survivors are stood on the ring.
        let sides = scene_sides(&drawn);
        let mode = arena.state.word() as i32;
        if stand_bracket_round(
            scene,
            &fighters,
            &mut kits,
            level.as_deref_mut(),
            (&sides, mode),
            now,
            &mut commands,
        ) {
            info!("arena match: tournament round {round} raised the bracket fight");
        } else {
            warn!("arena match: tournament round {round} has no bracket");
        }
    } else {
        let entered = scene.player_entry.is_some();
        let fighter_dead = fighters.iter().any(|(_, _, condition, body, player)| {
            (if entered { player } else { body.company }) && *condition == Condition::DEAD
        });
        let opponent_present = fighters
            .iter()
            .any(|(_, _, condition, body, _)| !body.company && *condition != Condition::DEAD);
        let fight = regenerate_round(
            scene.level,
            scene.tier,
            arena.rounds.losses,
            fighter_dead,
            opponent_present,
        );
        stand_next_round(
            &fight,
            scene,
            career.as_deref(),
            &fighters,
            &mut kits,
            level.as_deref_mut(),
            now,
            &mut commands,
        );
    }
    // Both the next round's staging and the bracket's set the
    // staging freeze: held by the transition screen it runs under,
    // else counted.
    if held {
        exa_ai::freeze::stage_held(fixed.as_deref_mut(), freeze.as_deref_mut());
    } else {
        exa_ai::freeze::stage(fixed.as_deref_mut(), freeze.as_deref_mut());
    }
    arena.rounds.round = round;
    arena.state = MatchState::Fighting;
    arena.screen = None;
    arena.started_ms = None;
    arena.resolved.clear();
}

/// The company bodies' health words go back into their records
/// ([`crate::arena_session::write_back_health`]): the original's write-back
/// runs at the start of the next round, the bracket round and the return.
fn write_back_bodies(
    career: &mut crate::hub_windows::ArenaCareer,
    fighters: &Query<MatchBody<'_>>,
) -> usize {
    let returned: Vec<ReturnFighter> = fighters
        .iter()
        .filter(|(_, _, _, body, player)| body.company || *player)
        .map(|(_, fighter, _, body, player)| ReturnFighter {
            company: true,
            id: if player { 0 } else { body.id },
            health: fighter.health.red,
            max: fighter.health.max,
            word: Some(crate::arena_match::health_word(
                fighter.health.red,
                fighter.health.max,
            )),
        })
        .collect();
    write_back_health(career, &returned)
}

/// The next round's bodies: the winner stays and the reserve or a fresh
/// opponent comes, after the round's yield. Whoever stays is re-dressed
/// first: the continuing fighter, or the opponent who won when the fighter
/// is replaced.
#[allow(clippy::too_many_arguments)]
fn stand_next_round(
    fight: &RoundFight,
    scene: &mut ArenaScene,
    career: Option<&crate::hub_windows::ArenaCareer>,
    fighters: &Query<MatchBody<'_>>,
    kits: &mut Query<KitBody<'_>>,
    mut level: Option<&mut exa_assets::live_objects::LiveObjects>,
    now_ms: u32,
    commands: &mut MessageWriter<SceneCommand>,
) {
    let opponent = opponent_place(scene.spawn_at);
    let ours = our_place(scene.spawn_at);
    // The scene work waits the round's yield before it touches the bodies;
    // the bodies' own AI clock starts at the action grace after that.
    let now = now_ms.wrapping_add(fight.yield_ms);
    let mut kept = Vec::new();
    for (entity, _, condition, body, player) in fighters {
        // A company fighter that still stands fights on (`FreshOpponent`);
        // a dead one is cleared and the reserve takes its place. The
        // opponent who won keeps its place, which is the round's opponent
        // place already; every other opponent is cleared.
        let clear = if body.company {
            *condition == Condition::DEAD
        } else {
            !fight.keeps_opponent || *condition == Condition::DEAD
        };
        if clear {
            commands.write(SceneCommand::Despawn(entity));
            continue;
        }
        // The fighter who goes on is re-dressed. When that fighter is dead
        // the opponent who won is re-dressed instead, and the reserve comes in.
        let dress = if fight.keeps_opponent {
            !body.company
        } else {
            body.company || player
        };
        if dress {
            redress(kits, level.as_deref_mut(), entity);
            kept.push((entity, player, body.company || player));
        }
    }
    if !fight.keeps_opponent {
        let players: Vec<Entity> = kits
            .iter()
            .filter(|(.., player_body)| *player_body)
            .map(|(entity, ..)| entity)
            .collect();
        for entity in players {
            // The player's body carries no scene marker, so the loop above
            // never sees it. It is the continuing fighter when the round was
            // not lost.
            if fighters.iter().all(|(body, ..)| body != entity) {
                redress(kits, level.as_deref_mut(), entity);
                kept.push((entity, true, true));
            }
        }
    }
    // Whoever stays is reset for the next round: the kept opponent, then
    // the continuing fighter, as the original resets them in that order
    // (the roll steps once per non-player). The kept opponent stands at the
    // opponent's place; the continuing fighter at the company's, facing
    // the opponent (the original's next-round template, which the match
    // set-up points at the company's spot).
    kept.sort_by_key(|&(_, player, _)| player);
    for (body, player, ours_side) in kept {
        let reset = crate::match_layout::reset_kept(player, now, &mut scene.roll);
        let place = if ours_side { ours } else { opponent };
        commands.write(SceneCommand::Reset { body, reset, place });
    }
    if let Some(plan) = &fight.opponent {
        let body = opponent_plan(*plan, &opponent, HOSTILE, now, MATCH_ACT_GRACE_MS);
        commands.write(SceneCommand::Spawn(Box::new(body)));
    }
    if let NextRound::Reserve(index) = fight.company {
        let (member, id) = scene.entrant(career, index);
        let copy = scene.match_copy(career, index);
        let body = company_plan(
            member,
            copy,
            &ours,
            MANAGER_ROLE,
            id,
            now,
            MATCH_ACT_GRACE_MS,
        );
        commands.write(SceneCommand::Spawn(Box::new(body)));
    }
}

/// The bodies of a tournament's next round: resolve the bracket's groups,
/// then keep the player where the bracket puts it and stand every other
/// surviving side. Returns false when the match carries no bracket.
#[allow(clippy::too_many_arguments)]
fn stand_bracket_round(
    scene: &mut ArenaScene,
    fighters: &Query<MatchBody<'_>>,
    kits: &mut Query<KitBody<'_>>,
    mut level: Option<&mut exa_assets::live_objects::LiveObjects>,
    (sides, mode): (&[crate::career::SideFighter], i32),
    now_ms: u32,
    commands: &mut MessageWriter<SceneCommand>,
) -> bool {
    let Some(bracket) = scene.bracket.as_mut() else {
        return false;
    };
    // The watched fight's winner is the scene draw's: the standing bodies'
    // teams weighted by their health (a won fight leaves the player's team
    // and any downed rival). Its group keeps it; every other group's winner
    // is drawn from the same roll.
    let mut roll = scene.roll;
    let winner =
        crate::career::draw_unwatched(sides, crate::arena_match::PLAYER_TEAM, mode, &mut roll);
    info!("arena match: the scene draw names team {winner:?} the fight's winner");
    let survivors = bracket_next_fight(bracket, winner.unwrap_or(0), &mut roll);
    scene.roll = roll;
    // The forced outfit table is indexed by team id: the company the
    // week's field drew for each rival team.
    let outfits = crate::arena_spawn::rival_outfits(&scene.rivals);
    // The next fight is the first `sides` surviving teams in slot order (the
    // player's group leads), as the original's team walk stops at the
    // event's side count; the raise derives the role flag from how many
    // teams it formed (hostile below three, else a free-for-all).
    let fight = FreshMatch {
        slots: &survivors,
        per_team: bracket.team_size,
        teams: bracket.sides,
        spawn_at: scene.spawn_at,
        player_team: 1,
        company_entries: usize::try_from(bracket.team_size).unwrap_or(1),
        player_entry: scene.player_entry,
        tier: scene.tier,
        outfits: &outfits,
        event_weapon: i32::from(scene.weapon_variants.is_some()),
        now_ms: now_ms.wrapping_add(crate::arena_session::NEXT_ROUND_YIELD_MS),
    };
    let raised = raise_fresh_match(&fight, |_| true);
    for report in &raised.reports {
        warn!("arena match: {}: {}", report.title, report.message);
    }
    // The kept list: the company's bodies (the player's included) that are
    // not dead, in their entry order; a dead one leaves the list. Every
    // other body goes (the original's between-round reload drops every body
    // it does not take back).
    let entry_of = |body: &SceneFighter| {
        scene
            .entrants
            .iter()
            .position(|&slot| slot == body.id as usize)
            .unwrap_or(body.id as usize)
    };
    let player_entry = scene.player_entry.unwrap_or(0);
    let mut kept: Vec<(usize, Entity, bool)> = Vec::new();
    for (entity, _, condition, body, player) in fighters {
        let company = player || body.company;
        if company && *condition != Condition::DEAD {
            let entry = if player { player_entry } else { entry_of(body) };
            kept.push((entry, entity, player));
        } else if player {
            warn!("arena match: the player is dead at a bracket round; its body is left lying");
        } else {
            commands.write(SceneCommand::Despawn(entity));
        }
    }
    // The player's body carries no scene marker when it is the debug
    // player; it stays at its entry.
    if !kept.iter().any(|&(.., player)| player) {
        let players: Vec<Entity> = kits
            .iter()
            .filter(|(.., player_body)| *player_body)
            .map(|(entity, ..)| entity)
            .collect();
        for entity in players {
            if fighters.iter().all(|(body, ..)| body != entity) {
                kept.push((player_entry, entity, true));
            }
        }
    }
    kept.sort_by_key(|&(entry, ..)| entry);
    let now = now_ms.wrapping_add(crate::arena_session::NEXT_ROUND_YIELD_MS);
    let kept_places = crate::match_layout::kept_side_places(
        &survivors,
        bracket.team_size,
        bracket.sides,
        scene.spawn_at,
        fight.player_team,
        kept.len(),
    );
    let mut kept_stood = false;
    let mut teams_seen: Vec<i32> = Vec::new();
    for fighter in raised.fighters {
        match fighter.body {
            // The company's side: the kept bodies are re-dressed and stood on
            // the side's places in list order, once, where the side comes in
            // the team walk. The original takes them back after the reload
            // (the kit equip, the level attach, the kept body's reset, whose
            // non-player clocks step the roll).
            RaisedBody::Player | RaisedBody::Company { .. } => {
                if kept_stood {
                    continue;
                }
                kept_stood = true;
                let Some(places) = kept_places.as_ref() else {
                    continue;
                };
                for (&(_, entity, player), place) in kept.iter().zip(places) {
                    redress(kits, level.as_deref_mut(), entity);
                    let reset = crate::match_layout::reset_kept(player, now, &mut scene.roll);
                    commands.write(SceneCommand::Reset {
                        body: entity,
                        reset,
                        place: *place,
                    });
                }
            }
            RaisedBody::Opponent {
                plan,
                role,
                act_at_ms,
                ..
            } => {
                if !teams_seen.contains(&fighter.team) {
                    teams_seen.push(fighter.team);
                }
                let place = fighter.place;
                let mut body = match plan {
                    crate::match_layout::OpponentPlan::Generic(plan) => {
                        opponent_plan(plan, &place, role, 0, act_at_ms)
                    }
                    // The event row's variant is drawn while the record is
                    // generated, from the reseeded sequence.
                    crate::match_layout::OpponentPlan::Hire(plan) => {
                        match scene.weapon_variants.filter(|_| plan.draw_weapon) {
                            Some(row) => hire_row_plan(plan, &place, role, row, 0, act_at_ms),
                            None => hire_opponent_plan(plan, &place, role, None, 0, act_at_ms),
                        }
                    }
                };
                body.team = u32::try_from(fighter.team).unwrap_or(0);
                commands.write(SceneCommand::Spawn(Box::new(body)));
            }
        }
    }
    info!(
        "arena match: the bracket round keeps {} company bodies against {} sides",
        kept.len(),
        teams_seen.len()
    );
    true
}

/// Settle `finish`, take the round's bodies away and ask for the hub's
/// level; the settled career stays in its resource. The pause page's
/// forfeit leaves this way, and the end screen's Continue without the flow
/// (with it, [`end_bracket`] runs at the Continue and [`settle_and_exit`] at
/// the transition screen's black).
#[allow(clippy::too_many_arguments)]
fn finish_and_exit(
    arena: &mut ArenaMatch,
    scene: &mut ArenaScene,
    mut career: Option<&mut crate::hub_windows::ArenaCareer>,
    finish: Finish,
    tournament: bool,
    (fighters, sides, mode): (&Query<MatchBody<'_>>, &[crate::career::SideFighter], i32),
    commands: &mut MessageWriter<SceneCommand>,
    exits: &mut MessageWriter<ArenaExit>,
) {
    end_bracket(scene, career.as_deref_mut(), sides, mode);
    settle_and_exit(
        arena, scene, career, finish, tournament, fighters, commands, exits,
    );
}

/// A tournament's end (or forfeit) first draws the scene's winner, clears
/// every bracket and names the first open event's winner, before the return
/// to the hub settles. The end menu's Continue does this before it opens the
/// transition screen over the return.
fn end_bracket(
    scene: &mut ArenaScene,
    career: Option<&mut crate::hub_windows::ArenaCareer>,
    sides: &[crate::career::SideFighter],
    mode: i32,
) {
    if let (Some(bracket), Some(career)) = (scene.bracket.as_mut(), career) {
        let mut roll = scene.roll;
        let winner =
            crate::career::draw_unwatched(sides, crate::arena_match::PLAYER_TEAM, mode, &mut roll);
        crate::arena_session::finish_bracket(bracket, winner, &mut career.entries, &mut roll);
        scene.roll = roll;
        info!(
            "arena match: the bracket ends with team {winner:?} drawn; events {:?}",
            career.entries.map(|row| row.team_id)
        );
    }
}

/// The return to the hub proper (the transition screen's callback when the
/// flow runs): settle `finish`, take the round's bodies away and ask for the
/// hub's level.
#[allow(clippy::too_many_arguments)]
fn settle_and_exit(
    arena: &mut ArenaMatch,
    scene: &mut ArenaScene,
    mut career: Option<&mut crate::hub_windows::ArenaCareer>,
    finish: Finish,
    tournament: bool,
    fighters: &Query<MatchBody<'_>>,
    commands: &mut MessageWriter<SceneCommand>,
    exits: &mut MessageWriter<ArenaExit>,
) {
    settle_match(arena, scene, &mut career, finish, tournament, fighters);
    for (entity, ..) in fighters {
        commands.write(SceneCommand::Despawn(entity));
    }
    // The career resource and the settle survive into the hub; the host's
    // arena exit sets the return placement and changes the level in process.
    exits.write(ArenaExit::Hub);
    arena.screen = None;
}

/// The arena pause page's FORFEIT: a dead manager is ignored, anything else
/// settles the match as forfeited and returns to the hub. The flow's pause
/// page sends [`crate::flow::ForfeitMatch`] when its third row runs.
///
/// Two instances read the requests every frame: the frame's (`AT_QUIT`
/// false) and the quit's (`AT_QUIT` true, in `Last`, after the quit's
/// gate and before its saves); only the one for the app's state acts.
#[allow(clippy::too_many_arguments)]
pub(crate) fn forfeit_match<const AT_QUIT: bool>(
    mut forfeits: Option<MessageReader<crate::flow::ForfeitMatch>>,
    mut arena: Option<ResMut<ArenaMatch>>,
    mut scene: Option<ResMut<ArenaScene>>,
    mut career: Option<ResMut<crate::hub_windows::ArenaCareer>>,
    (fighters, drawn): (Query<MatchBody<'_>>, Query<DrawBody<'_>>),
    mut commands: MessageWriter<SceneCommand>,
    mut exits: MessageWriter<ArenaExit>,
    (quit, mut saves): (
        Option<Res<crate::save_live::QuitAutosave>>,
        Option<MessageWriter<crate::save_live::SaveStep>>,
    ),
) {
    let Some(forfeits) = forfeits.as_mut() else {
        return;
    };
    let quitting = quit.as_ref().is_some_and(|quit| quit.0);
    if forfeits.read().count() == 0 || quitting != AT_QUIT {
        return;
    }
    let (Some(arena), Some(scene)) = (arena.as_mut(), scene.as_mut()) else {
        return;
    };
    if arena.state != MatchState::Fighting {
        return;
    }
    // The forfeit first resolves the company bodies that fell since the
    // last tick.
    sweep_fallen(arena, &fighters, career.as_deref_mut());
    match arena.forfeit(scene.tournament) {
        crate::arena_session::ForfeitResult::Ignored => {
            info!("arena forfeit: the manager is already down, ignored");
        }
        crate::arena_session::ForfeitResult::Return { tournament } => {
            info!("arena forfeit: returning to the hub");
            // The forfeit's session word is 0x10: the scene draw leaves the
            // player's team out.
            let sides = scene_sides(&drawn);
            finish_and_exit(
                arena,
                scene,
                career.as_deref_mut(),
                Finish::Forfeited,
                tournament,
                (&fighters, &sides, crate::career::FORFEIT_DRAW),
                &mut commands,
                &mut exits,
            );
            // The quit's forfeit saves the settled career's player data
            // (the pause page's own FORFEIT row saves nothing).
            if quitting && let Some(saves) = saves.as_mut() {
                saves.write(crate::save_live::SaveStep::ForfeitSave);
            }
        }
    }
}

/// The sweep the end screen's Continue and the forfeit run first: every
/// company body (the player's included) whose health is under 0.01 and
/// whose fall the once-a-second tick has not resolved yet is resolved now,
/// as a death or a save, and marked resolved
/// ([`crate::arena_session::resolve_dead_bodies`]).
fn sweep_fallen(
    arena: &mut ArenaMatch,
    fighters: &Query<MatchBody<'_>>,
    career: Option<&mut crate::hub_windows::ArenaCareer>,
) {
    let mut entities = Vec::new();
    let mut scene = Vec::new();
    for (entity, fighter, condition, body, player) in fighters {
        let company = player || body.company;
        entities.push(entity);
        scene.push(crate::arena_match::SceneFighter {
            team: if company {
                crate::arena_match::PLAYER_TEAM
            } else {
                body.team as i32
            },
            dead: *condition == Condition::DEAD,
            company,
            resolved: arena.resolved.contains(&entity),
            health: fighter.health.red,
            max: fighter.health.max,
            id: u8::try_from(body.id).unwrap_or(u8::MAX),
            // A hired fighter's death record names him by his record.
            name: if player {
                None
            } else {
                crate::arena_match::roster_name(
                    career.as_deref(),
                    u8::try_from(body.id).unwrap_or(u8::MAX),
                )
            },
            manager: player,
        });
    }
    let died = &mut arena.died;
    let mut career_log = career.map(|career| &mut career.log);
    let resolved = crate::arena_session::resolve_dead_bodies(
        &mut scene,
        arena.physician,
        &mut arena.log,
        &mut arena.deaths,
        |id, log| {
            info!("arena match: company fighter {id} died");
            crate::arena_match::book_death(died, id, log, career_log.as_deref_mut());
        },
    );
    for index in resolved {
        info!(
            "arena match: the sweep resolved company fighter {}",
            scene[index].id
        );
        arena.resolved.push(entities[index]);
    }
}

/// The match's end: settle the result into the career, as the return to the
/// hub does. The round's fallen company bodies were already resolved by the
/// match tick, so this pass only books the result.
fn settle_match(
    arena: &mut ArenaMatch,
    scene: &mut ArenaScene,
    career: &mut Option<&mut crate::hub_windows::ArenaCareer>,
    finish: Finish,
    tournament: bool,
    fighters: &Query<MatchBody<'_>>,
) {
    let Some(career) = career.as_deref_mut() else {
        warn!("arena return: no career to settle");
        return;
    };
    career.log.records.append(&mut arena.log.records);
    let mut returned: Vec<ReturnFighter> = fighters
        .iter()
        .filter(|(_, _, _, body, player)| body.company || *player)
        .map(|(_, fighter, _, body, player)| ReturnFighter {
            company: true,
            // The player's body is the manager, index 0, whatever id tag
            // its plan was spawned with.
            id: if player { 0 } else { body.id },
            health: fighter.health.red,
            max: fighter.health.max,
            word: None,
        })
        .collect();
    let day = career.company.day;
    let entered = any_entry(&career.entries);
    let out = settle_return(
        finish,
        arena.template,
        tournament,
        scene.prize,
        &mut returned,
        &mut scene.records,
        &mut career.log,
        day,
        entered,
        career.matches_today,
    );
    // The return writes each participant's health word back into its record
    // before the hub opens; the manager's record is separate from the
    // roster's.
    let written = write_back_health(career, &returned);
    debug_assert!(written <= out.healths_written);
    // The company fighters who died are off the books by the time the hub
    // opens (the original removes each as his fall is resolved; the match
    // here still finds its records by id until the return).
    for member in crate::arena_match::remove_dead(career, &arena.died) {
        info!("arena return: {} died and leaves the company", member.name);
    }
    arena.died.clear();
    career.matches_today = out.matches_today;
    career.company.copper = career.company.copper.saturating_add(out.paid);
    career.company.earned = career.company.earned.saturating_add(out.paid);
    // The return books the counts it changes onto the career's current
    // records: wins, losses, forfeits and Beast wins, plus the company
    // deaths the match's falls counted (the original raises that record as
    // each fall is resolved). The other counts (tournaments, seasons, hires,
    // dismissals and the lifetime coin totals) are left as the career holds
    // them.
    let mut records = career.company.records();
    records.matches_won = scene.records.matches_won;
    records.matches_lost = scene.records.matches_lost;
    records.matches_forfeited = scene.records.matches_forfeited;
    records.beasts_defeated = scene.records.beasts_defeated;
    records.deaths = records.deaths.wrapping_add(arena.deaths);
    career.company.book_records(&records);
    if out.day.drop_unfit {
        // An entrant's id is its participant id: 0 the manager, `i`
        // roster member `i - 1`.
        let healths: Vec<u16> = std::iter::once(career.player.health)
            .chain(career.company.roster.iter().map(|member| member.health))
            .collect();
        arena::drop_unfit(&mut career.entries, |id| {
            healths.get(id as usize).copied().unwrap_or(0)
        });
    }
    if out.day.assign_round_ids {
        let mut rng = crate::item_container::Rng(career.roll);
        arena::assign_round_ids(&mut career.entries, &mut rng);
        career.roll = rng.0;
    }
    info!(
        "arena return: paid {}, {} healths written, {} matches today",
        out.paid, out.healths_written, out.matches_today
    );
}

/// The match's setup from the hub's launch: its template, level, tier and
/// the player's own entry, else the command line's. The career the launch
/// carried comes with it.
#[must_use]
pub fn setup_from(
    launch: Option<MatchSetup>,
    options_template: i32,
    level: i32,
    tier: i32,
) -> MatchSetup {
    launch.unwrap_or(MatchSetup {
        template: options_template,
        level,
        tier,
        tournament: false,
        // A match started from the command line has no hub entry list: the
        // debug player is the company's entry zero.
        player: Some(0),
        prize: 0,
        weapon_variants: None,
    })
}

/// The brain a company body takes.
pub const COMPANY_BRAIN: BodyBrain = BodyBrain::Npc;

/// The transition a match's end opens, as the live scene reads it.
#[must_use]
pub fn screen_word(screen: Option<Screen>) -> Option<u32> {
    screen.map(Screen::word)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::arena_match::ArenaMatch;
    use crate::match_layout::{opponent_place, our_place};
    use crate::scene_bodies::{BodyLook, SceneFighter};

    fn scene() -> ArenaScene {
        ArenaScene::new(
            1,
            [900.0, 0.0, 100.0],
            3,
            2,
            false,
            Some(0),
            Vec::new(),
            12,
            None,
        )
    }

    fn arena_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_message::<SceneCommand>()
            .add_message::<ArenaExit>()
            .add_message::<ArenaContinue>()
            .init_resource::<ArenaAutoContinue>()
            .init_resource::<ArenaEndScreen>()
            .add_systems(
                Update,
                (open_end_screen, update_end_screen, continue_round)
                    .chain()
                    .after(crate::flow::FlowSystems::ApplySteps),
            );
        app.init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<ButtonInput<MouseButton>>();
        app.insert_resource(crate::effects::EffectClock { now_ms: 1_000 });
        app
    }

    fn press_continue(app: &mut App) {
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Enter);
    }

    fn body(team: u32, company: bool, condition: Condition) -> impl Bundle {
        (
            Fighter::default(),
            condition,
            SceneFighter {
                team,
                company,
                id: 1,
                damage: 100.0,
                condition_health: 100.0,
                act_at_ms: 0,
                behaviour: None,
            },
        )
    }

    #[test]
    fn the_first_round_is_stood_once_the_level_is_ready() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_message::<SceneCommand>()
            .init_resource::<exa_physics::sim_app::FixedCommands>()
            .init_resource::<exa_ai::freeze::StagingFreeze>()
            .init_resource::<crate::flow::FlowSteps>()
            .add_systems(Update, stand_first_round);
        app.insert_resource(crate::effects::EffectClock { now_ms: 1_000 });
        app.insert_resource(scene());
        app.world_mut().spawn(exa_world::LevelRoot {
            name: "arena1.rfc".into(),
        });
        app.world_mut()
            .spawn((crate::player::Player, Transform::default()));
        app.update();
        let spawned: Vec<SceneCommand> = app
            .world_mut()
            .resource_mut::<Messages<SceneCommand>>()
            .drain()
            .collect();
        assert_eq!(spawned.len(), 1);
        assert!(app.world().resource::<ArenaScene>().placed);
        // The player's own entrant's start shot: the fast two-point shot
        // from the spawn spot raised by the near offset.
        let spot = Vec3::from_array(app.world().resource::<ArenaScene>().spawn_at);
        let view = crate::flow::Viewport {
            width: 1600,
            height: 900,
        };
        assert_eq!(
            app.world().resource::<crate::flow::FlowSteps>().steps,
            vec![crate::flow::Step::Camera(
                crate::flow::camera::frame_two_points(
                    spot,
                    spot + crate::flow::camera::NEAR_OFFSET,
                    true,
                    view,
                )
            )]
        );
        // The match start's staging freeze is posted for the next fixed
        // step, which sets it.
        exa_physics::sim_app::run_fixed_commands(app.world_mut());
        assert!(
            app.world()
                .resource::<exa_ai::freeze::StagingFreeze>()
                .frozen()
        );
        // The round is stood once.
        app.update();
        assert!(
            app.world_mut()
                .resource_mut::<Messages<SceneCommand>>()
                .drain()
                .next()
                .is_none()
        );
    }

    /// The match start in the wind wakes every cloth of the level to 32 at
    /// the next fixed step; a calmer wind, or a not-a-number, wakes none.
    /// The single 0.1 widened to double is just over the double 0.1, so it
    /// wakes them.
    #[test]
    fn a_windy_match_start_wakes_the_levels_cloths() {
        assert!(wakes_cloths(0.1) && !wakes_cloths(0.099_999) && !wakes_cloths(f32::NAN));
        let run = |wind: f32| {
            let mut app = App::new();
            app.add_plugins(MinimalPlugins)
                .add_message::<SceneCommand>()
                .init_resource::<exa_physics::sim_app::FixedCommands>()
                .init_resource::<exa_ai::freeze::StagingFreeze>()
                .add_systems(Update, stand_first_round);
            app.insert_resource(crate::effects::EffectClock { now_ms: 1_000 });
            app.insert_resource(scene());
            let mut env = exa_world::scene_env::SceneEnvironment::new(5);
            env.wind = wind;
            app.insert_resource(exa_world::environment::SceneEnvRuntime::new(env, 1));
            app.world_mut().spawn(exa_world::LevelRoot {
                name: "arena1.rfc".into(),
            });
            app.world_mut()
                .spawn((crate::player::Player, Transform::default()));
            let mut cloth = exa_physics::cloth::Cloth::from_mesh(
                &[Vec3::ZERO, Vec3::X, Vec3::Y],
                &[[0, 1, 2]],
                &[0],
            );
            cloth.sleep = 3;
            let cloth = app
                .world_mut()
                .spawn(exa_world::cloth::ClothInstance::new(
                    cloth,
                    Handle::default(),
                    vec![0, 1, 2],
                ))
                .id();
            app.update();
            exa_physics::sim_app::run_fixed_commands(app.world_mut());
            app.world()
                .get::<exa_world::cloth::ClothInstance>(cloth)
                .unwrap()
                .cloth
                .sleep
        };
        assert_eq!(run(0.5), MATCH_WIND_WAKE);
        assert_eq!(run(0.05), 3);
    }

    #[test]
    fn a_duel_lays_out_the_player_and_one_opponent() {
        let spawn = [900.0, 0.0, 100.0];
        let slots = layout(1, 3, 2, spawn);
        let plans = round_plans(&scene(), None, &slots, 1_000, 3_000);
        assert_eq!(plans.len(), 1);
        assert_eq!(plans[0].team, 2);
        assert!(matches!(plans[0].brain, BodyBrain::Npc));
        assert_eq!(
            plans[0].at,
            Vec3::from_array(opponent_place(spawn).position) / 100.0
        );
        assert_eq!(plans[0].act_at_ms, 4_000);
        // Without the manager entered, the company's first entry is spawned.
        let mut scene = scene();
        scene.player_entry = None;
        let plans = round_plans(&scene, None, &slots, 1_000, 3_000);
        assert_eq!(plans.len(), 2);
        assert_eq!(plans[1].team, crate::arena_spawn::PLAYER_TEAM);
        assert_eq!(
            plans[1].at,
            Vec3::from_array(our_place(spawn).position) / 100.0
        );
    }

    /// A company entry is the lineup's pool slot: its member and its id
    /// come from the participant list, the manager from slot 0.
    #[test]
    fn a_company_entry_is_its_lineup_slot() {
        let mut career = crate::hub_windows::ArenaCareer::default();
        career.player.name = "Boss".into();
        career.player.health = 60_000;
        for (name, health) in [("Ash", 40_000), ("Bram", 50_000)] {
            career.company.roster.push(crate::career::Member {
                name: name.into(),
                health,
                ..career.player.clone()
            });
        }
        let scene = scene().with_entrants(vec![2, 0]);
        assert_eq!(scene.entrant(Some(&career), 0), (Some(("Bram", 50_000)), 2));
        assert_eq!(scene.entrant(Some(&career), 1), (Some(("Boss", 60_000)), 0));
        // Past the list (a command-line match) the entry is its own slot.
        assert_eq!(scene.entrant(Some(&career), 2), (Some(("Bram", 50_000)), 2));
        assert_eq!(scene.entrant(None, 0), (None, 2));
    }

    #[test]
    fn a_hired_entrant_wears_its_match_copy_and_the_manager_the_template() {
        use exa_formats::characters::ItemRef;
        let mut career = crate::hub_windows::ArenaCareer::default();
        let mut member = career.player.clone();
        member.name = "Ash".into();
        // Four levels: rank 2, under the scene's tier 2.
        member.skill_xp = [20_000, 0, 0, 0, 0, 0];
        member.loadouts[2].hands[0] = ItemRef::new(77, 0);
        member.loadouts[2].worn = vec![ItemRef::new(55, 0)];
        career.company.roster.push(member);
        let scene = scene().with_entrants(vec![0, 1]);
        let slots = vec![
            Slot {
                who: SlotFighter::Ours(1),
                place: our_place([0.0; 3]),
            },
            Slot {
                who: SlotFighter::Ours(0),
                place: our_place([0.0; 3]),
            },
        ];
        let mut scene = scene;
        scene.player_entry = None;
        let plans = round_plans(&scene, Some(&career), &slots, 0, 3_000);
        let BodyLook::Company(company) = &plans[0].look else {
            panic!("the hired entrant has no match copy: {:?}", plans[0].look);
        };
        assert_eq!(company.copy.kit.hands[0].id, 77);
        assert_eq!(company.copy.kit.worn, vec![ItemRef::new(55, 0)]);
        assert_eq!(company.copy.combat_skill, 50.0);
        // The manager's kit is not the career's to copy.
        assert!(matches!(plans[1].look, BodyLook::Actor(_)));
    }

    #[test]
    fn a_tournament_side_takes_the_drawn_weapon_and_its_team() {
        let spawn = [0.0; 3];
        let mut scene = scene().with_rivals(RIVALS);
        scene.tournament = true;
        scene.weapons = vec![0x1234];
        let slots = vec![Slot {
            who: SlotFighter::Opponent {
                level: 3,
                tier: 2,
                team: 6,
                flags: HOSTILE,
            },
            place: opponent_place(spawn),
        }];
        let plans = round_plans(&scene, None, &slots, 0, 3_000);
        assert_eq!(plans[0].weapon, Some(0x1234));
        assert_eq!(plans[0].team, 6);
        assert_eq!(plans[0].act_at_ms, 3_000);
        // Team 6's rival company (the field's fifth, 13) is forced into
        // the hire profile, and the event's weapon sets the armed flag.
        let BodyLook::Generated(fighter) = plans[0].look else {
            panic!("a tournament side takes a generated record");
        };
        let crate::arena_records::OutfitSource::Profile(profile) = fighter.outfit else {
            panic!("a hire side takes the profile pass");
        };
        assert_eq!(profile.flags(), 0x0f);
        assert_eq!(profile.level(), 2);
        assert_eq!(&profile.0[5..8], &[97, 20, 0]);
    }

    const RIVALS: [i32; 16] = [15, 17, 4, 1, 13, 9, 5, 14, 8, 6, 11, 2, 18, 3, 19, 10];

    #[test]
    fn a_launched_tournament_side_draws_from_the_event_row() {
        let mut scene = scene()
            .with_rivals(RIVALS)
            .with_weapon_variants(Some([7, 8, 0]));
        scene.tournament = true;
        let slots = vec![Slot {
            who: SlotFighter::Opponent {
                level: 3,
                tier: 2,
                team: 2,
                flags: HOSTILE,
            },
            place: opponent_place([0.0; 3]),
        }];
        let plans = round_plans(&scene, None, &slots, 0, 3_000);
        // No weapon is chosen before the record: the generation draws it.
        assert_eq!(plans[0].weapon, None);
        let BodyLook::Generated(fighter) = plans[0].look else {
            panic!("a tournament side takes a generated record");
        };
        assert_eq!(fighter.weapon_row, Some([7, 8, 0]));
        let crate::arena_records::OutfitSource::Profile(profile) = fighter.outfit else {
            panic!("a hire side takes the profile pass");
        };
        // Team 2 wears the field's first company, 15.
        assert_eq!(&profile.0[5..8], &crate::arena_spawn::COMPANY_OUTFITS[14]);
    }

    #[test]
    fn the_beast_round_asks_for_the_beast_record() {
        let slots = vec![Slot {
            who: SlotFighter::Beast { level: 4, tier: 4 },
            place: opponent_place([0.0; 3]),
        }];
        let plans = round_plans(&scene(), None, &slots, 0, 3_000);
        assert_eq!(plans[0].look, BodyLook::Beast(4));
        assert_eq!(plans[0].id, 4);
    }

    /// The end-session menu's arena CONTINUE runs the arena's continue: the
    /// next round starts as the arena's own end screen's Continue starts it.
    #[test]
    fn the_end_session_menus_continue_runs_the_arena_continue() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_message::<SceneCommand>()
            .add_message::<ArenaExit>()
            .init_resource::<ArenaAutoContinue>()
            .init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<ButtonInput<MouseButton>>()
            .init_resource::<exa_ui::gui_draw::GuiFrames>()
            .init_resource::<crate::flow::FlowSteps>()
            .insert_resource({
                let mut flow = crate::flow::Flow::default();
                flow.mode = crate::flow::Mode::Arena;
                flow
            })
            .insert_resource(crate::effects::EffectClock { now_ms: 1_000 })
            .add_plugins(crate::end_menu_screen::EndMenuScreenPlugin)
            .add_systems(Update, continue_round);
        let mut arena = ArenaMatch::begin(7);
        arena.rounds.round = 2;
        arena.rounds.losses = 1;
        arena.state = MatchState::NextFight;
        arena.screen = Some(Screen::NextRound);
        app.insert_resource(arena);
        let mut scene = scene();
        scene.player_entry = None;
        app.insert_resource(scene);
        app.world_mut().spawn(body(2, false, Condition::NORMAL));
        {
            let mut menu = app
                .world_mut()
                .resource_mut::<crate::end_menu_screen::EndMenuScreen>();
            menu.show(crate::flow::end_menu(
                crate::flow::MENU_ARENA,
                crate::flow::Mode::Arena,
            ));
            menu.selected = Some(0);
        }
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Enter);
        app.update();
        app.update();
        // The menu closed without leaving to the title.
        assert!(
            !app.world()
                .resource::<crate::end_menu_screen::EndMenuScreen>()
                .open
        );
        assert!(
            app.world()
                .resource::<crate::flow::FlowSteps>()
                .steps
                .is_empty()
        );
        let arena = app.world().resource::<ArenaMatch>();
        assert_eq!(arena.state, MatchState::Fighting);
        assert_eq!(arena.rounds.round, 3);
        assert_eq!(arena.screen, None);
    }

    #[test]
    fn the_continue_starts_the_next_round_with_the_winner_still_up() {
        let mut app = arena_app();
        let mut arena = ArenaMatch::begin(7);
        arena.rounds.round = 2;
        arena.rounds.losses = 1;
        arena.state = MatchState::NextFight;
        arena.screen = Some(Screen::NextRound);
        app.insert_resource(arena);
        let mut scene = scene();
        // The player's own entry is not spawned by the generator: the
        // company bodies in the query are the fell fighters.
        scene.player_entry = None;
        app.insert_resource(scene);
        let opponent = app
            .world_mut()
            .spawn(body(2, false, Condition::NORMAL))
            .id();
        let fallen = app.world_mut().spawn(body(1, true, Condition::DEAD)).id();
        press_continue(&mut app);
        app.update();
        let arena = app.world().resource::<ArenaMatch>();
        assert_eq!(arena.state, MatchState::Fighting);
        assert_eq!(arena.rounds.round, 3);
        assert_eq!(arena.screen, None);
        assert_eq!(arena.started_ms, None);
        let mut spawned = 0;
        let mut killed = Vec::new();
        let mut reset = Vec::new();
        let messages: Vec<SceneCommand> = app
            .world_mut()
            .resource_mut::<Messages<SceneCommand>>()
            .drain()
            .collect();
        for message in messages {
            match message {
                SceneCommand::Spawn(_) => spawned += 1,
                SceneCommand::Despawn(entity) => killed.push(entity),
                SceneCommand::Reset {
                    body,
                    reset: kept,
                    place,
                } => reset.push((body, kept, place)),
            }
        }
        // The reserve entrant comes in against the opponent who won, which
        // stays on its place and is reset with the AI's clocks held.
        assert_eq!(spawned, 1);
        assert!(!killed.contains(&opponent));
        assert_eq!(killed, vec![fallen]);
        assert_eq!(reset.len(), 1);
        assert_eq!(reset[0].0, opponent);
        assert_eq!(reset[0].1.act_at_ms, Some(1_000 + 13 + 10_000));
        // It is re-placed at the opponent's place.
        let spawn = app.world().resource::<ArenaScene>().spawn_at;
        assert_eq!(reset[0].2, opponent_place(spawn));
    }

    /// The next round's staging sets the staging freeze (here at once: the
    /// bare app has no fixed queue), as the original's next-round routine
    /// sets it once it has stood the bodies.
    #[test]
    fn the_next_round_sets_the_staging_freeze() {
        let mut app = arena_app();
        app.init_resource::<exa_ai::freeze::StagingFreeze>();
        let mut arena = ArenaMatch::begin(7);
        arena.state = MatchState::NextFight;
        arena.screen = Some(Screen::NextRound);
        app.insert_resource(arena);
        let mut scene = scene();
        scene.player_entry = None;
        app.insert_resource(scene);
        app.world_mut().spawn(body(1, true, Condition::NORMAL));
        app.world_mut().spawn(body(2, false, Condition::DEAD));
        app.update();
        let freeze = *app.world().resource::<exa_ai::freeze::StagingFreeze>();
        assert!(!freeze.frozen());
        press_continue(&mut app);
        app.update();
        assert_eq!(
            app.world().resource::<ArenaMatch>().state,
            MatchState::Fighting
        );
        assert!(
            app.world()
                .resource::<exa_ai::freeze::StagingFreeze>()
                .frozen()
        );
    }

    /// The arena's next round behind the flow's transition screen, on the
    /// real flow and arena systems: the Continue opens the screen (music
    /// fade off) and closes the end menu; the round is staged only at black,
    /// 600 ms later, where the pause counter is raised for three frames and
    /// the staging freeze is held; the screen's close, after the 500 ms hold
    /// and the 600 ms fade back in, releases the freeze.
    #[test]
    fn the_next_round_is_staged_behind_the_transition_screen() {
        use exa_physics::sim_app::{FixedCommands, SimPause, run_fixed_commands};
        let frame_ms = 16_u32;
        let mut app = arena_app();
        app.init_resource::<exa_ui::gui_draw::GuiFrames>()
            .add_plugins((
                crate::menu_screen::MenuScreenPlugin,
                crate::flow::FlowPlugin,
                crate::arena_fade::ArenaFadePlugin,
            ))
            .init_resource::<FixedCommands>()
            .init_resource::<SimPause>()
            .init_resource::<exa_ai::freeze::StagingFreeze>()
            .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
                std::time::Duration::from_millis(u64::from(frame_ms)),
            ));
        app.update();
        app.world_mut()
            .resource_mut::<crate::menu_screen::MenuScreen>()
            .close();
        {
            let mut flow = app.world_mut().resource_mut::<crate::flow::Flow>();
            flow.mode = crate::flow::Mode::Arena;
            flow.phase = crate::flow::Phase::Playing;
            flow.menu_open = false;
        }
        let mut arena = ArenaMatch::begin(7);
        arena.state = MatchState::NextFight;
        arena.screen = Some(Screen::NextRound);
        app.insert_resource(arena);
        let mut scene = scene();
        scene.player_entry = None;
        app.insert_resource(scene);
        app.world_mut().spawn(body(1, true, Condition::NORMAL));
        app.world_mut().spawn(body(2, false, Condition::DEAD));
        app.update();
        assert!(app.world().resource::<ArenaEndScreen>().open);
        let round = app.world().resource::<ArenaMatch>().rounds.round;
        press_continue(&mut app);
        app.update();
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .release(KeyCode::Enter);
        // The screen opened over the round; the menu closed and stays closed.
        assert!(
            app.world()
                .resource::<crate::flow::Flow>()
                .transition_running()
        );
        assert!(
            app.world()
                .resource::<crate::arena_fade::ArenaStaging>()
                .round_pending()
        );
        assert_eq!(app.world().resource::<ArenaMatch>().rounds.round, round + 1);
        let spawned = |app: &mut App| {
            app.world_mut()
                .resource_mut::<Messages<SceneCommand>>()
                .drain()
                .filter(|c| matches!(c, SceneCommand::Spawn(_)))
                .count()
        };
        spawned(&mut app);
        // The fade out: the game runs on, nothing is staged.
        let mut frames = 0;
        while app.world().resource::<ArenaMatch>().state != MatchState::Fighting {
            app.update();
            assert!(!app.world().resource::<ArenaEndScreen>().open);
            frames += 1;
            assert!(frames < 100, "the round was never staged");
            if app.world().resource::<ArenaMatch>().state != MatchState::Fighting {
                assert_eq!(app.world().resource::<SimPause>().raised(), 0);
            }
        }
        // 600 ms at 16 ms a frame: the 38th frame is past black.
        assert!((37..=40).contains(&frames), "{frames} frames to black");
        assert_eq!(spawned(&mut app), 1, "the fresh opponent");
        // The staging raised the pause; the freeze is held by the screen.
        assert_eq!(app.world().resource::<SimPause>().raised(), 1);
        run_fixed_commands(app.world_mut());
        assert!(
            app.world()
                .resource::<exa_ai::freeze::StagingFreeze>()
                .held()
        );
        // Three frames paused, lowered on the screen's second frame after
        // the callback.
        for frame in 0..3 {
            assert_eq!(app.world().resource::<SimPause>().raised(), 1, "{frame}");
            app.update();
        }
        assert_eq!(app.world().resource::<SimPause>().raised(), 0);
        // The hold and the fade back in: the freeze holds until the close.
        let mut frames = 0;
        while app
            .world()
            .resource::<crate::flow::Flow>()
            .transition_running()
        {
            run_fixed_commands(app.world_mut());
            assert!(
                app.world()
                    .resource::<exa_ai::freeze::StagingFreeze>()
                    .frozen()
            );
            app.update();
            frames += 1;
            assert!(frames < 200, "the screen never closed");
        }
        // 1100 ms at 16 ms a frame.
        assert!((68..=72).contains(&frames), "{frames} frames to the close");
        run_fixed_commands(app.world_mut());
        assert!(
            !app.world()
                .resource::<exa_ai::freeze::StagingFreeze>()
                .frozen()
        );
        assert_eq!(
            app.world().resource::<crate::flow::Flow>().phase,
            crate::flow::Phase::Playing
        );
        assert!(!app.world().resource::<ArenaEndScreen>().open);
    }

    /// The arena app with the flow, its transition screen and the arena's
    /// stagings, on a 16 ms frame, playing an arena session.
    fn flow_arena_app() -> App {
        use exa_physics::sim_app::{FixedCommands, SimPause};
        let mut app = arena_app();
        app.init_resource::<exa_ui::gui_draw::GuiFrames>()
            .add_plugins((
                crate::menu_screen::MenuScreenPlugin,
                crate::flow::FlowPlugin,
                crate::arena_fade::ArenaFadePlugin,
            ))
            .init_resource::<FixedCommands>()
            .init_resource::<SimPause>()
            .init_resource::<exa_ai::freeze::StagingFreeze>()
            .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
                std::time::Duration::from_millis(16),
            ));
        app.update();
        app.world_mut()
            .resource_mut::<crate::menu_screen::MenuScreen>()
            .close();
        let mut flow = app.world_mut().resource_mut::<crate::flow::Flow>();
        flow.mode = crate::flow::Mode::Arena;
        flow.phase = crate::flow::Phase::Playing;
        flow.menu_open = false;
        app
    }

    fn drain_exits(app: &mut App) -> Vec<ArenaExit> {
        app.world_mut()
            .resource_mut::<Messages<ArenaExit>>()
            .drain()
            .collect()
    }

    /// The Continue after the match's last round opens the transition screen
    /// over the return (the music fade on): the game runs on while it fades
    /// out and nothing is settled; at black the callback raises the pause,
    /// settles the match into the career and asks for the hub's level, and
    /// the screen then holds black over that change before it fades back in.
    #[test]
    fn the_return_to_the_hub_is_settled_behind_the_transition_screen() {
        use exa_physics::sim_app::SimPause;
        let mut app = flow_arena_app();
        let mut arena = ArenaMatch::begin(1);
        arena.state = MatchState::Won;
        arena.screen = Some(Screen::End);
        app.insert_resource(arena);
        app.insert_resource(scene());
        app.insert_resource(crate::hub_windows::ArenaCareer::default());
        app.world_mut().spawn(body(1, true, Condition::NORMAL));
        app.update();
        assert!(app.world().resource::<ArenaEndScreen>().open);
        let copper = app
            .world()
            .resource::<crate::hub_windows::ArenaCareer>()
            .company
            .copper;
        press_continue(&mut app);
        app.update();
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .release(KeyCode::Enter);
        assert!(
            app.world()
                .resource::<crate::flow::Flow>()
                .transition_running()
        );
        assert!(
            app.world()
                .resource::<crate::arena_fade::ArenaStaging>()
                .round_pending()
        );
        let mut frames = 1;
        loop {
            app.update();
            frames += 1;
            assert!(!app.world().resource::<ArenaEndScreen>().open);
            let exits = drain_exits(&mut app);
            if !exits.is_empty() {
                assert_eq!(exits, vec![ArenaExit::Hub]);
                break;
            }
            // The callback's frame settles too: before it nothing is paused.
            assert_eq!(app.world().resource::<SimPause>().raised(), 0);
            assert_eq!(
                app.world()
                    .resource::<crate::hub_windows::ArenaCareer>()
                    .company
                    .copper,
                copper,
                "settled before black"
            );
            assert!(frames < 100, "the return never ran");
        }
        assert!((37..=40).contains(&frames), "{frames} frames to black");
        let career = app.world().resource::<crate::hub_windows::ArenaCareer>();
        assert_eq!(career.company.won, 1);
        assert_eq!(career.matches_today, 1);
        assert!(career.company.copper > copper);
        assert_eq!(app.world().resource::<SimPause>().raised(), 1);
        assert!(app.world().resource::<ArenaMatch>().screen.is_none());
        // Black over the change's wait, then the two frames and the fade in.
        let mut held = 0;
        while app.world().resource::<SimPause>().raised() > 0 {
            app.update();
            held += 1;
            assert!(held < 100);
        }
        assert!(held > crate::flow::LOAD_WAIT_FRAMES, "{held}");
        while app
            .world()
            .resource::<crate::flow::Flow>()
            .transition_running()
        {
            app.update();
        }
        assert_eq!(
            app.world().resource::<crate::flow::Flow>().phase,
            crate::flow::Phase::Playing
        );
        assert!(drain_exits(&mut app).is_empty(), "the return runs once");
    }

    /// A dead manager's Continue leaves to the title behind the transition
    /// screen (the music fade on): no exit message and no settle, and the
    /// title's mode once the screen is black.
    #[test]
    fn a_dead_managers_continue_leaves_to_the_title_behind_the_screen() {
        let mut app = flow_arena_app();
        let mut arena = ArenaMatch::begin(1);
        arena.state = MatchState::Lost;
        arena.screen = Some(Screen::End);
        arena.manager_screen = Some(3);
        app.insert_resource(arena);
        app.insert_resource(scene());
        app.update();
        press_continue(&mut app);
        app.update();
        assert!(
            app.world()
                .resource::<crate::flow::Flow>()
                .transition_running()
        );
        assert!(app.world().resource::<ArenaMatch>().screen.is_none());
        let mut frames = 1;
        while app.world().resource::<crate::flow::Flow>().mode != crate::flow::Mode::Menu {
            app.update();
            frames += 1;
            assert!(frames < 100, "the leave never ran");
        }
        assert!((37..=40).contains(&frames), "{frames} frames to black");
        assert!(drain_exits(&mut app).is_empty());
        assert_eq!(
            app.world().resource::<crate::flow::Flow>().phase,
            crate::flow::Phase::Menu
        );
    }

    #[test]
    fn a_living_company_fighter_stays_for_the_next_round() {
        let mut app = arena_app();
        let mut arena = ArenaMatch::begin(7);
        arena.state = MatchState::NextFight;
        arena.screen = Some(Screen::NextRound);
        app.insert_resource(arena);
        let mut scene = scene();
        scene.player_entry = None;
        app.insert_resource(scene);
        let company = app.world_mut().spawn(body(1, true, Condition::NORMAL)).id();
        let opponent = app
            .world_mut()
            .spawn(body(2, false, Condition::NORMAL))
            .id();
        press_continue(&mut app);
        app.update();
        let messages: Vec<SceneCommand> = app
            .world_mut()
            .resource_mut::<Messages<SceneCommand>>()
            .drain()
            .collect();
        let mut spawned = 0;
        let mut killed = Vec::new();
        let mut reset = Vec::new();
        for message in messages {
            match message {
                SceneCommand::Spawn(_) => spawned += 1,
                SceneCommand::Despawn(entity) => killed.push(entity),
                SceneCommand::Reset { body, place, .. } => reset.push((body, place)),
            }
        }
        // The fighter fights on and is reset; a fresh opponent replaces the
        // old one.
        assert_eq!(spawned, 1);
        assert_eq!(killed, vec![opponent]);
        assert!(!killed.contains(&company));
        // It is re-placed at the company's place, facing the opponent.
        let spawn = app.world().resource::<ArenaScene>().spawn_at;
        assert_eq!(reset, vec![(company, our_place(spawn))]);
    }

    #[test]
    fn the_end_of_the_match_settles_the_career_and_asks_for_the_hub() {
        let mut app = arena_app();
        let mut arena = ArenaMatch::begin(1);
        arena.state = MatchState::Won;
        arena.screen = Some(Screen::End);
        app.insert_resource(arena);
        app.insert_resource(scene());
        let mut career = crate::hub_windows::ArenaCareer::default();
        career.company.roster.push(crate::career::Member {
            name: "Ash".to_string(),
            profession: crate::career::Profession::Combatant,
            xp: 0,
            skills: [0; 6],
            skill_xp: [0; 6],
            techniques: [[0; 6]; 6],
            loadouts: Default::default(),
            health: u16::MAX,
            in_entry: false,
            outfit: Default::default(),
        });
        app.insert_resource(career);
        let mut fighter = Fighter::default();
        fighter.health.red = 0.125;
        let fighter = app
            .world_mut()
            .spawn((
                fighter,
                Condition::NORMAL,
                SceneFighter {
                    team: 1,
                    company: true,
                    id: 1,
                    damage: 100.0,
                    condition_health: 100.0,
                    act_at_ms: 0,
                    behaviour: None,
                },
            ))
            .id();
        assert!(app.world().get::<Fighter>(fighter).is_some());
        press_continue(&mut app);
        app.update();
        let exits: Vec<ArenaExit> = app
            .world_mut()
            .resource_mut::<Messages<ArenaExit>>()
            .drain()
            .collect();
        assert_eq!(exits, vec![ArenaExit::Hub]);
        // The match settles into the live career resource; the hub's level
        // change then reads the same career in process (no hand-off file).
        let career = app.world().resource::<crate::hub_windows::ArenaCareer>();
        assert_eq!(career.company.copper, 355);
        assert_eq!(career.company.won, 1);
        assert_eq!(career.matches_today, 1);
        // The body's half-health went back into its record as the rounded
        // word (0.125 / 0.25 · 65535).
        assert_eq!(career.company.roster[0].health, 32_768);
    }

    fn hired(name: &str) -> crate::career::Member {
        crate::career::Member {
            name: name.to_string(),
            profession: crate::career::Profession::Combatant,
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

    #[test]
    fn a_company_fighter_who_dies_is_logged_counted_and_leaves_the_company() {
        let mut app = arena_app();
        app.add_plugins(crate::arena_match::ArenaMatchPlugin);
        app.insert_resource(ArenaMatch::begin(1));
        app.insert_resource(scene());
        let mut career = crate::hub_windows::ArenaCareer::default();
        career.company.roster.push(hired("Ash"));
        career.company.roster.push(hired("Bo"));
        career.company.deaths = 2;
        // Ash is entered in the first event with Bo; an earlier save of Bo
        // is in the log.
        career.entries[0].count = 2;
        career.entries[0].entries[0].actor_id = 1;
        career.entries[0].entries[1].actor_id = 2;
        career.log.append_value(crate::arena_match::LOG_SAVED, 2);
        app.insert_resource(career);
        let marker = |id| SceneFighter {
            team: 1,
            company: true,
            id,
            damage: 100.0,
            condition_health: 100.0,
            act_at_ms: 0,
            behaviour: None,
        };
        app.world_mut().spawn((
            crate::player::Player,
            crate::player::PlayerBody::new(0.0),
            Fighter::default(),
            Condition::NORMAL,
        ));
        let mut ash = Fighter::default();
        ash.health.red = -0.1;
        app.world_mut().spawn((ash, Condition::DEAD, marker(1)));
        app.world_mut()
            .spawn((Fighter::default(), Condition::NORMAL, marker(2)));
        let mut foe = Fighter::default();
        foe.health.red = -0.1;
        app.world_mut().spawn((
            foe,
            Condition::DEAD,
            SceneFighter {
                team: 2,
                company: false,
                ..marker(0)
            },
        ));
        // The first frame stamps the match; a second later the tick counts.
        app.update();
        app.world_mut()
            .resource_mut::<crate::effects::EffectClock>()
            .now_ms = 2_000;
        app.update();
        {
            let arena = app.world().resource::<ArenaMatch>();
            assert_eq!(arena.state, MatchState::Won);
            assert_eq!(arena.died, vec![1]);
            assert_eq!(arena.deaths, 1);
            let death = arena.log.records[0];
            assert_eq!(death.id, crate::arena_match::LOG_DIED);
            assert_eq!(
                death.words,
                crate::staff::name_words(&crate::hub_windows::name16("Ash"))
            );
            // Bo's save already names his place after Ash's removal.
            let career = app.world().resource::<crate::hub_windows::ArenaCareer>();
            assert_eq!(career.log.records[0].words[0], 1);
            // The roster still finds the match's fighters by id.
            assert_eq!(career.company.roster.len(), 2);
        }
        press_continue(&mut app);
        app.update();
        let career = app.world().resource::<crate::hub_windows::ArenaCareer>();
        // Ash is off the books, Bo is id 1, the event names Bo only, and the
        // staff list is asked to release Ash's kit.
        let names: Vec<&str> = career
            .company
            .roster
            .iter()
            .map(|member| member.name.as_str())
            .collect();
        assert_eq!(names, vec!["Bo"]);
        assert_eq!(career.entries[0].count, 1);
        assert_eq!(career.entries[0].entries[0].actor_id, 1);
        assert_eq!(career.staff_left, vec![0]);
        assert_eq!(career.company.deaths, 3);
        assert!(
            career
                .log
                .records
                .iter()
                .any(|record| record.id == crate::arena_match::LOG_DIED)
        );
    }

    #[test]
    fn the_return_books_all_thirteen_records() {
        let mut app = arena_app();
        let mut arena = ArenaMatch::begin(11);
        arena.state = MatchState::Won;
        arena.screen = Some(Screen::End);
        arena.deaths = 1;
        app.insert_resource(arena);
        let mut scene = scene();
        scene.template = 11;
        scene.prize = 432;
        app.insert_resource(scene);
        let mut career = crate::hub_windows::ArenaCareer::default();
        career.company.hires = 4;
        career.company.fired = 2;
        career.company.deaths = 3;
        career.company.total_earned = 700;
        career.company.total_spent = 300;
        app.insert_resource(career);
        press_continue(&mut app);
        app.update();
        let career = app.world().resource::<crate::hub_windows::ArenaCareer>();
        // The Beast win counts one more beast defeated and the win; the
        // counts the match does not touch survive the settle.
        assert_eq!(career.company.won, 1);
        assert_eq!(career.company.beasts_defeated, 1);
        // The fall the match counted joins the career's deaths.
        assert_eq!(career.company.deaths, 4);
        assert_eq!(career.company.hires, 4);
        assert_eq!(career.company.fired, 2);
        assert_eq!(career.company.total_earned, 700);
        assert_eq!(career.company.total_spent, 300);
        assert_eq!(career.company.earned, 432);
        assert_eq!(career.company.copper, crate::career::STARTING_COPPER + 432);
    }

    #[test]
    fn space_no_longer_runs_the_end_screens_continue() {
        let mut app = arena_app();
        let mut arena = ArenaMatch::begin(1);
        arena.state = MatchState::Won;
        arena.screen = Some(Screen::End);
        app.insert_resource(arena);
        app.insert_resource(scene());
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Space);
        app.update();
        assert_eq!(app.world().resource::<ArenaMatch>().state, MatchState::Won);
        assert!(app.world().resource::<ArenaEndScreen>().open);
        assert!(
            app.world_mut()
                .resource_mut::<Messages<ArenaExit>>()
                .drain()
                .next()
                .is_none()
        );
    }

    #[test]
    fn a_dead_manager_goes_to_the_main_menu() {
        let mut app = arena_app();
        let mut arena = ArenaMatch::begin(1);
        arena.state = MatchState::Lost;
        arena.screen = Some(Screen::End);
        arena.manager_screen = Some(3);
        app.insert_resource(arena);
        app.insert_resource(scene());
        press_continue(&mut app);
        app.update();
        let exits: Vec<ArenaExit> = app
            .world_mut()
            .resource_mut::<Messages<ArenaExit>>()
            .drain()
            .collect();
        assert_eq!(exits, vec![ArenaExit::MainMenu]);
    }

    #[test]
    fn escape_opens_the_arena_pause_page_and_a_closed_page_resumes() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<crate::flow::Flow>()
            .init_resource::<crate::flow::FlowSteps>()
            .init_resource::<crate::menu_screen::MenuScreen>()
            .init_resource::<ArenaEndScreen>()
            .add_systems(Update, arena_pause);
        app.init_resource::<ButtonInput<KeyCode>>();
        {
            let mut flow = app.world_mut().resource_mut::<crate::flow::Flow>();
            flow.mode = crate::flow::Mode::Arena;
            flow.phase = crate::flow::Phase::Playing;
        }
        app.insert_resource(ArenaMatch::begin(1));
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Escape);
        app.update();
        let flow = app.world().resource::<crate::flow::Flow>();
        assert_eq!(flow.phase, crate::flow::Phase::Paused);
        let steps = core::mem::take(
            &mut app
                .world_mut()
                .resource_mut::<crate::flow::FlowSteps>()
                .steps,
        );
        assert!(steps.contains(&crate::flow::Step::ShowMenu(
            crate::menu_pages::Page::ArenaPause
        )));
        // The menu closed (its page's Escape, or its CONTINUE row): the pause
        // ends on the next frame.
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .reset_all();
        app.update();
        assert_eq!(
            app.world().resource::<crate::flow::Flow>().phase,
            crate::flow::Phase::Playing
        );
    }

    #[test]
    fn the_end_screen_shows_the_arenas_rows() {
        let mut screen = ArenaEndScreen::default();
        screen.show(Screen::NextRound);
        assert!(screen.open);
        assert_eq!(screen.kind, 3);
        assert_eq!(screen.menu().unwrap().rows, ["NEXT FIGHT"]);
        assert_eq!(screen.menu().unwrap().scale, 0.9);
        screen.show(Screen::End);
        assert_eq!(screen.kind, 2);
        assert_eq!(screen.menu().unwrap().rows, ["EXIT;"]);
        // The frame goes through the same GUI path as the other end menus:
        // the backdrop alone when no font texture is installed.
        let frame = crate::end_menu_screen::end_menu_frame(
            1600,
            1080,
            &screen.menu().unwrap(),
            Some(0),
            &GuiTextures::default(),
        );
        assert_eq!(frame.layers.len(), 1);
        // Only the first row continues; confirming closes the screen.
        assert!(screen.confirm());
        assert!(!screen.open);
    }

    #[test]
    fn a_tournament_continue_raises_the_brackets_next_fight() {
        let mut app = arena_app();
        let spawn = [900.0, 0.0, 100.0];
        let mut arena = ArenaMatch::begin_match(7, spawn, 3, 2, true, Some(0));
        arena.state = MatchState::NextFight;
        arena.screen = Some(Screen::NextRound);
        app.insert_resource(arena);
        let mut scene = ArenaScene::new(7, spawn, 3, 2, true, Some(0), Vec::new(), 0, None);
        scene.roll = 42;
        // Template 7 is four rounds of duels: sixteen one-fighter teams, two
        // sides a fight. After the won first round eight teams survive, but
        // the next fight is only the first two of them: the player and one
        // drawn winner.
        let mut bracket = scene.bracket.clone().unwrap();
        assert_eq!((bracket.team_size, bracket.sides), (1, 2));
        let mut roll = 42;
        let survivors = bracket_next_fight(&mut bracket, 1, &mut roll);
        assert_eq!(survivors.iter().filter(|&&id| id > 0).count(), 8);
        let expected = 1;
        app.insert_resource(scene);
        let player = app
            .world_mut()
            .spawn((
                body(1, true, Condition::NORMAL),
                crate::player::PlayerBody::new(0.0),
            ))
            .id();
        // The won round's opponent is dead: the scene draw sets it aside and
        // names the player's team the fight's winner.
        let opponent = app.world_mut().spawn(body(2, false, Condition::DEAD)).id();
        press_continue(&mut app);
        app.update();
        let arena = app.world().resource::<ArenaMatch>();
        assert_eq!(arena.state, MatchState::Fighting);
        assert_eq!(arena.screen, None);
        let messages: Vec<SceneCommand> = app
            .world_mut()
            .resource_mut::<Messages<SceneCommand>>()
            .drain()
            .collect();
        let mut spawned = Vec::new();
        let mut killed = Vec::new();
        let mut placed = Vec::new();
        let mut reset = Vec::new();
        for message in messages {
            match message {
                SceneCommand::Spawn(plan) => spawned.push(*plan),
                SceneCommand::Despawn(entity) => killed.push(entity),
                SceneCommand::Reset { body, place, .. } => {
                    reset.push(body);
                    placed.push(place);
                }
            }
        }
        assert_eq!(spawned.len(), expected);
        assert_eq!(killed, vec![opponent]);
        // The kept player is reset and placed at its bracket place.
        assert_eq!(reset, vec![player]);
        assert_eq!(placed.len(), 1);
        // Every raised opponent is generated from its record and outfit.
        assert!(
            spawned
                .iter()
                .all(|plan| matches!(plan.look, BodyLook::Generated(_)))
        );
        assert!(spawned.iter().any(|plan| plan.team != 1));
        // Two teams fight, so the opponent is hostile (the original's role
        // flag is a free-for-all only from three teams up).
        assert!(spawned.iter().all(|plan| plan.roles == vec![HOSTILE]));
    }

    /// A later bracket round of a team event keeps the company's living
    /// bodies: their health words go back into their records, a dead
    /// company body leaves with the opponents, and the player and the living
    /// fighter are re-dressed and reset on their side's first two places in
    /// entry order (the dead one's place is not kept open), while the
    /// opposing side is generated.
    #[test]
    fn a_team_bracket_round_keeps_the_companys_living_bodies() {
        let mut app = arena_app();
        let spawn = [900.0, 0.0, 100.0];
        let mut arena = ArenaMatch::begin_match(7, spawn, 3, 2, true, Some(0));
        arena.state = MatchState::NextFight;
        arena.screen = Some(Screen::NextRound);
        app.insert_resource(arena);
        let mut scene = ArenaScene::new(7, spawn, 3, 2, true, Some(0), Vec::new(), 0, None)
            .with_entrants(vec![0, 1, 2]);
        scene.roll = 42;
        let slot = |id| crate::career::BracketSlot { id, strength: 1.0 };
        let slots: Vec<_> = [1, 3, 4, 5]
            .into_iter()
            .flat_map(|team| [slot(team), slot(team), slot(team)])
            .collect();
        scene.bracket = Some(BracketState {
            slots,
            team_size: 3,
            sides: 2,
            rounds: 2,
        });
        app.insert_resource(scene);
        let mut career = crate::hub_windows::ArenaCareer::default();
        career.company.roster.push(hired("Ash"));
        career.company.roster.push(hired("Birch"));
        app.insert_resource(career);
        let company = |app: &mut App, id: u32, condition: Condition, red: f32| {
            let mut fighter = Fighter::default();
            fighter.health.red = red;
            app.world_mut()
                .spawn((
                    fighter,
                    condition,
                    SceneFighter {
                        team: 1,
                        company: true,
                        id,
                        damage: 100.0,
                        condition_health: 100.0,
                        act_at_ms: 0,
                        behaviour: None,
                    },
                ))
                .id()
        };
        // Spawned out of entry order: the kept list follows the entries.
        let fallen = company(&mut app, 2, Condition::DEAD, 0.0);
        let ash = company(&mut app, 1, Condition::NORMAL, 0.125);
        let player = app
            .world_mut()
            .spawn((
                body(1, true, Condition::NORMAL),
                crate::player::PlayerBody::new(0.0),
            ))
            .id();
        let opponents: Vec<Entity> = (0..3)
            .map(|_| app.world_mut().spawn(body(3, false, Condition::DEAD)).id())
            .collect();
        press_continue(&mut app);
        app.update();
        let messages: Vec<SceneCommand> = app
            .world_mut()
            .resource_mut::<Messages<SceneCommand>>()
            .drain()
            .collect();
        let mut spawned = Vec::new();
        let mut killed = Vec::new();
        let mut reset = Vec::new();
        for message in messages {
            match message {
                SceneCommand::Spawn(plan) => spawned.push(*plan),
                SceneCommand::Despawn(entity) => killed.push(entity),
                SceneCommand::Reset { body, place, .. } => reset.push((body, place)),
            }
        }
        killed.sort();
        let mut expected = opponents.clone();
        expected.push(fallen);
        expected.sort();
        assert_eq!(killed, expected);
        assert_eq!(
            reset.iter().map(|(body, _)| *body).collect::<Vec<_>>(),
            vec![player, ash]
        );
        // The side's places are its three members' arc; the two kept bodies
        // take the first two.
        let bracket = app
            .world()
            .resource::<ArenaScene>()
            .bracket
            .clone()
            .unwrap();
        let survivors: Vec<i32> = bracket.slots.iter().map(|slot| slot.id).collect();
        let places = crate::match_layout::kept_side_places(&survivors, 3, 2, spawn, 1, 3).unwrap();
        assert_eq!(reset[0].1, places[0]);
        assert_eq!(reset[1].1, places[1]);
        // No company body is generated again; one opposing side of three.
        assert_eq!(spawned.len(), 3);
        assert!(spawned.iter().all(|plan| plan.team != 1));
        // The living fighter's health word went back into his record.
        let career = app.world().resource::<crate::hub_windows::ArenaCareer>();
        assert_eq!(
            career.company.roster[0].health,
            crate::arena_match::health_word(0.125, Fighter::default().health.max)
        );
        assert_eq!(career.company.roster[1].health, 0);
    }

    #[test]
    fn the_scene_draw_reads_each_bodys_team_and_sets_the_dying_aside() {
        use bevy::ecs::system::RunSystemOnce;
        let mut world = World::new();
        world.spawn((
            Fighter::default(),
            Condition::NORMAL,
            crate::player::PlayerBody::new(0.0),
        ));
        world.spawn(body(1, true, Condition::NORMAL));
        world.spawn(body(3, false, Condition::COLLAPSING));
        world.spawn(body(4, false, Condition::DOWN));
        // A fighter the match did not stand is not counted.
        world.spawn((Fighter::default(), Condition::NORMAL));
        let mut sides: Vec<(i32, bool)> = world
            .run_system_once(|bodies: Query<DrawBody>| {
                scene_sides(&bodies)
                    .iter()
                    .map(|side| (side.team, side.aside))
                    .collect::<Vec<_>>()
            })
            .unwrap();
        sides.sort();
        // A downed rival still counts; a collapsing one is set aside.
        assert_eq!(sides, vec![(1, false), (1, false), (3, true), (4, false)]);
    }

    #[test]
    fn a_forfeit_settles_the_match_and_asks_for_the_hub() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_message::<SceneCommand>()
            .add_message::<ArenaExit>()
            .add_message::<crate::flow::ForfeitMatch>()
            .add_systems(Update, forfeit_match::<false>);
        let mut arena = ArenaMatch::begin(1);
        arena.state = MatchState::Fighting;
        app.insert_resource(arena);
        app.insert_resource(scene());
        let mut career = crate::hub_windows::ArenaCareer::default();
        career.company.copper = 500;
        app.insert_resource(career);
        app.world_mut().spawn((
            Fighter::default(),
            Condition::NORMAL,
            SceneFighter {
                team: 1,
                company: true,
                id: 2,
                damage: 100.0,
                condition_health: 100.0,
                act_at_ms: 0,
                behaviour: None,
            },
        ));
        app.world_mut().write_message(crate::flow::ForfeitMatch);
        app.update();
        let exits: Vec<ArenaExit> = app
            .world_mut()
            .resource_mut::<Messages<ArenaExit>>()
            .drain()
            .collect();
        assert_eq!(exits, vec![ArenaExit::Hub]);
        // The forfeit settles into the live career resource the hub's
        // in-process level change reads.
        let career = app.world().resource::<crate::hub_windows::ArenaCareer>();
        assert_eq!(career.company.copper, 500);
        assert_eq!(career.company.lost, 1);
        assert_eq!(career.matches_today, 1);
    }

    /// The quit's forfeit (the app closing from the arena's pause page)
    /// settles the match in `Last` and asks for the forfeit's player-data
    /// save; the frame's instance stands aside, and a pause-page forfeit
    /// asks for no save.
    #[test]
    fn the_quits_forfeit_settles_then_asks_for_the_player_data_save() {
        for quitting in [false, true] {
            let mut app = App::new();
            app.add_plugins(MinimalPlugins)
                .add_message::<SceneCommand>()
                .add_message::<ArenaExit>()
                .add_message::<crate::flow::ForfeitMatch>()
                .add_message::<crate::save_live::SaveStep>()
                .insert_resource(crate::save_live::QuitAutosave(quitting))
                .add_systems(Update, forfeit_match::<false>)
                .add_systems(Last, forfeit_match::<true>);
            let mut arena = ArenaMatch::begin(1);
            arena.state = MatchState::Fighting;
            app.insert_resource(arena);
            app.insert_resource(scene());
            app.insert_resource(crate::hub_windows::ArenaCareer::default());
            app.world_mut().write_message(crate::flow::ForfeitMatch);
            app.update();
            let saves: Vec<crate::save_live::SaveStep> = app
                .world_mut()
                .resource_mut::<Messages<crate::save_live::SaveStep>>()
                .drain()
                .collect();
            let settled = app
                .world()
                .resource::<crate::hub_windows::ArenaCareer>()
                .company
                .lost;
            // One settle either way, by the instance for the app's state.
            assert_eq!(settled, 1);
            assert_eq!(
                saves,
                if quitting {
                    vec![crate::save_live::SaveStep::ForfeitSave]
                } else {
                    Vec::new()
                }
            );
        }
    }

    #[test]
    fn the_match_setup_comes_from_the_hub_and_else_the_command_line() {
        let launch = MatchSetup {
            template: 9,
            level: 4,
            tier: 2,
            tournament: true,
            player: Some(1),
            prize: 20,
            weapon_variants: Some([9, 0, 4]),
        };
        let setup = setup_from(Some(launch), 1, 0, 0);
        assert_eq!(setup.template, 9);
        assert!(setup.tournament);
        assert_eq!(setup.player, Some(1));
        assert_eq!(setup.prize, 20);
        let setup = setup_from(None, 6, 2, 1);
        assert_eq!(setup.template, 6);
        assert_eq!((setup.level, setup.tier), (2, 1));
        assert!(!setup.tournament);
        // A bare command-line match's debug player is entry zero.
        assert_eq!(setup.player, Some(0));
    }
}
