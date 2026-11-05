# Design

## Context

`exa_ui::form` holds the base form's rules as functions and
`exa_ui::gui_core::GuiTask` the window manager's (focus, hover, press,
capture, release, tooltip request and tick). The game's screens put plain
record forms on a task nobody drove. This change gives the task a form that
runs the base rules and drives it from the frame's pointer.

Formats page: none (GUI behaviour; `docs/UI.md`, The form base class).

## Decisions

- **One live form type.** `BaseForm` keeps what the base slots read: place,
  size, committed words, client, records (root coordinates, as the screens
  lay them out), the vertical scroll (fraction, step, content), hover,
  held code, grab, over flag and repeat time. Each `FormHost` method is one
  original slot: hit test, hover (the task's select), press, drag move,
  release, wheel.
- **The skin's hooks.** The frame hit and the rail are the line skin's
  slots (the base form calls them through its skin); the rail's start, end
  and two-unit thumb are what the hit, the thumb press and the thumb drag
  read.
- **Draw order.** The task's form list becomes a draw order; a press held by
  a form raises its top-level form by the layer rule. The screens draw in
  that order.
- **Timer.** The GUI timer is one tick of 100 clock units; the press
  re-arms it seven ticks ahead; a fired timer re-arms one tick after the
  clock that fired it. Exarust's clock is real milliseconds and the timer
  is checked once a frame.
- **Client top.** The client measure the show runs is ported for the line
  skin's lengths; only the inventory's layout takes the title's shift (its
  border is two units, so its view and records move down two pixels). The
  journal and the loot window keep their layouts (border 2; the shift would
  be eight pixels and their designs were not traced against it).

## Open questions and how they were answered

- Which slot is which: the slot offsets from the vtable's calls (hit at the
  drag's own call, set-scroll, draw, repeat, wheel), checked against each
  body's assembly.
- The thumb's track: the line skin's rail slot writes the start, end, thumb
  length and thumb place the base form's hit, press and drag read.
- The wheel's receiver: the window procedure hands it to the focused form,
  which the hover's focus follows.
- The tooltip's text: the tooltip timer takes a button's second string and
  every other record's text; the retail inventory shows no tooltip over its
  captioned buttons, so a button asks only when its second string is set.
- Hub table scrolling: no writer of the bar attributes for them (all six
  writers belong to the settings unit, the drop-down list, the creator and
  the game GUI unit), and the table view keeps the base wheel.
