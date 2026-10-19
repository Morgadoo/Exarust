# Design

## Context

The carrot and the out-of-scene slide are shipped. The same path step then sets the run bit and, close to the end, calls the soft stop.

## Goals / Non-Goals

**Goals:**

- Set or clear the run bit from the urgency and the carrot fraction.
- Soft-stop inside 30 cm of the carrot, or inside 100 cm of the last point when the urgency is at least 1.

**Non-Goals:**

- The line walk and the re-plan flag.
- The in-scene driver that may set the run bit again afterwards.

## Decisions

1. The fraction is promoted to a double before it is compared with 0.99. The f32 value 0.99 promotes to slightly more than that double, so it counts as past. A NaN fraction does not.
2. Both distance tests are strict and 3-D. Exactly 900 and exactly 10000 do not stop.
3. A soft stop clears the run bit, sets the stopping bit, clears free walk and following, and replaces a carrot farther than 8·speed + 20 with the standing point plus the facing times that limit. The distance that decides the stop is from the ground to the carrot. The pull is from the standing point.

## Risks / Trade-offs

- Two path marks cleared by the soft stop are not named here, so they are not written.

## Migration Plan

No data migration. No format page: no field changes.

## Open Questions

- None for this slice. The compares and the soft stop were read before this change.
