## 1. Measure

- [x] 1.1 Synthetic bench: 15 hair-sized particle-mode cloths on 15 of 21 living bodies (336 parts) over a floor, with a particle checksum
- [x] 1.2 Bench before the change (`--profile play`): tick time and checksum

## 2. Fix

- [x] 2.1 Mark a stepped cloth's mesh stale; write stale meshes once a frame after the fixed ticks, in place
- [x] 2.2 Run each cloth's contacts and step at once; keep the object list's updates sequential in its order
- [x] 2.3 `EXA_DEBUG_CLOTH` split: parts, plan, cloths (bodies, level, step), write-back, whole fixed tick
- [x] 2.4 Tests: each cloth steps as it would alone; meshes written once a frame
- [x] 2.5 Bench after: same checksum, tick time

## 3. Docs and checks

- [x] 3.1 exa_world README, docs/DEBUGGING.md (the GPU check), the hair switch's doc; docs/PLAN.md and docs/formats/ unchanged here (the integrator notes the round in the plan; no format changes)
- [x] 3.2 Real-installation check: the hair tests that read EXANIMA_DIR (`a_long_style_hangs_as_pinned_cloth`) pass on the worker
- [x] 3.3 fmt, clippy -D warnings, tests (exa_world, exa_character)
