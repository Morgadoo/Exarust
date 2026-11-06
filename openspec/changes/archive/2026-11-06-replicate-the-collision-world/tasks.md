## 1. Changes

- [x] 1.1 `FramePhysics`, `CollisionChanges`, `share_collision`, posted inserts, removals and clears; test `the_replica_picks_as_the_steps_world`
- [x] 1.2 Frame pick systems on the replica; step removals replayed on the frame's stand-in list and node map
- [x] 1.3 NavGrid moved, `NavGridView` for the overlay

## 2. Checks

- [x] 2.1 Walk, duel, drag: one world vs bridge identical, and identical to round 84; the hand's press and grab logs identical
- [x] 2.2 Replica cost in exanima01 and exanima06 (worst case and walk), under 1 ms
- [x] 2.3 Corpus check: not applicable
- [x] 2.4 exa_physics README; docs/PLAN.md and docs/COMPONENT-MAP.md unchanged
- [x] 2.5 fmt, clippy -D warnings, tests of the touched crates on the worker
