# Tasks

- [x] The hand's grab in the limp-body iteration (`exa_physics::body::Grab`, the pass counter) with the law's numbers pinned.
- [x] A solver test: a crate dragged 1 m across a floor.
- [x] `Physics::fixed_pick` and `StandIns::owner_of`.
- [x] `exa_game::pick`: the mobility field and the exact reach.
- [x] `exa_game::hand`: press, hold and reach systems; the pure rules pinned by tests; the cursor reads the hand; the movement request is kept while refused or lifting; the object hold reaches the steering.
- [x] Corpus check: none needed; the rules read no game file (the existing corpus tests are unchanged).
- [x] `docs/PLAYER.md`, `docs/UI.md` and the crate READMEs. `docs/PLAN.md` is updated by the integrator.
- [x] `make check` for `exa_physics` and `exa_game` (fmt, clippy -D warnings, tests) on the worker.
- [x] `openspec validate --all --strict`.
