## Why

With long hair as cloth on (`EXA_HAIR_CLOTH=1`), exanima01 runs at 16 FPS on the RTX 3090: the cloth stage counts 3.8 ms a tick while the fixed schedule takes 12.5 ms a tick over four ticks a frame (the catch-up). The stage wrote every awake cloth's mesh every tick (a fresh position list and an asset change per cloth per tick, though the renderer takes one copy a frame) and stepped its cloths one after another on one core, and its counters could not say what of the fixed tick lay outside the stage.

## What Changes

- The cloth meshes take their particles once a frame, after the frame's fixed ticks, and only for the cloths that stepped since the last write.
- Each cloth's contacts and step run at once on the compute pool; the frame, owner, carry and object-list re-buckets stay sequential in the object list's order. The simulation's results are unchanged bit for bit.
- `EXA_DEBUG_CLOTH=1` logs the stage's split (parts bucketing, the sequential plan, the cloths' wall time with their CPU time gathering bodies, gathering the level and stepping), the frame's mesh write-back and the whole fixed tick's time.
- A synthetic CPU bench (15 hair-sized cloths among 336 body parts) and tests that each cloth steps as it would alone and that the meshes are written once a frame.

Phase 3 (bodies: skeleton, physics, control) in docs/PLAN.md, the cloth stage of the bodies' tick.

Evidence: no rule of the original changes; the bench's particle checksum is identical before and after.

## Out of scope

- Turning the hair cloth on by default: it stays opt-in until a GPU run shows the level near 60 FPS.
- Any change to the cloth simulation itself, and normals/tangents of the written meshes (not recomputed before or after).
- Costs outside the cloth stage (the new fixed-tick timer is there to find them).
