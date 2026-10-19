# Proposal

## Why

A form still has no lower-right strip when mask bits 4 and 8 are both set and the tab bit is set.

## What Changes

- Those bits with the tab bit draw one strip from 80, 50 to 100, 70 at unit 5, width 100 and height 80.
- A missing bit draws none. A clear tab bit draws none.
- A clear hold bit with the tab bit draws the wider strip, from 70, 60 to 90, 80 at height 100.

## Capabilities

### New Capabilities

### Modified Capabilities

- `user-interface`: the lower-right strip drawn when mask bits 4 and 8 and the tab bit are set.

## Impact

- `exa_ui::skin`, from `dispatch_form`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`.
- Evidence: the quad after the mask compare. No new corpus files.
- Out of scope: the clear-tab arm of that compare.

## Phase

Phase 7, the interface. The strip, not the clear-tab arm.
