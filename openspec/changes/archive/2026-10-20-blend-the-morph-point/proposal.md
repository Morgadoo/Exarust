# Proposal

## Why

The morph controller answers a query point by starting from the block's base point and adding each block point times its weight, skipping weights under the epsilon. Exarust has the point-array blend but not the single-point query.

## What Changes

- The result starts at the base point.
- Each block point times its weight is added when the weight's magnitude is at least the epsilon.

## Capabilities

### New Capabilities

### Modified Capabilities

- `morph-controller`: the weighted point query.

## Impact

- `exa_assets::morph_controller`, from `blend_morph_point`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`: `RF_Morphing` moves from 7% to 8%.
- Evidence: the 268-byte `0x1000dd090`: the block at `0x138 + 0x1c0`, the copied base at `+8`, the `0x4c` entries at `0x50` with stride 32, the weight at `+0`, the epsilon `0x2ab6c0`, the accumulate `0x1000193d0` and the output copy. No new corpus files.
- Out of scope: the accumulate call.

## Phase

Phase 5, characters. The weighted point query.
