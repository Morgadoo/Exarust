# Design

## Behaviour

The orientation pass visits each point of the gathered array and exchanges its second and third floats. The pose advance copies the current pair into the previous one and the next into the current, keeps the next, and stores the frame. That is: previous := current, current := next, next unchanged, frame := argument.

## Model

`swap_barrier_yz(points)`; `BarrierPose` and `advance_barrier_pose(pose, frame)`.

## Open questions

The setup, update and draw are separate.

## Docs

`TForceBarrierEffect` is **6%**. The headline rises slightly.
