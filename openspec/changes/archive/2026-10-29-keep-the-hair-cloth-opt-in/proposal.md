## Why

Change `cull-the-bodies-a-cloth-is-offered` reproduced the original's nine-cell body offer and turned the hair cloth back on by default. The GPU check on the RTX 3090 (`EXA_DEBUG_CLOTH=1 play exanima01 --perf-log 5`) showed the cull working (parts offered about 1 600 a tick instead of about 108 300, the cloth stage 3.8 ms a tick instead of 8) but the level still at 16 FPS: the fixed schedule took 12.5 ms a tick over four ticks a frame, so part of the hair's cost lies outside the counted stage.

## What Changes

- The hair cloth stays opt-in (`EXA_HAIR_CLOTH=1`) until the rest of its cost is found.

Phase 3 (bodies) in docs/PLAN.md.

## Out of scope

Finding the uncounted cost (candidates: the mesh write-back, the carry into the head's frame, the particle-mode passes).
