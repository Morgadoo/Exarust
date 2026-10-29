## Context

Extends [docs/NAVIGATION.md](../../../docs/NAVIGATION.md) (Moving props, the new The refresh in play section, Not reproduced). No `docs/formats/` page changes: the change is behaviour, not data.

## Decisions

- **What queues.** Only the original's queuers that Exarust has a counterpart for: the item cursor's let-go, at the frame the lift saved (the original passes the lift's saved node matrix to the ring, so it is the floor the item stood on that is refreshed; where it comes to rest is refreshed only if a rectangle covers it). The let-go writes a message drained into the ring by the refresh system on the next fixed tick, as the original's main thread rings and its physics thread drains.
- **When.** Once per fixed tick in `FixedPostUpdate`, after the bodies' step, every pending rectangle at once: the original has no budget. With nothing queued the system returns before gathering samples, so frame time is flat.
- **Samples.** The original walks the bodies its simulation buckets in 6 m rooms, from `floor(x0 · 0.05)` up to but not including `ceil(x1 · 0.05)` of the grown rectangle, and takes each body's node points if the node is a mesh with points and no character mark. Exarust takes the moving meshes (nodes with a body, already marked for the build) and buckets each by its node's position: `floor(cm / 600)` plus half the room count, the simulation grid's origin read as centred like the navigation grid's (a guess, labelled).
- **The debug option.** Kept behind `--nav-refresh-moved` so play stays faithful while the integrator can watch the refresh work on doors and props. It merges (not rings) so many movers cannot wrap the ring.

## Open questions

- The node render-word flag (bit `0x100` of the third flag word) that makes registration queue a node: its source in the files is not traced; answer with a corpus scan of node records once its loader is found in REA.
- The simulation grid's origin offset: answer by reading the grid build's room offset in REA (the shift helpers name it) and comparing with `W/40`.
- The refresh's float order for the pre-scaled body matrix: Exarust rounds the world point; a tie on a cell or voxel edge may differ.

## Risks

- Faithfully, NPCs still cannot plan through a door that was closed at load; only `--nav-refresh-moved` opens it for planning.
- A refresh with a large rectangle (scene-wide countdown) reruns the cost passes over most of the grid in one tick; nothing starts it today.
