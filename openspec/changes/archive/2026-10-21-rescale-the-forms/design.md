# Design

## Behaviour

Lengths scale by integer division by the old unit, then times the new.
A border under 4 pixels stays; a larger one is its design value times a
quarter of the unit, rounded. Each axis: stretched forms keep both
edges' scaled gaps; otherwise the size scales as is (fixed-size forms)
or keeps its frame around a scaled client, and the form keeps its right
or bottom gap, stays centred, or scales its place. A title-only form is
its title and the skin's base. Children rescale inside the parent's old
and new client; records use 16-bit fields that wrap, and their vertical
place is corrected by half the height less four old units before and
four new units after (one unit when taller than eight).

## Open questions

- What runs in the form's after-rescale hook and in each control's
  resync. Answer: the classes that override them, by REA (the text box's
  margin hook is in).

## Docs

`docs/UI.md`, Forms.
