# Proposal

## Why

The seventh DeepSeek pass on the body dynamics class (lane `motiledyn7`,
docs/PLAN.md Phase 3, Bodies: skeleton, physics, control) called the
placement's leg snap at spawn and mapped its frame height to the feet,
let a placed body's targets feed the snap, poses the thighs by the
behaviour's in-combat flag and the state word's splay bit, carries the
motion start's trailing-foot request (Walk or Run replaced) into the
carry stage's plant with its 500 ms lapse, and drops an extra
normalisation of the stance reference's facing. Its review checked each
rule against the original's instructions and found two gaps: the body
tick asks for the combat thigh turn only when the condition is neither
off its feet nor walking, and the movement decision's move branch
clears the trailing-foot request. A GPU smoke of the live spawn snap
showed the standing player squatting for about a second after spawn
(pelvis 100 cm sinking to 51 cm, against main's steady 102 cm), so the
spawn keeps main's path and the snap with its feet map stays a tested,
uncalled rule.

## What Changes

- physical-body: the Femur posture requirement is modified; a new Trailing-foot request requirement.
- Code: `sampler::thigh_posture` takes the condition; `locomotion::decide`
  zeroes `Locomotor::plant_request` in its move branch and the sampler
  takes no request on a body that moved this tick.

## Evidence

The body tick's posture block (its three femur posture calls), its move
branch, request lapse and post-sample plant, the motion start routine and
the stance score routine, read as instructions in the motiledyn
decompilation pack.

## Out of scope

The AI and player "may act" gate's use of the request (Exarust's AI
reads it as 0), the AI step-end notice's consumer, the start's reset of
the step vectors, the start capture's half-lean, the placement resets of
twist, tone, energy and look, and calling the placement snap at spawn.
