## Context

The original's living link iteration runs children first. For each slack
link (slack over 0.01 cm) it reads the animated anchor, weights on the
parent that the clip sampler wrote this tick, on the parent's current
particles. It draws the anchor toward the part's point by the part's
relaxation λ. It adds a damping term from the PD history: the point and
the anchor on the previous particles, recorded at the end of the group
root's limb step. It saturates the result, scales it by strength × part
factor × √E·T and adds the radial excess beyond the slack. The research
notes, with addresses, are git-ignored: `context/re/body-links.md` (§4–5),
`body-drive.md` (§2–3), `body-tick-order.md` (§5.4, §8),
`body-q8-q9.md` (§7–9) and `body-controller-plan.md` (C5, Q1–Q3).

The format page this change extends is
[`docs/formats/RSQ.md`](../../../docs/formats/RSQ.md) (Physical body).

## Goals / Non-Goals

**Goals:**
- the link muscle law, its history and its per-character factors;
- the gains the character tick sets: stance factor, tone target, part
  factors, leg limit blend, arm stiffness, anchors and relaxation;
- the pose drive only where the original has one;
- race factors and the character's state reaching the body.

**Non-Goals:**
- the original's pose drive law, support pins, lift servo and balance
  (C6–C8, C14); attack arm states; weapon bodies (C21).

## Decisions

- **Anchors from the targets.** The character tick already places each
  part's animated world transform. The anchor of a link is the part's
  point (from its rest particles) under the rotation of
  `target[parent]⁻¹ · target[part]`, at the part's origin (the model's
  translation, which C3 stores per part), as the original resets the
  relative matrix's translation to the part's model translation. A fallen
  body's targets are aligned to its pelvis; relative turns are unchanged.
  Bodies keep their anchors through a fall and a get-up.
- **History from the tick's start.** The damping compares the gap with the
  gap where the tick began. Exarust's pose drive edits `previous` to damp,
  so the history reads the particles saved before integration (`start`).
- **Units.** Exarust stores metres. `muscle_pull` takes cm: the 0.5 and
  1.4 of the damping, c = 0.1 per cm and 1.19e-7 cm² are the original's.
  The relaxation's 32 and 0.001 per cm² apply to squared distances in cm.
- **Relaxation for every part.** The plan says arms and legs. The sampler's
  per-part loop sets λ for every part with link muscles (no arm test), and
  only the feet routine overrides a supported leg's. A negative target
  leaves λ as it was, as read; so only a slowly moving anchor lowers it.
- **The leg's rest reach.** The feet routine compares the hip-to-contact
  distance² with a shin field that muscle setup sets to (2 × the contact's
  distance)² and that the feet routine replaces with the animated reach
  only in flagged conditions (getting up). Exarust uses (2r)², which leaves
  a standing human's leg λ at 0.
- **Leg modes.** Standing uses mode 3: a thigh 0.2 × 1.25 × scale, the leg
  limit's target 0.5·S². Moving uses the walk's mode 0: a thigh 0.2, the
  target 0.4. Sprint (mode 1) and backpedal (mode 2) are not told apart.
- **Arm stiffness.** Q9's subset: off balance (fallen, lying or getting
  up) 0.5; tone under 0.85 or the Walk and Run clips 0.2; otherwise the
  relaxed blend; the turn floor uses the player's heading error (0 for
  NPCs, which turn at once). The original computes the floor from last
  tick's error; Exarust reads this tick's, after steering.
- **What the game tells the body.** `Bearing` (moving, run, striding, off
  balance, grounded, turn left) is set by `exa_game`'s character tick
  before the body is aimed. Moving is the controller's desired velocity,
  the run blend the player's, striding the Walk and Run actions, off
  balance the `Fallen` marker. `RaceMuscles` (stance factor, arm
  relaxation) travels on `Physical`; the viewer fills it from `races.rdb`
  for placed characters and the character view's race.
- **Strike range.** With the arms on link muscles the duellist's blade
  came within 0.18–0.26 m of the target's parts at best, against
  0.02–0.10 m before, and no strike landed. The 1.5 m range is Exarust's
  own guess; 1.2 m lands 10 of 17 swings on a passive player (11 of 41
  before). The original's reach and footwork come with C16 and C21.
- **Synthetic humanoid.** The tests' humanoid gets a link muscle per joint
  (12 cm along the part, 12 cm of slack) next to its hinge, so the fallen
  and shove scenarios run with muscles. A limp variant keeps only hinges
  until limp joints are damped (C4).

## Open questions

| question | how it is answered |
|---|---|
| Arm stiffness during attacks (the swing rule, W[20], the swing clock) | REA answered the rule (Q9); it needs the weapon's derived W[20] and the swing clock (Q10). Left at the relaxed value until C21. |
| Which leg mode a sprint and a backpedal use | REA on the writers of the gait flags (`player-control`); with C18. |
| Whether a standing leg's λ really stays 0 | Probe of the shin's relaxation and rest reach in the retail game (T0). |
| How far a retail arm lags a fast swing | Probe or screen capture of an attack against the same clip in Exarust (T0). |

## Risks / Trade-offs

- [Joints lag fast clips] → the original's damping acts on the joint's own
  motion. Measured: walking joints within 9° (before: 4°), an attacking
  arm up to 34° at the swing's peak speed. Strikes reach less far; the
  duellist stands closer.
- [Clip switches jolt the limbs] → without blending (C10) an Idle→Walk
  switch moves a particle at 14.5 m/s for one tick, as before (15.0 m/s),
  but the joints settle over 4 ticks instead of 2.
- [A softer stance] → T = 0.5 standing halves the link muscles against a
  tone of 1; standing bodies still hold every joint within 0.1° in the
  arena.
