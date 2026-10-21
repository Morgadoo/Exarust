# Proposal

## Why

The original's triangle kd-tree, its terrain block subclass, the
simulation's per-tick conversions, the live scene's bookkeeping and the
terrain manager's tile records had no model in Exarust. A first pass
modelled them from the decompiler's output alone; a review against the
machine code replaced the invented parts (the sphere, capsule and
closest-point queries, the previous-frame state of the frame fraction, a
scene plugin registry, the vertex stride and the normal pass).

Phase 3 (Bodies: skeleton, physics, control) for the kd-tree and the
simulation, and Phase 2 (Static world) for the scene and terrain manager.

## What Changes

- `exa_physics::kd_tree`: the 8-byte node, the work-stack build with
  clipped-polygon bounds and the surface-area cost search, the box-clipped
  nearest ray, the segment, the boolean ray and the swept traversals, and
  the 30 x 30 cell terrain block tree.
- `exa_physics::dynamics`: the environment's per-tick conversion, the
  frame interpolation fraction and the sector's node-batch append.
- `exa_physics::controller` and `sim_object`: the two abstract bases.
- `exa_world`: the scene's transition list, removal queue and despawn, the
  environment presets' fast-power linearisation, the scene editor's frame
  step and tick, and the terrain manager's constructor numbers, tile
  record, normal pass, height pass, bounds and static set-up.

## Out of scope

- Using the kd-tree in the running game: the level's collision is still
  rapier's trimesh, and no query in the game needs the original's
  front-face rule.
- The mesh-side leaf tests of the swept traversals and the mesh's triangle
  plane record (the orientation of the stored normal is assumed).
- Drawing paths for the scene bookkeeping, the terrain GL upload, worker
  and brush manager, and the sector constructor.

## Evidence

REA (Ghidra 12.1.4) on the original's kd-tree, simulation, scene, editor
and terrain units, read against their assembly (float sums, MINSS/MAXSS,
rounding modes and double compares checked, not taken from the
pseudo-code). All tests use synthetic data.
