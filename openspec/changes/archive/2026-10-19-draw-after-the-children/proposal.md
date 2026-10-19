# Proposal

## Why

The dispatcher opens a form and visits its children, and the drawing that follows that walk is still missing.

## What Changes

- A positive title measure is centred in the form width, toward zero.
- Style bits equal to 0x10 draw four corners of six times the unit.
- Bit 2 adds a four-unit square unless the mark is -4. Bit 0x400000 counts a wide call and draws no corners.

## Capabilities

### New Capabilities

### Modified Capabilities

- `user-interface`: the title and corner squares that follow the child walk.

## Impact

- `exa_ui::skin`, from `dispatch_form`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`.
- Evidence: the centred title and the four corners. No new corpus files.
- Out of scope: the later edge bars.

## Phase

Phase 7, the interface. The title and the corners, not the later edge bars.
