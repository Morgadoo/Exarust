# Design

## Context

The grid is built. The duellist sets a fixed approach speed toward the target. The player probes 75 + 6·v² cm, capped at 1 m.

## Goals / Non-Goals

**Goals:**

- A path around a wall on a synthetic grid.
- The look-ahead numbers, used by the player's probe.
- The duellist's velocity equal to `follow` on the path from it to the target.

**Non-Goals:**

- String pulling, the 256-cell window of a full level, and footprints.

## Decisions

1. Cost is 10·Δfloor² + 5 orthogonal or 7 diagonal + 2·clearance + penalty. A penalty above 600 is refused. Rise at most 4 floor units, drop less than 21. The search stops after 4095 nodes.
2. The order moves the heading a tenth of the way toward the carrot 100 cm along the path, then walks at 1.4 m/s or jogs at 3.6 m/s.
3. With no grid the duellist follows the two-point path from itself to the target, so it still closes, through the same function.

## Risks / Trade-offs

- The search does not yet string-pull, so a path keeps its corners. The test only requires that it avoids the wall and is longer than the straight line.

## Migration Plan

No data migration.

## Open Questions

None.
