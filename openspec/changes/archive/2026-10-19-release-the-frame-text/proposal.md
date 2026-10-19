# Proposal

## Why

The frame helper still does not release the string slot it is given, and the bytes the return jumps over are still unread.

## What Changes

- The helper releases one string slot.
- That slot starts empty, and an empty slot is not freed.
- A held band, a wide form, and a style that is not that band each release it once.
- The bytes the jump skips do not run.

## Capabilities

### New Capabilities

### Modified Capabilities

- `user-interface`: the string release inside the frame helper, and the skipped block staying untaken.

## Impact

- `exa_ui::skin`, from `dispatch_form`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`.
- Evidence: the frame helper and the bytes the return jumps over. No new corpus files.
- Out of scope: freeing a slot that is not empty, and the calls inside the skipped block.

## Phase

Phase 7, the interface. The empty-string release, not a raised fault.
