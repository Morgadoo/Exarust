## Context

`build-the-navigation-grid` fills the grid's columns from each terrain
tile's displaced ground (`GroundTile` on the tile's entity), over the whole
tile. `add-room-visibility` then made a tile-grid level build only the
tile's 6 m blocks whose grid cell carries plane A's terrain flag
(`0x800000`), and skip a tile with none; a tile with at least one block
still gave the grid its whole ground. In `exanima01` that is the rock
between the rooms, at about 0 m, which the grid took as walkable floor.

This change extends [`docs/NAVIGATION.md`](../../../docs/NAVIGATION.md) and
[`docs/formats/TERRAIN.md`](../../../docs/formats/TERRAIN.md) (blocks in
grid levels). Notes with addresses are git-ignored
(`context/re/master-movement.md` §19, `context/re/black-fill.md`).

## Goals / Non-Goals

**Goals:** the grid's terrain from the same blocks the original's grid
reads, with the same samples it took before for every block that exists.

**Non-Goals:** the original's flat tile for a flagged empty tile slot;
terrain height queries and collision; anything else in the grid build.

## Decisions

- **Walk the blocks, as the original does.** The original's terrain fill
  is a loop over each tile's built blocks: a block's first cell is half
  the grid plus its bounds' corner over 30 cm, rounded; its 20 × 20 cells
  sample the tile at vertex (i + ½)·1.5 + 30·block index. Exarust fills
  per present block the same way. For a block that exists the samples are
  the whole-tile fill's (the vertex step 0.3/0.2 is exactly 1.5 in single
  precision, and every corner is a whole number of cells), so levels
  without a grid and present blocks give the bytes they gave before: the
  corpus test rebuilds exanima01 with every block and gets the old
  counts.
- **The presence on the ground, not recomputed.** `GroundTile` carries the
  `TileBlocks` bits the terrain build already worked out from the grid, so
  the renderer and the grid cannot disagree; a tile the terrain skips has
  no ground, as before.
- **The empty-slot tile is left out.** In grid mode the original's
  presence builder also creates a tile, flat at height 0, for a tile slot
  that has a flagged block but no stored tile. No shipped level has one
  (all 17 levels with a terrain and a grid keep every flagged cell inside a
  stored tile), so Exarust neither draws nor fills one; documented in
  `docs/formats/TERRAIN.md`.

## Open questions (answered)

| question | answer | how |
|---|---|---|
| Does the grid read every block of a tile or only the built ones? | Only the built ones: it loops over the tile's block list, which the block builder appends to | REA: the grid build's terrain loop and the block builder |
| Does grid mode build only flagged blocks? | Yes; the presence builder builds flagged blocks and removes unflagged ones; without grid mode all 64 per tile | REA: the terrain build and the presence builder |
| Is the terrain built before the grid? | Yes: the level load builds the terrain, then calls the load-end hook the pathfinder installs, which builds the grid | REA and `exe.asm`: the level load and the hook's install |
| Do shipped levels have flagged cells in empty tile slots? | No (0 in all 17) | corpus test `shipped_levels_flag_no_empty_tile_slot` |

## Risks / Trade-offs

- The reach from exanima01's start drops from 338 283 to 190 954 cells:
  the walk no longer leaves the rooms over the rock. Nothing reads the
  grid yet (M5 adds the queries), so no behaviour changes in game.
