# Proposal

## Why

A second wind that has fired never adds yellow health. The armour's three-second heal is missing from the health tick.

## What Changes

- An active second wind adds the double 0.0003 to yellow on a living health tick and stores the float.
- The wind stays active while the game clock is at or before the ready clock minus 180000 ms plus 3000 ms.
- The tick that passes that clock still adds, then the wind ends.
- A quiet wind and a dead fighter add nothing. An attack does not skip the add.

## Capabilities

### New Capabilities

### Modified Capabilities

- `combat`: the yellow a second wind adds after it fires.

## Impact

- `exa_combat::fighter::heal_second_wind`, called from the health tick.
- `docs/PLAN.md`, `docs/COMPONENT-MAP.md`, `docs/formats/OTHER.md`, and `docs/UI.md`.
- Evidence: the body tick's health bookkeeping, the same double the regeneration ramp uses. No new corpus files.
- Out of scope: limb impairment, a knock-out from low yellow, and practice mode.

## Phase

Phase 4, the first duel. The second wind's yellow heal.
