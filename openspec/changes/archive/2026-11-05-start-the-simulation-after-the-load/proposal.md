## Why

Phase 1/4 infrastructure, TSimulation (when the simulation starts stepping)
and the level load: two runs of one scripted `--fixed-frames` scene
diverged from their first traced tick, so a scripted check could not be
repeated. The fixed step ran during the loading screen, the player's body
spawned once the level's collision was built (a wall-clock dependent number
of frames), and the colliders joined the world in the order their builds
finished on the thread pool, within a 4 ms clock budget per frame.

## Evidence

REA: the original's level load raises its pause counter for the whole
load and lowers it at the end, and its main loop skips the frame time while
the counter is set, so no simulation step runs during a load. Its level
geometry is loaded whole, and its collision queries work on the raw meshes
from the start; the kd-trees built on `TKDtreeBuilderThread` are swapped in
outside a step's queries and only speed the queries up.

## What Changes

- `exa_game::sim_control`: the fixed main loop runs only without a loading
  screen, and any time it had left is dropped while it shows.
- `exa_physics` static build: finished builds join the world in hand-out
  order; a frame hands out up to 120 000 triangles (no clock).
- Docs: docs/UI.md (load stages), docs/DEBUGGING.md (`--hold-loading`).

## Out of scope

- The bridge warnings about unbridged render components (pre-existing).
- Runs on two different machines (thread counts can change nothing here,
  but nothing else was compared).
