# Design

## Behaviour

The far variant runs the same look-band selection, the same approximate atan2 and asin, the same π/100 slew and the same ±π/4 body clamp as `track_head`. It differs in two constants: the published point is 150 cm along the direction, and the pitch target is not floored at −0.6, so a look far up can keep a steeper pitch.

## Model

`track_head_far(head, previous_dir, eye, body_forward, look_at)`; `track_head` and the far variant share an inner function parameterized by the look-ahead and an optional pitch floor.

## Open questions

The caller's eye frame is separate.

## Docs

`TCombatant` is **33%**. The headline stays 14.4% and 5.7%.
