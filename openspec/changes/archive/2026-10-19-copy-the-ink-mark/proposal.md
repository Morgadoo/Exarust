# Proposal

## Why

A wide form draws its four edges, and the ink copy that follows the right side is still missing.

## What Changes

- On that wide form, bit 0x40 of the first flag word counts one ink copy.
- A clear bit counts none. A form that is not on that wide path counts none.

## Capabilities

### New Capabilities

### Modified Capabilities

- `user-interface`: the ink copy that follows the wide right side.

## Impact

- `exa_ui::skin`, from `dispatch_form`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`.
- Evidence: the flag test after the right side. No new corpus files.
- Out of scope: the join after that copy.

## Phase

Phase 7, the interface. The ink copy, not the join after it.
