# Design

Format pages: none (behaviour only). `docs/UI.md` (In-game GUI unit,
Character view) describes the screens.

## The game's form draw

The game's skin is the line skin, a subclass of the draw skin that puts
its own method in the form-draw slot. For a form it draws, in order: a
drop shadow (attribute 0x40: the rounded program's ring eight units out,
black at the form group's fourth shade's alpha); a backdrop (attribute
0x20000 on a form that is not held: one solid quad in the form group's
fourth shade); the form's content and children inside the client clip;
the title (attribute 1: the text centred in the form's width, its
baseline the main font's ascent, in the label group's seventh shade); then
the frame from the skin atlas in the idle control ink:

- style 0x10 (and not wide): six-unit corner ornaments (the top right one
  a second ornament beside the close mark when attribute 2 is set, the
  close mark four units square unless the pointer is on it), edge bars two
  units thick reaching round(6.6 units) in, the top one split around the
  title with a cap at each side of the gap ((width − title − 5 units) / 2),
  and under a title (without attribute 0x800000) the progress strip split
  at the gap;
- style 0x80 (and not wide, without a scroll bar): the panel frame. A
  free form's lies two units in: the progress strip on top (without a
  title), bars down the sides and along the bottom, two joint pieces per
  corner (the top pair only when neither the form nor its owner has a
  title, or the owner has attribute 0x800000). A held form's lies on its
  edges: an edge mask marks each side within a unit of the owner's client
  (top, left, right, bottom); the top bar is left off when the form is
  flush with a titled owner's top; a corner flush on both of its sides
  closes with joints, any other with the curled corner piece reaching two
  units outside.

## The commit of a form's words

A form's constructor leaves its state's restyle bit set; the show pass
(and adding a child to a pending owner) commits the words before the
first draw: a form whose owner is not the task is held (state bit 4); for
attribute 0x10000 a held form loses its backdrop and a top-level one gains
style 0x10 and attributes 0x41. The inventory (owner the task) thus gets
its shadow, title and corner frame; the character view and the pack (owner
the inventory) are held and draw no backdrop. `exa_ui::form::set_style`
already held the rule; the painter now applies it.

## The records

The inventory constructor passes its record table's last index, 3: four
records, the fourth the Secondary button (x following Primary on its row
by four, the same row, 108 wide, radio group 1). Primary is stored lit
(state 0x60).

## Open questions

- What the character view's kept still holds behind the figure is not
  traced; the view's camera clears to the canvas colour (now black at 170,
  the window's backdrop alone; the retail capture shows the scene through
  the view at about that strength).
- Hover inks (state 0x40) and the hovered close mark are not driven.
