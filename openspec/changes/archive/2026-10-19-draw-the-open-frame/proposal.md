# Proposal

## Why

A form with the band bits and no wide bit still has no drawing when the hold bit is clear.

## What Changes

- A clear hold bit forces mask 15 and draws the wider strip, cap, and ledge.
- At unit 5 and height 100 the strip runs from 10, 40 to 20, 60.
- A clear bit 1 counts one call. A set bit counts none. The held mask stays 0.

## Capabilities

### New Capabilities

### Modified Capabilities

- `user-interface`: the wider frame used when the hold bit is clear.

## Impact

- `exa_ui::skin`, from `dispatch_form`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`.
- Evidence: the setup reached when the hold bit is clear. No new corpus files.
- Out of scope: the helper body.

## Phase

Phase 7, the interface. The wider frame, not the helper body.
