# Design

## Context

The previous change kept lightning off the flash clear and left open where
the lightning-tinted record fog colour is drawn. Exarust drew it as Bevy's
distance fog colour, which the world material never reads.

## Decisions

- **Fog uniform.** The global uniform build copies the environment's scratch
  colour into `FogColr` and overwrites its fourth slot with one over the
  stored span (distance² · far plane + 100 cm). Exarust's frame now hands the
  scratch colour and the span (in metres, start 0) to the cameras' linear
  `DistanceFog`, and the world shader reads Bevy's fog uniform rather than a
  binding of its own: no extra cost beyond a distance and a mix per pixel.
- **Formula.** b = min(d / span, 1) with d the eye distance;
  colour = mix(colour, fog · (amb · 0.5 + 0.5), b). Outdoors the original's
  `amb` is the voxel GI's light before its environment-cube factor, indoors
  the ambient; Exarust's GI ambient has no cube factor, so its ambient
  stands for both.
- **Record fog colour → GI sky light.** The environment registers itself on
  the scene; the GI manager reads its record-fog field each time it lights
  a block, scales it by its own constant 3 (constructor), skips it while the
  blue is at most 1.19e−7, and otherwise gives it to a type-2 light created
  with direction (0, 1, 0) and a 1024² depth map of reach 1600. That light
  joins each drawn sector's list first. The colour is used as the frame
  keeps it (no second linearisation) [guess: the light node's own
  activation path was not followed].
- **Shadow from above.** The original takes its map's first comparison
  alone under a sky. Exarust uses the volume's own columns: the slice above
  the column's highest voxel with an occupancy byte of 10 or more
  (openness under one half) [adaptation], uploaded once per block as a
  16-bit texture. Moving meshes' occupancy patches do not move it.
- **Indoors.** Turning the sky off zeroes the record fog colours, so the
  sky light is absent and indoor GI is unchanged except where a level with
  a record sun and no sky had that sun in its GI (the original never did).

## Open questions

- The light node's intensity and whether its activation linearises the
  colour: follow the node class's colour update from the manager's write
  (REA, the light node's colour setter).
- The outdoor ambient curve for the GI light (docs/RENDERING.md, Voxel GI):
  a separate change.
- Readers of the weather's fog near and far: search the renderer for the
  environment's current-fog fields.

Docs: docs/RENDERING.md (Environment and weather; Voxel GI).
