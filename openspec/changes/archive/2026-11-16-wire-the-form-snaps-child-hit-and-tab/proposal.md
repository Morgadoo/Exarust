## Why

Roadmap tasks B1 and B4 (lane `guiform2`): the base form (TGUIForm) still
had ported routines with no live caller or a missing branch: the snap
slot builder and the show's snap rebuild, the form hit's walk over child
forms, and the base form's key slot (Tab among the tab stops). The lane
also takes the census of the textured-skin forms' control records that
the line-skin lane left.

## Evidence

REA (Exanima.exe), checked against the assembly:

- The show rebuilds the snap slots of the parent (when it is not the
  desktop), of the form, and of each child that is a snap container
  (attribute 0x20) with children. The builder clears state bit 0x20000,
  starts the edges at the client's 0 and its width and height, widens each
  visible child with an attribute bit in 0x30 by its title spacing reserve
  (written by the client-area query from the skin's span when the style
  has a 0xc0 bit), and cuts the free cells into slots, the longer run of
  each free cell (the row on a tie). The rescale walk and the runtime
  resize rebuild a form whose slot count is above 0.
- The slots' only reader is the draw skin's form page (one quad per slot);
  the line skin overrides that slot.
- The form hit tests its children only for a point on its client, last
  first, each visible child without style bit 2 whose place is strictly
  below the point (x and y) and whose far edges are past it, through the
  child's hit slot, and returns the first nonzero answer.
- The base form's key slot: with a focused record, Tab moves to the next
  record with style bit 8 and without state bit 0x10, else wraps to the
  first with style bit 8 before it (Shift, flags 0x10, the other way) and
  focuses it through the focus routine; a Tab with nowhere to go ends.
- Census: the dialogue choices, divination card and item parchment
  constructors set no control records and no later call sets them; their
  draw slots never call the base form's draw. The skill selector builds
  eight power-slot rows but is never built.

## What Changes

- The window manager rebuilds the base forms' snap slots on show (game
  screens, the journal's text box, the manual, the settings form) and on
  the rescale walk.
- The window manager's hit walks a base form's children by the form hit's
  rule.
- The hub's forms take Tab through the base form's key slot.

## Impact

`exa_ui::{gui_core, base_form}`, `exa_game::{gui_screens, form_input,
option_forms}`. The hub's emblem form's Tab moves the focus between its
fields (was: nothing). A point on a child form's left or top edge goes to
the parent. No simulation change.
