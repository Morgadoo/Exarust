# Proposal

## Why

The parent skin draws one inset rectangle through a virtual and, when flag bit 0x40 is set, adds a cornered overlay at the same rectangle. Exarust does not model that dispatch.

## What Changes

- The rectangle is the place inset by the stored inset on every side.
- Two booleans are read from flag bits 0x10 and 0x40 and passed to the virtual.
- Flag bit 0x40 adds one cornered overlay call at the same rectangle.

## Capabilities

### New Capabilities

### Modified Capabilities

- `user-interface`: the parent skin's rectangle dispatch.

## Impact

- `exa_ui::skin`, from `draw_skin_rect`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`.
- Evidence: the 204-byte parent method and the cornered helper's two binds. No new corpus files.
- Out of scope: the virtual body and the overlay's pixels.

## Phase

Phase 7, the interface. The dispatch, not the overlay.
