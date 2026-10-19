# Proposal

## Why

A held form still has no quad when mask bits 1 and 4 are not both set and the tab bit is clear.

## What Changes

- A clear bit 0x200 then draws one quad from 70, -10 to 110, 30 at unit 5 and width 100.
- A set bit 0x200 draws the other sample instead.
- An edge of 4, 4 with a right room of 100 draws none of either.

## Capabilities

### New Capabilities

### Modified Capabilities

- `user-interface`: the same right-edge corners when the tab bit is clear.

## Impact

- `exa_ui::skin`, from `dispatch_form`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`.
- Evidence: the quad after the tabbed right-edge join. No new corpus files.
- Out of scope: the drawing after that join.

## Phase

Phase 7, the interface. The clear-tab quad, not the drawing after that join.
