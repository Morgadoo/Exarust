## Context

C8 ported the original's lift servo and target pelvis height: per leg,
the ground under the foot plus the clip root's height over the animated
ankle; the target is the lowest foot's, blended toward the other; standing
it is capped at the leg's reach. The capsule carries the body only across
the ground. On stairs the capsule (28 cm radius, autostep 35 cm) rides up
the step edge ahead while both feet, dragged by their links, are still on
the steps below, and the trailing foot keeps the target there. The research
notes, git-ignored, are `context/re/body-controller-plan.md` (C8, C13, C19
and the Merged line) and `body-drive.md` §14–14.1.

The format page this change extends is
[`docs/formats/RSQ.md`](../../../docs/formats/RSQ.md) (Physical body,
Exarust's own section).

## Goals / Non-Goals

**Goals:**
- the C8 walkers climb exanima01's stairs without falling and stand on
  them;
- level ground unchanged: the arena walk's target the same every tick,
  standing at 101.5 cm;
- the original's lift servo untouched; the rule in one module, easy to
  remove.

**Non-Goals:**
- the original's walking feet and leg IK (C19, C13);
- making the feet follow the body up (they drag behind until C19).

## Decisions

- **Read the ground no lower than the floor less an allowance.** Each
  leg's ground, as the target reads it, is max(ground, floor − 6 cm), the
  floor being the body root's height (the controller's feet). It is
  continuous: the target follows the capsule up as it rides the step edge,
  from the first centimetres, instead of jumping when a gap passes a
  threshold. The C8 trial (target ≥ floor + the walk's stance) is the same
  rule with no allowance, which lifted arena walkers by the capsule's
  float.
- **6 cm.** Traced in the C8 build: the capsule floats 2.01 cm over the
  arena floor walking and 5.05 cm standing after the spawn drop (the
  controller does not sweep a capsule that stands still), 2.9 cm in
  exanima01. 6 cm leaves every level-ground target as it is. On the stairs
  the body rides 6 cm under the capsule's floor plus its float; that
  climbed both starts.
- **Standing too.** Moving only (tried first, with 5 cm) climbed both
  starts, but a walker stopped by the door at the top dropped its target
  20–35 cm onto its trailing foot, two steps down, and fell twice from
  `--at 7.6,-1.3,66.2` (C8 alone stood there at balance 0.69). Standing
  bodies take the rule as well; the cost is a perched capsule: at
  exanima01's start (12.2 cm up on rubble) the target rises 2.1 cm and the
  pelvis 0.9 cm.
- **Only on a grounded controller, only standing.** `Bearing::carried`
  while the body has its controller's support, as the balance reads it: a
  capsule in the air (falling, spawned, placed by a get-up) does not lift
  the body, and neither does the capsule a fallen body left behind (until
  its get-up stands it on the controller again). A body without a
  controller counts as carried; its root is its floor (the `character`
  view: unchanged).
- **The grounds the lift reads, not the grounds measured.** The balance's
  grounded test, the Height Query and `Leg::ground` are left as they are;
  only the copy the target reads is raised.
- **The unit test's walker.** A static humanoid with its legs a stride
  apart (0.4 rad each way), carried at 1.4 m/s by a modelled capsule that
  rides each step edge within its radius; with the controller off the
  ground (the original's target alone) its trailing foot catches and it
  falls (lowest balance 0.05), so the test shows the stopgap at work. With
  the legs together, as first tried, C8's probe ahead lifts both feet at
  once and it climbs without the stopgap.

## Risks / Trade-offs

- The body rides on the capsule like C7 did, 6 cm lower: feet dragging
  behind on a lower step dangle, and the body can be lifted over a stone
  the capsule perches on. Accepted until C13/C19.
- A capsule stuck high against a wall or jamb holds the body up with it.

## Open questions

None for the original: this change reads nothing new from `Exanima.exe`.
The rule goes when C19 (swing flags, the probe ahead, the closing step)
and C13 (leg IK) land; their in-game checks should rerun both stair starts
without it.
