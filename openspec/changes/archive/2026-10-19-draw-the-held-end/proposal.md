# Proposal

## Why

A held form still has no cap when mask bits 1 and 4 are both set.

## What Changes

- Bit 0x200 then draws one cap from 70, 10 to 100, 30 at unit 5 and width 100.
- A clear bit 0x200 draws none.
- An edge that misses either bit, and a clear hold bit, draw none.

## Capabilities

### New Capabilities

### Modified Capabilities

- `user-interface`: the right cap drawn when mask bits 1 and 4 are set.

## Impact

- `exa_ui::skin`, from `dispatch_form`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`.
- Evidence: the quad after the sign flip. No new corpus files.
- Out of scope: the quad drawn when bit 0x200 is clear.

## Phase

Phase 7, the interface. The cap, not the quad after a clear tab bit.
