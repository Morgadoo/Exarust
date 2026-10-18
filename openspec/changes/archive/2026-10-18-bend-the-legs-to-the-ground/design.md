# Design

## Context

See proposal.md for why. The feet already plant and take a recovery step, and the lift already holds the pelvis over the feet. The stair coupling in the climbing module raises a foot's ground toward the controller floor. This change bends the legs onto their anchors and removes that coupling. Distances stay in centimetres through the body's existing conversion. The behaviour is the one in `docs/formats/RSQ.md` (Physical body, Feet and the stair bullet).

## Goals / Non-Goals

**Goals:**

- The knee angle is the law of cosines, clamped at 30° and 180°, and the muscles see it one tick later.
- A stepping thigh turns 75% of the way to the target, and the step lift matches the 40 cm human case.
- The four-step synthetic walk keeps its balance and its feet on the landing with each foot's own ground.

**Non-Goals:**

- The gait probe, swing flags, the closing step, and the locomoting IK routine.
- Combat steps, knockback, clip blending, and part spheres.

## Decisions

- The bend is a pose the next tick's muscles pull toward. Writing it into the particles on the same tick would skip the lag the notes describe, and it would fight the link solver directly. The muscles already pull toward the pose.
- The stepping branch is included: smoothing 0.2, the 75% thigh turn, and the lift. A foot planting on a higher step is a step. Leaving that branch out would keep the foot on the straight clip while the anchor sits on the step.
- The controller-floor coupling is removed in this change. Keeping it until the gait probe exists would leave Exarust's own lift in the stair path after the legs can bend. Level ground never used the coupling (the capsule floats less than 6 cm), so that scenario stays. The perched 6 cm raise and the stair pelvis's 72 cm capsule rise go away.
- The human lift uses the stance factor 0.5 already decoded for the race. Crouch stays 0 until a crouch exists, so the (1 − crouch) term is 1.
- The thigh's direction is taken in the character frame, the same frame the support pins use.

## Risks / Trade-offs

- [The bend fights the link muscles] → The correction is only a pose target. The synthetic test is that a foot whose anchor is on a 15 cm step meets that step with the knee bent, not a copied particle path.
- [The stair walk sinks once the coupling is gone] → The recovery step already puts the feet on the landing without the coupling, with balance at 1. The stair scenario now requires that, and not the capsule's 72 cm pelvis.
- [The exanima01 stair room still falls] → That walk needs the gait probe, which is out of scope. The fall is recorded and the coupling is not restored. The four-step synthetic walk is the gate.
- [The smoothing weight is applied backwards] → The 60° and 30° scenarios are the unsmoothed angles. The one-tick weight is read before the blend is coded.

## Open Questions

- Resolved: 0.02 and 0.2 are not blend weights. They are the most the stored correction may advance in one tick (radians), added to a stored rate, and the step never passes the target. The rate then becomes half of (old rate + step), clamped to ±0.1. From a zero correction and a zero rate, one tick toward a larger correction moves 0.02 rad (0.2 while stepping) and leaves the rate at half of that. The muscles read the quaternion written for the next tick.
- Resolved: an unanchored target is the anchor, lowered by twice the lift term stored before the lift clamps it. That term is the lift's unclamped factor. Standing, not stepping, at the target height it is 0.075 × 15 = 1.125 cm, so the drop is 2.25 cm. It is not a fixed drop.
- The placement snap runs the same bend with no smoothing. It is not this change's path. A later change can call the unsmoothed bend from placement.
