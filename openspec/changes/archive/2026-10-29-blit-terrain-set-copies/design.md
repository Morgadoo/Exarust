# Design

Extends `docs/formats/TERRAIN.md` (set copy) and `docs/RENDERING.md`
(Terrain). Opt-in with the hosted sets (`EXA_TERRAIN_SETS=1`).

**Record.** `CompositeSelector::last_copies` lists the frame's `BlockCopy`
values; it is cleared at the start of each composite pass.

**Queue.** `TerrainCopyQueue` has the bake queue's ticket discipline
(extracted each frame, kept until the render world reports a ticket). The
render world prepares every copy whose images and pipelines are ready and
runs them in the prepass chained before `run_bakes`.

**Gate.** `composite_draw::copy_ready` needs both sets hosted, no bake in
flight and the block's drawn state equal to the source set with its current
colour target; otherwise the block falls through to `draw_steps`, which
rebakes the low set from the layers.

**NaN.** `stream::within_reach` keeps a NaN distance; `high_resolution`
sends it low. Both selection copies share the two tests.
