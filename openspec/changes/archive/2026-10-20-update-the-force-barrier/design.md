# Design

## Behaviour

With a target, the fraction is the target's frame at `+0x5c` minus the barrier's frame at `+0x10`, both converted to float, plus the value returned by the probe `0x10008b7a0`. The position is the original's point lerp `a + (b − a)·t`, where a is the current pair at `+0x300` and b the previous at `+0x30c`; the result is written to the owner at `+0xd0` and the fraction is returned.

## Model

`update_barrier_position(current, previous, target_frame, base_frame, extra) -> ([f32; 3], f32)` and `barrier_lerp`.

## Open questions

The probe's meaning is not decoded; the caller supplies its value.

## Docs

`TForceBarrierEffect` is **9%**. The headline rises slightly.
