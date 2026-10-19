# Proposal

## Why

The parent skin has a shade method that draws one inset rectangle in a place colour. Exarust models the line class's shade but not this parent one.

## What Changes

- The method prepares, sets mode 1, draws one rectangle inset by the place's stored inset, and sets mode 0x10.
- The colour is the place's four bytes with alpha forced to 255.
- At place 10, 20 of size 100 by 80 and inset 5 the rectangle is 15, 25 to 105, 95.

## Capabilities

### New Capabilities

### Modified Capabilities

- `user-interface`: the parent skin's inner shade.

## Impact

- `exa_ui::skin`, from `draw_skin_shade`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`.
- Evidence: the parent shade method and the rectangle helper. No new corpus files.
- Out of scope: the virtual prepare body.

## Phase

Phase 7, the interface. The rectangle, not its prepare.
