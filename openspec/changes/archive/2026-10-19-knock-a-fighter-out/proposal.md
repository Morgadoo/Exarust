# Proposal

## Why

The unconscious state is a knock-out, and the wake path writes those fields itself. A blow does not knock a fighter out, and health does not store a limb impairment.

## What Changes

- A knock-out sets the wake clock to now plus the delay.
- Flags lose bits 0xF0 and gain dormant and waking. The condition is down.
- The statue mark clears, gait is 0, tone is 0, and the pin gain bits are 0x3da3d70a.
- A sleep countdown below 256 rises to 256. One already at 300 stays.
- A blow still does not call this, and no limb stores an impairment.

## Capabilities

### New Capabilities

### Modified Capabilities

- `npc-idle`: the knock-out a waking sleeper already uses.

## Impact

- `exa_ai::idle::knock_out`, called from the sleeper's wake.
- `docs/PLAN.md`, `docs/COMPONENT-MAP.md`, and `docs/formats/OTHER.md`.
- Evidence: the knock-out routine and the sleep raise. No new corpus files.
- Out of scope: a damage-path knock-out, a stored limb impairment, and spawning the practice partner.

## Phase

Phase 4, the first duel. The unconscious knock-out.
