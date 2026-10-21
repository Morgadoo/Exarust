# Tasks

- [x] Rebase the sector lane onto main and keep the sector work at the sector level.
- [x] Check every sector rule against the assembly: constants, comparisons, rounding, conversions, loop order, NaN paths.
- [x] Correct the meet test, the grid arithmetic, the span margin, the overflow guards, the overlap tests, the sphere bounds and the weight fold.
- [x] Bucket parts by their mesh centre, let touching spheres through the first look, add the all-pairs box tests and refresh bounds after hits.
- [x] Give registered props their node's mesh bounds.
- [x] Keep the pair stage's lists between ticks and time it with the ignored probe.
- [x] Real-installation check: none reads the sector rules from data; the prop bounds come from the model loader's existing node bounds; skipped without `EXANIMA_DIR`.
- [x] Update `crates/exa_physics/README.md` and `docs/formats/RSQ.md`; `docs/PLAN.md` is updated by the main session.
- [x] `make check` equivalent: `cargo fmt --all`, clippy `-D warnings` and tests for `exa_physics`, `exa_world`, `exa_game` (and `exa_combat`).
- [x] `openspec validate --all --strict`
