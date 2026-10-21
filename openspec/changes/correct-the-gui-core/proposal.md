# Proposal

## Why

A first pass extended the original's GUI core — the window manager, the
text support routines, the single-line editor, the 3D viewport control, the
main menu and the control records — and routed the running main menu
through the window manager. A review against the instructions found rules
that the pseudo-code had misled or that were invented: a "fallback focus"
that is the tooltip request, a focus that compared the wrong control, hover,
hint and key routing the original does not do, truncations where the
original rounds half to even, off-by-one walks in the ellipsis fit and the
caret, a width fit that measured the wrong span, wrapped text drawn upward,
a main-menu click handler that read one mode word for two and quit the game
from a story's EXIT, a practice page that greyed the wrong row, a layout
pass that re-placed earlier records, and viewport "key" routing that is the
view's pointer handlers.

## What Changes

- The window manager follows the original's task: the focus and its tooltip
  request, the tick that shows it after 400 clock units, the hide and fade,
  the modal form's hit and close, the pointer update with the drag threshold
  and capture, the press (double click within 450 units and one pixel), the
  press dispatch, the release, the drop hand-over, the forced reselect, the
  level switch's lengths (level 5 keeping unit 8's), the grips, the clamp and
  the toggle group's switch.
- The text routines: the ellipsis fit, the truncated line's advance, the
  tail's fit font, the empty line count, the width fit's span, unit step and
  count, wrapped glyphs hanging from the line top, and the menu font's
  rounding and texture name.
- The single-line editor: the caret walk's end, the window's end left to the
  scroll, the refused byte, the copy and cut of an unselected text, the
  paste's scroll, the draw's three paths, a zero tag alpha, the picture's
  line advance and the 16-byte texture key.
- The main menu: rounded page top and row step, the hit band's centring and
  miss rule, the paint's selection, press nudge, marks and unsigned shift,
  the click handler's commands by game mode and session state, the practice
  page's EXPERT gate, named constructor fields, the resume; the running
  screen selects under the pointer and runs a row on the release over it.
- The viewport: the jitter's projection scales, the still samples and the
  progressive ones, the stereo eye, the frame-rate counter and place, the
  pixel/space conversions' names and the cursor actions' press, release and
  drag.
- The control records: the append's truncation, the layout pass's two
  passes, back scan, class margin and trackbar initialisation, and the label
  class's draw.

## Capabilities

### New Capabilities

### Modified Capabilities

- `user-interface`: the window manager, text drawing and editing, the main
  menu, the viewport and the control-record rules above.

## Impact

- Phase 7 (game modes and content) of `docs/PLAN.md`: the main menu and the
  GUI forms the modes are built on.
- `exa_ui::{gui_core, gui_text, text_edit, viewport, gui_controls, menu}`,
  `exa_game::{menu_pages, menu_screen}`; `docs/UI.md`, `docs/RENDERING.md`
  and both crates' READMEs.
- Evidence: REA (Ghidra 12.1.4) on `Exanima.exe` 0.9.5 — the pseudo-code and
  the instruction listings of the six units, with the data tables read from
  the binary (the menu font record, the GUI lengths' initial values, the
  supersampling levels and the 123 jitter points, the class registry).

## Out of scope

- Drawing: the glyph quads, the edit field and the menu rows are not handed
  to the renderer here; the running menu still draws its rows in the base
  font.
- The forms' own classes (the original's form unit), which own the root's
  hit descent, a form's activation and the per-form translation; the
  session and form work behind the menu's commands; the tooltip form and its
  text; the gamepad layer's focus test; the system clipboard.
- The rescale of forms and records, already in.
