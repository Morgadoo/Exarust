# Proposal

## Why

The creation screen shows the sex toggles and a face variant label: female sets four toggles and a default caption, male clears them and letters the variant. Exarust has the label and change handler but not the look update.

## What Changes

- Female sets the four values to 16 and the default caption.
- Otherwise the four values are 0; a variant below 0 keeps the default caption and a variant at or above 0 uses the byte `variant + 0x41`.

## Capabilities

### New Capabilities

### Modified Capabilities

- `character-creation`: the look update.

## Impact

- `exa_game::create`, from `update_creation_look`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`: `SG_Character_Create` moves from 4% to 6%.
- Evidence: the 334-byte `0x10012d650`: the bit 0 test on the `0x54` word, the four widget values at `0x154`, `0x1bc`, `0x224` and `0x28c` set to 16 or 0, the default caption `0x100315e38`, the `0xf00000` variant shifted by 20 and less one, and the `+0x41` byte caption. No new corpus files.
- Out of scope: the string and widget routines.

## Phase

Phase 5, character creation. The look update.
