## Why

PLAN Phase 4 (items and their windows). The extended controls' combo box had its draw
laid out but no form painted it: the hub's loadout form drew a panel and
a left-placed name instead. The item description sentences were live but
had only hand-made tests; nothing checked them over the shipped items.

## Evidence

REA (Exanima.exe 0.9.5), read against the assembly:
- The combo box's front draw: unless style `0x8000000`, the small atlas's
  fifth built-in glyph (the down chevron) `(4·unit|1) − unit/2 + 1` in from
  the right and `(unit+1) − (unit&1)` down; state bit 0x10 first (plain in
  slot `0xd4`), then the edit state `0x10000` (the atlas's outlined draw at
  alpha 1.0, whole pixels), else plain in slot `0xe0`. Then the main font
  and the text: in the edit state the label origin and the skin's edit
  draw; else slot `0xe4` and the draw-through (label origin over the span
  `w − (4·unit|1) − 2·pad − unit`, the edit unit's run draw with the 0x400
  selection and 0x10 whole-run flags and text ink 6).
- The label alignment measures the drawn run (bytes `start + 1` to `end`)
  with the current font; the 0x300 case centres by the font's ascent.
- The combo box's creation clears the anchor and caret and fits the run
  from the start into the span with the edit unit's run fit.
- The two sentence builders (apparel and weapon) and the phrase pick, read
  again: the weapon builder's condition table choice on a `c]` name reads a
  register it never sets; the card builder calls it with that register 0
  (flawless then perfect), the card refresh with a section pointer in it.

## What Changes

- `exa_ui::controls::combo_draw`: the 0x10 test before the edit state (an
  edit state with 0x10 draws the plain arrow); `COMBO_ARROW_GLYPH`,
  `run_width`, `combo_text_origin`.
- `exa_game::gui_paint::paint_combo_front`: the arrow from the live glyph
  atlas, the creation's run fit, the label origin and the run draw in the
  skin's text slots. The hub's table-built forms paint combo boxes with it
  (the loadout form's fighter list: no arrow, its name centred).
- `exa_items::desc_phrase_corpus`: every shipped item whose card builds a
  sentence (450) is described with the installed tables, printed and
  checked against the original's rules.

## Impact

- `crates/exa_ui/src/controls.rs`, `crates/exa_game/src/gui_paint.rs`,
  `crates/exa_game/src/hub_forms.rs`, `crates/exa_game/src/hub_windows.rs`,
  `crates/exa_items/src/desc_phrase_corpus.rs`, `docs/UI.md`, crate READMEs.
