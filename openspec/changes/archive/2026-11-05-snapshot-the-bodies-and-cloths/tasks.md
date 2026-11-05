## 1. Changes

- [x] 1.1 Hard coverage error in debug and test builds, warning in play builds; a test
- [x] 1.2 `SimFrameStart`; the bodies' frame start moved there
- [x] 1.3 Cloth mesh snapshots; copy-in kind for `ClothInstance`

## 2. Checks

- [x] 2.1 `--sim-digest`: bridge and one world alike step by step (types that differ between two one-world runs set apart)
- [x] 2.2 Duel traces against one world and main (round 66); no unbridged type, no skipped system in the three levels; copy cost
- [x] 2.3 Corpus check: not applicable (no data format is read); the grid runs read the real installation
- [x] 2.4 exa_physics README, docs/DEBUGGING.md; docs/PLAN.md and docs/COMPONENT-MAP.md unchanged
- [x] 2.5 fmt, clippy -D warnings (workspace), tests of the touched crates on the worker
