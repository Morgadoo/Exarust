# Proposal

## Why

The terrain fill let a surface at or above the GI block's top slice write
a surface byte and 20s over the lid, skipped terrain less than a voxel
below the block, and took the column floor as a minimum. The GI programs'
binding table was missing, and the base ambient had no low shading quality
path. A DeepSeek pass (lane `vxgi5`) added these but stalled after two
commits without a report; the review checked them against the assembly
and kept what matched: the double-precision corner offsets, the program
table (with its shadow units and emission sampler corrected) and the
low-quality ambient. It reverted the lane's 29-column sector floor scan
and its reordered shaping sum, both misreadings, and redid its terrain
clamp, which kept the lid's 5 the original overwrites.

## What Changes

- Terrain heights truncate towards zero; at or above the top slice the
  height clamps to it, no surface byte is written and slices 0..D−2 hold
  20 (the lid's 5 becomes 20, the top slice's 26 stays). A surface less
  than a voxel below slice 0 counts as slice 0, its negative byte stored
  wrapped. The column floor is stored as the clamped height.
- The eight propagation corner offsets are formed per axis in double
  precision and rounded once to f32.
- `GiPass::layout` keeps the three linked programs' names, sampler units
  (`Shadow<i>` on 2 + i, `LVox` 0, `OVox` 1; the emission program sets
  only `OVox`) and uniform lookups.
- `GiSettings::high_quality` picks the base ambient scale: 0.001, or
  0.002 below shading quality 4.

## Capabilities

### New Capabilities

### Modified Capabilities

- `rendering`: Terrain in the GI volume; GI program bindings; GI base ambient quality.

## Impact

- `exa_world::gi_volume` (terrain fill, sector floors, settings),
  `exa_render::gi` (corner offsets, program table).
- `docs/RENDERING.md` (Voxel GI), `crates/exa_render/README.md`.
- Evidence: the terrain fill's CVTTSS2SI height, the signed compare
  against the slice count minus one, the CVTSS2SI surface byte, the fill
  loop to k − 1 inclusive and the plain store of the column floor; the
  sector floor loops (counter from −1, tested after the body against 29,
  so 30 columns and rows); the shaping test's MULSS/ADDSS order (dz² +
  dx² first, then dy² added); the manager init's CVTSI2SD/MULSD 1.33/DIVSD/
  CVTSD2SS per corner axis; the inject program builder's `Shadow0` digit
  patch with unit counter + 2 and its `LVox` 0, `OVox` 1 calls; the shape
  builder's `MPrj` and `OVox` 1; the emission and blit builders' lookups;
  the per-frame base ambient (0.002 × 0.5 when the high quality flag is
  set); the uniform names read from the executable's string table.
- No WGSL or uniform layout change; no per-frame cost.

## Out of scope

Feeding the original's shading quality into `GiSettings` (Exarust shades
at quality 5, so the default stays high), separate GL programs (the WESL
compute pass replaces them), and the other gaps already listed for the
GI manager: nine resident blocks with margin blits, the shapes' fragment
bodies, dynamic occupancy wiring.

## Phase

Phase 2, static world: the voxel GI (lighting milestone).
