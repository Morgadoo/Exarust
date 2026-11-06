## Why

Round 89's check held the free-by-default change: a hub-launched arena match crashed in free mode after the hub ->
arena level change. Step packets taken before the teardown arrived after it (the frame had held the simulation
without waiting, then applied the queued packets at the release), and one named an entity whose index the frame had
since given to a resource, which the frame despawned. The level start also reset the golem alarm by inserting the
simulation's resource on the frame (refused). docs/PLAN.md Phase 3, the simulation thread.

Evidence: REA (context/re/sim-thread-input.md): the original's level change and loading raise the pause counter;
the simulation thread checks it between steps, so no step runs into a dropped scene. The crash logs of round 89
(hub match, arena7).

## What Changes

- Entering a hold, the frame sends it with an answer asked for and waits: the simulation answers between steps,
  after every step taken before; the frame applies those steps at once.
- A level's teardown (every `LevelChangeRequest` teardown: story level changes, hub <-> match, a session leave, a
  save's load) holds the simulation and drains it first, and the hold lasts until the next level is in.
- The frame never despawns its own entity (a newer generation at the index, a resource) for a step's value.
- The level start's alarm reset is a posted write.

## Out of scope

- The bridge's copy cost in free mode (the frame's drawn transforms sent each frame).
