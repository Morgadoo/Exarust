# Proposal

## Why

The voxel GI injected the 32 lights nearest its block into every voxel and
propagated the whole block every step. The original works sector by
sector. It marks the 6 m cells the camera needs, takes each sector's lights
from its cell's flood list, groups sectors with the same lights, and clears
everything it does not draw. A first port of this (by another agent) had
the right outline but differed from the instructions in several rules. Its
second marking pass drew unbuilt cells and stepped rows by the width. Its
view test had no radius test and a wrong sign rule. Its grouping compared
light lists in order and merged without the fill estimate. Its level
without a grid used the marks and floors. Its block placement put odd grids
3 m off their cells.

## What Changes

- Sector marks: two passes over the cells within six of the focus cell, in
  reach and built, with the original's box test (radius, signed half
  extents, centimetres) and the second pass's ±1 / ± grid-height
  neighbours.
- Sector groups: light sets compared as sets; merges gated by the fill
  estimate; the spot-shaping test in the original's sum order and
  threshold; a level without a grid drawn as one group of 64 sectors.
- Lights: every registered light is fed, over its stored radius, and an
  unranged light floods the cell lists with unlimited reach.
- Unmarked sectors, margins and the top slice are cleared each step; voxel
  lights still add into them. Base ambient per channel.
- Rounding as the processor does it: splat shares, terrain fractions and
  voxel-light corners round half to even; the focus cell is a floor.
- One kept uniform buffer for the pass; no per-sector allocations.

## Capabilities

### New Capabilities

### Modified Capabilities

- `rendering`: GI light selection; GI sector marks, box test and mark widening; GI undrawn voxels; GI sector groups and merges.

## Impact

- `exa_render::gi` (`gi.rs`, `gi.wesl`), `exa_world::gi_volume`,
  `exa_world::shadow_links`, `exa_world::light_flood`, `exa_world` level
  load.
- `docs/RENDERING.md` (Voxel GI), `docs/PERFORMANCE.md`, crate READMEs.
- Evidence: REA/Ghidra pseudo-code and assembly of the manager's sector
  marking, light grouping, block layout, per-frame injection and setup,
  splat and terrain columns, the engine's convex-volume box test, its round
  helper (MXCSR set toward −∞) and its sign helper;
  `context/re/gi-occupancy.md`, `context/re/light-flood.md`,
  `context/re/light-record.md`.
- Picture: sectors out of view or past the reach go dark until marked
  again (as in the original); unranged lights' GI falls off over their
  stored radius instead of 400 m; base ambient takes the record's three
  channels. No retail capture was taken.
- Out of scope: several resident blocks and their margin copies, the sun
  and its shadow map in the GI, the injection's cube-shadow term, emissive
  GI shapes, dynamic occupancy updates, the top-slice flag of the level
  environment record, the `(1 − dim)²` factor (0 for shipped point lights).

## Phase

Phase 2, static world: the voxel GI (lighting milestone).
