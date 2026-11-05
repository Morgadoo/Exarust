## Why

Phase 4, TMotileDynamics, the motiledyn lane's second review (round 66):
the held S at exanima01's spawn. Tracing the stepping foot's lift and
landing, the stance bonus, the other foot's start and the support test
against the original found all four as Exarust has them, except that the
step lift reads the race record's stance factor (a human's 0.5), where
Exarust read the body's stance factor (raised by the gait, zeroed by a
knock-out).

## Evidence

REA: the feet routine's lift `−(1 − crouch)·f·(1.5 − race stance)`, its
amplitude set by the balance routine each tick the foot is chosen from the
horizontal contact-to-anchor distance (0.4·d, at most 30 cm); the 300 ms
stance bonus measured on the simulation clock from the foot's start; the
other foot's start landing an anchored foot where it is, ungated; the
support test `contact height ≤ ground + offset + 20` (30·scale moving)
with the ground probed toward the anchor.

## What Changes

- `exa_physics::body_bend`: the standing IK's step lift reads the race's
  stance factor.

## Out of scope

- The held S at exanima01's spawn still trips: the original's rules,
  traced, land the stepping foot in the air there as Exarust does (see
  docs/re/motiledyn-map.md); not compared with a retail capture.
