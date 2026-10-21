## Context

The first pass ported the routines from pseudo-code. Review against the
instructions found the usual losses: sums reordered (the original adds
z + (y + x)), min/max written as ordered compares, mixed f32/double
arithmetic, and invented fallbacks.

## Decisions

- Keep the DeepSeek commits and add corrections on top, citing the
  instruction semantics in test comments rather than addresses.
- Reuse `exa_physics::body` types; the swing, parry, weapon, stagger, recoil
  and burn rules stay library code until the combat stage wires them (a
  guessed trigger would change the running opponent fight).
- NaN-sensitive negated compares keep the original's branch and carry
  `#![allow(clippy::neg_cmp_op_on_partial_ord)]`.
- Wired now: the femur posture, head tilt, exertion, look target, start
  notice, carry walk and trailing-foot plant.
