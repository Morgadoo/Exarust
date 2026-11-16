## Why

Roadmap task B4 (lane `clothbody`, docs/roadmap/phase-b-close.md), the
TVCCloth rows of the function ledger that the cloth lane (round 231) and
the simsector lane (round 233) left. The cloth's body contact pushed only
the cloth: the body never took its share of the push, its slide friction
or the cloth's rustle, and the cloth stage ran after the bodies' post-step,
where it had no mutable body. The hand's grab pinned one vertex where the
original keeps a grab record spread over a triangle and smoothed. A
normal-mapped cloth kept its load-time tangents. The cloth object list and
the body parts' index used a 4096 × 4096 stand-in grid instead of the
scene's.

## Evidence

REA (Exanima.exe), checked against the assembly:

- The cloth's body-contact slot, after each body sphere that pushed: the
  summed push scaled by 0.015 (the body's class bit 0x10000), 0.01 (the
  cloth's class bit 1) or 0.002; the sphere's centre weighted in the
  body's rest tetrahedron (the approximate reciprocal); the body's motion
  record at that point read with the paired weighted sum; when its square
  is over the engine's small float, the cloth's rustle tail takes
  `double(v²)·2e-5`, the motion loses its part along the push, and
  `double(push²)·0.2 ≥ double(v²)` (or unordered) negates it, else it is
  scaled by `(float)−√(0.1·push²/v²)` in double, and the motion record
  takes it by the raw weights; the push itself goes on the body's
  particles by the raw weights (with the strain and edge restore); the
  body's frame is rebuilt into the copy the next sphere reads; after all
  spheres the frame is stored and the countdowns traded (the cloth's wake
  slot or the body's touch).
- The grid sector step calls the objects' body contacts after body–body
  detection and before body–level detection, the friction passes and the
  post-step; the particle mode's contact pushes no body.
- The grab record (60 bytes): weights, triangle, target, radius to the
  nearest pin in the mesh (5.0 with none, 0 at or under the small float),
  smoothed point now and a tick ago, pin; the integrate slot's damped
  follower (the target clamped to the radius about the pin's particle,
  then motion += 0.1 of the way, point += 0.3 of the motion); the
  constraint pass's spread by weight over `MAXSS` of the weights; the drag
  handler's target write and wake to 32; the release's drop of the newest.
- The draw update's tangent rebuild for a first part with flag 8: the
  face u-gradient through the exact reciprocal, summed into the vertices,
  less its part along the rebuilt normal (`(x + y) + z`), the approximate
  normalise, three lanes written.
- The integrate slot's motion energy: `min(double(√sum²)·0.001, 1)` over
  the small float, else 0.
- The draw update's countdown branch needs the node's draw word zero after
  testing it non-zero: unreachable in 0.9.5.

## What changes

- `exa_world::cloth_stage`: the cloth stage around the solver: the plan
  and every awake cloth's own step before it, the body contacts in the
  solver's object-contact hook one cloth at a time (runs of particle-mode
  cloths at once), the after-collision pass and the level contact after
  it. `SolverStage::sim_object_contacts` takes the sleeping bodies too.
- `exa_physics::cloth_body::touch_ragdoll`, `body_share`, `body_slide`,
  `trade_body_countdowns`, `sphere_sweep`; `Part::motion_at`,
  `Part::add_motion`; `sector::object_meets_part`.
- `exa_physics::cloth`: `integrate`, `touch_listed_bodies`,
  `after_bodies` (the step split), `GrabRecord`, `Cloth::add_grab`,
  `Cloth::grabs`, `Cloth::rest`, `rebuild_tangents`, `motion_energy`.
- `exa_world::cloth_parts::ClothGrid`: the scene's grid keys for the
  object list and the parts' index.
