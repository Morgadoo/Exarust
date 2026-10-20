# Proposal

## Why

The morph controller's 448-byte apply (`0x1000dd3c0`) walks every vertex it drives, builds a 3 × 3 basis from the three morph points its joint names, deforms the vertex's offset by it, subtracts the controller's base, applies the owner's orientation, and adds the vertex's own base into the owner's output array. Exarust has the controller's setup, nearest target, blends and parser, but not the apply.

## What Changes

- For each vertex: the joint's three indices go through the remap into the controller's points; those three points are the basis rows; the vertex's offset is multiplied by them; the controller's base is subtracted; the owner's rows are applied; the vertex's base is added.
- The result has one point per vertex.

## Capabilities

### New Capabilities

### Modified Capabilities

- `morph-controller`: the vertex apply.

## Impact

- `exa_assets::morph_controller`, from `apply_morph_vertices`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`: `RF_Morphing` becomes **14%** (2,346 of 17,099 bytes). The published headline stays 14.4% and 5.7%; the fresh sum stays about 15.5%.
- Evidence: `0x1000dd3c0`: the record at `this+0x20` (28 bytes: a joint word, a 3-float offset, a 3-float base), the joint table at owner `+0x290` (three words a joint), the remap at owner `+0x270`, the points at `this+0x60` (12 bytes each), `0x100018820` (rows × offset), `0x1000191a0` (minus `this+0x78`), `0x100018b00` (rows at owner `+0x40`), `0x1000190b0` (plus the record's base) and the store into the owner's output at `+0x280`. No new corpus files.
- Out of scope: the owner's tick and the 4,642-byte core.

## Phase

Phase 3, bodies. The morph controller's vertex apply.
