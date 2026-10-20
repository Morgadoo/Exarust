# Proposal

## Why

The `TMotileDynamics` dispatcher's branch conditions and leaf formulas are now decoded: the shock window, the posture/clip split, the attack arm curve, the arm floor, parry ramp, gait clamp, auto-parry deadline and switch phase. They are exact and can ship without the animation state machine.

## What Changes

- The unsigned `0x40` shock deadline (`now > deadline`).
- The `balance < 0.6 || condition == 0x300` posture split and `balance < 0.3 && motion < 25` lying-still gate.
- The attack arm curve with the large-body branch, the `clock > 0.3` split, the stab `0.3` scale and the second arm's `(1−clock)²·p·4`.
- The arm floor `min(0.4·|turn|, 0.4)`, parry ramp `0.8p+0.04`, gait clamp `1−0.15g` in 0.05..0.2.
- The auto-parry deadline selection and the `0x10004`/`0x10000` switch phase.
- The get-up clock scale `(1.5−0.5·enc)·base`.

## Capabilities

### New Capabilities

### Modified Capabilities

- `physical-body`: the motile dispatcher's leaves.

## Impact

- `exa_physics::motile`, from `AttackArms`, `attack_arms`, `arm_floor`, `parry_blend`, `gait_arm`, `shock_expired`, `posture_entry`, `lying_still`, `auto_parry_deadline`, `switch_phase` and `get_up_scale`.
- `docs/PLAN.md` and the generated `docs/COMPONENT-MAP.md`: `TMotileDynamics` becomes **30%** in the source rating. The headline rises slightly.
- Evidence: `0x16A57D` (shock), `0x16A5B5` (split), `0x16AA4A`–`0x16AC5D` (attack arms), `0x1001688C0` (arm floor), `0x16B0EB` (parry), `0x16B354` (gait), `0x16B224` (auto-parry) and `0x16B4C3` (switch). No new corpus files.
- Out of scope: the dispatcher's animation calls, the main draw, the limb pull and the remaining state branches.

## Phase

Phase 7, bodies. The dispatcher's leaves.
