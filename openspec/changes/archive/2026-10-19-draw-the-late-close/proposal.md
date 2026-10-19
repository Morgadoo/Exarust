# Proposal

## Why

The dispatcher joins after the ink copy, and the square drawn there when the close mark is -4 is still missing.

## What Changes

- A close mark of -4 draws one square after the other quads.
- At unit 5 and width 100 the square runs from 100, 0 to 80, 20.
- A mark of 0 or -3 draws none.

## Capabilities

### New Capabilities

### Modified Capabilities

- `user-interface`: the square drawn at the join when the close mark is -4.

## Impact

- `exa_ui::skin`, from `dispatch_form`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`.
- Evidence: the join after the ink copy. No new corpus files.
- Out of scope: the hold-bit edge test.

## Phase

Phase 7, the interface. The late square, not the hold-bit edge test.
