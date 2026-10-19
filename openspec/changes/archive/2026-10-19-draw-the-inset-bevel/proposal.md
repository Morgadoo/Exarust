# Proposal

## Why

The parent skin draws an inset bevel for flag bit 0x40 and a plain inset rectangle otherwise. Exarust models the un-inset bevel but not this inset one or its corner detail.

## What Changes

- The method prepares once.
- Flag bit 0x40 draws an inner fill inset by two, a border inset by one, an outline at the inset, and a corner detail at the border rectangle with mode 0xa0 and the unit.
- Any other flag draws one virtual inset rectangle with the flag's two booleans.

## Capabilities

### New Capabilities

### Modified Capabilities

- `user-interface`: the parent skin's inset bevel.

## Impact

- `exa_ui::skin`, from `draw_inset_bevel`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`.
- Evidence: the 698-byte parent method's two arms and four helper calls. No new corpus files.
- Out of scope: the helpers' bodies.

## Phase

Phase 7, the interface. The arm selection, not the helper pixels.
