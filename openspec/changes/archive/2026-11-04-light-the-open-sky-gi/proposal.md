# Proposal

## Why

`docs/RENDERING.md` recorded an open arena at noon as overbright inside the
GI block, with an estimate that the volume held 0.3 to 0.95 per channel in
open air and an upward surface took about 10. A read-back of the light
volume (`EXA_GI_PROBE`) showed neither: the volume's 8-bit sRGB storage
stops each channel's climb once a step's gain falls under half a code, so
open air settles at a few hundredths, and the open sky cube's up face is
about 0.2 at mip 7, not 0.65. The probe also showed that the sky light
reached no voxel at all: the sky floors counted the block's soft lid (26
in the top slice) as an occluder, so every column's floor was above the
block. And under a sky the base ambient still came from the level
record's `0x30` (0.0395 in arenao3), while the original's injection reads
the environment's fill field, which the sun placement keeps at the day
record's fill (0.156, 0.188, 0.223 at noon). What was left of the GI was
the braziers' warm light, shaded through the outdoor curve.

## What Changes

- `exa_render::gi::sky_floors` leaves the top slice out of its scan, so
  the lid no longer shuts the sky out; the injection also leaves voxels at
  or below −16 m dark, the depth window of the original's map.
- `exa_render::gi::GiBaseAmbient` carries the base ambient under an open
  sky; `exa_world::gi_volume` publishes the environment's current fill
  times the share every frame while the sky is on, and the step uses it
  for every block.
- `exa_render::gi_probe` (`EXA_GI_PROBE`, `EXA_GI_PROBE_AT`): a flag-gated
  read-back of the light volume, with the groups' lights and the sky
  cube's per-face means at mips 5 and 7 (logged by `exa_world`).
- `docs/RENDERING.md`: the status paragraph replaced with the measured
  levels.

## Capabilities

### New Capabilities

### Modified Capabilities

- `rendering`: GI sky visibility; GI base ambient quality. Added: GI
  volume probe.

## Impact

- `exa_render::{gi, gi_probe}`, `gi.wesl`, `exa_world::{gi_volume,
  environment}`; the `exa_render` and `exa_world` READMEs.
- Indoor levels are unchanged (no sky light, no published base); exanima01
  differs by 0.05 sRGB levels on average (run noise).
- Evidence: `context/artifacts/2026-10-30-gibright/`.

## Out of scope

The original's map is drawn once, around the world's origin, over a
±5120 cm square with clamped edges; Exarust keeps its column floors, so a
level wider than that square, or the map's own mesh culling (its frustum
is never refreshed for this map), is not modelled. No retail capture of an
outdoor arena was made: the career's match board could not be reached by
synthetic input in the time given.

## Phase

Phase 2, static world: the voxel GI (lighting milestone).
