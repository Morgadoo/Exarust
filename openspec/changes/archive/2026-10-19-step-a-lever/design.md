# Design

## Context

Doors already creak, travel, pair and unlock. A lever is the next mechanism: two detents, a hand, and a signal or a linked light.

## Goals / Non-Goals

**Goals:**

- Creak from the smoothed arm velocity.
- Choose the hand, the off detent, the stored detent, or the nearest, and step 0.02 rad toward it.
- Report a newly reached detent and arm a timed return.

**Non-Goals:**

- The return's later tick, the nudge of the arm particle, and the shared environment.

## Decisions

1. The creak uses the sliding-door formula on the squared smoothed speed. The smooth is half the old velocity plus half the new one.
2. A zero or non-finite span stays on the first detent. Halfway rounding is the language round, and the tests stay off the halfway.
3. A report of 0 with a linked light fades the light out and does not signal. A report of 1 fades it in and still signals. Forced and inverted, a report of 1 is −1.
4. Arming a return clears the returning flag in that tick. The later tick that starts the return is not this step.

## Risks / Trade-offs

- A placed lever does not yet call this each tick. The function is the step's rule.

## Migration Plan

No data migration.

## Open Questions

- None for this slice. The step was read before this change.
