# Proposal

## Why

The voxel GI still rebuilt its block around a point 20 m ahead of the
camera, injected neither the sun nor the point lights' cube shadows, and
had none of the manager's block layout, shape box, margin copy or dynamic
update rules. A DeepSeek pass (lane `vxgi2`) ported them; the review
against the assembly found that its light and voxel-light uniforms put
every light four times too far along the depth axis (they divided world Z
by the volume's height), that its sun position mixed texture and world
axes, that its no-sky sun sample was nearer instead of deeper, that the
sun lacked the manager's factor of 3, that the block lookup rounded where
the engine floors, that the built-cell scan missed two of the original's
cases, that the centred splat raised world Z instead of the height, and
that the shape box and dirty region rounded wrongly.

## What Changes

- Blocks are laid out once per level from the bounding box of the grid's
  built cells (eight-cell blocks slid inside the grid; one block at cells
  (3, 3) of the synthetic grid); the camera picks its own block by the
  floored offset, or the block nearest the focus 20 m ahead; the last
  three built occupancies are kept.
- The injection reads the light texture at the slice boundary with the
  height over the block's width, shadows point lights by the cached cube
  pool, and puts the sun first in every drawn sector's group at 3 × 0.3 of
  its colour, shadowed by the fitted sun map (first comparison under a
  sky, otherwise the difference with the comparison 0.01 deeper).
- Tested manager rules: presets, corner offsets, quad indices, shape list
  and draw box, margin copy, thin and centred splats, dynamic update and
  its dirty region, sub-upload bytes.

## Capabilities

### New Capabilities

### Modified Capabilities

- `rendering`: GI around the camera; GI sun injection; GI cube shadows; GI built-cell bounds; GI shape draw box; GI dynamic occupancy.

## Impact

- `exa_render::gi` (`gi.rs`, `gi.wesl`), `exa_world::gi_volume`,
  `exa_world::sun_shadow`, `exa_world::environment`.
- `docs/RENDERING.md` (Voxel GI), crate READMEs.
- Evidence: REA/Ghidra pseudo-code and assembly of the manager's block
  layout and built-cell scan, per-object block lookup (the engine's floor
  helper, MXCSR 0x2000), per-frame residency sort, dynamic update
  (CVTSS2SI with ±1.5), mesh splat, light uniform builder (0.1 share, ×1.5
  point, ×3 sun), per-frame sun setup (factor 3, one COMISS on the third
  component), shape loop (ceiling helper, MXCSR 0x4000; bounds 8 and
  248), margin copy table; the shipped `gi_inject_v/g/f` and
  `gi_emission` shader sources.
- Picture: outdoor levels gain sun-lit GI; point-light GI is shadowed by
  the cube pool; voxel and point lights sit on their own voxels. No retail
  capture was taken.

## Out of scope

Nine resident blocks and their recycling, the GPU margin blit, the
emissive shapes' own injection programs and any caller registering a
shape, wiring the dynamic update and sub-upload (no GI mesh moves), the
original's separate 1024² GI sun map and its second direct light, the
cube compare's shadow-bias factor, the third block-layout branch driven
by a terrain object's own block grid, and the 5 cm and 40 cm presets in
the running game.

## Phase

Phase 2, static world: the voxel GI (lighting milestone).
