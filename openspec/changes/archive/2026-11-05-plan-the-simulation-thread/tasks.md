## 1. Evidence

- [x] 1.1 Read the original's physics thread body with REA; check the due test, the lateness compare and the wait against the assembly
- [x] 1.2 Read Bevy 0.20's schedule and access APIs (`systems_with_access`, `SystemAccess`, `try_iter_access`)

## 2. Driver and map

- [x] 2.1 `exa_physics::step_driver::StepDriver` with tests (on time, back to back, exactly four steps late, more than four, the wait cap, keep-backlog, the wrap, a pause)
- [x] 2.2 `--sim-boundary <file>` (`exarust::sim_boundary`) with a test on a two-schedule app
- [x] 2.3 Take the map on exanima01 through the grid and size the stages with it

## 3. Docs and checks

- [x] 3.1 exa_physics and exarust READMEs; docs/DEBUGGING.md; docs/PLAN.md and docs/COMPONENT-MAP.md unchanged
- [x] 3.2 Corpus check: not applicable (no data format is read); the grid run reads the real installation
- [x] 3.3 fmt, clippy -D warnings, tests of the touched crates on the worker
- [x] 3.4 Frame and tick rates through one grid: exanima01, exanima06, exanima05_sw, arena duel
