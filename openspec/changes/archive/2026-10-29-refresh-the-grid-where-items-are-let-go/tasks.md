## 1. Refresh

- [x] 1.1 Sample the bodies bucketed in the rooms a grown rectangle spans; a node box's rectangle under its frame; an idle test
- [x] 1.2 Keep the queues with the grid; ring a moved node's box; refresh once per fixed tick after the bodies step, nothing when idle
- [x] 1.3 Unit tests: a box rectangle follows its frame; a body two rooms away raises nothing; a moved leaf frees its doorway; a let-go refreshes the floor the item left

## 2. Play

- [x] 2.1 The item cursor's let-go queues the held node's box at the frame the lift saved
- [x] 2.2 `--nav-refresh-moved`: a moving mesh come to rest somewhere new queues both places (unit test)
- [x] 2.3 Corpus (`EXANIMA_DIR`): `shipped_levels` unchanged (the build is untouched)

## 3. Docs and checks

- [x] 3.1 Update docs/NAVIGATION.md, docs/DEBUGGING.md and crates/exa_world/README.md (docs/PLAN.md is updated by the integrator)
- [x] 3.2 `make check` equivalent: fmt, clippy -D warnings, exa_world, exa_game and exarust tests
