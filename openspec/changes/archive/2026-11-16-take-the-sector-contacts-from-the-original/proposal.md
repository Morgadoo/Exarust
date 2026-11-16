## Why

Roadmap task B4 (lane `simsector2`, docs/roadmap/phase-b-close.md), the
remaining TSimSector rows of the function ledger. Compared block by block
with the assembly, the sector step ticked its characters in entity order
(the player last) where the original walks the simulation's character
list from its last entry; the level hit and the pair hit callbacks took
their weights through a matrix inverse, gated them with a NaN-rejecting
test, normalised with glam's sum order, stored the level plane 0.001 m
under the point in single precision and summed the impact travel left to
right; the immediate push's split took 2φ_A first; the level friction pass
worked in single precision; a part's slip weighted the particle
differences instead of subtracting two weighted points; and the all-pairs
sector ran the sim objects' contacts before the level detection.

## Evidence

REA (Exanima.exe), checked against the assembly:

- The sector step walks the simulation's character list from count − 1
  down to 0, ticking each whose node carries the sector flag; the list's
  registration appends, the player swapped to index 0; its removals (the
  hide slot, the body's end) move the last entry into the hole.
- The level hit: squared push (x² + y²) + z² against 1e-5 (NaN goes on to
  its own check, which returns); weights by the exact barycentric routine
  (one division); the maximum chain over the weights' magnitudes against
  1.0 (`comiss`, `jp`/`jbe` keep the weights); the normal by the exact
  normalisation (squares `y² + (x² + z²)`, `divss`); the plane
  `cvtsd2ss(cvtss2sd(n·q) − 0.1)`; the travel through the paired
  weighted point; the push over the sum then the largest normalisation;
  the strain by the length routine `(x² + z²) + y²`.
- The pair hit: the same weights, gate and normal; the touched side's
  weights over its rest particles by the exact routine; `2.0 / (φ_A + φ_B)`
  (`divss`) then `mulss` by φ_A.
- The level friction: `0.5·friction` in single, `n_y²` in single widened,
  `·0.99 + 0.01` and `·μ` in double, `comiss limit, t²` (`jp`/`jbe` to the
  scale branch), the scale `cvtsd2ss(double(limit)·−0.01)`.
- The slip: the 4×3 difference routine over the current and previous
  particle sets, each product summed term by term.
- The all-pairs step: body–level detection before the sim objects' body,
  after-collision and static passes; no sim-object pre-pass and no
  simulation callback.

## What changes

- `exa_game` character tick: `CharacterList` (registration, removal, the
  reverse walk) replaces the entity order with the player last.
- `exa_physics`: `Part::weights_exact`, `Part::rest_weights_exact`,
  `engine_math::normalize_exact`; the level hit in cm with the double
  plane, the gate, the two-step spread and the length-routine strain; the
  pair hit callbacks' weights, gate, normal and split order; the paired
  smoothed travel; the level friction pass in cm and doubles; the slip as
  the weighted difference; `SolverStage::all_pairs_sector` (the pair stage
  answers for the all-pairs broad phase) moves the sim objects' contacts
  after the level detection.
- A debug timing of the solver's tick (`tests/solver_timing.rs`, ignored).
