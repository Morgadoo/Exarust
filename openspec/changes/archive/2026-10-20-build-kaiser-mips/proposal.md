# Proposal

## Why

The original builds Kaiser-windowed mipmaps with a bank of 13 normalised kernels and a separable resample that reads the base level directly. Exarust only loads the base texture; `RFP_Mipmaps_Kaiser` is rated 5%.

## What Changes

- The Kaiser table uses `n = 8·2^(level−1)` taps, `h = n/2`, `c = 2^−level` and `w = I0(4√(1−t²))/I0(4)·sinc(πcu)`, zero outside `|t|<1`, normalised.
- The axis pass uses the integer ratio, starts at `−((n−ratio)/2)`, and wraps or clamps each tap.

## Capabilities

### New Capabilities

### Modified Capabilities

- `rendering`: the Kaiser mip chain.

## Impact

- `exa_render::mipmap`, from `bessel_i0`, `sinc`, `kaiser_table`, `mip_axis_start` and `resample_axis`.
- `docs/PLAN.md` and the generated `docs/COMPONENT-MAP.md`: `RFP_Mipmaps_Kaiser` becomes **20%** in `data.json`. The headline rises slightly.
- Evidence: the Kaiser window, its I0 and sinc helpers, the 13-table bank, and the wrap and clamp axis passes. No new corpus files.
- Out of scope: the format conversion, uploads, the generic NPOT path and the box-filter variant.

## Phase

Phase 2, rendering. The Kaiser mip chain.
