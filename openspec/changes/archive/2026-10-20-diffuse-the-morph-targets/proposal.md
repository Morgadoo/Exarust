# Proposal

## Why

The morph cook fills the vertices it could not bind from their neighbours: each unknown vertex takes the weighted mean of the known neighbours at `1/√(distance squared)`, is marked known for the next pass, and the passes repeat until nothing changes. Exarust has the cook's blob, deltas and skin weights, but not the target diffusion.

## What Changes

- An unknown vertex with known neighbours takes their weighted mean at `1/√(distance squared)`.
- It becomes known for the next pass.
- Passes repeat until one changes nothing; the number of changing passes is returned.

## Capabilities

### New Capabilities

### Modified Capabilities

- `morph-controller`: the target diffusion.

## Impact

- `exa_assets::morph_controller`, from `diffuse_morph_deltas`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`: `RF_Morphing` becomes **40%** (6,843 of 17,099 bytes). The published headline stays 14.4% and 5.7%; the fresh sum stays about 15.5%.
- Evidence: `0x1000dbdc0` phase F: the target bitmap cleared for the target list, the adjacency `obj+0x2c0`, the packed `(triangle, corner)` pairs at `obj+0x2b8`, the cyclic corner lookup `{1, 2, 0}` at `0x1002b4500`, the weight `1/√(distance squared)` at `0x1000dc384`, the weighted accumulation, the bit-2 mark and the bit-2-to-bit-1 promotion loop (`0x1000dc438` through `0x1000dc474`). No new corpus files.
- Out of scope: the stream registration and the owner's re-bind.

## Phase

Phase 3, bodies. The morph target diffusion.
