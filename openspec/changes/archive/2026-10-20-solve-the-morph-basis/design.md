# Design

## Behaviour

The control record is 48 bytes: a normal vector at 0, padding to `0x10`, then three point pointers at `0x10`, `0x18` and `0x20`. The query moves to the first point (`q − p0`), its component along the normal is removed with the dot taken as `(x·n + z·n) + y·n` in single precision, and the first point is added back. That is the projection onto the triangle's plane.

With `p0`, `p1`, `p2` and the projected `s`, the differences are `x0 = p0.x − p2.x`, `x1 = p1.x − p2.x`, `x2 = p2.x − s.x` (and the same for y and z). When `|x0 + x1| < |y0 + y1|` the weights are

`u = ((x2 + z2)·y1 − y2·(x1 + z1)) / ((x1 + z1)·y0 − (x0 + z0)·y1)`
`v = ((x2 + z2)·y0 − y2·(x0 + z0)) / ((x0 + z0)·y1 − (x1 + z1)·y0)`

and otherwise

`u = ((y2 + z2)·x1 − x2·(y1 + z1)) / (x0·(y1 + z1) − (y0 + z0)·x1)`
`v = ((y2 + z2)·x0 − x2·(y0 + z0)) / (x1·(y0 + z0) − (y1 + z1)·x0)`.

The third weight is `1 − u − v`.

## Model

`morph_barycentric(&MorphControl { normal, points }, query)` returns `[u, v, w]`.

## Open questions

None; the two branches cover every case.

## Docs

`RF_Morphing` is **18%** (3,113 of 17,099 bytes). The headline stays 14.4% and 5.7%.
