## Why

The original's floors and walls show deeper relief than Exarust's. Its
lit program displaces texture coordinates with the material's height map
(parallax) and shadows each light by the height map (displacement
shadows), both on at the player's shading quality. Exarust ignored height
maps.

## What Changes

- Parse each texture layer's amount, the material's extra flags (`0x4C`)
  and the displacement-shadow step and bias (`0x5C`, `0x60`).
- Parallax: the original's depth rule (a fixed number of height-map
  texels, from the height layer's amount and the map's size), and four
  accumulating steps along the tangent-space view. Every map reads at the
  shifted coordinates.
- Displacement shadows on lights and the sun: taps marched toward the light
  in the tangent plane, as the original's lighting include does (sixteen in
  all, as its four supersampling passes give).
- Parallax without displacement shadows darkens the ambient in hollows.

## Capabilities

### New Capabilities
(none)

### Modified Capabilities
- `rendering`: parallax and displacement shadows.

## Impact

- `exa_formats::material`, `exa_assets::model` (material build),
  `exa_render` (`ExaMaterial` height map, `exa.wesl`) and
  `docs/RENDERING.md`.
