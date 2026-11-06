## Why

TMotileDynamics gap (round 73): the AI's footwork. The player's combat steps
are walked by the body's own feet since round 69, but a duellist's chosen
step still slid its character at a walk velocity with Exarust's "Combat
Walk" clip, ended by distance (10 cm) or a 1 s timer.

## Evidence

REA (Exanima.exe): the AI's combat movement routine writes the chosen step
into its body's step fields, the same fields the player's footwork and the
body's queued-step taker write:
- a new step: the target is the chest plus the place's offset, then the
  grid snap; stepping on, no balance step, the landing count and the later
  stand-still stamp 0, the start clock now (the step length reset to the
  plain reach while the kind is 0);
- a long step's second half: kind 2 (long), the length
  (1.5 − 0.3·encumbrance)·reach·extension, the target re-aimed from the chest
  and line-walked from the chest at the ground, no new start;
- a step while one runs: the queue word 1 and the queued vector (the
  place's offset);
- a place of no step clears the queue word; a re-chosen step sets the kind
  to 0;
- a queue word under 0 (the body took the queued step) costs 8 or 4 and
  goes back to 0.
The body's queued-step taker bends the queued vector at the target's
personal circle when it points along the facing, then aims at the chest
plus it, line-walks from the chest and snaps.
The AI does not go through the player's word dispatcher: it reaches the same
body fields from its own place search.

## What Changes

- exa_ai: a duellist whose body has muscles reads its body's step (stepping,
  kind, age, landings, queue) as the combat state's, writes the outcome back
  (`apply_feet_steps`, after the duellist in the brain chain) and takes its
  stance instead of gliding; the taken queued step is bent and line-walked
  on the AI's next tick from the chest then (Exarust's body has no target
  nor grid). `StepMarks::queue_taken`.
- exa_physics: `FootworkStep::landed`; `Ragdoll::age_step` (tests, tools).
- A body without muscles keeps the interim glide.

## Out of scope

- The AI's route walking (chase, idle, flight) still drives the capsule.
