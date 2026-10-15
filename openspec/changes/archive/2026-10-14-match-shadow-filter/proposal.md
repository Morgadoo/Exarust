## Why

Torch shadows looked wrong next to the original's: the torch's own handle
threw a wide cone down the wall, and Bevy's filter softened shadows by its
own rule. The original leaves a light's own object out of that light's
shadow map, and filters with a fixed eight-tap disc whose radius follows
the light's softness.

## What Changes

- Self-exclusion: a light's siblings, parent and grandparent cast no
  shadows. For a torch that is its visible body; its hidden `TorchSdw` disc
  still casts.
- The original's point-shadow filter in the material shader:
  - normal offset 0.008·(1 − |N·L|)·d;
  - eight fixed disc taps turned per pixel, each a 2 × 2 comparison;
  - disc radius (softness + 1)·0.005;
  - distances compared 0.5 % short.
- Fires' shadow softness 2·j, with the original's jitter walk.

## Capabilities

### New Capabilities
(none)

### Modified Capabilities
- `rendering`: shadow filtering and self-exclusion.

## Impact

- `exa_render` (`exa.wesl`), `exa_world` (`spawn_model`, `point_light`,
  `fire`) and `docs/RENDERING.md`.
