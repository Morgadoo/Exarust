# Proposal

## Why

A held form still has no quad when the two edge bits are not both set.

## What Changes

- That form draws one quad from -10, -10 to 30, 30 at unit 5.
- An edge of 4, 4 draws none, and the cap remains.
- A clear hold bit draws none.

## Capabilities

### New Capabilities

### Modified Capabilities

- `user-interface`: the sign-flipped quad on a held form.

## Impact

- `exa_ui::skin`, from `dispatch_form`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`.
- Evidence: the quad after the edge-bit compare. No new corpus files.
- Out of scope: the next mask test.

## Phase

Phase 7, the interface. The quad, not the next mask test.
