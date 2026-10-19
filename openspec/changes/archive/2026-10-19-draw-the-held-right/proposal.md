# Proposal

## Why

A held form still has no right strip after the right-edge quads when the tab bit is set.

## What Changes

- Bit 0x200 then draws one strip from 80, 30 to 90, 50 at unit 5, width 100 and height 80.
- A clear bit 0x200 draws none.
- A clear hold bit with that bit set draws the wider strip, from 70, 40 to 80, 60 at height 100.

## Capabilities

### New Capabilities

### Modified Capabilities

- `user-interface`: the right strip drawn when the tab bit is set.

## Impact

- `exa_ui::skin`, from `dispatch_form`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`.
- Evidence: the quad after the right-edge join. No new corpus files.
- Out of scope: the drawing after that strip.

## Phase

Phase 7, the interface. The right strip, not the drawing after it.
