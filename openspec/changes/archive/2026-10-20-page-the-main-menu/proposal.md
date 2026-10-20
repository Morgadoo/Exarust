# Proposal

## Why

The main menu's page table (`TMainMenu`, `0x100122c40`) defines every screen's rows, their order, which rows are disabled and the row-origin fraction. Exarust modelled the menu's gate but not the pages themselves.

## What Changes

- The title page's six rows, with CONTINUE disabled without a save.
- The start page's four rows, or five with VERSUS when the arena is available.
- The in-game page's first row being CONTINUE or NEW GAME.
- The arena and story pause pages, and the class page with NOVICE gated.
- The page's row-origin fractions (0.38, 0.45/0.42, 0.39, 0.45).
- The retail screenshot confirms the effective row spacing (~70 px at 1600×1080); the raw `70·0.5·round(H/135)` reading is left unresolved rather than guessed.

## Capabilities

### New Capabilities

### Modified Capabilities

- `user-interface`: the main menu's page table.

## Impact

- `exa_game::menu_pages`, from `Page`, `RowKind`, `Row`, `PageState`, `page_rows` and `page_origin`.
- `docs/PLAN.md` and the generated `docs/COMPONENT-MAP.md`: `TMainMenu` becomes **35%** in the source rating. The headline rises slightly.
- Evidence: `0x100122c40` (the page table), `0x100128580` (the in-game first row and quit command), `0x1001289a0`/`0x100128a70` (hover and key routing already modelled), and the retail screenshot at 1600×1080 (rows ~70 px apart). No new corpus files.
- Out of scope: the row draw, the unresolved spacing formula, text advances, focus traversal and the settings/manual rows.

## Phase

Phase 7, UI. The main menu's pages.
