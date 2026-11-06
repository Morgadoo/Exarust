## Why

The collision world (Physics, StandIns, ColliderNodes) and the navigation grid were the last lent types both sides write. The frame's picks query the collision world every frame; a whole copy each step would cost several ms (exanima01 has 23,185 static colliders).

## What Changes

- `exa_physics::collision_share`: the frame's picks query `FramePhysics`, a replica with the same handles. The frame's inserts, removals and clears apply to it at once and are posted to the simulation; the step's removals are sent back; after a step that moved or changed anything the frame takes the moved colliders' poses and a copy of the broad-phase tree (`share_collision`). `Physics` and `CollisionChanges` move to the simulation.
- Stand-ins move only when their pose changes (no redundant marks).
- StandIns and ColliderNodes are the frame's (copied in); the step's takes are replayed there.
- NavGrid moves to the simulation; the debug overlay draws `NavGridView`, sent only while the overlay is on (`NavOverlay` copied in); a level's grid removal reaches the simulation before the frame's other changes (`FixedCommands::post_first_bridged`).

Phase 3 (bodies: skeleton, physics, control) in docs/PLAN.md: the simulation thread.

Rule change: none (picks see the last finished step, as before).

## Out of scope

- The lent types only the simulation touches (moved with the free loop), the free loop.
