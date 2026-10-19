# Proposal

## Why

A wide form draws a top and a left side, and the bottom that follows that side is still missing.

## What Changes

- The same wide form draws a bottom of one bar and two caps.
- At unit 5, width 100 and height 80 the bar runs from 10, 75 to 90, 85.
- Style 0x10 without bits 0x80, and a form that is not wide, draw none.

## Capabilities

### New Capabilities

### Modified Capabilities

- `user-interface`: the bottom that follows the wide left side.

## Impact

- `exa_ui::skin`, from `dispatch_form`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`.
- Evidence: the bottom after the left side. No new corpus files.
- Out of scope: the right side.

## Phase

Phase 7, the interface. The bottom, not the right side.
