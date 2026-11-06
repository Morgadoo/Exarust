## 1. Changes

- [x] 1.1 Removal numbering both ways; test `a_removal_is_not_undone_by_a_value_in_flight` (fails without the guard)
- [x] 1.2 Untouched frame values not sent; end menu not a hold; `EXA_BRIDGE_WRITES`

## 2. Checks

- [x] 2.1 Free mode: the player killed in arena1 lies (as lockstep); an NPC's collapse lies; exanima01's corpses lie as in lockstep
- [x] 2.2 Lockstep vs main: arena duel identical
- [x] 2.3 Corpus check: not applicable
- [x] 2.4 exa_physics README, docs/DEBUGGING.md; docs/PLAN.md and docs/COMPONENT-MAP.md unchanged
- [x] 2.5 fmt, clippy -D warnings, tests of the touched crates on the worker
