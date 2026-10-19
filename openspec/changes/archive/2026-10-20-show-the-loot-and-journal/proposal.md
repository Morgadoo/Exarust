# Proposal

## Why

The character-loot screen marks its owner and places a visible child when it becomes active, then calls the base method. The journal copies its origin, assigns a page string from a table row and calls the base. Exarust does not model either.

## What Changes

- An already inactive loot screen returns true without work.
- Otherwise it marks the owner's word with 0x20000000 and places a visible child.
- The journal copies its origin to two globals, assigns the string at its page's table row and calls the base.

## Capabilities

### New Capabilities

### Modified Capabilities

- `user-interface`: the loot and journal show paths.

## Impact

- `exa_ui::menu`, from `show_char_loot` and `show_journal`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`.
- Evidence: the 112-byte loot and 93-byte journal methods. No new corpus files.
- Out of scope: the base methods and the table.

## Phase

Phase 7, the interface. The show paths.
