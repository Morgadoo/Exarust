# Proposal

## Why

The voxel GI block is rebuilt when the camera's focus moves more than two
sectors. The new block was swapped in immediately and then warmed over about
40 frames from black. During that time the floor showed hard black patches in
shadow and a soft, circular dark halo around the part of the GI volume that
had lit so far. The loading screen waits for the first block's warm-up, but a
rebuild during play did not.

## What Changes

- Keep the block already on screen bound while a rebuilt block warms behind
  it.
- Warm the pending block in the render world on the same 16-steps-a-frame
  schedule; swap it in only when the full warm-up has been reported.
- On a level change, drop both blocks so the new level starts with its own
  first block, shown while the loading screen waits.
- Update every surface material when the active block swaps, not only when
  the first block is added.

## Capabilities

### New Capabilities

### Modified Capabilities

- `rendering`: a rebuilt GI block waits for its warm-up before it is shown.

## Impact

- `exa_render::gi`: a `GiPendingTextures` resource and `swap_warm_textures`;
  the bind group and propagation prefer the pending textures while they warm.
- `exa_world::gi_volume`: a level change removes the old active and pending
  textures.
- `docs/RENDERING.md` (Voxel GI) and `docs/PLAN.md`.
- Evidence: the user's manual-session floor captures (a soft circular halo
  around the lit part of a rebuilt block and hard black patches in its
  shadows); the new `a_rebuilt_block_waits_for_its_warm_up` and
  `the_first_block_is_used_while_it_warms` tests.
- Out of scope: the block build itself, the propagation shader and the
  number of resident blocks (the original keeps up to nine).

## Phase

Phase 2, rendering. Hold the old GI block until a rebuilt one is warm.
