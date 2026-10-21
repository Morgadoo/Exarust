# Tasks

- [x] Game flow, game events and splash in `exa_game::flow`, `game_event`, `splash`.
- [x] Review against the assembly: shot offsets added to the point, the story offset (600, 600, 1300), the near shot for every caller of the group, the respawn portal, the light angles and backgrounds, the engine's angle rows, the exit-trigger reach, box and row rules, the practice death guard and heights, the level-exit save and checkpoint rule, the quit autosave, the mode switch's hook and reset, the revival's save, the third awakening's mind domain, the click pitch, the splash's tint, clocks, fades and warm-up counters.
- [x] Synthetic tests with hand-worked numbers.
- [x] Real-data check: `the_installed_logos_are_readable` (`EXANIMA_DIR`), and a GPU run of `exarust --splash`.
- [x] `docs/UI.md` and the `exa_game` README. `docs/PLAN.md` is updated by the integrator.
- [x] fmt, clippy -D warnings and tests for `exa_game`, `exa_ui`, `exa_formats` (and clippy for `exarust`) on the worker.
- [x] `openspec validate --all --strict`.
