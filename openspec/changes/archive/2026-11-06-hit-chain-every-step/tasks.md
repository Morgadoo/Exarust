## 1. Changes

- [x] 1.1 `striker_frame`, `Blade::frame`, tracking per tick with a one-tick time step
- [x] 1.2 Hit chain, brains' reaction and debug pauses in `BodyPostStep`; wound countdown in `BodyPreStep`
- [x] 1.3 `deal_shoves` first in each tick; `begin_frame` keeps the tick count reset
- [x] 1.4 The swing routine's pushes read the tracked frames

## 2. Checks

- [x] 2.1 Walk, duel and corpse drag: one world vs bridge identical; walk identical to the previous build
- [x] 2.2 Duel against the previous build: hits, damage and timing compared
- [x] 2.3 Corpus check: not applicable (no data format is read)
- [x] 2.4 exa_combat and exa_physics READMEs, docs/DEBUGGING.md drag scene re-aimed; docs/PLAN.md and docs/COMPONENT-MAP.md unchanged
- [x] 2.5 fmt, clippy -D warnings, tests of the touched crates on the worker
