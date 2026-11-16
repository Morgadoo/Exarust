## Why

Roadmap task B4 (lane `motiledyn4`, docs/roadmap/phase-b-close.md; Phase B
close of docs/PLAN.md): the fourth pass over TMotileDynamics, the rows past
the body tick and the damage routine. The swing routine's opening block (an
attack aimed from a parry keeping its armed hold, two weapons guarding the
off hand) had no counterpart; the locomotion routine's tilt write, the
placement's clinch clear and the get-up tick's step and wait clears were
missing; the swing rate took the ordered side for a NaN progress.

## Evidence

REA (Exanima.exe 0.9.5), checked against the assembly, not only the
pseudo-code:

- The swing routine's opening: the stance-class test, the countered motion
  against the target's playing motion, the Ward bit, the 0.4 compare in
  double whose parity jump takes a NaN to the clear; the 0x90 class branch
  and its hand by motion; the motion start's unconditional clear when it
  leaves an attack.
- The rate block's compares (JP/JBE to the first-key shape, JP/JA to the
  smoothstep term) and the constants read from the image.
- The locomotion routine block by block; its supported tilt write after
  the run blend; the body tick's lean before it.
- The placement's opening (the parrying byte, then the clear).
- The balance routine's get-up branch (exactly 0x300) and its stores.
- The muscles-off sampler's only call: the gate's local flag has no store
  for a muscles-off body (unreachable).

## What Changes

- `parry::hold_counter_parries` runs before the swing in the attack stage;
  the attack's end releases a held arm bit with no countered motion.
- `Swing::rate` takes the unordered branches.
- `locomote` writes the head tilt with support.
- `limp::place_on_ground` opens with `parry::clear_for_placement`.
- `measure_balance`'s get-up branch ends the step and the posture wait.

## Out of scope

The placement's sampler (the original samples through the living sampler
in its placement mode; Exarust's uses the pose-only arithmetic), the weapon-
in-hand gate (open at once for a bone-attached weapon), the bracket's clamp
and guards, the jolt's order, other components' rows.

## Impact

Simulation: shield and two-weapon counters keep or drop their hold as the
original's; the two-weapon off hand is guarded through its attack; the
twist reads a tilt from the exertion of the tick; a get-up ends a step and
the posture wait. Determinism and grid scenes owed (GPU on hold).
