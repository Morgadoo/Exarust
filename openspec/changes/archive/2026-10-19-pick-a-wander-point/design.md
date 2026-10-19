# Design

## Context

A roam and a patrol already ask for a wander walk. The point they walk to is chosen from the room's exits.

## Goals / Non-Goals

**Goals:**

- Pick nothing when there is no room.
- Draw one exit, and one more only when it leads back.
- Place the goal a quarter of the way from that exit toward the room point, at height 0, with stop 20 cm and radius 150 cm.

**Non-Goals:**

- Follow and sleep.
- Searching a path to the point.

## Decisions

1. No room means a negative room index or no exits. The roll does not step.
2. The draw is the high half of the stepped roll times the exit count, the same generator the waits use.
3. The blend is the exit plus a quarter of the way to the room point. Height is written as 0. The pick ignores height.
4. The second draw is not checked again. An exit that leads back on both draws is kept.

## Risks / Trade-offs

- The idle tick still only asks to wander. It does not yet carry this point.

## Migration Plan

No data migration.

## Open Questions

- None for this slice. The wander goal was read before this change.
