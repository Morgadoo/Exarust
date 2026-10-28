# Proposal

## Why

docs/PLAN.md, Phase 3 (bodies: skeleton, physics, control), the player's
controls and behaviour: the playerctl7 lane (TPlayerBehaviour). The
behaviour tick's combat branch runs a step machine that Exarust only
approximated: its movement word, the counted (tapped) step, the queue of the
next step while one is under way, the movement goal's navigation write and
the mode word that decides whether the right button walks. Exarust's
footwork walked while a key was held and its 200 ms word persistence was
wired into exploration, where the original does not read it.

## What Changes

- Combat footwork lays discrete steps through the original's dispatcher:
  start a step toward the movement goal, re-lay a young step at the tapped
  stride, queue the next step (raw word and its full step vector) while
  one is under way; a landing takes the queue by the body's queue rule and
  the word takes the queued word back; the queue clears 200 ms after the
  word went quiet.
- The step gate: condition mask `0x82C1`, energy 0.7, no swing.
- The movement goal sums W, A, D, S in the original's order, and a taken
  goal snaps onto the grid's nearest walkable point.
- The goal no longer applies the deep crouch's 0.7 (the goal routine lays
  its own stride).
- The 200 ms movement word is the combat footwork's; exploration reads the
  keys as they are.
- The combat mode word (down, dazed under 0.85, switching, casting, right
  button, dash, left button) gates the right button's walk to the cursor.

## Evidence

The behaviour's word routine, the step dispatcher after it (the 300 ms
young-step window, the 20·scale under-way test on the flattened squares,
the queue writes), the goal routine (stride, line walk, 400 cm² test, the
grid snap after it), the motile's step-gate routine (condition mask, the
0.7 energy compare) and the combat tick's mode ladder (the 0.85 energy
compare that reads a NaN as enough), all read from the disassembly with
the REA tools.

## Out of scope

The interim feet (the capsule still moves at a walking speed; the
original's feet carry the step), the target-approach bend of a lone forward
word, the pad's analog word in combat (the pad reaches the footwork as
keys), the dash latch and the kept action of the mode ladder, door and
wheel bodies, level placement marker registration, the GUI item layer, the
deferred interaction list and the full release dispatch.
