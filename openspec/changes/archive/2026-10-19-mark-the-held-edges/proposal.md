# Proposal

## Why

A form that is not wide still has a hold-bit test, and the edge mask it stores is missing.

## What Changes

- With the hold bit and style bits 0xc0 equal to 0x80, a y or x below the unit sets a mask bit.
- A sum past the room on the right or the bottom sets another bit.
- A clear hold bit, a wide bit, and style 0x10 without those bits set none.

## Capabilities

### New Capabilities

### Modified Capabilities

- `user-interface`: the hold-bit edge mask on the path that is not wide.

## Impact

- `exa_ui::skin`, from `dispatch_form`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`.
- Evidence: the hold-bit compares. No new corpus files.
- Out of scope: the quads after that mask.

## Phase

Phase 7, the interface. The edge mask, not the quads after it.
