# Proposal

## Why

Phase 6 has the line walk. The straight walk, the obstacle test, and the nearest walkable point are the other grid queries a path uses.

## What Changes

- A straight walk allows a floor five above the current height and blocks one six above when the underside is within fifteen. After a lower floor the height becomes that floor.
- A line of 40 cm or less is not an obstacle. A longer line is one when a floor is more than seven too high, or when a cell before the last two has a penalty above 60.
- A cell whose clearance and penalty sum to less than 80 keeps the point and takes the floor's height. A cell at 80 steps to the first cheaper neighbour's centre.

## Capabilities

### New Capabilities

### Modified Capabilities

- `navigation-grid`: the straight walk, the obstacle test, and the nearest walkable point.

## Impact

- `exa_world::nav`, beside the line walk already shipped.
- `docs/PLAN.md`, `docs/COMPONENT-MAP.md`, `docs/NAVIGATION.md`.
- Evidence: the three grid queries. No format field changes, so the corpus step is a skip.
- Out of scope: footprints, and the shared stale visit map of the nearest search.
