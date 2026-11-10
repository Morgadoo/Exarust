## 1. Changes

- [x] 1.1 `resource_hold`: held resource entities on the frame's side (second update) and the simulation's (bridge start); tests `a_held_resource_does_not_move_later_ids` (fails without the hold), `a_held_entity_takes_the_value_and_keeps_it_through_a_removal`, `the_plugin_holds_on_the_second_update`
- [x] 1.2 Startup schedules single-threaded under `--fixed-frames`
- [x] 1.3 `EXA_ENTITY_TRACE=1`; `determinism_grid.sh` `GRID_ARGS` and missing-trace check

## 2. Checks

- [x] 2.1 Arena duel with presses, 3000 frames: four runs byte-identical (e502475c..., body ids included); r151 gave 60b8d40d and 525ad6ec in one grid, same values
- [ ] 2.1b exanima01 x4: not run, every attempt quit with a GPU Out of Memory (a vLLM engine held 18 GB of the 3090)
- [x] 2.2 Lockstep vs main: the duel's rows equal main's with the body column cut
- [x] 2.3 Corpus check: not applicable
- [x] 2.4 exa_physics, exa_game and exarust READMEs, docs/DEBUGGING.md; docs/PLAN.md and docs/COMPONENT-MAP.md unchanged
- [x] 2.5 fmt, clippy -D warnings, tests of exa_physics, exa_game and exarust on the worker
