## Why

The hair cloth (change `hang-hair-as-cloth`) was live by default. A GPU run of exanima01 on the RTX 3090 with it on dropped to 16 FPS: the living characters' hair never sleeps (a hair cloth stays awake on its head's countdown, as the original's), and each awake hair was offered every body part of the level (about 108 000 a tick), so the cloth stage cost about 8 ms a tick.

## What Changes

- Long hair hangs as cloth only with `EXA_HAIR_CLOTH=1`, until the cloth stage culls the bodies it offers an owned cloth.

Phase 3 (bodies) in docs/PLAN.md.

## Evidence

`EXA_DEBUG_CLOTH=1 play exanima01 --perf-log 5`: awake 15, parts offered about 108 300, stage 8.0 ms a tick, 16 FPS; arenahub holds 60 FPS.

## Out of scope

The body cull for owned cloths.
