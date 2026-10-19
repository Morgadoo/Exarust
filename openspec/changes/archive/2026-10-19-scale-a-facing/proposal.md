# Proposal

## Why

The fighting place does not yet scale a candidate by facing, and an action outside a swing does not read the idle clock. The dead-state entry knocks a body out. A blow does not.

## What Changes

- When the observer is not attacking, the approach is multiplied by one more than how far the candidate is from straight ahead.
- A press multiplies the strike and the approach by one plus ten times how far the opponent faces away.
- An action outside a swing recovers while the body's idle stamp is strictly above the clock.
- A dead body is knocked out at the current clock with no delay. A walking body stays. A blow does not call this.

## Capabilities

### New Capabilities

### Modified Capabilities

- `npc-attack`: the facing scale and the idle stamp.
- `npc-idle`: the dead-state knock-out.

## Impact

- `exa_ai::place::score_place`, `exa_ai::place::publish_arc`, and `exa_ai::idle::knock_out_dead`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`.
- Evidence: the position score's facing scale, the arc publisher's idle compare, and the dead-state entry. No new corpus files.
- Out of scope: the part rewrites that precede the dead knock-out, and a blow calling the knock-out.

## Phase

Phase 6, AI. The facing scale and the idle clock.
