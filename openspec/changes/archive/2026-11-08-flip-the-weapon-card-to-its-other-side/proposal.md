## Why

PLAN Phase 4 (items). The item GUI gap lane listed the item GUI unit's
routines against the workspace. The unit's sheet loader reads four sprite
sheets; the grip sheet was decoded by Exarust but drawn nowhere, and the
weapon card's button that shows it was taken for a "material swatch" and
never painted. Its handler, the side-row rewrite, was ported but called by
nothing, and the card refresh read striking side 0 because the card kept
no flip state.

## Evidence

REA (Exanima.exe), read in the assembly:

- The sheet loader makes the grip manager (cell eight units, the
  single-channel format) and stores the grip sheet's three cells from
  (0, 96), (96, 96) and (192, 96) at 96 pixels a side.
- The weapon card builder appends the template record after the striking
  rows when the weapon's flip byte is set: an image button at 0x90 on the
  anchor row -0x7ff9, style 0x80c00080, radio group 1, text "Flip weapon",
  its handler the unit's flip handler; the builder writes the grip
  manager's first cell and the manager into the record's image fields.
- The flip handler sets the card's side byte to `(state & 0x60) != 0` and
  runs the side-row rewrite: from the Slash row (control 0xd03) on, each
  non-zero byte of that side (the second side six bytes after the first)
  takes the next row's label and dots (byte × 0.05) and shows them; the
  rows with the ids after the last written one up to 0xd05 are hidden
  (state 0x110). A missing control resolves to the form's scratch record.
- The card refresh reads the striking bytes of the side the card's side
  byte names.
- The installed object database has twelve weapons with the flip byte
  (falchions, axes, pole weapons, hammers, picks); each has a second side.

## What Changes

- exa_items: `card::flip_rows` (the side-row rewrite on the built rows),
  `CardContent::{flip, hidden}` (the "material" gate renamed for what it
  is), `CardFacts::{flip, sides}`.
- exa_game: `item_card::{FlipButton, GripSheet}`; the card paints the flip
  button (line button frame, the grip icon through the skin's image draw
  in the state's ink), a left press and the release over it toggle it and
  turn the rows; hidden rows are laid out but not drawn; the refresh reads
  the side shown.

## Impact

Only a weapon card whose weapon has the flip byte changes. The arena duel
lockstep trace is unchanged against main; the exanima01 trace matches a
main trace in each of the run's two modes (main against main differs the
same way).
