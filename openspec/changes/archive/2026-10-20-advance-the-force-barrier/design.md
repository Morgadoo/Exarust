# Design

## Behaviour

The orientation pass visits each point of the gathered array and exchanges its second and third floats. The pose advance copies the point at `+0x300` into `+0x30c`, `+0x368` into `+0x300`, `+0x324` into `+0x330` and `+0x35c` into `+0x324`, then stores the frame at `+0x10`. That is: previous := current, current := next, next unchanged, frame := argument.

## Model

`swap_barrier_yz(points)`; `BarrierPose` and `advance_barrier_pose(pose, frame)`.

## Open questions

The setup, update and draw are separate.

## Docs

`TForceBarrierEffect` is **6%**. The headline rises slightly.
