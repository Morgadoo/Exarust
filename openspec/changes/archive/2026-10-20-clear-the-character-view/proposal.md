# Proposal

## Why

The character view has a 73-byte destructor and an 86-byte clear override. Exarust does not model either.

## What Changes

- The destructor releases on a set free flag, always runs the base destructor and makes a final call when freeing.
- The clear releases the view when present, clears the owner's 0x130 word, clears the view pointer and always runs the base method, returning its result.

## Capabilities

### New Capabilities

### Modified Capabilities

- `user-interface`: the character view's clear and destruction.

## Impact

- `exa_ui::menu`, from `destroy_character_view` and `clear_character_view`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`.
- Evidence: the 73- and 86-byte methods and their conditional calls. No new corpus files.
- Out of scope: the release and base bodies.

## Phase

Phase 7, the interface. The view lifecycle.
