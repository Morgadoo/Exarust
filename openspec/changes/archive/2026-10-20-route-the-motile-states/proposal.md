# Proposal

## Why

The dispatcher's branch order and guards are decoded: the collapse gate, the shock window, the posture/clip split, the posture leaves and the clip branches. They are the map every later state body hangs from.

## What Changes

- `Route` with the collapse skip, shock, get-up, lying-still, upright, stagger, fall, attack, weak, kick, parry, parry-end, power, gait, switch and standing branches.
- The exact order: collapse gate, shock bit, posture split, posture leaves, then clip branches.

## Capabilities

### New Capabilities

### Modified Capabilities

- `physical-body`: the motile dispatcher's route decision.

## Impact

- `exa_physics::motile`, from `Route` and `route`.
- `docs/PLAN.md` and the generated `docs/COMPONENT-MAP.md`: `TMotileDynamics` becomes **32%** in the source rating. The headline rises slightly.
- Evidence: `0x16A57D` (shock), `0x16A5B5` (posture split), `0x16A64E`–`0x16AA29` (posture leaves), `0x16AA2E`–`0x16B68F` (clip branches). No new corpus files.
- Out of scope: the branch bodies, the animation calls and the limb pull.

## Phase

Phase 7, bodies. The dispatcher's route.
