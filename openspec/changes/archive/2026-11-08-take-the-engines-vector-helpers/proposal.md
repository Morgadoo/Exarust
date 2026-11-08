## Why

PLAN Phase 5 (engine fidelity), FLX_System (unit routines). The base
library's vector routines were ported into `exa_physics::engine_math`, but
several live callers still did the same sums with their own copies or with
the Rust library's trigonometry, so the original's fast routines were
ported but never called there:

- the oriented-box slab test (RF_Intersection) divided exactly, where the
  original takes the processor's approximate vector reciprocal, and kept its
  own copies of the component minimum, maximum, smallest and largest;
- the combatant's head tracking (TCombatant, run by the player's behaviour
  while it stabs) normalised its look exactly and built the look direction
  with the library's sine and cosine, where the original calls the base
  library's fast zero-safe normalise and its pitch-and-yaw direction, which
  uses the engine's fast sine and cosine;
- the combatant's look direction did the same.

## Evidence

REA (Exanima.exe), checked against the assembly:

- The fast vector reciprocal loads (x, y, z, 0) and runs `RCPPS`; the
  oriented-box slab routines call it on the travel, then the component
  minimum and maximum (`MINPS`/`MAXPS`) and the largest and smallest
  component, as `slabs_hit` folds them.
- The pitch-and-yaw direction calls the fast sine-cosine on the pitch, then
  on the yaw, and writes `(cos p·sin y, −sin p, cos p·cos y)` (the sign by
  an XOR with the sign mask).
- The fast normalise sums `(x² + z²) + (y² + 0)`, takes `RSQRTSS` and masks
  it to zero when the sum is zero.
- Both head-tracking variants call the fast normalise on the difference (in
  the band), the heading twice, the fast arc sine, then the pitch-and-yaw
  direction; the combatant's look direction calls the same normalise,
  heading, arc sine and direction.

## What Changes

- `exa_physics::engine_math::reciprocal_approx3`: the fast vector reciprocal.
- `exa_physics::intersect`: the oriented-box slab test takes it, and the
  module uses the engine's component minimum, maximum, smallest and largest
  instead of its own copies (same results).
- `exa_combat::aim`: the head tracking normalises with
  `engine_math::normalize_approx` and builds the look with
  `engine_math::direction_from_pitch_yaw`.
- `exa_ai::combatant`: `direction` and `look_direction` take the same
  engine routines.
- `exa_world::sun_shadow`: `EXA_SUN_FIT_TRACE=1` logs each step of a view's
  fitted half extent (debug), so the sun fit's smallest-sphere solver
  (RFT_BoundSphere) can be seen running in a terrain level.

## Out of scope

- `exa_ai::motion` and `exa_ai::search` keep their exact-trigonometry yaw
  and direction helpers: Exarust's idle glue reads and writes a body's
  facing through them each tick (the round trip through the fast routines
  would turn a standing NPC), so they need that glue split first.
- The base library's thread, hook-list, deferred-call ring and timer
  plumbing, which Bevy's schedules, messages, task pools and std threads
  provide.

## Phase

Phase 5, engine fidelity: the base library's maths.
