## 1. Changes

- [x] 1.1 Held answer and drain on entering a hold; `hold_and_drain` before a teardown, released when the next level is in
- [x] 1.2 Frame-side guard against stale step values; alarm reset posted
- [x] 1.3 Tests `a_teardown_holds_the_free_simulation_first`, `a_stale_value_never_despawns_the_frames_newer_entity`

## 2. Checks

- [x] 2.1 Free mode: hub match (twice), exanima01 -> exanima02, a save's load from the saves' screen: no panic
- [x] 2.2 Lockstep vs main: arena duel and exanima01 identical
- [x] 2.3 Corpus check: not applicable
- [x] 2.4 exa_physics README; docs/PLAN.md and docs/COMPONENT-MAP.md unchanged
- [x] 2.5 fmt, clippy -D warnings, tests of the touched crates on the worker
