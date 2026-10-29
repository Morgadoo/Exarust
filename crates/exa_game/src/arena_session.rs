//! The arena match's end screen, its next round and its way back to the hub.
//!
//! A round's end sets the session's state word: 8 when another round
//! follows, 5 on the last loss and 4 on the last win. The end screen the
//! transition opens is 3 for the next round and 2 for the end of the match.
//! Experience is awarded on both. The end screen's Continue checks the
//! manager and the round count: a dead manager goes to the main menu;
//! another round follows while the round less the losses is under the extra
//! rounds and the state is the next fight; otherwise the match is settled
//! and the player data is saved with the manager at 300 cm and a half turn,
//! flag 0x100, before the hub opens.
//!
//! The next round keeps the company's fighter and generates a fresh
//! opponent from the match's level and gear tier, unless the last round was
//! lost and that fighter is dead: then the entrant at the place of the
//! losses comes in against the opponent who won. A tournament's next round
//! raises the bracket's next fight instead.
//!
//! Coming back to the hub writes the healths back, numbers the fighters and
//! books the result; on a tournament day with an entered event the unfit
//! are dropped from the entries instead of the day's match count rising,
//! and every tournament day reassigns the pending events' rivals.
//!
//! Evidence: the arena round result, end screen Continue, forfeit, next
//! round and return-to-hub routines, read from `Exanima.exe` 0.9.5.

use crate::arena_log::ArenaLog;
use crate::arena_match::{
    CompanyRecords, Continue, Finish, MatchState, continue_match, count_returned_match,
    health_word, return_to_hub,
};
use crate::arena_spawn::{GenericPlan, generic_plan};
use crate::match_layout::{NextRound, next_round};

/// The transition screen that opens the next round.
pub const SCREEN_NEXT_ROUND: u32 = 3;
/// The transition screen that opens the end-of-match menu.
pub const SCREEN_END: u32 = 2;
/// The next round waits this many milliseconds before it touches the scene.
pub const NEXT_ROUND_YIELD_MS: u32 = 13;
/// The saved placement's distance field, in centimetres.
pub const RETURN_DISTANCE: f32 = 300.0;
/// The saved placement's heading, a half turn.
pub const RETURN_HEADING: f32 = std::f32::consts::PI;
/// The saved placement's trailing flag word.
pub const RETURN_FLAGS: u32 = 0x100;

/// The transition a round's end asks for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Screen {
    /// Screen 3: the next round follows.
    NextRound,
    /// Screen 2: the match's end menu.
    End,
}

impl Screen {
    /// The transition a round's end opens for the state it set: the next
    /// round while the state is the next fight, else the end menu.
    pub fn after(state: MatchState) -> Self {
        if state == MatchState::NextFight {
            Self::NextRound
        } else {
            Self::End
        }
    }

    /// The screen's word.
    pub fn word(self) -> u32 {
        match self {
            Self::NextRound => SCREEN_NEXT_ROUND,
            Self::End => SCREEN_END,
        }
    }
}

/// What the arena's round result did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RoundEnding {
    /// The player's side won the round.
    pub won: bool,
    /// The session state the result wrote.
    pub state: MatchState,
    /// The transition screen the result opened.
    pub screen: Screen,
    /// Experience is awarded whatever the result.
    pub award_experience: bool,
    /// The end screen's Continue runs at once, without waiting for a
    /// press. It follows an auto-continue switch the game never sets, so
    /// in play this is always false.
    pub run_end_handler: bool,
}

/// End a round and choose its transition.
///
/// A win continues while the round less the losses is below the extra
/// rounds, else the match is won. A loss counts one more loss and continues
/// while the losses are below the lives, else the match is lost. The
/// transition is 3 while the state is the next fight and 2 otherwise. With
/// the auto-continue switch on (the game never sets it) a match's end runs
/// the end screen's Continue at once.
pub fn round_ending(
    won: bool,
    rounds: &mut crate::arena_match::MatchRounds,
    auto_continue: bool,
) -> RoundEnding {
    let state = rounds.finish_round(won);
    RoundEnding {
        won,
        state,
        screen: Screen::after(state),
        award_experience: true,
        run_end_handler: auto_continue && state != MatchState::NextFight,
    }
}

