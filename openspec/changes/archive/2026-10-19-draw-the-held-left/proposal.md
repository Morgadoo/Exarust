# Proposal

## Why

A held form draws a post quad, and the left strip that follows that quad is still missing.

## What Changes

- A held form that is not wide draws a left strip.
- At unit 5 and height 80 the strip runs from 0, 30 to 10, 50.
- A clear bit 0x200 still draws it. A clear hold bit and a wide bit draw none.

## Capabilities

### New Capabilities

### Modified Capabilities

- `user-interface`: the left strip that follows the post quad.

## Impact

- `exa_ui::skin`, from `dispatch_form`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`.
- Evidence: the strip after the post quad. No new corpus files.
- Out of scope: the mask compare after that strip.

## Phase

Phase 7, the interface. The left strip, not the compare after it.
