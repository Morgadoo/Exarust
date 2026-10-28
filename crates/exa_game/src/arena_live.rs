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
//! team one, the other groups draw a winner) and stands every surviving
//! side. Escape opens the arena's pause page and its FORFEIT row settles
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
use crate::arena_outfit::Lcg;
use crate::arena_session::{
    BracketState, EndAction, ReturnFighter, RoundFight, Screen,
    bracket_for_template, bracket_next_fight, regenerate_round, settle_return,
};
use crate::arena_spawn::{MANAGER_ROLE, generic_plan, hire_plan};
use crate::match_layout::{
    FREE_FOR_ALL, Fighter as SlotFighter, FreshMatch, HOSTILE, MATCH_ACT_GRACE_MS, NextRound,
    RaisedBody, Slot, layout, opponent_place, our_place, raise_fresh_match,
};
use crate::scene_bodies::{
    BodyBrain, SceneBodyPlan, SceneCommand, SceneFighter, beast_scene_plan, company_plan,
    hire_opponent_plan, opponent_plan,
};

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
    /// The roll a bracket's unwatched pairs and its weapon draws use.
    pub roll: u32,
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
            records: career.map_or_else(CompanyRecords::default, records_of),
            bracket: if tournament {
                bracket_for_template(template)
            } else {
                None
            },
            weapon_variants: None,
            roll: crate::arena_match::counter_now() as u32,
        }
    }

    /// The event weapon row's variants, for the bracket's later fights.
    #[must_use]
    pub fn with_weapon_variants(mut self, variants: Option<[u32; 3]>) -> Self {
        self.weapon_variants = variants;
        self
    }
}

