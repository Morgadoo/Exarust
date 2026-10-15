## Why

The original's frames look crisper than Exarust's. At the player's
`Supersample = 4` the original turns on a sharpen in its final post pass:
an unsharp mask of strength 0.9375 that also eases strong edges. Exarust
had none.

## What Changes

- The sharpen in the final post pass, before the grading curve, as the
  original computes it:
  - four diagonal samples half a pixel out;
  - a weight (min(0.16a, 0.8) − clamp(1 − a, 0, 1)¹⁶)·strength from their
    contrast a;
  - the strength set to the 4× supersampling value, 1.5 × (0.075·3 + 0.4).

## Capabilities

### New Capabilities
(none)

### Modified Capabilities
- `rendering`: post-processing sharpen.

## Impact

- `exa_render` (`post.rs`, `post.wesl`) and `docs/RENDERING.md`.
