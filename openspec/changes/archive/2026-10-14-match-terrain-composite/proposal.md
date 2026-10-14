## Why

At the game's camera, exanima01's terrain floor still looked wet, too
bright and flat, and its rooms lacked the original's bounce light. The
original composites terrain with rules Exarust had guessed:
- gloss from gloss textures, not from the colour;
- a 0.75 diffuse factor;
- parallax from the bump slot.

The GI also had no terrain in it, and on large levels it injected arbitrary
lights rather than those near the camera.

## What Changes

- Terrain shading follows the original's composite:
  - per-layer gloss textures, roughness 1 − gloss;
  - albedo × 0.75;
  - parallax from the bump slot's height (f20 + (B − f21)·f22, stacked like
    the normals), at the original's scale of 0.007 per 6 m block, eased
    back where the height changes sharply.
- Each layer's height, gloss and mask are packed into one texture, to fit
  the per-stage texture limit.
- GI occupancy fills terrain as the original does: solid below the surface,
  with the covered fraction in the surface voxel.
- The GI injects the lights whose reach overlaps the block, nearest first,
  instead of the first 32 in the world.
- Viewer: `--torch x,y,z` places a torch fire, e.g. to stand in for a held
  torch. `EXA_GI_DEBUG` shows the raw GI sample. The `EXA_GI_DUMP` file
  also writes the sector floors.

## Capabilities

### New Capabilities
(none)

### Modified Capabilities
- `terrain`: composite gloss, albedo and parallax.
- `rendering`: terrain in the GI; GI light selection.

## Impact

- `exa_world` (`terrain`, `gi_volume`), `exa_render` (`gi.rs`, `exa.wesl`,
  `material.rs`), `exarust` (viewer) and `docs/RENDERING.md`.
