# Proposal

## Why

A held form draws a tab, and the lower tab that follows that tab is still missing.

## What Changes

- The same bit draws a lower tab hanging from the seat.
- A seat of 80 at unit 5 sits from 16, 60 to 26, 70.
- Mask bits 4 and 8 together raise that tab so its top is 65.

## Capabilities

### New Capabilities

### Modified Capabilities

- `user-interface`: the lower tab that follows the held tab.

## Impact

- `exa_ui::skin`, from `dispatch_form`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`.
- Evidence: the anchor after the held tab. No new corpus files.
- Out of scope: the anchor after that lower tab.

## Phase

Phase 7, the interface. The lower tab, not the anchor after it.
