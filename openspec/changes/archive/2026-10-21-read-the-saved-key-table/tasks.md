# Tasks

- [x] `exa_game::bindings`: the reader and writer of the saved table, the row order, the conversion and its inverse, pinned by tests (shipped rows to shipped slots, version shapes, short and long counts, row 21, truncation, round trips).
- [x] The sided modifier codes read their own key.
- [x] Corpus check: none needed; the payload is the player's own settings store, not game data.
- [x] `docs/PLAYER.md` (Bindings). `docs/PLAN.md` is updated by the integrator.
- [x] `make check` for the workspace (fmt, clippy -D warnings, tests) on the worker.
- [x] `openspec validate --all --strict`.
