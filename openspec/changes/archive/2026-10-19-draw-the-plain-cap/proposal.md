# Proposal

## Why

A held form still has no cap when mask bits 1 and 4 are set and the tab bit is clear.

## What Changes

- A clear bit 0x200 draws a narrower cap from 90, 10 to 100, 30 at unit 5 and width 100.
- A set bit 0x200 draws the wider cap instead.
- An edge that misses either bit, and a clear hold bit, draw none.

## Capabilities

### New Capabilities

### Modified Capabilities

- `user-interface`: the narrow cap drawn when the tab bit is clear.

## Impact

- `exa_ui::skin`, from `dispatch_form`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`.
- Evidence: the quad after a clear tab bit. No new corpus files.
- Out of scope: the flag test after that cap.

## Phase

Phase 7, the interface. The narrow cap, not the flag test after it.
