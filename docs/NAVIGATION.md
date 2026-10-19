# Navigation grid

How the original finds its way around a level, and what Exarust has of it.
The original has no navigation mesh: its pathfinder (`TPathFinder`) keeps a
grid of 30 cm cells built from the level at load, with at most two floors
per cell, and plans over it with A\*. The player's wall look-ahead, combat
footwork and every NPC path go through it.

Exarust builds the grid as the original does (`exa_world::nav`, changes
`build-the-navigation-grid` and `build-nav-terrain-only-where-blocks-are`).
The planner searches that grid (`nav::find_path`) and the look-ahead is
`nav::look_ahead` (75 + 6·v² cm, 45 + 6·v² in combat, 70 cm back). An NPC
follows the path by the same heading and desired velocity the player
writes. A line walk steps the longer axis and blocks on a floor more than
four above the current height when its underside is within ten. A straight
walk allows five and follows a floor down. A line of 40 cm or less is not
an obstacle. A point on a cell cheaper than 80 keeps its place. A footprint adds
63 at its centre, 32 on the eight neighbours, and 16 on the twelve
cells beyond. The mover refresh is not in yet. With no grid there is no path.

Evidence: reverse engineering of `Exanima.exe` 0.9.5 (the grid build, its
span, layer and cost passes, the scene fields it reads, the level loader),
and the installed levels (`exa-tool nav`, the corpus test
`exa_world::nav::tests::shipped_levels`).

## Size and height

- Cells are 30 cm, 20 to a 6 m tile-grid room: a level whose tile grid is
  32 × 40 rooms gets 640 × 800 cells. A level whose grid gives no width
  gets 320 × 320. The grid is centred on the level origin: cell `i` spans
  `[(i − W/2)·30, (i − W/2 + 1)·30)` cm.
- Each cell holds a column of 256 voxels of 10 cm. Voxel 0 tops out at the
  level's grid top less 25.5 m, voxel `v` at that base plus `v`·10 cm.
- The grid top is the environment record's `0x90`. The level loader puts
  the highest point (`0x98`) there when `0x90` is 0 (the arenas 1, 2, 4, 5
  and 6), and a level without the record has 340 cm
  ([formats/RFC.md](formats/RFC.md), Environment).

## Columns

In order:

1. **Terrain.** Each 6 m block the level's terrain builds fills its
   20 × 20 cells from voxel 0 up to the voxel of the ground's height at the
   cell centre, rounded (only within the column's 25.5 m). The blocks are
   the renderer's: in a tile-grid level only those whose grid cell carries
   the terrain flag, elsewhere every block of every tile
   ([formats/TERRAIN.md](formats/TERRAIN.md), Blocks in grid levels). The
   original's fill walks each tile's list of built blocks; a block's first
   cell is half the grid plus its corner over 30 cm, and its cells sample
   the tile at vertex (i + ½)·1.5 + 30 × the block's index. The height is
   bilinear between the four vertices around the centre, as the game has
   moved them (the height pass, [formats/TERRAIN.md](formats/TERRAIN.md)).
   A centre next to a hole has no ground. The original reads the hole mask
   a byte at a time, so it misses the right-hand vertex where that vertex
   starts the next byte (every eighth column); Exarust reads it the same
   way.
2. **Static meshes.** Every mesh node's **GI sample points** (chunk
   `0x3D0CEC05`, the same points the voxel GI splats,
   [RENDERING.md](RENDERING.md)) are put in the world and each marks the one
   voxel it rounds to: cell `round(x/30 − ½ + W/2)`, voxel
   `round((y − base)/10 + ½)`, ties to even. Points on the grid's border
   cells or outside voxels 0–254 mark nothing. Character bodies never mark.
3. **Floors.** Tile-grid cells' floors (the floor set's pieces and the wall
   pieces' `+F` strips) store no points. The original generates them for
   each cell's merged `+Floor` mesh: one ray per 10 cm column takes the
   highest upward face and marks the 10 cm cell under it (the one whose top
   is the nearest 10 cm boundary); every 20 cm cube, on the world's 20 cm
   lattice, whose four columns all mark it gets a point at its centre. A
   flat 6 m floor at 0 gets 900 points at −10 cm. These points are stamped
   half a voxel higher than other points (the `+Floor` node's extra
   height), so a floor at 0 marks the voxel that tops out at 0.

