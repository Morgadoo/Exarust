## Why

Terrain still used Bevy's standard PBR material, which applies the
camera's exposure and Bevy's light units, so in exanima01 the dungeon
floor stayed almost black under its lights. Light pools, torch light and
the GI were all missing from it. The original shades terrain with the same
lighting includes as its other surfaces.

## What Changes

- Move the lighting into a shared WESL library, `exa_render::shading`:
  GGX lights, the cube-shadow filter, the sun, GI sampling and ambient.
  `exa.wesl` imports it.
- The terrain becomes its own material. It keeps the layered splat colour
  and normals, and is shaded with the shared lighting:
  - Fresnel F0 0.04;
  - the original's colour-derived terrain gloss;
  - GI or environment ambient and reflections;
  - specular scaled by min(255·r, 1).

  The GI volume and environment cube are bound to it.
- The light record read as the original reads it: the multiplier scales only
  the GI injection; the softness sets the shadow radius; spot values q, p are
  present for nodes flagged `0x200` under `0x300` and shape the light
  vertically, on surfaces and in the GI injection; node flag `0x40000` makes
  it fall off with distance.

## Capabilities

### New Capabilities
(none)

### Modified Capabilities
- `terrain`: shading.
- `rendering`: vertical light shaping.

## Impact

- `exa_render` (`shading.wesl`, `exa.wesl`, `gi.rs`, `gi.wesl`, `lib.rs`),
  `exa_world` (`terrain`, `point_light`) and `docs/RENDERING.md`.
