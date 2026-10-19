# Proposal

## Why

A held form draws a left strip, and the cap under that strip is still missing.

## What Changes

- Mask bits 1 and 2 together draw a cap under the left strip.
- At unit 5 the cap runs from 0, 10 to 10, 30.
- An edge of 5, 5 draws none, and a clear hold bit draws none.

## Capabilities

### New Capabilities

### Modified Capabilities

- `user-interface`: the cap that follows the held left strip.

## Impact

- `exa_ui::skin`, from `dispatch_form`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`.
- Evidence: the cap after the left strip. No new corpus files.
- Out of scope: the flag test after that cap.

## Phase

Phase 7, the interface. The cap, not the flag test after it.
