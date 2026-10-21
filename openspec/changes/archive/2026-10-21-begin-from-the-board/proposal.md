# Proposal

## Why

The board's Begin picks the week's next tournament event, warns once that
the manager is entered, and the manage window only lets the day end once
it has been fought in (or, on a tournament day, once every entered event
is decided). The company tables draw each fighter's health as a bar.
These gates are not in.

## What Changes

- The next tournament event: the first with no winner and an entrant.
- Whether an event is pending on a tournament day, whether any event has
  entrants, and whether the day may end.
- The manager warning's one-shot flag.
- The company tables' health bar share.

## Capabilities

### New Capabilities

### Modified Capabilities

- `arena-career`: board and day gates.

## Impact

- `exa_game::arena_match::{next_event, any_entry, event_pending,
  day_can_end, warn_manager, health_bar}`.
- Evidence: REA/Ghidra on `Exanima.exe` 0.9.5 (the board's Begin, the three
  day gates, the health cell). Notes: `context/re/arena-match.md`.
- Out of scope: the board and manage windows, the participants' mapping
  from the board's entry list, the tip window.

## Phase

Phase 7, M7 "arena season": running the board.
