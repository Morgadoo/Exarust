# Proposal

## Why

A form still has no quad when mask bits 4 and 8 are not both set and the tab bit is clear.

## What Changes

- A clear tab bit then draws one quad from 70, 50 to 110, 90 at unit 5, width 100 and height 80.
- A set tab bit draws the other sample instead.
- Both bits set draw none.
- A clear hold bit draws none.

## Capabilities

### New Capabilities

### Modified Capabilities

- `user-interface`: the quad drawn when mask bits 4 and 8 differ and the tab bit is clear.

## Impact

- `exa_ui::skin`, from `dispatch_form`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`.
- Evidence: the clear-tab arm after the tab-set quad. No new corpus files.
- Out of scope: the join after that quad, and the helper body on the clear-hold path.

## Phase

Phase 7, the interface. The clear-tab quad, not the join after it.
