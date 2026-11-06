## 1. Changes

- [x] 1.1 Simulation-only lent resources moved for good; alarm zones copied in, alarm state a snapshot; profiler tick counter shared; per-world `FixedCommands`
- [x] 1.2 `SimMode::Free`, `--sim-free`: the original's loop, `SimPause` hold from the screens, `FreeStamp`, `exchange_free`
- [x] 1.3 Render fraction as the frame's overstep; `bridge_copy_drawn` for drawn copies
- [x] 1.4 Tests: free steps on its own clock and sleeps while held; commands in post order; the render fraction

## 2. Checks

- [x] 2.1 Lockstep vs main e57173e5b: arena duel, exanima01, exanima02, exanima06 traces of every body identical
- [x] 2.2 Free mode: walk and duel without falls or teleports, the duel fought, hits land; story-level walks
- [x] 2.3 FPS free vs lockstep in exanima01, exanima06 and the arena in one grid
- [x] 2.4 Corpus check: not applicable (no format change)
- [x] 2.5 exa_physics README, docs/DEBUGGING.md; docs/PLAN.md and docs/COMPONENT-MAP.md unchanged
- [x] 2.6 fmt, clippy -D warnings, tests of the touched crates on the worker
