# Proposal

## Why

A held form stores an edge mask, and the bar that follows that mask is still missing.

## What Changes

- The same held form draws one bar from six times the unit across the top.
- At unit 5 and width 100 the bar runs from 30, 0 to 70, 10.
- A y below the unit with owner bit 1 and without bit 0x800000 draws none.

## Capabilities

### New Capabilities

### Modified Capabilities

- `user-interface`: the bar that follows the hold-bit edge mask.

## Impact

- `exa_ui::skin`, from `dispatch_form`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`.
- Evidence: the bar after the mask. No new corpus files.
- Out of scope: the drawing after that bar.

## Phase

Phase 7, the interface. The held bar, not the drawing after it.
