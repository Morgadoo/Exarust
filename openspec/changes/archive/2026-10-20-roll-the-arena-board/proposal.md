# Proposal

## Why

The arena's templates and rank roll are modelled, but not the rolled instance record, the six-row daily/tournament generators, the weekly prize draw or the company shuffle. The fresh decode gives their exact LCG/MT split and record fields.

## What Changes

- `MatchInstance` and `roll_instance`: the rank (MT unit draw, nearest-even), rank window, weapon row (LCG), cash and secondary scaling.
- `needs_rank` to keep the MT stream untouched for flag-2 and unranked templates.
- `generate_daily_board` with the 7-template retry loop, caps, minimum slots and reject masks.
- `draw_prize` with the eight-index history, `generate_tournament_board` with the five-template loop and forced 5/row ranks, and `shuffle_companies` continuing the same LCG.

## Capabilities

### New Capabilities

### Modified Capabilities

- `arena-career`: the rolled instance and the board generators.

## Impact

- `exa_game::arena`, from `MatchInstance`, `needs_rank`, `roll_instance`, `generate_daily_board`, `TournamentBoard`, `draw_prize`, `generate_tournament_board` and `shuffle_companies`.
- `docs/PLAN.md` and the generated `docs/COMPONENT-MAP.md`: `XA_Arena` becomes **24%** in the source rating. The headline rises.
- Evidence: `0x100183f80` (roll), `0x10018cf70` (daily), `0x10018cc90` (tournament/prize), `0x100187bf0` (shuffle), the template arrays and the 0x70 instance layout. No new corpus files.
- Out of scope: the MT19937 state/seeding, the roster/entrant flow, opponent spawn and the live board UI.

## Phase

Phase 7, arena. The rolled boards.
