## Why

The original always renders with voxel global illumination. It is a light
volume of 20 cm voxels where torchlight spreads through the open space, and
it gives interiors their warm fill. Without it, Exarust's dungeons stay dark
and cold beyond the reach of each light.

## What Changes

- Read each mesh's GI sample points (chunk `0x3D0CEC05`, a 40 cm lattice
  over its surfaces).
- Voxelise a level into one GI block:
  - 256 × 256 × 64 (or 128) voxels below the level's highest point;
  - each sample point splats weight 20 trilinearly;
  - a soft lid of 26 and 5 on the top two slices.
- Propagate the light on the GPU with the original's rules:
  - one step per frame, after a warm-up at load;
  - 8-bit sRGB storage;
  - the eight diagonal neighbours weighted by openness;
  - the base ambient from level record `0x30`;
  - direct light from the level's lights.
- Shade surfaces from the volume:
  - four taps 20 cm along the normal, with the original's lift curve, × the
    level ambient × 15 × openness;
  - reflections are the cube × that ambient × 16;
  - the environment-cube path remains outside the block.
- Viewer: `--no-gi` and `--gi-warm-up <steps>`.

## Capabilities

### New Capabilities
(none)

### Modified Capabilities
- `rendering`: voxel global illumination.

## Impact

- `exa_formats::rfc` (GI points), `exa_assets` (model nodes),
  `exa_world::gi_volume`, the new `exa_render::gi` with `gi.wesl`, and
  `exa.wesl` and `ExaMaterial` (GI volume binding).
- The viewer and `docs/RENDERING.md`.
