## Why

The original's torches burn with animated flames. Exarust lit the scene
with each fire's light but drew no flame, and this was the most visible
difference left in the menu comparison.

## What Changes

- Simulate the small fire's particles as the original does, at its 83 Hz
  tick:
  - emission with the original's accumulator rule, from a ring around the
    fire node;
  - Verlet integration with buoyancy and a pull toward the axis;
  - random ageing, and a reach cap on each particle's life.
- Draw each particle as a diamond stretched along its motion, with an atlas
  cell and a square symmetry picked per particle. Colour each from the
  fire's eleven keys over its life. The new `FlameMaterial` blends them
  premultiplied (texture × vertex colour).
- The fire light's density follows the live particle count.
- The GI warm-up waits two seconds for fires to build up.

## Capabilities

### New Capabilities
(none)

### Modified Capabilities
- `rendering`: flame particles.

## Impact

- `exa_render::flame` with `flame.wesl`, the new `exa_world::flame`,
  `exa_world::fire` (density, settling) and `docs/RENDERING.md`.
