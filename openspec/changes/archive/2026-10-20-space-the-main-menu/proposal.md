# Proposal

## Why

The main menu's page table is modelled, but its row geometry was not. The live menu's `row_h` is 1.0 at 1600×1080 and the retail screenshot shows ~70 px rows; Paint's advance is `trunc(70·row_h)`.

## What Changes

- The row advance `trunc(70·row_h)`.
- The row top `trunc(height·origin) + index·advance`.
- The hit row using the original's `top <= y < top+band`.
- The `row_h` value stays supplied state: the scale rule that produces it is still not decoded, so no guessed resolution formula is shipped.

## Capabilities

### New Capabilities

### Modified Capabilities

- `user-interface`: the main menu's row geometry.

## Impact

- `exa_game::menu_pages`, from `ROW_UNITS`, `row_advance`, `row_top` and `hit_row`.
- `docs/PLAN.md` and the generated `docs/COMPONENT-MAP.md`: `TMainMenu` becomes **38%** in the source rating. The headline rises slightly.
- Evidence: the menu's paint (its 70·row_h advance and `trunc(H·fraction)` origin), the hit band, the live menu scale of 1.0 at 1080, and the retail 1600×1080 screenshot (~70 px rows). No new corpus files.
- Out of scope: the still-undecoded `row_h` scale rule, the draw, the constructor and the settings/manual rows.

## Phase

Phase 7, UI. The main menu's row geometry.
