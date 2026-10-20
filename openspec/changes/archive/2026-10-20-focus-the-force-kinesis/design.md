# Design

## Behaviour

The scale is `1/speed` widened to double, plus 0.015, halved and narrowed: 0.5075 at speed 1 and 0.2575 at 2. The effect keeps a state and a target.

An event whose low bits meet `0xa` sets the state to removal (2). In removal a held target is released and cleared, the cleanup callback runs, and the step returns 2. In the active state (1) the owner hook is installed when the owner accepts, the target's `+0xd40->+0x96f` byte is raised, the frame callback is registered, and the step returns 0. Any other state returns 0 without side effects.

## Model

`kinesis_scale(speed)`, `KinesisEffect { state, target }`, `step_force_kinesis(effect, event, can_hook) -> KinesisStep`.

## Open questions

The grab update and the release body are separate; the active state's tail beyond the hook is not decoded.

## Docs

`TPwrForceKinesis_S` is **10%**. The headline rises slightly.
