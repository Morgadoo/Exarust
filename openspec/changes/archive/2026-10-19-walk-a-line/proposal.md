# Proposal

## Why

Phase 6 follows a path and soft-stops near its end. It does not yet walk the straight line from the ground to the carrot, or ask for a re-plan when that line is blocked.

## What Changes

- A line walk steps from the cell after the start along the longer axis. A floor more than four above the current height, whose underside is within ten, blocks. The first blocked step keeps the start. A later one stops before that cell and keeps the asked height. A clear line keeps the asked end.
- In the scene, a blocked line from the ground to the carrot clears the run bit and asks for a re-plan, and leaves the carrot. A clear line leaves the run bit and the re-plan flag. Out of the scene, the line is not walked.

## Capabilities

### New Capabilities

### Modified Capabilities

- `navigation-grid`: the line walk that blocks on a floor too high to step onto.
- `npc-steering`: the re-plan asked when that walk from the ground to the carrot is blocked.

## Impact

- `exa_world::nav::line_walk`, and `exa_ai::path` calling it.
- `docs/PLAN.md`, `docs/COMPONENT-MAP.md`, `docs/NAVIGATION.md`.
- Evidence: the line walk and the path step that calls it. No format field changes, so the corpus step is a skip.
- Out of scope: the straight walk, the obstacle test, and the nearest walkable point.
