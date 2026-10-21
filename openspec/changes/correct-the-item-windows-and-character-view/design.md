# Design

## Behaviour

- Grid: the aspect is a double quotient narrowed to single; columns are
  the half-to-even rounding of the single square root of `n · aspect`;
  rows the round-up of the narrowed `n / columns`. A slot's centre is
  `round((slot + ½) · (span ÷ div))` with the half added in single and the
  rest in double; the first slot goes to the last entry; each of the five
  relax passes ends with the clamp.
- Placement: the new entry's spot is a quarter plus a draw below half the
  width (and height) from the engine's shared generator; each pass pushes
  an axis only when its overlap is above the pass number, signed.
- Scatter and worn panel: both draw from the shared generator; each also
  stores a seed for the run-time library's generator, which they do not
  read. The worn push reads each icon's horizontal half extent for both
  axes (the original's loop indexes the extents without the axis).
- Codecs: saves divide in double, scale y by the single reciprocal of the
  vertical scale, convert half to even to a 64-bit integer and keep its
  low word; loads multiply in double (`unit · (v · 1/16)`, `unit · (v ·
  0.0125)`) and round half to even.
- Drops: linked first (its held entry shown again), then the panel's own
  dragged entry by icon; otherwise an append that adds bulk only from
  outside the linked panel; the clamp is the icon's half extents, right,
  left, top, bottom.
- Card request: with an item hit and no request pending, use types 0–1
  ask for a delayed card, 2 runs the use first, others build at once; the
  delay is two of the GUI timer's 100 ms steps of real time.
- Card text: the condition blocks are chosen first (the first block whose
  two-digit limit reaches the condition's percentage), then pronoun
  markers whose last field names a table entry.
- Character view: the camera is the figure plus `(eye − figure)` scaled to
  410 cm with the engine's vector length; the wash decodes the scheme from
  sRGB when sRGB targets are on; the task eases the race share by
  `frame_ms · 0.003` and takes the processor's minimum with the target.

## Model

Pure functions and the `Container` state in `exa_game::item_container`;
`exa_items::card::{pronoun, expand_pronouns, select_condition_text,
expand_card_text, weapon_handling, plate_armour_phrase}`;
`exa_items::item_gui::{Sockets::commit, side_rows}`;
`exa_items::possession::Possessions::give_to` with an optional panel;
`exa_ui::character_view` for the figure's rules.

## Open questions

- Which skill block the insight check reads: answered by its place in the
  record (the fifth 8-byte block, Insight) only as a guess; a record
  layout read through REA would settle it.
- The card's condition-block syntax: the instruction listing fixes the
  byte positions, not what the four bytes after the at sign spell; the
  shipped description tables (not loaded yet) would show it.
- The sRGB decode helper takes a vector by address; it is taken to decode
  each channel, as the scalar decode the creator's looks use.
- The formats pages are unchanged: no data format is touched.
