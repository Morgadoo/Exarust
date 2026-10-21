# Proposal

## Why

The class-select confirm stores the chosen class, copies it to the globals, plays a sound and opens the next form; a zero or negative selection returns 0x40 instead. Exarust does not model it.

## What Changes

- A positive selection stores itself and one less in the global, returns 7, plays a sound when the global is positive, copies the menu page and shows the class form.
- A zero or negative selection returns 0x40 and does nothing else.

## Capabilities

### New Capabilities

### Modified Capabilities

- `user-interface`: the class-select confirm.

## Impact

- `exa_ui::menu`, from `confirm_class`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`.
- Evidence: the 154-byte class-select confirm's guard, globals and form call. No new corpus files.
- Out of scope: the sound and form bodies.

## Phase

Phase 7, the interface. The confirm.
