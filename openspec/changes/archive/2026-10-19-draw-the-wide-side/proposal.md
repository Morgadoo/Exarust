# Proposal

## Why

A wide form draws a top, and the left side that follows that top is still missing.

## What Changes

- The same wide form draws a left side of one bar and two caps.
- At unit 5 and height 80 the bar runs from -5, 10 to 5, 70.
- Style 0x10 without bits 0x80, and a form that is not wide, draw none.

## Capabilities

### New Capabilities

### Modified Capabilities

- `user-interface`: the left side that follows the wide top.

## Impact

- `exa_ui::skin`, from `dispatch_form`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`.
- Evidence: the left side after the wide top. No new corpus files.
- Out of scope: the bottom after that side.

## Phase

Phase 7, the interface. The left side, not the bottom after it.