/// Where the return saves the manager: 300 cm out, a half turn round, flag
/// 0x100. The three field meanings are read from the values, not traced.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ReturnPlacement {
    pub distance: f32,
    pub heading: f32,
    pub flags: u32,
    /// The player data is saved before the hub opens.
    pub saves: bool,
}

/// The placement the match's return writes.
pub const RETURN_PLACEMENT: ReturnPlacement = ReturnPlacement {
    distance: RETURN_DISTANCE,
    heading: RETURN_HEADING,
    flags: RETURN_FLAGS,
    saves: true,
};

/// What the end screen's Continue does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EndAction {
    /// The manager is dead: the main menu, not the hub.
    MainMenu,
    /// Another round follows. The round counter rose by one. A tournament
    /// (`bracket`) raises the bracket's next fight, any other match the
    /// next round's bodies ([`regenerate_round`]).
    NextRound { round: i32, bracket: bool },
    /// The match is settled and the hub opens.
    ReturnToHub { tournament: bool },
}

/// The end screen's Continue.
///
/// A dead manager goes to the main menu without settling. Another round
/// follows while the state is the next fight and the round less the losses
/// is below the extra rounds ([`continue_match`]); the round counter rises.
/// Anything else settles the match (the bracket is resolved for a
/// tournament) and returns to the hub.
pub fn end_action(
    manager_dead: bool,
    state: MatchState,
    rounds: &crate::arena_match::MatchRounds,
    tournament: bool,
) -> EndAction {
    let mut next = *rounds;
    match continue_match(&mut next, state, manager_dead) {
        Continue::MainMenu => EndAction::MainMenu,
        Continue::NextRound => EndAction::NextRound {
            round: next.round,
            bracket: tournament,
        },
        Continue::Hub => EndAction::ReturnToHub { tournament },
    }
}

/// What the forfeit leaves behind.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ForfeitResult {
    /// The manager was already down: the forfeit is ignored.
    Ignored,
    /// The healths are written back, the falls resolved and the player data
    /// saved. The bracket is resolved before the save in a tournament.
    Return { tournament: bool },
}

/// Forfeit during a match. A dead manager makes it a no-op; any other
/// forfeit resolves the falls first, writes the healths back and saves only
/// the player data.
pub fn forfeit(manager_dead: bool, tournament: bool) -> ForfeitResult {
    if manager_dead {
        ForfeitResult::Ignored
    } else {
        ForfeitResult::Return { tournament }
    }
}

/// The bodies of the next round of a multi-round match.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RoundFight {
    /// The company's side: its current fighter or the reserve entrant.
    pub company: NextRound,
    /// The opponent generated for the round, at the match's level and gear
    /// tier; `None` when the surviving opponent fights on.
    pub opponent: Option<GenericPlan>,
    /// The surviving opponent is cleared and stood again for the round
    /// (the reserve entrant meets the opponent who won).
    pub keeps_opponent: bool,
    /// The scene work starts after this many milliseconds.
    pub yield_ms: u32,
}

/// The next round's bodies.
///
/// While no round is lost, or the company's fighter is not dead, that
/// fighter is stood again and the opponent is dropped: a fresh generic
/// opponent is generated at the match's level and gear tier. When a round
/// was lost and the fighter is dead, the entrant at the place of the losses
/// comes in and the opponent who won stays, cleared and stood again; only a
/// round without an opponent body generates a fresh one.
pub fn regenerate_round(
    level: i32,
    tier: i32,
    losses: i32,
    fighter_dead: bool,
    opponent_present: bool,
) -> RoundFight {
    let company = next_round(losses, fighter_dead);
    let keeps_opponent = matches!(company, NextRound::Reserve(_)) && opponent_present;
    RoundFight {
        company,
        opponent: (!keeps_opponent).then(|| generic_plan(level, tier)),
        keeps_opponent,
        yield_ms: NEXT_ROUND_YIELD_MS,
    }
}

