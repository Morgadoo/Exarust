# Proposal

## Why

The factory and procedural-material work (`FLX_ItemFactory`,
`FLX_Materials_Procedural`, `TProcMaterial`) was extended by a first pass
that left rules off the original's instructions: a random set read as
decimal, the copper colour class, the stat pass's second curve (it ignores
the deform's amount), the child's scale, the component box, the bend's
trigonometry, and many terms of the material programs (the rust weight's
fourth power, the blade's own specular and normal rules, the gems' maps).
The factory paint job also painted with the apparel ramp, without the
finish level, the damage remapping, the atlas tiling or the seed's tile
offsets. Phase 4 (Combat and items) and Phase 5 (Visual fidelity: weapons
and apparel are painted at load and seen in every level).

## What Changes

- `exa_formats::factory`: the colour roll by material class (shared with
  the item palette), quality and offsets in double, the 32-bit exclusion
  bit, the stored flag after a full roll, the engine's packed sine and
  cosine and epsilon in the bend, a total radial deform on axis 4, the
  parent's recipe scale for the attach point.
- `exa_formats::procedural`, `palette`: random set 8, the wear table, the
  plates' rust grain, the ramps' precision, the lifted colour bytes.
- `exa_formats::weapon_stats`: the second curve without the amount, the
  double-precision terms, no flipped-side deform rules.
- `exa_assets::paint`: the tile rules (`MatTile`, `DmgTile`, region),
  per-layer tint and wear weight, the specular pass after the normal pass
  reading the painted normal map, and the program terms checked against
  their sources.
- `exa_assets::weapon`/`model`: the factory paint job (`FactoryPaint`),
  stored node boxes, the engine's Euler order.

## Evidence

REA (Ghidra) on the factory roll, the replay and store paths, the stat
pass, the assembly, the paint-job builder and the job runner, the
material registry and its texture loads, the colour roll of the item
description unit, the palette ramps and HSV conversion, the packed sine
and cosine and the pass draws' uniforms; the decoded texture programs
(`textures.flb`); the 106 shipped factories (parse, roll, stored choices,
assembly of every part with a mesh and a painted material).

## Out of scope

The shared atlas (each part keeps its own painted textures), texture-only
overlay parts, emblem layers and pattern opacity, the texture-only
deform's `UVScale`, the merged mesh and its normals, the rigid body and
impact sound, the low-quality half atlas, GPU filtering in the painter,
and the stale cached-factory state the original reads in edge cases.
