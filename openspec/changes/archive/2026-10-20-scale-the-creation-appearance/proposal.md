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
- Evidence: the 173-byte and 149-byte appearance scales: the three source floats, the constants `255.0`, `0.0` and `65535.0`, `cvtss2si`, and the three byte stores and two word stores. No new corpus files.
- Out of scope: the two calls that follow each scale.

## Phase

Phase 5, character creation. The appearance scales.
