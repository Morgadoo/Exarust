# Proposal

## Why

The mind-sight draw anchors each node's quad from the node's three frame rows, its two body anchors and the camera, blending two cross-product pairs by `1 − |view·dir.x|` and scaling the axes by 0.025. Exarust has the overlay fade, pick and weights but not the quad basis.

## What Changes

- The eye anchors at the origin plus `(anchor_a + 15)·m1 + anchor_b·m2`.
- The direction is the normalized eye-to-camera vector.
- The axes blend the cross products by `1 − |view·dir.x|` and scale U and V by 0.025.

## Capabilities

### New Capabilities

### Modified Capabilities

- `thaumaturgy`: the sight quad's basis.

## Impact

- `exa_game::mind`, from `SightBasis` and `sight_billboard`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`: `SG_Thaum_Mind` becomes **9%**. The published headline stays 14.4% and 5.7%; the fresh sum stays about 15.5%.
- Evidence: `0x1001e01d0`: the eye anchor at `0x1001e0687` (the `+15.0` at `0x10032f208`), the normalize and view multiply, `1 − |d.x|` at `0x1001e077f`, the `m2×dir`, `a×dir`, `m0×dir`, `c×dir` chain, the blended axes and the 0.025 half-size at `0x1002e89b0`. No new corpus files.
- Out of scope: the shader upload and per-view matrices.

## Phase

Phase 7, thaumaturgy. The sight quad.
