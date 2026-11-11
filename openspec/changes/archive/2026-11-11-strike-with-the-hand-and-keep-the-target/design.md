## Context

The pair stage already hands every striker/hittable pair its hit event,
and a held weapon body is classified there (`block::strike_held`). A fist
had no body of its own, so Exarust swept a segment from the hand.

## Decisions

- The fist's striker is its hand part: the hit event names that part of
  the wielder's body, and the fist's `Blade` (child of the hand's scene
  entity) gives the frame, record and tip speed for the damage routine.
  The contact point is used as it is (the fist's frame is the hand's).
- The strike-chain bit is written only on window changes, like the held
  weapon's, and cleared when the fist leaves the world.
- A fist on no body part (a test rig, a body still building) keeps the
  sweep as a fallback.
- `CombatTarget` is a component so the bridge copies it; a fighter without
  it keeps the nearest-fighter stand-in (tests, bodies with no behaviour).
