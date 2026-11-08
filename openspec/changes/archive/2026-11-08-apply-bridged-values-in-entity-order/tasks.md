## 1. Changes

- [x] 1.1 Bridged component values and removals applied in entity order on the simulation's side; test `the_frames_archetypes_do_not_order_the_simulations_queries` (fails without the sort)
- [x] 1.2 `EXA_ARCHETYPE_TRACE=1`; `context/tools/determinism_grid.sh`

## 2. Checks

- [x] 2.1 exanima01, four identical scenes in one grid, twice (700 frames) and once (200 frames): all traces byte-identical; per tick the same rows as main's two modes, in index order
- [x] 2.2 Lockstep vs main: arena duel identical
- [x] 2.3 Corpus check: not applicable
- [x] 2.4 exa_physics and exa_game READMEs, docs/DEBUGGING.md; docs/PLAN.md and docs/COMPONENT-MAP.md unchanged
- [x] 2.5 fmt, clippy -D warnings, tests of the touched crates on the worker
