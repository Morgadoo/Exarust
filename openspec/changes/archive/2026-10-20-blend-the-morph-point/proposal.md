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
- Evidence: the 268-byte point blend: the owner's morph block, the copied base, the morph entries with stride 32 and their weights, the epsilon, the accumulate helper and the output copy. No new corpus files.
- Out of scope: the accumulate call.

## Phase

Phase 5, characters. The weighted point query.
