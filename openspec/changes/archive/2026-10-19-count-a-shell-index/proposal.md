# Proposal

## Why

Phase 7 stores a skinned shell vertex's section. The index room those triangles ask for is not counted.

## What Changes

- A section with bit 2 adds one for each triangle from the first through the last. A section without that bit adds nothing. A hidden section that also has bit 2 still counts.
- The room asks for 12 bytes plus 8, plus 4 bytes a triangle.
- The section kind is 3, the base is 0, the end is one less than the triangle count, and the vertex end is one less than the kept count.
- A range that ends one early adds nothing. A range that ends two early subtracts one.

## Capabilities

### New Capabilities

### Modified Capabilities

- `thaumaturgy`: the counted index room of a skinned shell.

## Impact

- `exa_game::thaum`, beside the section slot.
- `docs/formats/OTHER.md`, `docs/PLAN.md`, `docs/COMPONENT-MAP.md`.
- Evidence: the skinned index-room count. No new corpus files. Tests use synthetic ranges.
- Out of scope: the stored size of 800, and uploading the GPU buffers.
