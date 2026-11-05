## Why

The owner asked why the frame rate is so low. On main (round 59), exanima01 runs at 18 FPS: the main thread spends 46 ms of a 57 ms frame in the fixed step, four 83 Hz ticks of about 11.5 ms each, while the GPU needs about 6.5 ms. A sampling profile of the running game showed that a quarter of the process's CPU went to six small systems that look each held item's holder up by walking every body of the level (4948 bodies in exanima01, nearly all sleeping props), once per item, every tick. The pair stage also built the full pose of every sleeping part every tick only to read its grid cell. The sewer (exanima05_sw), which runs at 1 to 4 FPS, spends almost all of its fixed tick in the character controller's sweep: the player's capsule follows its body every tick into dense prop meshes (about 4000 triangles within the capsule's box).

## What Changes

- The held-item systems (the weapon refresh, the grip modes and flips, the collapse release, the release of freed grips, the head height) and the limb-impact pass look holders up in one walk per system that ends once every key is found, with the same answer a per-item `find` gives (the first body in the query's order with that key). The weapon refresh buckets the held items by holder once instead of walking them once per body. The limb-impact pass keeps the part states only of the bodies a touch names.
- The pair stage tests a sleeping part's grid cell first (computed the same way as its pose's) and builds the rest of its pose only when the cell is near an awake part.
- The plain bodies' sound pass no longer keeps a set of every body it saw; a voice loop is freed when its body has left the query.
- Debug aids: a grid scene's leading `exe=<path>` runs that build and `env:<KEY>=<value>` sets the child's environment, so two builds or settings run side by side under the same load; `EXA_SWEEP_STATS=1` logs what the character sweeps meet; `EXA_FIXED_EXECUTOR=single` runs the fixed step's systems one at a time.
- No rule of the simulation changes; every result is the one the old code computes.

Phase 3 (bodies: skeleton, physics, control) in docs/PLAN.md: the fixed tick's cost.

Evidence: the original's step driver (Exanima.exe, read with REA) runs the simulation on its own time-critical thread, one step at a time, and drops its backlog only once it is more than four steps late; rendering is not on that thread. Exarust runs its fixed ticks on the main thread between frames (at most four a frame), so the tick's cost adds to the frame's.

## Out of scope

- The sewer's character sweep: a fix changes behaviour (the capsule's follow, or what the capsule collides with) and is left to a lane that ports the original's character movement.
- Running the fixed step on its own thread, as the original does.
- Incremental rebuilds of the sector body grid and the cloth stage's work.
