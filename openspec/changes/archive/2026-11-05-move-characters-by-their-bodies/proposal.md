# Proposal

## Why

Phase 3 (M3 "standing man"), the character movement. Exarust moved every
character's place with a rapier kinematic character controller: a 28 cm
capsule swept each fixed step toward the body's frame (a tenth of the way, at
most 2 cm), with autostep, slope limits, ground snap and gravity. The
original has none of it. Round 62's physics profile found that, in the
sewer (exanima05_sw), almost the whole fixed tick (56 to 100 ms) was that
controller computing its ground contact against dense prop meshes (17 to 20
colliders and about 4 000 triangles in the capsule's box), and the capsule
never closed its gap to the body.

Evidence (REA, Ghidra on `Exanima.exe`, checked against the assembly):

- The body tick builds the character frame as a pure yaw at the character's
  position, whose height it then sets to the pelvis's. No sweep, no
  controller and no collision query stands between the body and that
  position.
- The character position has a small set of writers: the anchor routine
  (the frame follows the pelvis by its rules: onto the pelvis while
  moving, past 5 cm under a support of 0.95, within 5 cm down, within
  10 cm early in a get-up), the dead falling body (onto the pelvis), the
  get-up start, the matrix and ground placements, the level's spawn, and
  the path follower of a body out of the scene (2 cm a call along its
  route, no physics). It never moves by itself.
- The player's and the AI's per-tick "snap to the navigation grid" call is
  the grid's base-class slot, which the pathfinder does not override: it
  returns false and moves nothing. The grid reaches movement through the
  spawn snap (third behaviour tick), the look-ahead's line walk (the
  player stops short of walls) and the AI's planner and footprints.
- Walls and props block the body itself: its parts' spheres are tested
  against the front of the level's triangles where they are and along
  their path (already ported, `exa_physics` contact.rs), and bodies meet
  bodies through their pairs (body_pairs.rs). Steps up and down are the
  feet's Height Query probes, the lift (which aims the pelvis over the
  ground under the feet) and the leg IK (already ported).
- The feet routine sets the balance to 0 whenever the body does not
  locomote and has no support, with no exception (Exarust kept a standing
  body's balance while its capsule stood on the ground, "perched").

## What changes

- The capsule, its rigid body and collider, its registration and release,
  the kinematic targets and the sweep statistics are removed.
- Each fixed step a character goes at once, across the ground, where its
  body's frame was placed in its last character tick, and stands at the
  character's ground its body measured (the lowest foot's with support, the
  feet's blend walking, the ground under the pelvis without); without
  support it goes down with its pelvis, never under that ground. Nothing is
  swept. The body's carry still leaves that move out.
- A character without muscles (no frame) walks by its desired velocity and
  stands on the level under it (a short ray down), falling where there is
  none: Exarust's own, the original has no such walker.
- The perched exception goes: a standing body with no grounded foot loses
  its balance, as the original's does.
- The look-ahead's no-grid fallback keeps its 28 cm clearance as a constant.

## Out of scope

- The interim combat footwork still moves the character at its step's
  velocity (now unswept); the original's footwork step is the body's.
- The scheduling of the fixed step (the `cl/simthread` lane).
- Out-of-scene bodies sliding along their routes.
