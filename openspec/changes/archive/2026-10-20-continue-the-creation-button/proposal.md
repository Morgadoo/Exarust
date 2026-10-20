# Proposal

## Why

The creation screen's continue button is labelled "Continue from save" or "Continue from checkpoint" from the mode, mirrored with bit `0x40000000` and enabled by a threshold. Exarust has several creation behaviours but not the button.

## What Changes

- Mode 2 gives "Continue from save" and sets bit `0x40000000`.
- Any other mode gives "Continue from checkpoint" and clears the bit.
- Bit `0x10` is set when the mode is not below the threshold.

## Capabilities

### New Capabilities

### Modified Capabilities

- `character-creation`: the continue button.

## Impact

- `exa_game::create`, from `creation_continue_button`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`: `SG_Character_Create` moves from 11% to 12%.
- Evidence: the 153-byte `0x10012f840`: the mode at `0x274`, the label literals at `0x100316238` and `0x100316268`, the `0x40000000` bit at `0x18`, the threshold at `0x1004f0400` and the `0x10` bit at `0x1c`. No new corpus files.
- Out of scope: the two preparation calls and the widget lookup.

## Phase

Phase 5, character creation. The continue button.
