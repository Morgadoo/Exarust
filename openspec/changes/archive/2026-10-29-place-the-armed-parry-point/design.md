# Design

## Context

The armed parry routine computes a parry point on the attacker's blade
before it enables a grip link. The blade is a tetrahedral rigid body: a
local point (0, h, 0) is carried by its rest-tetrahedron barycentrics onto
its corners now and a tick ago.

## Decisions

- Pure functions, not wiring. The armed parry is not modelled in the live
  fight (a weapon-holder does not parry), so the geometry is exposed as
  `parry_blade_height`, `blade_point`, `parry_point` and `parry_distance`
  for the future wiring to feed `armed_grip`'s `passed` and
  `horizontal_cm`.
- Float order from the instructions. The barycentrics use the fast
  variant (approximate reciprocal), the 4×3 product sums in pairs (the
  pseudo-code shows a sequential sum), the projection removal sums x and
  z first, the two normalisations use `RSQRTSS` with the zero mask (new
  `engine_math::normalize_approx`), the sweep dots sum x, y, then z, and
  the strengths are double chains.
- NaN branches follow the compares: a NaN gap takes the within-reach
  branch, a NaN lead is kept, a NaN comparison is not passed, a NaN
  falloff distance falls through to the linear branch.
- `stance_by_energy` is one shared rule; the AI's guard choice uses it.
  The parry's end keeps `parry_stance`, whose dazed test is already the
  body's double compare.
- The map in `docs/re/` names routines by role and their index position
  only; shares are estimates from decompiles and a code sweep.

## Risks

- The AI guard boundary moves by one float step; no visible change is
  expected (`play arena1 --opponent` should look the same).
