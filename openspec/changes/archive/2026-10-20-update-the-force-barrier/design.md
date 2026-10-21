# Design

## Behaviour

With a target, the fraction is the target's frame minus the barrier's frame, both converted to float, plus the value the probe returns. The position is the original's point lerp `a + (b − a)·t`, where a is the current pair and b the previous; the result is written to the owner and the fraction is returned.

## Model

`update_barrier_position(current, previous, target_frame, base_frame, extra) -> ([f32; 3], f32)` and `barrier_lerp`.

## Open questions

The probe's meaning is not decoded; the caller supplies its value.

## Docs

`TForceBarrierEffect` is **9%**. The headline rises slightly.
