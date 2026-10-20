# Proposal

## Why

Three evaluators read the cooked blob: `0x1000dd010` restores the rest pose, `0x1000dd090` returns the weighted base, and `0x1000dd1a0` evaluates every point. Exarust stores and parses the blob but cannot yet read it back into the owner's state.

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
- Evidence: `0x1000dd010` (the magic check, the base and the rest copy), `0x1000dd090` (the base at blob `+8`, the parts at `ctl+0x50`, the `1.19209e-7` epsilon at `0x1002ab6c0` and the pivot accumulation), `0x1000dd1a0` (the magic gate, the rest copy, the per-part pivot and per-vertex delta accumulation `0x1000193d0` and `0x100019240`, and the owner base write). No new corpus files.
- Out of scope: the 1,128-byte frame/apply.

## Phase

Phase 3, bodies. The morph point evaluators.
