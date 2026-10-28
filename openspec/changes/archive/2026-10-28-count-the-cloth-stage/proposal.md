## Why

Round 26's GPU check of exanima01 (RTX 3090, `--no-gi`) found the cloth contacts still too costly. With the contacts on by default, the level ran at 14 FPS (72 ms frames, about 55 ms of cloth work) and loaded in 24.6 s. With them off it held 60 FPS. arenahub held 60 FPS either way. The fidelity fixes stay. The default goes back to off, and the stage gets counters so that the next check can show where the time goes.

## What Changes

- `ClothContacts` is off by default again (`EXA_CLOTH_CONTACTS=1` turns it on).
- `EXA_DEBUG_CLOTH=1` (`ClothDebug`) logs the cloth stage's per-tick work once a second, then one line per cloth.
- `Cloth::level_tests` counts the particle-triangle tests of the last level stage.

Phase 3 (bodies) in docs/PLAN.md.

## Out of scope

The cause of the remaining cost. The counters exist to find it.
