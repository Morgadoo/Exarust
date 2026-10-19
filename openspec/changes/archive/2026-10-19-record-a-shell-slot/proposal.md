# Proposal

## Why

Phase 7 numbers the vertices a skinned shell keeps. The section and the local index stored beside each kept vertex are not in.

## What Changes

- A kept vertex stores its section and the corner index minus that section's base.
- The first link in its half-open range whose mark is not 255 supplies both. A later link is ignored.
- A vertex with none is absent.
- A short corner list or a missing base is an error. A base of -1 subtracted from 1 stores 2.

## Capabilities

### New Capabilities

### Modified Capabilities

- `thaumaturgy`: the section slot beside a compacted shell vertex.

## Impact

- `exa_game::thaum`, beside the compacted vertex map.
- `docs/formats/OTHER.md`, `docs/PLAN.md`, `docs/COMPONENT-MAP.md`.
- Evidence: the skinned vertex slot. No new corpus files. Tests use synthetic corners.
- Out of scope: the stored size of 800, and writing the GPU buffers.
