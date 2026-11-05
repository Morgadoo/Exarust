## 1. Changes

- [x] 1.1 `SimPreStep`, run before every fixed step from `FrameStartStep`, moved with the fixed step
- [x] 1.2 The rise, the restore and the alarm zones in `SimPreStep`

## 2. Checks

- [x] 2.1 Walk and duel: one world vs bridge, and against the previous build, identical
- [x] 2.2 The undead rise at the same time in both worlds; the corpse scene identical in four runs
- [x] 2.3 Corpus check: not applicable (no data format is read)
- [x] 2.4 exa_physics README; docs/PLAN.md and docs/COMPONENT-MAP.md unchanged
- [x] 2.5 fmt, clippy -D warnings (workspace), tests of the touched crates on the worker