Wall sets that store no GI points (the menu's `GEWS_UWA02`) leave their
walls out of the grid, as they leave them out of the GI.

## Spans and layers

Each interior column is scanned from the top:

- A solid slab with more than 12 free voxels (120 cm) above it starts a
  span; the first slab from the top always does. A slab with 12 or fewer
  free voxels above joins the span above it.
- A span keeps its **floor**, the top voxel of its first slab (255, open
  sky, above voxel 240), and its **underside**, under its lowest slab.
- At most three spans are kept. A span whose underside is voxel 235 or
  higher is dropped, unless it is the column's last.

The original scans a byte (8 voxels) at a time with a cursor on one of
its bits, and Exarust scans the same way, because the byte edges change
the result:

- It looks for the next solid voxel, then the next free one, and so on,
  each time only at the cursor's bit and below in the current byte, and
  moves the cursor under the voxel it finds. Where the rest of the byte
  has none, it takes the voxel above the cursor as the change and goes
  on. So the underside steps down to its byte's bottom: a lone voxel at
  221 has its underside at 216, a slab filled from voxel 0 up to 221 has
  217, and one filling whole bytes to the bottom has 0.
- The room above a slab is counted from the last change, which has
  stepped down the same way: a ceiling from 235 over a floor at 221 leaves
  13 free voxels, but the scan counts 11 (from 232) and joins them into
  one span whose floor is the ceiling's top. A floor at 219 (13 counted)
  keeps its own span under it.
- A byte that is all free (or all solid, while it looks for a free voxel)
  is skipped, the cursor keeping its bit. After a slab whose bottom is a
  byte's bottom the cursor sits one bit down, so the top voxel of the next
  byte with anything in it is missed: a floor at 199 under a slab at 216
  reads 198.

The top span is the **upper layer** and the second the **lower layer**; a
single span is the lower layer; a third span is dropped. Each layer keeps
a floor byte and an underside byte per cell.

## Cost

Per layer, over the grid less its border, each pass one cell further in:

1. **Roughness**: no floor 120, open sky 60; otherwise the hardest step
   onto any of the eight neighbours, weighted 256 orthogonally and 181
   diagonally, over 64. The step onto a neighbour, own floor `f`, the
   neighbour's lower floor and underside `fl`, `ul` and upper `fu`, `uu`
   (10 cm units):
   - no lower floor: 15;
   - `fl > f + 4` (its ground more than 40 cm up): 8 when `ul < f + 5` or
     there is no upper floor, else `f − fu` clamped to 8–15;
   - no upper floor: the height difference, 15 for a drop of 150 cm or
     more;
   - `uu < f + 5` (less than 50 cm under its upper slab): the rise to the
     upper floor when under 50 cm, else 8;
   - `uu < f + 15` (under 150 cm of room): 8, or the drop to the lower
     floor when it is 90 cm or more (15 from 150 cm);
   - else the height difference to the lower floor (15 from a drop of
     150 cm).
2. **Blur**: no floor 120, open sky 60; otherwise (own + Σ neighbour × weight
   / 512) / 2, each neighbour's value from the layer a walker meets: the
   lower one unless its upper floor (or the upper slab's underside) lies
   within 150 cm above.
3. **Clearance**: no floor or open sky 240; otherwise the larger of the own
   blurred value and each neighbour's (capped at 120) × weight / 512.

A walker of size class `s` may enter a cell whose clearance is at most
180/(`s` + 1). A flat floor reads 0; a cell on the edge of a drop reads 132
(size class 0 passes, class 1 does not), the next 60, then 18, then 0.

Each layer also has a penalty byte (character footprints, added at run
time with the planner).

## Exarust

- `exa_world::nav::NavGrid` is the grid (a resource once built),
  `NavSource` what it is built from, and `NavPlugin` builds it for each
  level when the load ends, from what is under the level root, on the
  async compute pool. The log gives the inputs and the counts.
- `exarust --debug-nav` (or F6) draws it ([DEBUGGING.md](DEBUGGING.md)).
- `exa-tool nav <level> [--png file]` builds a level's grid without a
  window, prints its counts and draws its layers to a picture.

What the installed game's levels give (`shipped_levels`, the reach counted
from the level's start by the planner's step rules: under 50 cm up, under
2.1 m down, onto cells size class 0 passes):

| level | cells | floor cells (lower, upper) | open sky (lower, upper) | reached from the start |
|---|---|---|---|---|
| `arena` | 640 × 640 | 1935, 0 | 531, 0 | 1856 |
| `arenao1` | 640 × 640 | 14677, 3657 | 2867, 270 | 5050 |
| `exanima01` | 640 × 800 | 219292, 792 | 101745, 104 | 190954 |

In exanima01 the walk from the start reaches the foot of the stair room
(8.9, −1, 64), whose floor reads voxel 211, a metre under the corridors,
down steps of 10–20 cm. Its terrain fills only its 682 flagged blocks: of
the cells over its 477 closed grid cells (the rock between the rooms) 1120
keep a floor, at the walls' edges. Filling the whole tiles instead (what
Exarust did before `build-nav-terrain-only-where-blocks-are`) made the
rock's ground, at about 0, floor too: 186 721 such cells, and a walk from
the start reaching 338 283 cells. arenao1 fills its 34 flagged blocks of
its one tile.

Not reproduced, with what it changes:

- The original stamps the meshes of the scene's top-level objects' direct
  children only, and skips nodes flagged `0x10` in their second flag word
  unless their first has `0x80000`. Exarust stamps every mesh node with
  points under the level root, as its GI does (30 of the 3310 meshes with
  points carry the flag, mostly creature parts).
- The original also gives some floor-set details (children of the
  centre-floor variants flagged `0x10000` or `0x8`) generated surface
  points; Exarust takes them only into the floor's heightfield, as its GI
  does.
- Moving props (physics bodies) raise the floors they stand on in the
  original's build; Exarust's props are all static level geometry until
  props become bodies, so they mark voxels instead.
- Each cell's floor points come from all tile-grid floors over its columns
  rather than from its own merged mesh; they differ only where two cells'
  floors overlap.
- In a tile-grid level the original gives a tile slot the `.rft` leaves
  empty a flat tile at height 0 when one of its cells is flagged, and that
  tile's blocks fill the grid. No shipped level has such a cell, so
  Exarust has no such tile ([formats/TERRAIN.md](formats/TERRAIN.md)).
