# Proposal

## Why

Phase 7 grants a season's item ids, then a save forgets the day, the weeks, and those ids. A loaded career cannot finish the season it had already started.

## What Changes

- A save restores the day, which weeks were won, the seasons and tournaments won, the reward count, and the reward item ids.
- A career saved on day 8 with the first two weeks won still grants season 0 when the remaining weeks are swept.

## Capabilities

### New Capabilities

### Modified Capabilities

- `session-save`: the season, the weeks, and the reward ids.

## Impact

- `exa_game::session`, the Exarust save the career already writes.
- `docs/PLAN.md`, `docs/COMPONENT-MAP.md`.
- Evidence: the night already records the day, the four week flags, and the four item ids. The save was dropping them. No retail format field changes, so the corpus step is a skip.
- Out of scope: the original's save packages, roster health, and emblems.
