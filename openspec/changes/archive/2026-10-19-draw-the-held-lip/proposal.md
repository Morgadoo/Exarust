# Proposal

## Why

A form still has no narrower strip when mask bits 4 and 8 are both set and the tab bit is clear.

## What Changes

- A clear tab bit then draws one strip from 90, 50 to 100, 70 at unit 5, width 100 and height 80.
- A set tab bit draws the wider strip instead.
- A clear hold bit with a clear tab bit draws the wider clear strip, from 80, 60 to 90, 80 at height 100.

## Capabilities

### New Capabilities

### Modified Capabilities

- `user-interface`: the narrower strip drawn when the tab bit is clear.

## Impact

- `exa_ui::skin`, from `dispatch_form`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`.
- Evidence: the clear-tab arm after the lower-right strip. No new corpus files.
- Out of scope: the next quad.

## Phase

Phase 7, the interface. The narrower strip, not the next quad.
