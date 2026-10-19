# Design

## Context

A signal of 1 asks the doors under a node to open. The walk plays one unlock sound, then each latched or motorised door waits.

## Goals / Non-Goals

**Goals:**

- Play the unlock sound once for the group.
- Arm a 120-tick wait on a latched or motorised door, and end it when the countdown is below 2.

**Non-Goals:**

- The unlock sound's random pitch, and starting the open command when the wait ends.

## Decisions

1. An empty list plays nothing. Any non-empty list plays the sound once, even if no door is armed.
2. Only a latched or motorised door becomes pending at 120. The others stay idle.
3. A pending countdown of 2 or more falls by 1. Below 2 the wait ends and the countdown is cleared. An idle door does not count.

## Risks / Trade-offs

- The placed door does not yet call this each tick. The functions are the wait's rule.

## Migration Plan

No data migration.

## Open Questions

- None for this slice. The walk and the wait were read before this change.
