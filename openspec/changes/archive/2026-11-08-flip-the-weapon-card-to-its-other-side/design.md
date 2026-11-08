## Context

Exarust's card rebuilds its records from `BuiltCard` every frame, so the
flip handler's in-place record writes become writes into the built rows,
with a hidden mask standing for the records' 0x110 state.

## Decisions

- The rewrite is positional from the Slash row, as the original's pointer
  walk is: a side with more non-zero bytes than the built rows writes over
  the row after them (Thrust's). Writes past the card's rows are skipped
  (the original writes into the next records).
- A card without a Slash row is left alone: the original's lookup returns
  the scratch record then.
- The button keeps its own state word in the card; the toggle is the image
  button's release rule (`gui_controls::toggle`).
- The grip icon's ink follows the skin's image draw: the label group's
  seventh shade, its eleventh while on, its fourth while pressed or
  disabled.
