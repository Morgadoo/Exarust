# Design

## Context

See proposal.md for why. The balance measure already runs before the feet are planted, and the support pins already pull toward clamped targets at the start of each solver iteration. This change fills the step those two leave open. Distances stay in the original's centimetres, converted at the body's existing centimetre scale. The pull formula of the pins is unchanged.

## Goals / Non-Goals

**Goals:**

- One recovery step, chosen by the stance score, ended by the original's rules, with the thorax leading while the chest is back inside the trigger.
- The existing foot scenarios still hold: under 5 cm a free foot stays, one foot moves at a time, a stalled step lands where it is.

**Non-Goals:**

- Leg IK, the stepping lift, gait plants, the closing step, combat and knockback steps, footstep sounds, the jolt, and the stair stopgap.

## Decisions

- The radius includes `(1 + balance bonus)`. A human's bonus is 1, so a settled human (base 30, energy 1, support 1) steps at 45 cm. The 22.5 cm product is the same formula with bonus 0, and both are tested. The alternative, dropping the bonus to keep 22.5 for a human, contradicts the binary.
- The radius base starts at the 30 cm floor, widens to 65 cm while down or getting up, and narrows by 0.5 cm a tick. Locomotion, getting up and balance under 0.2 end a step before a new one can start.
- Foot spots are a shifted copy of the pose. The character frame is not moved, so the pose, the pins' clamp and the standing position stay distinct from the step target.
- The side axis of the stance score is the pose's foot-to-foot direction, so feet in pose order score as a natural stance and crossed feet do not. Using the body's lateral axis the other way around penalises every normal stand.
- A step does not stamp the foot's change time. Stamping it would subtract 10 on the next tick and cancel the foot that just started. The early bonus (2, inside 300 ms of the start) is what keeps that foot chosen.
- The thorax lead follows the horizontal error from the pin's world point to the posed target plus the step offset. It is multiplied by 0.4 while Idle plays and 0.7 otherwise, by (direction + 0.5), by (2 − 25·s), by 1.2, by the square root of body scale and by the stance level. Direction is (h²/60 + 0.25)·0.36·forwardness. A 40 cm sideways error with a still chest and scale 1 moves the target 33.6 cm; the same error straight ahead moves it 684.768 cm, and the feet-centre clamp then limits how far the pin can pull. The per-body field is the square root of body scale, which is 1 for the scale this slice uses. Knockback's kind factor of 1 is not applied. The pelvis pin does not lead. The lead runs only while stepping and not past the trigger, and before the clamp.
- The trigger reads last tick's support and last tick's stepping flag. The out-of-balance flag is set in the same measure that wants a step, so the pin pass of that tick does not lead.

## Risks / Trade-offs

- [A light shove or a walk starts a step] → The 2 m/s shove sways about 2 cm and walking counts as locomotion, both under the trigger. The existing shove, walk and stair tests cover them.
- [The 45 cm human reach is wider than the notes' 22.5] → Locked as the bonus, with 22.5 kept for bonus 0. A chest shove that crosses 22.5 but not 45 does not step.
- [A forward error saturates the thorax clamp] → Straight ahead the direction term is large, so the feet-centre clamp, not the raw shift, limits the pull. Sideways stays proportional. The pin spring is unchanged.
- [A stand on the stairs reaches the landing without the stopgap] → During the stand the recovery step puts the feet on the landing. The pelvis stays more than 5 cm below the stopgap climb and balance stays at 1. The stopgap stays.
- [An anchored foot that is not touching is re-targeted] → The score does that; the old farthest-foot stand-in did not. The foot-contact scenarios are unchanged.

## Open Questions

- The radius base is stored as 60 when the balance state is created and again on a full pose reset (placement and re-seat). A recovery-step end does not store it. The same tick decays the base before the chest test, so a base of 60 compares at 59.5 cm and a base already at 30 stays 30. This change still starts at the 30 cm floor. The settled 45 cm requirement does not depend on that reset, and wiring it would move the standing integration test off the floor. That reset is a later change.
