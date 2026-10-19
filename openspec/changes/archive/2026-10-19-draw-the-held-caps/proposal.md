# Proposal

## Why

A held form still has no caps after the bar when the tab bit is set and mask bit 4 is clear.

## What Changes

- The same clear plain bit then draws two caps.
- At unit 5, width 100 and height 80 the upper cap runs from 96, 0 to 106, 20 and the lower cap runs from 96, 60 to 106, 80.
- That plain bit, mask bit 4, a clear tab bit, and a clear hold bit draw none.

## Capabilities

### New Capabilities

### Modified Capabilities

- `user-interface`: the two caps that finish the bar after the right strip.

## Impact

- `exa_ui::skin`, from `dispatch_form`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`.
- Evidence: the two caps after the bar. No new corpus files.
- Out of scope: the drawing on the clear tab path.

## Phase

Phase 7, the interface. The two caps, not the clear-tab drawing.
