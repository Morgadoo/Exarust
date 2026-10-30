# Proposal

## Why

Phase 2 (static world) of docs/PLAN.md: the grid environment manager and
the terrain grass still had rules Exarust did not reproduce. A placed door
was never bound to the door link it stands in, the world-point room query
clamped a column off the grid's side instead of reading the linear cell,
the cell-active query was missing, plants stood on Exarust's own triangle
split instead of the original's ground query, and large plants took the
terrain normal although the original's loader makes that branch dead.

Evidence: the original's door-binding routine, world-point room query,
cell-active query, the engine's yaw helper and its quick arccosine, the
plant ground query and its caller in the per-block baker (which stores the
result as the instance height), and the plant-library attach that sets
every variant's kind byte after the small-plant library, read in REA
(Ghidra listing) and the gridenv pack's assembly.

## What Changes

- `exa_world::rooms`: `room_at` reads the point's linear cell, checked only
  against the cell count; `RoomTracker::cell_active`; `DoorBodies::bind`
  writes a door body into both rooms' entries for the link its cell and
  facing select; `cell_step_from_dir` takes the engine's quick yaw and the
  original's mixed f32/f64 quadrant edges.
- `exa_anim::quick_yaw`: the engine's yaw helper, shared with
  `exa_ai::combat::yaw` (a NaN cosine stays NaN).
- `exa_world::terrain::vegetation`: `ground_on_grid` places every plant by
  the original's ground query (wired into `Plants::spawn`); every variant's
  own normals turn with the instance.

## Out of scope

- Feeding `DoorBodies` from level placements and a door mechanism that
  reads it (doors still count as open for light and reach).
- The single-sector (room-mode-off) light path, the omni-shadow ceiling
  shell, callers for the occluder and lid segment queries.
- The grass wind program, the shading-quality shader variant, the
  mask-texture colour lookup and the grass vertices' 0.75 colour.

## Impact

`exa_world`, `exa_anim`, `exa_ai`; docs/RENDERING.md, docs/formats/TERRAIN.md,
crates/exa_world/README.md.
