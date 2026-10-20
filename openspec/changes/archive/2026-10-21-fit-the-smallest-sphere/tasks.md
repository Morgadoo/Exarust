# Tasks

- [x] Read the sphere driver, its handlers, spheres and vector helpers in REA, with the assembly for the order of sums and NaN branches.
- [x] Add `exa_render::bound_sphere` with tests pinned to worked numbers (pairs, the three triangle branches, the flat triangle and tetrahedron, the two-point slot order, the tetrahedron winner, the slack, the empty set, an exhaustive cross-check).
- [x] Update `docs/RENDERING.md` (Sun shadow fit) and the `exa_render` README; `docs/PLAN.md` and the component map are the integrator's.
- [x] No corpus fields change; checks that need `EXANIMA_DIR` stay skipped.
- [x] `make check` for `exa_render` (fmt, clippy -D warnings, tests) on the worker.
