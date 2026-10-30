# Proposal

## Why

The voxel GI manager's shape bodies, its nine resident light textures, its
neighbour margin copies, its light-slot move and its light uniform order
were tested functions or missing; the running GI drew none of them.

Evidence (REA on the 0.9.5 build and the objdump listing, checked against
the assembly):

- The per-block injection walks the registered shapes after the point-light
  groups and draws each shape's own program over the manager's clipped box
  and slice range.
- Before drawing a group the injection moves the first light whose shadow
  is a 2D matrix map: from index 1 to slot 0 when the record's flag is
  clear, or from index 2 to slot 1 when it is set, each scan a do-while
  that reaches the last light.
- A point light's uniform is (position − block centre) · 1/(W·fc) + 0.5 on
  the two ground axes, below/W + (y − centre y) · 1/(W·fc) for the height,
  and 1/(R · 1/(W·fc)) for the reach, with 1/(W·fc) formed from the integer
  product.
- The margin copy reads a fixed table of texture fractions (0, 1/32, 31/32,
  1) and UV offsets (+15/16, 0, −15/16) for the neighbour at −1, 0 and +1 on
  each axis, for each neighbour whose state has both texture bits, over
  every slice but the top one.
- The sky light's map set-up: 1024², two depth fields both 1600, softness
  (width·10)·0.5 from the integer product, and a symmetric view matrix that
  keeps X and swaps Y and Z with both negated.

Phase: Phase 5 (world rendering): the voxel GI manager (`TVXGIManager`).

## What Changes

- `exa_render::gi`: the six shape bodies drawn into the light pass per
  voxel over each shape's clipped box; up to nine blocks' light textures
  kept (`GiTextureCache`) and reused when the camera returns; each resident
  neighbour's margin copied into the active block (`margin_blit`, the
  `margin_blit` compute entry); the slot move (`plan_light_slots`) keyed to
  the sky light; the light uniforms in the original's centimetre order;
  `sky_shadow_setup` keeps the sky light's map numbers for reference.
- `gi.wesl`: the shape bodies with GL comparison semantics and the
  drivers' half-away rounding; the margin copy.
- docs/RENDERING.md and the crate README.

## Out of scope

- Stepping all nine resident blocks every frame (only the camera's block
  steps; residents keep their last light).
- Per-object `VXGIPos` block selection and texture 0 outside a block.
- The sky light's own 1024² map (the volume's columns stand in for it).
- The spot-light branch of the injection, and each shape class's own
  `Tex0` (one white texel is bound); no effect registers a shape yet.
- The level record's stored block grid.
