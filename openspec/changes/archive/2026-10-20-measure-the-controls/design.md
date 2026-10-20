# Design

## Behaviour

The three text spans are `w−2pad−2U`, `w−(4U|1)−2pad−U` and `w−(3U|1)−2pad−2U`; the `|1` keeps the odd-bit quirk (U=4 gives 13, not 14).

A trackbar maps a cursor x to `(cursor−x−pad−span/2)/(w−2pad−span)`, clamps to 0..1 (NaN passes), and quantises to `round(steps·value)/steps`. The thumb is at `x+pad+rint(travel·value)`; the caption centres in the knob slot. Lists use a `6U` row, a `3U` separator and a 1-based row index from `1+trunc((y−top)/(6U))`, clamped. Over 16 rows the height is `2·inset+16·row` and the page fraction `1/(count−16)`. The content starts at `2U`, adds separator/row heights, takes the widest text plus the `8U` shortcut inset and adds `2·inset+16U`. Radio items clear `0x100` on every radio item and set it on the chosen one; checkable items report the inverse of `0x100`.

## Model

`exa_ui::controls` exposes the pure functions and the `Track`, `ListMetrics`, `ListExtent`, `ItemMeasure`, `MenuItem` and `Activation` types.

## Open questions

The text field, drop-down events, spinner, colour picker, progress bar, image/texture views and the menu draw.

## Docs

`RF_GUI_Controls_Ex` is **12%**. The headline rises slightly.
