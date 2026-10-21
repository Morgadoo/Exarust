# Proposal

## Why

`TMotileDynamics` (123.7 KB) drives a living body's parts. Its timing curves, charge reservoirs, inverse-mass redistribution, friction and action gates are exact and unimplemented; they are the first verified slice of the largest remaining gap.

## What Changes

- The time warps are `t²(3−2t)` and `0.11t + 1 − 1/(1+8t)`.
- A tired body's clock advances by `0.75·dt`.
- The dual reservoir drains its capacity with spent time and its charge with the drain, or refills by `0.1·√capacity·rate` with `rate = (1 or 1.5)/3320 + bonus`.
- Inverse masses redistribute as `n/((20·m₀ + Σ1/m)·strength)` from the 18-entry segment table; mobility is `0.175·inverse mass`.
- Friction is per mode (0.2/0.4, 0.6/0.4 or the base/awareness formula); contacts smooth as `0.9c + 0.1t` or `0.8c + 0.04`.
- Action gates check the `0x82C1` mask, grip 0.7, awareness 0.7 and the blocking state; the ready gate uses a 40-tick cooldown and 0.9/0.99 clarity.
- A cycle reset restores drag, grip, tone and sag; impacts count in a local-space band.

## Capabilities

### New Capabilities

### Modified Capabilities

- `physical-body`: the motile body's timing, resource and action rules.

## Impact

- `exa_physics::motile`, from `smoothstep`, `ease`, `advance_clock`, `Charge`, `redistribute_inverse_mass`, `friction_for`, `contact_next`, `action_allowed`, `Gate`, `action_ready`, `cycle_reset`, `in_impact_band` and `ImpactCounter`.
- `docs/PLAN.md` and the generated `docs/COMPONENT-MAP.md`: `TMotileDynamics` becomes **25%** in the source rating. The headline rises slightly.
- Evidence: the two smoothstep helpers, the mode-2 step, the clock, the reservoirs, the mass table and its reader, the friction, the two gates, the cycle reset and the impact band. No new corpus files.
- Out of scope: the 20.3 KB tick kernel, the solver, the state machine `SetState`, torque and the animation kernels.

## Phase

Phase 7, bodies. The motile body's bookkeeping.