/// Resolve every company body whose health fell under 0.01 and was not
/// already resolved, before the end screen or the return to the hub.
///
/// The manager's fatal fall is the permadeath branch; a hired fighter's is
/// logged and counted, or logged as saved when the fall was not fatal. The
/// body is marked resolved either way. The returned indices are the bodies
/// this pass resolved, in scene order.
pub fn resolve_dead_bodies(
    fighters: &mut [crate::arena_match::SceneFighter],
    physician: bool,
    log: &mut ArenaLog,
    deaths: &mut u32,
) -> Vec<usize> {
    let mut resolved = Vec::new();
    for (index, fighter) in fighters.iter_mut().enumerate() {
        // The health is widened to double before the 0.01 compare, so a
        // single 0.01 counts as below it; NaN never does.
        if !fighter.company
            || fighter.resolved
            || f64::from(fighter.health) >= crate::arena_match::DEAD_HEALTH
            || fighter.health.is_nan()
        {
            continue;
        }
        let fall =
            crate::arena_match::resolve_fall(fighter.health, fighter.max, physician, fighter.id);
        crate::arena_match::log_fall(log, fall, fighter.id, fighter.name, deaths);
        fighter.resolved = true;
        resolved.push(index);
    }
    resolved
}

/// One body whose health the return writes back into its record.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ReturnFighter {
    /// The body carries the match-participant flag: a company fighter.
    pub company: bool,
    /// The arena index: 0 is the manager, `n` the company roster's member
    /// `n - 1`.
    pub id: u32,
    pub health: f32,
    pub max: f32,
    /// The record's health word after the write.
    pub word: Option<u16>,
}

/// What the return to the hub did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MatchReturn {
    /// Copper the win paid.
    pub paid: u32,
    /// Company bodies whose health words were written.
    pub healths_written: usize,
    /// The day's tournament work.
    pub day: HubDayChecks,
    /// The day's match count after the return.
    pub matches_today: u32,
}

/// Come back to the hub after a match.
///
/// Every company body's health is written to its record as a word (0 at no
/// health, else the share of the maximum times 65535 rounded to even). The
/// result is booked: outside a tournament a win pays the prize and logs it,
/// a loss and a forfeit log theirs; the records count the match. On a
/// tournament day the day's checks run: with an entered event the unfit are
/// dropped and the pending events' rivals reassigned, otherwise the day's
/// match count rises.
#[allow(clippy::too_many_arguments)]
pub fn settle_return(
    finish: Finish,
    template: i32,
    tournament: bool,
    prize: u32,
    fighters: &mut [ReturnFighter],
    records: &mut CompanyRecords,
    log: &mut ArenaLog,
    day: u32,
    any_entry: bool,
    matches_today: u32,
) -> MatchReturn {
    let mut written = 0;
    for fighter in fighters.iter_mut().filter(|fighter| fighter.company) {
        fighter.word = Some(health_word(fighter.health, fighter.max));
        written += 1;
    }
    let paid = return_to_hub(records, log, finish, template, tournament, prize).prize;
    let checks = hub_day_checks(day, any_entry);
    let mut matches_today = matches_today;
    count_returned_match(day, any_entry, &mut matches_today);
    MatchReturn {
        paid,
        healths_written: written,
        day: checks,
        matches_today,
    }
}

/// Write the returned health words into the career's records: index 0 is
/// the manager, every other index the roster member one less. The original
/// writes each participant record's two health words before it numbers the
/// fighters and books the result; Exarust keeps one health word per member,
/// so both are the same value. Bodies with no record (a roster slot the
/// career no longer has) are left alone. Returns how many words were
/// written.
pub fn write_back_health(
    career: &mut crate::hub_windows::ArenaCareer,
    fighters: &[ReturnFighter],
) -> usize {
    let mut written = 0;
    for fighter in fighters {
        if !fighter.company {
            continue;
        }
        let Some(word) = fighter.word else {
            continue;
        };
        if fighter.id == 0 {
            career.player.health = word;
        } else {
            let Some(member) = career.company.roster.get_mut(fighter.id as usize - 1) else {
                continue;
            };
            member.health = word;
        }
        written += 1;
    }
    written
}

/// The participant list a saved lineup starts with.
///
/// The list has the match's participant count. It is filled from the
/// employees after the manager (index 0), taking every combatant — a class
/// below 4 — in order until the list is full. A short staff leaves the rest
/// zeroed, as the original's uninitialised slots would.
pub fn meet_participants(count: usize, staff: &[(i32, u8)]) -> Vec<i32> {
    let mut list = vec![0; count];
    let mut filled = 0;
    for &(id, class) in staff.iter().skip(1) {
        if filled >= count {
            break;
        }
        if class < 4 {
            list[filled] = id;
            filled += 1;
        }
    }
    list
}

