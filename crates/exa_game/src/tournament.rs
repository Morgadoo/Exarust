//! The week's tournament as the hub starts it, and what arriving at the
//! hub sets up.
//!
//! Starting a tournament event first gives every earlier event the company
//! did not enter a winner: from the first event, one with no winner and no
//! entrant takes the company `2 + high(roll × 6)` (the roll stepped by
//! `× 0x08088405 + 1`), and the first event with no winner that has an
//! entrant stops the pass. The event then runs its rounds less one as extra
//! rounds from round 0, its bracket filled with `sides << (rounds − 1)`
//! teams of the event's team size, team `t` holding id `t + 1` (or `t + 2`
//! when the caller asks for it), each slot of strength 1 and its member
//! index. Forfeiting the entire tourney (after "Do you wish to forfeit the
//! entire tourney?") empties every event that has no winner.
//!
//! Arriving at the hub removes the physician's mark when a physician is on
//! the staff, takes the player's record as the manager's, renumbers the
//! staff, builds the tournament board when it holds no prize, drops the
//! unfit entrants and arms the hub's fall check. A rematch (the original's
//! automatic replay) restores the last match and enters as many of the
//! staff, from the second employee on, whose class is under 4 as the match
//! fields per team.
//!
//! Evidence: the retail tournament start, bracket fill, unentered events,
//! tourney forfeit, hub arrival and rematch routines read from
//! `Exanima.exe` (`context/re/arena-manage.md`).

use crate::arena::Roster;
use crate::career::BracketSlot;

/// The forfeit dialog's question.
pub const FORFEIT_TOURNEY: &str = "Do you wish to forfeit the entire tourney?";
/// The roll's step.
const STEP: u32 = 0x0808_8405;

/// One bracket slot: the team's id, its strength and its member index.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BracketEntry {
    pub id: i32,
    pub strength: f32,
    pub member: i32,
}

impl BracketEntry {
    /// The slot as the unwatched bracket reads it.
    pub fn slot(self) -> BracketSlot {
        BracketSlot {
            id: self.id,
            strength: self.strength,
        }
    }
}

/// Fill an event's bracket: `sides << (rounds − 1)` teams (a 32-bit shift
/// by the low five bits) of `team_size` slots, team `t` holding id
/// `t + base` (`base` 1 or 2).
pub fn fill_bracket(team_size: i32, sides: i32, rounds: i32, base: i32) -> Vec<BracketEntry> {
    let shift = (i64::from(rounds) - 1) as u32 & 31;
    let teams = (sides as u32).wrapping_shl(shift) as i32;
    let mut slots = Vec::new();
    for team in 0..teams {
        for member in 0..team_size {
            slots.push(BracketEntry {
                id: team + base,
                strength: 1.0,
                member,
            });
        }
    }
    slots
}

/// Give the events before the first one the company entered a winner: an
/// event with no winner and no entrant takes a company id of 2 to 7.
pub fn settle_unentered(rows: &mut Roster, roll: &mut u32) {
    for row in rows.iter_mut() {
        if row.team_id != 0 {
            continue;
        }
        if 0 < row.count as i32 {
            return;
        }
        *roll = roll.wrapping_mul(STEP).wrapping_add(1);
        row.team_id = ((u64::from(*roll) * 6) >> 32) as u32 + 2;
    }
}

/// A started tournament event.
#[derive(Clone, Debug, PartialEq)]
pub struct EventStart {
    /// The rounds after the first.
    pub extra_rounds: i32,
    /// The round under way.
    pub round: i32,
    pub bracket: Vec<BracketEntry>,
    /// The match is a tournament's.
    pub tournament: bool,
}

/// Start a tournament event of `team_size`, `sides` and `rounds`.
pub fn start_event(
    rows: &mut Roster,
    roll: &mut u32,
    team_size: i32,
    sides: i32,
    rounds: i32,
) -> EventStart {
    settle_unentered(rows, roll);
    EventStart {
        extra_rounds: rounds.wrapping_sub(1),
        round: 0,
        bracket: fill_bracket(team_size, sides, rounds, 1),
        tournament: true,
    }
}

/// Forfeit the entire tourney: every event with no winner loses its
/// entrants.
pub fn forfeit_tourney(rows: &mut Roster) {
    for row in rows.iter_mut() {
        if row.team_id == 0 {
            row.count = 0;
        }
    }
}

/// What arriving at the hub does beyond the staff's renumbering.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HubArrival {
    /// A physician is on the staff: his mark in the hub is removed.
    pub remove_physician_mark: bool,
    /// The tournament board holds no prize and is built.
    pub build_tournament: bool,
}

