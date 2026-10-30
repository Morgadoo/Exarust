# Design

## Skin slots instead of copies

The control dispatches call the draw skin through its class table. Each
slot the lane modelled by hand already had a port in `skin`: the panel
slot is `draw_panel`, the box slot `draw_control_box`, the image slot
`draw_skin_image`, and the selection band `selection_band`. The fixes call
those ports so the arithmetic stays in one place; `gui_controls` only
orders the calls and picks the inputs (the record's atlas cell, the label
shades, the GUI white alpha 255).

## The choice list's placement

The opener passes the list's corner and two alternatives to the general
form placement: the corner's x or the x plus the table's width, and the
y under the header row or a bottom one pixel below the corner. The model
returns that request rather than a size, so the caller can convert to the
screen and call `gui_core::place_form`, which already reproduces the
placement.

## Guesses kept

The destination backdrop's ink is not traced; it keeps the form group's
fourth shade rather than the skin's clean ink. The skin's first flag bit
is a caller boolean.
