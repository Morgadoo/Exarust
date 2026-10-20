# Proposal

## Why

The original's forms are built from a handful of widgets that Exarust
did not have: the multi-line text box of the manual pages, character
creation and item cards, the icon atlases every list and button draws
from, the twelve built-in glyphs, the table view, the item view and the
basic controls (button, check box, trackbar, label, icon and picture
buttons). They all lay text out with the interface's own bitmap fonts,
whose per-byte widths the original carries in its executable; Exarust
only measured `fontbase24r`'s inked columns.

## What Changes

- The six GUI fonts with their recorded advances, glyph quads, slants
  and line heights; the six GUI scale levels with their units, fonts and
  icon cells.
- Text layout: line breaks at carriage returns, after the last space that
  fits or after an overflowing byte; the inline markup (colour, italics,
  centring, pictures, `{%` spans, key names); caret moves, the pointer's
  caret and the caret's quad.
- The text box: its line count, snapped and eased scroll, and every key
  and pointer edit of the original, quirks included.
- The icon atlas: packing, growth and repacking, uploads, and the plain,
  turned, mirrored, outlined and glowing draws; the twelve built-in glyphs
  plotted at the GUI unit; colour icons hit by their alpha.
- The table view, the item view and the basic controls: layout, hits,
  presses, keys and their draw positions.
- In the app: a resource that follows the GUI scale with the level, the
  atlases and the glyph texture.

## Capabilities

### New Capabilities

### Modified Capabilities

- `user-interface`: GUI fonts and levels, text layout, the text box, icon
  atlases and glyphs, the table view, the item view and the basic
  controls.

## Impact

- `exa_ui`: new modules `gui_font`, `text_layout`, `text_box`,
  `icon_atlas`, `gui_glyphs`, `table_view`, `item_view`, `gui_controls`
  and `gui_level` (registered by `UiPlugin`); `scale::follow_window` is
  crate-visible for ordering.
- `docs/UI.md` (Fonts, Text boxes, Icons, Tables, Item views, Controls)
  and the `exa_ui` README.
- Evidence: reverse engineering of `Exanima.exe` 0.9.5 with REA (Ghidra):
  the text box's class and its line, caret, hit, key and pointer methods;
  the text layout class's draw; the GUI font records and their start-up
  gap; the scale-level routine and the atlas set-up; the icon atlas's
  class (add, grow, upload, the eight draws and the turn table); the glyph
  unit's twelve plotters and its build order; the table view's and item
  view's classes; the control classes' registry and message procedures.
  Addresses are in `context/re/gui-widgets.md`. No corpus files.
- Out of scope: the GL draw bodies and shaders the draw lists feed; the
  single-line text field the table and item view edit with (its own
  unit); the item container and item cards (another lane); the
  `GUISymbl` pack image (Exarust draws the cross the original falls back
  to); the image library's Kaiser resize and sharpen of large icons (the
  atlas takes icons at its cell size); wiring text boxes, tables and item
  views into forms, since no Exarust form uses them yet.

## Phase

Phase 7, the interface: the original's widgets.
