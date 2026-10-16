## Context

The original's body tick smooths the chest's velocity, decides falls from
last tick's balance, recomputes the chest point, runs the balance routine
(feet centre, support point, score, counter, balance), then the feet
routine (a Height Query per leg, the grounded test, support S). The
research notes, with addresses, are git-ignored:
`context/re/body-feet.md` (§0–3, §7, §13, §14), `body-drive.md` (§5),
`body-tick-order.md` (§8) and `body-controller-plan.md` (C6).

The format page this change extends is
[`docs/formats/RSQ.md`](../../../docs/formats/RSQ.md) (Physical body).

## Goals / Non-Goals

**Goals:**
- the chest, its smoothed velocity, the support point and its shifts;
- the score, the vertical cut, the counter and the recovery, the get-up
  rise;
- the Height Query against the level, the per-leg grounded and no-ground
  tests, has-support, the landing weight, the airborne counter, balance 0
  without support, S, the get-up support near the ground;
- S feeding the link muscles and the trace; Exarust's fall and get-up
  reading the new balance.

**Non-Goals:**
- recovery steps (C15), foot anchors and friction (C12), the lift servo
  (C8), support pins (C14), the original's falls and get-ups (C11).

## Decisions

- **Where it runs.** `Ragdoll::measure_balance(level)` runs in the
  character tick (`exa_game::limp::tick_characters`), after the body learns
  its bearing and before it is aimed, on the particles the last tick left:
  the original also measures before its limb step carries the body. The
  link muscles' gains (set in `aim`) read this tick's S.
- **Units.** Exarust stores metres. One helper turns metres (or metres a
  tick) into cm, and every rule is written in the original's cm and ticks:
  3 and 6 cm/tick, 25 cm², 20/30/30 cm, 70/5/10/20 cm, 2000 cm.
- **Chest velocity lag.** The original smooths chest(N−1) − chest(N−2)
  before recomputing the chest; Exarust keeps the last two chest points and
  does the same, so a motion shows one tick late.
- **The contact is the ankle.** Muscle setup moves each lower leg's contact
  to its foot part's origin before anything reads it. Exarust builds the
  legs from the setup: the contact point is the foot part's origin when the
  lower leg has a foot part (bones 4 and 7), else the stored point. The leg
  length is the lower leg's origin in the thigh plus the contact's distance;
  the sole height is the foot box's depth under its origin less 0.5 cm,
  from the part's mesh box (the box the original uses is its part's box;
  its source is inferred). C5's leg relaxation reads the same point, so a
  straight standing leg relaxes at 0.2.
- **Height Query.** `Level::height_below(at, radius, depth)`: the lowest
  point of a sphere starts at `at`, swept straight down. It gathers the
  column's triangles from the level, keeps upward faces with the sphere in
  front of them (as Exarust's level contacts, one-sided; the original's
  per-mesh test is not read), tests them highest first with an analytic
  drop onto face, edges and corners (a parry shape cast misplaced a contact
  by 5 mm on random cases), and stops once no face can be met sooner. A
  2 m column is tried before the whole 20 m (same answer, fewer
  triangles). The level is the static colliders: character capsules are
  kinematic and bodies are not in it, so living bodies are excluded; dead
  bodies and loose props are not part of it either.
- **What the original does that Exarust leaves out.** The gait bump
  (condition 1 sways the probe forward by the clip phase and widens it to
  20 cm; it needs the gait phase, C19), the smoothed ground's
  gait-weighted fall (falls at once here), the combat condition's 5 cm
  probe, the stepping flag (no recovery steps: "still" is "not moving").
- **Bodies without two legs** (the tests' synthetic bodies) are not
  measured: support is 1 standing, 0 fallen, as before.
- **Fall trigger.** Exarust's trigger (balance < 0.6) moves to the fixed
  step, before the character ticks, so it reads last tick's balance at any
  frame rate; the clip is Fall Forward when the lean (support point to
  chest) points ahead of the body root, else Fall Backward, as the
  original's fall branch picks. The get-up's balance ramp is the balance
  routine's get-up branch, driven by `Bearing::getting_up`.
- **Perched capsules.** Exarust's controller capsule (28 cm radius) climbs
  rubble up to 35 cm; in exanima01's start room it stood on a 27 cm stone
  with both ankles 35–44 cm up, so the standing body lost its balance and,
  after each get-up, fell again (the get-up teleport left the controller
  hovering: it skips sweeps while grounded and still). Two fixes: a get-up
  marks the controller ungrounded so it settles under the body; and,
  Exarust's own until the body stands on its feet (C8, C18), a standing
  body whose controller stands on the ground keeps its balance when no foot
  is grounded (`Bearing::carried`). S stays grounded × balance.
- **Arms.** The original's arm state "balance < 0.6 or getting up" now
  reads the measured balance too.

## Open questions

- Is the original's per-mesh Height Query test one-sided? (REA on the
  mesh callback at the Height Query's object loop; it matters only for a
  body down, whose probe starts at its ankle.)
- The source of the part box (its centre and half extents) that gives the sole
  height: the node mesh's bounds or the collision spheres' (REA on the
  writers at part registration). Exarust uses the mesh box: 9.68 cm.
- Retail capture of balance, counter and S through the T0 probe (expected
  1 / 80 / 1 standing) once the probe exists.
