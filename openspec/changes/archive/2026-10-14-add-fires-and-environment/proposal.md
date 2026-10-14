## Why

The first render pass had no warm torchlight, no environment ambient or
reflections, and showed cobwebs as blended sheets and floors as mirrors. The
original's fires, environment cube, dirt and blend words are now decoded.

## What Changes

- Fires: model nodes with a fire emitter (a torch's `_Fire`) become omni
  lights per the original's template: colour (1, 0.68, 0.43), intensity 8√s,
  radius 1500√s cm, 5 cm above the node. They flicker by the original's
  soft-bounded random walk.
- Environment cube `genref01r`: ambient is the cube at the normal (mip 5) ×
  level ambient × 16, and reflections sample it by roughness.
- Blend words decide opaque, premultiplied, additive, alpha-tested
  (cobwebs) or blended.
- Gloss maps apply when the material flags name them. Dirt maps roughen
  surfaces. Legacy materials use the original's older lighting program.
- Viewer: `--no-shadows` and `--size`. A Wayland compositor stall was worked
  around by running on X11 (`WAYLAND_DISPLAY` unset).

## Capabilities

### New Capabilities
(none)

### Modified Capabilities
- `rendering`: fires, environment lighting, blending and the legacy program.

## Impact

- `exa_formats::rfc` (emitter nodes), `exa_assets` (environment cube, blend
  modes, dirt and gloss maps), `exa_render` (shader, material), the new
  `exa_world::fire`, the viewer, and `docs/RENDERING.md`.
