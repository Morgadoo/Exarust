# Design

Notes: [`crates/exa_physics/README.md`](../../../crates/exa_physics/README.md),
[`docs/formats/RSQ.md`](../../../docs/formats/RSQ.md), How the original
simulates a body.

## Units

The original works in centimetres, Exarust in metres. World-link
corrections are worked in centimetres (the 10·strength and 5 cm caps, the
0.8 plane removal in double, the 3·√mobility drag cap) and converted back,
so the decisions are the original's up to rounding. The soft weld's share
0.1/(m + m') is divided in double and cast to single, as the original's.

## Faithful quirks kept

- A pivot's correction has no epsilon guard in the original:
  strength/(|d|²·softness + 1) in centimetres.
- A slack link inside its slack compares |len − L| against the epsilon with
  the original's direction, so a NaN change is skipped.
- A slider's correction is pushed twice; the plain slack link records the
  taut distance and forgets it inside the slack.
- The repair puts the current points at the tick's start shifted by the
  box centre's displacement and leaves the previous points alone.

## Departures

- The limp pivot skips a gap under the epsilon (3.4 µm): closing those too
  only lengthens a resting limp body's settling (the sleeping test).
- The repair's inverted test compares the vector's side with the rest
  tetrahedron's own, so a body built the other way round is not repaired
  every tick; the original tests the sign alone.
- The draw fraction is Bevy's overstep fraction, not the node clock.
