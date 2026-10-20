# Design

## Behaviour

Rows advance by `trunc(70·row_h)`. The first row starts at `trunc(height·origin)`, where the origin is the page fraction (0.38 on the title). A point hits row `i` when it lies in `[top + i·advance, top + i·advance + band)`. `row_h` is supplied state, not derived: the live value at 1080 is 1.0 and the retail screenshot confirms 70 px rows. The rule that computes `row_h` from the resolution remains open.

## Model

`exa_game::menu_pages` adds `ROW_UNITS`, `row_advance`, `row_top` and `hit_row`.

## Open questions

The `row_h` scale rule, the draw, the constructor and the settings/manual rows.

## Docs

`TMainMenu` is **38%**. The headline rises slightly.
