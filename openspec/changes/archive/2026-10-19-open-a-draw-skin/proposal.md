# Proposal

## Why

The draw skin is the largest UI component still at 0%, and nothing stores its opening metrics.

## What Changes

- Opening a skin stores twice the GUI unit, the unit, and twice the unit twice more, then 8, 4, 8 and 8.
- The three colours are each 255, 250, 230, 100. The mark is 1.
- Argument 1 counts an allocation. A failed allocation returns nothing. A non-zero argument counts the call after the fill. Argument 0 does not.

## Capabilities

### New Capabilities

### Modified Capabilities

- `user-interface`: the draw skin's opening metrics.

## Impact

- `exa_ui::skin`, from `open_skin`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`.
- Evidence: the constructor's stores of the unit and the four colour bytes. No new corpus files.
- Out of scope: the base fill's body, the call after the fill, the slider, the frame, and the widget dispatcher.

## Phase

Phase 7, the interface. This constructor, not the widget dispatcher.
