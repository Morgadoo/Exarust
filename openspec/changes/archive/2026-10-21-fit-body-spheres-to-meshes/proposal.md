# Proposal

## Why

The original fits a rigid body's collision spheres, volume and point masses
to its mesh with its dynamics-volume routines (`RF_Dynamics_Volumes`, seven
routines, 14.6 KB): the shape builders of character limbs, character roots
and factory weapons run them on the voxeliser's map. A first port added the
whole pipeline and the rigid body's remaining placements (`TTetraRBD`'s
translate, place-at-matrix and settling ticks), but a review against the
assembly found its grid, its mesh queries and its sphere extraction wrong in
ways that changed most fits: the short axes floored where the original's
round helper rounds up, the corner-mask neighbour tables were read two
bytes off (wrong shifts and masks), the candidate radius was bounded by the
bordered grid instead of the cell counts, the strictly largest candidate was
pruned, the sphere and ray queries were Exarust's own (inclusive closest
point, two-sided rays with a margin on both ends) and the fit kept a grid of
its own beside the voxeliser's map. The translate ignored its weights and
counted strain without its flag, the place-at-matrix dropped contacts the
original keeps, and a settling body kept the velocity of every settling
tick.

## What Changes

- The fit runs on the voxeliser's map (`exa_physics::voxel::VoxelMap`) in the
  builders' order: grid and near-surface fill, six-ray solid fill, volume and
  centre, corner masks, sphere extraction or its fallback.
- The grid is the mesh's stored box; the short axes round their proportion
  up; the wide-query flag's radii are taken in double precision.
- The mesh queries are the original's triangle tests: the record set-up with
  the approximate reciprocal square root and reciprocal, the region test of
  the nearest point, the strict squared-radius test less its epsilon, and the
  one-sided ray test with its 1e-8 limits; the −axis rays stop at the mesh box
  widened by 0.01 cm, the +axis rays reach 0.2 cm (a double) past it.
- The extraction bounds candidates by the cell counts, never prunes the
  strictly largest, covers over the original's window, places spheres with
  the extraction's own cell sizes, and the corner spheres use the original's
  shift and share tables and mirror.
- The summary returns mass 0 for a grid with no filled cell; point masses
  use the original's triple-product barycentric weights of the centre moved
  by the builder's origin.
- The rigid body's translate weighs each particle and counts strain only with
  its restore flag; place-at-matrix keeps contacts and strain and sums the
  original's point transform; a settling tick records no velocity.
- A real-installation check fits every shipped `Objlib` body node and
  compares its spheres bit for bit with the stored list.

## Capabilities

### New Capabilities

### Modified Capabilities

- `object-physics`: the volume summary; the body voxel grid, mesh queries,
  sphere fit, corner spheres and weighing.
- `physical-body`: the rigid body's placements.

## Impact

- `exa_physics::volume` (rewritten over `exa_physics::voxel::VoxelMap`),
  `exa_physics::body` (`Ragdoll::translate`, `place_at`, the settling step),
  `exa_world::bodies` (the shipped-body check).
- `crates/exa_physics/README.md`.
- Evidence: REA (Ghidra) pseudo-code and the instruction listings of the seven
  dynamics-volume routines and the rigid body's translate, place-at-matrix and
  Verlet step (`context/re/packs/tetra/`); at their callees and callers the
  mesh kd-tree's sphere and ray queries and their per-triangle tests, the
  triangle record builder, the nearest-point routine, the tree's box set-up,
  the round helper (rounding mode set toward +∞), the barycentric helper and
  the three shape builders' arguments; the executable's data for the ray
  records, the neighbour, share, mirror and corner tables and the builders'
  resolutions; the installed build's `Objlib.rpk` bodies.

## Out of scope

- Wiring the fit to a live builder: character limbs and roots and factory
  weapons keep their current spheres until their builders land (Phase 3,
  C20/C21); props and items keep their stored spheres, as the original does.
- The builders themselves: their sphere sort, rest tetrahedron, mobility
  divisor and the factory weapons' record fields.
- Which settings raise the builders' quality flag and the voxeliser's
  wide-query flag.
- The rigid body's world links, inverted-tetra rebuild, clone sharing and
  sounds.

## Phase

Phase 3, bodies: the rigid-body shape builders' fit (C20 of the physics
master plan), with the rigid body's placements.
