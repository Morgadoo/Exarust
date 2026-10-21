# Proposal

## Why

The cooked morph data is a byte blob the owner stores. Exarust models its fields but not its little-endian byte layout, so the data cannot round-trip through storage.

## What Changes

- Serialization writes the magic, the size, the base, the `V` rest points, the part count, then each part's pivot and `V` deltas, all little-endian.
- Parsing accepts only the magic `0xEDC0C589` and a size that matches both the bytes and the vertex count; the part count comes from `12·(P+1)·(V+1)+12`.

## Capabilities

### New Capabilities

### Modified Capabilities

- `morph-controller`: the blob's bytes.

## Impact

- `exa_assets::morph_controller`, from `MorphData::to_bytes` and `MorphData::from_bytes`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`: `RF_Morphing` becomes **46%** (7,866 of 17,099 bytes). The published headline stays 14.4% and 5.7%; the fresh sum stays about 15.5%.
- Evidence: the morph core's allocation and header phases: the magic `0xEDC0C589`, the size `12(P+1)(V+1)+12`, the owner's base point, the rest copy and the part count. No new corpus files.
- Out of scope: the three remaining evaluators (rest restore, weighted base, full evaluation) and the frame/apply.

## Phase

Phase 3, bodies. The morph blob's bytes.
