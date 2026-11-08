## 1. Trace

- [x] 1.1 The fast vector reciprocal, normalise and pitch-and-yaw direction in REA, with their assembly
- [x] 1.2 Their callers in the oriented-box slab test, the head tracking and the look direction

## 2. Port

- [x] 2.1 `engine_math::reciprocal_approx3`
- [x] 2.2 The slab test and the intersection module on the engine helpers
- [x] 2.3 The head tracking and the combatant's look direction on the engine helpers
- [x] 2.4 `EXA_SUN_FIT_TRACE` (debug) and `docs/DEBUGGING.md`; no corpus fields change, checks that need `EXANIMA_DIR` stay skipped
- [x] 2.5 Unit tests; crate README; `docs/PLAN.md` is the integrator's

## 3. Check

- [x] 3.1 `make check` for the crates touched (fmt, clippy -D warnings, tests) on the worker; play build
- [x] 3.2 GPU: lockstep traces against main, and the sun fit's trace in a terrain level
