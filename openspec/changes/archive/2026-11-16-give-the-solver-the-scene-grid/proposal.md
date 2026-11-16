## Why

Roadmap task B4 (lane `simsector`, docs/roadmap/phase-b-close.md), the
TSimSector rows of the function ledger. The live solver bucketed parts in
an unbounded grid around the world origin, so the original's grid step
over the scene's own grid (its size, its bias, its flat cell index, the
bounds pass's position-overflow reset) had no counterpart; the sector
constructor's choice of the grid or the all-pairs step had no caller; the
static collider grid's stand-in origin was negative, which kept every
collider in its centre's cell alone (a wall was not found from a cell two
away from its centre); the pair contact built its relative frames with a
full inverse where the original takes the transpose; the pair pass and
the pair friction worked in single precision where the original uses
double for the slop compare, the friction limit and the slide factor, and
the pair pass took a NaN limit to the full branch; the level impact gates
compared in single precision and did not skip a disabled body.

## Evidence

REA (Exanima.exe), checked against the assembly:

- The grid sector step, after the iterations and the pair list's clear:
  for each solver body, the world contact count cleared, the inverted
  tetra repaired, the frame and the mesh box rebuilt, and the cell index
  `(floor(z·1/600) + oz)·columns + floor(x·1/600) + ox` taken; an index
  below `columns` or at or past `(rows − 1)·columns` logs "Object position
  overflow" once a run, copies the tick-before-last particles over the
  current and the previous ones, rebuilds the frame, copies it to the
  previous frame and parks the body at index `columns + 1`; then the bound
  sphere and the cell migration.
- The grid build halves the scene's two grid words (unsigned shifts) for
  the bias and writes the flat neighbour table 0, −w−1, −w, −w+1, −1, 1,
  w−1, w, w+1; a sector caches the bias plus its node's cell (×1/600,
  rounded half to even). The level import gives the scene its grid chunk's
  width and height (all 29 levels have one: 24×24 to 64×64).
- The sector constructor takes the grid step when the scene word has
  `0x2000000` or `0x4000000`, the all-pairs step otherwise.
- The relative frame routine dots each axis of one frame and the offset of
  the origins with the other frame's axes (its transpose), summing x, y
  then z.
- The pair pass: depth `(b − a)·n` compared in double with 0.01 (NaN
  skips); correction `n·(float)(double(depth) − 0.01)`; travel
  `(b_start − a_start) − (b − a)`; across `t + n·(−t·n)`; limit
  `(float)((double(n_y²)·0.99 + 0.01)·double(μ))`; `limit ≥ across²` or
  unordered takes `c += across·(float)(double(limit)·−0.01)`, else
  `c −= across`; the push multiplies the share by `max(w, 0)` first. The
  pair friction's limit and its 0.01 scale are the same doubles.
- The body–level detection's impact: class bit `0x40000`, state `0x800`
  clear, `double(travel²) > 0.1` and `double(strength) > 0.1`, both
  unordered-false.

## What changes

- `exa_physics::SceneGrid` (set by the level load, reset by the
  teardown) and `sector::GridShape` (bias, flat offsets, flat index), which
  the model grid of `dynamics` now takes too.
- The pair stage takes the scene grid each step: flat cell keys, the
  park cell, the bounds pass's overflow reset (a new solver stage hook
  after the inverted-tetra repair); a scene without a grid takes the
  all-pairs broad phase.
- The static collider grid takes the scene's shape (records bucketed
  again on a change); its stand-in origin is the grid's middle.
- `sector::relative_frame` for the pair contact's sphere queries.
- The pair correction, its push and the pair friction in the original's
  precision; the level impact gates in double, and none for a disabled
  body.
