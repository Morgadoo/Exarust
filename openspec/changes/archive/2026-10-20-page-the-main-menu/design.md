# Design

## Behaviour

Each page is a fixed sequence of rows with a label, a kind and an enabled flag. The title page has BEGIN, CONTINUE, ARENA, MANUAL, SETTINGS and EXIT; CONTINUE is live only with a save. The start page has NEW CAREER, CONTINUE, PRACTICE, then VERSUS only when the arena is available, then BACK. The in-game page's first row is CONTINUE when a game is in progress and NEW GAME otherwise. Arena pause has CONTINUE, SETTINGS and FORFEIT; story pause CONTINUE, SETTINGS and EXIT; class select NOVICE (gated), EXPERT and BACK.

The row origin is a fraction of the client height: 0.38 on the title, 0.45 with four start rows and 0.42 with five, 0.39 on the pause pages and 0.45 on class select.

The retail screenshot shows about 70 px between rows at 1600×1080. The binary's `70·0.5·round(H/135)` cannot produce that with the client height, so the scale factor is left open and no row-spacing formula is shipped.

## Model

`exa_game::menu_pages` exposes `Page`, `RowKind`, `Row`, `PageState`, `page_rows` and `page_origin`.

## Open questions

The row spacing, text advances, focus traversal and the settings/manual rows.

## Docs

`TMainMenu` is **35%**. The headline rises slightly.
