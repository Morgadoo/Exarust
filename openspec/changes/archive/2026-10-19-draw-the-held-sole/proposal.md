# Proposal

## Why

A form still has no ledge after the lower strip when mask bits 2 and 8 are both set.

## What Changes

- Those bits then draw one ledge from 10, 70 to 30, 80 at unit 5 and height 80.
- A missing bit draws none. A set tab bit still draws it.
- A clear hold bit draws the wider ledge, from 20, 80 to 40, 90 at height 100.

## Capabilities

### New Capabilities

### Modified Capabilities

- `user-interface`: the ledge that follows the lower strip.

## Impact

- `exa_ui::skin`, from `dispatch_form`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`.
- Evidence: the quad after the lower strip. No new corpus files.
- Out of scope: the other arm of that compare.

## Phase

Phase 7, the interface. The ledge, not the other arm.
