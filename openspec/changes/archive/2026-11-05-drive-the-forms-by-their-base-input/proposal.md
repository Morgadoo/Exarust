# Proposal

## Why

The form base class's pointer rules (`press_plan`, the title and thumb drags,
the release marks, the arrows' repeat, the wheel, the hover change) were
ported as functions, but nothing in the running game called them. The
inventory, journal, loot window and container popup sat on a window manager
whose forms answered only their records' rectangles; a window could not be
moved, its close mark did nothing, and the manual's page scrolled by a
stand-in (a tenth of the page per wheel line) with no working scroll bar.
The Settings Controls page answered the wheel only. The skin's hover inks
on the close mark and the scroll bar's arrows and thumb were not drawn, no
default tooltip was ever shown, and round 58 left open how a title moves a
form's client top.

Phase: Phase 6 (story mode: the in-game screens) of docs/PLAN.md; no
PLAN.md edit in this change (the owner's).

Evidence (REA, Ghidra on `Exanima.exe` 0.9.5, checked against the
assembly): the base form's hit, hover, focus, press, drag, release,
repeat-step, wheel, client-measure, vertical-metrics and draw slots; the
line skin's hit slot, its scroll-rail slot and its form draw's hover inks
(the scroll bar's three parts and the late close mark); the window
procedure's wheel hand-off; the GUI timer (its table, the press's restart
and the fired timer's re-arm); the tooltip timer's text source per record
class; the table view's overrides and every writer of the vertical-bar
attribute. Capture: the retail inventory with Primary hovered (no tooltip)
from round 58's retail set.

## What Changes

- `exa_ui::base_form`: a live base form (`BaseForm`, a `FormHost`) running
  the base rules over the line skin's frame hit and scroll rail; the GUI
  timer's arrow repeat (first step on the press, the next seven ticks
  later, then one per tick, re-armed after the frame that fired it).
- `exa_ui::gui_core`: the task keeps a draw order and raises a pressed
  form by the layer rule; the wheel goes to the focused form; a form is
  handed the task's pointer and press point before a press, drag or release.
- `exa_ui::line_skin`: the client measure (`line_client`: a titled
  top-level form's client starts at two units plus the border metric 2);
  the hover inks of the scroll bar's arrows and thumb and the late hovered
  close mark (255, 204, 153 at the GUI alpha).
- `exa_ui::gui_controls`: a button's tooltip text is its record's second
  string (`hint`); a button asks for a tooltip only with one.
- `exa_game::form_input`: one frame's pointer, left button and wheel fed
  to a task of base forms, and the default tooltip's show, fade and draw.
- `exa_game::gui_screens`: the inventory, journal, loot window and popup
  are base forms on a rooted task driven by the pointer (title drag, close
  mark, hover inks on the frame and the records, draw order, tooltip); the
  inventory's view and records sit two pixels lower under its title.
- `exa_game::help_live`: the manual and its page are base forms (title
  drag, close mark, the page's scroll bar and the wheel, the text box's
  ease toward the fraction); the stand-in wheel is gone.
- `exa_game::settings_form`: the Controls page's scroll bar and the wheel
  run through its base form.
- `exarust`: `--wheel delta@frame,...` and `--debug-hint TEXT` (debug).

## Out of scope

- The hub's tables: the trace finds no writer of the vertical-bar or
  horizontal-bar attribute for them and the table view does not override
  the wheel, so the base form gives them no scroll; their rows move with
  the selection keys (not ported here).
- Grip resizing, maximise and hide marks (no game window here has them),
  the horizontal bar, snapping, the keyboard focus and Tab traversal on the
  live forms, the item views' and text boxes' own pointer slots (the
  manual's topic list keeps its click), the Settings form's own title drag.
- The record hint strings of the original's constructors (none known for
  the running screens); the click passing through a window to the world.
