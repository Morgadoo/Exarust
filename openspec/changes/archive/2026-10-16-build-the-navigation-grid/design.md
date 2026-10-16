## Context

The original's `TPathFinder` is attached to the level when it loads and
builds its grid from the scene: the terrain, then every static mesh node,
then the spans, layers and cost layers. Exarust spawns a level a step at a
time behind its loading screen (`LevelLoad`), and its meshes carry the GI
sample points (`GiPoints`), its tile-grid floors are marked (`GiFloor`) and
its terrain tiles are built from the `.rft` with the height pass applied.

This change adds [`docs/NAVIGATION.md`](../../../docs/NAVIGATION.md) and
extends [`docs/formats/RFC.md`](../../../docs/formats/RFC.md) (Environment).
Notes with addresses are git-ignored (`context/re/master-movement.md` §10,
`gi-occupancy.md`, `master-plan.md` D3).

## Goals / Non-Goals

**Goals:** the grid's size, height reference, columns, spans, layers and
cost bytes as the original's, from the same inputs; built at load without
stalling the window; a way to look at it in game and without a window.

**Non-Goals:** queries and planning (M5, M6), footprints, moving props,
dirty rectangles.

## Decisions

- **Points, not colliders.** The master plan proposed voxelising
  Exarust's triangle-mesh colliders until RB28 was answered. The exe
  answers it: the grid stamps each node's GI sample point array (the
  pointer and count the GI gather reads), plus the heightfield points the
  grid-cell builder generates for each cell's merged `+Floor` (with the
  node's extra height 0.5 voxel). So the grid reads `GiPoints` and
  regenerates the floor points from the `GiFloor` meshes. Walls of sets
  without points stay out of the grid, as in the original.
- **Floor points on the world lattice.** The original runs the generator
  per cell in the cell's space. Cell centres are odd multiples of 300 cm
  and its bounds snap to 20 cm, so its 10 cm columns and 20 cm cubes fall
  on the world's lattice: one world-wide pass over all floor faces gives
  the same points except where two cells' floors overlap.
- **Terrain from the displaced vertices.** The original samples its tile
  vertex array, which the height pass has moved. Terrain tiles keep their
  displaced heights and holes (`Ground`, shared); the headless build reads
  them from the same spawn.
- **The span scan as the original's.** The exe scans a column a byte at
  a time with a cursor on one bit; where the rest of a byte holds nothing
  of what it seeks it takes the voxel above the cursor as the change, and
  it skips a whole free or solid byte without moving the cursor's bit. So
  undersides step down to their byte's bottom, the room above a slab is
  counted from there (a ceiling 13 voxels over a floor can count 11 and
  join it), and a byte's top voxel can be missed after a skip. These
  change which floors exist, so `column_spans` scans the same way rather
  than voxel by voxel.
- **Centimetres for rounding.** The grid's rules round to nearest (ties to
  even, the exe's `cvtss2si`). Exarust's metres carry float noise that
  would decide exact ties at random; coordinates are converted to
  centimetres snapped to 10 µm first, so lattice points on cell or voxel
  boundaries round as the exact ties they are. Which side a tie falls on
  can still differ from the original's single-precision sums.
- **Off the main thread.** As the GI does: `LevelLoad::finish` inserts
  `PendingNav` (root, size, base); a `PostUpdate` system after transform
  propagation collects shared copies (points with their world transforms,
  floor mesh copies, ground tiles) and spawns the build; another inserts
  the `NavGrid` resource. A new level drops a build in flight.
- **Headless build.** `nav::headless::level_source` spawns the level into a
  bare world (as the level-load test does), works out world transforms
  from the parents and collects the same inputs: one path for `exa-tool
  nav` and the corpus test.
- **Overlay around the view.** Drawing 1.6 million cells is not possible
  with gizmos; the overlay draws the cells within 8 m of the floor the
  camera looks at, one square each.
- **Kept bytes.** Roughness and blur stay in the grid (the original keeps
  them for its dirty-rectangle rebuilds), and the penalty layers start at
  zero for M6's footprints.

## Risks / Trade-offs

- Exarust stamps every mesh node with points under the level root; the
  original only the top-level objects' direct children, skipping nodes
  flagged `0x10` (second word) without `0x80000`. 30 of 3310 meshes with
  points carry that flag, mostly creature parts; level props rarely.
- Exarust's props are static geometry, so they mark voxels; the original's
  movable props instead raise the floors they stand on. They converge when
  props become bodies (M32) and M6 adds the raster.

## Open questions (and how they were answered)

- Grid top for levels whose `0x90` is 0 (arenas 1, 2, 4, 5, 6): REA on the
  level loader, which copies `0x98` into `0x90` when it is 0.
- Origin of the stamped vertex sets (RB28, U8): REA on the build's stamp
  (the node's point count and pointer) against the GI gather; the floor generator and
  its 0.5 extra height from the grid-cell builder.
- The terrain sampler: REA (bilinear over the tile's vertex array, the
  hole bytes of two rows tested at the column's bit and the next).
- Whether the region flood matters: REA (it writes only a count).
- The span scan's exact walk (cursor, byte skips, the change taken where
  a byte has nothing left): the scan's instructions read in `exe.asm`, and
  an emulation of them compared with `column_spans` over 222 000 columns.
- Corpus numbers: `exa-tool nav` and the corpus test on the installed game.
