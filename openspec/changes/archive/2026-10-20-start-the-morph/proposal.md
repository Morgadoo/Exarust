# Proposal

## Why

The morph start (`0x1000dd880`) builds the controller's per-vertex records: it creates the owner's helper when missing, ensures the control points, binds every owner vertex to its nearest control candidate and stores the candidate's barycentric weights as the vertex's offset. Exarust has the basis solver and the vertex apply but not the start.

## What Changes

- Create the owner's helper when it has none; ensure the owner's controls only when both are missing.
- For each vertex: transform by the frame, bind to the nearest candidate by squared distance from the maximum float, store that candidate's barycentric weights as the offset and the vertex as the base.
- A vertex with no candidate gets joint 0, a zero offset and an unmatched entry.

## Capabilities

### New Capabilities

### Modified Capabilities

- `morph-controller`: the start.

## Impact

- `exa_assets::morph_controller`, from `start_morph`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`: `RF_Morphing` becomes **23%** (4,006 of 17,099 bytes). The published headline stays 14.4% and 5.7%; the fresh sum stays about 15.5%.
- Evidence: `0x1000dd880`: the helper at owner `+0x2e0` created from class `0x349ee0` with the virtuals `0xc8` and `0xd0`, the controls ensured by `0x10005f6b0`, the frame `0x100018870` over the owner output at `+0x280`, the nearest callback `0x1000de0b0` (squared distance summed `(dx² + dy²) + dz²`, strictly below the maximum float at `0x1002ab6d0`), the solver call at `0x1000dd580` with the nearest point, the unmatched count at `+0x28`, the unmatched list at `+0x30`, and the 28-byte record (joint, offset, base). No new corpus files.
- Out of scope: the 1,128-byte apply/frame routine `0x1000ddc40`.

## Phase

Phase 3, bodies. The morph start.
