## Context

The original puts each tick's animated pose on a character frame: a pure
yaw at an origin (the character's place) that a small routine moves to the
pelvis by tolerances chosen per state in the body tick, the get-up start
and the locomotion branch. Only the frame's yaw reaches the pose drive;
its place reaches the support pins and the foot targets. The research
notes, with addresses, are git-ignored: `context/re/body-drive.md` (§6 and
the new §15), `body-misc.md` (§2.4, §2.7), `body-feet.md` (§9.4–9.5, §10)
and `body-controller-plan.md` (C9).

The format page this change extends is
[`docs/formats/RSQ.md`](../../../docs/formats/RSQ.md) (Physical body).

## Goals / Non-Goals

**Goals:**
- the original's frame and its follow rules, in its centimetres;
- a standing body that keeps its stance spot while steady and stays where
  it is pushed while unsteady, with Exarust's capsule hold aimed at the
  frame instead of the capsule;
- the capsule following the body (never teleported), so the camera does;
- a get-up placed by the frame's re-anchoring instead of a capsule move.

**Non-Goals:**
- support pins (C14), foot targets (C12), recovery steps and the jolt
  (C15), clip blends (C10), the original's falls and get-ups (C11),
  locomotion by the carry and the capsule's removal (C18).

## Decisions

- **The rules, as read.** The routine takes a point (the pelvis origin)
  and a tolerance. Stepping, it sets the origin to the point itself.
  Otherwise the wanted origin is the point less the root offset (the clip
  root turned by the frame), at the pelvis's height, taken when its
  squared distance from the origin, in all three axes, is over the
  tolerance squared. The standing branch of the body tick calls it with
  5 cm while down, 0 while stepping or without support, and, standing
  supported, first sets the origin's height to the pelvis's and calls it
  with 5 cm only while the support (a float) is under the double 0.95. The
  locomotion branch writes the pelvis into the origin directly, every
  tick. The get-up start calls it with 10 cm when its clip changes or is
  in its first quarter, and the body tick calls the get-up start every
  tick while the clip is under 0.4. Exarust ports these as one rule table
  (`follow`) and one placement (`anchored`), with the stepping case left
  out (no steps yet).
- **Moving onto the pelvis, not the pelvis less the offset.** That is the
  original's direct write. In Exarust a walk's in-place root sits 10.9 cm
  ahead of the root (the original zeroes it, C10), so the frame's pelvis
  target leads the pelvis while walking: the hold's pull is left out while
  the body moves (only its damping acts), or it would drag the body
  forward every tick.
- **Down.** The original's down condition is set below a balance of 0.3;
  Exarust reads it as off balance with a balance under 0.3 (a falling body
  above it follows the standing rules).
- **Where the targets are placed.** The character tick still computes the
  root's world transform times the clip pose; `aim` then moves every
  target across the ground by the frame origin's offset from the root, so
  the frame is the root's turn at the anchor. Translations only reach the
  hold (and the trace); the muscles read rotations and relative turns, the
  lift relative heights.
- **The carry and the follow.** The capsule still carries a standing body
  by its own walk and turn (until C18), and the frame rides with that
  carry. The capsule then follows the frame: a tenth of the way a step,
  at most 2 cm (a brisk walk), through the controller, before its walk.
  That follow is the body's own move, so `Ragdoll::followed` takes it out
  of the next carry; without that the capsule would chase its own body.
  The cap keeps a get-up's re-anchoring (up to a metre) a glide of about
  half a second.
- **The get-up.** Exarust starts a get-up's clip at its first key, with no
  lead-in, so the re-anchoring takes that key's root offset and the clip's
  pelvis lands on the body; the original uses the offset its sampler left
  the tick before (the lying clip's) and blends into the clip (C10). The
  first quarter is read from the clip clock against the clip's last key,
  last tick's clock.
- **First placement.** A body's first aim places the origin at once, under
  the body as it was built.

## Open questions

- Whether the retail frame origin of a standing player stays put to the
  millimetre at support 1 and follows by 5 cm under hits: a capture with
  the T0 probe (the frame origin against the pelvis) would confirm it; the
  asm is unambiguous.
- Where the original's camera focus sits (the pelvis node, read every
  frame) against Exarust's capsule: Q12, with `exa_freecam`.

## Risks / Trade-offs

- While Exarust's body lies in its own lying clip (Unconcious 1, root
  57 cm off), the frame's origin and so the capsule and the camera sit
  57 cm off the pelvis, and a get-up glides them back (up to 85 cm in the
  arena). C11 replaces that clip with the original's looping fall clips,
  whose roots sit near the origin.
- The walking body is no longer pulled toward the capsule: it walks about
  2 % faster than commanded (the in-place walk pushes on the floor), and
  the capsule follows it.
- The capsule follows into walls through the controller and is stopped
  there; the body's own particles meet the level themselves.
