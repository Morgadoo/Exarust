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

### Tile (`0xBDAD`)

| type | field |
|---|---|
| u32 | flags (`0x200`: has a hole mask) |
| u32 × 2 | grid x, y |
| u32 | unknown (differs per tile, perhaps a checksum) |
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
coarsest octave weighs most.

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
evaluates the same weights per vertex and layers the four heaviest materials
of a tile in a shader. Normal maps are layered the same way, with the N-pass
sharpness (f4, f5) and strength (f11). Gloss uses the scalar f10 without the G
texture. Not yet reproduced: the G texture, the height displacement of the
mesh (from the H pass), and the grass placed from the plant records.

### Plant (104 bytes)

`u32 flags`, `name[16]` (an object, e.g. `Trn_GrassA01`), then 21 f32
parameters (colours and sizes). Not interpreted yet.
