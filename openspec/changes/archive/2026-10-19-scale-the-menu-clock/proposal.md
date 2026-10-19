# Proposal

## Why

The main menu's update method copies a clock word, quantises it to 1/135 and scales it by 1/8 into field 0x158. Exarust does not model that conversion.

## What Changes

- A clock value is multiplied by 1/135, rounded to nearest even, converted back and multiplied by 1/8.
- The result is returned as a single-precision float.

## Capabilities

### New Capabilities

### Modified Capabilities

- `user-interface`: the main menu's clock scale.

## Impact

- `exa_ui::menu`, from `menu_clock_scale`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`.
- Evidence: the 72-byte main-menu method's two double constants and its store at 0x158. No new corpus files.
- Out of scope: the base update it calls first.

## Phase

Phase 7, the interface. The clock conversion, not the menu update.
