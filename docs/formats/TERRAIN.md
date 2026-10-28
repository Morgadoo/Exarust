# Terrain (`.rft`) and terrain palettes (`.ftb`)

Status: **verified**. All 17 `.rft` and 15 `.ftb` files in `Resource.rpk` parse
cleanly (`cargo exa corpus terrain`). The runtime meaning of each field was
recovered from the executable's terrain loader, rule evaluator and texture
passes (see `docs/RE-TOOLING.md` for the tooling). Parser:
`exa_formats::terrain`. Renderer: `exa_world::terrain`.

A level `name.rfc` that has outdoor ground ships a `name.rft` beside it. The
terrain names its palette (`<palette>.ftb`, looked up without regard to case).

## `.rft` terrain

Little-endian.

| offset | type | field |
|---|---|---|
| 0 | u32 | magic `0x3EEFAD01` |
| 4 | u32 | 0 |
| 8 | name[16] | palette name |
| 24 | f32 | vertex spacing, cm (always 20) |
| 28 | u32 × 2 | tile grid width, height |
| 36 | u32 | tile count (tiles can be missing from the grid) |
| 40 | chunks | `u32 tag, u32 size, data`; tag `0xBDAD` = tile |

Tiles are 48 m squares (240 × 20 cm). The tile at grid (x, y) is centred at
`((x − (w−1)/2) · 48, (y − (h−1)/2) · 48)` metres, on the X/Z plane, with
x along +X and y along +Z.

### Blocks in grid levels

A tile is 8 × 8 blocks of 6 m (30 × 30 quads), one per grid cell. In a
level with a tile grid (every level with a grid chunk, the outdoor arenas
included) the original builds a block only where its grid cell has plane
A's terrain flag `0x800000`: block (bx, by) of tile (tx, ty) is cell
(x0 + 8·tx + bx, y0 + 8·ty + by), with x0 = (grid width − 8 × tiles
across)/2 and y0 likewise, each halved towards zero. Each block goes to the
room of its centre ([RFC.md](RFC.md), Grid building), else to the default
sector. Without a grid every block is built. Evidence: REA on the
terrain's frame hook and the grid loader; corpus: `exanima01` flags 682
cells, all open (its 477 closed cells, the rock between rooms, get no
ground), `arenao3` a 16 × 15 rectangle around the arena, `arenao1` and
`arena7` their arena floors. Exarust drops the plants of absent blocks as
it drops those of holes [guess: the original's grass chunks were not
traced].

