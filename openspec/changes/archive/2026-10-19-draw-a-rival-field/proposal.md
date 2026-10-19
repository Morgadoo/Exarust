# Proposal

## Why

A tournament names 16 of the 19 rival companies. The draw steps the same roll as the bracket and does not repeat a company. That field is not in the career yet.

## What Changes

- Fill a bag with 0 through 18.
- Sixteen times, step the roll by `× 0x08088405 + 1` and take the high half of the remaining count times that word.
- Store one more than the chosen bag value.
- If the chosen slot is not the last, copy each later value down one and shrink the bag.

## Capabilities

### New Capabilities

### Modified Capabilities

- `arena-career`: the 16 rival companies drawn for a tournament.

## Impact

- `exa_game::career::draw_rival_field`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`.
- Evidence: the tournament field draw. No new corpus files.
- Out of scope: spawning those companies, the six event records, and menus.

## Phase

Phase 7, arena career. The rival field, not the whole of `XA_Arena`.
