# Design

## Context

A hinged door already hides the room behind it when it is shut. Its own step creaks, counts a pull, and shoves itself open.

## Goals / Non-Goals

**Goals:**

- Smooth the handle velocity and play the creak from that speed.
- Count a pull down from 20 and unlatch below 0.
- Shove 3 cm along the facing when the door opens.

**Non-Goals:**

- The motor, the slider, the partner door, and the slam's random pitch.

## Decisions

1. The smoothed velocity is half the old value plus half the new handle velocity.
2. The same tick both adds 1, up to 20, and subtracts 4 while the door is held and not latched and the angle is still short of closed. The unlatch sound plays only when the count goes below 0, and the count is then 0.
3. A latched door does not lose count while it is held.
4. The shove is 3 cm along the facing. A zero facing shoves nothing.

## Risks / Trade-offs

- The level does not yet run this step on a placed door. The functions are the step's rules.

## Migration Plan

No data migration.

## Open Questions

- None for this slice.