/// The career's thirteen records as the match settles them.
fn records_of(career: &crate::hub_windows::ArenaCareer) -> CompanyRecords {
    CompanyRecords {
        matches_won: career.company.won,
        matches_lost: career.company.lost,
        matches_forfeited: 0,
        tournaments_won: career.company.tournaments_won,
        tournaments_lost: 0,
        seasons_won: career.company.seasons_won,
        beasts_fought: 0,
        beasts_defeated: 0,
        hires: 0,
        deaths: career.company.deaths,
        fired: 0,
        earned: career.company.earned,
        spent: career.company.spent,
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

/// Whether the end screen's Continue runs without a key (unattended runs
/// and tests). In play it is always false.
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
                    mark_arena_session,
                    stand_first_round,
                    arena_pause,
                    open_end_screen,
                    update_end_screen,
                    continue_round,
                    forfeit_match,
                    draw_end_screen,
                )
                    .chain()
                    .after(crate::gui_screens::draw_screens),
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
                let member = career.and_then(|career| {
                    career
                        .company
                        .roster
                        .get(index.saturating_sub(1))
                        .map(|member| (member.name.as_str(), member.health))
                });
                plans.push(company_plan(
                    member,
                    &slot.place,
                    MANAGER_ROLE,
                    u32::try_from(index).unwrap_or(0).wrapping_add(1),
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
                    let weapon = scene.weapons.get(opponent).copied().filter(|&id| id != 0);
                    // The event's weapon field adds the profile's armed flag;
                    // the drawn variant itself is the item shown.
                    let event_weapon = i32::from(weapon.is_some());
                    // The team ids after the player's are the company ids
                    // the forced outfit table is indexed by [guess: the
                    // bracket's company table is filled from the event's
                    // teams].
                    hire_opponent_plan(
                        hire_plan(tier as u8, team as i32, event_weapon),
                        &slot.place,
                        flags,
                        weapon,
                        now_ms,
                        act_grace_ms,
                    )
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
                let beast =
                    crate::arena_spawn::beast_plan(u32::try_from(tier).unwrap_or(1).max(1), 0.0);
                let mut body = beast_scene_plan(&beast, &slot.place, now_ms, act_grace_ms);
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
fn open_end_screen(arena: Option<Res<ArenaMatch>>, mut screen: Option<ResMut<ArenaEndScreen>>) {
    let (Some(arena), Some(screen)) = (arena, screen.as_mut()) else {
        return;
    };
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

/// Stands the first round's bodies once the level is loaded and the player
/// is there.
#[allow(clippy::too_many_arguments)]
fn stand_first_round(
    scene: Option<ResMut<ArenaScene>>,
    arena: Option<ResMut<ArenaMatch>>,
    career: Option<Res<crate::hub_windows::ArenaCareer>>,
    loading: Option<Res<exa_ui::LoadingStatus>>,
    levels: Query<&exa_world::LevelRoot>,
    building: Query<(), With<exa_physics::StaticGeometry>>,
    players: Query<(), With<crate::player::Player>>,
    clock: Option<Res<crate::effects::EffectClock>>,
    mut commands: MessageWriter<SceneCommand>,
) {
    let Some(mut scene) = scene else {
        return;
    };
    if scene.placed || loading.is_some() || !building.is_empty() || players.is_empty() {
        return;
    }
    if !levels
        .iter()
        .any(|level| crate::arena_match::arena_index(&level.name).is_some())
    {
        return;
    }
    let now = clock.as_ref().map_or(0, |clock| clock.now_ms);
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
}

/// The end screen's Continue, the next round and the return.
#[allow(clippy::too_many_arguments)]
fn continue_round(
    mut arena: Option<ResMut<ArenaMatch>>,
    mut scene: Option<ResMut<ArenaScene>>,
    mut career: Option<ResMut<crate::hub_windows::ArenaCareer>>,
    mut confirms: MessageReader<ArenaContinue>,
    mut screen: Option<ResMut<ArenaEndScreen>>,
    auto: Res<ArenaAutoContinue>,
    clock: Option<Res<crate::effects::EffectClock>>,
    fighters: Query<MatchBody<'_>>,
    mut commands: MessageWriter<SceneCommand>,
    mut exits: MessageWriter<ArenaExit>,
) {
    let (Some(arena), Some(scene)) = (arena.as_mut(), scene.as_mut()) else {
        return;
    };
    if arena.screen.is_none() || arena.state == MatchState::Fighting {
        return;
    }
    // The end screen's row is the only input now; the auto-continue switch
    // runs the same handler without it.
    let pressed = confirms.read().next().is_some();
    if !pressed && !auto.0 {
        return;
    }
    if auto.0
        && let Some(screen) = screen.as_mut()
    {
        screen.close();
    }
    let action = arena.end_action(scene.tournament);
    let now = clock.as_ref().map_or(0, |clock| clock.now_ms);
    match action {
        EndAction::MainMenu => {
            for (entity, ..) in &fighters {
                commands.write(SceneCommand::Despawn(entity));
            }
            exits.write(ArenaExit::MainMenu);
            arena.screen = None;
        }
        EndAction::NextRound { round, bracket } => {
            if bracket {
                // A tournament's next round raises the bracket's fight: the
                // won group keeps team one, the other groups draw their
                // winner, and the survivors are stood on the ring.
                if stand_bracket_round(scene, career.as_deref(), &fighters, now, &mut commands) {
                    info!("arena match: tournament round {round} raised the bracket fight");
                } else {
                    warn!("arena match: tournament round {round} has no bracket");
                }
            } else {
                let entered = scene.player_entry.is_some();
                let fighter_dead = fighters.iter().any(|(_, _, condition, body, player)| {
                    (if entered { player } else { body.company }) && *condition == Condition::DEAD
                });
                let opponent_present = fighters.iter().any(|(_, _, condition, body, _)| {
                    !body.company && *condition != Condition::DEAD
                });
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
                    now,
                    &mut commands,
                );
            }
            arena.rounds.round = round;
            arena.state = MatchState::Fighting;
            arena.screen = None;
            arena.started_ms = None;
            arena.resolved.clear();
        }
        EndAction::ReturnToHub { tournament } => {
            let finish = if arena.state == MatchState::Won {
                Finish::Won
            } else {
                Finish::Lost
            };
            finish_and_exit(
                arena,
                scene,
                career.as_deref_mut(),
                finish,
                tournament,
                &fighters,
                &mut commands,
                &mut exits,
            );
        }
    }
}

/// The next round's bodies: the winner stays and the reserve or a fresh
/// opponent comes, after the round's yield.
#[allow(clippy::too_many_arguments)]
fn stand_next_round(
    fight: &RoundFight,
    scene: &ArenaScene,
    career: Option<&crate::hub_windows::ArenaCareer>,
    fighters: &Query<MatchBody<'_>>,
    now_ms: u32,
    commands: &mut MessageWriter<SceneCommand>,
) {
    let opponent = opponent_place(scene.spawn_at);
    let ours = our_place(scene.spawn_at);
    for (entity, _, condition, body, _) in fighters {
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
        }
    }
    // The scene work waits the round's yield before it touches the bodies;
    // the bodies' own AI clock starts at the action grace after that.
    let now = now_ms.wrapping_add(fight.yield_ms);
    if let Some(plan) = &fight.opponent {
        let body = opponent_plan(*plan, &opponent, HOSTILE, now, MATCH_ACT_GRACE_MS);
        commands.write(SceneCommand::Spawn(Box::new(body)));
    }
    if let NextRound::Reserve(index) = fight.company {
        let member = career.and_then(|career| {
            career
                .company
                .roster
                .get(index.saturating_sub(1))
                .map(|member| (member.name.as_str(), member.health))
        });
        let body = company_plan(
            member,
            &ours,
            MANAGER_ROLE,
            u32::try_from(index).unwrap_or(0).wrapping_add(1),
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
    career: Option<&crate::hub_windows::ArenaCareer>,
    fighters: &Query<MatchBody<'_>>,
    now_ms: u32,
    commands: &mut MessageWriter<SceneCommand>,
) -> bool {
    let Some(bracket) = scene.bracket.as_mut() else {
        return false;
    };
    // The player's side won the watched fight, so team one keeps its place;
    // every other group's winner is drawn from the roll.
    let mut roll = scene.roll;
    let survivors = bracket_next_fight(bracket, 1, &mut roll);
    scene.roll = roll;
    // The forced outfit table is indexed by the team ids after the
    // player's. A team without an entry keeps the profile's own bytes.
    let outfits: Vec<i32> = (0..20).collect();
    let teams = survivors.iter().filter(|&&id| id > 0).count()
        / usize::try_from(bracket.team_size.max(1)).unwrap_or(1);
    // The original's role flag follows the number of *groups* in the round,
    // not the number of surviving teams: hostile below three groups, else a
    // free-for-all.
    let groups = teams / usize::try_from(bracket.sides.max(1)).unwrap_or(1);
    let role = if groups < 3 { HOSTILE } else { FREE_FOR_ALL };
    let fight = FreshMatch {
        slots: &survivors,
        per_team: bracket.team_size,
        teams: teams as i32,
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
    // Every body but the player's is rebuilt; the player is placed on its
    // bracket spot instead.
    for (entity, _, _, _, player) in fighters {
        if !player {
            commands.write(SceneCommand::Despawn(entity));
        }
    }
    for fighter in raised.fighters {
        let place = fighter.place;
        match fighter.body {
            RaisedBody::Player => {
                if let Some((entity, ..)) = fighters.iter().find(|(_, _, _, _, player)| *player) {
                    commands.write(SceneCommand::Place {
                        body: entity,
                        at: Vec3::from_array(place.position) / 100.0,
                        yaw: place.forward[0].atan2(place.forward[2]),
                    });
                }
            }
            RaisedBody::Company {
                role, act_at_ms, ..
            } => {
                let member = career.and_then(|career| {
                    career
                        .company
                        .roster
                        .get(fighter.index.saturating_sub(1))
                        .map(|member| (member.name.as_str(), member.health))
                });
                let body = company_plan(
                    member,
                    &place,
                    role,
                    u32::try_from(fighter.index).unwrap_or(0).wrapping_add(1),
                    0,
                    act_at_ms,
                );
                commands.write(SceneCommand::Spawn(Box::new(body)));
            }
            RaisedBody::Opponent {
                plan, act_at_ms, ..
            } => {
                let mut body = match plan {
                    crate::match_layout::OpponentPlan::Generic(plan) => {
                        opponent_plan(plan, &place, role, 0, act_at_ms)
                    }
                    crate::match_layout::OpponentPlan::Hire(plan) => {
                        let weapon = if plan.draw_weapon {
                            plan.weapon_row.and(scene.weapon_variants).map(|row| {
                                let mut lcg = Lcg::new(scene.roll);
                                let step = lcg.step();
                                scene.roll = lcg.state();
                                crate::arena_spawn::pick_weapon_slot(row, step)
                            })
                        } else {
                            None
                        };
                        hire_opponent_plan(plan, &place, role, weapon, 0, act_at_ms)
                    }
                };
                body.team = u32::try_from(fighter.team).unwrap_or(0);
                commands.write(SceneCommand::Spawn(Box::new(body)));
            }
        }
    }
    true
}

/// Settle `finish`, take the round's bodies away and ask for the hub's
/// level; the settled career stays in its resource. Both the end screen's Continue and the pause page's
/// forfeit leave this way.
#[allow(clippy::too_many_arguments)]
fn finish_and_exit(
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
#[allow(clippy::too_many_arguments)]
fn forfeit_match(
    mut forfeits: Option<MessageReader<crate::flow::ForfeitMatch>>,
    mut arena: Option<ResMut<ArenaMatch>>,
    mut scene: Option<ResMut<ArenaScene>>,
    mut career: Option<ResMut<crate::hub_windows::ArenaCareer>>,
    fighters: Query<MatchBody<'_>>,
    mut commands: MessageWriter<SceneCommand>,
    mut exits: MessageWriter<ArenaExit>,
) {
    let Some(forfeits) = forfeits.as_mut() else {
        return;
    };
    if forfeits.read().next().is_none() {
        return;
    }
    let (Some(arena), Some(scene)) = (arena.as_mut(), scene.as_mut()) else {
        return;
    };
    if arena.state != MatchState::Fighting {
        return;
    }
    match arena.forfeit(scene.tournament) {
        crate::arena_session::ForfeitResult::Ignored => {
            info!("arena forfeit: the manager is already down, ignored");
        }
        crate::arena_session::ForfeitResult::Return { tournament } => {
            info!("arena forfeit: returning to the hub");
            finish_and_exit(
                arena,
                scene,
                career.as_deref_mut(),
                Finish::Forfeited,
                tournament,
                &fighters,
                &mut commands,
                &mut exits,
            );
        }
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
        .map(|(_, fighter, _, _, _)| ReturnFighter {
            company: true,
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
    career.matches_today = out.matches_today;
    career.company.copper = career.company.copper.saturating_add(out.paid);
    career.company.earned = career.company.earned.saturating_add(out.paid);
    career.company.won = scene.records.matches_won;
    career.company.lost = scene.records.matches_lost;
    career.company.tournaments_won = scene.records.tournaments_won;
    career.company.seasons_won = scene.records.seasons_won;
    career.company.deaths = scene.records.deaths;
    if out.day.drop_unfit {
        let healths: Vec<u16> = career
            .company
            .roster
            .iter()
            .map(|member| member.health)
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
                (open_end_screen, update_end_screen, continue_round).chain(),
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

    #[test]
    fn a_duel_lays_out_the_player_and_one_opponent() {
        let spawn = [900.0, 0.0, 100.0];
        let slots = layout(1, 3, 2, spawn);
        let plans = round_plans(&scene(), None, &slots, 1_000, 3_000);
        assert_eq!(plans.len(), 1);
        assert_eq!(plans[0].team, 2);
        assert!(matches!(plans[0].brain, BodyBrain::Duellist { .. }));
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

    #[test]
    fn a_tournament_side_takes_the_drawn_weapon_and_its_team() {
        let spawn = [0.0; 3];
        let mut scene = scene();
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
        // The team's company bytes are forced into the hire profile, and
        // the event's weapon sets the profile's armed flag.
        let BodyLook::Generated(fighter) = plans[0].look else {
            panic!("a tournament side takes a generated record");
        };
        let crate::arena_records::OutfitSource::Profile(profile) = fighter.outfit else {
            panic!("a hire side takes the profile pass");
        };
        assert_eq!(profile.flags(), 0x0f);
        assert_eq!(&profile.0[5..8], &[96, 227, 0]);
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
        let messages: Vec<SceneCommand> = app
            .world_mut()
            .resource_mut::<Messages<SceneCommand>>()
            .drain()
            .collect();
        for message in messages {
            match message {
                SceneCommand::Spawn(_) => spawned += 1,
                SceneCommand::Despawn(entity) => killed.push(entity),
                SceneCommand::Place { .. } => {}
            }
        }
        // The reserve entrant comes in against the opponent who won, which
        // stays on its place.
        assert_eq!(spawned, 1);
        assert!(!killed.contains(&opponent));
        assert_eq!(killed, vec![fallen]);
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
        for message in messages {
            match message {
                SceneCommand::Spawn(_) => spawned += 1,
                SceneCommand::Despawn(entity) => killed.push(entity),
                SceneCommand::Place { .. } => {}
            }
        }
        // The fighter fights on; a fresh opponent replaces the old one.
        assert_eq!(spawned, 1);
        assert_eq!(killed, vec![opponent]);
        assert!(!killed.contains(&company));
    }

    #[test]
    fn the_end_of_the_match_settles_the_career_and_asks_for_the_hub() {
        let mut app = arena_app();
        let mut arena = ArenaMatch::begin(1);
        arena.state = MatchState::Won;
        arena.screen = Some(Screen::End);
        app.insert_resource(arena);
        app.insert_resource(scene());
        app.insert_resource(crate::hub_windows::ArenaCareer::default());
        let fighter = app
            .world_mut()
            .spawn((
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
        // The same bracket rule the system runs, for the expected count.
        let mut bracket = scene.bracket.clone().unwrap();
        let mut roll = 42;
        let survivors = bracket_next_fight(&mut bracket, 1, &mut roll);
        let expected = survivors.iter().filter(|&&id| id > 0).count() - 1;
        app.insert_resource(scene);
        let player = app
            .world_mut()
            .spawn((
                body(1, true, Condition::NORMAL),
                crate::player::PlayerBody::new(0.0),
            ))
            .id();
        let opponent = app
            .world_mut()
            .spawn(body(2, false, Condition::NORMAL))
            .id();
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
        for message in messages {
            match message {
                SceneCommand::Spawn(plan) => spawned.push(*plan),
                SceneCommand::Despawn(entity) => killed.push(entity),
                SceneCommand::Place { body, .. } => placed.push(body),
            }
        }
        assert_eq!(spawned.len(), expected);
        assert_eq!(killed, vec![opponent]);
        assert_eq!(placed, vec![player]);
        // Every raised opponent is generated from its record and outfit.
        assert!(
            spawned
                .iter()
                .all(|plan| matches!(plan.look, BodyLook::Generated(_)))
        );
        assert!(spawned.iter().any(|plan| plan.team != 1));
        // Four groups fight this round, so every opponent is a free-for-all
        // side (the original's role flag), not a hostile one.
        assert!(spawned.iter().all(|plan| plan.roles == vec![FREE_FOR_ALL]));
    }

    #[test]
    fn a_forfeit_settles_the_match_and_asks_for_the_hub() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_message::<SceneCommand>()
            .add_message::<ArenaExit>()
            .add_message::<crate::flow::ForfeitMatch>()
            .add_systems(Update, forfeit_match);
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
