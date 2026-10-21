# Proposal

## Why

The morph start builds the controller's per-vertex records: it creates the owner's helper when missing, ensures the control points, binds every owner vertex to its nearest control candidate and stores the candidate's barycentric weights as the vertex's offset. Exarust has the basis solver and the vertex apply but not the start.

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
- Evidence: the morph start: the owner's helper created from its class with two of its virtuals, the controls ensured, the frame over the owner's output, the nearest callback (squared distance summed `(dx² + dy²) + dz²`, strictly below the maximum float), the basis solver called with the nearest point, the unmatched count and list, and the 28-byte record (joint, offset, base). No new corpus files.
- Out of scope: the 1,128-byte apply/frame routine.

## Phase

Phase 3, bodies. The morph start.
