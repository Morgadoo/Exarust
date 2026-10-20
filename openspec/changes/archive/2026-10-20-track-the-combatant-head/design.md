# Design

## Behaviour

The eye is the body frame applied to the local `(−20, 10, 0)`; the look direction is `look_at − eye`, and its squared horizontal length `d² = dx² + dz²` decides the band. Under 2,500 (50 cm) the previous unit direction is used unchanged. Under 10,000 (100 cm) the vertical component is replaced by `(10000 − d²)·1e-4` and the vector is normalized; otherwise it is normalized. These are centimetres, so the mid-band value is 0…0.75 before normalizing.

Yaw chases `atan2_xz(d)` by at most π/100, where the angle is the engine's cheap approximation: `c = z/√(dx²+dz²)`, `π` at `c ≤ −1`, `0` at `c > 1`, otherwise `π/2 − f(c)` with `f(x) = (π/2 + 0.2075x + 0.0535x²)·√(1+x) − π/2`, negated for a negative x. That approximation is deliberately not the true atan2 (for a target 10 cm right of 10 m ahead it returns about 0.551 rad); the model reproduces it. After the step, yaw is held within ±π/4 of the body's facing, overriding the step when needed.

Pitch chases `−f(y)` (the target floored at −0.6) by the same π/100 step. The published direction is `(cos p·sin y, −sin p, cos p·cos y)`, the aim point is 250 cm along it, and the flat direction is `(dir.x, 0, dir.z)` normalized.

## Model

`HeadTrack { yaw, pitch }`, `HeadLook { dir, point, flat }`, `track_head(head, previous_dir, eye, body_forward, look_at)` and `head_atan2_xz(x, z)`.

## Open questions

The eye frame and the point-of-interest selection belong to the caller.

## Docs

`TCombatant` is **30%**. The headline stays 14.4% and 5.7%.
