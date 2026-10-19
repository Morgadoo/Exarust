# Proposal

## Why

A wide form draws a top, a left side and a bottom, and the right side that follows is still missing.

## What Changes

- The same wide form draws a right side of one bar and two caps.
- At unit 5, width 100 and height 80 the bar runs from 95, 10 to 105, 70.
- Style 0x10 without bits 0x80, and a form that is not wide, draw none.

## Capabilities

### New Capabilities

### Modified Capabilities

- `user-interface`: the right side that follows the wide bottom.

## Impact

- `exa_ui::skin`, from `dispatch_form`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`.
- Evidence: the right side after the bottom. No new corpus files.
- Out of scope: the mark after that side.

## Phase

Phase 7, the interface. The right side, not the mark after it.
