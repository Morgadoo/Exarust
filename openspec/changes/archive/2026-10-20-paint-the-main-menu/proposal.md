# Proposal

## Why

The main menu's paint pass is decoded: the first row starts at `trunc(H·origin) + line_height`, rows advance by `trunc(70·row_h)`, the selected row is skipped and drawn last, labels centre, styles bracket and disabled rows grey.

## What Changes

- `Painted` and `paint`.
- The selected row's last-position nudge `unit/3`.
- The disabled `(128,128,128,128)` and normal white colours.
- The `[t]`/`[[t]]` label styles.

## Capabilities

### New Capabilities

### Modified Capabilities

- `user-interface`: the main menu's paint pass.

## Impact

- `exa_game::menu_pages`, from `Painted` and `paint`.
- `docs/PLAN.md` and the generated `docs/COMPONENT-MAP.md`: `TMainMenu` becomes **44%** in the source rating. The headline rises slightly.
- Evidence: the menu's label layout (origin, advance, selected reserve, centring, styles, colours, nudge) and the font metrics for the line height. No new corpus files.
- Out of scope: the sprite/quad submission, the constructor and the global back router.

## Phase

Phase 7, UI. The main menu's paint.
