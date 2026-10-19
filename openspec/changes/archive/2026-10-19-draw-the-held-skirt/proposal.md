# Proposal

## Why

A held form still has no quad when mask bits 2 and 8 are not both set.

## What Changes

- Those bits not both set draw one quad from -10, 50 to 30, 90 at unit 5 and height 80.
- Both bits set draw the ledge instead.
- A clear hold bit draws none.

## Capabilities

### New Capabilities

### Modified Capabilities

- `user-interface`: the quad drawn when mask bits 2 and 8 differ.

## Impact

- `exa_ui::skin`, from `dispatch_form`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`.
- Evidence: the other arm of the mask compare. No new corpus files.
- Out of scope: the next quad.

## Phase

Phase 7, the interface. The quad, not the next one.
