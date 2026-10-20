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
- Evidence: `0x10023d020` (the target check, the `+0x5c − +0x10` term, the probe value and the lerp) with the helper `0x1000194a0` and the 0.2 constant at `0x10033e820`. No new corpus files.
- Out of scope: the setup and draw.

## Phase

Phase 7, thaumaturgy. The barrier update.
