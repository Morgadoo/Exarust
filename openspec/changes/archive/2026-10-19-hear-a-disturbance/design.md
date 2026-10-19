# Design

## Context

Idle behaviours check a disturbance, then an alert, then their own activity. The retail disturbance sample is a constant 0, so the walk never starts from that sample. The threshold is still the check.

## Goals / Non-Goals

**Goals:**

- Return the point to walk to when the disturbance is above 0.3.
- Face a known target more than 45° off when the alert is at least 0.2, or look around after the gap.

**Non-Goals:**

- Reading the grid cell, choosing the wander heading, and the roam, patrol and sentry cycles.

## Decisions

1. The disturbance is checked first. A value of 0.3 does not leave the idle.
2. The look-around gap is clamped into 0.7–3 seconds. A non-finite gap waits the long end.
3. Facing the target wins over looking around.

## Risks / Trade-offs

- A caller that never supplies a disturbance above 0.3 matches the retail sample, which is 0.

## Migration Plan

No data migration.

## Open Questions

- How the look-around heading is drawn.
