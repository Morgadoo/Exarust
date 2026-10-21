# Design

## Approach

The skin is modelled as plain functions over a `SkinPalette` and a
`SkinBatch`. The batch keeps the program, blend, ink and texture state
the original keeps in globals; each end appends a primitive naming its
slice of one shared vertex list, so a batch cleared and refilled every
frame does not allocate once warm. Integer places the original converts
to floats stay exact as `f32`; where the original computes in doubles
(the 10/11 step, the selection band and arrow centres, the fade clock)
the model does too.

Parts that call the skin's virtual slots (the frame slot, the box slot)
call the base skin's own implementation; the scroll bar's box slot is a
labelled guess (the base raised box).

## Answered questions

- Float order, rounding and saturation: read from the listings. Scaled
  channels round to nearest even and saturate; toned channels and alpha
  products keep the low byte of a 64-bit conversion; the luminance sums
  green and red before blue; the minimum returns its second operand on
  NaN.
- Which palette shades each part reads: from the palette offsets in the
  listings, checked with a test palette whose shades name their group and
  index.
- Which form word a bit lives in: the chrome, background and page read
  most bits from the style word and a few from the state word.

## Open questions

- The class check that moves the texture with a form, and the virtual
  slot the shell, panel and fill call first, are named but not resolved
  (REA: follow the class pointer and the slot's target).
- The GUI globals (unit, title, pad, close and rail sizes) may change with
  the GUI level; the skin reads their static values (REA: the writers of
  those globals).

## Docs

docs/UI.md, "Skin"; the `exa_ui` README. No docs/formats page changes.
