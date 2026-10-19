# Proposal

## Why

A held form still has no quad when mask bits 1 and 4 are not both set and the tab bit is set.

## What Changes

- Bit 0x200 then draws one quad from 70, -10 to 110, 30 at unit 5 and width 100.
- A clear bit 0x200 draws none.
- An edge of 4, 4 with a right room of 100 draws the wider cap instead.

## Capabilities

### New Capabilities

### Modified Capabilities

- `user-interface`: the quad past the right edge when mask bits 1 and 4 differ.

## Impact

- `exa_ui::skin`, from `dispatch_form`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`.
- Evidence: the quad after the shelf's join. No new corpus files.
- Out of scope: the quad drawn when that tab bit is clear.

## Phase

Phase 7, the interface. The quad, not the clear-tab quad on that path.
