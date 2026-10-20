# Proposal

## Why

The creation record stores normalised appearance values as bytes and as 16-bit words. Exarust has the record move, labels, slot clear, settle, copy, change handler and look, but not the scales.

## What Changes

- Three values are multiplied by 255, rounded to the nearest even integer and kept in the low byte.
- Two values are multiplied by 65535, rounded the same way and kept in the low 16 bits.

## Capabilities

### New Capabilities

### Modified Capabilities

- `character-creation`: the appearance scales.

## Impact

- `exa_game::create`, from `appearance_bytes` and `appearance_words`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`: `SG_Character_Create` moves from 6% to 7%.
- Evidence: the 173-byte `0x10012c9c0` and the 149-byte `0x10012ca70`: the source floats at `0x158`/`0x15c`/`0x1a8 + 0x38`, the constants `255.0` at `0x100315db8`, `0.0` at `0x100315dc0` and `65535.0` at `0x100315dc8`, `cvtss2si`, and the byte stores at `0x44`–`0x46` and word stores at `0x3c`/`0x3e`. No new corpus files.
- Out of scope: the two calls that follow each scale.

## Phase

Phase 5, character creation. The appearance scales.
