# Proposal

## Why

A held form still has no right strip when the tab bit is clear.

## What Changes

- A clear tab bit draws one strip from 90, 30 to 100, 50 at unit 5, width 100 and height 80.
- A set tab bit draws none.
- A clear hold bit with the tab bit clear draws the wider strip, from 80, 40 to 90, 60 at height 100.

## Capabilities

### New Capabilities

### Modified Capabilities

- `user-interface`: the right strip drawn when the tab bit is clear.

## Impact

- `exa_ui::skin`, from `dispatch_form`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`.
- Evidence: the strip on the clear tab path. No new corpus files.
- Out of scope: the next mask test.

## Phase

Phase 7, the interface. The strip, not the next mask test.
