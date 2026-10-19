# Proposal

## Why

The parent skin turns a control code into the small rectangle that code refers to: the close square, the tab and the rail ends. Exarust does not model that switch.

## What Changes

- The unit-derived metrics are modelled: six units less the trim, four units plus one, that plus half the unit, and four units less one.
- Codes -4, -6 and -5 return the close band, shifted left by the tab metrics selected by flag bits 1 and 3.
- Codes -9, -10, -13 and -14 return the rail and tab squares around the place's stored fields.
- Any other code returns no rectangle and writes nothing.

## Capabilities

### New Capabilities

### Modified Capabilities

- `user-interface`: the control-code rectangle.

## Impact

- `exa_ui::skin`, from `control_rect`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`.
- Evidence: the 807-byte parent method's branch arms and the unit initialiser that writes the four metrics. No new corpus files.
- Out of scope: the hit test that returns the codes.

## Phase

Phase 7, the interface. The rectangle, not the hit test.
