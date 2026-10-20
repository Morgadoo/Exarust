# Proposal

## Why

The creation screen also steps the face variant up, stopping at 6, and updates the widget caption. Exarust has the down step but not the up step.

## What Changes

- The variant is bits 20 through 23.
- Above 5 nothing changes.
- Otherwise the variant rises by one and is written back into those bits.
- With the widget present, its caption is the byte `(variant − 1) + 0x41` for the new variant.
- The refresh runs after the step.

## Capabilities

### New Capabilities

### Modified Capabilities

- `character-creation`: the variant advance.

## Impact

- `exa_game::create`, from `advance_creation_variant`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`: `SG_Character_Create` moves from 8% to 9%.
- Evidence: the 249-byte `0x10012d4d0`: the `0xf00000` variant shifted by 20, the `5` cap, the `0xff0fffff` mask, the `0x41` caption byte, the widget lookup `0x100` and the two refresh calls. No new corpus files.
- Out of scope: the widget and refresh routines.

## Phase

Phase 5, character creation. The variant advance.
