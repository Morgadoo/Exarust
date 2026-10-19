# Design

## Context

The duellist closed and then played Attack Mid, Attack Up and Attack Down in order. The combat state picks the blow from the fighter's hands, the distance against eight tenths of reach, and a score.

## Goals / Non-Goals

**Goals:**

- Choose overhead, thrust, the main side or the other side, and refuse the swing when energy is under eight tenths of the maximum.
- Have the duellist start that blow.

**Non-Goals:**

- Hearing, idle activities, the footwork search that produces the candidate, and which of a side's three clips plays.

## Decisions

1. The generator is the 32-bit one already used for combat rolls. An integer draw is the high half of the state times the range.
2. A side plays that side's first clip. The index within the side was not read.
3. While already attacking, a down or deeply crouched target that is not stepping asks for an overhead and does not draw.

## Risks / Trade-offs

- The duellist does not yet search the sixteen footwork candidates, so the score uses a zero aim yaw unless the caller sets one. The unarmed and forward-step paths do not need it.

## Migration Plan

No data migration.

## Open Questions

- Which clip index a side attack uses.
