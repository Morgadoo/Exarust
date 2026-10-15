# Proposal

## Why

The executable shows that wall pieces bring their own floor and ceiling
strips, plus occluders that are never drawn. Exarust drew the floor set's
strip as well, and showed the occluders and black ceiling strips. Terrain also
z-fought with dungeon floors because the height pass that lifts and sinks the
ground was missing.

## What Changes

- Grid builder: the direct `+F`/`+R`/`[` children of wall, corner and special
  pieces are redirected as in the original. Floor strips take the floor set's
  surface material, and ceiling strips appear only with ceilings.
- Model parts carry their file material names.
- Terrain height pass: H maps, masks and the boosted opacities displace the
  vertices, and normals are recomputed.

## Capabilities

### New Capabilities

### Modified Capabilities
- `grid-building`: piece children rules.
- `terrain`: height displacement.

## Impact

`crates/exa_assets` (part material names), `crates/exa_world` (grid,
terrain), docs.
