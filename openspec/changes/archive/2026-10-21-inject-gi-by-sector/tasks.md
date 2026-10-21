# Tasks

- [x] Rebase the first port onto main.
- [x] Check the marks, grouping, placement, shaping test, injection order and rounding against the pseudo-code and the assembly (sum order, unordered compares, sign-bit masks, the round and sign helpers, 32/64-bit arithmetic).
- [x] Fix the second pass (reach, built cell, ± grid-height stride), the box test (radius, signed half extents, centimetres), the focus cell and the integer grid halves.
- [x] Fix the grouping (set equality, fill-estimate merge gate), the shaping test (order, NaN, threshold) and the level without a grid (one group of 64 sectors from slice 0).
- [x] Keep the pass's uniform buffer across frames and drop per-sector allocations.
- [x] Tests: hand-worked marks, strides, box cuts, focus cells, odd-grid placement, groups and their costs, shaping cases.
- [x] Real-installation check: `cargo test -p exa_world -- --ignored` builds a shipped level in steps on the worker (`EXANIMA_DIR` set there, skipped without it); no corpus command reads the GI marks.
- [x] Update `docs/RENDERING.md`, `docs/PERFORMANCE.md` and the crate READMEs; `docs/PLAN.md` is updated by the main session.
- [x] `make check` equivalent on the worker: `cargo fmt --all`, clippy `-D warnings` and tests for `exa_render` and `exa_world`.
- [x] `openspec validate --all --strict`
