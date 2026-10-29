## Why

Round 29 left the navigation grid's refresh unwired: Exarust builds the grid once per level, and nothing queues a rectangle, so the refresh (drain the ring, restore the floors as built, raise them from the movers, rerun the cost passes) never runs in play. The lane was asked to make an opened door's doorway walkable again. Reading the original showed that it never queues a body that physics moves: a door swung open, a crate knocked aside or a body falling asleep queue nothing. A rectangle is queued only when a node is registered in a simulation sector with a flag in its render word (at load or on spawn), when an item the cursor lifted is let go (the held node's box at the frame the lift saved, in the ring), and, for an item let go into a window without a saved frame, after its deferred placement (merged). The refresh then runs once per pass of the physics thread, after every simulation has stepped, refreshing every pending rectangle with no budget, and samples the bodies the simulation's 6 m room grid holds in the rooms each grown rectangle spans.

## What Changes

- `exa_world::nav::mover`: the refresh samples bodies (a node frame and its GI points) bucketed by their node's position in the rooms a grown rectangle spans (`MoverPoints`); `rect_of_box` gives a node box's rectangle under its frame; `movers_idle` says when nothing waits.
- `exa_world::nav::view`: `NavMovers` keeps the queues with the level's grid; `NavMoved` rings a node's box; `refresh_grid` runs once per fixed tick in `FixedPostUpdate`, doing nothing when nothing is queued.
- `exa_game::hand`: the item cursor's let-go queues the held node's box at the frame the lift saved, as the original's release does.
- `--nav-refresh-moved` (Exarust's debug option, off by default): a moving mesh whose node has held a new frame for 32 ticks queues the rectangles of where the grid last saw it and where it rests, so a door opened by the hand frees its doorway for planning. The original does not do this.

Phase 6 (AI) in docs/PLAN.md: the navigation grid's movers.

## Evidence

REA assembly and pseudo-code of `Exanima.exe` 0.9.5: the pathfinder's ring, merge and refresh slots and every caller of them in the binary (the simulation sector's body registration and its subtree registration walk, both behind a render-word flag of the node; the item cursor's let-go and its deferred restore); the hook that gives a sector its scene's pathfinder; the physics thread's pass that installs the global pathfinder, steps the simulations and then calls the refresh. No other code reaches these slots, through the sector's pathfinder, the scene's or the global one.

## Out of scope

- Queuing nodes registered with the render-word flag (Exarust tracks no such flag; the build covers what is registered at load).
- The let-go into a window without a saved frame (Exarust's windows take no world item).
- The scene-wide countdown's writer (still not found).
- Bodies outside the simulation grid, which the original buckets in its first inner room.
