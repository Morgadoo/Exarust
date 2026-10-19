# Proposal

## Why

A held form draws a cap, and the ledge that follows that cap is still missing.

## What Changes

- Mask bits 1 and 2 together draw a ledge from 10, 0 to 30, 10 at unit 5.
- Bit 1, or the owner's bit 1, draws none unless the owner also has bit 0x800000.
- An edge of 5, 5 and a clear hold bit draw none.

## Capabilities

### New Capabilities

### Modified Capabilities

- `user-interface`: the ledge that follows the held cap.

## Impact

- `exa_ui::skin`, from `dispatch_form`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`.
- Evidence: the flag test after the cap. No new corpus files.
- Out of scope: the sign flip after that ledge, and the setup used when the hold bit is clear.

## Phase

Phase 7, the interface. The ledge, not the sign flip after it.
