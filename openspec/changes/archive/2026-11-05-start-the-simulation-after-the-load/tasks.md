## 1. Trace

- [x] 1.1 REA: the original's load and the simulation (pause counter, kd-tree builder)
- [x] 1.2 Find what differs between two scripted runs (spawn tick, collider order)

## 2. Port

- [x] 2.1 No fixed step behind the loading screen
- [x] 2.2 Static colliders in hand-out order, a triangle budget per frame
- [x] 2.3 Test: loads of different lengths tick alike

## 3. Check

- [x] 3.1 No corpus format is read (no `cargo exa` check applies); one GPU grid: exanima01 walk ×3, exanima06 script ×2, arena duel ×2, load times old vs new
- [x] 3.2 Update docs/UI.md and docs/DEBUGGING.md (PLAN.md is the integrator's)
- [x] 3.3 fmt, clippy -D warnings and tests on the worker; play build
