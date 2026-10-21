# Proposal

## Why

The career prices a hire and a candidate, but the staff itself is not in:
who may be hired, what a hire takes and gives, a recruited fighter of one's
own, what a dismissal, a death or a departure undoes, the ids the staff
gives its fighters, and the manage window's daily report that tells the
player all of it.

## What Changes

- The staff: classes (recruit, combatant, merchant, physician, trainer),
  fighters counted with the manager, ids renumbered by place, the control
  flag for recruits, the manage table's order list and its quirks.
- A hire: the purse, room and duplicate checks with their messages, the
  price paid into today's spending, the candidate leaving the list, the hire
  count, the joining log, a combatant's kit (hand and worn items of six
  loadouts, item 8 left out, once each) stowed with 0x400, a specialist's
  items with 0x200.
- A recruitment for 12 copper without a room check.
- A leaving: the arsenal flags released, a merchant's stock emptied, the
  id dropped from the tournament entries and from the log's saves, later ids
  moved down; a dismissal logs and counts one more fired.
- The daily report: the day's title, each log record's line, the fighters
  ready to train and their ranks, the specialists on the hire list, the days
  to the tournament.

## Capabilities

### New Capabilities

### Modified Capabilities

- `arena-career`: staff, hire, recruitment, leaving, dismissal, daily
  report.

## Impact

- New `exa_game::{staff, report}`.
- Docs: `docs/formats/OTHER.md` (Arena tables), `crates/exa_game/README.md`.
- Evidence: REA/Ghidra on `Exanima.exe` 0.9.5 (hire, recruitment, renumber,
  kit, remove, dismiss, the manage window refresh and its report, the match
  phrase, the vowel test, the rank names, the training cap). Notes:
  `context/re/arena-staff.md`.
- Out of scope: the manage window, the hire list window and their widgets;
  placing a merchant's or physician's figure in the hub; moving a dismissed
  fighter's items into the chosen arsenal box; the world character
  registration; the employee and hire-list save chunks.

## Phase

Phase 7, M7 "arena season": running the company between matches.
