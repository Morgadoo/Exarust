# Proposal

## Why

Phase 4 names what a click finds. It does not yet pull a grabbed point along the cursor, or refuse a box that sticks out of the drag's height.

## What Changes

- The pull starts 1000 cm along the cursor direction. A nearer accepted hit replaces that distance. A hit at 1000 cm or beyond does not.
- The grabbed box is refused when any corner, after it is placed, is higher than 280 cm or lower than 1000 cm. A corner on either limit is kept.

## Capabilities

### New Capabilities

### Modified Capabilities

- `player-movement`: the drag's reach and the box's height gate.

## Impact

- `exa_game::drag`.
- `docs/PLAN.md`, `docs/PLAYER.md`, `docs/COMPONENT-MAP.md`.
- Evidence: the drag reach of 1000 cm and the box gate at 280 cm and −1000 cm. No format field changes, so the corpus step is a skip.
- Out of scope: the level polygons that produce a hit, the door creak, and pouring.
