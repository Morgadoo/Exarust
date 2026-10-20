# Proposal

## Why

A torch held close to the body smeared the character's skinned shadow across
the floor as a large, soft blob instead of a body-shaped shadow. Exarust's
point lights kept Bevy's 10 cm shadow-map near plane, while the original uses
3 cm; at close range the 10 cm depth range loses the precision the shadow
comparison needs. The light id was also stored in Bevy's depth-bias field as
a whole number up to a few hundred, which would mis-bias any surface that
Bevy's own shadow sampling shades.

## What Changes

- Point lights render their Bevy shadow maps with the original's 3 cm near
  plane.
- A light's id rides above Bevy's default 0.08 depth bias in 1e-5 steps; the
  surface shader recovers it as `round((bias - 0.08) / 1e-5)`.
- The torch's character shadow follows the body: head, torso and legs read
  as a silhouette instead of a floor-wide blob.

## Capabilities

### New Capabilities

### Modified Capabilities

- `rendering`: the point-shadow depth range and the light-id encoding.

## Impact

- `exa_world::light_bundle` sets `shadow_map_near_z` to 0.03 on every level
  light and fire.
- `exa_render::lights` encodes ids above the default bias;
  `shading.wesl` decodes them.
- `docs/RENDERING.md` records both rules.
- Evidence: the original's 3 cm point-shadow near plane already used by the
  cached cube maps (`cube_shadows::NEAR`), and the close-torch comparison in
  `target/verify/shadow-near.png` against `shadow-fixed.png`.
- Out of scope: the 512² face size, the eight-tap filter and the cached cube
  layout.

## Phase

Phase 2, rendering. Correct point-shadow depth precision.
