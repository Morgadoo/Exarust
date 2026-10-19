# Proposal

## Why

A dispatch still does not record the return that passes the frame, restores the saved register, and pops the frame.

## What Changes

- Every dispatch passes the frame to one call.
- It then restores the saved register, sets the stack to the frame, and pops the frame.
- A held band, a wide form, and a style that is not that band each do so.

## Capabilities

### New Capabilities

### Modified Capabilities

- `user-interface`: the return that passes the frame and restores the stack.

## Impact

- `exa_ui::skin`, from `dispatch_form`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`.
- Evidence: the only return of the widget dispatcher. No new corpus files.
- Out of scope: the helper body, and the bytes the jump skips.

## Phase

Phase 7, the interface. The return, not the helper body.
