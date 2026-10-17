## Context

The original's foot contacts are 0x98-byte records, one per lower leg,
read in four places of a body's tick: the standing sampler plants a foot
that does not touch where it is (the walking sampler lifts and plants the
feet by the walk's phase instead); the balance routine's footwork, while
the body does not move, clears every foot of a body lying down, stands
every foot on its stance spot while the body gets up (to nine tenths of
its clip) or has no support, and otherwise lands a foot that reached its
anchor or stalled and steps at most one foot to its stance spot by its
stance scores; the feet routine sets each leg's friction by its foot's
state, probes the ground under it, smooths it, sets the anchor on it plus
the sole height and the toe's floor, counts the grounded feet, and
measures the stance's level and the legs' extension; and the limb link
iteration, in each of the 20 iterations, drives each contact toward its
anchor (a velocity servo while walking, a PD grip otherwise, a lifted foot
only pushed up) and pushes the toe up to its floor. The research notes,
with addresses, are git-ignored: `context/re/body-feet.md` §1, §3.5, §5,
§7–8 and §18 (this change's reading), `body-tick-order.md` §5.4, §8.3,
§8.5, `master-movement.md` §7 (Q5), `master-body.md` §3.

The format page this change extends is
[`docs/formats/RSQ.md`](../../../docs/formats/RSQ.md) (Physical body: a new
Feet part before Falls and get-ups; level friction, support, the carry
and the trace amended).

## Goals / Non-Goals

**Goals:**
- the foot contacts' records and states, and every rule of the drive and
  the sole push, as the original's, pinned to their numbers and its
  double-precision steps;
- the feet routine's per-leg work that C6 left out: friction by state, the
  probe by the condition, the smoothed ground, the anchor height, the toe's
  floor, no ground, the stance's level and the legs' extension;
- the sampler's and footwork's state writes that need no stance scores:
  planting, lying, getting up, unsupported, the landing pass, the step's
  execution; the get-up's clear;
- a standing body's feet that stay where they stand.

**Non-Goals:**
- the footwork's scores and timers and the recovery steps (C15), the walk's
  plants (M16/C19), the leg IK (C13), the support pins (C14);
- the sole's orientation drive.

## Decisions

- **Condition 1 is walking.** The notes called the servo branch "normal";
  the condition word's writers (M38) give 1 for the walk or the run
  playing and 0 for standing. So standing and combat feet take the PD
  grip, a standing probe's sphere is 5 cm (C6 had taken 10), and the
  ground falls at once outside the walk. `Bearing::walking` is the
  condition, not the controller's motion; `Bearing::off_feet` is its bit
  0x200 (down, getting up, dormant, dead).
- **Where the work runs.** The states and the feet's measurement run in
  `Ragdoll::measure_balance` (Exarust's movement stage, before its
  sampler: the stance spots are last tick's targets, a tick late, as the
  balance already is); the legs' extension in the lift (the carry stage,
  with this tick's targets); the drive and the sole inside `iterate`,
  after each part's links and before its level contacts, as the original's
  limb link iteration orders them. No new solver stage: they are a living
  body's own links.
- **Planting on contact** (Exarust's own until the leg IK, C13). The
  original plants a foot that does not touch where it is every tick, even
  in the air, and its leg IK then lowers the foot onto its anchor. Without
  the IK a foot planted mid-stride when a walk stops hangs in the air for
  good (the grip fades over its anchor and has no vertical pull), so
  Exarust plants a foot once its part has level contacts.
- **The footwork's choice** (Exarust's own until C15). The step itself is
  the original's (anchored, friction 0, the anchor re-taken every tick at
  the foot's height, the drive pulling it, reached within 5 cm, landed by
  the next landing pass, stalled under an approach of 1e-5). Which foot
  steps is a stand-in for the stance scores: a foot already stepping, else
  the standing foot farthest from its spot once that is 5 cm or more (10
  after a stall), which is where the original's score bias steps a foot.
  Without it a walk's stop left a foot hanging and the body leaning.
- **Walking** (Exarust's own until M16/C19): a walking body's feet do not
  touch, so nothing grips them while the controller carries the body; on
  the stop they are planted where they come down and step to their spots.
- **The carry** moves the anchors with the body (as it moves the level
  contacts), so a standing turn carries the planted feet round until C18
  removes the carry.
- **The toe's floor.** The original's sole query meets only colliders the
  living feet skip (thin horizontal plates and flagged meshes); Exarust's
  level has none of them, so the floor is the ground plus the leg's lift
  offset (0 until C19), capped at the lower leg's origin.
- **The tests' standing body.** Its ankles stood on the floor with a
  3.3 cm "sole" that tuned the lift; anchored on ground plus sole, its
  ankles were pulled up 3.3 cm. Its sole is now 0 and the lift reads its
  legs 3.3 cm longer, which keeps every lift and frame number. A new body
  on feet (10 cm deep, toe ahead) tests the feet.

## Risks / Trade-offs

- A heavily dazed body (three dazes, energy 0.13) now sinks onto its
  planted feet and gets up at once (`--shove 60`: Rebalance at 0.64, down
  21 ticks later, Get Up 2 4 ticks after) instead of toppling backward:
  the original's grip has no energy factor, so this is likely closer;
  checked against the retail game later (RR7).
- The stand-in choice steps feet the original's scores might keep (or the
  reverse); C15 replaces it.

## Open questions

- Which shipped meshes the original's sole query meets (flags 0x800000,
  thin plates): a corpus statistic of the levels' collider flags; only
  matters once Exarust skips those colliders for feet.
- The time stamps of the footwork (100/200/300/600 ms) and its sounds: C15
  reads the game clock (`BodyTicks::millis`).
