# Proposal

## Why

The arena boards are generated, but the roster rows, entrant records, opponent spawn profile, unfit filter, round-id assignment and weekly/seasonal resolution were missing. The decode gives their exact layouts and counters.

## What Changes

- `Entrant`, `RosterRow` and the six-row `Roster`.
- The 8-byte spawn profile with the armed/company flags, tier, three attribute bytes and the company's three build bytes.
- `drop_unfit` for pending rows below 0xCCC.
- `assign_round_ids` for empty pending rows, stopping at the first live row.
- `SeasonState` counters and `resolve_tournament`: tally by team, 3+ no-tie champion, week flags, season set `(day % 28) % 4`, championships/days/seasons.

## Capabilities

### New Capabilities

### Modified Capabilities

- `arena-career`: the roster and tournament resolution.

## Impact

- `exa_game::arena`, from `Entrant`, `RosterRow`, `Roster`, `spawn_profile`, `drop_unfit`, `assign_round_ids`, `SeasonState`, `Resolution` and `resolve_tournament`.
- `docs/PLAN.md` and the generated `docs/COMPONENT-MAP.md`: `XA_Arena` becomes **28%** in the source rating. The headline rises.
- Evidence: the 0x30 row and 0x14 entrant layouts, `0x10018ac20` (drop), `0x10018c270` (round ids), `0x100192b90` (resolution), `0x100185ef0` (profile), the counters at `0x1004f14xx` and the season table at `0x1002bcc40`. No new corpus files.
- Out of scope: the MT19937 seeding, the live board UI, the actor spawn/body calls and the shop.

## Phase

Phase 7, arena. The roster and resolution.
