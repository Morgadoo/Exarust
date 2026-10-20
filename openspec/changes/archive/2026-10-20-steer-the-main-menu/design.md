# Design

## Behaviour

A page's selection is a 0-based row index. Moving by a delta clamps to `0..rows−1`, so the ends do not wrap. Activating the selected row returns its action only when the row is enabled; a disabled CONTINUE or NOVICE returns `None`. Every row kind maps to one action, including the in-game first row's CONTINUE/NEW GAME and the pause pages' FORFEIT.

## Model

`exa_game::menu_pages` adds `MenuAction`, `MenuState` and `RowKind::action`.

## Open questions

The draw, the `row_h` scale rule and the settings/manual rows.

## Docs

`TMainMenu` is **40%**. The headline rises slightly.
