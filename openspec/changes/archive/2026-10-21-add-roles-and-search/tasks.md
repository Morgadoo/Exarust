# Tasks

- [x] Parse `charroles.rdb` and find a level's locale, with synthetic tests and an installed-corpus test that skips without `EXANIMA_DIR`.
- [x] Roles, relations, the attitude, encounters and the entry finder, with tests pinned to the shipped table's matrix.
- [x] Apply roles: init assignments, mind-control follow, idle state, registry, the wait and the home-side point.
- [x] The search activity, its item weighing, comments, re-arming and tidying.
- [x] The AI manager's frame, zones, alert and approach clamp.
- [x] Wire roles, relations and the brain into the level's characters and the player.
- [x] Corpus: the shipped roles were parsed (49 roles, each ending at its size; 8 relation roles); the remote worker has no game data, so its corpus test skips.
- [x] Update `docs/formats/OTHER.md` and `crates/exa_ai/README.md`. `docs/PLAN.md` and `docs/COMPONENT-MAP.md` are left to the integrator.
- [x] `make check` equivalent on the remote worker: tests and clippy `-D warnings` for `exa_formats`, `exa_ai`, `exa_game` and `exarust`.
