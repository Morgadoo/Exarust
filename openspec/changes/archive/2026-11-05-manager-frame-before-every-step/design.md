## Context

Follows `narrow-the-hierarchy-copy`. No `docs/formats/` page is involved.

## Decisions

- `FrameStartStep` runs `SimPreStep` on every pass, then `SimFrameStart` on the first pass of a frame: the manager frame keeps its place before the hit chain and the bodies' frame start.
- No rate guard is added: the original's routine has none.

## Risks / Trade-offs

- The rise's timing now has step granularity (a rise logged 10 008 ms after the death, 834 steps).

## Open questions

- None for reverse engineering of the manager frame; the blows' queueing in the original is not traced yet.
