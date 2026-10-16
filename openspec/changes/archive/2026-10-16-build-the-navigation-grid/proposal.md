## Why

The original finds its way through a grid, not a navigation mesh: its
pathfinder builds, for every level at load, a grid of 30 cm cells with a
column of 10 cm voxels each, reduced to at most two walkable floors per
cell with cost bytes. The player's wall look-ahead, combat footwork and
every NPC path read it. Exarust has none: its player stops at walls by a
ray from the hip and its duellist walks a straight line.

Phase 6 (AI), item "navigation on tile grids and terrain"; the master
plan's lane D change M4, first of milestone MS4 "working dungeon" (M5 line
walks, M6 A\* and M7 NPC steering build on it).

## What Changes

- **Grid top** (`exa_formats::environment`): `Environment::grid_top`, the
  record's `0x90`, or its highest point `0x98` when `0x90` is 0, as the
  original's level loader fills it in; 3.4 m without the record.
- **The grid** (new `exa_world::nav`): `NavGrid`, built by the original's
  rules from `NavSource`:
  - size: 20 cells per tile-grid room, 320 × 320 without one; voxel 0 at the
    grid top less 25.5 m;
  - columns: terrain filled to its bilinear height at each cell centre (the
    original's hole-mask reading included); every static mesh's GI sample
    points marking one voxel each; the points the original generates for
    tile-grid floors (10 cm columns, 20 cm cubes), half a voxel higher;
  - spans, scanned a byte at a time as the original does (more than 12
    free voxels above, at most 3, undersides from voxel 235 dropped unless
    last, floors above 240 open sky) and two layers;
  - roughness, blur and clearance per layer (weights 256/181, the step
    measure, the layer a walker meets), and zeroed penalty layers;
  - queries for what a cell holds, its centre, the cell of a point, its
    floor's height, and whether a walker of a size class may enter.
- **In game** (`NavPlugin`): `LevelLoad` asks for the grid when a level is
  spawned; the inputs under the level root are collected once their world
  transforms are known and the grid is built on the async compute pool,
  then kept as a resource (`NavState`: none, building, built). Terrain tiles
  carry their displaced ground (`Ground`).
- **Debug helpers**: `exarust --debug-nav` (F6) draws the floor cells
  within 8 m of where the camera looks, coloured by layer and clearance;
  `exa-tool nav <level> [--png file]` builds a level's grid without a
  window, prints its counts and draws both layers.
- Docs: new `docs/NAVIGATION.md`; `docs/formats/RFC.md` (environment
  `0x90`), `docs/DEBUGGING.md`, `docs/PLAN.md` (Phase 6), `docs/INDEX.md`,
  READMEs of `exa_world`, `exa_tool`, `exarust`.

Evidence:
- Reverse engineering of `Exanima.exe` 0.9.5 (REA, Ghidra, `exe.asm`): the
  grid build, its terrain fill and point stamp (round to nearest, the
  bounds), the span scan (12, 235, 240, 3), the layer split, the cost
  passes (weights 181/256 from the exe's data, 120/60/240, the step
  measure's thresholds, the blur and dilation, their rectangles), the
  terrain height sampler (bilinear, holes), the scene's grid top (`0x90`,
  filled from `0x98` by the level loader, 340 by default), the node filter
  and the `+Floor` extra height 0.5 (`context/re/master-movement.md` §10,
  `gi-occupancy.md` §9, appended this change).
- RB28 answered from the exe: the "collision vertex set" the grid stamps is
  the mesh's GI sample point array (the same pointer and count the GI
  gathers), not the collision mesh; the plan's stopgap (voxelise trimesh
  colliders) is not needed.
- Corpus: every level's environment record and tile grid; grids built for
  `arena`, `arenao1` and `exanima01` (`exa-tool nav`, corpus test).

Out of scope:
- the queries and the planner (line walk, straight walk, obstacle test,
  cost sample, nearest point, A\*, footprints, dirty rectangles): M5, M6;
- props as moving bodies raising floors (the original's movable-prop
  raster), until props become bodies (M32);
- the region flood, whose only output is a count;
- surface points generated for floor-set details (missing from the GI too).

## Capabilities

### New Capabilities
- `navigation-grid`: the level's navigation grid built as the original's.

### Modified Capabilities
- `world-viewer`: the `--debug-nav` overlay.
- `asset-inspection-cli`: `exa-tool nav`.

## Impact

- `exa_formats` (`environment.rs`: `grid_top`)
- `exa_world` (new `nav/`: `mod.rs`, `source.rs`, `view.rs`,
  `headless.rs`, `tests.rs`; `lib.rs`: the plugin and `PendingNav` at the
  end of a load; `terrain/mod.rs`: `Ground` on each tile)
- `exa_tool` (`nav.rs`, the `nav` command; depends on `exa_world`)
- `exarust` (`main.rs`: `--debug-nav`)
- `docs/NAVIGATION.md`, `docs/formats/RFC.md`, `docs/DEBUGGING.md`,
  `docs/PLAN.md`, `docs/INDEX.md`, READMEs
