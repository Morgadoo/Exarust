# Proposal

## Why

The coordinator approved porting the cell builder's whole floor pipeline.
The merged `+Floor` goes through the render conversion with the
vertex-cache reorder (the node does not set the keep-order bit), so its
triangle 0 (which the Ground Query ignores) is the reorder's first pick.

Evidence (REA, assembly of `Exanima.exe`): the cell builder's merge tail
(the entry frame copied over the node's matrix, the surface swap, the
append, regroup, planar pass, weld and conversion calls with their
arguments), the conversion's opt-out test, the identity entry frame
constant, and the model import (conversion only for meshes with
modifiers and the flag `0x100000`; otherwise the stored order is kept).

## What Changes

- `exa_assets::cell_floor`: node meshes as loaded (`ModelLoader::mesh_nodes`),
  `merge_cell_floor` (editable append, regroup, planar pass, weld, render
  with reorder), `floor_group_meshes`.
- The grid's `FloorMerge` lists the cell's entries and draws, collides
  and gives GI floor points from the converted mesh; the run holding its
  triangle 0 is `FirstFloorPart`. Entries ignore their node's stored
  transform. The centre variant's children are separate nodes. Built
  floors are shared by cells with the same entries.

## Impact

`exa_assets`, `exa_world`; docs/formats/RFC.md, exa_world README.
