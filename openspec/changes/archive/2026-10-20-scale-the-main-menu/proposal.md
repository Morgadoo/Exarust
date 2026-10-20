# Proposal

## Why

The main menu's row scale was the last open geometry question. A live read at a 632×453 client gives `row_h = 0.375`, and at 1080 it is 1.0; both follow `0.5·round(H/135)/4`.

## What Changes

- `row_h(height) = 0.5·round(H/135)/4`.
- The row advance remains `trunc(70·row_h)`.
- The close: 1080 → 1.0 and 70 px rows; 453 → 0.375 and 26 px rows; 720 → 0.625; 1440 → 1.375.

## Capabilities

### New Capabilities

### Modified Capabilities

- `user-interface`: the main menu's row scale.

## Impact

- `exa_game::menu_pages`, from `row_h`.
- `docs/PLAN.md` and the generated `docs/COMPONENT-MAP.md`: `TMainMenu` becomes **42%** in the source rating. The headline rises slightly.
- Evidence: the live client `[0x10049a930]+0x2c` and menu `[0x1004f0170]+0x158` at 453×632 (0.375) and 1080 (1.0); the MeasureItems formula `0x100128450`. No new corpus files.
- Out of scope: the draw and the settings/manual rows.

## Phase

Phase 7, UI. The main menu's row scale.
