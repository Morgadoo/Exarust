# Proposal

## Why

Phase 6 can toggle a door. A walker on a path does not yet hold for a door that is at most 0.7 open, or toggle it.

## What Changes

- A walker with no path, a cursor at the end, or no door keeps walking.
- A door more than 0.7 open is left alone.
- A door at 0.7 or below holds the walker. If it is not already toggling, the walker toggles it. A door already toggling stays as it is.

## Capabilities

### New Capabilities

### Modified Capabilities

- `npc-steering`: the walker at a door.

## Impact

- `exa_ai::path`, calling `exa_physics::door::toggle`.
- `docs/PLAN.md`, `docs/COMPONENT-MAP.md`.
- Evidence: the path step that meets a door. No format field changes, so the corpus step is a skip.
- Out of scope: the rest of the path.
