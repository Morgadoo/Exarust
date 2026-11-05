## Why

Phase 4 (Combat and items) in docs/PLAN.md, TMotileDynamics, the motiledyn
gap lane (round 66). The class's two largest live gaps the shipped game
reaches every fight and every spawn:

- The combat footwork moved the player's character at a step velocity of
  its own (Exarust's interim capsule speed, 1.4 m/s) while a step was under
  way, and played the combat walk clip. The original's behaviour writes its
  body's own step fields (stepping, the balance-step flag, the step target,
  the long kind, the queue) and the body walks the step on its feet: the
  stance moves to the target, the feet step there by the stance score, the
  chest pins lead toward it, and the character follows the body's frame.
  Its stance clip plays throughout.
- The placement on the ground left the balance's chest history, the foot
  states and the feet centre as they were before the move, so the next tick
  read the jump as chest velocity.

## Evidence

REA (Ghidra listing of `Exanima.exe`, checked in the assembly): the player
behaviour's step dispatcher and goal routine (the under-way test on the
body's stepping and balance-step bytes and the target against the feet
centre, the start's writes, the goal written in place as the body's step
target, the tapped stride's kind 2), the queued-step taker (target, grid
clamp and snap, stepping on, the flag and landing count cleared, the start
clock, the later stand-still stamp 0), the balance routine's balance-step
start, the knockback's queue clear, the locomotion gate's and the lift
servo's reads of the body's stepping byte, the player's look-ahead gate on
it, and the placement on the ground's tail (the chest routine, last tick's
chest, both chest velocities zeroed, the feet mean of the anchors, every
foot state cleared; the pins and their mean, which the body tick takes anew
from the frame before any reader).

## What Changes

- `exa_physics`: the body's step keeps the balance-step flag and the queue
  word; `Ragdoll::footwork_step` / `write_footwork` (`FootworkStep`) give the
  player's behaviour the body's step; a queued step is taken only while the
  queue waits and leaves its countdown; the movement gates, the muscles and
  the lift read the body's own step episode; the placement on the ground
  restarts the chest history, the feet centre and the foot states.
- `exa_game`: the footwork reads the body's step before its dispatcher and
  writes it back after; the under-way test reads the feet centre; a taken
  queued target is clamped by the line walk and the grid snap on the next
  behaviour tick; the capsule step velocity, its landing and give-up timer,
  `Locomotion::stepping` and the combat walk clip are gone; the walks' look-
  ahead runs only while the body does not step.

## Out of scope

- The step length the goal routine writes (the body keeps computing its own
  footwork length from the reach) and the other per-tick rewrites of it.
- The ground placement's own start (its probe set-up, the frame's ground),
  cloth and attached bodies, its behaviour hook, and its other callers
  (teleports, summons, saves, the AI's push out of a blocked region).
- The out-of-scene route slide, the AI's footwork (it still steps its
  capsule), the support pins' per-part order in the link iteration.
