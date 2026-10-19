# Proposal

## Why

Phase 7 copies a shell's visible triangles. A skinned shell keeps a section only when bit 2 is set, and that filter is not in.

## What Changes

- A skinned shell copies a section only when its flags include bit 2.
- Each copied triangle keeps its three indices in order, after the remap.
- A section without that bit is skipped, even when it is not hidden.
- A hidden section that also has bit 2 is still copied.
- A range that ends before it starts copies nothing. A short list or a missing remap is an error.

## Capabilities

### New Capabilities

### Modified Capabilities

- `thaumaturgy`: the skinned shell section filter.

## Impact

- `exa_game::thaum`, beside the shell index list.
- `docs/formats/OTHER.md`, `docs/PLAN.md`, `docs/COMPONENT-MAP.md`.
- Evidence: the skinned section test. No new corpus files. Tests use synthetic triangles.
- Out of scope: the compacted vertex map, and the stored size of 800.
