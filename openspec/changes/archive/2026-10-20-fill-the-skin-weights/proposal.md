# Proposal

## Why

The morph cook also builds its 32-byte skin-weight stream: a four-slot joint merge, a distance-weighted fill for the target vertices, normalization, and three relaxation passes. Exarust has the weapon-free four-slot merge and the blob, but not the stream.

## What Changes

- A merge adds a scaled source through the four-slot joint merge.
- Normalization stops at the first zero joint, zeroes a negative weight, and divides the weights through the last used slot by their sum.
- A fill merges each neighbour at `1/√(distance squared)` and divides by the distance sum.
- Three relaxation passes make each record half itself plus half of `1/√(distance squared)` times each neighbour, then normalize.

## Capabilities

### New Capabilities

### Modified Capabilities

- `morph-controller`: the skin weight stream.

## Impact

- `exa_assets::morph_controller`, from `SkinRecord`, `merge_skin_record`, `scale_skin_record`, `normalize_skin_record`, `fill_skin_record` and `relax_skin_records`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`: `RF_Morphing` becomes **37%** (6,327 of 17,099 bytes). The published headline stays 14.4% and 5.7%; the fresh sum stays about 15.5%.
- Evidence: the morph core: the 32-byte records (four joints then four weights), the merge with its `1.19209e-7` epsilon, the normalization (the first zero joint, the negative weight zeroed, `1.0` divided by the sum), the fill weights `1/√(distance squared)`, and the three relaxation passes with their `0.5`. No new corpus files.
- Out of scope: the target bitmap diffusion and the stream's registration in the owner.

## Phase

Phase 3, bodies. The morph skin weights.
