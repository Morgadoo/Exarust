# Proposal

## Why

The main menu's pages and row geometry are modelled, but selecting a row, clamping the selection and turning an enabled row into an action were not. The original's key handler moves the selection within the row range and ignores disabled rows.

## What Changes

- A selection state per page, clamped to the row range.
- An action for every row kind.
- Activation returning `None` for a disabled row.
- The page transitions exposed as actions (Begin, Continue, NewGame, NewCareer, Practice, Versus, Arena, Manual, Settings, Exit, Back, Forfeit, Novice, Expert).

## Capabilities

### New Capabilities

### Modified Capabilities

- `user-interface`: the main menu's selection and actions.

## Impact

- `exa_game::menu_pages`, from `MenuAction`, `MenuState`, `RowKind::action` and the selection methods.
- `docs/PLAN.md` and the generated `docs/COMPONENT-MAP.md`: `TMainMenu` becomes **40%** in the source rating. The headline rises slightly.
- Evidence: the menu's key routing (the up/down clamp and activate branch), its hover and the page table. No new corpus files.
- Out of scope: the draw, the still-undecoded `row_h` scale rule and the settings/manual rows.

## Phase

Phase 7, UI. The main menu's selection.
