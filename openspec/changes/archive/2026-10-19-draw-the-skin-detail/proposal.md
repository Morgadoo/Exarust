# Proposal

## Why

The parent skin draws an inset rectangle and, for the horizontal and vertical style bits, adds one detail helper. Exarust does not model that dispatch.

## What Changes

- The method prepares, then draws one rectangle inset by the stored inset.
- Style bit 0x20000000 with flag bit 0x40 clear adds the horizontal helper at the original place with offset −unit/2 and mode 0x50.
- Style bit 0x40000000 with flag bit 0x40 set, or flag bit 0x80, adds the vertical helper at the inset rectangle.

## Capabilities

### New Capabilities

### Modified Capabilities

- `user-interface`: the parent skin's detail dispatch.

## Impact

- `exa_ui::skin`, from `draw_skin_detail`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`.
- Evidence: the 321-byte parent method and the two helper signatures. No new corpus files.
- Out of scope: the helpers' bodies.

## Phase

Phase 7, the interface. The dispatch, not the helper pixels.
