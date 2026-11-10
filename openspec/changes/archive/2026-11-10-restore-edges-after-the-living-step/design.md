## Context

Exarust steps a whole body at once (`Ragdoll::step`: every part's
inertia, the anchors, the pose drive), where the original steps each part
of the tree in its sector list's order.

## Decisions

- The root's edge pass is placed at the end of `Ragdoll::step`, after the
  pose drive, because the root steps last in the original's list. Each
  part's pass only moves its own particles, so the tree order inside the
  pass does not matter.
- The approximations use the running processor's RSQRTSS and RCPSS (the
  same helpers as the AI driver and the engine's vector reciprocal), exact
  arithmetic off x86-64.
- `Drive::muscle_gains` returns D and k apart, so the damping term is
  formed as the original forms it.

## Open questions

- A sector migration appends a tree in node order again, so the root
  stays last unless the tree is split over sectors (not traced).
