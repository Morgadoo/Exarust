# Proposal

## Why

Pressing I showed only the character figure, "Encumbrance:" and a
"Primary" button at the screen's left edge: no window frame, no title, no
panel frames, no Secondary button. The retail inventory (captured from the
user's own installation, an existing story save loaded without writing)
has a titled window with ornamented corners and a close mark, a thin-line
frame around the character view and the pack, and two page buttons. The
paint went through the default skin's background with the constructors'
raw words; the game's skin overrides the form draw, and the show commits
the words before the first draw.

The run logs' `GUI texture "Rayform10R" / "Rayform08R" not found` errors
are not the frame skins: they are the GUI level fonts, which the original
reads from resources embedded in its executable. The frame skin is the
`guiskin` atlas, which loads.

Phase: Phase 6 (story mode: the in-game screens) of docs/PLAN.md; no
PLAN.md edit in this change (the owner's).

Evidence (REA, Ghidra on `Exanima.exe` 0.9.5, checked against the
assembly): the line skin's form-draw override and the slot it fills in the
skin's method table, its shadow, backdrop, title, corner frame and panel
frame (free and held), the progress strip, the skin's sample-record table,
the palette offsets its inks read and its fade hook; the base form
constructor, the 3-D view's and the character view's constructors, the
inventory constructor and its record table (four records), the show pass
that commits a form's words and the commit rule (owner a task or not).
Capture: the retail inventory at 1600 × 1080 (windowed client 1592 × 1046)
and Exarust at the same client size.

## What Changes

- `exa_ui::line_skin::draw_line_form`: the line skin's form draw: the
  shadow and backdrop flags, the centred title's x, the corner frame of a
  top-level form (corners, close mark, edge bars with the title's gap, the
  title strip), and the panel frame of a free or held child (edge mask
  from the owner's client, joints or curled corners).
- `exa_ui::skin::sample_corners`: the line skin's whole sample-record
  table, so every sample's quad takes its record's eight coordinates (the
  fallback mirrored the first two corners, which gave the edge bars and
  caps a zero-width sample).
- `exa_game::gui_paint`: `commit_words` (the show's commit),
  `paint_line_form` (shadow, backdrop, title and frame layers),
  `LineFormAt`; the character view's attribute word corrected (0x70092).
- `exa_game::gui_screens`: the inventory paints through the line skin's
  form draw with the committed words (the window top-level, the view and
  the pack held); the record table gains the Secondary button; both page
  buttons are drawn with their own words (radio group 1, Primary lit).

## Out of scope

- The journal, loot window, container popup, item card and the hub's
  windows still paint through the default background; moving them to the
  line skin's form draw is a follow-up (the loot window would gain the same
  frame).
- The panel frame's scroll-bar branch and the wide frame stay unported
  (flagged as incomplete; no inventory form takes them).
- The Rayform fonts: reading the executable's embedded resources is a
  separate change.
- The page buttons' callback (switching weapon sets) and lighting the
  Secondary button from the record flag 0x1000000 (the fill's rule is
  ported in `exa_ui::game_gui`; the record word is not read here).
