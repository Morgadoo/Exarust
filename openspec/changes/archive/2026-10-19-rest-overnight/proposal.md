# Proposal

## Why

Phase 7 ends an arena day. The night does not yet raise health, so a week of injuries never recovers.

## What Changes

- Ending a day adds 6553 to every employee's health, or 9830 when a physician is hired, and stops at 65535.
- A day that is a multiple of 7 still drops an entry whose health is under 3276 after that rest. A pool of 3275 is rested above that line and stays entered.

## Capabilities

### New Capabilities

### Modified Capabilities

- `arena-career`: the night's rest, and the drop that follows it.

## Impact

- `exa_game::career`, the day the career already ends.
- `docs/PLAN.md`, `docs/COMPONENT-MAP.md`.
- Evidence: the overnight rest and the day that calls it. No format field changes, so the corpus step is a skip.
- Out of scope: the navigation countdown, whose writer was not found; a trainer's lessons; the hire price; reward item identities.