/// Arriving at the hub, from the staff's classes and the tournament
/// board's prize word.
pub fn arrive_at_hub(classes: &[u8], tournament_prize: u32) -> HubArrival {
    HubArrival {
        remove_physician_mark: classes.contains(&crate::staff::PHYSICIAN),
        build_tournament: tournament_prize == 0,
    }
}

/// A rematch's lineup: from the second employee on, those whose class is
/// under 4, until `wanted` are found or the staff ends (the original reads
/// on past the staff).
pub fn rematch_lineup(classes: &[u8], wanted: i32) -> Vec<usize> {
    let mut lineup = Vec::new();
    let mut index = 1;
    while (lineup.len() as i32) < wanted && index < classes.len() {
        if classes[index] < 4 {
            lineup.push(index);
        }
        index += 1;
    }
    lineup
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::arena::{Entrant, RosterRow, TOURNAMENTS};

    fn ids(slots: &[BracketEntry]) -> Vec<i32> {
        slots.iter().map(|s| s.id).collect()
    }

    #[test]
    fn brackets_hold_every_team_of_the_event() {
        let elimination = &TOURNAMENTS[0];
        let slots = fill_bracket(
            elimination.teams as i32,
            elimination.fighters as i32,
            elimination.rounds as i32,
            1,
        );
        assert_eq!(ids(&slots), (1..=16).collect::<Vec<_>>());
        assert!(slots.iter().all(|s| s.member == 0 && s.strength == 1.0));
        let conquest = &TOURNAMENTS[4];
        let slots = fill_bracket(
            conquest.teams as i32,
            conquest.fighters as i32,
            conquest.rounds as i32,
            2,
        );
        assert_eq!(slots.len(), 16);
        assert_eq!(&ids(&slots)[..4], &[2, 2, 3, 3]);
        assert_eq!(slots[3].member, 1);
        assert_eq!(slots[15].id, 9);
        let fray = fill_bracket(1, 6, 1, 1);
        assert_eq!(ids(&fray), [1, 2, 3, 4, 5, 6]);
        assert_eq!(
            fray[2].slot(),
            BracketSlot {
                id: 3,
                strength: 1.0
            }
        );
    }

    fn entered(count: u32, team_id: u32) -> RosterRow {
        RosterRow {
            count,
            entries: [Entrant::default(); 2],
            team_id,
        }
    }

    #[test]
    fn events_the_company_skipped_take_a_rival_winner() {
        let mut rows = [
            entered(0, 0),
            entered(0, 5),
            entered(0, 0),
            entered(1, 0),
            entered(0, 0),
            entered(0, 0),
        ];
        let mut roll = 1;
        settle_unentered(&mut rows, &mut roll);
        assert_eq!(rows.map(|r| r.team_id), [2, 5, 7, 0, 0, 0]);
        assert_eq!(roll, 0xdc6d_ac1f);
        let mut rows = [entered(0, 0); 6];
        let mut roll = 1;
        settle_unentered(&mut rows, &mut roll);
        assert_eq!(rows[2].team_id, 3);
        assert!(rows.iter().all(|r| (2..=7).contains(&r.team_id)));
    }

    #[test]
    fn an_event_starts_on_round_zero() {
        let mut rows = [entered(1, 0); 6];
        let mut roll = 1;
        let start = start_event(&mut rows, &mut roll, 1, 2, 4);
        assert_eq!(roll, 1);
        assert_eq!(
            (start.extra_rounds, start.round, start.tournament),
            (3, 0, true)
        );
        assert_eq!(start.bracket.len(), 16);
    }

    #[test]
    fn forfeiting_empties_the_open_events() {
        let mut rows = [
            entered(2, 0),
            entered(1, 4),
            entered(1, 0),
            entered(0, 0),
            entered(2, 1),
            entered(1, 0),
        ];
        forfeit_tourney(&mut rows);
        assert_eq!(rows.map(|r| r.count), [0, 1, 0, 0, 2, 0]);
    }

    #[test]
    fn the_hub_marks_the_physician_and_builds_the_board() {
        assert_eq!(
            arrive_at_hub(&[2, 8], 0),
            HubArrival {
                remove_physician_mark: true,
                build_tournament: true
            }
        );
        assert_eq!(
            arrive_at_hub(&[2, 4], 7),
            HubArrival {
                remove_physician_mark: false,
                build_tournament: false
            }
        );
    }

    #[test]
    fn a_rematch_skips_the_first_employee() {
        assert_eq!(rematch_lineup(&[2, 8, 1, 2, 2], 2), [2, 3]);
        assert_eq!(rematch_lineup(&[2, 2], 3), [1]);
        assert!(rematch_lineup(&[2, 2], 0).is_empty());
    }
}
