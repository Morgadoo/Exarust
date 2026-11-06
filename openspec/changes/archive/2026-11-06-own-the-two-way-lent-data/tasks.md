## 1. Changes

- [x] 1.1 `FrameCommands` (resource and component writes), `write_resource`; test `a_steps_write_to_frame_data_lands_after_the_step`
- [x] 1.2 Seven frame-owned types with queued step writes
- [x] 1.3 Four simulation-owned snapshots with posted frame writes
- [x] 1.4 Seven types moved to the simulation, frame writes posted or sent as messages

## 2. Checks

- [x] 2.1 Walk, duel, corpse drag, exanima01: one world vs bridge identical; walk, duel and exanima01 identical to round 82
- [x] 2.2 No wrong-side, skipped-system or unposted-write warning; copy cost unchanged
- [x] 2.3 Corpus check: not applicable
- [x] 2.4 exa_physics README; docs/PLAN.md and docs/COMPONENT-MAP.md unchanged
- [x] 2.5 fmt, clippy -D warnings, tests of the touched crates on the worker
