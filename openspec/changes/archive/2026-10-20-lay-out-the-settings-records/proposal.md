# Proposal

## Why

The Settings and Controls pages are authored as 0x68-byte control records with anchor codes, not as a fixed screenshot layout. The record decode now gives the exact classes, ids, positions, anchors, skin sizes and the 4-px y grid.

## What Changes

- The `Record` layout: class, id, x/y, w/h, text, style and state.
- The skin sizes for classes 1–5 and 0x13.
- The y-anchor deltas (0, 5…13 grid units) and x gaps (0…72 px).
- The `place` resolver: previous-row anchors, the first-row defaults, width/height skin defaults and 4-px rounding.
- The exact 9 tab/button records, 27 settings records and 22 binding rows.
- The 434×536 design form at unit 4, scaled by `unit/4`, with a 408-px settings list, a 416-px controls view and 676-px content.

## Capabilities

### New Capabilities

### Modified Capabilities

- `user-interface`: the Settings and Controls record layout.

## Impact

- `exa_ui::settings`, from `Record`, `skin`, `y_delta`, `x_gap`, `place`, `TABS`, `SETTINGS`, `Binding`, `BINDINGS` and `Layout`.
- `docs/PLAN.md` and the generated `docs/COMPONENT-MAP.md`: `TGUIForm` becomes **15%** in the source rating. The headline rises slightly.
- Evidence: the 0x68 records at `0x1002b2af0`, `0x1002b2ea0` and `0x1002b1700`; the loader `0x10002b200`; the anchor resolver `0x100033c90`; the skin registry and the 434×536 design. No new corpus files.
- Out of scope: the scroll-wheel handler, the tab page switch bodies and the settings apply/reset logic.

## Phase

Phase 7, UI. The Settings and Controls records.
