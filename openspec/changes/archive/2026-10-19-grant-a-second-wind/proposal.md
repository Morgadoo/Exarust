# Proposal

## Why

A blow that leaves a fighter alive under 0.05 yellow always starts the heartbeat path. The armour that grants a second wind never fires.

## What Changes

- A granted second wind fires when the game clock is past its ready clock.
- It waits 180000 ms and skips the heartbeat message.
- The heal sound uses volume bits 0x3dcccccd and pitch bits 0x3f800000.
- At that ready clock the wind waits and the heartbeat message is written.

## Capabilities

### New Capabilities

### Modified Capabilities

- `combat`: the second wind on a low-health blow.

## Impact

- `exa_combat::fighter::answer_low_health`, called from the hit path.
- `docs/PLAN.md`, `docs/COMPONENT-MAP.md`, `docs/formats/OTHER.md`, and `docs/UI.md`.
- Evidence: the damage call's low-health check. No new corpus files.
- Out of scope: limb impairment, a knock-out from low yellow, and practice mode.

## Phase

Phase 4, the first duel. A second wind instead of the heartbeat.
