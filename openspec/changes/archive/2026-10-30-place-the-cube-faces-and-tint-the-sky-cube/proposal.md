# Proposal

## Why

The outdoor GI shading reads the environment cube around the normal, and
the outdoor result looked wrong: a halo on the character and a hazy frame.
Exarust uploaded the cube's six stored faces in stored order, but the
original sends them to other face targets, so the sky stood on +Z instead
of up. The open sky's cube was also bound untinted, and the cube path's
reflections kept `Ambient.w` 8 under a sky.

Evidence (REA/Ghidra and the objdump listing of the 0.9.5 build; the cube
images):

- The texture uploader sends stored face k of a cube to the GL target in a
  six-entry table: −X, −Y, −Z, +X, +Y, +Z. Stored face 4 is the sky of
  `genref02r` and the vault of `genref01r`; face 1 is the ground.
- The environment's constructor keeps CPU copies of `genref02r` and
  `genref02rm` and a copy for the upload. At each sun placement it tints
  the copy through the mask by the environment record's sky tint (white
  mixed towards the tint by mask/255, times the texel, back to a byte)
  over the image's whole texel count (every face's chain) and uploads it
  over the sky cube.
- Without voxel GI the reflections are (e + e²)·`Ambient.xyz`·`Ambient.w`;
  `Ambient.w` is 3 under a sky. The ambient there has no `Ambient.w` term.

Phase: Phase 5 (world rendering): the environment cube.

## What Changes

- `exa_assets::texture`: cube layers take the stored faces on the
  original's targets (`CUBE_LAYER_FACES`).
- `exa_assets::model`: `SkyCubeSource` (the cube and mask bytes and the
  upload settings).
- `exa_world::scene_env`: the frame publishes the record's sky tint once a
  sun placement has run; `exa_world::environment` tints the cube again when
  it changes (`SkyCube`, `tinted_sky_cube`).
- The open-sky flag is set by the cube binder with or without GI; the cube
  path's reflections take `Ambient.w` 3 under a sky (world material and
  terrain).

## Out of scope

- The overbright open arena that the corrected cube exposes: Exarust's
  open-air GI volume is far brighter than the original's must be. Not
  traced here.

## Impact

`exa_assets` (texture, model), `exa_world` (scene_env, environment, terrain,
level load), `exa_render` (exa.wesl, gi.rs); docs/RENDERING.md.
