# Design

## Behaviour

The time warps are single-precision: `t²(3−2t)` and `0.11t + 1 − 1/(1+8t)` with the reciprocal taken in `f32` before widening to `f64`. The clock adds `dt` to the spent time, the budget and the host accumulator, after multiplying by 0.75 when the tired flag is set.

The reservoir has two halves. Spent time above 0.001 raises a rate by 0.001 (capped at 10) and subtracts `rate·spent` from the capacity and the spent time, zeroing the level. A drain above 0.0001 raises the charge rate and subtracts `rate·drain` from the charge and the drain, zeroing the level. Without a drain the rate falls by 0.01 and the level climbs by `rate·0.001` up to the rate; the charge gains `0.1·√capacity·rate`, capped at the capacity.

Inverse masses use the segment table: the sum is the root's `1/m` plus every boned part after the first, the total is `(20·m₀ + sum)·strength` with `m₀` the cube of the mass scale, and the scale is `n/total`. Each part's inverse mass is `weight·scale`, its mobility `0.175·inverse mass`; a boneless part gets tone 0.01.

Friction and contact smoothing, the action masks and thresholds, the ready gate's cooldown and clarity tests, the cycle reset and the local impact band follow the original exactly.

## Model

`exa_physics::motile` exposes the pure functions and the `Charge`, `MassPart`, `Gate`, `Cycle` and `ImpactCounter` types.

## Open questions

The tick kernel, solver, `SetState`, torque and animation kernels are separate; the field names are inferred from use, not from symbols.

## Docs

`TMotileDynamics` is **25%**. The headline rises slightly.
