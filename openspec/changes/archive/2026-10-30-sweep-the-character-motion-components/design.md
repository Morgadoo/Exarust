# Design

## Matrices

The engine keeps a matrix as three axis rows and a translation (12 floats)
and applies it to row vectors: its product applies the left matrix first,
then the right one, each row summed `(x·b₀ + y·b₁) + z·b₂` and the
translation carried as a point. Exarust's `compose(a, b)` is that product
(`b * a` in Bevy's terms). The skin pass multiplies the inverse bind by the
joint placed in the mesh node's frame, which the original computes with its
placement helper: every joint row dotted with the node's rows, and the
joint's translation less the node's dotted with them (the transpose, a
rotation's inverse).

## Bounds and tint

The interpolation helper computes `a + (b − a)·t`, so the bounds' middle is
`min + (max − min)·½`; the length helper sums `(x² + z²) + y²`. The tint
compares the range with the squared distance and lets a NaN distance pass
(the compare jumps on parity into the tint).

## Part transfer

The role copy ends at a new part whose controller lacks the copy flag. A
source part without a controller does not end it: the new part's node word
is cleared and the source's children are still matched. The match ignores
the source child's controller; a claimed child takes the source child's id
and its new parent controller's link word, and its state from the
recursion. The id walk only descends below a part it numbered.

## Duplicates removed

`shock::recover` read the same routine as `force_release` (the stepping
byte as a grab, the clip rate as a slide word); `force_release` is the
traced, live one. `alarm_return` now offers bodies through `radius_query`.

## Open questions

- The node word the role copy clears and the controller link word are not
  traced to a reader.
- The tangent's w in the deform output is left as the buffer held it; the
  port keeps the source tangent's w.
