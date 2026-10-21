# Proposal

## Why

The original keeps its scene in a graph of sectors and nodes: it registers
nodes in sectors by kind, keeps each sector's bounds and each mesh's 6 m
cell, classifies sectors every frame (hidden, active, seen through
doorways), and decides per mesh whether it is drawn, drawn through a portal
scissor, faded or not at all. Exarust had none of this outside the light
flood and the light links; the room view draws from the room states alone.

## What Changes

- A pure model of the original's scene-node unit in `exa_world`: node
  kinds, registration by kind, batch insertion, locked sectors, change
  bits and the membership rebuild, subtree moves between sectors, child
  validation, frames in sector space.
- A sector's two mesh boxes and union radius, and the scene's bounds, with
  the original's arithmetic (seeds, carried-over corners, radius caps,
  lerped centres, sum orders).
- The 6 m cell and spill mask of a node (one byte, `1/600`, −1 cm and
  601 cm, the low side first), shared with the light flood, and the cell
  handler rule.
- Per-frame classification (the scene's own sector, active sectors against
  the main views, adjacent sectors through their doorways to active
  sectors with the first main view and the screen scissor) and the node
  passes (hidden pass, doorway test, inherited frame bits, cut-away fade).
- The light flood's directional fill.

## Capabilities

### New Capabilities

- `scene-nodes`: the scene graph's registration, bounds, cells and
  per-frame visibility as the original computes them.

### Modified Capabilities

## Impact

- `exa_world::scene_nodes` (new), `exa_world::light_flood` (the mesh cell
  test now uses the shared cell rule; `fill_directional`),
  `crates/exa_world/README.md`.
- Nothing is on the draw path yet: levels load and look as before. The
  light links' cell test changes only for positions on exact cell
  boundaries (the original multiplies by `1/600` where Exarust divided by
  600) and for positions far outside the grid (the one-byte cell).
- Evidence: REA (Ghidra 12.1.4) on the original's scene-node unit: the
  pseudo-code for structure and the instruction listings for arithmetic
  (comparison order and NaN sides, double-precision thresholds, sum
  orders, the rounding conversion); constants read from the executable's
  data (`1/600`, 601, −1, 0.003, 0.001, 0.999, 1e−6, 20, the fade
  epsilon). No corpus file is needed: tests build synthetic scenes.
- Out of scope: wiring the model into the draw path (portal scissor,
  per-mesh masks and the dithered and partial draws in `exa_render`),
  building doorway quads from the level data, the light aliases'
  activation and relinking each frame, world lights' part flags, the far
  sort key, draw buckets and material index remapping, the scene
  creation, freeing and cleanup hook, and the per-frame removal queue.

## Phase

Phase 2, static world: sectors and portals for visibility.
