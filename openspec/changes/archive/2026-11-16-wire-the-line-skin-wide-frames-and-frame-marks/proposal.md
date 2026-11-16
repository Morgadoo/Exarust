## Why

Roadmap tasks B1 and B4 (lane `linesrest`): the line skin (TGSLines) and
the draw skin (TGUIDrawSkin) still had ported routines with no live caller
or with a missing branch: the line form draw's wide arms and the wide
window frame, the default background's frame marks and their icons, the
draw skin's hit test, the line skin's wide title band (F24), the span bar's
cap inset (F29), the line shade and the draw skin's control fill, and the
skill screen's own copy of the rounded fill.

## Evidence

REA (Exanima.exe), checked against the assembly where order, constants or
branches matter:

- The drop-down list's constructor writes the attribute word 0x4a1470 and
  style 0x98; the palette swatch popup's 0x421060 and style bit 0x10. Both
  have the wide bit 0x40_0000, and both openers copy the opener's form
  skin into the popup. The combo opener, for a skin whose mark word has
  bit 1 (the line skin's opening sets it to 1), clears the list's style
  bit 0x80 and lifts it by a unit less one.
- The line form draw: the corner style with the wide bit calls the wide
  window frame and binds the idle ink again; the panel style with the wide
  bit draws twelve quads straddling the edges (side lengths 0 and two
  units) in the idle or hot ink by state bit 0x40, before any scroll test.
- The wide window frame reads the form's state word (bit 4 held, 0x40
  hot) and the owner's attribute word (bit 1 titled).
- The line skin's hit reads a byte that starts at 1, is copied from a
  never-written byte that starts at 1 or set to 1 by the hardware settings
  and the Steam input, and is cleared only around the form's record pass,
  after the form's hit has called the skin's.
- The form's hit calls the skin's hit slot only off the client. The draw
  skin's hit probes each mark in the rectangle its form-rect slot answers.
- The colour swatch's draw message calls the current skin's control fill:
  the line skin's shade (six atlas quads, the on ink or the highlight ink
  under the pointer, the colour a unit in at the GUI's alpha), the draw
  skin's style frames and the colour inset by the margin.
- With the line skin a table view draws one span bar at the row height
  less half a unit and one, across the client, instead of its header boxes;
  the span bar's cap inset is the skin's first side length.
- The default background's mark icons come from the small icon atlas:
  active marks through its outlined draw at scale 1, others plain after
  binding the label group's seventh shade.

## What Changes

- The combo lists and the palette swatch popup draw through the line form
  draw (shadow, backdrop, wide window frame) instead of the default
  background.
- The colour picker keeps its frame-mark words; its close mark lights,
  draws pressed, closes the picker, and its icon draws.
- The base form's line hit takes the six-unit title band.
- The emblem swatches draw the line shade, the picker's swatches the draw
  skin's control fill.
- The board's and Manage Company's tables draw the header line.
- The skill tooltips' boxes go through the skin's rounded fill.

## Impact

`exa_ui::{line_skin, skin, record_form, table_view, base_form}`,
`exa_game::{option_forms, hub_forms, base_form, gui_paint, hub_windows,
skill_screen}`. Pictures change in the hub (combo lists, palette popup,
emblem swatches, table header lines) and on the colour picker (close mark
icon, its hover and press boxes, swatch backs). No simulation change.
