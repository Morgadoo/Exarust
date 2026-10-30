# Design

The original's unit-change hook sets the icon size to forty times the
interface unit, resizes the icon renderers, rebuilds the atlas and renders
every cache entry again; for each entry that renders it takes a new cell
and sets the half extents to half the render's measured size (rounded to
even) plus the unit. Exarust renders icons asynchronously, so the rescale
queues every slot that still holds an id, gives each the provisional
extents (unit plus half the cell side) and lets the finished render set
the measured ones through the existing `apply_rendered`. Renders already
in flight at the old size no longer match the cell side and are dropped.
The system compares the scale with the cache's unit every frame and does
nothing when they agree, so it costs nothing in a steady state.

The take-in job tests the attached scene node's flags, not the item's
record flags. The job queue's runner removes a job only when it returns
false, so a missing mark keeps the job queued; `take_in_ready` returns
whether it can run. That the marks mean "loaded" is labelled as a guess.

The weapon attach's `0x100` object test marks every mesh child of the held
model and sets every part's cast-shadow byte, so the decision is
`casts_shadow`. The shield attach marks its part `0x800` (not modelled).

The protection figure computes `w·(√(c·0.01)·0.005)` in double, rounds it
to single, multiplies by the single-precision resistance sum and adds it;
coverage and weight share the bone index.
