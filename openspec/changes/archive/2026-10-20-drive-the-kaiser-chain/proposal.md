# Proposal

## Why

The Kaiser kernel and axis pass are modelled, but not the driver that produces the chain: levels from the base, horizontally then vertically, until the original's size limit.

## What Changes

- Level L is generated while both destination dimensions stay above zero and their sum is at least 3.
- Both dimensions must divide the base exactly (the power-of-two path).
- Each level is resampled from the base with its own Kaiser table, horizontally then vertically.

## Capabilities

### New Capabilities

### Modified Capabilities

- `rendering`: the Kaiser mip chain driver.

## Impact

- `exa_render::mipmap`, from `MipLevel` and `build_mip_chain`.
- `docs/PLAN.md` and the generated `docs/COMPONENT-MAP.md`: `RFP_Mipmaps_Kaiser` becomes **30%** in the source rating. The headline rises slightly.
- Evidence: `0x1000474f0` (the exact power-of-two probe), `0x100047470` (one level per call), `0x100048eb0` (the two passes) and `0x100049458` (the `dstW + dstH < 3` stop). No new corpus files.
- Out of scope: the NPOT fallback, the format converters and the upload.

## Phase

Phase 2, rendering. The Kaiser chain driver.
