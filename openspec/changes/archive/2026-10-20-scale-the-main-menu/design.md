# Design

## Behaviour

`row_h` is `0.5·round(H/135)/4`, with round-to-nearest-even. Paint advances by `trunc(70·row_h)`. The live reads are 0.375 at 453 px and 1.0 at 1080 px, matching the formula; 720 gives 0.625 (43-px rows) and 1440 gives 1.375 (96-px rows).

## Model

`exa_game::menu_pages` adds `row_h`.

## Open questions

The draw and the settings/manual rows.

## Docs

`TMainMenu` is **42%**. The headline rises slightly.
