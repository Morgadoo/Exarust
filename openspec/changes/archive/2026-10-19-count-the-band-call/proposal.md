# Proposal

## Why

A banded form still does not count the call that follows the close-mark compare, whether or not that compare draws the square.

## What Changes

- On that band the dispatcher counts one call with argument 0.
- A close mark of -4 still counts one, and the square remains.
- A style that is not that band counts none.
- A wide bit counts none.

## Capabilities

### New Capabilities

### Modified Capabilities

- `user-interface`: the call counted after the close-mark compare on the band.

## Impact

- `exa_ui::skin`, from `dispatch_form`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`.
- Evidence: the call after the close-mark compare. No new corpus files.
- Out of scope: the bytes the jump skips, and the helper body on the clear-hold path.

## Phase

Phase 7, the interface. The call count, not the bytes the jump skips.
