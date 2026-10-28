# Proposal

## Why

docs/PLAN.md, Phase 3 (bodies: skeleton, physics, control), the player's
controls and behaviour: the playerctl5 lane (SG_PlayerControls unit
routines, TPlayerBehaviour) added the movement word's 200 ms tap, the bone
lock, the auto-target, a look snap and a yaw step, per-sphere placement
sweeps, level placement markers, a cloth grab, a door hold, attack timing
on the live swing and use-cursor readers. Its review checked each rule
against the executable's instructions; several were wrong and three
changed the running game wrongly (an invented attack gate, a door hold
ending after 2000 ms, and spheres swept from where the item stood).

## What Changes

- Movement word: A is 2 and D is 8, opposing keys cancel (A with D, W with
  S), the analog test is a double compare a NaN fails, and the tap flag
  reads the crouch amount (under 0.1), not the body's speed.
- Bone lock: a NaN fraction stays NaN; the locked height is the reference
  point moved toward the nearest bone by its fraction.
- The routine read as a pin-cursor aim step is the camera's follow: over
  0.1π from the body's yaw, the camera's yaw becomes the body's put back
  0.1π on the camera's side. The look snap compares look minus cursor,
  wrapped in double precision, strictly over the single 30°.
- Placement sweep: every part's spheres ride with the placed grab point.
- Placement markers: eight hexadecimal name characters give the id; the
  segment runs to the pick (10 000 cm without one) and meets the entry's
  sphere in the engine's arithmetic; an item needs an object id.
- The door drag lasts until the release (the 2000 ms stamp is the last
  pusher's timer); the cloth's drag plane faces the camera.
- The attack gate on the recovery clock is removed (the executor reads it
  only for Feint, which exa_combat::swing already keeps); the duplicate
  world-use rule is removed.

## Evidence

The behaviour's aim point and bone fraction, its auto-target, look snap,
camera follow step, attack timing and movement-word routine, the key
reader, the controls' drag dispatch (placement and markers), the
segment-sphere test and hexadecimal reader it calls, the door and cloth
drags, the per-frame update's deferred list and the deferred drop, read
from the disassembly; the last-pusher reading from the physics notes.

## Out of scope

The movement word's reset path and its 300 ms step branch, the combat
footwork and dashes, the bone lock's, auto-target's, camera follow's and
look snap's callers (no target entity, no camera-follow option), level
placement marker registration and object ids on props, door and wheel
bodies, the cloth's own grab record in the cloth step, and the GUI item
layer.
