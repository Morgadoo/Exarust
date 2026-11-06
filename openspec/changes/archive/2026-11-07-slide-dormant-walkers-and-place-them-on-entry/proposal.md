## Why

TMotileDynamics gap (round 88): the out-of-scene route slide needs the
re-entry placement; the AI driver's obstacle check was not called. A walker
that left the 40 m activation range mid-route froze where it left.

## Evidence

REA (Exanima.exe):
- The AI's path follower, out of the scene (the scene's low bits clear),
  slides the standing point and the ground point 2 cm a tick along the route
  from its segment and fraction, with no physics; the whole AI tick keeps
  running, so the roam moves on through its legs and pauses.
- The re-entry routine (TMotileAI, after the scene puts the node back):
  matrix = the character frame, or while a route is followed a basis facing
  from the chest toward the goal; translation = the ground standing point,
  snapped by the pathfinder's nearest walkable point; then the body reset
  (TMotileDynamics #61: condition 0, the clip snapped to the stored stance,
  moving, free walk, flags, turning and stepping cleared), the living
  sampler and the placement on the ground (#62) with that matrix.
- The driver's obstacle check: each nearby character but the remembered one,
  the carried one and a dead one (condition 0xA00), moved 20 cm along the
  walk direction, tested by the 120 cm steer line; a hit is remembered and
  the driver's stop routine soft-stops and ends the route follow. The planner
  forgets the remembered body on its turn once that body's flag bit 2 is off.

## What Changes

- exa_ai: `walker::slide_walk`; `idle_walk` keeps a dormant roam running and
  slides its character's root, holds the walk through the scene's leaving
  and entering ticks; the entering reset snaps the clip to the stance;
  `drive::obstacle` (`motion::body_in_way`) in the roam, the flight and the
  follower, forgetting the remembered body at the planning turn [guess].
- exa_game: `limp::place_entering` (nearest walkable, `GroundPlacement` with
  the model's own turn).
- exa_physics: the placement on the ground starts the frame afresh (the
  root's last aim, the anchor and the frame cleared) and wakes the body.

## Out of scope

- The carried body in the obstacle check (no carried characters yet).
