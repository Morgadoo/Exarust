# Design

## Context

The idle ticks are phase counters. Roam uses six phases. Patrol uses three. Sentry only sets the guard pose.

## Goals / Non-Goals

**Goals:**

- Step roam and patrol from a wait, through a path request, to the next wait.
- Hold a sentry in the guard pose.
- Let a disturbance during a wait take the tick.

**Non-Goals:**

- Choosing the wander or home point, follow, sleep, and the statue-mode scale.

## Decisions

1. A wait ends only when the clock is strictly past the deadline.
2. The pause after a wander arrival is 500 ms plus a draw of 2000. The pause after a home arrival is 1000 ms plus a draw of 4000. The generator starts at 0 unless the routine says otherwise.
3. The path is requested on the tick after the wait ends. Acceptance moves the phase to the walk. Arrival schedules the next wait on that tick.
4. Investigation is checked only while waiting. An alert look holds every roam or patrol phase.

## Risks / Trade-offs

- The points themselves are supplied by the caller. This slice decides when to ask for them.

## Migration Plan

No data migration.

## Open Questions

- Where a wander point and a home-side point are picked.
