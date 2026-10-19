# Design

## Context

The grid and its search are shipped. The line walk those queries use, and the path step that asks for a re-plan when the walk is blocked, are not.

## Goals / Non-Goals

**Goals:**

- Walk a straight line on the grid and stop before a floor that cannot be stepped onto.
- In the scene, clear the run bit and ask for a re-plan when the walk from the ground to the carrot is blocked.

**Non-Goals:**

- The straight walk, the obstacle test, and the nearest walkable point.
- Writing the clamped point back onto the carrot.

## Decisions

1. The walk steps the longer axis. The start cell is not tested. A floor more than four above the height blocks when its underside is within ten of that height. An empty floor is skipped.
2. The first blocked step returns the start. A later one returns the previous cell's place and the asked end's height.
3. The path step walks only when a low scene bit is set. A blocked result clears the run bit and sets the re-plan flag. The carrot stays.

## Risks / Trade-offs

- The clamp uses the cell place directly. The original's fast reciprocal square root is not reproduced; the path step discards that point.

## Migration Plan

No data migration. No format page: no field changes.

## Open Questions

- None for this slice. The walk and the re-plan flag were read before this change.
