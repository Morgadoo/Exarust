# Design

## Placement

`node_pixels` multiplies the node's place by ten and by the zoom, then adds the pan. Unless the view is dragged, each coordinate rounds to the nearest even integer. `place_node` then scales that point about the origin by the view factor.

## The line

`tree_link` takes two already placed ends. Both marks at zero return nothing. The link's first two words are turns, as fractions of a circle. Each end moves by `(sin, cos)` of the turn times a full circle, times the reach.

The reach is computed in double precision and stored as a float: 0.514, or 0.295 when the power's high mark is set, minus 0.003 times the zoom, times the zoom rounded to an even 256. A zero source mark sets the line dim.

The samples are the cubic from the first end to the second. The step count is the same rounded count the line uses, clamped from 2 to 30. The unit vector along the gap uses the approximate reciprocal square root.

## Open questions

- The fast sine polynomial is not reproduced. A turn of zero is straight up, which matches the sine and cosine.
- The thick-line quads, icons, frames and glows are not built.

## Docs

`docs/formats/OTHER.md` gains the on-screen rule. `docs/PLAN.md` and `docs/COMPONENT-MAP.md` record the line and leave the component at 12%.
