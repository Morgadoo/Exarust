# Design

Extends [docs/formats/TERRAIN.md](../../../docs/formats/TERRAIN.md) and the
Terrain section of [docs/RENDERING.md](../../../docs/RENDERING.md).

## Decisions

- **Block meshes from the shared patch.** The original's Init builds the
  patch over (tile cells / blocks) = 30 cells a side, flattens it with
  scales (1, 241) and draws each block from vertex 30·(241·by + bx).
  Exarust adds the same base to the flattened patch per block and leaves out
  the quads whose low corner is a hole (the smallest index of a quad is its
  low corner). The skirt stays an Exarust addition.
- **Worker.** One `AsyncComputeTaskPool` task builds a tile's heights,
  weights, masks and block meshes; the plane cache is shared behind a
  mutex. When a tile finishes, the next built tile starts at once, so the
  main thread's texture loads and spawns overlap the next build. The
  original's one worker thread runs through every tile; building ahead
  changes nothing on screen.
- **Height query.** The original's arithmetic is in centimetres with its
  1/4800 constant and the engine's round-down conversion (MXCSR toward −∞,
  NaN to the integer indefinite). The slot is checked as one index, so a
  coordinate past a row's end reads the next row's tile, as in the
  original.
- **Streaming rules** are a model with tests: the original's compares are
  kept (NaN in the far test is kept, NaN in the near test is low; the
  rank scan lowers or else raises), the focus moves −1000 cm along the
  camera's back axis.
- **Lit program.** `shadetrn_f` forms `difCol = DifTex × MatDif` before
  the specular mask, so the mask is after the 0.75; the normal and the
  parallax view vector use `ax = normalize(X − s·s.x)`, `ay = cross(ax, s)`.
  Roughness is `1 − G`; at full gloss the GGX term is 0/0 where N·H = 1,
  and a 1e-30 floor on its divisor keeps that point at 0 without changing
  any other value.

## Open questions

- Whether a missing gloss map's layer uses the flat gloss factor (the
  composite's untextured path mixes toward `Colr`): answer by reading the
  gloss pass's program word build with REA.
- Whether the palette's header float reaches the draw's environment (the
  wet-floor flag): answer by tracing the scene environment's writer with
  REA.
- Whether the original's height buffer holds displaced heights: answer by
  a capture of the running game's ground query on a painted tile.

## Risks

- One mesh per block multiplies terrain entities by up to 64 per tile; Bevy
  batches them per material and culls them per block, as the original
  draws and culls blocks.
