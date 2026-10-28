# Design

## Normal order

The original builds one circular list per position element while walking
faces and corners forward, each new corner inserted after the head, so a
walk yields the corners in exact reverse order. The list is then cut into
runs in place: the first remaining corner opens a run and every later
corner with the same key is swapped into the run's next slot. Exarust
builds the same reversed lists as prefix-bounded arrays and runs the same
swap, keyed by the mesh's per-vertex smoothing key, so each key's sum is
accumulated in the original's float order. Face normals are cached per
face so the load pass does not grow.

## Merge float order

The original inverts the destination frame as a rigid transform (the
transposed turn and the turned, negated origin) and multiplies each source
row through it, the origin row adding the inverse's origin to its third
term before the final sum; points are placed as `(x·r0 + y·r1) + (z·r2 +
t)`. `merge.rs` keeps the round-19 render-form merge untouched and gives
the editable merge its own helpers for this order.

## NaN

The face tests' range rejections compile to `COMISS` followed by a parity
jump over the rejection, so NaN passes them; only the final distance
checks jump to the miss on parity. The Rust tests are written as the
positive rejection (`if x < 0.0 { miss }`) to match.
