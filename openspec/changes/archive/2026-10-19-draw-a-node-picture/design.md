# Design

## Icon

`icon_square` returns nothing when the sheet has no columns. The cell is the power's low byte minus one, split across the columns. The square is centred on the node. A second byte of 1 sets the bias to 1; any other uses the stored bias. The pad is the size times 0.05 plus 8.

## Frame

`frame_square` takes the one-based slot, walks a sheet of four across and two down, and draws four fifths of the size, computed in double precision. The two shrink factors use the size before that scale.

## Span and glow

`picture_span` drops the fraction of the view for a wide node and uses half the view for a narrow one. `glow_ring` places 32 spokes on a ring whose inner radius is 0.84 of the width and whose outer radius adds the outer size. The first spoke is straight up, and the list closes the circle.

## Open questions

- The fast sine polynomial is not reproduced. A spoke at step zero is straight up.
- Learning a node and the thick-line quads are not in.

## Docs

`docs/formats/OTHER.md` gains the picture rule. `docs/PLAN.md` and `docs/COMPONENT-MAP.md` leave the component at 15%.
