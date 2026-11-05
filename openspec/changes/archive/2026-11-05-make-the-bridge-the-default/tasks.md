## 1. Harness and fix

- [x] 1.1 `--sim-digest`, `EXA_DIGEST_DUMP`, `--hold-loading`
- [x] 1.2 Find the drift (unbridged query filters: the player's steering matched nothing); coverage and boundary map read filters; markers registered
- [x] 1.3 Visibility components on the simulation's entities explained (required-component defaults)

## 2. Cost and default

- [x] 2.1 Changed values through storage change ticks; cloths lent
- [x] 2.2 The bridge by default; `--no-sim-bridge`

## 3. Checks

- [x] 3.1 Duel traces: bridge vs this build's one world within one world's run-to-run noise; against main (grid)
- [x] 3.2 No unbridged type, no skipped system in the arena, exanima01, exanima06; copy cost per step (grid)
- [x] 3.3 Corpus check: not applicable (no data format is read); the grid runs read the real installation
- [x] 3.4 READMEs, docs/DEBUGGING.md; docs/PLAN.md and docs/COMPONENT-MAP.md unchanged
- [x] 3.5 fmt, clippy -D warnings (workspace), tests of the touched crates on the worker
