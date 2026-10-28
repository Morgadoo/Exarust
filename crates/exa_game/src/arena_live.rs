//! The arena match's live scene: the round's bodies, the end screen's
//! Continue, the next round and the way back to the hub.
//!
//! The hub's board pick starts a match by restarting the process on the
//! drawn arena (a live level change is blocked; see
//! [`crate::level_transition`]). Before the restart the hub writes the
//! career and the match's setup into the hand-off file
//! ([`crate::arena_handoff`]); this module reads it and stands the round's
//! bodies from the match's layout and the spawn plans — a generated
//! opponent from [`crate::arena_spawn::generic_plan`] or a tournament's
//! hire-list profile with its drawn weapon, the Beast, and the company's
//! other entrants. The player's own body is the one the play start already
//! made.
//!
//! When the round result opens a transition screen, the end screen's
//! Continue runs on Enter or Space (the screen itself is not drawn yet; the
//! key is the message box's confirming key, a stand-in). A next round
//! despawns the round's bodies and stands the winner and the reserve or a
//! fresh opponent after the original's 13 ms yield. The end of the match
//! settles the result into the career, drops the unfit on a tournament day
//! and writes the settled career and the return placement for the hub's
//! process, which then places the manager at the saved 300 cm, half turn,
//! flag `0x100` placement.
//!
//! Evidence: the arena's match tick, end screen, next-round routine and
//! return (`arena_session`, `arena_match`, `match_layout`).

use bevy::prelude::*;
use exa_combat::{Condition, Fighter};

use crate::arena;
use crate::arena_handoff::{self, Handoff, MatchSetup};
use crate::arena_match::{ArenaMatch, CompanyRecords, Finish, MatchState, any_entry};
use crate::arena_session::{
    EndAction, RETURN_PLACEMENT, ReturnFighter, RoundFight, Screen, regenerate_round, settle_return,
};
use crate::arena_spawn::{MANAGER_ROLE, generic_plan, hire_plan};
use crate::match_layout::{
    Fighter as SlotFighter, HOSTILE, MATCH_ACT_GRACE_MS, NextRound, Slot, layout, opponent_place,
    our_place,
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
        }
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

/// The match's exit: where the process goes when the end screen's Continue
/// settles it.
#[derive(Message, Clone, Debug, PartialEq)]
pub enum ArenaExit {
    /// The settled match returns to the hub's level. The career and the
    /// return placement are in the hand-off file.
    Hub,
    /// The manager is dead: the main menu.
    MainMenu,
}

/// The hand-off the process started with, once it was read.
#[derive(Resource, Clone, Debug, Default, PartialEq)]
pub struct HandedOver(pub Option<Handoff>);

/// Where the hand-over file lives: the temporary directory by default, and a
/// test's own file when it says so.
#[derive(Resource, Clone, Debug, PartialEq, Eq)]
pub struct HandoffPath(pub std::path::PathBuf);

impl Default for HandoffPath {
    fn default() -> Self {
        Self(arena_handoff::default_path())
    }
}

/// Whether the end screen's Continue runs without a key (unattended runs
/// and tests). In play it is always false.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ArenaAutoContinue(pub bool);

/// Runs the live arena scene.
pub struct ArenaLivePlugin;

impl Plugin for ArenaLivePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ArenaAutoContinue>()
            .init_resource::<HandedOver>()
            .init_resource::<HandoffPath>()
            .add_message::<ArenaExit>()
            .add_message::<SceneCommand>()
            .add_systems(PreStartup, restore_handoff)
            .add_systems(Update, (stand_first_round, continue_round).chain());
    }
}

