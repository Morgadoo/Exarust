# Design

## Context

A paired door already decides when a leaf is held. Unlocking it does not open the latch at once. The latch stays until a wait has passed, and the other leaf opens with it.

## Goals / Non-Goals

**Goals:**

- Start a 1000 ms wait and clear only the saved latch.
- Open both latches once the clock is strictly past that wait.

**Non-Goals:**

- Which key fits the door, and the unlock sound's random pitch.

## Decisions

1. The wait is `now + 1000` ms, saturating at the top of the clock. The partner receives the same wait and keeps its saved latch. Neither leaf's latch bit clears yet.
2. The latch opens only when the wait is pending and `ready < now`. At the exact millisecond it stays latched. The partner's latch opens in that same tick. The partner's pending wait is left set, so its own tick can finish too.

## Risks / Trade-offs

- The placed door does not yet call this each tick. The function is the wait's rule.

## Migration Plan

No data migration.

## Open Questions

- Which held object fits a door was corrected once and is not re-read here.
