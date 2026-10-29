## Context

The cloth stage runs after the bodies' tick. Each cloth is offered the body parts linked in the nine 6 m cells around its own, by each part's mesh box centre, as the original's grid sector links its bodies. Exarust rebuilt that bucketing from every body every tick: on exanima01 (~4 800 bodies) a rigid transform per part and a hash insert per part, about 1.6 ms a tick, plus a per-tick map of every part's countdown for the hair's owner.

## Decisions

- **Change detection as the "moved" test.** A body's part cells are a pure function of its `Ragdoll` (particles, rest frame, pairing). The body stage hands deep sleepers to the solver through `bypass_change_detection` and marks one changed only when it wakes; drawing (`draw_bodies`) and prop stand-ins (`Changed<Ragdoll>`) already rely on an unchanged sleeper being where it was. So an unchanged body keeps its cells, and only changed or new bodies are re-bucketed; gone bodies are swept when the walk is shorter than the index.
- **Order kept exactly.** The offered order (cells in the original's scan order, then each cell's insertion order) feeds the contact gathering, so each body's walk rank is recorded every tick (a cheap pass over ~4 800 bodies) and each gathered cell is sorted by (rank, part index), which is the order a full re-bucket links them.
- **Read only the cloths' cells.** The plan (frames, owners, object-list re-bucket) now runs before the gathering, so the needed cells are known; the parts' contact data (transform, sphere, countdown) is built only for parts in those cells.
- **Sleeping bodies are not skipped.** The original masks a body's sleep bit only when the object sleeps, so an awake hair is offered sleeping bodies; skipping them would change results.
- **Owner lookup.** The index keeps every part's body (linked or not), so a hair's owner (the nearest part above it) and its countdown come from it.

## Risks

- A system that writes a sleeping body's particles through `bypass_change_detection` without waking it would leave the index stale (the same assumption the drawing makes); none does today.
- A system that marks every body changed each tick (a `&mut Ragdoll` deref for sleepers) costs a full re-bucket again; the debug log's `bodies re-bucketed` shows it.
