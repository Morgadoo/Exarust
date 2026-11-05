## Context

The grid host (`exarust grid`) runs each scene as an offscreen child and forwards the host window's input to the active quadrant over its standard input.

## Decisions

- Drop forwarded input only for `--fixed-frames` runs, which are scripted and unattended; interactive grid play keeps the host's input.
- Investigated and ruled out as the first difference: steps 1–15 agree between runs; the colliders' digests differ from step 16 only in hash-map order.

## Risks / Trade-offs

- A `--fixed-frames` quadrant cannot be steered from the grid window.

## Open questions

- None for reverse engineering.
