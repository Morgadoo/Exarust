# Proposal

## Why

The creation screen steps the face variant down one: it rewrites bits 20 through 23 and updates the widget caption. Exarust has the look update and scales but not the step.

## What Changes

- The variant is bits 20 through 23.
- Below 1 nothing changes.
- Otherwise the variant falls by one and is written back into those bits.
- With the widget present, its caption is the default at the new variant 0, or the byte `(variant − 1) + 0x41`.
- The refresh runs after the step.

## Capabilities

### New Capabilities

### Modified Capabilities

- `character-creation`: the variant step.

## Impact

- `exa_game::create`, from `step_creation_variant`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`: `SG_Character_Create` moves from 7% to 8%.
- Evidence: the 249-byte `0x10012d3b0`: the `0xf00000` variant shifted by 20, the `0xff0fffff` mask, the `0x41` caption byte, the default `0x100315e38`, the widget lookup `0x100` and the two refresh calls. No new corpus files.
- Out of scope: the widget and refresh routines.

## Phase

Phase 5, character creation. The variant step.
