# Proposal

## Why

Three evaluators read the cooked blob: one restores the rest pose, one returns the weighted base, and one evaluates every point. Exarust stores and parses the blob but cannot yet read it back into the owner's state.

## What Changes

- Restoring returns the blob's base and its rest points.
- The weighted base is the blob's base plus each part's pivot times its weight, skipping a weight whose absolute value is under 1.19209e-7.
- The full evaluation starts from the rest points and adds each part's pivot and per-vertex delta times its weight, returning the base and the points.

## Capabilities

### New Capabilities

### Modified Capabilities

- `morph-controller`: the blob evaluators.

## Impact

- `exa_assets::morph_controller`, from `restore_morph_positions`, `morph_weighted_base` and `evaluate_morph_positions`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`: `RF_Morphing` becomes **51%** (8,755 of 17,099 bytes). The published headline stays 14.4% and 5.7%; the fresh sum stays about 15.5%.
- Evidence: the rest restore (the magic check, the base and the rest copy), the weighted base (the blob's base, the controller's parts, the `1.19209e-7` epsilon and the pivot accumulation) and the full evaluation (the magic gate, the rest copy, the per-part pivot and per-vertex delta accumulation through the scale and accumulate helpers, and the owner base write). No new corpus files.
- Out of scope: the 1,128-byte frame/apply.

## Phase

Phase 3, bodies. The morph point evaluators.
