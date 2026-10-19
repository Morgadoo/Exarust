# Proposal

## Why

The line class stores three row heights that follow the interface scale. No Exarust code scales them yet.

## What Changes

- A scale of 1 leaves the heights 80, 104 and 128.
- A scale of one half stores 40, 52 and 64.
- A scale of 2 stores 160, 208 and 0, because the low byte wraps.
- One call to the parent's scale is counted.

## Capabilities

### New Capabilities

### Modified Capabilities

- `user-interface`: the line class's scaled row heights.

## Impact

- `exa_ui::skin`, from `scale_line`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`.
- Evidence: the line-class scale method and the three float constants it multiplies. No new corpus files.
- Out of scope: the parent's scale body and the methods around it.

## Phase

Phase 7, the interface. The heights, not the parent's scale.
