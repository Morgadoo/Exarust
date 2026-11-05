# Proposal

## Why

A retail comparison of exanima01's start room found Exarust's frame about
40 % redder than the original's, and the round 56 night lane traced the
warm o3/o4 arena nights to fire light through direct light and GI. Most
of the start room's excess was the torch in the player's hand (the
original's story start leaves it on the floor, round 42). What was left
was in the voxel GI: with the frame split into its direct and GI shares
and fitted against retail captures (the menu scene and the start room, at
the retail camera), the GI needed × (1.04, 1.13, 1.35) per channel in the
menu and its blue × 1.5 in the start room, and the start room's dim
periphery had 1.7 × Exarust's neutral light in retail.

The colour path from a fire into the volume matches the original step by
step (the template's sRGB (1, 0.68, 0.43) linearised, intensity ×
multiplier × (1 − dim)² × 0.15 × 0.01, no exposure or settings term, a
constant colour under the flicker; the injection's terms; the ambient's
level × exposure × 15 and the indoor lift). The difference is the 8-bit
store. The original renders each step into a `GL_SRGB8_ALPHA8` texture
with `GL_FRAMEBUFFER_SRGB` on, so the hardware encodes it, and the volume
stalls where a step's gain no longer reaches the next code. Exarust
rounded to the nearest code. A sweep of every value through that format
on the reference GPU (the RTX 3090 the retail captures come from, driver
615.78) shows the driver stepping up at 0.40 to 0.45 of a code along the
curve's foot and between 0.40 and 0.60 higher up, deterministically
(`GL_DITHER` changes nothing). Under nearest rounding a fire's faint blue
(0.156 of its red) and the base ambient stalled early, so the bounce
light came out redder and the far corners darker than the original's.

## What Changes

- `exa_render::gi::SRGB8_STEPS` (measured), `encode_srgb8` and
  `encode_srgb8_from`; `gi.wesl` stores the step and the dark voxels
  through the steps (uploaded in `GiParams`, one table read a channel).
- `EXA_GI_DEBUG=direct|ambient`: the frame split into its direct and GI
  shares (world material and terrain), for fitting retail captures.
- `--floor-torch x,y,z[,rx,ry,rz]`: the comparison torch's frame, so it can
  lie as retail's does, the flame end on the floor.
- `docs/RENDERING.md` (Voxel GI, the 8-bit store; the start room's
  periphery), `docs/DEBUGGING.md`, the `exa_render` and `exa_game` READMEs.

## Capabilities

### New Capabilities

### Modified Capabilities

- `rendering`: Voxel light volume (the driver's sRGB steps). Added: GI
  debug split.
- `world-viewer`: Light comparison options (the floor torch's frame).

## Impact

- `exa_render::gi`, `gi.wesl`, `exa.wesl`, `material`;
  `exa_world::terrain` (`terrain.wesl`); `exa_game::torch`; `exarust`.
- Indoor GI: menu scene GI factors (0.96, 1.00, 1.06); exanima01's
  periphery (8.4, 6.5, 5.3) sRGB against retail's (8.6, 6.8, 5.7), from
  (5.7, 3.7, 2.4). Noon arenao3 (same player spot): the frame (173.1,
  165.0, 151.1) to (174.4, 166.3, 151.2), the volume 5–10 % up, the
  shade under the stands 6 levels warmer (no retail outdoor capture to
  judge it). The GI pass costs about 0.4 ms more (one table read a
  channel).
- Evidence: `context/artifacts/2026-10-30-giwarmth/`.

## Out of scope

Other GPUs' encoders (the steps are the reference machine's; another
vendor's may differ). The additive voxel-light and shape passes are
summed before one encode, where the original's blend encodes twice. The
alpha channel's encode (1 − 5·occ, rounded by the unorm store). Exarust's
direct light in the menu reads × (1.25, 1.19, 1.13) dimmer than retail
(not traced here). No retail outdoor arena capture.

## Phase

Phase 2, static world: the voxel GI (lighting milestone).
