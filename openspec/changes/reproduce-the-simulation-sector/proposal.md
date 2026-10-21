# Proposal

## Why

The original runs its 83 Hz step one simulation sector at a time: the
sector buckets its rigid bodies and static colliders in a 600 cm grid (or
tests all pairs when small), keeps each body's sphere-set and mesh bounds,
and gates every body pair through a two-way meet test before generating
contacts. Exarust's pair stage had its own grid and a sphere-against-sphere
first look only. A first port (DeepSeek lane) added the sector module but
read several rules from the pseudo-code: a 0.1 cm² static/swept split and a
true sphere–box distance in the meet test, division by 600, a one-sided
span margin, a summed overhang in the overlap-only test, a box test where
the static pre-test compares centres, a minimum extent and midpoint rule in
the sphere bounds, and the sphere-set centre as a body's cell.

## What Changes

- `exa_physics::sector`: the grid arithmetic (round-down and round-to-even
  of × 1/600 in single precision, the processor's indefinite for NaN), the
  static span with one cell of margin all round, the body and sim-object
  overflow guards, the sparse cell grid, sphere-set and mesh bounds, the
  slab meet test, the overlap-only, placement and static-collider
  pre-tests, the point-to-mesh-box distance, the weight normalisations, and
  the step's stage order and budgets.
- The pair stage buckets parts by their mesh box's world centre, lets
  touching spheres through the first look, runs the meet test, refreshes
  both parts' world bounds after a pair with hits, and offers the
  all-pairs walk with its two point-to-mesh-box tests. Its working lists
  are kept between ticks.
- Registered props carry their node's box and bounding radius as their
  mesh bound.

## Capabilities

### Modified Capabilities

- `physical-body`: simulation sector grid, pair meet test, sector overlap
  tests.

## Impact

- `exa_physics::{sector, body::pairs, body::rigid}`, `exa_world::bodies`.
- `crates/exa_physics/README.md`, `docs/formats/RSQ.md` (contacts between
  bodies).
- Evidence: REA/Ghidra pseudo-code and the assembly of the sector's grid
  insertion, body registration, grid and all-pairs steps, the two-way meet
  test and its slab test, the overlap-only, placement and static overlap
  tests, the weight helpers, the engine's round-down helper and the
  body-bounds routine; `context/re/body-pairs.md` §2.2, §3;
  `context/re/packs/simsector`.
- Out of scope: the static-collider grid in the running game (rapier keeps
  the level's broad phase), the overflow guard's reset to the previous
  positions, the overlap and placement queries' callers, placed (kinematic)
  pairs, sim objects, contact hit events, collider masks, sphere-against-
  mesh contact generation, an automatic grid/all-pairs choice, and the
  body tick kernel (another lane).

## Phase

Phase 3, bodies: physics (contacts between bodies).
