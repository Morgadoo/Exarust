# Proposal

## Why

The original makes GI sample points for meshes that ship none: a floor
heightfield for each tile-grid cell's merged floor, and a surface generator
for floor-set props and character body parts. Its rigid-body shape builders
also keep a voxel map per mesh. Exarust rasterised floors block-wide with its
own rounding and had no voxel map or surface generator. A first port ran the
floor generator on every floor mesh in world space: the generator's edge rule
is relative to the cell centre, so each mesh got a grid reaching back to the
world origin, meshes of one cell were no longer merged (a 20 cm block split
between two meshes lost its point), and the points were recomputed at every
block rebuild.

## What Changes

- The floor heightfield, the general surface generator (greedy cover and
  per-cell variant), the down-ray surface query and the master-to-instances
  point copy, as the original runs them, including its single/double
  precision and summation order and the downsample's walk on odd row counts.
- The rigid-body voxel map: the constructor's resolution, scale and tag; the
  bordered, Z-fastest grid of cell words; the fill bytes; the sphere list; the
  array release that frees the cells and fill bytes and keeps the spheres.
- The GI volume merges each grid cell's floor meshes and runs the floor
  heightfield once per cell, in the cell's space, when the level's GI inputs
  are collected.

## Capabilities

### New Capabilities

### Modified Capabilities

- `rendering`: grid floors in the GI volume; generated surface points.
- `object-physics`: the rigid-body voxel map.

## Impact

- `exa_physics::voxel` (new), `exa_world::gi_volume`, `exa_world::grid`
  (floor meshes carry their cell and their place in it).
- `docs/RENDERING.md` (Voxel GI), `crates/exa_physics/README.md`,
  `crates/exa_world/README.md`.
- Evidence: REA/Ghidra pseudo-code and assembly of the voxel map's
  constructor, destructor and array release, the down-ray query, the floor
  heightfield and its downsample, the surface generator, its two passes and
  covers, the builder's constructor (cover weights and collision level), the
  mesh's collision-plane setup, and the shape builder's grid fill and sphere
  extraction (map layout only); `context/re/gi-occupancy.md` §9.
- Out of scope: the shape builders' grid fill, solid fill, corner masks,
  volume, sphere extraction and fallback; the node's kd-tree sphere query and
  its collision levels (a closest-point stand-in is provided); wiring the
  surface generator to floor-set props and character bodies; collision
  triangles (render triangles stand in); retail captures of the points.

## Phase

Phase 2, static world: the voxel GI occupancy (with Phase 3's rigid-body
shape builders as the voxel map's future caller).
