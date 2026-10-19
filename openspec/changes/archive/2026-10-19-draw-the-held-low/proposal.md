# Proposal

## Why

A form still has no lower strip when mask bits 2 and 8 are both set.

## What Changes

- Those bits draw one strip from 0, 50 to 10, 70 at unit 5 and height 80.
- A missing bit draws none. A set tab bit still draws it.
- A clear hold bit draws the wider strip, from 10, 60 to 20, 80 at height 100.

## Capabilities

### New Capabilities

### Modified Capabilities

- `user-interface`: the lower strip drawn when mask bits 2 and 8 are set.

## Impact

- `exa_ui::skin`, from `dispatch_form`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`.
- Evidence: the quad after the mask compare. No new corpus files.
- Out of scope: the next quad.

## Phase

Phase 7, the interface. The lower strip, not the next quad.
