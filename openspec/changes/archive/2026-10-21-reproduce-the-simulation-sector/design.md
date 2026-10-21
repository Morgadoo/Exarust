# Design

Extends `docs/formats/RSQ.md` (contacts between bodies) and the
`exa_physics` README. No data format changes.

## Rules checked against the assembly

- Meet test: the slab test multiplies by the approximate reciprocal of the
  raw travel; the entry folds the near values y against x then z with the
  processor's maximum, an entry of 1 or more (not NaN) misses, the entry is
  clamped at 0 and the result is the sign bit of entry − exit. There is no
  0.1 cm² threshold here (that belongs to the per-sphere sweeps); a zero
  travel gives infinite reciprocals and an inside-the-grown-box test.
- Cells: the round-down helper sets the rounding mode to down and
  converts to 32 bits; the static span's edges use the default
  round-half-to-even conversion to 64 bits, the origin added before the
  value is kept to 32 bits; both multiply by the single-precision 1/600.
- The static insertion loops cover min − 1 … max + 1, z outer, x inner,
  and prepend each record to its cell's list.
- Body overflow: `index < columns`; sim objects: `index <= columns`.
- Overlap-only body test: each overhanging axis overwrites the measure.
- Static-collider pre-test: box distance against r², centre distance
  against (r + bounding radius)²; placement: centre distance against
  (r + 2 + bounding radius)², then box distance against (r + 2)².
- Body bounds: first centre as the start, strict compares, midpoint by
  lerp, no minimum extent, the epsilon just under 1.19e−7 cm².
- Largest weight: the second weight against the first.

## Deviations kept

- The approximate reciprocals are exact in Exarust; only rounding differs.
- The meet test composes rigid transforms where the original takes the
  sphere bound's world centre from the particle weights.
- Exarust lists awake parts before sleeping ones, so the all-pairs walk
  meets an awake part with a sleeper listed before it in the original.
- The narrow phase stays sphere against sphere: the mesh bounds feed only
  the broad phase and the meet test.

## Open questions

- The sector constructor's choice between the grid and all pairs (it reads
  the scene's extent): answer with REA on the constructor's callers.
- Character parts' node bounding radius: the parts take the box corner
  distance until the volume builder's node data is read (REA).

## Frame time

The pair stage computes each part's transform and world bounds once per
tick and keeps its lists between ticks. An ignored probe
(`pair_detection_timing`, 400 awake balls) measured 676 µs per detection
before the branch and 580 µs after on the build worker.
