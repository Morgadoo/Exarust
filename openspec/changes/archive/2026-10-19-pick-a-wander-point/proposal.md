# Proposal

## Why

Phase 6 can wait and then ask for a wander walk. It does not yet choose the point: a quarter of the way from a random exit toward the room, retrying once when that exit leads back.

## What Changes

- A walker with no room does not pick, and the roll stays.
- One exit is drawn. A second draw happens only when the first exit leads back to the room the walker came from. That second draw is kept even if it leads back too.
- The goal is a quarter of the way from the exit toward the room point, at height 0. The stop is 20 cm and the radius is 150 cm. The goal ignores height.

## Capabilities

### New Capabilities

### Modified Capabilities

- `npc-idle`: the wander point a roam or a patrol walks to.

## Impact

- `exa_ai::idle`.
- `docs/PLAN.md`, `docs/COMPONENT-MAP.md`.
- Evidence: the idle wander goal. No format field changes, so the corpus step is a skip.
- Out of scope: follow, sleep, and the path search that walks to the point.
