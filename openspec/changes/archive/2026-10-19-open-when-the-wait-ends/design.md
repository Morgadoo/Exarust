# Design

## Context

A latched or motorised door waits 120 ticks after a signalled open. The tick that finds the countdown below 2 calls the open command.

## Goals / Non-Goals

**Goals:**

- Clear the latch, the shut flag and the pull count.
- Start the hinge or slider open, including the 3 cm shove on a hinge.

**Non-Goals:**

- The unlatch sound's random pitch, and telling navigation.

## Decisions

1. The wait calls the open. It does not clear itself to idle first and leave the door shut.
2. A plain hinge goes idle. A motorised hinge sets fraction 0 and drives open for 100 ticks. Both are shoved 3 cm along local Z.
3. A plain slider jumps to 0.6 for 80 ticks. A motorised slider keeps its fraction and drives open for 300 ticks. A slider wakes and is not shoved.

## Risks / Trade-offs

- The placed door does not yet call this each tick. The wait's last tick is the shipped path.

## Migration Plan

No data migration.

## Open Questions

- None for this slice. The open command was read before this change.
