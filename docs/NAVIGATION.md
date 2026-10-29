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
an obstacle. A point on a cell cheaper than 80 keeps its place; else a
best-first flood finds the cheapest cell ([Queries](#queries)). A footprint adds
63 at its centre, 32 on the eight neighbours, and 16 on the twelve
cells beyond. Each fixed tick, in the behaviour stage, every character
under its controller moves its footprint to where its feet stand: an NPC
always, the player unless it walks (`exa_game::footprints`). A plan lifts
that footprint and stamps it again. A dirty
rectangle joins another when the padded union is smaller; a ring of sixteen
holds objects as they move, overwrites its oldest slot and never merges.
A refresh drains the ring into the merged list, grows each side by three
cells, starts each rectangle's working floors from the floors as built and
raises them from the samples (a mover's vertices, voxel
`y·0.1 + ½ − base·0.1` rounded): a missing floor takes any higher sample, a
sample less than fifteen above the working floor takes it, and a sample
fifteen or more above, or at or under the lower slab's underside, raises
the upper floor when that is missing or less than fifteen below the
sample; where a working floor differs from the grid's, the floors are
written and the cost passes run again, so a mover that has gone leaves its
floor as built. A scene-wide countdown holds the merge and the refresh (the
ring still fills) for its frames, then hands the whole grid less its
twenty-cell rim in as one rectangle. With no grid there
is no path.

Evidence: reverse engineering of `Exanima.exe` 0.9.5 (the grid build, its
span, layer and cost passes, the moving props' raise, the sector mesh
registration and the body attach it reads, the scene fields, the level
loader),
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
   The build walks every mesh registered in every sector (the scene's own
   first, then the rooms; registration takes a node's whole subtree). A
   mesh whose node carries a rigid body marks nothing unless the node is
   level geometry (node flag `0x80000`, a prop placed static): attaching a
   body to its node (when the body is read with the node and when an
   object's instance copies it) sets the node's second flag word's `0x10`,
   and the stamp skips a node with that bit unless the first word has
   `0x80000`. Such meshes raise floors instead ([Moving props](#moving-props)).
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

## Moving props

Once the layers are written, and before the cost passes, every registered
mesh with points whose node carries a rigid body (of class `0x10000000`,
which every shipped body has; character parts, marked `0x400`, never)
raises the floors under it, level geometry or not. Each point goes through
the node's world matrix, pre-scaled so that x and z come out in cells
(`/30 − ½ + W/2`) and y in voxels (`·0.1 + ½ − base·0.1`), rounded to the
nearest. A point in cells 1 to `W − 3` (and `D − 3`) and voxels 0–254
raises its cell, one point after another, by the refresh's rule: a missing
lower floor takes any higher point; a point less than fifteen above the
lower floor takes it; one fifteen or more above, or at or under the lower
slab's underside, raises the upper floor when that is missing or the point
is less than fifteen above it. Only the current floors change: the floors
as built, which the refresh starts from, stay. So a sack or a barrel turns
its cells into a step, a table gives an upper floor over the one under it,
and a door leaf, whose points stand a voxel apart, walks its doorway's
floor up to its top: **a closed door blocks the grid** until it moves and
the refresh puts the floors back.

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

## Queries

Besides planning, the pathfinder answers point and line questions on the
grid (heights in voxels, `h` the start's, `fl`/`ul` and `fu`/`uu` a
cell's lower and upper floor and underside):

- **Planning** (`nav::find_path`; `nav::find_path_with` takes the accept
  distance and the penalty shift): the start and goal become cells by
  rounding `cm · (1/30) + ½` ties to even; both must lie within the grid's
  half size less ten and within 253 cells of each other. A 256 × 256
  window is centred between them; its border is never entered. Heights
  are `round((y − base) · 0.1)`, ties to even. The start takes the upper
  layer when its height is above the upper underside less 3 and below its
  floor plus 15, otherwise the lower one; every step reads the floor of the
  node it leaves, on that node's layer. Neighbours go north-west, north,
  north-east, west, east, south-west, south, south-east; a step needs a
  lower floor, may rise at most four floor units and fall less than 21. A
  lower floor more than four above is refused unless its slab leaves ten
  of room, and then only the upper floor can take the step; an upper floor
  more than four above passes when its slab leaves ten of room (an empty
  underside with a floor not below the lower one also passes) and keeps
  the lower floor; an upper floor within four and above the lower one is
  entered. The step pays `10·Δ² + 5 or 7 + 2·clearance + (penalty << shift)`,
  refused when the shifted penalty is over 600 or the clearance over
  `180/(size + 1)`. The open list is LIFO, popped by the smallest total
  (compared unsigned) with the newest winning a tie; a node met again more
  cheaply keeps its place and a node already taken is not put back. The
  heuristic is `round(10 · √(Δx² + Δz² + Δh²))`, its height leg from the
  node's floor to the goal's height. The search stops once the closest
  node's heuristic is under the accept distance (`nav::find_path` uses 10,
  the goal's cell), with an empty open list, or at the 4096th node (the
  accept test still runs), and answers the closest node. A search that did
  not reach the goal counts as reached when that node's centre lies within
  100 cm of the goal; otherwise the path stops at the parent of the rough
  node (clearance over `64/(size + 1)`) nearest the start. The parent chain
  keeps the last node of each run of one direction and layer; from the
  start, each corner after the next is offered to the budget line walk
  back to the last kept point, and the corner before it is kept when the
  line is blocked. The line walk holds the far corner's floor, steps every
  cell of the longer axis to the kept point and is blocked by a refused
  cell or a cost (the penalty shifted once more) over what the search paid
  between the two plus ten. Points sit on cell centres at the node's layer
  floor; fewer than two corners give no points. Each thread keeps one set
  of window buffers, and a search clears only the window's interior.
- **Disturbance** (`nav::disturbance`): the original's sampler is a stub
  that always answers 0.
- **Line walk** (`nav::line_walk`; the player's look-ahead, soft stop and
  footwork targets): one cell a step along the line's longer axis; a floor
  more than 4 above the running height blocks when less than 10 lie under
  its slab (an upper slab with nothing measured under it and its floor not
  below the lower one passes); a reachable floor raises the running
  height; drops never block. Blocked, the end moves back to where the line
  passes the last free cell's centre (the start when the first cell
  blocks), keeping the asked end's height.
- **Straight walk** (`nav::straight_walk`, a walker without feet): from the
  floor under the start, 5 up and 15 under, following the floors down too;
  it reads the floors as built, not those movers raised.
- **Obstacle** (`nav::obstacle`): none on lines of 40 cm or less; longer,
  a floor more than 7 up with less than 15 under it, or, before the
  line's last two cells, a penalty over 60.
- **Cost** (`nav::cost_at`): the clearance and penalty bytes over 255 on
  the layer whose slab the point's height meets (the upper one when the
  height lies above its underside less 3 and below its floor plus 15).
- **Nearest walkable point** (`nav::nearest_walkable`): the layer is the
  one the walker stands on (none without a floor, the upper one with
  fewer than 11 between the lower floor and the upper slab, the lower one
  with fewer than 11 under the lower slab, else the nearer floor). With a
  layer whose clearance plus penalty is under 80 the point stays, on its
  floor. Else a best-first flood in a 256-cell window round the point's
  cell: from a point off the floors (no layer, or clearance 255) first the
  nearest cell across the ground that has a floor (not open sky, with
  more than 10 between its floor and the other layer's slab); then, from
  the cell, each neighbour in the original's order the line walk's reach
  steps onto (under 50 cm up, under 2.1 m down), keyed by its distance
  from the point (×10, in 3-D), keeping the cheapest; it stops at a cell
  under 80, with nothing left, or after 220 cells, and answers that
  cell's centre on its floor. Without a grid the point stays.
- **The spawn snap**: on its third behaviour tick, unless down, the player
  is moved onto the nearest walkable point (across the ground; the
  original does it once, not on its first three ticks).

Exarust leaves out two quirks of the off-floor search: the original adds
to every distance the square of a register it never sets there (the same
for every cell, so it changes nothing but the bound), and its met map is
a shared buffer cleared only ±15 cells round the centre; Exarust starts
each search with a fresh map.

## Exarust

- `exa_world::nav::NavGrid` is the grid (a resource once built),
  `NavSource` what it is built from, and `NavPlugin` builds it for each
  level when the load ends, from what is under the level root, on the
  async compute pool. The log gives the inputs and the counts.
- The game's one pathfinder is `nav::InstalledPathfinder`: the level's
  grid once built, or the abstract 64 × 64 fallback while the grid is
  being built or when the level has none. The fallback answers every
  query with nothing (no obstacle, no clamped line, no snap, no reach,
  zero cost, the point unchanged), which is why a level without a grid
  never blocks the player's look-ahead. `NavPlugin` installs the
  fallback for each new level and the grid when its build finishes; the
  fallback needs no teardown (Rust drops it).
- The NPC chase route (`exa_ai::chase`, `exa_ai::route`) plans through
  `nav::find_path`, one brain a frame (the original's plan turn), and is
  followed when its ends lie more than 150 cm apart.
- `exarust --debug-nav` (or F6) draws it ([DEBUGGING.md](DEBUGGING.md)).
- `exa-tool nav <level> [--png file]` builds a level's grid without a
  window, prints its counts and draws its layers to a picture.

What the installed game's levels give (`shipped_levels`, the reach counted
from the level's start by the planner's step rules: under 50 cm up, under
2.1 m down, onto cells size class 0 passes):

| level | cells | floor cells (lower, upper) | open sky (lower, upper) | reached from the start |
|---|---|---|---|---|
| `arena` | 640 × 640 | 1956, 10 | 516, 0 | 1861 |
| `arenao1` | 640 × 640 | 14677, 3678 | 2867, 270 | 5005 |
| `exanima01` | 640 × 800 | 220850, 2279 | 99982, 83 | 46113 |

In exanima01 the start's rooms are closed by their doors, whose leaves
raise the doorways' floors ([Moving props](#moving-props)): the walk from
the start meets 46 113 cells. On the floors as built (every body away, as
when the doors stand open) it meets 195 049 and reaches the foot of the
stair room (8.9, −1, 64), whose floor reads voxel 211, a metre under the
corridors, down steps of 10–20 cm. Before `place-node-records-in-the-level-frame`
the doors' leaves, like every node a placement's records move, stood
45–90 m from their doorways, so the doorways were open (191 331 cells).
Its terrain fills only its 682 flagged blocks: of the cells over its 477
closed grid cells (the rock between the rooms) 1120 keep a floor, at the
walls' edges (1555 while the records heaped props in the rock). Filling
the whole tiles instead (what Exarust did before
`build-nav-terrain-only-where-blocks-are`) made the rock's ground, at
about 0, floor too: 186 721 such cells. arenao1 fills its 34 flagged blocks of
its one tile. The counts follow the terrain's height pass over every
material a tile uses (`layer-every-terrain-material`); with only the four
heaviest, arenao1 reached 5050 cells and exanima01 190 954.

Not reproduced, with what it changes:

- The stamp skips nodes flagged `0x10` in their second flag word unless
  their first has `0x80000`. Exarust applies it to the nodes whose rigid
  body sets the bit, not to the 30 of the 3310 Resource meshes with points
  that carry it in their file (mostly creature parts), which it stamps.
- The original also gives some floor-set details (children of the
  centre-floor variants flagged `0x10000` or `0x8`) generated surface
  points; Exarust takes them only into the floor's heightfield, as its GI
  does.
- The original's refresh puts a moved body's cells back when it moves
  (a door opened, a sack kicked away); Exarust builds the grid once per
  level and does not queue its moving bodies, so an opened door's doorway
  stays raised until the level loads again.
- Each cell's floor points come from all tile-grid floors over its columns
  rather than from its own merged mesh; they differ only where two cells'
  floors overlap.
- In a tile-grid level the original gives a tile slot the `.rft` leaves
  empty a flat tile at height 0 when one of its cells is flagged, and that
  tile's blocks fill the grid. No shipped level has such a cell, so
  Exarust has no such tile ([formats/TERRAIN.md](formats/TERRAIN.md)).
