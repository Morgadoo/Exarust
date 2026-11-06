## 1. Changes

- [x] 1.1 Free by default, `--sim-lockstep`
- [x] 1.2 Impact sounds: touches walked once a step (test `struck_by_body` against the per-body walk)
- [x] 1.3 Draw trace with body ticks

## 2. Checks

- [x] 2.1 Lockstep (`--fixed-frames`) vs main: arena duel, exanima01: identical
- [x] 2.2 Free default: pause, single steps and resume draw as lockstep; level change, death, exanima06 walk
- [x] 2.3 Corpus check: not applicable
- [x] 2.4 exa_physics README, docs/DEBUGGING.md; docs/PLAN.md and docs/COMPONENT-MAP.md unchanged
- [x] 2.5 fmt, clippy -D warnings, tests of the touched crates on the worker
