# Design

## Context

See proposal.md for why. The body iteration already walks each part's links, foot contacts and edges (`Ragdoll::iterate`). The horizontal stance hold (`hold_on_frame`, gain 0.168 × tone² × E⁴) ran once in the integration step and was what kept a standing pelvis on the frame. Support S, the frame, the feet, the spawned pin factor (`BodyStats::pin_factor`) and `Drive::pin_gain` (0.08, recovering to 0.081) already exist. Extends [docs/formats/RSQ.md](../../../docs/formats/RSQ.md) (Physical body: the character frame and the support pins).

## Goals / Non-Goals

**Goals:**

- One pure pull of the clamped horizontal error, support, pin-gain, P, the spawned pin factor and scale, called from the existing iteration for the two thorax pins and the pelvis pin.
- Stop the capsule path from pulling or braking the pelvis horizontally onto the stance spot. Leave the ground carry and the stair stopgap.

**Non-Goals:**

- Recovery steps, leg IK, removing the capsule, gait walking, crouch as a pose, and any change to how falls, get-ups or planted feet choose clips.
- A forward shift of the targets while jumping. Nothing in the body marks a jump yet.

## Decisions

- **The ×0.3 reading.** `context/re/body-drive.md` §6 writes the spring as "capped at 8 cm, × gain × 0.3". `context/re/master-body.md` §5.3 says the stored pin-gain (init 0.08) is the 0.08 inside the gain S²·0.08·P·pin factor. Those are one gain, not two stacked multipliers: the 0.08 is the living pin-gain, and the 0.3 is the get-up factor. They apply together only while getting up (S² · pin-gain · P · pin factor · 0.3). They do not apply as an extra always-on 0.3 on top of pin-gain. The spec's one-application law is the contract; this note is that reading, not a third formula.
- **Where it runs.** The pull is one pure function. `iterate` calls it once per iteration for each thorax pin and the pelvis pin, after the part's links are not required to have run first: once at the start of the iteration, so each of the 20 passes applies the law to the point where that pass finds it. The displacement it subtracts is the point's horizontal slip since the tick began (where integration stored the particles), in centimetres through `cm`, the same slip the foot grip damps. Later passes of the same tick therefore see the motion already applied. The result is pushed with the pin's weights, like every other point correction; the law is that vector, and the point moves by the sum of the squared weights.
- **Targets.** With support, the target is the pin on the character frame's pose, clamped horizontally to the feet centre (the mean of the ankle contacts). Thorax radius (60 − 60·crouch)·0.6·scale cm, or (60 − 30·crouch)·0.6·scale while any foot is stepping (anchored, not yet reached). Pelvis radius 60·scale cm. The pelvis pin is the pelvis origin. Crouch is the bearing's existing value, which stays 0 until a crouch exists. With no support the same frame point is shifted 5·scale cm along the frame's horizontal facing and the pull is zero, including its damping term.
- **P.** √E · T · E⁴, T the tone target the link muscles already use. Non-finite inputs yield no pull.
- **The old hold.** `hold_on_frame` and its 0.168 gain go away. The integration step no longer adds a horizontal pull or a brake on the pelvis. The carry across the ground and the stair stopgap stay.

Alternatives considered: applying the pull once per tick in the integration step would make one tick equal one application, but the helper belongs in the iterations, and one application is the function, not the whole tick. An always-on extra 0.3, or treating §6's 0.3 as a replacement for the 0.08 pin-gain, contradicts the spec. Last tick's velocity as the subtracted displacement would not see corrections already applied in this tick's passes.

## Risks / Trade-offs

- [The pins are a weak spring next to the old 0.168 hold, so a shove could rest farther from the spot] → The planted feet already grip the ankles at the stance anchors. At support 1 the frame stays and the body still rests within 1 cm of its spot. At support 0.95 the frame follows, and a foot that has reached its spot does not step again while that spot is under 5 cm off, so the pins leave the pelvis within 2 cm of where the frame puts it. That 2 cm is the spring the law produces. The old hold is not put back to close it.
- [A light sideways blow sways the chest about 2 cm, not 4 cm] → The pin damping takes most of the slip each pass. The feet still stay within 1 cm of their anchors. The law is not weakened to restore a larger sway.
- [A chest shove turns the torso about half a degree] → It still turns off the pose and turns back, with balance staying up. The spec only requires that sway and the return.
- [Without the stair stopgap the body no longer topples] → The pins keep its balance. The pelvis and the feet still end short of the landing; the stopgap remains what carries it up. The stopgap is not removed.
- [A body with no feet is not sprung back to the origin] → The pins do not run without legs, and the old hold is gone. Its energy still recovers.
- [The hip-socket shove uses a body standing on its feet] → The old hold had been propping a footless fixture. The 2 mm socket limit stays. The limp comparison stays footless.
- [Twenty applications a tick compound] → Each pass subtracts the slip so far, so later passes damp what earlier ones added. The tests pin one application to the law and the multi-tick return separately.
- [Jumping still has no forward target shift] → Deferred until a jump flag exists. Not guessed here.

## Migration Plan

One change, no data migration. Rollback is reverting the commit: the stance hold returns with it.

## Open Questions

None that change the spec, the approach or the tasks. The jump's 5·scale cm shift waits on a jump flag and is out of scope.

A later read of the shipped helper, after this design was written, finds an always-on 0.3 on every pin that runs, and finds that getting up skips the pin calls, so that second 0.3 never reaches a running correction and a get-up applies no pull. The two factors do not stack. The acceptance contract stays the worked examples: scale 0.05 for an alert human, and 0.3 of the standing pull while getting up. This change does not switch to the helper's always-on 0.3.

The same read leaves these for a later change, not this one: the pull is also skipped while walking and while down; one state flag replaces P with 1; the pelvis pin is applied twice in an iteration; a tiny error skips the damper as well as the spring.
