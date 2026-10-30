# Design

The torrent draw fills a sample array, then copies the last entry one past
the end (the count is not raised) before doubling. Exarust clones the
frame's samples and pushes the repeat, so `hermite_midpoint` sees the
repeat as the last real segment's `next` and the closing segment has no
`next` (its one-sided difference is zero). The original's quad emitter
drops the first call's two triangles and rewinds its triangle base to the
first pair; with Exarust's forward-joining `push_torrent_quad`, the same
strip is every call's triangles but the last call's, appended at the
mesh's own vertex base, so no stream joins another.

The end normals take `cross(s₁ − s₀, eye)` and
`cross(s_{N−1} − s_{N−3}, eye)` on the sample array, normalised, then
blended (0.1 and 0.03) without a second normalisation. The interior shares
are `i/(2N − 2)` in double.

MINSS/MAXSS answer their second operand on NaN;
`exa_physics::engine_math::{min, max}` already implement exactly that, so
the force code calls them.
