## Why

TMotileDynamics gap (round 86): the AI's route walking (the roam's and
patrol's legs, the flight, the combat chase, the follower's walk) still slid
the character at a capsule velocity in a walk or run clip; the AI's queued
step was bent one tick late and the player's not at all.

## Evidence

REA (Exanima.exe):
- The AI's locomotion driver (TMotileAI, called by every walking state)
  writes the body's locomotion fields, the ones the player's exploration
  steering writes: the movement goal (the path follower's 100 cm carrot,
  then 150 cm out from the standing point when the body is aligned), the
  heading (eased 0.1 toward the goal's direction, made a unit with the
  approximate reciprocal square root), the flags (move; run at urgency ≥ 1
  on cheap ground with the alignment over 0.9; the sprint latch at urgency
  2), the moving byte, the look (eased 0.1 toward the heading) and the
  cursors; a stepping body is busy. The body tick's movement decision and
  locomotion routine walk it. Idle states set the slow-turn word to 1, the
  combat state to 0; the chase's urgency is 1, the flight's 2.
- The soft stop clears the run bit, sets the stopping bit and pulls a far
  goal to 8·speed + 20 cm along the frame's forward.
- The queued-step taker (TMotileDynamics #17) bends the queued vector at the
  body target's circle when the vector points along the body's look
  direction, inside the balance pass; only the line walk and the snap need
  the pathfinder.
- Out of the scene the path follower slides the standing point 2 cm a tick
  along the path; the driver only sets the moving byte. Re-entry places the
  body at that point. TMotileDynamics #35 is not this: the body tick calls it
  for a statue body (slow-turn word bits 2|4) whose scene node is in no
  sector, to flag the node tree's objects.

## What Changes

- exa_ai `drive`: the driver (`drive`, `soft_stop`, `face`, `grid_cost`),
  `Driven`, `add_locomotion` (a fighter with muscles gets the locomotion
  request at its first brain tick), `face_standing` (a standing character's
  request and body yaw take the AI's facing; a walk clip the body no longer
  walks ends in the stance over 20 frames, fast ease).
- exa_ai `brain::idle_walk`, `flee_walk`, the duellist's chase and exa_game
  `npc_search::follow_and_search` drive the request for such a body.
- exa_physics: `approach_bend` moves here from exa_game; `queued_step` takes
  the look and the body's target (`Ragdoll::aim_steps`, `look_steps`) and
  bends; the movement stage sets the look, the player's footwork and the
  duellist the target.

## Out of scope

- The out-of-scene slide and the re-entry placement it needs.
- The driver's obstacle check (another character ahead on the line).
