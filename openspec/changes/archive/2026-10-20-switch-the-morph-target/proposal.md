# Proposal

## Why

The morph controller switches to a tagged target: it copies the point and points, stores the point at the target and refreshes it. Exarust has the setup and nearest scans but not the switch.

## What Changes

- Nothing happens unless the target's tag is `0xEDC0C589`.
- Otherwise the point and points are copied, the point is stored at the target's `0x64` and the target is refreshed.

## Capabilities

### New Capabilities

### Modified Capabilities

- `morph-controller`: the target switch.

## Impact

- `exa_assets::morph_controller`, from `switch_morph_target`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`: `RF_Morphing` moves from 2% to 3%.
- Evidence: the 122-byte rest restore: the target, its tag against `0xedc0c589`, the blob's base point, the points copy, the two stores and the owner refresh. No new corpus files.
- Out of scope: the copy and refresh bodies.

## Phase

Phase 5, characters. The morph target switch.
