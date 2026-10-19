# Proposal

## Why

Phase 2 holds a paired door shut until its partner is open. An unlock does not yet wait, and it does not yet open both leaves.

## What Changes

- An unlock clears the saved latch and starts a wait of 1000 ms on the door and on its partner. Both stay latched. The partner keeps its own saved latch.
- At the exact wait both stay latched. Once the clock is strictly past the wait, both latches open.

## Capabilities

### New Capabilities

### Modified Capabilities

- `object-physics`: the unlock wait.

## Impact

- `exa_physics::door`.
- `docs/PLAN.md`, `docs/COMPONENT-MAP.md`.
- Evidence: the unlock and the hinged door step's wait. No format field changes, so the corpus step is a skip.
- Out of scope: which key fits the door, and the unlock sound's random pitch.