/// What arriving at the hub (and the after-load hook) does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HubFixups {
    /// A physician is employed: clear the weapon object at its spot.
    pub clear_physician_weapon: bool,
    /// The manager and fighters are numbered again.
    pub number_fighters: bool,
    /// No tournament was generated yet: generate one.
    pub generate_tournament: bool,
    /// Drop the unfit entrants.
    pub drop_unfit: bool,
    /// The hub's session hook is installed after the fixups.
    pub install_session_hook: bool,
}

/// The hub's after-load fixups. The first physician in the staff clears the
/// weapon object at the physician's spot; the manager is restored and the
/// fighters renumbered; a missing tournament is generated; the unfit
/// entrants are dropped; and the session hook is installed.
pub fn hub_arrival(has_physician: bool, has_tournament: bool) -> HubFixups {
    HubFixups {
        clear_physician_weapon: has_physician,
        number_fighters: true,
        generate_tournament: !has_tournament,
        drop_unfit: true,
        install_session_hook: true,
    }
}

/// The hub-return day's tournament work.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HubDayChecks {
    /// A tournament day with an entered event drops the unfit entrants.
    pub drop_unfit: bool,
    /// A tournament day reassigns the pending events' rival team ids.
    pub assign_round_ids: bool,
    /// The match counter rises unless the day is a tournament day with an
    /// entered event.
    pub count_match: bool,
}

/// The return's tournament checks for a day and whether any event has an
/// entrant. On a multiple of seven with an entry the unfit are dropped and
/// the rival ids reassigned; without an entry, and on any other day, the
/// match counter rises instead.
pub fn hub_day_checks(day: u32, any_entry: bool) -> HubDayChecks {
    let tournament_day = day.is_multiple_of(7);
    HubDayChecks {
        drop_unfit: tournament_day && any_entry,
        assign_round_ids: tournament_day,
        count_match: !(tournament_day && any_entry),
    }
}

/// A tournament event's bracket and its place in the event.
///
/// The slots are the original's sixteen-byte team records flattened to one
/// entry per fighter slot: `team_size` consecutive slots hold a team's id,
/// and the group of `sides` teams fights each round. Starting an event fills
/// `sides << (rounds − 1)` teams with ids one and up (team one is the
/// player's side). A won round clears every other team of each group,
/// keeping the player's team in its own group, and the survivors become the
/// next fight's sides.
#[derive(Clone, Debug, PartialEq)]
pub struct BracketState {
    pub slots: Vec<crate::career::BracketSlot>,
    /// Fighters per team (the event's `teams` field).
    pub team_size: i32,
    /// Teams in each round's group (the event's `fighters` field).
    pub sides: i32,
    /// The event's rounds, for the record.
    pub rounds: i32,
}

/// The bracket a tournament template starts with: its teams, team size and
/// rounds, team one being the player's. `None` for a standard match.
#[must_use]
pub fn bracket_for_template(template: i32) -> Option<BracketState> {
    let template = crate::arena::TOURNAMENTS
        .iter()
        .find(|entry| i32::try_from(entry.id).ok() == Some(template))?;
    if template.rounds == 0 || template.teams == 0 || template.fighters == 0 {
        return None;
    }
    let entries = crate::tournament::fill_bracket(
        template.teams as i32,
        template.fighters as i32,
        template.rounds as i32,
        1,
    );
    Some(BracketState {
        slots: entries
            .into_iter()
            .map(|entry| crate::career::BracketSlot {
                id: entry.id,
                strength: entry.strength,
            })
            .collect(),
        team_size: template.teams as i32,
        sides: template.fighters as i32,
        rounds: template.rounds as i32,
    })
}

/// Resolve the bracket's round and return the surviving team ids in slot
/// order. The group the player's team is in keeps `forced` (team one on a
/// won fight); every other group draws its winner with one step of `roll`.
pub fn bracket_next_fight(bracket: &mut BracketState, forced: i32, roll: &mut u32) -> Vec<i32> {
    crate::career::clear_losing_bracket(
        &mut bracket.slots,
        bracket.team_size,
        bracket.sides,
        forced,
        roll,
    );
    bracket.slots.iter().map(|slot| slot.id).collect()
}

