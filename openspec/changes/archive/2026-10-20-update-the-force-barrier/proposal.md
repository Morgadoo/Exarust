# Proposal

## Why

The barrier's per-tick update lerps its current pair toward the previous one by the target's frame minus its own, plus a probe value. Exarust models the pose but not that update.

## What Changes

- The fraction is `(target frame − base frame) + extra`.
- The position is `a + (b − a)·t`, with a the current pair and b the previous pair.
- The stored padding is 0.2.

## Capabilities

### New Capabilities

### Modified Capabilities

- `thaumaturgy`: the barrier update.

## Impact

- `exa_game::thaum`, from `BARRIER_PADDING`, `barrier_lerp` and `update_barrier_position`.
- `docs/PLAN.md` and the generated `docs/COMPONENT-MAP.md`: `TForceBarrierEffect` becomes **9%** in the source rating. The headline rises slightly.
- Evidence: the barrier update (the target check, the frame difference, the probe value and the lerp) with the lerp helper and the 0.2 constant. No new corpus files.
- Out of scope: the setup and draw.

## Phase

Phase 7, thaumaturgy. The barrier update.
