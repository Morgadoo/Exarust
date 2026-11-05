# Proposal

## Why

Backlog item C13. Exarust's default spawn put each living body's pelvis
where the first clip's root puts it over the model root and never ran the
placement's leg snap; the play flag `--original-spawn` ran the snap at
spawn, for every living body, twice (once at the clip height, once at the
snapped height), and left the controller where it was. Neither is the
original's order.

Evidence (REA, Ghidra on `Exanima.exe`, checked against the assembly):

- The living builder samples the first clip with the pose-only sampler
  (which writes its pelvis target), then puts the root node (the pelvis)
  at the placement's height plus that target less 8 cm, at the
  placement's yaw, with the character frame at the placement plus the
  target and the character's ground at the placement's height. It runs no
  leg snap. The condition word is still 0 at that sample, so the target
  is the not-walking margin: the stand height plus twice the scale plus
  5 cm.
- The player behaviour's tick counts its first ticks; on the third,
  unless the condition has the off-feet bit, it moves the character
  frame's origin in place to the navigation grid's nearest walkable point
  (the origin at the pelvis's height, the answer on the cell's floor) and
  runs the placement on the ground with that frame and the Height Query.
  The placement on the ground runs the leg snap first (with the body as
  it stands), writes the snap's frame height into the frame, copies the
  frame into the character's, samples the current clip with the pose-only
  sampler, puts every part at the pose lowered by the pelvis's posed
  height onto the frame with zero velocity, re-anchors each foot at its
  posed contact on the frame less the clip root at its ground plus the
  sole, restarts the slack links' smoothed lengths and takes the frame's
  height from the placed pelvis. A new player body taken by a mind swap
  starts the count past three (no snap); the arena loadout restarts it.
  The level's NPC spawn and the AI behaviour never run it (their ground
  placement comes only from teleports, summons, saves and the AI's push
  out of a blocked region).

The earlier "4.5 cm lower" with the flag came from Exarust's own order
(the second snap's leg IK run at the lowered pelvis), not from the
original's write-back: with the original's order the player stands where
it stood without the snap (exanima01 92.4 against 93.5 cm over the
capsule after 2 000 ticks, arena and exanima06 95.3 against 95.3).

Phase 3, physical body (TMotileDynamics). No docs/PLAN.md edit in this
change (the main session owns it).

## What Changes

- physical-body: new requirements "Spawn placement at a matrix" and
  "Placement on the ground"; "Placement leg snap" reads the character
  frame for its IK.
- player-movement: "Spawn snap" moves the frame's origin (at the
  pelvis's height) and asks for the placement on the ground; it counts
  only once the body exists.
- Code: `Ragdoll::placement_target` and `Ragdoll::place_on_ground`
  (`exa_physics`), `placement::SPAWN_PELVIS_DROP`; `limp::spawn_pose`
  places every living body at a matrix; `limp::GroundPlacement` and
  `limp::place_on_ground` run the player's placement on the ground;
  `Steering::snap` returns the walkable point; `--original-spawn` and
  `limp::OriginalSpawn` are gone; `--spawn-watch <ticks>` lengthens the
  spawn's pelvis log, which now also gives the height over the
  character's ground.
