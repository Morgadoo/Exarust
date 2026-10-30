# Proposal

## Why

Exarust's world material sampled no fog, so outdoor arenas had none on
their surfaces, and a lightning strike showed nowhere: the strike tints the
environment's record fog colour, and Exarust drew that colour as the fog's
(Bevy's distance fog, which the world material ignores). The original's
world programs take a different fog colour, and the record fog colour feeds
something else entirely.

Evidence (REA, Ghidra listing of the 0.9.5 build, the objdump listing, and
the shader library dumped from `shading.flb`):

- The global uniform block's `FogColr` is the environment's scratch colour
  (the horizon under a sky, the fog colour without one) with one over the
  stored fog span in its fourth slot. The weather's per-frame near and far
  values are not read by the uniform build.
- The `fog` snippet, fog mode 1 below the voxel-march quality, mixes the
  surface by b = min(distance · FogColr.w, 1) towards
  FogColr.rgb · (amb · 0.5 + 0.5), after the emissive term; an unlit surface
  with only an opacity map has its alpha scaled by 1 − b.
- The record fog colour (×0.375, lightning blue added during a strike) is
  read once, by the voxel GI manager: times the manager's constant 3 it is
  the colour of a directional light that shines straight down, with its own
  1024² depth map, and that light is the first light of every drawn GI
  sector. The scene's sun light does not reach the GI volume. Exarust's "GI
  sun" (the scene sun, times the same 3) was this light misread.

Phase: Phase 5 (world rendering): the scene environment's fog and weather,
and the voxel GI.

## What Changes

- `exa_render` world material (`exa.wesl`): the original's fog on every lit
  program (hair included) and on unlit surfaces with a bare opacity map,
  reading the view's linear distance fog (start 0, end the span).
- `exa_world::scene_env`: the frame's fog is the scratch colour over the
  stored span from the eye; the record fog colour (lightning included) is
  published as the frame's `sky_light`, tinted whether or not the level
  fogs.
- `exa_render::gi`: the GI's directional light is the sky light
  (`GiSkyLight` × 3 × 0.3 × 0.01), straight down, lit where the voxel stands
  above its column's highest occluder; the sun's shadow-map path, its
  resource and the world's publisher of it are removed.
- `exa_world::environment`: publishes the sky light at the load and every
  changed frame.
- Tests with the original's numbers; docs/RENDERING.md.

## Out of scope

- The 16-step voxel fog march (shading quality above 5) and fog mode 2's
  use outside the character screen.
- The outdoor ambient curve of the GI shading (0.08/0.035 and the
  environment cube × `Ambient.w`·5) that the original applies to the GI's
  light under a sky; Exarust keeps the indoor curve.
- Where the weather's per-frame fog near and far go (no reader in the
  renderer's uniform build was found).
- The writer of the environment's fog mode outside the outdoor preset
  (Exarust fogs only under a sky, as before).
- Fog on the terrain's own draw, smoke and other effect shaders.

## Impact

`exa_render` (exa.wesl, gi.rs, gi.wesl), `exa_world` (scene_env,
environment, sun_shadow); docs/RENDERING.md.
