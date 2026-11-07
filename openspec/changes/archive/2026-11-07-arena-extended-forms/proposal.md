## Why

PLAN Phase 4 (the arena's interface). Round 114 censused which forms use the
extended GUI controls; three of them were not drawn. Tracing their record
arrays corrected the census: the `Look for:` and `Rank:` combo boxes belong
to the merchant's options in the trade window (not a hire form), and the
emblem form's colour swatches open the palette swatch popup (not the colour
picker, which only the dormant editor's lighting panel opens).

## Evidence

REA (Exanima.exe), read against the assembly:
- The trade window's opener adds a four-record form (0x104 × 0x44 at
  0x12c, 0x190, attributes less 0x20 and 0x20000) when it trades with the
  merchant. Its constructor writes the stock type's row (the table 0, 0x1000,
  0xc000) and the specialisation plus two into the combo records' selection
  words; the `Look for:` callback stores the table's word before the
  selection, the `Rank:` callback the selection less two.
- The emblem form's 23 records, its constructor (0x268 × 0x258, the four
  starting palette bytes, the arrows' icon set), the apply step (swatches
  from the lifted ramp with the index in the alpha byte, the mirror boxes),
  the refresh (the colours from the linear ramp), the background and design
  setters (the clamps, the step buttons' disabled bits, the number field),
  the step, number, mirror, swatch-owner, Randomise, Previous and name
  callbacks (the preset gives the colours' palette bytes 7, 0, 0, 6 and
  clears the layer word). The company dialog's BACK and NEXT.
- The colour swatch callback the emblem uses opens `TGUIColSwatch` with the
  swatch's index, placing the current cell under the pointer.
- `TColourPicker`'s 17 records and constructor (0x192 × 0x11a, "Colour",
  border 1), its gradients, and the record callbacks' events.

## What changes

- `exa_ui`: the merchant options, emblem form, company dialog and colour
  picker record arrays; combo item lists on records; the extended classes'
  release reports without touching on bits; a record form placed by its
  owner.
- `exa_game::option_forms`: the forms on the hub's form stack, the combo
  drop-down list, the palette swatch popup, text-field typing, spinner
  steps and the image's point; the extended classes' paint; the emblem
  preview over the form. The klean preset and the preview's palette are
  corrected.
- `--open-hub-window merchant|company|picker`.
