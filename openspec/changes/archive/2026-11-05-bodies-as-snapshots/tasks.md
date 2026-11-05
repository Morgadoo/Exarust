## 1. Changes

- [x] 1.1 `FixedCommands::write`, `write_pair`, `write_many`; the writes travel in the step's packet and run first
- [x] 1.2 The hand's press, hold and reach, the GUI drop's lift, the use cursor's unlock posted
- [x] 1.3 The flow's body steps and the alarm return posted
- [x] 1.4 `bridge_snapshot`; bodies, doors, levers and plates as snapshots
- [x] 1.5 The hand's debug log

## 2. Checks

- [x] 2.1 Bridge test: posted writes and an unposted one, one world, inline and thread
- [x] 2.2 Scripted walk and duel, one world and bridged in one grid: within one world's start-alignment figure
- [x] 2.3 Scripted corpse drag, twice in each world: identical grab points and joints
- [x] 2.4 No unposted write warned in the walk, duel, drag, exanima01, exanima06; copy cost measured
- [x] 2.5 Corpus check: not applicable (no data format is read); the grid runs read the real installation
- [x] 2.6 exa_physics README, docs/DEBUGGING.md; docs/PLAN.md and docs/COMPONENT-MAP.md unchanged
- [x] 2.7 fmt, clippy -D warnings (workspace), tests of the touched crates on the worker
