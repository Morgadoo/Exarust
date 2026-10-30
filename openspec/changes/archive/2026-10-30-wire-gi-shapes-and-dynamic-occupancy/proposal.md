# Proposal

## Why

The GI manager's emissive shapes had a registry and a draw box but no
fragment bodies; a moving GI mesh's occupancy update existed as a tested
function with no caller and a full block upload in place of the
original's sub-upload; and the settings screen's shading quality never
reached the GI. A DeepSeek pass (lane `vxgi6`) ported the six shape
bodies from the original's `shading.flb`, wired the dynamic update with a
sub-upload and fed the shading quality. The review checked the work
against the assembly and the shader library: it fixed the update's patch
list, which was never drained (it grew without bound and every patch was
re-written each frame), added the original's build-in-flight gate and a
catch-up for blocks built after a mesh moved, ended the upload one voxel
short of the size, put the shape box's half-extent product and the
textured bodies' window products in the original's order, fixed the
cuboid constructor's frame, and took the high-quality light share in
double precision.

## What Changes

- `exa_render::gi_shape`: the six `shading.flb` shape bodies (`gi_dome`,
  `gi_cuboid`, `gi_bubble`, `gi_telebubble`, `gi_gradline`,
  `gi_multipoint`) evaluated per voxel, and a `GiShape` component that
  lists and unlists its entity in `GiShapes`.
- `shape_draw` uses the manager's single-precision chain: the block
  centre, `(p − centre)·s + 0.5` times the width rounded half to even, and
  half extents `ceil(width·(h·s))`.
- `exa_world::gi_volume::update_dynamic` runs every frame after the block
  follow: nothing while a build is in flight; old points out (−20), new in
  (+20) in each built block the padded boxes meet; a block built after a
  move is brought from the source frame to the last one; the changed box
  goes out as a `GiOccupancyPatch`, drained every frame, and the renderer
  writes it into the bound block and one warming behind it.
- `ShadingQuality` (default 4) is a resource the options screen writes;
  the world copies `> 3` into `GiSettings::high_quality`;
  `gi::injection_share` is 0.1, or 0.1 × 0.9 in double precision.

## Capabilities

### New Capabilities

### Modified Capabilities

- `rendering`: GI shape draw box; GI dynamic occupancy; GI base ambient
  quality. Added: GI dynamic occupancy timing; GI shell shape fragments;
  GI line and point shape fragments.

## Impact

- `exa_render::{gi, gi_shape, quality}`, `exa_world::gi_volume`,
  `exa_game::hw_config` (the options write the shading quality).
- `docs/RENDERING.md` (Voxel GI), the `exa_render` and `exa_world` READMEs.
- Evidence: the shape bodies from `shading.flb` (decoded with
  `context/re/flb_decode.py`); the per-block injection's shape loop
  (SUBSS/MULSS/ADDSS 0.5 per axis, the vertical term below/width added to
  (p.y − 0)·s, CVTSS2SI ties to even for the centre, h·s then times the
  width into the ceiling helper whose MXCSR is 0x4000, the 8/248 clips,
  the `≥ 2` slice test); the dynamic update's build-count gate before the
  node-kind and point-count tests, its −20 then +20 splats, its old-frame
  copy at the end; the sub-upload's clamp of the end to size − 1; the
  per-frame entry's CVTSS2SD/MULSD 0.9/CVTSD2SS share and the 0.5 base
  ambient scale under the quality flag.
- Per-frame cost: one affine compare per GI mesh; a moved mesh edits only
  the blocks its box meets, in place.

## Out of scope

The nine resident blocks with their margin copies and per-object block
choice, emissive shapes on the GPU (no effect registers one yet), the
tile-map branch of the block layout, the per-block mesh gather, and the
original's own sun map.

## Phase

Phase 2, static world: the voxel GI (lighting milestone).
