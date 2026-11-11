## Why

PLAN Phase 2 (outdoor and story levels). Terrain gap lane (round 192, tag terrain) for
TTerrainMesh, RF_Terrain (unit routines), TTerrainManager and RF_kdTrees (unit routines).
The TTerrainMesh rating named the mesh's own build as missing: a block's activation
builds its triangle list from the tile's hole mask, which Exarust approximated by
dropping whole patch quads whose low corner is a hole.

## Evidence

REA (Exanima.exe), checked in the assembly:
- The block activation sets the active bit, runs the mesh activation, sets a unit bound
  and, unless the block's built bit is set, builds its triangle list.
- The list walks the block's 30 x 30 cells row by row. For cell low vertex v it packs
  this row's two hole bits into bits 0-1 and the next row's into bits 2-3; with no bit
  of 0x7 set it emits (v+1, v, v+241), with no bit of 0xe set (v+1, v+241, v+242).
- When the count is below 1800 the block draws that list; under 200 triangles it drops
  its kd-tree (the mesh queries then test every triangle), otherwise it builds a fresh
  tree. A full block draws the shared patch with a copy of the manager's block tree.
- The tile upload activates every block not yet active before it marks the blocks built,
  so the list is built on every shipped tile. 34 exanima01 blocks are holed; exanima08
  has 159.

## What Changes

- exa_world: `terrain::patch::hole_triangles` and `BLOCK_TRIANGLES`; `build_tile`
  builds a holed block's mesh, and with it the block's static collider, from that list.
- The exa_world README.

## Out of scope

The kd-tree itself: Exarust's collider is a Rapier trimesh of the drawn triangles, so
the tree, its 200-triangle cut-off and the builder thread are acceleration only.
