# Proposal

## Why

The centred icon is in, and the widget dispatcher still owes its opening: the three spans, the shade, the backdrop, and which children are shown.

## What Changes

- A dispatch stores twice the unit, three times the unit, and four times the unit.
- Bit 0x40 counts a shade. Bit 0x20000 with bit 4 clear draws a quad to the size.
- The scroll shift is the spare times the value, unless bit 0x800 clears it. Shown children are visited.

## Capabilities

### New Capabilities

### Modified Capabilities

- `user-interface`: the opening of the widget dispatcher.

## Impact

- `exa_ui::skin`, from `dispatch_form`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`.
- Evidence: the spans, the quad, the shift, and the child walk. No new corpus files.
- Out of scope: the rest of the dispatcher after the child walk.

## Phase

Phase 7, the interface. The dispatch opening, not the rest of that function.
