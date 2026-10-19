# Proposal

## Why

Phase 7 places a shell vertex. The copied triangle list is not in.

## What Changes

- A shell copies each section that is not marked hidden, from its first triangle through its last.
- Each triangle keeps its three indices in that order, after the remap.
- A hidden section is skipped. A range that ends before it starts copies nothing.
- A short index list or a missing remap is an error.

## Capabilities

### New Capabilities

### Modified Capabilities

- `thaumaturgy`: the shell's copied index list.

## Impact

- `exa_game::thaum`, beside the shell vertex.
- `docs/formats/OTHER.md`, `docs/PLAN.md`, `docs/COMPONENT-MAP.md`.
- Evidence: the shell index copy. No new corpus files. Tests use synthetic triangles.
- Out of scope: the skinned section filter, and the stored size of 800.
