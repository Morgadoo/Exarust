## Why

Phase 4, TMotileDynamics, the backfall lane (round 73): a scripted held S
falls in exanima06 and at exanima01's spawn. Traced tick by tick, the
stepping foot rose to 30 cm over its ground and was landed there by the
other foot's start once its 300 ms stance bonus ended. The step lift that
raised it is computed by the original's balance routine into a field of the
character, while the feet routine reads the same offset of the lower leg's
own part, which nothing writes: the original's stepping foot has no step
lift.

## Evidence

REA (assembly): the balance routine's four accesses to the lift scale go
through its `this` (the character, stored once at entry); the feet
routine's four reads go through the leg list's part; no other instruction
in the image addresses that offset; Free Pascal zero-fills a new instance.

## What Changes

- `exa_physics::body_bend`: `step_lift` takes the lift scale and the
  squared distance (the feet routine's form, its epsilon branch included);
  the standing IK passes the zero scale the original reads.
- Tests: the rule at a 16 cm scale and at the zero scale; the backward
  footwork test asserts no foot is lifted.

## Out of scope

- The static collider build's wall-clock budget, which makes two runs of a
  scripted scene diverge from their first ticks (not frame-count based).
- The stair edge Rebalance in one of two stairs runs (the standing foot's
  probe drops off the step: the original's probe).
