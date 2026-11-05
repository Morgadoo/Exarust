# Proposal

## Why

Rounds 56 and 57 moved the inventory, the journal, the loot window, the
container popup, the item card, the hub's record forms and the trade window
to the game skin's own form draw. Manage Company's two tables and the match
board's panes still painted the default skin's background, as did the
Settings form and its pages, the manual, the continue screen and its delete
box, the character creator and the arena's arsenal. The skin's panel frame
had no port of its scroll-bar branch, so a scrolled page (Settings'
Controls, the manual's page) could not be drawn.

The item card drew its name as a baseline one unit under the card's top
(so above the frame), its rows as text at made-up places (labels on top of
their values, values printed as numbers) and never used the title tint its
builder computes.

Phase: Phase 6 (story mode: the in-game screens) of docs/PLAN.md; no
PLAN.md edit in this change (the owner's).

Evidence (REA, Ghidra on `Exanima.exe` 0.9.5, checked against the
assembly): the line skin's form draw (its scroll-bar branch, the records it
samples), the base form, table view, item view and text box constructors;
Manage Company's and the match board's builders; the Settings, manual,
delete box and arsenal constructors; the card's constructor, its three
stats builders and its template arrays (records and label strings), the
card opener (title, text box child, icon area, height and place) and the
card's own draw (the icon place); the slider class's draw; the ink setter's
byte order. Capture: retail at 1600 × 1080 through a private headless
compositor from the existing "Tester" save (menu, Settings, Controls with
its scroll bar, manual, continue screen, inventory, plain cards).

## What Changes

- `exa_ui::line_skin`: the panel frame's scroll-bar branch (`LineFormScroll`:
  the two arrows, the thumb at the line skin's scroll refresh's place, the
  right bar moved in beside the bar, the rail outside a free right edge, the
  scroll bar's corner pieces).
- `exa_game::gui_paint`: the table view, item view, text box, manual,
  arsenal, column frame and embedded inventory words; plain child panes by
  their owners' style bits; `paint_held_scrolling`, `held_layers`, a title
  ink slot on `LineFormAt`.
- Manage Company's tables and the match board's tables, picture pane and
  reward panel as held forms (the season picture, a picture record, draws
  no frame); Settings (top form, the Settings page drawing nothing of its
  own, the Controls page with its scroll bar); the manual (top form, topic
  list, scrolled page); the continue screen and creator (the full-screen
  creation form draws nothing of its own, its children held); the delete
  box (style 0xb800, upper layer, border 0); the creator's message box; the
  arsenal (base form, its corner-framed column frame, item-panel columns,
  the embedded inventory).
- The item card: its records from the builders' templates (Weight slider,
  Impact and Balance dots, a label and dots per striking row, Flip weapon,
  Sockets, Rank, Points, Price; Coverage and the protection rows for
  apparel; Weight, Impact, Encumbrance for a shield) laid out by the
  two-pass layout and rescaled with the card; the name as the form's title
  in the builder's tint; the description in the small font in the text box
  child; the card's height, width and place as the opener computes them;
  the icon at the card's own draw's place. The weapon card's second row is
  "Impact", apparel's first "Coverage" (the templates' labels).

## Out of scope

- The wide frame branch and the frame's hover inks on the scroll arrows
  and thumb.
- The slider's knob icon (the rating sheet's whole dot stands in), the coin
  icons, the Flip weapon picture; the match board's prize and reward item
  slots and the loadout form's panels keep the default background.
- The title bar's effect on a form's client top (records and children are
  still placed from the border).
