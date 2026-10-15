## Why

Exarust's frames did not look like the original's: Bevy's filmic tonemapper,
physically based point lights and its standard material gave a flat, grey,
desaturated picture. The original's colour pipeline and shaders are now
decoded.

## What Changes

- Replace Bevy's tonemapping with the original's post chain:
  - the scene clamps at 1;
  - a half-resolution bright pass and a 31-tap separable blur at a quarter;
  - the always-on grading curve, then the screen-like bloom combine.
- Add `ExaMaterial`, which shades every model like the original's main lit
  program: linear colours, legacy ×1.5 diffuse, GGX without 1/π, Schlick
  Fresnel, packed specular maps, emissive, opacity maps, two-sided.
- Read the material record's emissive colour, opacity, glossiness, specular
  level, blend word, flags and type. Rename the slots (`0x400000` opacity,
  `0x80000` emissive, `0x8000000` dirt).
- Lights in the original's units: linearised colour × intensity × 2/π,
  falling to nothing at 1.25 radii as (1 − d/range)⁴, casting shadows.
  Ambient is the level's colour × 16 × the environment map's level.
- The viewer gains `--fov`. `docs/RENDERING.md` documents the pipeline.

## Capabilities

### New Capabilities
- `rendering`: the original's colour pipeline, post-processing, surface
  shading and light units.

### Modified Capabilities
(none)

## Impact

- `exa_render` (post, material, shaders), `exa_assets::model` (materials),
  `exa_formats::material`, `exa_world` (lights, ambient, plants, grid),
  `exa_character`, the viewer; `exa_assets` now depends on `exa_render`.
