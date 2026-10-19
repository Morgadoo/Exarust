# Proposal

## Why

A form still has no quad where the two lower-right arms join, after the strip drawn when mask bits 4 and 8 are both set.

## What Changes

- Both tab arms then draw one quad from 70, 70 to 90, 80 at unit 5, width 100 and height 80.
- A set tab bit still draws it, and the wider strip remains.
- A missing bit draws none.
- A clear hold bit draws the wider quad, from 60, 80 to 80, 90 at height 100.

## Capabilities

### New Capabilities

### Modified Capabilities

- `user-interface`: the quad both tab arms join into after the narrower strip.

## Impact

- `exa_ui::skin`, from `dispatch_form`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`.
- Evidence: the join after the lower-right strip. No new corpus files.
- Out of scope: the other arm, and the helper body on the clear-hold path.

## Phase

Phase 7, the interface. The quad both arms join into, not the other arm.
