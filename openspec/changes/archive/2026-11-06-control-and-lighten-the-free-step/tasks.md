## 1. Changes

- [x] 1.1 `--fixed-frames` keeps lockstep
- [x] 1.2 Free mode's step and speed keys; stamp with clock and rate; test `a_held_free_simulation_takes_single_steps_and_runs_at_its_speed`
- [x] 1.3 Single-threaded step executor; body grid kept for unchanged bodies; `EXA_SIM_TIMING`

## 2. Checks

- [x] 2.1 Lockstep traces vs main (arena duel, exanima01, exanima06): identical
- [x] 2.2 Step time and steps a second free and lockstep in one grid
- [x] 2.3 Corpus check: not applicable
- [x] 2.4 exa_physics README, docs/DEBUGGING.md; docs/PLAN.md and docs/COMPONENT-MAP.md unchanged
- [x] 2.5 fmt, clippy -D warnings, tests of the touched crates on the worker
