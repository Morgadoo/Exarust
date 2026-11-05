## Context

Follows `make-the-bridge-the-default`. No `docs/formats/` page is involved.

## Decisions

- The bodies' frame start has no frame-side reader of what it writes (the tick count is read only by the solver), and its blows are messages that wait in their buffer; running it at the first step of a frame, or at the next frame that ticks, gives the solver the same inputs.
- Under the bridge the frame never runs `SimFrameStart` itself, so the schedule is still unbuilt when the first step moves it.
- Cloth meshes: the last update of a frame is the newest state, which is what the frame wrote before. A cloth built on a frame without ticks shows its first positions one frame later than in one world.

## Risks / Trade-offs

- The copy cost is now dominated by the per-type bookkeeping and the transforms' and hierarchy's scans (2.1 ms a step in exanima01), not by bodies.

## Open questions

- None for reverse engineering.
