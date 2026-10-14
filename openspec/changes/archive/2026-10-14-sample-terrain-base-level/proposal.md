## Why

The original composites each 6 m terrain block into textures without mip
levels, so its floors keep the full grain of the stone layers. Exarust
sampled the layers with mipmaps, so at the game's camera distance the
floor looked smoother and less detailed than the original's. With 4×
supersampling now in place, base-level sampling no longer hatches.

## What Changes

- The terrain shader reads its colour and normal layers at the base level.

## Capabilities

### New Capabilities
(none)

### Modified Capabilities
- `terrain`: texture detail.

## Impact

- `exa_world` (`terrain.wesl`) and `docs/RENDERING.md`.
