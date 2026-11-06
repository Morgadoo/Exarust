## Why

Stage 6 runs the simulation apart from the frame. 87 types are lent to the simulation for each lockstep step; 52 of them are also touched by frame systems, which a free-running simulation cannot lend back and forth. Each needs an owner the simulation and the frame can keep apart.

## What Changes

- Two one-way resource kinds in `exa_physics::sim_bridge`: `bridge_copy_in_resource` (the frame owns it; copied in on change, never back; a simulation write warned of once) and `bridge_snapshot_resource` (the simulation owns it; copied out on change, in only on insert; a frame write warned of once and not sent).
- The 24 lent resources written on one side only (boundary map, `context/artifacts/2026-11-06-simfree/lentmap.txt`) take them: 20 copied in, 4 snapshots.
- The search reads `OpenSearchScreens` (loot windows' characters, popup ids), kept by the frame before its fixed steps, instead of the whole `GameScreens`, which no longer crosses.

Phase 3 (bodies: skeleton, physics, control) in docs/PLAN.md: the simulation thread.

No rule change: lockstep results are unchanged.

## Out of scope

- The 35 lent types written on both sides, the sim-only lent types (move them when the free loop lands), the free loop itself.
