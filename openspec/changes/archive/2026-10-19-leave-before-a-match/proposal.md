# Proposal

## Why

Before a match, the first specialist who is not well enough leaves the company. Health under 700 counts as a death. Health from 700 through 64999 is a departure. That walk is not in the career yet.

## What Changes

- Walk the roster from the start. Skip every combatant.
- Remove the first specialist under 700 and raise the death count by one.
- Remove the first specialist from 700 through 64999 without that rise.
- Leave a specialist at 65000 or more, and leave everyone once one person has gone.
- An empty roster removes nobody.

## Capabilities

### New Capabilities

### Modified Capabilities

- `arena-career`: who leaves the company before a match.

## Impact

- `exa_game::career::Company::leave_before_match`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`.
- Evidence: the roster walk, the two health compares, and the death-count add. No new corpus files.
- Out of scope: releasing a kit, unlinking a world character, the message log, and menus.

## Phase

Phase 7, arena career. One roster walk before a match, not the match itself.
