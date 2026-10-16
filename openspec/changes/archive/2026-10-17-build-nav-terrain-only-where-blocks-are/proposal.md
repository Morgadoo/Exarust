## Why

Since `add-room-visibility`, a tile-grid level builds a terrain tile's 6 m
blocks only where the grid flags the cell as terrain-floored, as the
original does. The navigation grid (`build-the-navigation-grid`) still
filled the whole tile of every tile that keeps a block, so the rock between
`exanima01`'s rooms (and the ground outside the arenas' flagged floors) was
walkable floor in the grid, where the original has no ground at all: its
grid build walks each tile's list of built blocks, and that list holds only
the flagged ones.

Phase 6 (AI), item "navigation on tile grids and terrain"; master plan
change M4b, the follow-up of lane D's M4 `build-the-navigation-grid`
(milestone MS2 "standing man", which M5's line walks build on).

## What Changes

- **Ground blocks** (`exa_world::nav::GroundTile`): each terrain tile's
  ground carries which of its 8 × 8 blocks exist (every block without a
  tile grid; in a tile-grid level the blocks the terrain builds, the same
  presence the renderer uses).
- **Terrain fill** (`NavSource::build`): a 6 m block at a time, as the
  original walks the built blocks: each present block fills its 20 × 20
  cells from the ground's height at their centres, sampled exactly where
  the whole-tile fill sampled them; absent blocks fill nothing.
- **Corpus numbers**: `shipped_levels` pins the new counts (arenao1 14 677
  lower and 3657 upper floors; exanima01 219 292 and 792, reach 190 954
  from the start, the stair room's foot still reached) and that only
  1120 cells over exanima01's 477 closed rooms keep a floor (186 721 with
  whole tiles).
- Docs: `docs/NAVIGATION.md` (columns, numbers, the dropped guess about
  the ground between rooms), `docs/formats/TERRAIN.md` (the grid reads the
  built blocks; the original's tile for a flagged empty slot; closed cell
  count corrected to 477), `docs/DEBUGGING.md` (the `exa-tool nav`
  example), `docs/PLAN.md`, the `exa_world` README.

Evidence:
- Reverse engineering of `Exanima.exe` 0.9.5 (REA, Ghidra, `exe.asm`): the
  grid's terrain fill loops over each tile's block list; the block builder
  appends to that list, and in grid mode the presence builder builds only
  flagged blocks (and removes unflagged ones), while without grid mode all
  64 blocks of every tile are built; the level load builds the terrain
  before it calls the load-end hook that builds the grid.
- Corpus: all 17 shipped levels with a terrain and a grid: every flagged
  cell lies in a stored tile (the original's path that creates a flat tile
  for a flagged empty slot never runs); `exa-tool nav` pictures and counts
  of exanima01, arenao1, arenao3, arena7 and arena before and after.

Out of scope:
- the original's flat tile for a flagged cell in a tile slot the `.rft`
  leaves empty (no shipped level has one);
- the height query and collision of absent blocks (nothing in Exarust
  reads them yet);
- props as bodies raising floors (M32), the grid queries (M5).

## Capabilities

### New Capabilities
(none)

### Modified Capabilities
- `navigation-grid`: terrain only under the built blocks; the shipped
  levels' counts.
- `asset-inspection-cli`: `exa-tool nav exanima01`'s floor count.

## Impact

- `exa_world` (`nav/source.rs`: `GroundTile::blocks`, `with_blocks`,
  `has_block`, the block-wise fill; `terrain/mod.rs`: the tile's ground
  takes its blocks; `nav/tests.rs`)
- `docs/NAVIGATION.md`, `docs/formats/TERRAIN.md`, `docs/DEBUGGING.md`,
  `docs/PLAN.md`, `crates/exa_world/README.md`
