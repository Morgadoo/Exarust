# Proposal

## Why

Phase 7 keeps a skinned shell section. The map that numbers the vertices that section keeps is not in.

## What Changes

- A mark list starts at 255. A skinned section writes its index. Any other section writes 255.
- A vertex takes the next compacted id when a link in its half-open range names a mark other than 255, and stops at the first such mark.
- A vertex with none is absent. An index becomes that id.
- A short list or an absent id is an error.

## Capabilities

### New Capabilities

### Modified Capabilities

- `thaumaturgy`: the compacted shell vertex map.

## Impact

- `exa_game::thaum`, beside the skinned section filter.
- `docs/formats/OTHER.md`, `docs/PLAN.md`, `docs/COMPONENT-MAP.md`.
- Evidence: the skinned vertex numbering. No new corpus files. Tests use synthetic marks.
- Out of scope: the stored size of 800, and writing the GPU buffers.