/// Reads the hand-off once, at the process start: the hub's launched match
/// takes the career and its setup, the hub takes the settled career and the
/// return placement. The file is removed either way.
fn restore_handoff(
    mut commands: Commands,
    mut handed: ResMut<HandedOver>,
    path: Res<HandoffPath>,
    mut hub: Option<ResMut<crate::arena_hub_live::ArenaHub>>,
) {
    let Some(handoff) = arena_handoff::take_at(&path.0) else {
        return;
    };
    // The career the process starts with is the hub's own, so the hand-off's
    // replaces it; the career resource itself is created by the hub plugin.
    if let Some(career) = handoff.career.clone() {
        commands.insert_resource(career);
    }
    if let Some(hub) = hub.as_mut()
        && let Some(returned) = handoff.returned
    {
        hub.returned = Some(returned);
    }
    info!(
        "arena hand-off: career {}, launch {}, return {}",
        handoff.career.is_some(),
        handoff.launch.is_some(),
        handoff.returned.is_some()
    );
    *handed = HandedOver(Some(handoff));
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
                    hire_opponent_plan(
                        hire_plan(tier as u8, 0, event_weapon),
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
    keys: Option<Res<ButtonInput<KeyCode>>>,
    auto: Res<ArenaAutoContinue>,
    clock: Option<Res<crate::effects::EffectClock>>,
    path: Option<Res<HandoffPath>>,
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
    let pressed = keys
        .as_ref()
        .is_some_and(|keys| keys.just_pressed(KeyCode::Enter) || keys.just_pressed(KeyCode::Space));
    if !pressed && !auto.0 {
        return;
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
                // The bracket's next fight is not raised yet: the round takes
                // a fresh generic opponent (`match_layout::raise_fresh_match`
                // holds the bracket's rules).
                info!("arena match: tournament round {round} raises no bracket fight yet");
            }
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
                now,
                &mut commands,
            );
            arena.rounds.round = round;
            arena.state = MatchState::Fighting;
            arena.screen = None;
            arena.started_ms = None;
            arena.resolved.clear();
        }
        EndAction::ReturnToHub { tournament } => {
            settle_match(arena, scene, career.as_deref_mut(), tournament, &fighters);
            for (entity, ..) in &fighters {
                commands.write(SceneCommand::Despawn(entity));
            }
            let handoff = Handoff {
                career: career.as_deref().cloned(),
                launch: None,
                returned: Some(RETURN_PLACEMENT),
            };
            let path = path
                .as_ref()
                .map_or_else(arena_handoff::default_path, |path| path.0.clone());
            match arena_handoff::write(&path, &handoff) {
                Ok(()) => info!("arena return: wrote the settled career for the hub"),
                Err(error) => warn!("arena return: cannot write the hand-off: {error}"),
            }
            exits.write(ArenaExit::Hub);
            arena.screen = None;
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

/// The match's end: settle the result into the career, as the return to the
/// hub does. The round's fallen company bodies were already resolved by the
/// match tick, so this pass only books the result.
fn settle_match(
    arena: &mut ArenaMatch,
    scene: &mut ArenaScene,
    career: Option<&mut crate::hub_windows::ArenaCareer>,
    tournament: bool,
    fighters: &Query<MatchBody<'_>>,
) {
    let finish = if arena.state == MatchState::Won {
        Finish::Won
    } else {
        Finish::Lost
    };
    let Some(career) = career else {
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
    handoff: Option<&Handoff>,
    options_template: i32,
    level: i32,
    tier: i32,
) -> (MatchSetup, Option<crate::hub_windows::ArenaCareer>) {
    let setup = handoff
        .and_then(|handoff| handoff.launch)
        .unwrap_or(MatchSetup {
            template: options_template,
            level,
            tier,
            tournament: false,
            // A match started from the command line has no hub entry list:
            // the debug player is the company's entry zero.
            player: Some(0),
            prize: 0,
        });
    (setup, handoff.and_then(|handoff| handoff.career.clone()))
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
    use crate::scene_bodies::SceneFighter;

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
            .init_resource::<ArenaAutoContinue>()
            .add_systems(Update, continue_round);
        app.init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<HandoffPath>();
        app.insert_resource(crate::effects::EffectClock { now_ms: 1_000 });
        app
    }

    /// A test's own hand-off file, so the tests do not share one.
    fn handoff_path(name: &str) -> HandoffPath {
        HandoffPath(
            std::env::temp_dir().join(format!("exarust-handoff-{name}-{}", std::process::id())),
        )
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
        let path = handoff_path("settle");
        app.insert_resource(path.clone());
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
        let handoff = arena_handoff::take_at(&path.0).expect("a settled hand-off");
        assert_eq!(handoff.returned, Some(RETURN_PLACEMENT));
        let career = handoff.career.expect("the career");
        assert_eq!(career.company.copper, 355);
        assert_eq!(career.company.won, 1);
        assert_eq!(career.matches_today, 1);
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
    fn the_process_restores_the_handed_career_and_the_return_once() {
        let mut career = crate::hub_windows::ArenaCareer::default();
        career.company.name = "Handed".into();
        career.company.copper = 99;
        let handoff = Handoff {
            career: Some(career.clone()),
            launch: Some(MatchSetup {
                template: 7,
                level: 2,
                tier: 1,
                tournament: false,
                player: Some(0),
                prize: 8,
            }),
            returned: Some(RETURN_PLACEMENT),
        };
        let path = handoff_path("restore");
        arena_handoff::write(&path.0, &handoff).unwrap();
        let mut app = App::new();
        app.add_plugins(MinimalPlugins).add_plugins(ArenaLivePlugin);
        app.insert_resource(path.clone());
        app.insert_resource(crate::arena_hub_live::ArenaHub::default());
        app.insert_resource(crate::hub_windows::ArenaCareer::default());
        app.update();
        assert_eq!(
            app.world().resource::<crate::hub_windows::ArenaCareer>(),
            &career
        );
        assert_eq!(
            app.world()
                .resource::<crate::arena_hub_live::ArenaHub>()
                .returned,
            Some(RETURN_PLACEMENT)
        );
        assert_eq!(
            app.world()
                .resource::<HandedOver>()
                .0
                .as_ref()
                .unwrap()
                .launch,
            handoff.launch
        );
        // The file is used once.
        assert!(arena_handoff::read(&path.0).is_none());
    }

    #[test]
    fn the_match_setup_comes_from_the_hub_and_else_the_command_line() {
        let handoff = Handoff {
            launch: Some(MatchSetup {
                template: 9,
                level: 4,
                tier: 2,
                tournament: true,
                player: Some(1),
                prize: 20,
            }),
            ..Default::default()
        };
        let (setup, career) = setup_from(Some(&handoff), 1, 0, 0);
        assert_eq!(setup.template, 9);
        assert!(setup.tournament);
        assert_eq!(setup.player, Some(1));
        assert_eq!(setup.prize, 20);
        assert!(career.is_none());
        let (setup, _) = setup_from(None, 6, 2, 1);
        assert_eq!(setup.template, 6);
        assert_eq!((setup.level, setup.tier), (2, 1));
        assert!(!setup.tournament);
        // A bare command-line match's debug player is entry zero.
        assert_eq!(setup.player, Some(0));
    }
}
