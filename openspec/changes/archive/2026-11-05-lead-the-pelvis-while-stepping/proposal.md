## Why

Phase 4 (Combat and items), TMotileDynamics, the motiledyn lane's review
(round 66): with the combat footwork on the body's feet, holding S made the
player stumble and fall. Tracing the backward step against the original
found three deviations in the step's lead and stance, which together
dragged the body ever farther past its target:

- the pelvis pin had no lead while stepping (the original leads it by its
  own rule, direction dependent: half the lead backwards);
- both leads measured the step's offset from a stale standing point, not
  from the standing point the anchor routine keeps on the pelvis while
  stepping, so the lead never faded as the body reached its target;
- a step of the long kind clamped its stance with the length the feet
  routine rewrites (1.5 × the reach); the original clamps every kind with
  the stored reach (1.1·reach·scale, in double).

The thorax reach also narrows by the step episode (not by a stepping foot).

## Evidence

REA (assembly and pseudo-code of the body tick's pin block and the balance
routine's stance origin): the root part's point record led while stepping
and not out of balance (the cosine with the frame's third row, halved
backwards, squared share of the step length forwards, 0.4/0.7, 0.75, 1.2,
level), the 60·scale clamp; the step offset as the step target less the
standing point; the stance clamp reading the stored reach × 1.1 (double) ×
scale; the thorax clamp reading the stepping byte. Constants read from
`Exanima.exe`.

## What Changes

- `exa_physics::body_steps`: `pelvis_lead_pose`; `stance_origin` in double
  with the plain reach for every kind; `knockback_length` and the trigger's
  extension and encumbrance removed (no reader).
- `exa_physics::body_pins`: the pelvis pin led; both leads from the frame
  anchor; the thorax reach by the step episode.
- A three-second backward footwork test on a synthetic humanoid.

## Out of scope

- A comparison with the retail game: exanima01's spawn spot (the left foot
  13 cm higher than the right) still trips on a held S (the stepping foot
  lifts 30 cm and is landed in the air when its 300 ms stance bonus ends).
- The jolt and step end writing a standing point the frame does not read
  (the original keeps one standing point).
