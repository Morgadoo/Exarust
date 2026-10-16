## Context

The original's living sampler blends every clip in from a capture of the
body, and its body tick runs each clip's clock at a rate set by what the
body does. Exarust switched clips in one tick, removed every clip's root
travel and ran all clocks at 60 frames a second. The research notes, with
addresses, are git-ignored: `context/re/body-misc.md` (§2.2–2.7 and the new
§2.9, which answers Q7), `master-animation.md` (§2–§5),
`combat-stances.md` (§6, §8), `combat-damage.md` (§1, the W record) and
`body-controller-plan.md` (C10).

The format page this change extends is
[`docs/formats/RSQ.md`](../../../docs/formats/RSQ.md) (Physical body, and
the new Motion playback section).

## Goals / Non-Goals

**Goals:**
- the original's clip clocks for standing, off-balance, getting-up,
  attacking and walking bodies, from the race's base;
- clip starts that capture where the body is and blend to the first key,
  with the original's eases and rebase;
- the follow-up blend table, loops by the last key, root travel kept but
  for the walks;
- the pose hold of a dazed or unsteady body.

**Non-Goals:**
- deciding when a body falls, staggers or gets up (C11), the locomotion
  clock's stride terms and overlay (C18), the daze's Defence stance (with
  the combat stance), weapon bodies and the swing's other states (C21).

## Decisions

- **Where the capture is taken.** The original captures inside the start.
  Exarust's starts happen in frame and fixed systems that have no body at
  hand, so a start marks its capture pending and the body's next character
  tick takes it before advancing the clock, from the same state (last
  tick's targets, the particles where the last tick left them). A
  follow-up started by the clock's end test captures at once.
- **The capture's bones.** The original tests the node's 1-based skin bone:
  bones 1–7 (pelvis and legs) keep their target, the rest take their
  physical turn relative to the parent part. A whole capture takes every
  part's, the pelvis's relative to last tick's frame. The lumbar's and
  thorax's correction toward the torso twist is the identity without a
  twist (Exarust has none yet), and the femur posture flag is never set.
- **Physical turns.** Each part's rotation from its particles relative to
  its parent part's, through any plain node between them; the pelvis's
  relative to the character frame. Computed only when a capture is
  pending or the body holds its pose.
- **Rates.** Computed by the character tick (`clip_frames`), in the
  original's order: walking with its locomotion clip, getting up,
  attacking, off balance, otherwise. The walking speed is the controller's
  own walk in the last step (less its follow), in cm a tick; the stride
  modulation is taken as 1 (an aligned walk, C18 brings the rest). The run
  blend picks walk → jog by √run; the backpedal clip reads the backpedal
  row (the original plays the walk backward, C18).
- **The swing.** From the held weapon's record as the original's
  holding pass builds it: its swing speed W15 ((1 − weight)·0.15 + 1.05,
  ×1.2 unarmed, with no armour encumbrance), its swing shaping W18
  (1 − balance) and its stance blend W19 (round((weight + 1 − balance)·10)
  for a human). A wielder without a blade swings like fists (1.44, 0, 0).
- **The hold's lerp.** The original lerps the quaternion components with
  no sign check; Exarust takes the short way (glam's `lerp`), which only
  differs when the two turns are in opposite hemispheres.
- **Exarust's footwork clip.** `Combat Walk` (0x2B) is Exarust's interim
  footwork, which the original never plays; it is played in place like
  the walks, or its 80 cm of travel would fight the controller.
- **Frame-ticked bodies.** Bodies without a body tick keep a 60 frames a
  second clock (Exarust's own: every character of the original is a
  body), with the same starts and loops and no physical capture.
- **Strike range.** Exarust's duellist range stand-in goes from 1.2 to
  1.15 m: at the original's swing speed the link-muscle arm lags more and
  the blade passed 1–4 cm short of the head at 1.2 m (no hits in a
  minute's duel); at 1.15 m three of five strikes land.

## Risks / Trade-offs

- A dazed standing body holds its physical pose in full (as the original)
  and, without the daze's Defence start (which smooths the original's
  recovery), its targets return to the clip in one tick when its energy
  reaches 0.95 of its most. The start comes with the combat stance.
- The rising re-anchor (10 cm in a get-up's first quarter) now meets the
  get-up's root travel, so the frame steps in jumps of 8–9 cm until the
  quarter passes, as the original's rule does.
- Root travel kept for Unconcious 1 (Exarust's own lying clip) slides the
  down frame 5 cm at a time while the clip blends in, where it jumped 59 cm.

## Open questions

| question | how it will be answered |
|---|---|
| Live clock rates (idle 0.6, get-up 0.9, walk 0.74 at 1.6 cm a tick) | the retail probe (T0) sampling the clock, progress and speed each tick |
| Per-weapon attack durations | the probe through one uninterrupted Attack Mid with the arena sword |
| Whether the whole capture's frame is last tick's (the heading turn before it) | REA of the body tick's frame build against the posture block |
