# Design

Extends `docs/formats/TERRAIN.md` (compositor passes). No shader or uniform
layout changes.

**Node level.** Every pass computes its mask window from the level of the
node it draws: `256 >> L` texels holding `240 >> L` cells at a block-unit
offset, and a slice starting at `b / 8` of the tile spanning `1 / 2^L`.
`terrain::composite::node_quad` and `MaskRegion::for_node` take that level;
the set passes pass the block level 3 (`block_quad`, `MaskRegion::new`), the
tile passes the root node's level 0. The root level is inferred from the
whole-tile height square and the caller's whole-tile read-back, and is
labelled so in the code.

**Quad units.** `PassVertex` carries `palette` (unit 0), `mask_uv` (unit 1)
and `vertex` (the viewport's unit square), matching the helpers' order.

**Release.** `release_terrain_caches` runs before the composite draw each
frame and does nothing but emptiness checks while a level's terrain exists,
so frame time is unchanged; the original's own release (program cache and
shared shader) stays a model in `terrain::plugins::TerrainCaches`.
