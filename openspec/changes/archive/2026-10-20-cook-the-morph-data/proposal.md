# Proposal

## Why

The morph core (`0x1000dbdc0`) cooks the serialized morph data once: a `0xEDC0C589` blob holding the base point, the rest points, and each part's pivot and per-vertex deltas. A delta is the barycentric triangle point minus the part's pivot, rotated by the owner's rows. Exarust has the controller's start, basis, apply and remap, but not the cook.

## What Changes

- The blob carries the magic, the size `12·(P+1)·(V+1)+12`, the base, the rest points and each part's pivot and deltas.
- A vertex's delta is the barycentric triangle point minus the part's pivot, rotated by the owner's three rows.

## Capabilities

### New Capabilities

### Modified Capabilities

- `morph-controller`: the data cook.

## Impact

- `exa_assets::morph_controller`, from `build_morph_data`, `morph_blob_size`, `morph_delta` and `MORPH_MAGIC`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`: `RF_Morphing` becomes **32%** (5,472 of 17,099 bytes). The published headline stays 14.4% and 5.7%; the fresh sum stays about 15.5%.
- Evidence: `0x1000dbdc0`: the magic `0xEDC0C589`, the size `12(P+1)(V+1)+12`, the base at `obj+0x64`, the rest copy from `obj+0x280`, the part count, the part pivot `part+0x18[k]` and the delta `M·(Σ bary·corners − pivot)` through `0x100018820`, `0x1000191a0` and `0x100018b00`, with the 0.5 and 1.0 constants at `0x10030d920` and `0x10030d918`. No new corpus files.
- Out of scope: the target diffusion, the skin-weight stream and its three relaxation passes.

## Phase

Phase 3, bodies. The morph data cook.
