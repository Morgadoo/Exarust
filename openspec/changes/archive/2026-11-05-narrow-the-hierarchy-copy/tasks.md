## 1. Changes

- [x] 1.1 `SimShared` marking and the narrowed hierarchy gathers; `EXA_BRIDGE_WIDE`
- [x] 1.2 `Children` orders move only the named children
- [x] 1.3 The frame cloth's particles refreshed from the mesh updates behind the bridge

## 2. Checks

- [x] 2.1 Bridge test: a frame-only child and a lone entity stay home, a body's parent crosses and follows
- [x] 2.2 Cloth test: the bridged frame cloth takes the newest places without a change mark
- [x] 2.3 Walk and duel in the arena, walks in exanima01 and exanima06: one world vs bridge identical for the player
- [x] 2.4 Copy cost narrowed vs wide in exanima01, exanima06 and the arena
- [x] 2.5 Corpus check: not applicable (no data format is read); the grid runs read the real installation
- [x] 2.6 exa_physics README, docs/DEBUGGING.md; docs/PLAN.md and docs/COMPONENT-MAP.md unchanged
- [x] 2.7 fmt, clippy -D warnings (workspace), tests of the touched crates on the worker
