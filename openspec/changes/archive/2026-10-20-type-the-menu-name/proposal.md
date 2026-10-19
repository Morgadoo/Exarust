# Proposal

## Why

The main menu's character handler shifts a ten-byte name buffer, lowercases A–Z and fires a submit routine when the buffer reads "barechoons". Exarust does not model name entry.

## What Changes

- A key without bit 0x2 goes to the base handler.
- Otherwise the ten bytes shift left by one and the character is lowered if it is A–Z.
- When the buffer then reads "barechoons" the submit routine runs.

## Capabilities

### New Capabilities

### Modified Capabilities

- `user-interface`: the main-menu name entry.

## Impact

- `exa_ui::menu`, from `type_menu_name`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`.
- Evidence: the 225-byte method's ten-byte shift, its A–Z range and the static "barechoons" string at 0x2b7770. No new corpus files.
- Out of scope: the base handler and the submit routine.

## Phase

Phase 7, the interface. Name entry.
