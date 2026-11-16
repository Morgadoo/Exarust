## Why

Roadmap task B4 (lane `motiledyn3`, docs/roadmap/phase-b-close.md; Phase B
close of docs/PLAN.md): the third pass over TMotileDynamics. The swap key
swapped the weapon sets at once instead of queueing the switch clip, and
the clip's swap point traded two held weapons' slots, which the original
never does; the body tick's two-hand group call had no caller; a light
blow left the struck body's step start alone; the posture block had not
been compared arm by arm, and a shocked body never left the shock.

## Evidence

REA (Exanima.exe 0.9.5), checked against the assembly, not only the
pseudo-code:

- The body tick's queued-motion block, the view's swap request (busy bits
  0x10202, the desk drag, its own open inventory window: swap now, else
  write the switch motion into the requested motion) and the posture's
  switch arm (past 0.6 in double, a NaN progress keeps the switching word;
  with bit 4 clear it posts the set swap to the main thread's queue, then
  writes the switched word).
- The two-hand group call block (in-combat byte, the off-feet bit, walking,
  attacking) and the group routine (the hand list, their parents, the
  second hand's grandparent, bit 30); the death's call with mode 2.
- The damage routine's knockback test (J over 0.5 by a float compare; else
  the step start stamp = clock − 1000 ms) and the readers of that stamp
  (100, 200, 300 ms windows). The flesh path's armour loop and channels
  compared instruction by instruction (constants read from the image).
- The scripted damage hook's global has no writer in the image (no store,
  no data pointer): its branch is unreachable in 0.9.5.
- The posture block arm by arm (shock deadline, stagger capture, relaxed
  blend's tangle test, the attack arms' read of the swing strength before
  the attack routine's stall drain).

## What Changes

- The swap key queues the switch motion unless the character's own
  inventory is open; the posture's switch arm sends `WeaponSwapDue` and the
  frame swaps the sets; the held-slot trade is removed.
- The two-hand group is written every living body tick and cleared at death.
- A light blow's shove dates the struck body's step start a second back.
- The shocked arm ends the shock past the force release's busy deadline;
  a stagger always takes the whole capture; the relaxed arm blend reads the
  tangle counter; the attack arms read the strength before the stall drain,
  and the drain keeps a NaN.

## Out of scope

The sampler call's tail (part matrices and foot contacts on the frame:
Exarust's carry and aim are structurally different), the Fend skill's
auto-parry entry, the dormant lying arm's frame-height write, the get-up
start's heading source, other components' rows.

## Impact

Simulation: weapon switches play their clip before the sets change (the
AI's draw now swaps the sets too); hands and forearms stop meeting each
other while grouped; light blows end running step episodes early; shocks
end; stalled swings set their arms from the strength before the drain.
Determinism and grid scenes owed (GPU on hold).
