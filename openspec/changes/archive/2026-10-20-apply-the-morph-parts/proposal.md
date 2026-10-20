# Proposal

## Why

The morph controller applies its morph to every eligible part: a node whose kind masked with `0xFFF` is `0x89` and whose `0xC00000` bits are `0x400000`, then to each child. Exarust has the blend and nearest searches but not the part walk.

## What Changes

- A node is eligible when `kind & 0xFFF == 0x89` and `flags & 0xC00000 == 0x400000`.
- The walk visits every child recursively.
- The number of eligible nodes is returned.

## Capabilities

### New Capabilities

### Modified Capabilities

- `morph-controller`: the part walk.

## Impact

- `exa_assets::morph_controller`, from `apply_morph_parts`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`: `RF_Morphing` moves from 5% to 6%.
- Evidence: the 154-byte `0x1000dbc30`: the kind mask `0xfff` at `0x8`, the flag band `0xc00000` at `0x204`, the apply `0x1000645e0`, the child count `0x118` and list `0x120`, and the recursive call. No new corpus files.
- Out of scope: the apply call.

## Phase

Phase 5, characters. The morph part walk.
