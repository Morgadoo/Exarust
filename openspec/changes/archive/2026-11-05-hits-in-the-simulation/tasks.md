## 1. Changes

- [x] 1.1 The hit chain, `react_to_blows` and the debug pauses in `SimFrameStart`; their data bridged
- [x] 1.2 `SimFrameStartSystems`; `FrameStartStep` on the first fixed pass; frame count bridged
- [x] 1.3 Tests updated where they assumed `PostUpdate` timing (debug pauses, blood, the blow in a tickless frame)

## 2. Checks

- [x] 2.1 Scripted walk (Tab, D, A, W, S) exact against one world; duel within main's own run-to-run variation; digest without differences outside one world's own
- [x] 2.2 No unbridged type, skipped system or failed command; copy cost in exanima01, exanima06, arena
- [x] 2.3 Corpus check: not applicable (no data format is read); the grid runs read the real installation
- [x] 2.4 exa_physics README; docs/PLAN.md and docs/COMPONENT-MAP.md unchanged
- [x] 2.5 fmt, clippy -D warnings (workspace), tests of the touched crates on the worker
