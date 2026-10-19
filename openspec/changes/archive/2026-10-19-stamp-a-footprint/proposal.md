# Proposal

## Why

Phase 6 plans around walls. A character does not yet add its footprint to the penalty byte, so other walks do not pay for standing on a cell.

## What Changes

- A footprint adds 63 to its centre, 32 to the eight neighbours, and 16 to the twelve cells beyond, on the layer that contains the standing height.
- The outer five cells of the grid are not stamped. Removing the footprint takes the same amounts back.
- The centre's penalty makes the obstacle test fire until the footprint is removed.

## Capabilities

### New Capabilities

### Modified Capabilities

- `navigation-grid`: the footprint written into a layer's penalty byte.

## Impact

- `exa_world::nav`, read by the obstacle test already shipped.
- `docs/PLAN.md`, `docs/COMPONENT-MAP.md`, `docs/NAVIGATION.md`.
- Evidence: the footprint stamp and its removal. No format field changes, so the corpus step is a skip.
- Out of scope: the mover refresh that rebuilds floors inside a dirty rectangle.
