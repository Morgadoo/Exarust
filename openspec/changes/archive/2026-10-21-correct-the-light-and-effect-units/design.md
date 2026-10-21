# Design

Documentation: [`docs/RENDERING.md`](../../../docs/RENDERING.md) (Lights,
Skill-tree splines, Blood).

## Rules checked against the disassembly

- Float sums keep the processor's order (the directional view matrix sums
  its translation as `z + (y + x)`), the normalising step uses the
  reciprocal square-root estimate, and single-precision converts only where
  the original does (the spline curvature runs in double precision).
- Conversions that round use ties to even and keep the indefinite value for
  inputs that do not fit; the spline fade keeps only the low byte.
- Table loops are read as written: the spline index table runs one quad
  past its constant.
- Sizes: the allocator and the cube draw size a map separately, so both are
  kept; the hardware cap compares signed numbers.

## Wiring

Only the voxel light constructor is wired (`VoxelLight::from_record`, so
the radii and flag word exist on spawned nodes) and the loader's fallback
material. Camera planes, the material manager, the shadow plumbing, the
splines, the particle table and the spray node description are data with
tests but no caller: applying them would change lighting, frame time or
needs systems Exarust does not have yet.

## Removed after review

A reset copied onto the blood sprays is the weapon-blood list's count
clearing (`exa_combat::weapon_blood`); the invented noise buffer and the
duplicate convex volume had no original behind them.
