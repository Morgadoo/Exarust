# Proposal

## Why

docs/PLAN.md, Phase 6 (NPC AI), the TMotileAI paragraph: a pass over the
NPC brain's motion, idle-driver and constructor arithmetic left several
rules that did not match the original's instruction listing, and a review
of that pass found more.

## What Changes

- Personality: the eagerness is `((3 − r) + (3 + c))/8` in that order; the
  shares over 100 multiply by the double 0.01 and the blend's product and
  sum run in double.
- Ally sense: the observed body's state factor is 0.25 with its in-combat
  mark, otherwise 0.1 (no stand-in byte).
- Walk: the short facing threshold is formed in double, a NaN facing takes
  the near branch, the direction is scaled by the reciprocal length, a NaN
  or sub-centimetre square uses the facing at length 1, and the no-foe
  branch zeroes the head offset's target before it eases.
- Alert look: both draws are made every tick, the narrow pan spreads the
  f32 draw over the double π, a NaN panic passes the gate, and a foe over
  45° takes the look before the body turn.
- Body in the way: the remembered and carried bodies are skipped and each
  body is shifted 20 cm along the walk direction.
- Look writes: leaving combat writes the look, heading and eased offset;
  the room-walk settle writes the look, heading and head look; the aim
  snap also writes the body's look, smoothing and heading.
- Idle driver: the room walk weighs every unvisited exit (the previous
  cell's included), going back clears the avoided exits, the walk runs
  while its deadline lasts, the first avoided exit sets a marker, and the
  use state's ten-second timeout (64-bit sum, the exit's own bit) runs even
  after a completed use.
- Investigation: the 0.02 gap and the 0.3 gate are compared in double.
- Re-placement: a collapsed entering body keeps its stored placement, any
  other takes its point and faces its path goal.

## Evidence

The original's AI constructor, attach, ally sense, walk driver, alert
look-around, body-in-way test, leave/settle/snap routines, room-walk goal
callback, idle driver, investigation and re-placement, read instruction by
instruction in the TMotileAI pack's assembly listing.

## Out of scope

Wiring the idle driver, the walk and the alert look into the running game
(they stay tested functions); the scene holder move and the physics
re-placement; the standard sense's per-contact threat estimate.
