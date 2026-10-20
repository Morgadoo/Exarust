# Proposal

## Why

Terrain floors showed pure black holes with ragged edges: in exanima01's dirt
rooms, and wherever a tile paints more than four materials. A tile kept only
its four heaviest materials. When the black `Mat STint` (the flat tint that
fills unseen ground) was among them and a lighter material covering it was
dropped, the black showed through in the dropped material's blend-mask
shapes. 11 of exanima01's 20 tiles use more than four materials, and so do
most tiles of the outdoor levels (up to 12 per tile, 10 in one quad). The
original stacks every material a tile uses (up to 32, in groups of four) in
palette order.

## What Changes

- A tile stacks every material whose share passes 1/510 of the tile, up to
  16 (the shipped maximum is 12), in palette order, as the original does.
- The per-vertex "over" opacities move from the vertex colour (four
  channels) to a per-tile splat texture of four RGBA layers, sampled at the
  vertices' texel centres.
- The terrain shader layers up to 16 materials and skips layers with no
  cover at a fragment.

## Capabilities

### New Capabilities

### Modified Capabilities

- `terrain`: a tile stacks every material it uses, not its four heaviest.

## Impact

- `exa_world::terrain`: `TerrainSplat` binds 16 layers; `build_tile` keeps
  up to 16 materials and builds the splat texture; meshes lose their vertex
  colour.
- `terrain.wesl`: layer loops over the tile's layers.
- `docs/formats/TERRAIN.md` (Composition), `docs/RENDERING.md`, `docs/PLAN.md`.
- Evidence: the user's exanima01 capture and an overhead capture of three
  dirt rooms with black holes; `--debug-column` at a hole finds the terrain
  surface there; the shipped terrains' material counts per tile, block and
  quad.
