# Proposal

## Why

With voxel GI on, `play arenao3` (scene seed 15) showed a hard, straight
brightness seam across the shadowed ground, about two thirds down the
default view. It is the edge of the camera's GI block: surfaces inside the
block took the GI ambient, surfaces beyond it fell back to the environment
cube, and the two disagree. The block beyond was built and resident, but
nothing stepped its light or let a surface sample it.

Evidence (REA on the 0.9.5 build):

- The per-object GI lookup, called by the lit-material builder, the terrain
  draw and the grass draw for each object (each terrain block once), floors
  the object's position less the block grid's low edge, in centimetres,
  times 1/4800 on each ground axis, takes that block, and binds its light
  texture with the block's `VXGIPos`. A position outside the grid, or a
  block whose state lacks either texture bit, binds texture 0 with a
  default position. The last pick is cached on the manager.
- The surface program (`ambient.glsl`) samples `VoxelGI` with no box test:
  a fragment beyond the bound block reads the texture's clamped edge, and
  texture 0 reads (0, 0, 0, 1), so such an object has no GI ambient.
- The block texture is created with the engine's default wrap (the clamp
  constant) and linear filtering.
- The manager steps every resident block each frame and copies each
  resident neighbour's margin into it (the earlier margin-copy change).

Phase: Phase 5 (world rendering): the voxel GI manager (`TVXGIManager`).

## What Changes

- `exa_world::gi_volume`: every block the residency steps besides the
  camera's is published to the renderer (`GiNeighbourVolumes`, layout
  order); each published block carries its place in the block grid
  (`GiPick`); each stepping block gets its own sector marks and cell
  lights; a level change drops the neighbours and the texture cache.
- `exa_render::gi`: the resident neighbours get textures (reusing a block's
  kept light), their lights, their own parameters and margin copies, and
  one light step a frame after the camera block's (a new one warms up with
  a shared budget of 16 extra steps a frame). Margins copy from the
  stepping blocks. Materials bind up to nine light volumes (`GiSlots`).
- `exa.wesl`, `terrain.wesl`, `shading.wesl`: the per-object pick from the
  mesh's origin (a terrain block's centre for the terrain), no box test,
  and black for an object no resident block holds.
- Plant chunks stand at their 6 m block's centre so the pick reads the
  block's position, as the grass caller does.
- docs/RENDERING.md.

## Out of scope

- The original's per-node exception that drops the half-voxel lift
  (`flags2` bit) for ordinary meshes.
- The original's nine fixed light texture objects handed between blocks:
  Exarust keeps a block's own textures and rebinds the surfaces when the
  resident set changes.
- The outdoor ambient curve and the specular cone trace.
- docs/PLAN.md and docs/COMPONENT-MAP.md (left to the integrator).
