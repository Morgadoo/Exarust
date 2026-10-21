# Proposal

## Why

`TForceBarrierEffect` (5 KB) orients its points with a y/z swap and keeps a pair of current and previous points with the frame. Exarust does not model either helper.

## What Changes

- The orientation pass swaps every point's y and z.
- The pose advance makes the current pairs previous, the next pairs current, and stores the frame.

## Capabilities

### New Capabilities

### Modified Capabilities

- `thaumaturgy`: the force barrier's pose and orientation.

## Impact

- `exa_game::thaum`, from `swap_barrier_yz`, `BarrierPose` and `advance_barrier_pose`.
- `docs/PLAN.md` and the generated `docs/COMPONENT-MAP.md`: `TForceBarrierEffect` becomes **6%** in the source rating. The headline rises slightly.
- Evidence: the barrier's y/z swap loop and its advance (the four Vec3 pair moves and the frame store). No new corpus files.
- Out of scope: the barrier's setup, update and draw.

## Phase

Phase 7, thaumaturgy. The force barrier's pose.
