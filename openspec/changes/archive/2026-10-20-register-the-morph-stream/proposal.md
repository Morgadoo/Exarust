# Proposal

## Why

The morph cook ends by registering its 32-byte skin-weight stream in the owner and rebuilding the controller's target list from the mesh's flagged triangles. Exarust has the stream's maths but not the registration or the target list.

## What Changes

- Registration runs only when the controller has weights and the owner carries bit `0x200000`.
- It reuses the stream whose low flag bits are `0x1000`, or grows the owner's list by one with flags `0x04000001` and type `0x02a41000`.
- Either way it zeroes the stream's `V·32` bytes.
- Target collection adds the remapped corners of every triangle whose flags carry `0xC00000`, in the order 1, 2, 0, without duplicates.

## Capabilities

### New Capabilities

### Modified Capabilities

- `morph-controller`: the stream registration and target list.

## Impact

- `exa_assets::morph_controller`, from `register_morph_stream`, `MorphStream` and `collect_morph_targets`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`: `RF_Morphing` becomes **43%** (7,353 of 17,099 bytes). The published headline stays 14.4% and 5.7%; the fresh sum stays about 15.5%.
- Evidence: the morph core's stream phases: the controller guard and the owner's `0x200000` flag, the stream search for the `0x1000` kind, the list growth with 0x18-byte entries and flags `0x04000001`/type `0x02a41000`, the `V·32` memset, and the owner's target scan with mask `0xC00000`, the three remapped corners and the corner lookup `{1, 2, 0}`. No new corpus files.
- Out of scope: the blob's byte serialization and the owner re-bind pair.

## Phase

Phase 3, bodies. The morph stream registration.
