## Why

The cloth contacts were opt-in since round 24 because exanima01 stalled with them on. Round 28 found the last cause (the placed banners' saved positions were read as node-local, heaping them with misplaced wall rings, and they started awake instead of asleep) and ported the original's face-flag pins. A GPU run on the RTX 3090 with the contacts on now shows all 14 cloths asleep from the load, the cloth stage at about 1.6 ms a tick and 59 FPS.

## What Changes

- The cloth contacts are on by default; `EXA_CLOTH_CONTACTS=0` turns them off.

Phase 3 (bodies) in docs/PLAN.md.

## Evidence

`EXA_CLOTH_CONTACTS=1 EXA_DEBUG_CLOTH=1 play exanima01 --perf-log 5`: awake 0, asleep 14, stage 1.57–1.63 ms a tick, 59 FPS.

## Out of scope

The node transform records' level frame (still a known divergence).
