# Proposal

## Why

A clear tab bit draws a narrow cap, and the shelf that follows that cap is still missing.

## What Changes

- The same clear tab bit draws a shelf from 70, 0 to 90, 10 at unit 5 and width 100.
- Bit 1, or the owner's bit 1, draws none unless the owner also has bit 0x800000.
- A set bit 0x200, a missing edge bit, and a clear hold bit draw none.

## Capabilities

### New Capabilities

### Modified Capabilities

- `user-interface`: the shelf that follows the narrow cap.

## Impact

- `exa_ui::skin`, from `dispatch_form`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`.
- Evidence: the flag test after the narrow cap. No new corpus files.
- Out of scope: the quad after that shelf.

## Phase

Phase 7, the interface. The shelf, not the quad after it.
