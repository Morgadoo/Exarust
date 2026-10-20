# Design

`CasterCells` maps each caster to its cell, `(⌊x/6⌋, ⌊z/6⌋)` of its sphere's
centre, or to the wide list when its radius exceeds 6 m, and each cell to
its casters. `relink` files the caster each time it computes its sphere
(a move to another cell takes it out of the old one). For the lights moved
this frame, the candidates are the casters filed within the light's radius
plus 6 m on each axis (a caster's centre is at most R + r ≤ R + 6 m away
when it can link), the wide casters, and the casters in the light's cube
list; casters moved this frame are relinked in full as before. The
candidates are gathered before any caster is refiled this frame.
