# Design

## Context

The hinge already creaks and counts a pull. The same step also runs a motor while a toggle or an open command is active.

## Goals / Non-Goals

**Goals:**

- Drive the angle one step toward the command's aim.
- End the command when it arrives, or when its countdown is below 2, after this tick's drive.

**Non-Goals:**

- The sliding door, the partner door, and moving the stop particle.

## Decisions

1. The aim is `(1 − fraction) · range · sign + closed`.
2. A plain door adds `0.001 · |error / range|` to `0.0003`. A zero range drops that term. A motorised door uses `0.0003`, or `0.005` when it is fast.
3. A toggle has arrived when the absolute error is within the step. An open command has arrived within `0.1` rad. Arrival drives to the aim itself.
4. A countdown below 2 clears the command and still returns this tick's drive. A door that bars itself sets its latch and leaves a toggle running.

## Risks / Trade-offs

- The placed door does not yet call this each tick. The function is the command's rule.

## Migration Plan

No data migration.

## Open Questions

- None for this slice.
