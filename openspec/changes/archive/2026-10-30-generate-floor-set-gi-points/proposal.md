# Proposal

## Why

Third pass over the mesh voxeliser and voxel map (docs/PLAN.md Phase 5,
visual fidelity: the voxel GI's sample points). Meshes that ship without
a GI point chunk get points at load in the original only where its
floor-set loader and its character physics-body builder run the surface
generator; Exarust had the generator but never ran it at load.

Evidence: the original's assembly for the floor-set loader's first-level
walk (name prefixes `#`, `+`, `[` tested only on the direct children of
the centre-floor variants), the recursive tree walk (class `0x89`, second
flag word bit `0x10000` or `0x8`, generator on the node's master, count
and buffer copied to every instance), the generator wrappers (parameter
20 from one constant, a second builder with collision level 2 for the
character bodies) and the generator's unconditional store; the shipped
floor sets (only `gefs_uwa01` and `gefs_uwb02` have marked nodes, none
with a chunk).

## What Changes

- `exa_physics::voxel::generated_surface_points` (wired): the generator
  in the original's centimetres at parameter 20 with the greedy cover,
  points back in metres; `floor_set_node_generates`, the flag test.
- `exa_physics::voxel::MeshQuery`: a triangle tree that answers the
  generator's sphere queries exactly like the plain closest-point query.
- The generator's cell list is built by a stable sort equal to the
  original's insert-after-equals.
- `exa_assets::model` (wired): the floor-set walk marks nodes (an instance
  marks its master); a marked node's points are generated and replace any
  chunk; every other mesh keeps its chunk.

## Out of scope

- The character physics-body trigger: Exarust's characters carry no GI
  points, so it is not run (the function is tested).
- The node's own collision query and its collision levels (the node's
  triangles stand in).
- The generator's spacing field, which nothing in Exarust reads.
