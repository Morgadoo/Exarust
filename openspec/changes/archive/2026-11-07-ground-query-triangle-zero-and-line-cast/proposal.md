## Why

Phase 4 (Combat and items) in docs/PLAN.md, TMotilePart, the motilepart2
gap lane (round 95). The latest rating left the Ground Query's triangle-0
rejection and the line cast with its callers open. Both were traced again
against the assembly.

## Evidence

REA (Ghidra and the objdump listing of `Exanima.exe` 0.9.5):

- The Ground Query calls each floor mesh's ray callback and tests the
  returned triangle index with `test eax,eax; jle`, so index 0 (and the miss,
  −1) is dropped. The line cast and the barricade's floor cast test it with
  `jl`, so they keep triangle 0. The mesh's default callback walks its
  sections' triangle ranges in index order with a one-sided test against the
  engine's epsilon (`0x33ffffd7`), sums dot products x, then y, then z, takes
  a hit strictly inside `(0, limit)` and strictly nearer than the last, and
  writes the shortened limit back. The caller copies its nearest hit into
  that limit before each mesh and only takes the result back when the index
  passed, so a dropped hit does not shorten the ray.
- The line cast walks the scene's sectors' mesh nodes (class 0x89) whose
  second flag word has `0x200000` (cleared by the cast's own exception
  handler; [inf] every collision mesh), skipping disabled bodies and living
  characters. Its three callers: the start of play when no player exists
  (the start node lifted 1000 cm, cast 3000 cm down, the player added at the
  hit), the save unit's placement on a loaded level (lifted 100 cm, 300 cm
  down, then the 108.18 − 11.1·extent lift: `exa_game::save_level`, whose
  plan is not called by the live load yet), and the arena spawner's snap
  (lifted 1000 cm, 3000 cm down, the hit plus 5 cm) for opponents, the
  company and the elimination and reserve spots. The earlier note that these
  "place bodies Exarust places by other means" held only for the last two:
  Exarust's start of play stood the player on the start's stored height.

## What Changes

- `exa_physics::contact::mesh_ray`: the plain mesh ray as the callback runs
  it. `Level::indexed_triangles` gives each triangle with its index in its
  collider's mesh; `Level::floor_skips_first` says a floor piece holds its
  node mesh's first triangle.
- The Ground Query (`floor_query`) runs the mesh ray per floor piece with the
  carried limit and drops a hit on triangle 0 of such a piece.
- `exa_assets::ModelNode::first_part`, `exa_physics::FirstFloorPart` and
  `StaticColliders::{insert_floor(handle, first), skips_first}`: a floor
  node's part built from the group the index list starts with. A grid cell's
  merged floor is not one mesh in Exarust, so it never carries it.
- `exa_physics::contact::line_cast` and `Physics::line_cast` (every fixed
  solid collider, walked whole so the frame's replica answers before the
  step shares its tree); `exa_world::start::cast_onto_floor`; the viewer puts
  a new player at the level's own start onto the cast's hit.

Not changed: the save placement (its plan is not live) and the arena snap
(left for the arena lanes; the placements keep their stored heights).
