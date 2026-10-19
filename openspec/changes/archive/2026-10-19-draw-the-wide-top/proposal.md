# Proposal

## Why

A wide form skips the corners and the edge bars, and the top it draws instead is still missing.

## What Changes

- A wide form whose style word has bits 0xc0 equal to 0x80 draws three top quads.
- At unit 5 and width 100 the bar runs from 10, -5 to 90, 5, with a cap on each end.
- Style 0x10 without those bits, and a form that is not wide, draw none.

## Capabilities

### New Capabilities

### Modified Capabilities

- `user-interface`: the wide top that follows the edge bars.

## Impact

- `exa_ui::skin`, from `dispatch_form`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`.
- Evidence: the three top quads on the wide path. No new corpus files.
- Out of scope: the side bars after that top.

## Phase

Phase 7, the interface. The wide top, not the side bars after it.
