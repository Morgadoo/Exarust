# Proposal

## Why

Phase 3 — Bodies: skeleton, physics, control (docs/PLAN.md), the body
dynamics class. The motiledyn10 lane ported three body-tick blocks that the
review found already in Exarust under other names (the femur posture, the
living retention factor, the attack arm stiffness) and reproduced them with
the wrong axis, inputs or constants; they are removed. Its fourth block,
read as a weapon switch timer, is the body tick's ramp of the root's
link-iteration count, which Exarust left fixed at 20.

Evidence: REA (Ghidra) and the objdump listing of the femur posture routine
and its lerp, normalise and axis-angle helpers; the body tick's femur
posture calls and 0xAE8 share (balance and the race ratio); the body tick's
attack grip block (two scalars, pose 3's 0.6 and 0.4); and the body tick's
link-iteration block (the dormant state test, the 5, the feral level
target with its single-precision divide and half-even conversion).

## What Changes

- `exa_physics::body::timers::link_iteration_step`, run by
  `Ragdoll::tick_timers`: 5 while dormant, else one a tick up toward
  round((1 − 0.25·feral)·1660/83) and down to it at once.
- Removed: `body::foot_rest_turn` and its use on the feet,
  `Drive::pose_factor`, `motile::grip_arms`, `weapon_switch_step`.
- `limp::placement_pose` (the default spawn's pose-only key pick) returns
  its bracket so the original spawn samples it once.

## Out of scope

- The dormant branch's mark bit and the scene-graph flag it raises on the
  owning object (no reader in Exarust).
- The rest of the body tick, the parry arm IK and the held-item grip
  consumers listed in the lane report.
