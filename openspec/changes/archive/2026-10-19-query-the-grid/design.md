# Design

## Context

The line walk steps the longer axis and blocks on a rise of more than four. The straight walk, the obstacle test, and the nearest walkable point use the same grid with different thresholds.

## Goals / Non-Goals

**Goals:**

- Walk a straight line that allows a rise of five and follows a floor down.
- Report an obstacle past 40 cm, from a rise of more than seven or a penalty above 60.
- Snap a cheap cell to its floor, and step off a costly cell to a cheaper neighbour.

**Non-Goals:**

- Footprints written into the penalty byte.
- The shared visit map that keeps stale cells beyond fifteen.

## Decisions

1. The straight walk's height is replaced by the floor just stood on. Its upper floor blocks on underside alone, with no open-underside exception. The rise limit is five and the reach is fifteen.
2. The obstacle test refuses a 2-D length of 40 cm or less before it walks. Penalty is read only on a cell before the last two, and only above 60.
3. A clearance plus penalty under 80 keeps the queried place and takes the floor height. At 80 the first cheaper neighbour, in the original neighbour order, wins.

## Risks / Trade-offs

- The nearest search does not reproduce the original's stale shared map. A fresh query does not see another query's cells.

## Migration Plan

No data migration. No format page: no field changes.

## Open Questions

- None for this slice. The three queries were read before this change.
