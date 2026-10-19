# Design

## Context

A span choice and a door wait are in. After that choice the path step snaps the walker's aim to the body and asks for the idle stance.

## Goals / Non-Goals

**Goals:**

- Copy the body's frame origin.
- Set the heading, the stored heading and the base heading to the yaw of the body's heading vector.
- Step the roll and set the timer the wander look reads.
- Clear the pace word and the body's marker.
- Ask for idle through the stance helper.

**Non-Goals:**

- The other path phases.
- Naming the pace word and the marker byte beyond the clear. Their other readers were not established.

## Decisions

1. The yaw is atan2 of the heading's x and z. A heading of +Z is 0. Height is not in the yaw.
2. The roll step is the same `roll * 0x8088405 + 1` the wander look uses. The gap is the high half of that step times 3000. The timer adds the clock and 1000, so a look at that clock does not draw a new offset.
3. The stance call is the helper already shipped. The old stance's own clip starts Idle over 30 frames, smoothly. A different clip is left. Idle already playing does not restart.

## Risks / Trade-offs

- The snap is not yet called from a placed character. The function is the step's rule.

## Migration Plan

No data migration.

## Open Questions

- None for this slice. The aim snap was read before this change. The pace word and the marker byte are cleared; what else they mean was not read.
