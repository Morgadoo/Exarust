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
//! The next round clears the opponent's body and creates a fresh opponent
//! from the match's level and gear tier; the company's side keeps the same
//! fighter unless the last round was lost and that fighter is dead, in which
//! case the entrant at the place of the losses comes in. A spare opponent
//! is created for the round after.
//!
//! Coming back to the hub writes the healths back, numbers the fighters and
//! books the result; on a tournament day with an entered event the unfit
//! are dropped from the entries instead of the day's match count rising,
//! and every tournament day reassigns the pending events' rivals.
//!
//! Evidence: the arena round result, end screen Continue, forfeit, next
//! round and return-to-hub routines, read from `Exanima.exe` 0.9.5.

use crate::arena_log::ArenaLog;
use crate::arena_match::{CompanyRecords, Finish, MatchState, health_word, return_to_hub};
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
    /// Only a player-controlled match runs the end screen's handler.
    pub run_end_handler: bool,
}

/// End a round and choose its transition.
///
/// A win continues while the round less the losses is below the extra
/// rounds, else the match is won. A loss counts one more loss and continues
/// while the losses are below the lives, else the match is lost. The
/// transition is 3 while the state is the next fight and 2 otherwise. The
/// end handler runs only for a player-controlled match whose state is not
/// the next fight.
pub fn round_ending(
    won: bool,
    rounds: &mut crate::arena_match::MatchRounds,
    player_controls: bool,
) -> RoundEnding {
    let state = rounds.finish_round(won);
    let screen = if state == MatchState::NextFight {
        Screen::NextRound
    } else {
        Screen::End
    };
    RoundEnding {
        won,
        state,
        screen,
        award_experience: true,
        run_end_handler: player_controls && state != MatchState::NextFight,
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
    /// Another round follows. The round counter rose by one.
    NextRound { round: i32 },
    /// The match is settled and the hub opens.
    ReturnToHub { tournament: bool },
}

/// The end screen's Continue.
///
/// A dead manager goes to the main menu without settling. Another round
/// follows while the state is the next fight and the round less the losses
/// is below the extra rounds; the round counter rises. Anything else
/// settles the match (the bracket is resolved for a tournament) and returns
/// to the hub.
pub fn end_action(
    manager_dead: bool,
    state: MatchState,
    rounds: &crate::arena_match::MatchRounds,
    tournament: bool,
) -> EndAction {
    if manager_dead {
        return EndAction::MainMenu;
    }
    let ahead = i64::from(rounds.round) - i64::from(rounds.losses);
    if state == MatchState::NextFight && ahead < i64::from(rounds.extra_rounds) {
        EndAction::NextRound {
            round: rounds.round.wrapping_add(1),
        }
    } else {
        EndAction::ReturnToHub { tournament }
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

/// The fighter who fights the next round of a multi-round match.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RoundFight {
    /// The company's side: its current fighter or the reserve entrant.
    pub company: NextRound,
    /// The opponent created for the round.
    pub opponent: GenericPlan,
    /// The previous opponent's body is cleared before the new one comes.
    pub clears_opponent: bool,
    /// A spare opponent is created for the round after this one.
    pub spare_opponent: bool,
    /// The scene work starts after this many milliseconds.
    pub yield_ms: u32,
}

/// The next round's bodies.
///
/// The company keeps its fighter unless the last round was lost and that
/// fighter is dead (then the entrant at the place of the losses comes in).
/// A fresh opponent is generated from the match's level and gear tier, the
/// old one is cleared, and a spare is created for the next round.
pub fn regenerate_round(level: i32, tier: i32, losses: i32, fighter_dead: bool) -> RoundFight {
    RoundFight {
        company: next_round(losses, fighter_dead),
        opponent: generic_plan(level, tier),
        clears_opponent: true,
        spare_opponent: true,
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
        if !fighter.company || fighter.resolved || !(fighter.health < 0.01) {
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
    let matches_today = if checks.count_match {
        matches_today.wrapping_add(1)
    } else {
        matches_today
    };
    MatchReturn {
        paid,
        healths_written: written,
        day: checks,
        matches_today,
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::arena_match::{MatchRounds, MatchState};

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
            EndAction::NextRound { round: 4 }
        );
        assert_eq!(
            end_action(true, MatchState::NextFight, &rounds, false),
            EndAction::MainMenu
        );
        assert_eq!(
            end_action(false, MatchState::Won, &rounds, true),
            EndAction::ReturnToHub { tournament: true }
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
        let fight = regenerate_round(4, 2, 0, true);
        assert_eq!(fight.company, NextRound::FreshOpponent);
        assert_eq!(fight.opponent.level, 4);
        assert_eq!(fight.opponent.outfit_level, 2);
        assert!(fight.clears_opponent);
        assert!(fight.spare_opponent);
        assert_eq!(fight.yield_ms, 13);
        // A loss with a dead fighter brings the reserve entrant in.
        let fight = regenerate_round(3, 1, 2, true);
        assert_eq!(fight.company, NextRound::Reserve(2));
        // A loss with the fighter alive keeps it.
        assert_eq!(
            regenerate_round(3, 1, 2, false).company,
            NextRound::FreshOpponent
        );
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
    }

    #[test]
    fn the_return_writes_healths_books_the_result_and_counts_the_day() {
        let mut fighters = [
            ReturnFighter {
                company: true,
                health: 0.25,
                max: 0.5,
                word: None,
            },
            ReturnFighter {
                company: false,
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
    fn a_saved_lineup_takes_the_combatants_after_the_manager() {
        let staff = [(9, 0), (1, 0), (2, 4), (3, 1), (4, 0)];
        assert_eq!(meet_participants(3, &staff), vec![1, 3, 4]);
        assert_eq!(meet_participants(5, &staff), vec![1, 3, 4, 0, 0]);
        assert_eq!(meet_participants(0, &staff), Vec::<i32>::new());
        // The manager is never taken even when no one else is on the books.
        assert_eq!(meet_participants(1, &[(9, 0)]), vec![0]);
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
