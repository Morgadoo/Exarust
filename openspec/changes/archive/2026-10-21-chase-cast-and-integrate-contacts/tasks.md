# Tasks

- [x] Rebase the lane onto main and use `exa_world::nav::find_path` as the chase's router
- [x] Check the cast chooser, the chase route, the plan turn, the run stop, the can-act gate, the contact integration, the target pass, the nearby insert, the greeting and the morale against the instructions; fix the differences
- [x] Wire the chase, the run stop, the can-act gate and the swing release into the duellist; unschedule the brain's pass
- [x] Update `crates/exa_ai/README.md` and `docs/NAVIGATION.md`
- [x] Verify in the running game: `exarust --opponent --press W@40-120 --fixed-frames --frames 1800 play arena` fights as on main, with `--perf-log` frame times unchanged (the real installation; skipped without EXANIMA_DIR)
- [x] docs/PLAN.md is updated by the integrating session (the review returns its note)
- [x] fmt, clippy -D warnings and tests for exa_ai, exa_world and exa_game
