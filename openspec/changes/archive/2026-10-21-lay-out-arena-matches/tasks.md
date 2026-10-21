# Tasks

- [x] Read the eleven setups, the ring placement, the placement templates and the next round; record them in `context/re/arena-layout.md`.
- [x] Add `exa_game::match_layout` (fixed places, ring, fray, the eleven layouts, the next round) with tests.
- [x] Set the running match's rounds by template; add `--arena-match <id>` to play mode.
- [x] No corpus field changes (skipped without `EXANIMA_DIR`).
- [x] Update `docs/formats/OTHER.md`, `docs/DEBUGGING.md` and the `exarust` README. `docs/PLAN.md` and `docs/COMPONENT-MAP.md` are the integrator's.
- [x] `make check` (remote: `cargo test -p exa_game -p exarust`, clippy `-D warnings`, `cargo fmt`).
