# Proposal

## Why

The morph controller blends its point arrays by adding a source point times a weight to a destination point, once per count and once per target entry. Exarust has the setup and nearest searches but not the blend.

## What Changes

- Blending adds each source point times the weight to the matching destination point.
- The result has the shorter of the two counts.

## Capabilities

### New Capabilities

### Modified Capabilities

- `morph-controller`: the weighted point blend.

## Impact

- `exa_assets::morph_controller`, from `blend_morph`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`: `RF_Morphing` moves from 3% to 5%.
- Evidence: the 355-byte `0x1000db850`: the counts `0x48` and `0x4c`, the arrays at `0x60` and each entry's `0x50 + 0x10`, the weights at `0x50` and each entry's `0x0`, and the calls `0x100019240` and `0x1000193d0`. No new corpus files.
- Out of scope: the two vector calls.

## Phase

Phase 5, characters. The weighted point blend.
