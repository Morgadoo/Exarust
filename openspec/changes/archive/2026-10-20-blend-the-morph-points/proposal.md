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
- Evidence: the 355-byte points blend: the point and entry counts, the point arrays of the owner and of each entry, the weights, and the scale and accumulate helpers. No new corpus files.
- Out of scope: the two vector calls.

## Phase

Phase 5, characters. The weighted point blend.
