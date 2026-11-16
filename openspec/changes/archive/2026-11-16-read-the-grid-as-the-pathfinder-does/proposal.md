# Proposal

## Why

Phase 4 (AI) of docs/PLAN.md, roadmap task B4 (docs/roadmap/phase-b-close.md):
the pathfinder's remaining open ledger rows. The planner, its corner line
test and the nearest-walkable flood were compared block by block with the
instructions; the movers' sampling, the simulation's room offset and the
merge's only callers (the sector's registration of a node) were read.

Evidence: REA (Ghidra) on `Exanima.exe` 0.9.5 — the planner, the corner line
test, the nearest-walkable query, the build's raise and the refresh (their
pre-scaled frames and room walk), the matrix-point helper, the sector's
registration walk and body insertion, the pending-node callback and its
drain in the simulation's update, the simulation's grid build and sector
constructor, the level import's order (nodes added, then the grid built),
the pathfinder's set-up; the corpus (`exa-tool corpus render-words`: 1536
meshes carry the render word's `0x100`; `shipped_levels`).

## What Changes

- The planner, its line test and the flood read the grid by the original's
  unchecked 32-bit index: a window column past the grid's side reads the
  neighbouring row.
- The build's raise and the refresh sample points through the original's
  pre-scaled single-precision frame.
- The refresh walks its rooms row by row and buckets each body by its box
  centre; the room offset (half the scene's room count) is confirmed.
- A mesh whose render word has `0x100` queues its box for the refresh when
  it registers with a grid; the level's own nodes, registered while the
  level loads, leave nothing queued (the grid's set-up empties the list).
- `exa-tool corpus render-words` counts and lists the bit.

## Out of scope

- The shared stamp table (DEC-5, open).
- The off-floor search's leftover register term (its value needs a capture
  of the running game).
- Marking a model dropped from a window (the GUI drop belongs to the player
  controls' lane).

## Impact

`exa_world` (nav grid, planner, flood, movers, view, headless, bodies,
level spawn), `exa_tool` (corpus); docs/NAVIGATION.md, docs/formats/RFC.md,
docs/DEBUGGING.md, crates/exa_world/README.md.
