## Why

The GI block was built once, around the level's centre. Large levels such
as exanima01 extend well beyond its 51 m, so wherever the camera looked
outside it, surfaces fell back to the grey environment-cube ambient, unlike
the original, which keeps blocks resident around the camera.

## What Changes

- Collect the level's GI points and floor triangles once, in world space.
- Keep the block centred on the point 20 m in front of the camera, rebuilt
  with a new warm-up when that point moves more than two sectors.

## Capabilities

### New Capabilities
(none)

### Modified Capabilities
- `rendering`: the GI block follows the camera.

## Impact

- `exa_world::gi_volume` and `docs/RENDERING.md`.
