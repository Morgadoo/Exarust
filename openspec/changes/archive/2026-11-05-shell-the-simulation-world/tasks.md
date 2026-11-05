## 1. Shell

- [x] 1.1 `SimShell` with inline and threaded runners; `SimStep`, `SimClock`
- [x] 1.2 `SimInbox` (post order, `spawn_linked`, `despawn_linked`), `SimOutbox` (send order)
- [x] 1.3 `SnapshotBuffer`, `Snapshots`, `SimSnapshot`
- [x] 1.4 `SimId`, `SimIds`, `SimLinks`
- [x] 1.5 `SimLockstep` before `FixedFirst`; `LockstepCheck` and its log; `--sim-inline`

## 2. Parity

- [x] 2.1 Unit tests: one step per pass in both modes; commands and events in order; linked objects in both worlds; the buffer; ids and links
- [x] 2.2 Body test: a shoved, muscle-driven body's every pose bit for bit with no shell, inline and threaded
- [x] 2.3 Grid: joint traces of the arena duel, the stage 1 build twice against this build threaded and inline; lockstep logs show 0 mismatches

## 3. Docs and checks

- [x] 3.1 exa_physics and exarust READMEs, docs/DEBUGGING.md; docs/PLAN.md and docs/COMPONENT-MAP.md unchanged
- [x] 3.2 Corpus check: not applicable (no data format is read); the grid runs read the real installation
- [x] 3.3 fmt, clippy -D warnings, tests of the touched crates on the worker
- [x] 3.4 Frame and tick rates against the stage 1 build in one grid: exanima01, exanima06, exanima05_sw, arena duel
