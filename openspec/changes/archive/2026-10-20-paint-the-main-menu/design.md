# Design

## Behaviour

`y0 = trunc(height·origin) + line_height`; each row is at `y0 + index·trunc(70·row_h)`. The selected row is not drawn in the first pass; it is drawn last at its own y, with `unit/3` added when it is the final row. Row x is `(width − text_width) >> 1`. Disabled rows use `(128,128,128,128)`, live rows white. Styles 1 and 2 bracket the label.

## Model

`exa_game::menu_pages` adds `Painted` and `paint`.

## Open questions

The sprite submission, constructor and back router.

## Docs

`TMainMenu` is **44%**. The headline rises slightly.
