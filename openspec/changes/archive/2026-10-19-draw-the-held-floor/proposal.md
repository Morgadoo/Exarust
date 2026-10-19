# Proposal

## Why

A form on that band still has no bottom bar after the two mask arms.

## What Changes

- Both arms then draw one bar from 30, 70 to 70, 80 at unit 5, width 100 and height 80.
- A set tab bit still draws it.
- A clear hold bit draws the wider bar, from 40, 80 to 60, 90 at height 100.

## Capabilities

### New Capabilities

### Modified Capabilities

- `user-interface`: the bottom bar both mask arms join into.

## Impact

- `exa_ui::skin`, from `dispatch_form`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`.
- Evidence: the bar after the two mask arms. No new corpus files.
- Out of scope: the next mask test.

## Phase

Phase 7, the interface. The bar, not the next mask test.
