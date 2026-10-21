# Proposal

## Why

The journal and the character-loot screen share an 89-byte key override: when the key mask carries both bits 0x2000 and 0x2, the screen closes with argument 0 and returns 1. Otherwise the base handler runs. Exarust does not model it.

## What Changes

- A mask with both bits 0x2000 and 0x2 closes and returns 1 without calling the base handler.
- Any other mask returns the base handler's result unchanged.

## Capabilities

### New Capabilities

### Modified Capabilities

- `user-interface`: the journal and loot close key.

## Impact

- `exa_ui::menu`, from `menu_key_close`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`.
- Evidence: the two identical 89-byte key-close overrides. No new corpus files.
- Out of scope: the base handler body and the close's own body.

## Phase

Phase 7, the interface. The key decision, not the close.
