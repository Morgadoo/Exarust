## Why

The original's textures look more detailed than Exarust's. The menu floor
showed bricks about twice too large, because the original discards the
stored UVs of tile-grid floors and projects its own, one repeat per 6 m
cell. Textures were also filtered more softly than the original does at the
player's settings (`TextureQuality = 5`, `Supersample = 4`).

## What Changes

- Floors of tile-grid cells: every `+`-material triangle of the cell's floor
  pieces gets planar UVs u = x/6 + ½, v = ½ − z/6 (cell-local metres, after
  the piece's rotation), with tangents rebuilt. The re-mapped meshes are
  cached per mesh and placement.
- Texture sampling at the original's top quality: 16× anisotropic
  filtering, and a −0.75 LOD bias applied in the material shader (wgpu
  samplers have no bias).

## Capabilities

### New Capabilities
(none)

### Modified Capabilities
- `grid-building`: floor UVs.
- `rendering`: texture sampling.

## Impact

- `exa_world::grid`, `exa_assets::texture`, `exa_render` (`exa.wesl`),
  `docs/formats/RFC.md` and `docs/RENDERING.md`.
