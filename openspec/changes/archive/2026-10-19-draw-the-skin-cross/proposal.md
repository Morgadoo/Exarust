# Proposal

## Why

The parent skin draws a small cross of four line segments around a point. Exarust does not model it.

## What Changes

- The half extent is the unit plus one, halved, plus one.
- Eight corners form four lines around the point: two horizontal at y minus and plus the unit, and two vertical at x minus and plus the unit plus one.
- At unit 5 and point 10, 20 the corners are (8, 15), (13, 15), (8, 25), (13, 25), (6, 18), (6, 23), (16, 18), (16, 23).

## Capabilities

### New Capabilities

### Modified Capabilities

- `user-interface`: the parent skin's cross.

## Impact

- `exa_ui::skin`, from `draw_skin_cross`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`.
- Evidence: the parent cross method's eight vertex calls. No new corpus files.
- Out of scope: the caller that picks the point.

## Phase

Phase 7, the interface. The cross, not its point.
