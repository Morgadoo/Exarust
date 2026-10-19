# Proposal

## Why

A held form still has no bar after the right strip when the tab bit is set and mask bit 4 is clear.

## What Changes

- A clear bit 0x200 of the plain word then draws one bar from 96, 20 to 106, 60 at unit 5, width 100 and height 80.
- That plain bit draws none, and the right strip remains.
- Mask bit 4, a clear tab bit, and a clear hold bit draw none.

## Capabilities

### New Capabilities

### Modified Capabilities

- `user-interface`: the bar after the right strip.

## Impact

- `exa_ui::skin`, from `dispatch_form`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`.
- Evidence: the bar after the right strip. No new corpus files.
- Out of scope: the drawing after that bar.

## Phase

Phase 7, the interface. The bar, not the drawing after it.