A tile's geometry is one 241 × 241 vertex grid (240 × 240 cells of 20 cm)
and one mesh per 6 m block, the block's 30 × 30 cell window of that grid.
The original builds one triangle list over a block's 30 × 30 cells and
shares it between every block: serpentine cell rows, row 0 along +X and row
1 back along −X, each quad split along the diagonal from its low corner
(i, j) to its high corner (i + 1, j + 1). Its indices are flattened into the
tile's 241-vertex rows, and each block draws them from its own first vertex,
30·(241·by + bx). Exarust builds each block's mesh from the same patch
(`terrain::patch`), so a room's terrain is one mesh per block rather than
one per room. Evidence: REA on the patch builder (its 30 × 30 size is the
tile's 240 cells over its 8 blocks), the patch flattener (scales 1 and the
tile's row of 241) and the block draw's vertex offset.

### Height query

The original's ground slot answers a world (`x`, `z`) in centimetres: the
tile coordinate is the floor of `width/2 + x/4800` (the engine's "round"
helper rounds down, NaN giving the integer indefinite), and the tile is slot
`tz · width + tx` wherever that lies in `0 .. width × height` (the axes are
not checked one by one, so one tile past a row's end is the next row's
first), the 6 m block the floor of the tile fraction times eight, the point
is refused where the block is not built, and its height is
the bilinear interpolation of the four vertices around it, the products and
sums in the original's order (the +X corner's product added to the low
corner's, then the +Z corner's, then the high corner's). A vertex is a hole when it or its +X
neighbour is set in its row or the row below of the hole mask, read a byte
at a time (the +X neighbour is not tested where it starts the next byte);
any corner being a hole means no ground. Results at or below −100 m are no
ground (the hole answer is −1000 m). Exarust's `TerrainGround`
(`exa_world::terrain::ground`) builds the same field from the displaced
tile heights as a level loads, and keeps each tile's block bits.

The navigation grid reads the same blocks: its terrain fill walks each
tile's list of built blocks, and the level load builds the terrain before
the grid ([NAVIGATION.md](../NAVIGATION.md)). In a grid level the original
also creates a tile, flat at height 0, for a tile slot the file leaves
empty when one of that slot's cells is flagged. None of the 17 shipped
levels with a terrain (all have a grid) has a flagged cell outside a
stored tile, so Exarust creates no such tile. Evidence: REA on the
presence builder and the tile constructor; corpus: the 17 levels
(`exa_world` test `shipped_levels_flag_no_empty_tile_slot`).

### Tile (`0xBDAD`)

| type | field |
|---|---|
| u32 | flags (`0x200`: has a hole mask) |
| u32 × 2 | grid x, y |
| u32 | vegetation seed (random when the tile was created) |
| f32 [244 × 244] | heights, row-major (rows = y) |
| u8 [31 × 241] | hole mask, only with flag `0x200` |
| chunks | `0xBD01`: painted layers |

Heights: vertex (i, j), with i and j in 0..241, sits at index
`(j + 1) · 244 + (i + 1)`. Row and column 0 and 242 overlap the neighbouring
tiles (the apron, used for normals); 243 is padding. Stored values become
centimetres when multiplied by spacing × 0.1, which is 2 for every shipped
terrain. Verified across tile seams: vertex 240 of one tile equals vertex 0 of
the next.

Hole mask: one bit per vertex (LSB first), 31 bytes per row, 241 rows. A set bit
removes the surface there. Underground levels use holes where the dungeon
pieces take over.

Layers (`0xBD01`): `u32 count`, then per layer `name[16]` (a palette rule or
material) and `u8 [241 × 241]` weights. A vertex's weights sum to about 255.
A few layers in the shipped data name nothing in their palette. They do
not count as painted, so their weight falls to node 0 (see below).

## `.ftb` palette

| offset | type | field |
|---|---|---|
| 0 | u32 | magic `0x3EEFBD03` (v3), `0x3EEFBD02` (v2) or `0x3EEFBD01` (v1) |
| 4 | u32 | 0 |
| 8 | name[16] | palette name |
| 24 | f32 | environment value; the original copies it into the scene's environment record, and the terrain passes do not read it (0 in every shipped palette) |
| 28 | f32 | wet-floor source: the terrain draw sets its specular level to it + 0.3 (unused with the gloss pass) and turns on the wet-floor flag above 0.4; 0 in every palette but `UWSwrA1`'s 0.5 |
| 32 | | zero padding to offset 144 |
| 144 | u32 | material count, then the material records |
| | u32 | rule count, then 64-byte rule records |
| | u32 | plant count, then 104-byte plant records |

Version 2 material records store 25 parameters; the loader inserts four
defaults (0.5, 0.3, 0.5, 1.0) at index 20, so the file's parameters 20–24
become the runtime's 24–28. A missing palette falls back to `default.ftb`,
with the "Could not find … (Terrain Palette)" message; a palette file that
is there but does not parse is not replaced (the loader fails).

A version 1 palette (`0x3EEFBD01`) has no 144-byte header: after the name
come the two header floats (offsets 24 and 28, as in later versions) and the
material count at offset 32, then 152-byte materials, a rule count with
64-byte rules, and no plants. REA on the loader's upgrade gives each field's
move into the runtime record (the 232-byte version 3 shape):

| v1 offset | content | runtime |
|---|---|---|
| 0 | name[16] | name |
| 16, 32, 64 | texture names | diffuse, normal, height slots (the name at 48, an old specular map, is dropped) |
| 80 | f32 | parameter 0 and parameter 1 |
| 84 | f32 height amount | above `f32::EPSILON`: the mask slot is the diffuse name with its last non-zero byte turned to `M` |
| 92, 96, 100 | f32 × 3 | parameters 6–8, times 1.4 |
| 104 | f32 offset | parameter 9, negated; 1 when that is below `f32::EPSILON` |
| 112 | f32 | parameter 11 is 1 less it |
| 116–147 | f32 × 8 | parameters 12–19 |
| 148 | u32 | flags |

Parameters 2–5 become 3, 1, 3 and 1.5, parameter 10 is 0.1 and 20–28 stay
zero (the four defaults a version 2 palette gains are not inserted). A
diffuse name whose fourth and fifth letters are `Sh` (an old shader grass)
takes `Trn_GrasA01D`/`N`/`G`/`M` and parameter 0 (and so 1) of 2; a height
name whose fourth letter is `r` or `R` zeroes parameters 16 and 17. The
palette gets three plants, `Trn_GrassA01`, `Trn_SPlntA01` and
`Trn_BPlntA01`, each flags 1 with the colours (0.72, 0.56, 0.6, 1) twice and
then 0.1, 0.1, 0.35, 1, 0.5, 0.3, 0.6, 2, 0, 1, 0, 1, 0. No shipped palette
is version 1 (two are version 2: `default.ftb`, `uwa1b.ftb`; the rest are
version 3), so the shading meaning of the old fields is not exercised.

### Material (232 bytes in v3, 216 in v2)

`name[16]`, `u32 flags`, six texture names `[16]` in slot order **D** (diffuse),
**N** (normal), **G** (gloss), **M** (blend mask), **H** (height),
**B** (fine displacement), then f32 parameters (29 in v3, 25 in v2):

| param | meaning |
|---|---|
| 0 | texture tiling: repeats = f0 × 4 per 48 m (≤ 0 means 1) |
| 1 | unused at run time |
| 2, 3 | blend-mask sharpness `s`, `t` for D, G and M |
| 4, 5 | the same for N and B |
| 6..8 | tint, sRGB; diffuse = D × linear(tint) × 1.5 × min(f9, 1) |
| 9 | cover: how strongly a masked layer hides the layers below |
| 10 | gloss intensity: G × f10 × 1.5 × min(f9, 1) |
| 11 | normal-map strength |
| 12..14 | colour of the mask pass (probably vegetation density) |
| 16..18 | height pass: f16 + (H − f17) × f18 |
| 20..22 | displacement pass: f20 + (B − f21) × f22 |

Flags (`0x1`, `0x10`, `0x40`, `0x1000`) are not read by the terrain code. They
follow surface classes (rock, grass, paved) and are probably for gameplay.

### Rule (64 bytes)

`name[16]`, `u32 flags` (unused at run time), `u32 kind`, `a[16]`, `b[16]`,
`f32 lo`, `f32 hi`. Rules and materials form one node list, rules first.
Names resolve without regard to case, and rules take precedence.

For each vertex, a rule reads `v = input[kind]`. If `v ≤ lo`, all weight goes
to child `a`. If `v ≥ hi`, all weight goes to child `b`. Between the two it
splits linearly. The kinds:

| kind | input |
|---|---|
| 0 | height, metres |
| 1 | (normal y)², normal from a 3 × 3 Sobel filter on the heights |
| 2 | a map the game never fills: 0 |
| 3 | noise map A (seed 137) folded into a ridge peaking at mid-range |
| 4 | noise map B (seed 476) |
| 5 | unused |
| 6 | an override that is never set: same as 3 |
| 7 | noise map C (seed 7359) |
| 8 | constant 0.5 |

Noise maps are 480 × 480 bytes, one sample per terrain vertex. They repeat
every two tiles and are continuous across tiles. Each map starts as white
noise from a 32-bit linear congruential sequence
(`s = s · 0x08088405 + 1`, byte = high word of `s · 255`). Then four octaves of
coarser copies of that noise (bilinear, wrapping) are averaged in, so the
coarsest octave weighs most. Map A is folded into its ridge once, after
the octaves. The exact octave, rounding and fold rules are in
[`../RENDERING.md`](../RENDERING.md) (Procedural noise). They were
established by reverse engineering of the terrain texture renderer and the
noise unit (REA), and are pinned by tests on a separate model of the rules.

Painted layer weights descend through the rules. Weight the layers leave over
(1 − Σ) goes to node 0, the first rule.

The palette's rules and materials also build a **brush table**: one entry per
name, sorted by its sixteen bytes with `A`–`Z` lowered (compared as raw
bytes, so `_` sorts below letters), rules inserted before materials, each by
binary search (an equal name met by the search is inserted before; lookups
return the equal name the search meets first). A tile takes at most
**eleven** painted brushes; at the next one found the original reports
"Maximum brushes exceeded." and reads no further layers. The base rule tree
follows them. Material weights accumulate into one 256 × 256 byte channel
each (vertex row·256 + column of the 241 × 241 tile), each weight times 255
rounded to nearest even and capped at 255. A tile whose flags have all of
bits 0–2 merges the used materials (weight above 1/510, material 0 always),
all but the first and the last, over the 241 × 241 corner, and packs them
four per interleaved mask plane. Height rules (kind 0) can be rescaled:
bound × scale + offset in single precision. The texture pass's Gaussian
blur kernel has its negative half and zero computed (sigma a third of the
radius, taken in double), mirrored, normalised by its sequential single sum
and widened to four equal floats an entry.

### Composition

Per tile, the materials used beyond 1/510 of the tile are stacked in palette
order. The first is opaque. Each later material covers what is below with the
"over" opacity `a_k = w_k / Σ_{j≤k} w_j`. When a material has an M mask, its
opacity is sharpened against the mask, `a′ = clamp((s + t)·a − s·(1 − M))`, and
what is below is scaled by `1 − a′ × cover`. Materials without textures show
their flat tint (e.g. black `Mat STint`, which fills unseen ground).

The original bakes this into per-6 m-block textures instead of evaluating it
per fragment. A block gets a set of four targets: colour (RGB8), tangent-space
normal (RG8), gloss (R8) and parallax height (R8 at half the colour's
resolution). The sizes follow the texture-detail setting: its detail texture
is 8192 texels at setting 3 or above and 4096 below, the high set's side is
`32 << (log2(size/8) − 5)` (1024² at 8192) and the low set's one level less
(512²); the displacement target is half the set's side (512² and 256²). The
pools hold 21 high and 43 low sets; a set's colour, normal and gloss share
the level and the displacement is halved. The pass targets are allocated with
the initial filter word `0x30`, which has no mipmap bit, so the composites
have no mip levels (colour and normal would gain one only if the filter's
`0x40` bit were set).

Each pass walks the tile's layers in groups of four, with one RGBA weight
upload per group; groups after the first are additive (`AddPass`) and use the
group's alpha as what survives below. The program key's low nibble is the
group's layer count, `0x10 << i` marks a layer textured, `0x100 << i` a layer
masked, `0x10000000` an additive group and `0x20000000` the normal pass's
`BaseNrm` (off at the shipped settings). The passes:

| pass | target | layer texture | per-layer values |
|---|---|---|---|
| normal | normal set | N | amount f11, sharpness f4/f5 |
| colour | colour set | D | linear tint × 1.5·min(f9, 1), alpha f9, sharpness f2/f3 |
| gloss | gloss set | G | f10 × 1.5·min(f9, 1), alpha f9, sharpness f2/f3 |
| displacement | displacement set, half size | B | f20, f21, f22, 1; sharpness f4/f5 |
| mask | mask working target | M | f12, f13, f14, 1; cover f9; sharpness f2/f3 |
| height | height working target | H | f16, f17, f18, 1; cover f9, boosted by min(1.5·a, 1); sharpness f2/f3 |

The sharpness pair travels to the shaders as `(s, t/s + 1)`. The mask pass
clears to 0 and the height pass to 0.5. The mask and height passes upload a
32 × 32 window per block from texel `(by·256 + bx)·30` of each material's
256² plane. The colour pass's quads cover a
block's palette slice at the palette scale 4 and 8 blocks a side,
`[bx, bx+1] × [−by−1, −by] · 4/8`; their mask coordinates run from
`0.5/32` to `30.5/32` of the block's 32-texel upload, so the 30 × 30 cells
are vertex-centred. The layer sampler multiplies the palette coordinates by
f0 (or 1 when f0 ≤ 0). A block's lit `TexPos` is
`(2^(level−1) − (bx >> (tile level − level)), 2^(level−1) − (by >> (tile level
− level)))`, `(0.5, 0.5)` for a whole-tile block.

The selection and the frame hook decide which block is rebaked. Blocks within
44.4 m of the look-ahead point (the camera moved 10 m along its back axis and
snapped to a 6 m block centre) join a list of at most 189; those within 15 m
want the high set and the rest the low set. A NaN distance is kept as low. The
list is sorted by a key: visible blank blocks (no composite yet) first (0),
visible blocks that want work by `0xffff − (ring − priority)` (clamped to 1),
and invisible blocks by their nearest light's rounded distance plus 0x10000.
The walk bakes every key-0 block it meets, and stops at the first block with
a non-zero key once anything changed, so at most one baked block a frame
after the blank ones; a wanted high set that cannot be had still counts as a
change. A low set comes from the pool, else from the scan down the list from
a persistent cursor, which takes the first low-set holder it meets if that
block is hidden and gives nothing if it is visible. A block that turns high
with no free high set takes one from the last selected block that holds a
high set at low resolution (state `0x900`): if that holder is not blank it
gets a low set with a copy of its composite, else it keeps sharing the high
set. A block that turns high while holding a low set (state `0x600`) trades
its low set back for a high one. A block that leaves the list hands its set
back to its pool.

Exarust evaluates the same stacking per pixel in `terrain.wesl` — the
colour, normal, gloss, parallax-height and height rules above, every material
a tile uses (up to 16) in palette order — so it has no offscreen sets, but
`terrain::composite` reproduces the sizes, pools, pass keys, per-layer values,
quads, `TexPos`, selection, sort and state transitions with the original's
numbers, pinned by tests. The palette's wet-floor source (offset 28) above 0.4
turns on the lit program's wet-floor term: the composited gloss rises by
`8c²`, capped at 0.85, with `c = max((y + 35)·−0.03, 0)² · n.y` and y in world
centimetres, so ground below −35 cm grows glossier (the sewer's 0.5 is the
only shipped source). The draw's specular level is the same float `+ 0.3`,
unused while the gloss pass is on. The `trndtl01d` detail texture is looked
up once at start-up; the shipped install has no such entry, and no terrain
pass reads its handle in any case.

G, B and the M mask are packed into one texture per material by
`exa_world::terrain::surface_image`.

The H pass is evaluated per vertex on the CPU and moves the mesh as the
original does, by `(byte − 127) × 1.25 × (2 − normal y)` cm. The opacity
there is boosted by 1.5, and each material contributes
`f16 + (H − f17) × f18`. Neutral materials (0.5) store as byte 128, so painted
ground rests about 1.25 cm above dungeon floors laid at 0 and covers them
(dirt over flagstones). Grass and plants grow from the mask pass (see
Vegetation); the plant records only give their colour.

### Plant (104 bytes)

`u32 flags`, `name[16]` (e.g. `Trn_GrassA01`), then 21 f32. The runtime reads
only the first record's f0..f2, the plants' colour (a linear multiplier); the
names and other values are editor data.

## Vegetation

One system grows grass and plants (`exa_world::terrain::vegetation`):

- **Shapes**: every mesh of `trn_grassa02.rfc` (1 grass), `trn_splnta02.rfc`
  (6 small) and `trn_lplnta02.rfc` (29 large) is a variant, all textured with
  `PlantSet_01D`/`N`. Grass is shrunk to 0.75. Grass and small plants are
  lit as if facing up: the plant loader sets their normals to (0, 1, 0)
  (the large plants keep theirs). No plant library stores vertex colours,
  and the plant program's fourth stream is the tangent, so the plants are
  not tinted per vertex. Alpha test 0.1, no fade.
  The original's grass program sways them in the wind; Exarust draws them
  still. Exarust gives the chunks no collider, and casts their shadows only
  above shadow quality 4 (the original's shadow hook).
- **Points**: one Poisson-disc set for every tile, from the 32-bit LCG
  (`s = s·0x08088405 + 1`) seeded `0x17972D3E`: radius 1.3 vertex steps
  (26 cm) over 240 × 240, 30 attempts per active point, margin r/4. The set
  is partitioned in place into 8 × 8 chunks of 6 m, rows first.
- **Density**: the mask pass layers each material's f13 (green), with cover
  forced to 1. A point reads it as a byte, d = G/255 − 0.2, or −1 in a hole.
  The mask is 240 × 240 texels: a vertex step `p` reads texel
  `round(p × 239/240)`, while the hole bit uses `round(p)`. Nothing grows
  below d = 0.35, so only materials with f13 ≥ 0.55 (e.g. `Mat GrassT`)
  carry plants.
- **Choice** (tile seed, the `.rft` tile word after x and y): for every
  point in chunk order, two steps decide: grass when (s·N)>>32 < 10 with
  N = round(50d²), then the variant. A second pass, two steps per point,
  gives rotation rand·2π (applied times 2π again) and scale
  (2d + 0.1)(0.4 + 0.33·rand), each two-product sum in double precision.
  Each chunk's growing points are then sorted by the original's key: small
  plants −1, grass 0, large plants 1 + their mean vertex height × scale.
- **Placement**: on the displaced ground of the cell, on the triangle the
  rendered patch splits it into (the low-to-high diagonal), rotated about Y
  only (the instance's matrix turns by the stored angle less a quarter
  turn), never tilted to the slope. A grass or small-plant mesh without a
  UV stream gets `(1/128, 0)` for every vertex. The plant program takes its
  normal-mapped variant only above shading quality 3 with
  `PlantSet_01N` present (Exarust's single plant shader always carries it).

Each tile also hangs a 0.5 m skirt from its edges (an Exarust addition). It
hides cracks where neighbouring tiles, keeping different materials, lift a
shared edge differently.
