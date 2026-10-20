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
| 0 | u32 | magic `0x3EEFBD03` (v3) or `0x3EEFBD02` (v2) |
| 4 | u32 | 0 |
| 8 | name[16] | palette name, zero-padded to offset 144 |
| 144 | u32 | material count, then the material records |
| | u32 | rule count, then 64-byte rule records |
| | u32 | plant count, then 104-byte plant records |

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

### Composition

Per tile, the materials used beyond 1/510 of the tile are stacked in palette
order. The first is opaque. Each later material covers what is below with the
"over" opacity `a_k = w_k / Σ_{j≤k} w_j`. When a material has an M mask, its
opacity is sharpened against the mask, `a′ = clamp((s + t)·a − s·(1 − M))`, and
what is below is scaled by `1 − a′ × cover`. Materials without textures show
their flat tint (e.g. black `Mat STint`, which fills unseen ground).

The original bakes this into per-tile colour, normal, gloss, mask, height and
displacement textures, using up to 32 materials in groups of four. Exarust
evaluates the same weights per vertex and layers every material of a tile
(up to 16) in a shader, in palette order; each vertex's over opacities go to
the shader in a per-tile texture, one texel per vertex. Shipped tiles use up
to 12 materials (11 in one 6 m block, 10 in one quad): 11 of `exanima01`'s
20 tiles use more than four, as do most outdoor tiles. Keeping only the four
heaviest dropped lighter materials and left `Mat STint`'s black showing in
their mask shapes. Normal maps are layered the same way, with the N-pass
sharpness (f4, f5) and strength (f11). Gloss uses the scalar f10 without the G
texture. The H pass is evaluated per vertex on the CPU and moves the mesh as
the original does, by `(byte − 127) × 1.25 × (2 − normal y)` cm. The opacity
there is boosted by 1.5, and each material contributes
`f16 + (H − f17) × f18`. Neutral materials (0.5) store as byte 128, so painted
ground rests about 1.25 cm above dungeon floors laid at 0 and covers them
(dirt over flagstones). Not yet reproduced: the G texture and the fine
displacement pass (B). Grass and plants grow from the mask pass (see
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
  lit as if facing up. Alpha test 0.1, no wind, no fade.
- **Points**: one Poisson-disc set for every tile, from the 32-bit LCG
  (`s = s·0x08088405 + 1`) seeded `0x17972D3E`: radius 1.3 vertex steps
  (26 cm) over 240 × 240, 30 attempts per active point, margin r/4. The set
  is partitioned in place into 8 × 8 chunks of 6 m, rows first.
- **Density**: the mask pass layers each material's f13 (green), with cover
  forced to 1. A point reads it as a byte, d = G/255 − 0.2, or −1 in a hole.
  Nothing grows below d = 0.35, so only materials with f13 ≥ 0.55 (e.g.
  `Mat GrassT`) carry plants.
- **Choice** (tile seed, the `.rft` tile word after x and y): for every
  point in chunk order, two steps decide: grass when (s·N)>>32 < 10 with
  N = round(50d²), then the variant. A second pass, two steps per point,
  gives rotation rand·2π (applied times 2π again) and scale
  (2d + 0.1)(0.4 + 0.33·rand).
- **Placement**: on the displaced ground of the cell's triangle, rotated
  about Y only, never tilted to the slope.

Each tile also hangs a 0.5 m skirt from its edges (an Exarust addition). It
hides cracks where neighbouring tiles, keeping different materials, lift a
shared edge differently.
