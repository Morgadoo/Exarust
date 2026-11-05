## Why

Phase 4 (Combat and items) in docs/PLAN.md, TMotilePart, the motilepart gap
lane (round 70). The last rating's gap named the Ground Query's floor bit,
the line cast and the grip-mode writes. Tracing them in REA:

- The Ground Query (the feet routine's point ray under each toe) meets only
  meshes and rigid bodies whose node has the floor bit (`0x800000` in its
  second flag word). The data has 875 such mesh nodes (818 library pieces:
  stairs, steps, bridges, walkways, floor grates; 47 in the level files; 10
  objects such as pit doors and a portcullis frame), and the grid loader
  gives every cell's merged `+Floor` mesh the bit. Exarust's colliders
  carried none, so the query always missed.
- The force barricade's floor cast takes the same bit, on static meshes
  only; Exarust's stand-in probed every collider.
- The grip mode is written by a motion's start (from the clip) and by the
  locomotion routine on each tick it runs (the general mode running or in
  combat, the walk's otherwise). Exarust re-derived it from the current
  clip every tick and dropped the locomotion's writes.

## Evidence

REA (Ghidra and the listing of `Exanima.exe`): the Ground Query (the grid
scene gate, the static list's floor-bit test, the body list's floor-bit,
out-of-world and living-limb tests, the mesh ray callback whose result must
be a triangle index above 0), the feet routine's use of the hit (raise a
moving body's ground on `hit > ground`, the floor `low < hit ? hit : low`
with an unordered compare keeping `low`), the floor cast (static meshes
with the floor bit, any hit index), the mesh ray callback (one-sided
Möller–Trumbore returning the nearest triangle's index or −1), the grid
loader's merged floor words, the mode setter and its 17 callers (motion
start, translate and re-place call the clip rule; the locomotion routine
writes 1 or 2 at its start; weapon attach and drop reset to −1 then 0; two
AI writers). `exa-tool corpus floor-nodes` lists the flagged nodes.

## What Changes

- `exa_formats::rfc::Node::FLOOR`; `exa_assets::ModelNode::floor` from the
  node's own second flag word.
- `exa_physics::FloorMesh` marks a floor node's mesh; `exa_world` puts it on
  a flagged node's parts and on a tile grid cell's floor parts (the merged
  floor's contents). The collider build records those colliders
  (`StaticColliders::is_floor`, level pieces and stand-ins), and the body
  detection's level answers `Level::floor_piece` from it, so the feet
  routine's Ground Query meets them.
- `feet::sole_floor` takes the hit only when the low floor is under it.
- `Physics::static_floor_below`: the floor cast; the debug barricade uses it.
- `exa_physics::grip::tick_mode`: the mode after a holder's tick (a clip
  start's rule, the locomotion's write over a behaviour's start, the
  locomotion's own walk start last); `exa_character::grip_runtime` reads the
  holder's `Locomotor` and applies it after the character ticks, before the
  bodies step.
- `exa-tool corpus floor-nodes`.

## Out of scope

- The Ground Query's rejection of a hit on a mesh's triangle 0 (Exarust's
  colliders keep no original triangle order for merged floors).
- The line cast (spawn without a player, save load, arena spawn snap): each
  caller places a body Exarust places by other means (the level spawn and
  nav snap, the save's own positions, the match layout with the body
  settling); a cast without a caller would be invented use.
- The grip-mode writes of the AI (two writers) and the attach/drop reset to
  mode 0 (Exarust derives a new grip's first mode from its holder's clip).
