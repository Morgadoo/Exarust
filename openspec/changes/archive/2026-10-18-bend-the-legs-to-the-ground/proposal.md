# Proposal

## Why

A foot whose anchor sits on a higher step stays at the clip's straight leg, so the body catches the riser or needs the controller to lift its pelvis. The original bends the two leg bones onto the anchor. That is the next Phase 3 gap after recovery steps (`docs/PLAN.md` §7, milestone M3 "standing man"), and it is what retires the stair stopgap.

## What Changes

- Each leg bends toward its foot anchor by the law of cosines. The knee is 30° when the cosine is above 0.866 and 180° when it is below −1. Standing smoothing is 0.02, stepping smoothing is 0.2, and the muscles follow the correction on the next tick.
- While stepping, the thigh turns 75% of the way from its rest direction toward the hip-to-target direction. Otherwise that extra turn decays by 10% a tick.
- An anchored stepping foot lifts by the distance curve below. A human (stance factor 0.5, no crouch) at 40 cm lifts −11.5185 cm. Other lifts decay by 0.8 or 0.9 a tick.
- **BREAKING:** the lift no longer raises a foot's ground toward the controller's floor. A grounded controller 12 cm up no longer adds 6 cm. The four-step walk keeps its balance with its feet on the landing without that coupling.

## Capabilities

### New Capabilities

None. Leg bending belongs to the physical body.

### Modified Capabilities

- `physical-body`: adds the leg bend, the thigh turn and the step lift, and changes climbing so the controller floor no longer raises the pelvis target. The level-ground target stays the original's.

## Impact

- `exa_physics`: the feet routine writes the next tick's leg pose; the lift stops calling the stair coupling. The climbing module goes away.
- `docs/PLAN.md`, `docs/formats/RSQ.md` and the physics crate README.
- Evidence: the feet notes' leg-IK section, the body-tick order's feet routine, and the controller plan's bend-the-legs change. The human stance factor 0.5 is already pinned by the race tests. Two fields (the smoothing blend's direction, and how far an unanchored target drops) are read with REA before they are coded.
- Out of scope: gait plants, the probe ahead of a walking foot, swing flags and the closing step; the separate locomoting IK routine; combat and knockback steps; clip blending; part spheres; the radius-base reset to 60.
