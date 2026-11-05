## 1. Registration

- [x] 1.1 Every plugin with fixed-step systems registers its data; coverage empty in the game (264 types)
- [x] 1.2 Move and children-order kinds; lent resources swap with a spare
- [x] 1.3 Run conditions in the coverage; unbridged components warned; per-type cost log; `EXA_BRIDGE_ALL`

## 2. Checks

- [x] 2.1 No system skipped with `--sim-bridge` in the arena, exanima01 and exanima06 (grid)
- [x] 2.2 Copy cost per step measured in exanima01, exanima06 and the arena
- [x] 2.3 Arena duel joint traces with `--sim-bridge` against main (grid)
- [ ] 2.4 The bridged duel tick for tick with the one-world duel

## 3. Docs and checks

- [x] 3.1 exa_physics and exarust READMEs, docs/DEBUGGING.md; docs/PLAN.md and docs/COMPONENT-MAP.md unchanged
- [x] 3.2 Corpus check: not applicable (no data format is read); the grid runs read the real installation
- [x] 3.3 fmt, clippy -D warnings (workspace), tests of the touched crates on the worker
