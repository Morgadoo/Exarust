# Proposal

## Why

A form still has no quad when mask bits 4 and 8 are not both set and the tab bit is set.

## What Changes

- The tab bit then draws one quad from 70, 50 to 110, 90 at unit 5, width 100 and height 80.
- Both bits set draw the bottom quad instead.
- A clear tab bit draws none.
- A clear hold bit draws none.

## Capabilities

### New Capabilities

### Modified Capabilities

- `user-interface`: the quad drawn when mask bits 4 and 8 differ and the tab bit is set.

## Impact

- `exa_ui::skin`, from `dispatch_form`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`.
- Evidence: the tab-set arm after the bottom quad. No new corpus files.
- Out of scope: the clear-tab arm, and the helper body on the clear-hold path.

## Phase

Phase 7, the interface. The tab-set quad, not the clear-tab arm.