/// A tournament's bracket at the match's end or forfeit: the scene draw's
/// winner (`winner`, [`crate::career::draw_unwatched`]) keeps the first
/// group, every other group draws its own, and the brackets are cleared
/// again with no forced team while a pass still resolves more than one
/// group; then the first event with no winner takes the first surviving
/// team ([`crate::tournament::settle_first_undecided`]). All draws step the
/// one shared `roll`.
///
/// [guess] With no team standing the original reads an unset result slot;
/// Exarust forces no team then (0).
pub fn finish_bracket(
    bracket: &mut BracketState,
    winner: Option<i32>,
    events: &mut crate::arena::Roster,
    roll: &mut u32,
) {
    let mut forced = winner.unwrap_or(0);
    while crate::career::clear_losing_bracket(
        &mut bracket.slots,
        bracket.team_size,
        bracket.sides,
        forced,
        roll,
    ) {
        forced = 0;
    }
    let mut winners = events.map(|row| row.team_id);
    let field: Vec<u32> = bracket.slots.iter().map(|slot| slot.id as u32).collect();
    crate::tournament::settle_first_undecided(&mut winners, &field, roll);
    for (row, winner) in events.iter_mut().zip(winners) {
        row.team_id = winner;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::arena_match::{MatchRounds, MatchState};

    #[test]
    fn the_end_clears_every_bracket_and_names_the_first_open_event() {
        let slot = |id, strength| crate::career::BracketSlot { id, strength };
        let mut bracket = BracketState {
            slots: vec![slot(5, 4.0), slot(9, 10.0), slot(3, 4.0), slot(7, 2.0)],
            team_size: 1,
            sides: 2,
            rounds: 2,
        };
        let mut events = crate::arena::Roster::default();
        events[0].team_id = 2;
        let mut roll = 1;
        finish_bracket(&mut bracket, Some(5), &mut events, &mut roll);
        // The first pass keeps 5 and draws 3 over 7; the second, unforced,
        // draws 3 over 5 (pick 172 of 200 passes the first 100).
        let ids: Vec<i32> = bracket.slots.iter().map(|slot| slot.id).collect();
        assert_eq!(ids, [0, 0, 3, 0]);
        // The first open event steps the roll once more and takes team 3.
        assert_eq!(roll, 0x33dc_589c);
        assert_eq!(events.map(|row| row.team_id), [2, 3, 0, 0, 0, 0]);
    }

    #[test]
    fn a_won_round_opens_the_next_fight_or_the_end_menu() {
        let mut rounds = MatchRounds::elimination();
        for round in 0..4 {
            let ending = round_ending(true, &mut rounds, true);
            assert_eq!(ending.state, MatchState::NextFight, "{round}");
            assert_eq!(ending.screen, Screen::NextRound);
            assert_eq!(ending.screen.word(), 3);
            assert!(ending.award_experience);
            assert!(!ending.run_end_handler);
            // The round counter rises in the end screen's Continue.
            rounds.round = rounds.round.wrapping_add(1);
        }
        let ending = round_ending(true, &mut rounds, true);
        assert_eq!(ending.state, MatchState::Won);
        assert_eq!(ending.screen, Screen::End);
        assert!(ending.run_end_handler);
    }

    #[test]
    fn a_lost_round_sets_the_lost_state_and_the_end_screen() {
        let mut rounds = MatchRounds::default();
        let ending = round_ending(false, &mut rounds, false);
        assert_eq!(ending.state, MatchState::Lost);
        assert_eq!(ending.screen, Screen::End);
        assert!(!ending.run_end_handler);
        assert_eq!(rounds.losses, 1);
        // An AI-only match never runs the end handler.
        let mut rounds = MatchRounds::reserve();
        for _ in 0..2 {
            assert!(!round_ending(false, &mut rounds, false).run_end_handler);
        }
        assert_eq!(rounds.losses, 2);
        let last = round_ending(false, &mut rounds, true);
        assert_eq!(last.state, MatchState::Lost);
        assert!(last.run_end_handler);
    }

    #[test]
    fn continue_branches_on_the_manager_and_the_round_count() {
        let mut rounds = MatchRounds::elimination();
        rounds.round = 3;
        assert_eq!(
            end_action(false, MatchState::NextFight, &rounds, false),
            EndAction::NextRound {
                round: 4,
                bracket: false
            }
        );
        assert_eq!(
            end_action(true, MatchState::NextFight, &rounds, false),
            EndAction::MainMenu
        );
        assert_eq!(
            end_action(false, MatchState::Won, &rounds, true),
            EndAction::ReturnToHub { tournament: true }
        );
        // A tournament's next round raises the bracket's next fight.
        assert_eq!(
            end_action(false, MatchState::NextFight, &rounds, true),
            EndAction::NextRound {
                round: 4,
                bracket: true
            }
        );
        rounds.round = 4;
        assert_eq!(
            end_action(false, MatchState::Won, &rounds, false),
            EndAction::ReturnToHub { tournament: false }
        );
        // The next-fight state alone is not enough when the count is done.
        assert_eq!(
            end_action(false, MatchState::NextFight, &rounds, false),
            EndAction::ReturnToHub { tournament: false }
        );
    }

    #[test]
    #[allow(clippy::assertions_on_constants)]
    fn the_return_saves_the_managers_placement() {
        assert_eq!(RETURN_PLACEMENT.distance, 300.0);
        assert_eq!(RETURN_PLACEMENT.heading.to_bits(), 0x4049_0fdb);
        assert_eq!(RETURN_PLACEMENT.flags, 0x100);
        assert!(RETURN_PLACEMENT.saves);
    }

    #[test]
    fn a_dead_manager_ignores_the_forfeit() {
        assert_eq!(forfeit(true, false), ForfeitResult::Ignored);
        assert_eq!(
            forfeit(false, true),
            ForfeitResult::Return { tournament: true }
        );
        assert_eq!(
            forfeit(false, false),
            ForfeitResult::Return { tournament: false }
        );
    }

    #[test]
    fn the_next_round_takes_a_fresh_opponent_and_keeps_the_fighter() {
        let fight = regenerate_round(4, 2, 0, true, true);
        assert_eq!(fight.company, NextRound::FreshOpponent);
        let opponent = fight.opponent.unwrap();
        assert_eq!(opponent.level, 4);
        assert_eq!(opponent.outfit_level, 2);
        assert!(!fight.keeps_opponent);
        assert_eq!(fight.yield_ms, 13);
        // A loss with a dead fighter brings the reserve entrant in against
        // the opponent who won.
        let fight = regenerate_round(3, 1, 2, true, true);
        assert_eq!(fight.company, NextRound::Reserve(2));
        assert!(fight.keeps_opponent);
        assert_eq!(fight.opponent, None);
        // Without an opponent body a fresh one is generated.
        let fight = regenerate_round(3, 1, 2, true, false);
        assert!(!fight.keeps_opponent);
        assert_eq!(fight.opponent.unwrap().level, 3);
        // A loss with the fighter alive keeps it and draws a fresh opponent.
        let fight = regenerate_round(3, 1, 2, false, true);
        assert_eq!(fight.company, NextRound::FreshOpponent);
        assert!(fight.opponent.is_some());
    }

    #[test]
    fn the_end_screens_pass_resolves_each_fallen_company_body_once() {
        use crate::arena_match::{LOG_SAVED, SceneFighter};
        let mut bytes = [0u8; 16];
        bytes[..3].copy_from_slice(b"Ash");
        let mut scene = [
            SceneFighter {
                team: 1,
                dead: true,
                company: true,
                resolved: false,
                health: -0.1,
                max: 0.25,
                id: 2,
                name: Some(bytes),
                manager: false,
            },
            SceneFighter {
                team: 1,
                dead: true,
                company: true,
                resolved: true,
                health: 0.0,
                max: 0.25,
                id: 3,
                name: None,
                manager: false,
            },
            SceneFighter {
                team: 1,
                dead: true,
                company: true,
                resolved: false,
                health: 0.005,
                max: 0.25,
                id: 4,
                name: None,
                manager: false,
            },
        ];
        let mut log = ArenaLog::default();
        let mut deaths = 0;
        let resolved = resolve_dead_bodies(&mut scene, true, &mut log, &mut deaths);
        assert_eq!(resolved, vec![0, 2]);
        assert_eq!(deaths, 1);
        assert_eq!(log.records.len(), 2);
        assert_eq!(log.records[1].id, LOG_SAVED);
        assert!(scene.iter().all(|fighter| fighter.resolved));
        // A second pass resolves nobody.
        assert!(resolve_dead_bodies(&mut scene, true, &mut log, &mut deaths).is_empty());
        // The single 0.01 widens below the double 0.01: the body is resolved
        // (saved, not dead, with a physician); NaN never is.
        let mut edge = [scene[2], scene[2]];
        edge[0].resolved = false;
        edge[0].health = 0.01;
        edge[1].resolved = false;
        edge[1].health = f32::NAN;
        let resolved = resolve_dead_bodies(&mut edge, true, &mut log, &mut deaths);
        assert_eq!(resolved, vec![0]);
        assert_eq!(log.records.last().unwrap().id, LOG_SAVED);
    }

    #[test]
    fn the_return_writes_healths_books_the_result_and_counts_the_day() {
        let mut fighters = [
            ReturnFighter {
                company: true,
                id: 1,
                health: 0.25,
                max: 0.5,
                word: None,
            },
            ReturnFighter {
                company: false,
                id: 2,
                health: 0.1,
                max: 0.5,
                word: None,
            },
        ];
        let mut records = CompanyRecords::default();
        let mut log = ArenaLog::default();
        let out = settle_return(
            Finish::Won,
            1,
            false,
            12,
            &mut fighters,
            &mut records,
            &mut log,
            3,
            false,
            1,
        );
        assert_eq!(out.paid, 12);
        assert_eq!(out.healths_written, 1);
        assert_eq!(fighters[0].word, Some(32_768));
        assert_eq!(fighters[1].word, None);
        assert_eq!(records.matches_won, 1);
        assert_eq!(log.records.len(), 1);
        assert_eq!(out.matches_today, 2);
        // A tournament day with an entry keeps the count and drops the
        // unfit instead; the tournament itself is not booked in the log.
        let mut records = CompanyRecords::default();
        let mut log = ArenaLog::default();
        let out = settle_return(
            Finish::Won,
            9,
            true,
            0,
            &mut [],
            &mut records,
            &mut log,
            7,
            true,
            1,
        );
        assert!(out.day.drop_unfit);
        assert!(out.day.assign_round_ids);
        assert_eq!(out.matches_today, 1);
        assert_eq!(out.paid, 0);
        assert!(log.records.is_empty());
    }

    #[test]
    fn the_health_words_go_back_into_the_manager_and_the_roster() {
        let mut career = crate::hub_windows::ArenaCareer::default();
        for name in ["Ash", "Bryn"] {
            career.company.roster.push(crate::career::Member {
                name: name.to_string(),
                profession: crate::career::Profession::Combatant,
                xp: 0,
                skills: [0; 6],
                skill_xp: [0; 6],
                techniques: [[0; 6]; 6],
                loadouts: Default::default(),
                health: u16::MAX,
                in_entry: false,
            });
        }
        let mut fighters = [
            ReturnFighter {
                company: true,
                id: 0,
                health: 0.25,
                max: 0.5,
                word: None,
            },
            ReturnFighter {
                company: true,
                id: 2,
                health: 0.0,
                max: 0.5,
                word: None,
            },
            // A body from a roster slot the career no longer has writes
            // nothing.
            ReturnFighter {
                company: true,
                id: 9,
                health: 0.1,
                max: 0.5,
                word: None,
            },
        ];
        let mut records = CompanyRecords::default();
        let mut log = ArenaLog::default();
        settle_return(
            Finish::Lost,
            1,
            false,
            0,
            &mut fighters,
            &mut records,
            &mut log,
            3,
            false,
            0,
        );
        assert_eq!(fighters[0].word, Some(32_768));
        let written = write_back_health(&mut career, &fighters);
        assert_eq!(written, 2);
        assert_eq!(career.player.health, 32_768);
        assert_eq!(career.company.roster[0].health, u16::MAX);
        assert_eq!(career.company.roster[1].health, 0);
    }

    #[test]
    fn a_saved_lineup_takes_the_combatants_after_the_manager() {
        let staff = [(9, 0), (1, 0), (2, 4), (3, 1), (4, 0)];
        assert_eq!(meet_participants(3, &staff), vec![1, 3, 4]);
        assert_eq!(meet_participants(5, &staff), vec![1, 3, 4, 0, 0]);
        assert_eq!(meet_participants(0, &staff), Vec::<i32>::new());
        // The manager is never taken even when no one else is on the books.
        assert_eq!(meet_participants(1, &[(9, 0)]), vec![0]);
    }

    #[test]
    fn the_hub_arrival_clears_the_physician_weapon_and_fills_the_gaps() {
        assert_eq!(
            hub_arrival(true, false),
            HubFixups {
                clear_physician_weapon: true,
                number_fighters: true,
                generate_tournament: true,
                drop_unfit: true,
                install_session_hook: true
            }
        );
        assert!(!hub_arrival(false, true).clear_physician_weapon);
        assert!(!hub_arrival(false, true).generate_tournament);
    }

    #[test]
    fn a_tournament_bracket_starts_with_team_one_and_eight_pairs() {
        let bracket = bracket_for_template(7).expect("Elimination is a tournament");
        assert_eq!(bracket.team_size, 1);
        assert_eq!(bracket.sides, 2);
        assert_eq!(bracket.rounds, 4);
        assert_eq!(bracket.slots.len(), 16);
        assert_eq!(
            bracket.slots.iter().map(|slot| slot.id).collect::<Vec<_>>(),
            (1..=16).collect::<Vec<_>>()
        );
        assert!(bracket.slots.iter().all(|slot| slot.strength == 1.0));
        // A standard match has no bracket.
        assert!(bracket_for_template(1).is_none());
        // The doubles event pairs two slots per team.
        let doubles = bracket_for_template(16).unwrap();
        assert_eq!(doubles.team_size, 2);
        assert_eq!(doubles.slots.len(), 16);
        assert_eq!(
            &doubles.slots[..4].iter().map(|s| s.id).collect::<Vec<_>>(),
            &[1, 1, 2, 2]
        );
    }

    #[test]
    fn a_won_bracket_round_keeps_the_player_and_draws_the_other_pairs() {
        let mut bracket = bracket_for_template(7).unwrap();
        let mut roll = 0;
        let survivors = bracket_next_fight(&mut bracket, 1, &mut roll);
        assert_eq!(
            survivors,
            [1, 0, 3, 0, 5, 0, 0, 8, 9, 0, 11, 0, 0, 14, 15, 0]
        );
        assert_eq!(roll, 0x5195_c04b);
        assert_eq!(survivors.len(), 16);
        // Player team one beat team two; the other seven pairs draw a winner
        // each with the original's roll sequence.
        assert_eq!(survivors[0], 1);
        assert_eq!(survivors[1], 0);
        assert_eq!(survivors[2], 3);
        assert_eq!(survivors[3], 0);
        assert!(survivors.iter().filter(|&&id| id != 0).count() == 8);
        assert_ne!(roll, 0);
        // A second round groups the survivors and keeps team one again.
        let again = bracket_next_fight(&mut bracket, 1, &mut roll);
        assert_eq!(again, [1, 0, 0, 0, 5, 0, 0, 0, 9, 0, 0, 0, 0, 14, 0, 0]);
        assert_eq!(roll, 0x6cf8_f4be);
        assert_eq!(again.iter().filter(|&&id| id != 0).count(), 4);
        assert_eq!(again[0], 1);
    }

    #[test]
    fn the_returns_day_checks_follow_the_calendar() {
        assert_eq!(
            hub_day_checks(7, true),
            HubDayChecks {
                drop_unfit: true,
                assign_round_ids: true,
                count_match: false
            }
        );
        assert_eq!(
            hub_day_checks(14, false),
            HubDayChecks {
                drop_unfit: false,
                assign_round_ids: true,
                count_match: true
            }
        );
        assert_eq!(
            hub_day_checks(3, true),
            HubDayChecks {
                drop_unfit: false,
                assign_round_ids: false,
                count_match: true
            }
        );
    }
}
