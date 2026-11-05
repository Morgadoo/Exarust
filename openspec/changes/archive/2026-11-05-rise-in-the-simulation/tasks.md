## 1. Changes

- [x] 1.1 Rise and restore in `SimFrameStart`; `SimFrameStart` single-threaded; corpse records bridged
- [x] 1.2 REA: the manager frame is a hook the simulation thread's step calls
- [x] 1.3 Frame ambiguity report and single-threaded frame switch; the held-weapon case checked

## 2. Checks

- [x] 2.1 Scheduled rise test: one world, bridge inline and threaded rise alike
- [x] 2.2 Step digest and duel trace against one world; no unbridged type or skipped system
- [x] 2.3 Corpus check: not applicable (no data format is read); the grid runs read the real installation
- [x] 2.4 exa_physics README, docs/DEBUGGING.md; docs/PLAN.md and docs/COMPONENT-MAP.md unchanged
- [x] 2.5 fmt, clippy -D warnings (workspace), tests of the touched crates on the worker
